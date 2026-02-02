//! Order domain types.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side {
    Buy,
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Limit,
    Market,
}

/// Time-in-force for orders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum TimeInForce {
    /// Good-Til-Cancel: order remains active until filled or cancelled
    #[default]
    Gtc,
    /// Immediate-Or-Cancel: fill immediately, cancel unfilled portion
    Ioc,
    /// Fill-Or-Kill: fill entire order immediately or cancel entirely
    Fok,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    /// Order is open and active
    Open,
    /// Order is partially filled
    Partial,
    /// Order is completely filled
    Filled,
    /// Order has been cancelled
    Cancelled,
    /// Order has expired
    Expired,
    /// Order was rejected
    Rejected,
}

/// Order request for creating new orders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub market: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub size: Decimal,
    pub time_in_force: TimeInForce,
    pub reduce_only: bool,
    pub post_only: bool,
    pub client_order_id: Option<String>,
}

impl OrderRequest {
    pub fn limit(market: &str, side: Side, price: Decimal, size: Decimal) -> Self {
        Self {
            market: market.to_string(),
            side,
            order_type: OrderType::Limit,
            price: Some(price),
            size,
            time_in_force: TimeInForce::Gtc,
            reduce_only: false,
            post_only: false,
            client_order_id: None,
        }
    }

    pub fn market(market: &str, side: Side, size: Decimal) -> Self {
        Self {
            market: market.to_string(),
            side,
            order_type: OrderType::Market,
            price: None,
            size,
            time_in_force: TimeInForce::Ioc,
            reduce_only: false,
            post_only: false,
            client_order_id: None,
        }
    }

    pub fn with_time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = tif;
        self
    }

    pub fn with_reduce_only(mut self, reduce_only: bool) -> Self {
        self.reduce_only = reduce_only;
        self
    }

    pub fn with_post_only(mut self, post_only: bool) -> Self {
        self.post_only = post_only;
        self
    }

    pub fn with_client_order_id(mut self, id: &str) -> Self {
        self.client_order_id = Some(id.to_string());
        self
    }
}

/// Order state representing the current state of an order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_hash: String,
    pub market: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub size: Decimal,
    pub filled_size: Decimal,
    pub status: OrderStatus,
    pub time_in_force: TimeInForce,
    pub reduce_only: bool,
    pub post_only: bool,
    pub client_order_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Order {
    /// Returns the remaining unfilled size
    pub fn remaining_size(&self) -> Decimal {
        self.size - self.filled_size
    }

    /// Returns true if the order is still active
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::Open | OrderStatus::Partial)
    }

    /// Returns true if the order is terminal (no more updates expected)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Expired | OrderStatus::Rejected
        )
    }
}

/// Cancel request for cancelling orders
#[derive(Debug, Clone)]
pub enum CancelRequest {
    /// Cancel a single order by hash
    Single { market: String, order_hash: String },
    /// Cancel multiple orders by hash
    Batch { market: String, order_hashes: Vec<String> },
    /// Cancel all orders for a market
    AllForMarket { market: String },
}

/// Cancel acknowledgement returned by executor
#[derive(Debug, Clone)]
pub enum CancelAck {
    /// Single order cancel accepted
    Single { market: String, order_hash: String },
    /// Batch cancel accepted
    Batch { market: String, order_hashes: Vec<String> },
    /// All orders for a market cancel accepted
    AllForMarket { market: String },
}

// Display implementations for logging
impl std::fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeInForce::Gtc => write!(f, "GTC"),
            TimeInForce::Ioc => write!(f, "IOC"),
            TimeInForce::Fok => write!(f, "FOK"),
        }
    }
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "BUY"),
            Side::Sell => write!(f, "SELL"),
        }
    }
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatus::Open => write!(f, "open"),
            OrderStatus::Partial => write!(f, "partial"),
            OrderStatus::Filled => write!(f, "filled"),
            OrderStatus::Cancelled => write!(f, "cancelled"),
            OrderStatus::Expired => write!(f, "expired"),
            OrderStatus::Rejected => write!(f, "rejected"),
        }
    }
}
