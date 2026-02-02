//! Authentication token retrieval example using bluefin-pro SDK
//!
//! Tests:
//! - Sui wallet authentication with Ed25519 signature
//! - POST /auth/v2/token
//!
//! Usage:
//!   cargo run --example get_token -p bf_auth -- --run config/run/run_live.toml

use bf_config::{load_run_config, Environment as BfEnvironment};
use bf_core::redact_id;
use bluefin_api::models::LoginRequest;
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

fn parse_run_arg() -> String {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--run" {
            if let Some(val) = args.next() {
                return val;
            }
        }
    }
    "config/run/run_live.toml".to_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin Authentication Test (SDK) ===\n");

    let run_path = parse_run_arg();
    let runtime = load_run_config(Path::new(&run_path))?;
    let environment = to_sdk_env(&runtime.profile.env.name);

    // Load .env for secrets
    dotenvy::dotenv().ok();
    let secrets = bf_config::AppConfig::load_secrets()?;
    let private_key_hex = secrets.private_key;

    if private_key_hex.is_empty() {
        println!("ERROR: BLUEFIN_PRIVATE_KEY is empty");
        return Ok(());
    }

    if private_key_hex.starts_with("suiprivk") {
        println!("ERROR: Private key is in Bech32 format (suiprivkey...)");
        println!("The SDK expects raw hex format (64 characters)." );
        return Ok(());
    }

    if private_key_hex.len() != 64 {
        println!("WARNING: Private key length is {} (expected 64 hex chars)", private_key_hex.len());
    }

    let account_address = secrets
        .account_address
        .expect("BLUEFIN_ACCOUNT_ADDRESS not set in .env");

    println!("Account address: {}", redact_id(&account_address));
    println!("Environment: {:?}", runtime.profile.env.name);
    println!("Auth URL: {}", auth::url(environment));

    let raw_dir = Path::new(&runtime.app.paths.raw_path).join("rest");
    fs::create_dir_all(&raw_dir)?;

    println!("\n--- Authentication Flow ---");

    let login_request = LoginRequest::new(
        account_address.clone(),
        Utc::now().timestamp_millis(),
        auth::audience(environment).into(),
    );
    let private_key = PrivateKey::from_hex(&private_key_hex)?;
    let signature = login_request.signature(SignatureScheme::Ed25519, private_key)?;
    let token_response = login_request.authenticate(&signature, environment).await?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("auth_token_{}.json", timestamp));
    let output = serde_json::json!({
        "environment": format!("{:?}", runtime.profile.env.name),
        "account_address": account_address,
        "access_token_prefix": &token_response.access_token[..32.min(token_response.access_token.len())],
        "access_token_valid_for_seconds": token_response.access_token_valid_for_seconds,
        "refresh_token_valid_for_seconds": token_response.refresh_token_valid_for_seconds,
        "timestamp": Utc::now().to_rfc3339()
    });
    let mut file = File::create(&output_path)?;
    writeln!(file, "{}", serde_json::to_string_pretty(&output)?)?;
    println!("Saved to: {}", output_path.display());

    Ok(())
}
