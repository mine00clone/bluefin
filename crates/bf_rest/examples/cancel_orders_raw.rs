//! Raw cancel orders requester using bluefin-pro SDK
//!
//! Tests:
//! - PUT /trade/orders/cancel
//!
//! Usage:
//!   cargo run --example cancel_orders_raw -p bf_rest -- --run config/run/run_live.toml --market BTC-PERP --hash <order_hash>
//!   cargo run --example cancel_orders_raw -p bf_rest -- --run config/run/run_live.toml --market BTC-PERP --all

use bf_config::{load_run_config, Environment as BfEnvironment};
use bluefin_api::apis::{configuration::Configuration, trade_api::cancel_orders};
use bluefin_api::models::{CancelOrdersRequest, LoginRequest};
use bluefin_pro::prelude::*;
use chrono::Utc;
use hex::FromHex;
use std::env;
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

fn parse_args() -> (String, Option<String>, Vec<String>, bool) {
    let mut run = "config/run/run_live.toml".to_string();
    let mut market = None;
    let mut hashes: Vec<String> = Vec::new();
    let mut cancel_all = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--run" => {
                if let Some(val) = args.next() {
                    run = val;
                }
            }
            "--market" => {
                if let Some(val) = args.next() {
                    market = Some(val);
                }
            }
            "--hash" => {
                if let Some(val) = args.next() {
                    hashes.push(val);
                }
            }
            "--all" => cancel_all = true,
            _ => {}
        }
    }
    (run, market, hashes, cancel_all)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin Cancel Orders Raw ===\n");

    let (run_path, market_arg, hashes, cancel_all) = parse_args();
    let runtime = load_run_config(Path::new(&run_path))?;
    let environment = to_sdk_env(&runtime.profile.env.name);

    // Load .env for secrets
    dotenvy::dotenv().ok();
    let secrets = bf_config::AppConfig::load_secrets()?;
    let private_key_hex = secrets.private_key;
    let account_address = secrets
        .account_address
        .expect("BLUEFIN_ACCOUNT_ADDRESS not set in .env");

    if private_key_hex.starts_with("suiprivk") || private_key_hex.len() != 64 {
        println!("ERROR: Private key format issue.");
        println!("Expected 64-char hex private key.");
        return Ok(());
    }

    let market = market_arg.ok_or_else(|| anyhow::anyhow!("--market is required"))?;

    if hashes.is_empty() && !cancel_all {
        return Err(anyhow::anyhow!("Provide --hash or --all"));
    }

    println!("Account: {}", account_address);
    println!("Environment: {:?}", runtime.profile.env.name);
    println!("Market: {}", market);

    let raw_dir = Path::new(&runtime.app.paths.raw_path).join("rest");
    fs::create_dir_all(&raw_dir)?;

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
        base_path: runtime.profile.rest.trade_url.clone(),
        bearer_access_token: Some(token_response.access_token.clone()),
        ..Configuration::new()
    };

    let request = CancelOrdersRequest {
        symbol: market.clone(),
        order_hashes: if cancel_all { None } else { Some(hashes.clone()) },
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
