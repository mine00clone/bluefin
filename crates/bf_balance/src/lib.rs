//! Balance management for Bluefin trading bot.
//!
//! Provides:
//! - Balance tracking via REST snapshots and WS updates
//! - Available/total/locked balance calculations
//! - Balance history persistence

use bf_core::{Balance, BalanceRepository, BalanceUpdateEvent};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Error)]
pub enum BalanceError {
    #[error("Balance not found for asset: {0}")]
    NotFound(String),
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Balance manager for tracking account balances
pub struct BalanceManager<R: BalanceRepository> {
    /// In-memory balance state
    balances: Arc<RwLock<HashMap<String, Balance>>>,
    /// Repository for persistence
    repository: Arc<R>,
    /// Last REST sync time
    last_rest_sync: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// Last WS update time
    last_ws_update: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl<R: BalanceRepository> BalanceManager<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            balances: Arc::new(RwLock::new(HashMap::new())),
            repository,
            last_rest_sync: Arc::new(RwLock::new(None)),
            last_ws_update: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize balances from REST snapshot
    pub async fn init_from_rest(&self, balances: Vec<Balance>) -> Result<(), BalanceError> {
        let mut state = self.balances.write().await;
        state.clear();

        for balance in balances {
            // Save to repository
            self.repository
                .save_balance(&balance)
                .await
                .map_err(|e| BalanceError::StorageError(e.to_string()))?;

            state.insert(balance.asset.clone(), balance);
        }

        *self.last_rest_sync.write().await = Some(Utc::now());
        info!("Initialized {} balances from REST", state.len());
        Ok(())
    }

    /// Apply a WS balance update
    pub async fn apply_update(&self, event: BalanceUpdateEvent) -> Result<(), BalanceError> {
        let mut state = self.balances.write().await;

        // Save to repository
        self.repository
            .save_balance(&event.balance)
            .await
            .map_err(|e| BalanceError::StorageError(e.to_string()))?;

        state.insert(event.balance.asset.clone(), event.balance);
        *self.last_ws_update.write().await = Some(Utc::now());

        debug!("Applied balance update");
        Ok(())
    }

    /// Get balance for a specific asset
    pub async fn get_balance(&self, asset: &str) -> Option<Balance> {
        let state = self.balances.read().await;
        state.get(asset).cloned()
    }

    /// Get all balances
    pub async fn get_all_balances(&self) -> Vec<Balance> {
        let state = self.balances.read().await;
        state.values().cloned().collect()
    }

    /// Get available balance for trading
    pub async fn get_available(&self, asset: &str) -> Option<Decimal> {
        self.get_balance(asset).await.map(|b| b.available)
    }

    /// Get total balance (including locked)
    pub async fn get_total(&self, asset: &str) -> Option<Decimal> {
        self.get_balance(asset).await.map(|b| b.total)
    }

    /// Check if sufficient balance is available
    pub async fn has_sufficient(&self, asset: &str, amount: Decimal) -> bool {
        self.get_available(asset)
            .await
            .map(|available| available >= amount)
            .unwrap_or(false)
    }

    /// Get last sync timestamps for monitoring
    pub async fn get_sync_status(&self) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
        let rest = *self.last_rest_sync.read().await;
        let ws = *self.last_ws_update.read().await;
        (rest, ws)
    }
}
