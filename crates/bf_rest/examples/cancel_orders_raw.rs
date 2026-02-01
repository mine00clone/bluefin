//! Raw cancel orders requester using bluefin-pro SDK
//!
//! Tests:
//! - PUT /trade/orders/cancel
//!
//! Usage:
//!   BLUEFIN_ALLOW_TRADING=1 BLUEFIN_CANCEL_ORDER_HASHES=hash1,hash2 cargo run --example cancel_orders_raw -p bf_rest
//!   BLUEFIN_ALLOW_TRADING=1 BLUEFIN_CANCEL_ALL=1 BLUEFIN_CANCEL_MARKET=BTC-PERP cargo run --example cancel_orders_raw -p bf_rest

use bf_config::{AppConfig, Environment as BfEnvironment};
use bluefin_api::apis::{configuration::Configuration, trade_api::cancel_orders};
use bluefin_api::models::{CancelOrdersRequest, LoginRequest};
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
    println!("=== Bluefin Cancel Orders Raw ===\n");

    if std::env::var("BLUEFIN_ALLOW_TRADING").ok().as_deref() != Some("1") {
        println!("Set BLUEFIN_ALLOW_TRADING=1 to allow order cancellation.");
        return Ok(());
    }

    let config = AppConfig::load("config")?;
    let environment = to_sdk_env(&config.env.name);

    // Load .env for secrets
    dotenvy::dotenv().ok();
    let secrets = AppConfig::load_secrets()?;
    let private_key_hex = secrets.private_key;
    let account_address = secrets
        .account_address
        .expect("BLUEFIN_ACCOUNT_ADDRESS not set in .env");

    if private_key_hex.starts_with("suiprivk") || private_key_hex.len() != 64 {
        println!("ERROR: Private key format issue.");
        println!("Expected 64-char hex private key.");
        return Ok(());
    }

    let market = std::env::var("BLUEFIN_CANCEL_MARKET")
        .ok()
        .or_else(|| config.markets.symbols.first().cloned())
        .unwrap_or_else(|| "BTC-PERP".to_string());

    let order_hashes = std::env::var("BLUEFIN_CANCEL_ORDER_HASHES")
        .ok()
        .and_then(|raw| {
            let hashes: Vec<String> = raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if hashes.is_empty() { None } else { Some(hashes) }
        });

    let cancel_all = std::env::var("BLUEFIN_CANCEL_ALL").ok().as_deref() == Some("1");
    if order_hashes.is_none() && !cancel_all {
        println!("Provide BLUEFIN_CANCEL_ORDER_HASHES or set BLUEFIN_CANCEL_ALL=1");
        return Ok(());
    }

    println!("Account: {}", account_address);
    println!("Environment: {:?}", config.env.name);
    println!("Market: {}", market);

    // Create raw directory
    let raw_dir = Path::new("data/raw/rest");
    fs::create_dir_all(raw_dir)?;

    // Authenticate
    let login_request = LoginRequest::new(
        account_address.clone(),
        Utc::now().timestamp_millis(),
        auth::audience(environment).into(),
    );
    let private_key = PrivateKey::from_hex(&private_key_hex)?;
    let signature = login_request.signature(SignatureScheme::Ed25519, private_key)?;
    let token_response = login_request.authenticate(&signature, environment).await?;

    let trade_config = Configuration {
        base_path: trade::url(environment).into(),
        bearer_access_token: Some(token_response.access_token.clone()),
        ..Configuration::new()
    };

    let request = CancelOrdersRequest {
        symbol: market.clone(),
        order_hashes: order_hashes.clone(),
    };

    println!("\n--- Submitting Cancel ---");
    let response = cancel_orders(&trade_config, request.clone()).await;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("cancel_orders_{}.json", timestamp));
    let response_status = match &response {
        Ok(_) => "ok".to_string(),
        Err(e) => format!("error: {}", e),
    };
    let output = serde_json::json!({
        "request": request,
        "response": response_status,
        "timestamp": Utc::now().to_rfc3339(),
    });
    let mut file = File::create(&output_path)?;
    writeln!(file, "{}", serde_json::to_string_pretty(&output)?)?;
    println!("Saved to: {}", output_path.display());

    if let Err(e) = response {
        return Err(e.into());
    }

    Ok(())
}
