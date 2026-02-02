//! Capture raw market WS messages and extract last_price_e9 if present.
//!
//! Usage:
//!   cargo run --example market_last_price_raw -p bf_ws -- --url wss://stream.api.sui-prod.bluefin.io/ws/market --market SUI-PERP --out data/raw/ws_market_raw.json --seconds 10

use bf_ws::extract_last_price_e9;
use bluefin_api::models::{
    MarketDataStreamName, MarketSubscriptionMessage, MarketSubscriptionStreams, SubscriptionType,
};
use futures_util::{SinkExt, StreamExt};
use std::env;
use tokio_tungstenite::{connect_async, tungstenite::Message};

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
    let market = arg_value(&args, "--market").unwrap_or_else(|| "SUI-PERP".to_string());
    let out = arg_value(&args, "--out").unwrap_or_else(|| "data/raw/ws_market_raw.json".to_string());
    let seconds = arg_value(&args, "--seconds")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(10);

    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    let streams = vec![MarketSubscriptionStreams::new(
        market,
        vec![MarketDataStreamName::Ticker],
    )];
    let subscription = MarketSubscriptionMessage::new(SubscriptionType::Subscribe, streams);
    let subscribe_json = serde_json::to_string(&subscription)?;
    write.send(Message::Text(subscribe_json.into())).await?;

    let mut messages = Vec::new();
    let timeout = tokio::time::Duration::from_secs(seconds);
    let start = tokio::time::Instant::now();

    while start.elapsed() < timeout {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            messages.push(json);
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = write.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Err(_)) => break,
                    None => break,
                    _ => {}
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
        }
    }

    let content = serde_json::to_string_pretty(&messages)?;
    tokio::fs::write(&out, content).await?;

    for message in messages {
        if let Some(value) = extract_last_price_e9(&message) {
            println!("last_price_e9: {}", value);
        }
    }

    Ok(())
}
