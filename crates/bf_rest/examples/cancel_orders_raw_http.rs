//! Raw cancel orders via HTTP to capture response body.
//!
//! Usage:
//!   cargo run --example cancel_orders_raw_http -p bf_rest -- --run config/run/run_live.toml --market SUI-PERP --hash <order_hash>
//!   cargo run --example cancel_orders_raw_http -p bf_rest -- --run config/run/run_live.toml --market SUI-PERP --all

use bf_config::{load_run_config, Environment as BfEnvironment};
use bf_core::redact_id;
use bluefin_api::models::LoginRequest;
use bluefin_pro::prelude::*;
use chrono::Utc;
use hex::FromHex;
use reqwest::StatusCode;
use serde_json::Value;
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
    println!("=== Bluefin Cancel Orders Raw (HTTP) ===\n");

    let (run_path, market_arg, hashes, cancel_all) = parse_args();
    let runtime = load_run_config(Path::new(&run_path))?;
    let environment = to_sdk_env(&runtime.profile.env.name);

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

    println!("Account: {}", redact_id(&account_address));
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

    let url = format!("{}/api/v1/trade/orders/cancel", runtime.profile.rest.trade_url);
    let request_body = serde_json::json!({
        "symbol": market,
        "orderHashes": if cancel_all { Value::Null } else { Value::from(hashes) },
    });

    println!("\n--- Submitting Cancel ---");
    let client = reqwest::Client::new();
    let response = client
        .put(&url)
        .bearer_auth(token_response.access_token.clone())
        .json(&request_body)
        .send()
        .await?;
    let status = response.status();
    let body_text = response.text().await.unwrap_or_default();

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("cancel_orders_http_{}.json", timestamp));
    let output = serde_json::json!({
        "request": request_body,
        "status": status.as_u16(),
        "body": body_text,
        "timestamp": Utc::now().to_rfc3339(),
    });
    let mut file = File::create(&output_path)?;
    writeln!(file, "{}", serde_json::to_string_pretty(&output)?)?;
    println!("Saved to: {}", output_path.display());

    if status != StatusCode::OK {
        return Err(anyhow::anyhow!("Cancel failed with status: {}", status));
    }

    Ok(())
}
