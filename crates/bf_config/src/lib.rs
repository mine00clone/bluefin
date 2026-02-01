//! Configuration management for Bluefin trading bot.
//!
//! Loads configuration from:
//! - `config/*.toml` files
//! - `.env` file for secrets

use serde::Deserialize;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load config: {0}")]
    LoadError(#[from] config::ConfigError),
    #[error("Environment variable error: {0}")]
    EnvError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Environment configuration (staging or production)
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Staging,
    Prod,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Staging => "staging",
            Environment::Prod => "prod",
        }
    }
}

/// REST API configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RestConfig {
    pub exchange_url: String,
    pub trade_url: String,
    pub auth_url: String,
}

/// WebSocket configuration
#[derive(Debug, Clone, Deserialize)]
pub struct WsConfig {
    pub market_url: String,
    pub account_url: String,
}

/// Market configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MarketsConfig {
    pub symbols: Vec<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub db_path: String,
    pub raw_path: String,
}

/// Sui wallet authentication secrets (from .env)
#[derive(Debug, Clone)]
pub struct AuthSecrets {
    /// Sui wallet private key (hex format, required)
    pub private_key: String,
    /// Sui account address (optional, can be derived from private key)
    pub account_address: Option<String>,
}

/// Environment settings
#[derive(Debug, Clone, Deserialize)]
pub struct EnvConfig {
    pub name: Environment,
}

/// Main application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub env: EnvConfig,
    pub rest: RestConfig,
    pub ws: WsConfig,
    pub markets: MarketsConfig,
    pub storage: StorageConfig,
}

impl AppConfig {
    /// Load configuration from files and environment.
    ///
    /// Reads from:
    /// - `config/default.toml`
    /// - `.env` for secrets
    pub fn load<P: AsRef<Path>>(config_dir: P) -> Result<Self, ConfigError> {
        let config_dir = config_dir.as_ref();

        let settings = config::Config::builder()
            .add_source(config::File::from(config_dir.join("default.toml")))
            .build()?;

        let config: AppConfig = settings.try_deserialize()?;
        Ok(config)
    }

    /// Load authentication secrets from environment variables.
    ///
    /// Required:
    /// - `BLUEFIN_PRIVATE_KEY`: Sui wallet private key (hex format)
    ///
    /// Optional:
    /// - `BLUEFIN_ACCOUNT_ADDRESS`: Sui account address (can be derived from private key)
    pub fn load_secrets() -> Result<AuthSecrets, ConfigError> {
        // Load .env file if it exists
        let _ = dotenvy::dotenv();

        let private_key = std::env::var("BLUEFIN_PRIVATE_KEY")
            .map_err(|_| ConfigError::EnvError("BLUEFIN_PRIVATE_KEY not set".to_string()))?;

        if private_key.is_empty() {
            return Err(ConfigError::EnvError("BLUEFIN_PRIVATE_KEY is empty".to_string()));
        }

        let account_address = std::env::var("BLUEFIN_ACCOUNT_ADDRESS")
            .ok()
            .filter(|s| !s.is_empty());

        Ok(AuthSecrets {
            private_key,
            account_address,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_as_str() {
        assert_eq!(Environment::Staging.as_str(), "staging");
        assert_eq!(Environment::Prod.as_str(), "prod");
    }
}
