//! Public REST API example - no authentication required
//!
//! Tests:
//! - GET /exchange/info
//! - GET /exchange/tickers
//! - GET /exchange/depth
//!
//! Usage:
//!   cargo run --example public_read -p bf_rest

use bf_config::AppConfig;
use chrono::Utc;
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin Public REST API Test ===\n");

    let config = AppConfig::load("config")?;
    let base_url = config.rest.exchange_url;
    println!("Base URL: {}\n", base_url);

    let client = Client::new();
    let raw_dir = Path::new("data/raw/rest");
    fs::create_dir_all(raw_dir)?;

    // 1. Exchange Info
    println!("1. Fetching exchange info...");
    let url = format!("{}/exchange/info", base_url);
    println!("   URL: {}", url);

    let resp = client.get(&url).send().await?;

    let status = resp.status();
    println!("   Status: {}", status);

    if status.is_success() {
        let json: Value = resp.json().await?;
        let filename = format!("exchange_info_{}.json", Utc::now().format("%Y%m%d_%H%M%S"));
        let path = raw_dir.join(&filename);
        fs::write(&path, serde_json::to_string_pretty(&json)?)?;
        println!("   Saved to: {}", path.display());

        // Print summary
        if let Some(markets) = json.get("markets").and_then(|m| m.as_array()) {
            println!("   Markets count: {}", markets.len());
            for market in markets.iter().take(3) {
                if let Some(symbol) = market.get("symbol").and_then(|s| s.as_str()) {
                    println!("     - {}", symbol);
                }
            }
            if markets.len() > 3 {
                println!("     ... and {} more", markets.len() - 3);
            }
        }
    } else {
        let text = resp.text().await?;
        println!("   Error: {}", text);
    }

    println!();

    // 2. Tickers
    println!("2. Fetching all tickers...");
    let url = format!("{}/exchange/tickers", base_url);
    println!("   URL: {}", url);

    let resp = client.get(&url).send().await?;

    let status = resp.status();
    println!("   Status: {}", status);

    if status.is_success() {
        let json: Value = resp.json().await?;
        let filename = format!("tickers_{}.json", Utc::now().format("%Y%m%d_%H%M%S"));
        let path = raw_dir.join(&filename);
        fs::write(&path, serde_json::to_string_pretty(&json)?)?;
        println!("   Saved to: {}", path.display());

        // Print summary
        if let Some(tickers) = json.as_array() {
            println!("   Tickers count: {}", tickers.len());
            for ticker in tickers.iter().take(3) {
                if let (Some(symbol), Some(last_price)) = (
                    ticker.get("symbol").and_then(|s| s.as_str()),
                    ticker.get("lastPrice").and_then(|p| p.as_str()),
                ) {
                    println!("     - {}: ${}", symbol, last_price);
                }
            }
        }
    } else {
        let text = resp.text().await?;
        println!("   Error: {}", text);
    }

    println!();

    // 3. Order Book Depth (BTC-PERP)
    println!("3. Fetching order book for BTC-PERP...");
    let url = format!("{}/exchange/depth", base_url);
    println!("   URL: {}", url);

    let resp = client
        .get(&url)
        .query(&[("symbol", "BTC-PERP"), ("limit", "5")])
        .send()
        .await?;

    let status = resp.status();
    println!("   Status: {}", status);

    if status.is_success() {
        let json: Value = resp.json().await?;
        let filename = format!("depth_btc_perp_{}.json", Utc::now().format("%Y%m%d_%H%M%S"));
        let path = raw_dir.join(&filename);
        fs::write(&path, serde_json::to_string_pretty(&json)?)?;
        println!("   Saved to: {}", path.display());

        // Print summary
        if let (Some(bids), Some(asks)) = (
            json.get("bids").and_then(|b| b.as_array()),
            json.get("asks").and_then(|a| a.as_array()),
        ) {
            println!("   Bids: {} levels, Asks: {} levels", bids.len(), asks.len());
            if let Some(best_bid) = bids.first() {
                println!("   Best bid: {:?}", best_bid);
            }
            if let Some(best_ask) = asks.first() {
                println!("   Best ask: {:?}", best_ask);
            }
        }
    } else {
        let text = resp.text().await?;
        println!("   Error: {}", text);
    }

    println!("\n=== Done ===");
    println!("Raw responses saved to: {}", raw_dir.display());

    Ok(())
}
