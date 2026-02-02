//! Raw account details fetcher using bluefin-pro SDK
//!
//! Tests:
//! - GET /account/details
//!
//! Usage:
//!   cargo run --example account_details_raw -p bf_rest -- --run config/run/run_live.toml

use bf_config::{load_run_config, Environment as BfEnvironment};
use bf_core::redact_id;
use bluefin_api::apis::{account_data_api::get_account_details, configuration::Configuration};
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
    println!("=== Bluefin Account Details Raw Fetch ===\n");

    let run_path = parse_run_arg();
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

    println!("Account: {}", redact_id(&account_address));
    println!("Environment: {:?}", runtime.profile.env.name);

    // Create raw directory
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

    let account_config = Configuration {
        base_path: runtime.profile.rest.exchange_url.clone(),
        bearer_access_token: Some(token_response.access_token.clone()),
        ..Configuration::new()
    };

    println!("\n--- Fetching Account Details ---");
    println!("URL: {}/api/v1/account", runtime.profile.rest.exchange_url);

    let account_details = get_account_details(&account_config, Some(account_address.as_str())).await?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("account_details_{}.json", timestamp));
    let mut file = File::create(&output_path)?;
    writeln!(file, "{}", serde_json::to_string_pretty(&account_details)?)?;
    println!("Saved to: {}", output_path.display());

    Ok(())
}
