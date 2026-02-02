//! Plan model for multiple orders (Phase 1 minimal).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub orders: Vec<PlanOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanOrder {
    pub id: String,
    pub market: String,
    pub side: String,
    pub order_type: String,
    pub price_mode: String,
    pub price_bps: Option<String>,
    pub price_e9: Option<String>,
    pub size_mode: String,
    pub quantity: Option<String>,
    pub tif: String,
    pub post_only: bool,
    pub reduce_only: bool,
    pub cancel_after_ms: Option<u64>,
}
