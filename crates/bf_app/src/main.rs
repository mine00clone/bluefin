//! Main application binary for Bluefin trading bot.

use anyhow::Result;
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
