//! Raw exchange info dump
//!
//! Tests:
//! - GET /exchange/info
//!
//! Usage:
//!   cargo run --example raw_exchange_info_dump -p bf_rest

use bf_config::{AppConfig, Environment as BfEnvironment};
use bluefin_api::apis::{configuration::Configuration, exchange_api::get_exchange_info};
use bluefin_pro::prelude::*;
use chrono::Utc;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn to_sdk_env(env: &BfEnvironment) -> Environment {
    match env {
        BfEnvironment::Staging => Environment::Staging,
        BfEnvironment::Prod => Environment::Production,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin Exchange Info Raw Dump ===\n");

    let config = AppConfig::load("config")?;
    let environment = to_sdk_env(&config.env.name);

    let exchange_config = Configuration {
        base_path: exchange::url(environment).into(),
        ..Configuration::new()
    };

    let raw_dir = Path::new("data/raw/rest");
    fs::create_dir_all(raw_dir)?;

    println!("Fetching exchange info...");
    let info = get_exchange_info(&exchange_config).await?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("exchange_info_{}.json", timestamp));
    let mut file = File::create(&output_path)?;
    writeln!(file, "{}", serde_json::to_string_pretty(&info)?)?;
    println!("Saved to: {}", output_path.display());

    Ok(())
}
