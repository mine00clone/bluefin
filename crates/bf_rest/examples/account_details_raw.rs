//! Raw account details fetcher using bluefin-pro SDK
//!
//! Tests:
//! - GET /account/details
//!
//! Usage:
//!   cargo run --example account_details_raw -p bf_rest

use bf_config::{AppConfig, Environment as BfEnvironment};
use bluefin_api::apis::{account_data_api::get_account_details, configuration::Configuration};
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
    println!("=== Bluefin Account Details Raw Fetch ===\n");

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

    println!("Account: {}", account_address);
    println!("Environment: {:?}", config.env.name);

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

    let account_config = Configuration {
        base_path: account::url(environment).into(),
        bearer_access_token: Some(token_response.access_token.clone()),
        ..Configuration::new()
    };

    println!("\n--- Fetching Account Details ---");
    println!("URL: {}/api/v1/account", account::url(environment));

    let account_details = get_account_details(&account_config, Some(account_address.as_str())).await?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("account_details_{}.json", timestamp));
    let mut file = File::create(&output_path)?;
    writeln!(file, "{}", serde_json::to_string_pretty(&account_details)?)?;
    println!("Saved to: {}", output_path.display());

    Ok(())
}
