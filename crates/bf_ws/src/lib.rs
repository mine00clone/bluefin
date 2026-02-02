//! WebSocket client for Bluefin exchange streaming.
//!
//! Provides:
//! - Market data streaming (/ws/market)
//! - Account data streaming (/ws/account)
//! - Automatic reconnection
//! - Raw message storage

use bf_config::AppConfig;
use bf_core::{
    AccountEvent, Balance, BalanceUpdateEvent, MarketEvent, MarketTradeEvent, Order,
    OrderBookEvent, OrderStatus, OrderType, OrderUpdateEvent, PositionUpdateEvent, RawWsMessage,
    Side, TickerEvent, TimeInForce, TradeUpdateEvent,
};
use bluefin_api::models::{
    AccountDataStream, AccountOrderUpdate, AccountStreamMessage, AccountStreamMessagePayload,
    AccountSubscriptionMessage, AccountTradeUpdate, ActiveOrderUpdate, MarketDataStreamName,
    MarketSubscriptionMessage, MarketSubscriptionStreams, OrderStatus as ApiOrderStatus,
    OrderTimeInForce as ApiTimeInForce, OrderType as ApiOrderType, SubscriptionType, Trade,
    TradeSide as ApiTradeSide,
};
use chrono::{TimeZone, Utc};
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde_json::Value;
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
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
        auth_token: &str,
    ) -> Result<mpsc::Receiver<AccountEvent>, WsError> {
        let url = &self.config.ws.account_url;
        info!("Connecting to account stream: {}", url);

        let (tx, rx) = mpsc::channel(100);
        let url = url.to_string();
        let raw_path = self.raw_path.clone();
        let auth_token = auth_token.to_string();

        tokio::spawn(async move {
            if let Err(e) = run_account_stream(url, auth_token, raw_path, tx).await {
                error!("Account WS stream error: {}", e);
            }
        });

        Ok(rx)
    }

    /// Connect to market WebSocket stream
    pub async fn connect_market(
        &self,
        markets: Vec<String>,
    ) -> Result<mpsc::Receiver<MarketEvent>, WsError> {
        let url = &self.config.ws.market_url;
        info!("Connecting to market stream: {} for {:?}", url, markets);

        let (tx, rx) = mpsc::channel(100);
        let url = url.to_string();
        let raw_path = self.raw_path.clone();

        tokio::spawn(async move {
            if let Err(e) = run_market_stream(url, markets, raw_path, tx).await {
                error!("Market WS stream error: {}", e);
            }
        });

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

async fn run_account_stream(
    url: String,
    auth_token: String,
    raw_path: String,
    sender: mpsc::Sender<AccountEvent>,
) -> Result<(), WsError> {
    let mut request = url
        .into_client_request()
        .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", auth_token))
            .map_err(|e| WsError::ConnectionFailed(e.to_string()))?,
    );

    let (ws_stream, _) = connect_async(request)
        .await
        .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;
    let (mut write, mut read) = ws_stream.split();

    let subscription = AccountSubscriptionMessage::new(
        SubscriptionType::Subscribe,
        vec![
            AccountDataStream::AccountUpdate,
            AccountDataStream::AccountOrderUpdate,
            AccountDataStream::AccountPositionUpdate,
            AccountDataStream::AccountTradeUpdate,
        ],
    );
    let subscribe_json =
        serde_json::to_string(&subscription).map_err(|e| WsError::ParseError(e.to_string()))?;
    write
        .send(Message::Text(subscribe_json.into()))
        .await
        .map_err(|e| WsError::SendError(e.to_string()))?;

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let value: Value =
                    serde_json::from_str(&text).unwrap_or(Value::String(text.to_string()));
                save_raw_message_to(&raw_path, "account", &value).await?;
                match parse_account_events(&value) {
                    Ok(events) => {
                        for evt in events {
                            let _ = sender.send(evt).await;
                        }
                    }
                    Err(e) => {
                        error!("Account WS parse error: {}", e);
                        save_raw_error_to(&raw_path, "account_parse_error", &value, &e.to_string())
                            .await?;
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                write
                    .send(Message::Pong(data))
                    .await
                    .map_err(|e| WsError::SendError(e.to_string()))?;
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(e) => return Err(WsError::ReceiveError(e.to_string())),
        }
    }

    Ok(())
}

