//! Order creation example using BluefinOrderExecutor
//!
//! Creates a limit order on staging using SDK test account
//!
//! Usage:
//!   cargo run --example create_order -p bf_order_exec

use bf_auth::{BluefinEnvironment, TokenManager};
use bf_config::{AppConfig, Environment as BfEnvironment};
use bf_core::{OrderExecutor, OrderRequest, Side};
use bf_order_exec::BluefinOrderExecutor;
use rust_decimal_macros::dec;
use std::sync::Arc;

fn to_sdk_env(env: &BfEnvironment) -> BluefinEnvironment {
    match env {
        BfEnvironment::Staging => BluefinEnvironment::Staging,
        BfEnvironment::Prod => BluefinEnvironment::Production,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin Order Creation Test ===\n");

    let config = AppConfig::load("config")?;
    let environment = to_sdk_env(&config.env.name);

    // Prefer test keys on staging; otherwise use .env secrets
    let is_staging = matches!(environment, BluefinEnvironment::Staging);
    let token_manager = if is_staging {
        Arc::new(TokenManager::with_test_keys(environment)?)
    } else {
        let secrets = AppConfig::load_secrets()?;
        let account_address = secrets
            .account_address
            .expect("BLUEFIN_ACCOUNT_ADDRESS not set in .env");
        Arc::new(TokenManager::new(
            secrets.private_key,
            account_address,
            environment,
        )?)
    };

    println!("Account: {}", token_manager.account_address());
    println!("Environment: {:?}\n", config.env.name);

    // Create executor
    let executor = BluefinOrderExecutor::new(token_manager.clone(), 10);

    // Create a limit order request
    // Price is set very low to avoid execution (just for testing)
    let order_request = OrderRequest::limit(
        "ETH-PERP",
        Side::Buy,
        dec!(1000), // Very low price - won't execute
        dec!(1),    // 1 ETH
    )
    .with_post_only(true);

    println!("--- Creating Order ---");
    println!("Market: {}", order_request.market);
    println!("Side: {:?}", order_request.side);
    println!("Price: {}", order_request.price.unwrap());
    println!("Size: {}", order_request.size);
    println!("Post-only: {}\n", order_request.post_only);

    // Execute order
    match executor.create_order(order_request).await {
        Ok(order) => {
            println!("=== Order Created Successfully ===");
            println!("Order hash: {}", order.order_hash);
            println!("Market: {}", order.market);
            println!("Status: {:?}", order.status);

            // Cancel the order immediately
            println!("\n--- Cancelling Order ---");
            match executor.cancel_order("ETH-PERP", &order.order_hash).await {
                Ok(()) => {
                    println!("Cancel request submitted for: {}", order.order_hash);
                    println!("(Final confirmation comes via WebSocket)");
                }
                Err(e) => {
                    println!("Cancel failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("ERROR: Order creation failed: {}", e);
        }
    }

    println!("\n=== Test Complete ===");
    Ok(())
}
