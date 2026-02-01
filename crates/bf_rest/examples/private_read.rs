//! Private REST API example using bluefin-pro SDK
//!
//! Tests authenticated endpoints:
//! - GET /account/details (account info, balances)
//! - GET /trade/openOrders
//!
//! Requires:
//! - BLUEFIN_PRIVATE_KEY in .env (hex format, 64 chars)
//! - BLUEFIN_ACCOUNT_ADDRESS in .env
//!
//! Usage:
//!   cargo run --example private_read -p bf_rest

use bf_config::{AppConfig, Environment as BfEnvironment};
use bluefin_api::apis::{
    account_data_api::get_account_details,
    configuration::Configuration,
    trade_api::get_open_orders,
};
use bluefin_api::models::LoginRequest;
use bluefin_pro::prelude::*;
use chrono::Utc;
use hex::FromHex;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use sui_sdk_types::SignatureScheme;

fn to_sdk_env(env: &BfEnvironment) -> Environment {
    match env {
        BfEnvironment::Staging => Environment::Staging,
        BfEnvironment::Prod => Environment::Production,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin Private REST API Test (SDK) ===\n");

    let config = AppConfig::load("config")?;
    let environment = to_sdk_env(&config.env.name);

    // Load .env for secrets
    dotenvy::dotenv().ok();
    let secrets = AppConfig::load_secrets()?;
    let private_key_hex = secrets.private_key;
    let account_address = secrets
        .account_address
        .expect("BLUEFIN_ACCOUNT_ADDRESS not set in .env");

    // Check key format
    if private_key_hex.starts_with("suiprivk") || private_key_hex.len() != 64 {
        println!("ERROR: Private key format issue.");
        println!("\nFor testing on staging, use the SDK's test account:");
        if let Some(test_keys) = environment.test_keys() {
            println!("  BLUEFIN_PRIVATE_KEY={}", test_keys.private_key);
            println!("  BLUEFIN_ACCOUNT_ADDRESS={}", test_keys.address);
        }
        return Ok(());
    }

    println!("Account: {}", account_address);
    println!("Environment: {:?}", config.env.name);

    // Create raw directory
    let raw_dir = Path::new("data/raw/rest");
    fs::create_dir_all(raw_dir)?;

    // Step 1: Authenticate
    println!("\n--- Authenticating ---");
    let login_request = LoginRequest::new(
        account_address.clone(),
        Utc::now().timestamp_millis(),
        auth::audience(environment).into(),
    );
    let private_key = PrivateKey::from_hex(&private_key_hex)?;
    let signature = login_request.signature(SignatureScheme::Ed25519, private_key)?;
    let token_response = login_request.authenticate(&signature, environment).await?;
    println!("Token obtained (valid for {} seconds)", token_response.access_token_valid_for_seconds);

    // Create API configuration with auth token
    let account_config = Configuration {
        base_path: account::url(environment).into(),
        bearer_access_token: Some(token_response.access_token.clone()),
        ..Configuration::new()
    };

    let trade_config = Configuration {
        base_path: trade::url(environment).into(),
        bearer_access_token: Some(token_response.access_token.clone()),
        ..Configuration::new()
    };

    // Step 2: Get Account Details
    println!("\n--- Fetching Account Details ---");
    println!("URL: {}/api/v1/account", account::url(environment));

    match get_account_details(&account_config, Some(account_address.as_str())).await {
        Ok(account_details) => {
            println!("Account found!");
            println!("  Can trade: {}", account_details.can_trade);
            println!("  Cross effective balance: {} (e9)", account_details.cross_effective_balance_e9);
            println!("  Margin available: {} (e9)", account_details.margin_available_e9);
            println!("  Total account value: {} (e9)", account_details.total_account_value_e9);

            // Show assets (balances)
            println!("  Assets ({}):", account_details.assets.len());
            for asset in &account_details.assets {
                println!("    {} - Quantity: {}, Effective: {}, Max Withdraw: {}",
                    asset.symbol,
                    asset.quantity_e9,
                    asset.effective_balance_e9,
                    asset.max_withdraw_quantity_e9
                );
            }

            // Show positions
            println!("  Positions ({}):", account_details.positions.len());
            for pos in &account_details.positions {
                println!("    {} - Side: {:?}, Size: {}, Entry: {}, Unrealized PnL: {}",
                    pos.symbol,
                    pos.side,
                    pos.size_e9,
                    pos.avg_entry_price_e9,
                    pos.unrealized_pnl_e9
                );
            }

            // Save raw response
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let output_path = raw_dir.join(format!("account_details_{}.json", timestamp));
            let mut file = File::create(&output_path)?;
            writeln!(file, "{}", serde_json::to_string_pretty(&account_details)?)?;
            println!("  Saved to: {}", output_path.display());
        }
        Err(e) => {
            println!("ERROR getting account details: {}", e);
        }
    }

    // Step 3: Get Open Orders
    println!("\n--- Fetching Open Orders ---");
    println!("URL: {}/openOrders", trade::url(environment));

    match get_open_orders(&trade_config, None).await {
        Ok(open_orders) => {
            println!("Found {} open orders", open_orders.len());

            for (i, order) in open_orders.iter().take(5).enumerate() {
                println!("  Order #{}: {:?} {} {} @ {} (status: {:?})",
                    i + 1,
                    order.side,
                    order.quantity_e9,
                    order.symbol,
                    order.price_e9,
                    order.status
                );
            }
            if open_orders.len() > 5 {
                println!("  ... and {} more", open_orders.len() - 5);
            }

            // Save raw response
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let output_path = raw_dir.join(format!("open_orders_{}.json", timestamp));
            let mut file = File::create(&output_path)?;
            writeln!(file, "{}", serde_json::to_string_pretty(&open_orders)?)?;
            println!("  Saved to: {}", output_path.display());
        }
        Err(e) => {
            println!("ERROR getting open orders: {}", e);
        }
    }

    // Step 4: Get Open Orders for specific market
    println!("\n--- Fetching BTC-PERP Open Orders ---");
    match get_open_orders(&trade_config, Some("BTC-PERP")).await {
        Ok(orders) => {
            println!("Found {} BTC-PERP open orders", orders.len());
        }
        Err(e) => {
            println!("ERROR: {}", e);
        }
    }

    println!("\n=== Private REST API Test Complete ===");

    Ok(())
}
