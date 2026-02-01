//! Event types for WebSocket and state management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Balance, Fill, Order};

/// Event types received from WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AccountEvent {
    /// Order created, updated, or cancelled
    OrderUpdate(OrderUpdateEvent),
    /// Position updated
    PositionUpdate(PositionUpdateEvent),
    /// Balance updated
    BalanceUpdate(BalanceUpdateEvent),
    /// Trade executed (fill)
    TradeUpdate(TradeUpdateEvent),
}

/// Order update event from WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpdateEvent {
    pub order: Order,
    pub received_at: DateTime<Utc>,
}

/// Position update event from WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionUpdateEvent {
    pub market: String,
    pub payload: Value,
    pub received_at: DateTime<Utc>,
}

/// Balance update event from WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceUpdateEvent {
    pub balance: Balance,
    pub received_at: DateTime<Utc>,
}

/// Trade update event from WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeUpdateEvent {
    pub fill: Fill,
    pub received_at: DateTime<Utc>,
}

/// Market data event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MarketEvent {
    /// Order book update
    OrderBook(OrderBookEvent),
    /// Recent trade
    Trade(MarketTradeEvent),
    /// Ticker update
    Ticker(TickerEvent),
}

/// Order book update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookEvent {
    pub market: String,
    pub payload: Value,
    pub received_at: DateTime<Utc>,
}

/// Market trade event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTradeEvent {
    pub market: String,
    pub payload: Value,
    pub received_at: DateTime<Utc>,
}

/// Ticker update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerEvent {
    pub market: String,
    pub payload: Value,
    pub received_at: DateTime<Utc>,
}

/// Raw WebSocket message for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawWsMessage {
    pub stream_type: String,
    pub payload: Value,
    pub received_at: DateTime<Utc>,
}
