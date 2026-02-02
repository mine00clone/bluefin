use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct StrategyConfig {
    pub strategy: StrategySettingsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StrategySettingsConfig {
    pub id: String,
    pub enabled: bool,
    pub market: String,
    pub side: String,
    pub order_type: String,
    pub price_offset_bps: i64,
    pub size_quantity: String,
    pub tif: String,
    pub post_only: bool,
    pub reduce_only: bool,
}
