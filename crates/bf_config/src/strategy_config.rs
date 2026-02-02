use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct StrategyConfig {
    pub strategy: StrategySettingsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StrategySettingsConfig {
    pub id: String,
    pub enabled: bool,
}
