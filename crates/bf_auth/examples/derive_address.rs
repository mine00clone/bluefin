//! Derive Sui address from BLUEFIN_PRIVATE_KEY
//!
//! Usage:
//!   cargo run --example derive_address -p bf_auth

use anyhow::{anyhow, Result};
use bf_config::AppConfig;
use hex::FromHex;
use sui_crypto::ed25519::Ed25519PrivateKey;

fn normalize_hex_key(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.starts_with("suiprivk") {
        return Err(anyhow!(
            "Private key looks like bech32 (suiprivkey...). Convert to raw hex first."
        ));
    }
    let stripped = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    Ok(stripped.to_string())
}

fn main() -> Result<()> {
    // Load .env for secrets
    dotenvy::dotenv().ok();
    let secrets = AppConfig::load_secrets()?;

    let raw_key = secrets.private_key;
    let normalized = normalize_hex_key(&raw_key)?;

    let key_bytes = <[u8; 32]>::from_hex(&normalized)
        .map_err(|e| anyhow!("Invalid hex private key: {}", e))?;

    let private_key = Ed25519PrivateKey::new(key_bytes);
    let public_key = private_key.public_key();
    let address = public_key.derive_address();

    println!("Derived address: {}", address);
    if let Some(expected) = secrets.account_address {
        if expected == address.to_string() {
            println!("Match: OK");
        } else {
            println!("Match: NG (expected {})", expected);
        }
    } else {
        println!("BLUEFIN_ACCOUNT_ADDRESS is not set in .env");
    }

    Ok(())
}
