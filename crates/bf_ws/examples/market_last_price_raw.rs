//! Capture raw market WS messages and extract last_price_e9 if present.
//!
//! Usage:
//!   cargo run --example market_last_price_raw -p bf_ws -- --url wss://stream.api.sui-prod.bluefin.io/ws/market --out data/raw/ws_market_raw.json --seconds 10

use bf_ws::{extract_last_price_e9, RawWsConnection};
use std::env;

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|v| v == key)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let url = arg_value(&args, "--url")
        .unwrap_or_else(|| "wss://stream.api.sui-prod.bluefin.io/ws/market".to_string());
    let out = arg_value(&args, "--out").unwrap_or_else(|| "data/raw/ws_market_raw.json".to_string());
    let seconds = arg_value(&args, "--seconds")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(10);

    let conn = RawWsConnection::new(&url);
    let messages = conn.connect_and_capture(&out, seconds).await?;

    for message in messages {
        if let Some(value) = extract_last_price_e9(&message) {
            println!("last_price_e9: {}", value);
        }
    }

    Ok(())
}
