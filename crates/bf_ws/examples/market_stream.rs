//! WebSocket Market Stream example using bluefin-pro SDK
//!
//! Tests:
//! - Connect to market WebSocket using SDK's environment-aware URLs
//! - Subscribe to ticker, orderbook using SDK's message types
//! - Save raw messages
//!
//! Usage:
//!   cargo run --example market_stream -p bf_ws

use bf_config::{AppConfig, Environment as BfEnvironment};
use bluefin_api::models::{
    MarketDataStreamName, MarketStreamMessage, MarketSubscriptionMessage,
    MarketSubscriptionStreams, SubscriptionType,
};
use bluefin_pro::prelude::*;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async, tungstenite::Message};

fn to_sdk_env(env: &BfEnvironment) -> Environment {
    match env {
        BfEnvironment::Staging => Environment::Staging,
        BfEnvironment::Prod => Environment::Production,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin WebSocket Market Stream Test (SDK) ===\n");

    let config = AppConfig::load("config")?;
    let environment = to_sdk_env(&config.env.name);
    let symbol = config
        .markets
        .symbols
        .first()
        .map(String::as_str)
        .unwrap_or("BTC-PERP");

    let raw_dir = Path::new("data/raw/ws");
    fs::create_dir_all(raw_dir)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("market_stream_{}.ndjson", timestamp));
    let mut output_file = File::create(&output_path)?;

    // Use SDK's URL helper for environment-aware WebSocket URL
    let ws_url = ws::market::url(environment);
    println!("Environment: {:?}", config.env.name);
    println!("WebSocket URL: {}", ws_url);
    println!("Symbol: {}", symbol);
    println!("Output file: {}", output_path.display());
    println!("Press Ctrl+C to stop\n");

    let request = ws_url.into_client_request()?;
    let (ws_stream, response) = connect_async(request).await?;
    println!("Connected! Response: {:?}\n", response.status());

    let (mut write, mut read) = ws_stream.split();

    // Use SDK's message types for correct subscription format
    let subscription = MarketSubscriptionMessage::new(
        SubscriptionType::Subscribe,
        vec![MarketSubscriptionStreams::new(
            symbol.into(),
            vec![
                MarketDataStreamName::Ticker,
                MarketDataStreamName::PartialDepth10,
                MarketDataStreamName::RecentTrade,
            ],
        )],
    );

    let subscribe_json = serde_json::to_string(&subscription)?;
    println!("Subscribing: {}", subscribe_json);
    write
        .send(Message::Text(subscribe_json.into()))
        .await?;

    let mut message_count = 0;
    let start_time = tokio::time::Instant::now();
    let max_duration = Duration::from_secs(30); // Run for 30 seconds

    println!("\nReceiving messages (30 seconds)...\n");

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        message_count += 1;
                        let text_str = text.to_string();

                        // Try to parse as MarketStreamMessage for structured output
                        if message_count <= 5 {
                            if let Ok(market_msg) = serde_json::from_str::<MarketStreamMessage>(&text_str) {
                                println!("Message #{}: {:?}", message_count, market_msg);
                            } else if let Ok(json) = serde_json::from_str::<Value>(&text_str) {
                                println!("Message #{}: {}", message_count,
                                    serde_json::to_string(&json).unwrap_or_else(|_| text_str.clone()));
                            } else {
                                println!("Message #{}: {}", message_count, text_str);
                            }
                        } else if message_count == 6 {
                            println!("... (logging to file)");
                        }

                        // Write to file (NDJSON format)
                        let record = serde_json::json!({
                            "received_at": Utc::now().to_rfc3339(),
                            "message": serde_json::from_str::<Value>(&text_str).unwrap_or(Value::String(text_str))
                        });
                        writeln!(output_file, "{}", record)?;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        println!("Ping received, sending pong");
                        write.send(Message::Pong(data)).await?;
                    }
                    Some(Ok(Message::Close(frame))) => {
                        println!("Connection closed: {:?}", frame);
                        break;
                    }
                    Some(Err(e)) => {
                        println!("Error: {}", e);
                        break;
                    }
                    None => {
                        println!("Stream ended");
                        break;
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                if start_time.elapsed() > max_duration {
                    println!("\nTime limit reached");
                    break;
                }
            }
        }
    }

    // Unsubscribe
    let unsubscribe = MarketSubscriptionMessage::new(
        SubscriptionType::Unsubscribe,
        vec![MarketSubscriptionStreams::new(
            symbol.into(),
            vec![
                MarketDataStreamName::Ticker,
                MarketDataStreamName::PartialDepth10,
                MarketDataStreamName::RecentTrade,
            ],
        )],
    );
    let unsubscribe_json = serde_json::to_string(&unsubscribe)?;
    let _ = write.send(Message::Text(unsubscribe_json.into())).await;

    println!("\n=== Summary ===");
    println!("Messages received: {}", message_count);
    println!("Duration: {:?}", start_time.elapsed());
    println!("Output saved to: {}", output_path.display());

    Ok(())
}
