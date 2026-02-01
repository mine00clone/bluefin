//! Fill (trade execution) domain types.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::order::Side;

/// A single fill (partial or complete trade execution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub fill_id: String,
    pub order_hash: String,
    pub market: String,
    pub side: Side,
    pub price: Decimal,
    pub size: Decimal,
    pub fee: Decimal,
    pub fee_asset: String,
    pub filled_at: DateTime<Utc>,
}

impl Fill {
    /// Returns the notional value of this fill
    pub fn notional(&self) -> Decimal {
        self.price * self.size
    }
}

/// Trade history record (aggregated fill information)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub trade_id: String,
    pub order_hash: String,
    pub market: String,
    pub side: Side,
    pub price: Decimal,
    pub size: Decimal,
    pub fee: Decimal,
    pub realized_pnl: Option<Decimal>,
    pub executed_at: DateTime<Utc>,
}
