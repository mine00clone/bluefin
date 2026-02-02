//! Strategy intent types (strategy emits intent only).

use crate::{MarketEvent, OrderType, Side, TimeInForce};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Price specification for an intent.
#[derive(Debug, Clone)]
pub enum IntentPriceSpec {
    /// Absolute price.
    Absolute(Decimal),
    /// Offset from last price in bps.
    OffsetBps(i64),
    /// Market price (no limit).
    Market,
}

/// Size specification for an intent.
#[derive(Debug, Clone)]
pub enum IntentSizeSpec {
    /// Absolute quantity.
    Quantity(Decimal),
}

/// Intent emitted by strategy (no exchange-specific constraints applied yet).
#[derive(Debug, Clone)]
pub struct Intent {
    pub intent_id: String,
    pub strategy_id: String,
    pub market: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price_spec: IntentPriceSpec,
    pub size_spec: IntentSizeSpec,
    pub time_in_force: TimeInForce,
    pub post_only: bool,
    pub reduce_only: bool,
    pub cancel_after_ms: Option<u64>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Context provided to strategies.
#[derive(Debug, Clone)]
pub struct StrategyContext {
    pub now: DateTime<Utc>,
}

/// Strategy interface (event-driven only).
pub trait Strategy: Send {
    fn strategy_id(&self) -> &str;
    fn on_event(&mut self, ctx: &StrategyContext, event: MarketEvent) -> Vec<Intent>;
}
