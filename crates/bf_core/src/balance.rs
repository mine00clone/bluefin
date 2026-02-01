//! Balance domain types.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Account balance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub asset: String,
    /// Total balance (including locked)
    pub total: Decimal,
    /// Available balance for trading
    pub available: Decimal,
    /// Locked balance (in orders or positions)
    pub locked: Decimal,
    pub snapshot_at: DateTime<Utc>,
}

impl Balance {
    pub fn new(asset: &str, total: Decimal, available: Decimal) -> Self {
        Self {
            asset: asset.to_string(),
            total,
            available,
            locked: total - available,
            snapshot_at: Utc::now(),
        }
    }
}

/// Position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub market: String,
    pub size: Decimal,
    pub side: PositionSide,
    pub entry_price: Decimal,
    pub mark_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub leverage: Decimal,
    pub margin_type: MarginType,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionSide {
    Long,
    Short,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarginType {
    Cross,
    Isolated,
}