async fn run_market_stream(
    url: String,
    markets: Vec<String>,
    raw_path: String,
    sender: mpsc::Sender<MarketEvent>,
) -> Result<(), WsError> {
    let request = url
        .into_client_request()
        .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;
    let (ws_stream, _) = connect_async(request)
        .await
        .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;
    let (mut write, mut read) = ws_stream.split();

    let streams = markets
        .into_iter()
        .map(|market| {
            MarketSubscriptionStreams::new(
                market,
                vec![
                    MarketDataStreamName::Ticker,
                    MarketDataStreamName::PartialDepth10,
                    MarketDataStreamName::RecentTrade,
                ],
            )
        })
        .collect::<Vec<_>>();

    let subscription = MarketSubscriptionMessage::new(SubscriptionType::Subscribe, streams);
    let subscribe_json =
        serde_json::to_string(&subscription).map_err(|e| WsError::ParseError(e.to_string()))?;
    write
        .send(Message::Text(subscribe_json.into()))
        .await
        .map_err(|e| WsError::SendError(e.to_string()))?;

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let value: Value =
                    serde_json::from_str(&text).unwrap_or(Value::String(text.to_string()));
                save_raw_message_to(&raw_path, "market", &value).await?;
                if let Some(evt) = parse_market_event(&value) {
                    let _ = sender.send(evt).await;
                } else {
                    error!("Market WS parse error: unknown event");
                    save_raw_error_to(&raw_path, "market_parse_error", &value, "unknown event")
                        .await?;
                }
            }
            Ok(Message::Ping(data)) => {
                write
                    .send(Message::Pong(data))
                    .await
                    .map_err(|e| WsError::SendError(e.to_string()))?;
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(e) => return Err(WsError::ReceiveError(e.to_string())),
        }
    }

    Ok(())
}

async fn save_raw_message_to(
    raw_path: &str,
    stream_type: &str,
    message: &Value,
) -> Result<(), WsError> {
    let raw_msg = RawWsMessage {
        stream_type: stream_type.to_string(),
        payload: message.clone(),
        received_at: Utc::now(),
    };

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let filename = format!("{}_{}.json", stream_type, timestamp);
    let path = Path::new(raw_path).join("ws").join(&filename);

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = serde_json::to_string_pretty(&raw_msg)
        .map_err(|e| WsError::ParseError(e.to_string()))?;
    tokio::fs::write(&path, content).await?;

    debug!("Saved raw WS message to: {:?}", path);
    Ok(())
}

async fn save_raw_error_to(
    raw_path: &str,
    stream_type: &str,
    message: &Value,
    error: &str,
) -> Result<(), WsError> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let filename = format!("{}_{}.json", stream_type, timestamp);
    let path = Path::new(raw_path).join("ws").join(&filename);

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = serde_json::json!({
        "stream_type": stream_type,
        "error": error,
        "payload": message,
        "received_at": Utc::now().to_rfc3339(),
    });
    let serialized = serde_json::to_string_pretty(&content)
        .map_err(|e| WsError::ParseError(e.to_string()))?;
    tokio::fs::write(&path, serialized).await?;
    Ok(())
}

