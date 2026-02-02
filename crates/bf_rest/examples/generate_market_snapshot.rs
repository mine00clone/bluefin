//! Generate market snapshot from raw exchange info
//!
//! Usage:
//!   cargo run --example generate_market_snapshot -p bf_rest -- --input data/raw/rest/exchange_info_YYYYMMDD_HHMMSS.json --output config/markets/snapshot.json

use bf_core::{MarketMeta, MarketSnapshot};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;

fn parse_arg(flag: &str) -> Option<String> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next();
        }
    }
    None
}

fn main() -> anyhow::Result<()> {
    let input = parse_arg("--input")
        .ok_or_else(|| anyhow::anyhow!("--input is required"))?;
    let output = parse_arg("--output")
        .unwrap_or_else(|| "config/markets/snapshot.json".to_string());

    let raw = fs::read_to_string(&input)?;
    let json: Value = serde_json::from_str(&raw)?;

    let markets = json
        .get("markets")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid exchange info: markets not found"))?;

    let mut snapshot = MarketSnapshot { markets: Vec::new() };

    for m in markets {
        let market = m.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
        if market.is_empty() {
            continue;
        }
        let tick_size_e9 = m.get("tickSizeE9").and_then(|v| v.as_str()).unwrap_or("0");
        let step_size_e9 = m.get("stepSizeE9").and_then(|v| v.as_str()).unwrap_or("0");
        let min_order_quantity_e9 = m
            .get("minOrderQuantityE9")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let min_order_price_e9 = m
            .get("minOrderPriceE9")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let max_order_price_e9 = m
            .get("maxOrderPriceE9")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let meta = MarketMeta {
            market: market.to_string(),
            tick_size_e9: tick_size_e9.parse().unwrap_or(0),
            step_size_e9: step_size_e9.parse().unwrap_or(0),
            min_order_quantity_e9: min_order_quantity_e9.parse().unwrap_or(0),
            min_order_price_e9: min_order_price_e9.parse().unwrap_or(0),
            max_order_price_e9: max_order_price_e9.parse().unwrap_or(0),
        };

        snapshot.markets.push(meta);
    }

    let output_path = PathBuf::from(output);
    let content = serde_json::to_string_pretty(&snapshot)?;
    fs::write(&output_path, content)?;
    println!("Snapshot saved to: {}", output_path.display());

    Ok(())
}
