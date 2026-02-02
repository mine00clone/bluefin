use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PlanConfig {
    pub orders: Vec<PlanOrderConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlanOrderConfig {
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
