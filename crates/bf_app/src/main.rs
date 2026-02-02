//! Main application binary for Bluefin trading bot.

use anyhow::Result;
use bf_core::{MarketEvent, Strategy, StrategyContext};
use bf_order_exec::{normalize_intent, NormalizationPolicy, PolicyMode, RoundingMode};
use bf_strategies::ExampleStrategy;
use chrono::Utc;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Bluefin Trading Bot");

    // Load configuration
    let config = bf_config::AppConfig::load("config")?;
    info!("Loaded configuration for environment: {:?}", config.env.name);

    // Load runtime config (run.toml)
    let run_config_path =
        std::env::var("RUN_CONFIG").unwrap_or_else(|_| "config/run/run_paper.toml".to_string());
    let runtime = bf_config::load_run_config(std::path::Path::new(&run_config_path))?;
    info!("Loaded runtime config: {}", run_config_path);

    // Load secrets
    let _secrets = bf_config::AppConfig::load_secrets()?;
    info!("Loaded authentication secrets");

    // Initialize storage
    let _storage = bf_storage_sqlite::SqliteStorage::new(&config.storage.db_path).await?;
    info!("Initialized SQLite storage");

    // TODO: Initialize components
    // 1. TokenManager (bf_auth)
    // 2. RestClient (bf_rest)
    // 3. WsClient (bf_ws)
    // 4. BalanceManager (bf_balance)
    // 5. OrderManager (bf_order_manager)
    // 6. OrderExecService (bf_order_exec)
    // 7. HistoryManager (bf_history)

    // TODO: Start event loops
    // 1. WS account stream -> OrderManager, BalanceManager updates
    // 2. WS market stream -> Strategy updates
    // 3. Periodic REST reconciliation
    // NOTE: If OrderManager::apply_update returns Err (invalid transition),
    //       stop trading and trigger resync/reconcile before continuing.
    let mut strategies: Vec<Box<dyn Strategy>> = Vec::new();
    if let Some(strategy_config) = &runtime.strategy {
        if strategy_config.strategy.enabled {
            let strategy = ExampleStrategy::from_config(strategy_config)?;
            strategies.push(Box::new(strategy));
        }
    }
    let ctx = StrategyContext { now: Utc::now() };
    let dummy_event = MarketEvent::Ticker(bf_core::TickerEvent {
        market: "SUI-PERP".to_string(),
        payload: serde_json::json!({"last_price_e9": "100000000000000"}),
        received_at: Utc::now(),
    });
    let mut intents = Vec::new();
    for strategy in strategies.iter_mut() {
        intents.extend(strategy.on_event(&ctx, dummy_event.clone()));
    }
    info!("Strategy intents emitted: {}", intents.len());

    let last_price = extract_last_price(&dummy_event)?;
    let policy = NormalizationPolicy {
        price_policy: parse_policy(&runtime.app.normalization.price_policy)?,
        size_policy: parse_policy(&runtime.app.normalization.size_policy)?,
        price_rounding: parse_rounding(&runtime.app.normalization.price_rounding)?,
        size_rounding: parse_rounding(&runtime.app.normalization.size_rounding)?,
        max_deviation_bps: runtime
            .app
            .normalization
            .max_deviation_bps
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid max_deviation_bps: {}", e))?,
    };

    for intent in intents {
        let normalized = normalize_intent(
            &intent,
            &runtime.market_snapshot,
            &policy,
            last_price,
        )?;
        info!(
            "Normalized order: {} {} {} @ {:?}",
            normalized.market, normalized.side, normalized.size, normalized.price
        );
    }

    info!("Bot initialization complete - ready for trading");
    info!("Markets: {:?}", config.markets.symbols);

    // Keep running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    Ok(())
}

#[allow(dead_code)]
fn halt_on_oms_error(e: &bf_order_manager::OmsError) -> anyhow::Error {
    error!("OMS error detected; halting for resync: {}", e);
    anyhow::anyhow!("OMS error detected; halt trading: {}", e)
}

fn extract_last_price(event: &MarketEvent) -> anyhow::Result<Option<Decimal>> {
    let payload = match event {
        MarketEvent::OrderBook(e) => &e.payload,
        MarketEvent::Trade(e) => &e.payload,
        MarketEvent::Ticker(e) => &e.payload,
    };
    if let Some(value) = payload.get("last_price_e9") {
        if let Some(s) = value.as_str() {
            let e9 = s.parse::<i128>()?;
            return Ok(Some(Decimal::from(e9) / Decimal::from(1_000_000_000u64)));
        }
        if let Some(n) = value.as_i64() {
            return Ok(Some(Decimal::from(n) / Decimal::from(1_000_000_000u64)));
        }
    }
    if let Some(value) = payload.get("last_price") {
        if let Some(s) = value.as_str() {
            let p = s.parse::<Decimal>()?;
            return Ok(Some(p));
        }
        if let Some(n) = value.as_f64() {
            return Ok(Some(Decimal::from_f64(n).ok_or_else(|| anyhow::anyhow!("Invalid last_price"))?));
        }
    }
    Ok(None)
}

fn parse_policy(raw: &str) -> anyhow::Result<PolicyMode> {
    match raw {
        "snap" => Ok(PolicyMode::Snap),
        "reject" => Ok(PolicyMode::Reject),
        other => Err(anyhow::anyhow!("Invalid policy: {}", other)),
    }
}

fn parse_rounding(raw: &str) -> anyhow::Result<RoundingMode> {
    match raw {
        "down" => Ok(RoundingMode::Down),
        "up" => Ok(RoundingMode::Up),
        "nearest" => Ok(RoundingMode::Nearest),
        other => Err(anyhow::anyhow!("Invalid rounding: {}", other)),
    }
}
