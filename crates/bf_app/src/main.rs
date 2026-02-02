//! Main application binary for Bluefin trading bot.

use anyhow::Result;
use bf_auth::{BluefinEnvironment, TokenManager};
use bf_core::{ConfirmStatus, MarketEvent, OrderRequest, Strategy, StrategyContext};
use bf_order_exec::{
    normalize_intent, NormalizationPolicy, OrderExecService, PolicyMode, RoundingMode,
};
use bf_strategies::ExampleStrategy;
use bf_ws::{extract_last_price_e9, parse_market_event, WsClient};
use chrono::Utc;
use rust_decimal::Decimal;
use tokio::sync::mpsc;
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
    let secrets = bf_config::AppConfig::load_secrets()?;
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
    let market_event = load_latest_market_event("data/raw/ws_market_raw.json")
        .await
        .unwrap_or_else(|| MarketEvent::Ticker(bf_core::TickerEvent {
            market: "SUI-PERP".to_string(),
            payload: serde_json::json!({"last_price_e9": "100000000000000"}),
            received_at: Utc::now(),
        }));
    let mut intents = Vec::new();
    for strategy in strategies.iter_mut() {
        intents.extend(strategy.on_event(&ctx, market_event.clone()));
    }
    info!("Strategy intents emitted: {}", intents.len());

    let last_price = extract_last_price(&market_event)?;
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

    let mut executor_handle = None;
    let mut order_tx: Option<mpsc::Sender<OrderRequest>> = None;
    if config.orders.allow_trading && runtime.run.mode.trading.eq_ignore_ascii_case("live") {
        let env = to_sdk_env(&runtime.profile.env.name);
        let token_manager = if matches!(env, BluefinEnvironment::Staging) {
            TokenManager::with_test_keys(env)?
        } else {
            let account_address = secrets
                .account_address
                .clone()
                .ok_or_else(|| anyhow::anyhow!("BLUEFIN_ACCOUNT_ADDRESS not set in .env"))?;
            TokenManager::new(secrets.private_key.clone(), account_address, env)?
        };
        let ws_client = WsClient::new(config.clone());
        let token = token_manager.get_token().await?;
        let mut account_rx = ws_client.connect_account(&token).await?;
        let executor = bf_order_exec::BluefinOrderExecutor::new(
            std::sync::Arc::new(token_manager),
            config.orders.default_leverage,
        );
        let service = OrderExecService::new(std::sync::Arc::new(executor));
        let fallback_enabled = runtime.app.execution.fallback_enabled;
        let confirm_timeout = std::time::Duration::from_secs(
            runtime.app.execution.confirm_timeout_secs,
        );
        let (tx, mut rx) = mpsc::channel::<OrderRequest>(32);
        order_tx = Some(tx);

        executor_handle = Some(tokio::spawn(async move {
            while let Some(order) = rx.recv().await {
                let (ack, status) = service
                    .create_order_with_confirm(
                        order,
                        &mut account_rx,
                        confirm_timeout,
                        fallback_enabled,
                    )
                    .await
                    .map_err(|e| anyhow::anyhow!("Order execution failed: {}", e))?;
                info!("Order ACK: {:?}", ack);
                match status {
                    ConfirmStatus::Active
                    | ConfirmStatus::PartiallyFilled
                    | ConfirmStatus::Filled => {
                        info!("Order confirm status: {:?}", status);
                    }
                    ConfirmStatus::Canceled
                    | ConfirmStatus::Expired
                    | ConfirmStatus::TimedOut => {
                        return Err(anyhow::anyhow!("Order confirm failed: {:?}", status));
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        }));
    }

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
        if let Some(tx) = &order_tx {
            if let Err(e) = tx.send(normalized).await {
                error!("engine->executor transfer failed: {}", e);
                return Err(anyhow::anyhow!("engine->executor transfer failed: {}", e));
            }
        } else if config.orders.allow_trading {
            info!(
                "Trading enabled but mode is {}; execution skipped",
                runtime.run.mode.trading
            );
        } else {
            info!("Trading disabled by config.orders.allow_trading=false");
        }
    }

    drop(order_tx);
    if let Some(handle) = executor_handle {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(anyhow::anyhow!("executor task failed: {}", e)),
        }
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
    if let Some(value) = extract_last_price_e9(payload) {
        if let Ok(e9) = value.parse::<i128>() {
            return Ok(Some(Decimal::from(e9) / Decimal::from(1_000_000_000u64)));
        }
        if let Ok(p) = value.parse::<Decimal>() {
            return Ok(Some(p));
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

fn to_sdk_env(env: &bf_config::Environment) -> BluefinEnvironment {
    match env {
        bf_config::Environment::Staging => BluefinEnvironment::Staging,
        bf_config::Environment::Prod => BluefinEnvironment::Production,
    }
}

async fn load_latest_market_event(path: &str) -> Option<MarketEvent> {
    let content = tokio::fs::read_to_string(path).await.ok()?;
    let values: Vec<serde_json::Value> = serde_json::from_str(&content).ok()?;
    let mut last: Option<MarketEvent> = None;
    for value in values {
        if let Some(evt) = parse_market_event(&value) {
            last = Some(evt);
        }
    }
    last
}
