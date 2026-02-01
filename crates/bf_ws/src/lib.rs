//! WebSocket client for Bluefin exchange streaming.
//!
//! Provides:
//! - Market data streaming (/ws/market)
//! - Account data streaming (/ws/account)
//! - Automatic reconnection
//! - Raw message storage

use bf_config::AppConfig;
use bf_core::{AccountEvent, MarketEvent, RawWsMessage};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use std::path::Path;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info};

#[derive(Debug, Error)]
pub enum WsError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Send error: {0}")]
    SendError(String),
    #[error("Receive error: {0}")]
    ReceiveError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// WebSocket client for Bluefin streams
pub struct WsClient {
    config: AppConfig,
    raw_path: String,
}

impl WsClient {
    pub fn new(config: AppConfig) -> Self {
        let raw_path = config.storage.raw_path.clone();
        Self { config, raw_path }
    }

    /// Connect to account WebSocket stream
    pub async fn connect_account(
        &self,
        _auth_token: &str,
    ) -> Result<mpsc::Receiver<AccountEvent>, WsError> {
        let url = &self.config.ws.account_url;
        info!("Connecting to account stream: {}", url);

        let (_tx, rx) = mpsc::channel(100);

        // TODO: Implement actual WebSocket connection
        // 1. Connect to wss://stream.api.{env}.bluefin.io/ws/account
        // 2. Send authentication
        // 3. Handle incoming messages
        // 4. Parse into AccountEvent
        // 5. Send to channel

        Ok(rx)
    }

    /// Connect to market WebSocket stream
    pub async fn connect_market(
        &self,
        markets: Vec<String>,
    ) -> Result<mpsc::Receiver<MarketEvent>, WsError> {
        let url = &self.config.ws.market_url;
        info!("Connecting to market stream: {} for {:?}", url, markets);

        let (_tx, rx) = mpsc::channel(100);

        // TODO: Implement actual WebSocket connection
        // 1. Connect to wss://stream.api.{env}.bluefin.io/ws/market
        // 2. Subscribe to markets
        // 3. Handle incoming messages
        // 4. Parse into MarketEvent
        // 5. Send to channel

        Ok(rx)
    }

    /// Save raw WebSocket message to file
    pub async fn save_raw_message(
        &self,
        stream_type: &str,
        message: &serde_json::Value,
    ) -> Result<(), WsError> {
        let raw_msg = RawWsMessage {
            stream_type: stream_type.to_string(),
            payload: message.clone(),
            received_at: Utc::now(),
        };

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let filename = format!("{}_{}.json", stream_type, timestamp);
        let path = Path::new(&self.raw_path).join("ws").join(&filename);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&raw_msg)
            .map_err(|e| WsError::ParseError(e.to_string()))?;
        tokio::fs::write(&path, content).await?;

        debug!("Saved raw WS message to: {:?}", path);
        Ok(())
    }
}

/// Raw WebSocket connection for initial testing
pub struct RawWsConnection {
    url: String,
}

impl RawWsConnection {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
        }
    }

    /// Connect and receive raw messages (for testing/raw capture)
    pub async fn connect_and_capture(
        &self,
        output_path: &str,
        duration_secs: u64,
    ) -> Result<Vec<serde_json::Value>, WsError> {
        info!("Connecting to {} for raw capture", self.url);

        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();
        let mut messages = Vec::new();

        let timeout = tokio::time::Duration::from_secs(duration_secs);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            debug!("Received: {}", text);
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                messages.push(json);
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let _ = write.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("WebSocket closed");
                            break;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => break,
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
            }
        }

        // Save all messages to file
        let content = serde_json::to_string_pretty(&messages)
            .map_err(|e| WsError::ParseError(e.to_string()))?;
        tokio::fs::write(output_path, content).await?;
        info!("Saved {} raw messages to {}", messages.len(), output_path);

        Ok(messages)
    }
}
