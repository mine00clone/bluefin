//! Configuration management for Bluefin trading bot.
//!
//! Loads configuration from:
//! - `config/*.toml` files
//! - `.env` file for secrets

use serde::Deserialize;
use std::path::{Path, PathBuf};
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

/// Order defaults configuration
#[derive(Debug, Clone, Deserialize)]
pub struct OrdersConfig {
    pub allow_trading: bool,
    pub default_leverage: u32,
    pub post_only: bool,
    pub reduce_only: bool,
    pub self_trade_prevention_type: String,
    pub create: OrdersCreateConfig,
    pub cancel: OrdersCancelConfig,
}

/// Order creation parameters for raw create script
#[derive(Debug, Clone, Deserialize)]
pub struct OrdersCreateConfig {
    pub market: String,
    pub side: String,
    pub order_type: String,
    pub time_in_force: String,
    pub size: String,
    pub price_offset_bps: i64,
    pub price_override: Option<String>,
}

/// Order cancellation parameters for raw cancel script
#[derive(Debug, Clone, Deserialize)]
pub struct OrdersCancelConfig {
    pub market: String,
    pub order_hashes: Vec<String>,
    pub cancel_all: bool,
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
    pub orders: OrdersConfig,
}

// --- Runtime config layering (run.toml -> app/profile/plan/markets) ---

/// run.toml includes
#[derive(Debug, Clone, Deserialize)]
pub struct RunIncludeConfig {
    pub app: PathBuf,
    pub profile: PathBuf,
    pub markets_snapshot: PathBuf,
    pub plan: PathBuf,
    pub universe: Option<PathBuf>,
    pub strategy: Option<PathBuf>,
    pub risk: Option<PathBuf>,
}

/// run.toml mode
#[derive(Debug, Clone, Deserialize)]
pub struct RunModeConfig {
    pub trading: String, // Live | Paper | DryRun
    pub requires_env: Option<String>,
}

/// run.toml root
#[derive(Debug, Clone, Deserialize)]
pub struct RunConfig {
    pub include: RunIncludeConfig,
    pub mode: RunModeConfig,
}

/// App-wide config (app.toml)
#[derive(Debug, Clone, Deserialize)]
pub struct AppPathsConfig {
    pub db_path: String,
    pub raw_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppLoggingConfig {
    pub level: String,
    pub json: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppExecutionConfig {
    pub mode: String,
    pub requires_env: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppNormalizationConfig {
    pub price_policy: String,
    pub size_policy: String,
    pub price_rounding: String,
    pub size_rounding: String,
    pub max_deviation_bps: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppSettingsConfig {
    pub paths: AppPathsConfig,
    pub logging: AppLoggingConfig,
    pub execution: AppExecutionConfig,
    pub normalization: AppNormalizationConfig,
}

/// Profile config (env-specific URLs)
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileConfig {
    pub env: EnvConfig,
    pub rest: RestConfig,
    pub ws: WsConfig,
}

/// Plan config (Phase 1 minimal)
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

/// Runtime config resolved from run.toml
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub run: RunConfig,
    pub app: AppSettingsConfig,
    pub profile: ProfileConfig,
    pub plan: PlanConfig,
    pub markets_snapshot_path: PathBuf,
}

fn load_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ConfigError> {
    let settings = config::Config::builder()
        .add_source(config::File::from(path))
        .build()?;
    settings.try_deserialize::<T>().map_err(ConfigError::from)
}

fn resolve_relative(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

/// Load run.toml and resolve all referenced configs into a RuntimeConfig.
pub fn load_run_config(path: &Path) -> Result<RuntimeConfig, ConfigError> {
    let run: RunConfig = load_toml(path)?;
    let base_dir = path
        .parent()
        .ok_or_else(|| ConfigError::EnvError("run.toml has no parent dir".to_string()))?;

    let app_path = resolve_relative(base_dir, &run.include.app);
    let profile_path = resolve_relative(base_dir, &run.include.profile);
    let plan_path = resolve_relative(base_dir, &run.include.plan);
    let markets_snapshot_path = resolve_relative(base_dir, &run.include.markets_snapshot);

    let app: AppSettingsConfig = load_toml(&app_path)?;
    let profile: ProfileConfig = load_toml(&profile_path)?;
    let plan: PlanConfig = load_toml(&plan_path)?;

    let runtime = RuntimeConfig {
        run,
        app,
        profile,
        plan,
        markets_snapshot_path,
    };

    validate_runtime_config(&runtime)?;
    Ok(runtime)
}

/// Validate runtime configuration for safety and existence.
pub fn validate_runtime_config(cfg: &RuntimeConfig) -> Result<(), ConfigError> {
    // Live safety gate
    if cfg.run.mode.trading.eq_ignore_ascii_case("live") {
        if let Some(requirement) = &cfg.run.mode.requires_env {
            let parts: Vec<&str> = requirement.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0];
                let expected = parts[1];
                let actual = std::env::var(key).unwrap_or_default();
                if actual != expected {
                    return Err(ConfigError::EnvError(format!(
                        "Live run requires {}={}",
                        key, expected
                    )));
                }
            }
        }
    }

    if !cfg.markets_snapshot_path.exists() {
        return Err(ConfigError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "markets_snapshot not found: {}",
                cfg.markets_snapshot_path.display()
            ),
        )));
    }

    Ok(())
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
