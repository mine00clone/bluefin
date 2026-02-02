use crate::ConfigError;
use bf_core::MarketSnapshot;
use std::path::Path;

pub fn load_market_snapshot(path: &Path) -> Result<MarketSnapshot, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let snapshot: MarketSnapshot = serde_json::from_str(&content)
        .map_err(|e| ConfigError::EnvError(format!("Invalid market snapshot: {}", e)))?;
    Ok(snapshot)
}
