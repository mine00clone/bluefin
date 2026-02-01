//! Authentication token retrieval example using bluefin-pro SDK
//!
//! Tests:
//! - Sui wallet authentication with Ed25519 signature
//! - POST /auth/v2/token
//!
//! Requires:
//! - BLUEFIN_PRIVATE_KEY in .env (hex format, 64 chars)
//! - BLUEFIN_ACCOUNT_ADDRESS in .env (optional, derived if not set)
//!
//! Usage:
//!   cargo run --example get_token -p bf_auth

use bf_config::{AppConfig, Environment as BfEnvironment};
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
    println!("=== Bluefin Authentication Test (SDK) ===\n");

    let config = AppConfig::load("config")?;
    let environment = to_sdk_env(&config.env.name);

    // Load .env for secrets
    dotenvy::dotenv().ok();
    let secrets = AppConfig::load_secrets()?;
    let private_key_hex = secrets.private_key;

    if private_key_hex.is_empty() {
        println!("ERROR: BLUEFIN_PRIVATE_KEY is empty");
        println!("\nTo get a private key:");
        println!("  1. Install Sui Wallet browser extension");
        println!("  2. Create or import a wallet");
        println!("  3. Export the private key from settings (hex format)");
        return Ok(());
    }

    // Check key format
    if private_key_hex.starts_with("suiprivk") {
        println!("ERROR: Private key is in Bech32 format (suiprivkey...)");
        println!("\nThe SDK expects raw hex format (64 characters).");
        println!("You can convert using: sui keytool convert <bech32_key>");
        println!("\nOr for testing on staging, you can use the SDK's test account:");
        if let Some(test_keys) = environment.test_keys() {
            println!("  BLUEFIN_PRIVATE_KEY={}", test_keys.private_key);
            println!("  BLUEFIN_ACCOUNT_ADDRESS={}", test_keys.address);
        }
        return Ok(());
    }

    if private_key_hex.len() != 64 {
        println!("WARNING: Private key length is {} (expected 64 hex chars)", private_key_hex.len());
        println!("The key should be a 64-character hex string (32 bytes).");
        println!("\nFor testing on staging, you can use the SDK's test account:");
        if let Some(test_keys) = environment.test_keys() {
            println!("  BLUEFIN_PRIVATE_KEY={}", test_keys.private_key);
            println!("  BLUEFIN_ACCOUNT_ADDRESS={}", test_keys.address);
        }
    }

    let account_address = secrets
        .account_address
        .expect("BLUEFIN_ACCOUNT_ADDRESS not set in .env");

    if account_address.is_empty() {
        println!("ERROR: BLUEFIN_ACCOUNT_ADDRESS is empty");
        println!("Please set your Sui account address (0x...) in .env");
        return Ok(());
    }

    println!("Account address: {}", account_address);
    println!(
        "Private key loaded: {}...{}",
        &private_key_hex[..8.min(private_key_hex.len())],
        &private_key_hex[private_key_hex.len().saturating_sub(4)..]
    );
    println!("Environment: {:?}", config.env.name);
    println!("Auth URL: {}", auth::url(environment));

    // Create raw directory
    let raw_dir = Path::new("data/raw/rest");
    fs::create_dir_all(raw_dir)?;

    println!("\n--- Authentication Flow ---");

    // Step 1: Create LoginRequest
    let login_request = LoginRequest::new(
        account_address.clone(),
        Utc::now().timestamp_millis(),
        auth::audience(environment).into(),
    );
    println!("1. Created LoginRequest:");
    println!("   account: {}", account_address);
    println!("   audience: {}", auth::audience(environment));

    // Step 2: Parse private key and sign
    println!("\n2. Signing with Ed25519...");
    let private_key = PrivateKey::from_hex(&private_key_hex)?;
    let signature = login_request.signature(SignatureScheme::Ed25519, private_key)?;
    println!(
        "   Signature: {}...{}",
        &signature[..16.min(signature.len())],
        &signature[signature.len().saturating_sub(8)..]
    );

    // Step 3: Authenticate
    println!("\n3. Authenticating with Bluefin...");
    let token_response = login_request.authenticate(&signature, environment).await?;

    println!("\n--- Token Response ---");
    println!("Access token: {}...", &token_response.access_token[..32.min(token_response.access_token.len())]);
    println!(
        "Valid for: {} seconds",
        token_response.access_token_valid_for_seconds
    );
    println!(
        "Refresh token valid for: {} seconds",
        token_response.refresh_token_valid_for_seconds
    );

    // Save raw response
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("auth_token_{}.json", timestamp));
    let output = serde_json::json!({
        "environment": "staging",
        "account_address": account_address,
        "access_token_prefix": &token_response.access_token[..32.min(token_response.access_token.len())],
        "access_token_valid_for_seconds": token_response.access_token_valid_for_seconds,
        "refresh_token_valid_for_seconds": token_response.refresh_token_valid_for_seconds,
        "timestamp": Utc::now().to_rfc3339()
    });
    let mut file = File::create(&output_path)?;
    writeln!(file, "{}", serde_json::to_string_pretty(&output)?)?;
    println!("\nSaved to: {}", output_path.display());

    println!("\n=== Authentication Successful ===");
    println!("Token can be used for:");
    println!("  - Private REST API calls (Authorization: Bearer <token>)");
    println!("  - Account WebSocket streams (Authorization header)");

    Ok(())
}