pub fn parse_market_event(value: &Value) -> Option<MarketEvent> {
    let event = value.get("event")?.as_str()?;
    let payload = value.get("payload")?.clone();
    let market = payload
        .get("symbol")
        .or_else(|| payload.get("market"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let received_at = Utc::now();

    if event.contains("Ticker") {
        return Some(MarketEvent::Ticker(TickerEvent {
            market,
            payload,
            received_at,
        }));
    }
    if event.contains("Depth") || event.contains("OrderBook") {
        return Some(MarketEvent::OrderBook(OrderBookEvent {
            market,
            payload,
            received_at,
        }));
    }
    if event.contains("Trade") {
        return Some(MarketEvent::Trade(MarketTradeEvent {
            market,
            payload,
            received_at,
        }));
    }

    None
}

fn parse_account_events(value: &Value) -> Result<Vec<AccountEvent>, WsError> {
    let parsed = serde_json::from_value::<AccountStreamMessage>(value.clone())
        .map_err(|e| WsError::ParseError(e.to_string()))?;

    let received_at = Utc::now();
    let events = match parsed {
        AccountStreamMessage::AccountUpdate { payload, .. } => match payload {
            AccountStreamMessagePayload::AccountUpdate(update) => update
                .assets
                .iter()
                .filter_map(|asset| {
                    let available = parse_e9_decimal(&asset.max_withdraw_quantity_e9)?;
                    let balance = Balance::new(&asset.symbol, available, available);
                    Some(AccountEvent::BalanceUpdate(BalanceUpdateEvent { balance, received_at }))
                })
                .collect(),
            _ => Vec::new(),
        },
        AccountStreamMessage::AccountOrderUpdate { payload, .. } => match payload {
            AccountStreamMessagePayload::AccountOrderUpdate(update) => match update {
                AccountOrderUpdate::ActiveOrderUpdate(active) => {
                    if let Some(order) = order_from_active_update(&active, received_at) {
                        vec![AccountEvent::OrderUpdate(OrderUpdateEvent { order, received_at })]
                    } else {
                        Vec::new()
                    }
                }
                AccountOrderUpdate::OrderCancellationUpdate(cancel) => {
                    if let Some(order) = order_from_cancel_update(&cancel, received_at) {
                        vec![AccountEvent::OrderUpdate(OrderUpdateEvent { order, received_at })]
                    } else {
                        Vec::new()
                    }
                }
            },
            _ => Vec::new(),
        },
        AccountStreamMessage::AccountTradeUpdate { payload, .. } => match payload {
            AccountStreamMessagePayload::AccountTradeUpdate(AccountTradeUpdate { trade }) => {
                if let Some(fill) = fill_from_trade(&trade) {
                    vec![AccountEvent::TradeUpdate(TradeUpdateEvent { fill, received_at })]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        },
        AccountStreamMessage::AccountPositionUpdate { payload, .. } => {
            let payload_value = serde_json::to_value(&payload)
                .map_err(|e| WsError::ParseError(e.to_string()))?;
            let market = payload_value
                .get("symbol")
                .or_else(|| payload_value.get("market"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            vec![AccountEvent::PositionUpdate(PositionUpdateEvent {
                market,
                payload: payload_value,
                received_at,
            })]
        }
        _ => Vec::new(),
    };
    Ok(events)
}

fn order_from_active_update(
    update: &ActiveOrderUpdate,
    received_at: chrono::DateTime<Utc>,
) -> Option<Order> {
    let price = parse_e9_decimal(&update.price_e9)?;
    let size = parse_e9_decimal(&update.quantity_e9)?;
    let filled = parse_e9_decimal(&update.filled_quantity_e9)?;

    Some(Order {
        order_hash: update.order_hash.clone(),
        market: update.symbol.clone(),
        side: map_side(update.side)?,
        order_type: map_order_type(update.r#type),
        price: Some(price),
        size,
        filled_size: filled,
        status: map_order_status(update.status),
        time_in_force: map_time_in_force(update.time_in_force),
        reduce_only: update.reduce_only,
        post_only: update.post_only,
        client_order_id: update.client_order_id.clone(),
        created_at: millis_to_datetime(update.created_at_millis).unwrap_or(received_at),
        updated_at: millis_to_datetime(update.updated_at_millis).unwrap_or(received_at),
    })
}

fn order_from_cancel_update(
    update: &bluefin_api::models::OrderCancellationUpdate,
    received_at: chrono::DateTime<Utc>,
) -> Option<Order> {
    let remaining = parse_e9_decimal(&update.remaining_quantity_e9)?;
    // Cancellation updates do not include side/type/price; defaults are placeholders.
    Some(Order {
        order_hash: update.order_hash.clone(),
        market: update.symbol.clone(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: None,
        size: remaining,
        filled_size: Decimal::ZERO,
        status: OrderStatus::Cancelled,
        time_in_force: TimeInForce::Gtc,
        reduce_only: false,
        post_only: false,
        client_order_id: update.client_order_id.clone(),
        created_at: millis_to_datetime(update.created_at_millis).unwrap_or(received_at),
        updated_at: received_at,
    })
}

fn fill_from_trade(trade: &Trade) -> Option<bf_core::Fill> {
    let market = trade.symbol.clone().unwrap_or_default();
    let order_hash = trade.order_hash.clone().unwrap_or_default();
    if market.is_empty() || order_hash.is_empty() {
        return None;
    }
    let price = parse_e9_decimal(&trade.price_e9)?;
    let size = parse_e9_decimal(&trade.quantity_e9)?;
    let fee = trade
        .trading_fee_e9
        .as_ref()
        .and_then(|v| parse_e9_decimal(v))
        .unwrap_or(Decimal::ZERO);
    let fee_asset = trade
        .trading_fee_asset
        .clone()
        .unwrap_or_else(|| "USD".to_string());

    Some(bf_core::Fill {
        fill_id: trade.id.clone(),
        order_hash,
        market,
        side: map_side(trade.side)?,
        price,
        size,
        fee,
        fee_asset,
        filled_at: millis_to_datetime(trade.executed_at_millis).unwrap_or_else(Utc::now),
    })
}

fn parse_e9_decimal(raw: &str) -> Option<Decimal> {
    let value = Decimal::from_str(raw).ok()?;
    Some(value / Decimal::from(1_000_000_000u64))
}

fn millis_to_datetime(ms: i64) -> Option<chrono::DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms).single()
}

fn map_side(side: ApiTradeSide) -> Option<Side> {
    match side {
        ApiTradeSide::Long => Some(Side::Buy),
        ApiTradeSide::Short => Some(Side::Sell),
        ApiTradeSide::Unspecified => None,
    }
}

fn map_time_in_force(tif: ApiTimeInForce) -> TimeInForce {
    match tif {
        ApiTimeInForce::Ioc => TimeInForce::Ioc,
        ApiTimeInForce::Fok => TimeInForce::Fok,
        ApiTimeInForce::Gtt | ApiTimeInForce::Unspecified => TimeInForce::Gtc,
    }
}

fn map_order_type(order_type: ApiOrderType) -> OrderType {
    match order_type {
        ApiOrderType::Market
        | ApiOrderType::StopMarket
        | ApiOrderType::StopLossMarket
        | ApiOrderType::TakeProfitMarket => OrderType::Market,
        _ => OrderType::Limit,
    }
}

fn map_order_status(status: ApiOrderStatus) -> OrderStatus {
    match status {
        ApiOrderStatus::Open | ApiOrderStatus::Standby => OrderStatus::Open,
        ApiOrderStatus::PartiallyFilledOpen
        | ApiOrderStatus::PartiallyFilledCanceled
        | ApiOrderStatus::PartiallyFilledExpired => OrderStatus::Partial,
        ApiOrderStatus::Filled => OrderStatus::Filled,
        ApiOrderStatus::Cancelled => OrderStatus::Cancelled,
        ApiOrderStatus::Expired => OrderStatus::Expired,
        ApiOrderStatus::Unspecified => OrderStatus::Rejected,
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

/// Extract last_price_e9 from a raw WS payload (market stream).
pub fn extract_last_price_e9(payload: &serde_json::Value) -> Option<String> {
    let target = payload.get("payload").unwrap_or(payload);
    if let Some(value) = target.get("lastPriceE9") {
        if let Some(s) = value.as_str() {
            return Some(s.to_string());
        }
        if let Some(n) = value.as_i64() {
            return Some(n.to_string());
        }
    }
    if let Some(value) = target.get("marketPriceE9") {
        if let Some(s) = value.as_str() {
            return Some(s.to_string());
        }
        if let Some(n) = value.as_i64() {
            return Some(n.to_string());
        }
    }
    if let Some(value) = target.get("markPriceE9") {
        if let Some(s) = value.as_str() {
            return Some(s.to_string());
        }
        if let Some(n) = value.as_i64() {
            return Some(n.to_string());
        }
    }
    if let Some(value) = target.get("last_price_e9") {
        if let Some(s) = value.as_str() {
            return Some(s.to_string());
        }
        if let Some(n) = value.as_i64() {
            return Some(n.to_string());
        }
    }
    if let Some(value) = target.get("last_price") {
        if let Some(s) = value.as_str() {
            return Some(s.to_string());
        }
        if let Some(n) = value.as_f64() {
            return Some(format!("{}", n));
        }
    }
    None
}
