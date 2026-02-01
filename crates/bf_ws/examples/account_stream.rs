//! WebSocket Account Stream example using bluefin-pro SDK
//!
//! Tests authenticated account WebSocket:
//! - Connect to wss://stream.api.sui-staging.bluefin.io/ws/account
//! - Subscribe to account updates (orders, positions, trades)
//! - Save raw messages
//!
//! Requires:
//! - BLUEFIN_PRIVATE_KEY in .env (hex format, 64 chars)
//! - BLUEFIN_ACCOUNT_ADDRESS in .env
//!
//! Usage:
//!   cargo run --example account_stream -p bf_ws

use bf_config::{AppConfig, Environment as BfEnvironment};
use bluefin_api::models::{
    AccountDataStream, AccountStreamMessage, AccountSubscriptionMessage, LoginRequest,
    SubscriptionType,
};
use bluefin_pro::prelude::*;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use hex::FromHex;
use serde_json::Value;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use sui_sdk_types::SignatureScheme;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{connect_async, tungstenite::Message};

fn to_sdk_env(env: &BfEnvironment) -> Environment {
    match env {
        BfEnvironment::Staging => Environment::Staging,
        BfEnvironment::Prod => Environment::Production,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin WebSocket Account Stream Test (SDK) ===\n");

    let config = AppConfig::load("config")?;
    let environment = to_sdk_env(&config.env.name);

    // Load .env for secrets
    dotenvy::dotenv().ok();
    let secrets = AppConfig::load_secrets()?;
    let private_key_hex = secrets.private_key;
    let account_address = secrets
        .account_address
        .expect("BLUEFIN_ACCOUNT_ADDRESS not set in .env");

    // Check key format
    if private_key_hex.starts_with("suiprivk") || private_key_hex.len() != 64 {
        println!("ERROR: Private key format issue.");
        println!("\nFor testing on staging, use the SDK's test account:");
        if let Some(test_keys) = environment.test_keys() {
            println!("  BLUEFIN_PRIVATE_KEY={}", test_keys.private_key);
            println!("  BLUEFIN_ACCOUNT_ADDRESS={}", test_keys.address);
        }
        return Ok(());
    }

    println!("Account: {}", account_address);
    println!("Environment: {:?}", config.env.name);

    // Step 1: Authenticate to get token
    println!("\n--- Authenticating ---");
    let login_request = LoginRequest::new(
        account_address.clone(),
        Utc::now().timestamp_millis(),
        auth::audience(environment).into(),
    );
    let private_key = PrivateKey::from_hex(&private_key_hex)?;
    let signature = login_request.signature(SignatureScheme::Ed25519, private_key)?;
    let token_response = login_request.authenticate(&signature, environment).await?;
    println!("Token obtained (valid for {} seconds)", token_response.access_token_valid_for_seconds);

    // Create raw directory
    let raw_dir = Path::new("data/raw/ws");
    fs::create_dir_all(raw_dir)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("account_stream_{}.ndjson", timestamp));
    let mut output_file = File::create(&output_path)?;

    // Step 2: Connect to account WebSocket with auth header
    let ws_url = ws::account::url(environment);
    println!("\n--- Connecting to Account WebSocket ---");
    println!("URL: {}", ws_url);
    println!("Output file: {}", output_path.display());
    println!("Press Ctrl+C to stop\n");

    // Build request with Authorization header
    let mut request = ws_url.into_client_request()?;
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token_response.access_token))?,
    );

    let (ws_stream, response) = connect_async(request).await?;
    println!("Connected! Response: {:?}\n", response.status());

    let (mut write, mut read) = ws_stream.split();

    // Step 3: Subscribe to account streams
    let subscription = AccountSubscriptionMessage::new(
        SubscriptionType::Subscribe,
        vec![
            AccountDataStream::AccountUpdate,
            AccountDataStream::AccountOrderUpdate,
            AccountDataStream::AccountPositionUpdate,
            AccountDataStream::AccountTradeUpdate,
        ],
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

                        // Try to parse as AccountStreamMessage
                        if message_count <= 5 {
                            if let Ok(account_msg) = serde_json::from_str::<AccountStreamMessage>(&text_str) {
                                println!("Message #{}: {:?}", message_count, account_msg);
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
    let unsubscribe = AccountSubscriptionMessage::new(
        SubscriptionType::Unsubscribe,
        vec![
            AccountDataStream::AccountUpdate,
            AccountDataStream::AccountOrderUpdate,
            AccountDataStream::AccountPositionUpdate,
            AccountDataStream::AccountTradeUpdate,
        ],
    );
    let unsubscribe_json = serde_json::to_string(&unsubscribe)?;
    let _ = write.send(Message::Text(unsubscribe_json.into())).await;

    println!("\n=== Summary ===");
    println!("Messages received: {}", message_count);
    println!("Duration: {:?}", start_time.elapsed());
    println!("Output saved to: {}", output_path.display());

    Ok(())
}
