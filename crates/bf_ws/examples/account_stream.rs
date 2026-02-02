//! WebSocket Account Stream example using bluefin-pro SDK
//!
//! Tests authenticated account WebSocket:
//! - Connect to wss://stream.api.../ws/account
//! - Subscribe to account updates (orders, positions, trades)
//! - Save raw messages
//!
//! Usage:
//!   cargo run --example account_stream -p bf_ws -- --run config/run/run_live.toml

use bf_config::{load_run_config, Environment as BfEnvironment};
use bf_core::redact_id;
use bluefin_api::models::{
    AccountDataStream, AccountSubscriptionMessage, LoginRequest, SubscriptionType,
};
use bluefin_pro::prelude::*;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use hex::FromHex;
use serde_json::Value;
use std::env;
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

fn parse_run_arg() -> String {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--run" {
            if let Some(val) = args.next() {
                return val;
            }
        }
    }
    "config/run/run_live.toml".to_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin WebSocket Account Stream Test (SDK) ===\n");

    let run_path = parse_run_arg();
    let runtime = load_run_config(Path::new(&run_path))?;
    let environment = to_sdk_env(&runtime.profile.env.name);

    // Load .env for secrets
    dotenvy::dotenv().ok();
    let secrets = bf_config::AppConfig::load_secrets()?;
    let private_key_hex = secrets.private_key;
    let account_address = secrets
        .account_address
        .expect("BLUEFIN_ACCOUNT_ADDRESS not set in .env");

    if private_key_hex.starts_with("suiprivk") || private_key_hex.len() != 64 {
        println!("ERROR: Private key format issue.");
        return Ok(());
    }

    println!("Account: {}", redact_id(&account_address));
    println!("Environment: {:?}", runtime.profile.env.name);

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
    let raw_dir = Path::new(&runtime.app.paths.raw_path).join("ws");
    fs::create_dir_all(&raw_dir)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("account_stream_{}.ndjson", timestamp));
    let mut output_file = File::create(&output_path)?;

    // Step 2: Connect to account WebSocket with auth header
    let ws_url = ws::account::url(environment);
    println!("\n--- Connecting to Account WebSocket ---");
    println!("URL: {}", ws_url);
    println!("Output file: {}", output_path.display());
    println!("Press Ctrl+C to stop\n");

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
    let max_duration = Duration::from_secs(30);

    println!("\nReceiving messages (30 seconds)...\n");

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        message_count += 1;
                        let text_str = text.to_string();

                        if message_count <= 5 {
                            println!("Message #{} received (see raw file)", message_count);
                        } else if message_count == 6 {
                            println!("... (logging to file)");
                        }

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
