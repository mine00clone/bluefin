//! Market metadata types used for order normalization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMeta {
    pub market: String,
    pub tick_size_e9: i64,
    pub step_size_e9: i64,
    pub min_order_quantity_e9: i64,
    pub min_order_price_e9: i64,
    pub max_order_price_e9: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub markets: Vec<MarketMeta>,
}

impl MarketSnapshot {
    pub fn as_map(&self) -> HashMap<String, MarketMeta> {
        let mut map = HashMap::new();
        for meta in &self.markets {
            map.insert(meta.market.clone(), meta.clone());
        }
        map
    }

    pub fn get(&self, market: &str) -> Option<&MarketMeta> {
        self.markets.iter().find(|m| m.market == market)
    }
}
