//! Trade and order history management for Bluefin trading bot.
//!
//! Provides:
//! - Fill/trade history tracking
//! - Unfilled order history
//! - Time-range queries

use bf_core::{Fill, TradeHistoryRepository};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Query error: {0}")]
    QueryError(String),
}

/// History manager for trades and orders
pub struct HistoryManager<R: TradeHistoryRepository> {
    repository: Arc<R>,
}

impl<R: TradeHistoryRepository> HistoryManager<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Record a new fill
    pub async fn record_fill(&self, fill: Fill) -> Result<(), HistoryError> {
        self.repository
            .save_fill(&fill)
            .await
            .map_err(|e| HistoryError::StorageError(e.to_string()))?;

        debug!("Recorded fill {} for order {}", fill.fill_id, fill.order_hash);
        Ok(())
    }

    /// Get fills for a specific order
    pub async fn get_fills_for_order(&self, order_hash: &str) -> Result<Vec<Fill>, HistoryError> {
        self.repository
            .get_fills_for_order(order_hash)
            .await
            .map_err(|e| HistoryError::StorageError(e.to_string()))
    }

    /// Get fills in a time range
    pub async fn get_fills_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        market: Option<&str>,
    ) -> Result<Vec<Fill>, HistoryError> {
        self.repository
            .get_fills_in_range(start, end, market)
            .await
            .map_err(|e| HistoryError::StorageError(e.to_string()))
    }

    /// Get fills for the last N hours
    pub async fn get_recent_fills(
        &self,
        hours: i64,
        market: Option<&str>,
    ) -> Result<Vec<Fill>, HistoryError> {
        let end = Utc::now();
        let start = end - Duration::hours(hours);
        self.get_fills_in_range(start, end, market).await
    }

    /// Get fills for today
    pub async fn get_today_fills(&self, market: Option<&str>) -> Result<Vec<Fill>, HistoryError> {
        let now = Utc::now();
        let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let start = DateTime::from_naive_utc_and_offset(start, Utc);
        self.get_fills_in_range(start, now, market).await
    }

    /// Calculate total volume in a time range
    pub async fn calculate_volume(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        market: Option<&str>,
    ) -> Result<rust_decimal::Decimal, HistoryError> {
        let fills = self.get_fills_in_range(start, end, market).await?;
        let volume = fills.iter().map(|f| f.notional()).sum();
        Ok(volume)
    }

    /// Calculate total fees in a time range
    pub async fn calculate_fees(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        market: Option<&str>,
    ) -> Result<rust_decimal::Decimal, HistoryError> {
        let fills = self.get_fills_in_range(start, end, market).await?;
        let fees = fills.iter().map(|f| f.fee).sum();
        Ok(fees)
    }
}

/// Query builder for history queries
#[derive(Debug, Clone, Default)]
pub struct HistoryQuery {
    pub market: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl HistoryQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn market(mut self, market: &str) -> Self {
        self.market = Some(market.to_string());
        self
    }

    pub fn time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start = Some(start);
        self.end = Some(end);
        self
    }

    pub fn last_hours(mut self, hours: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::hours(hours);
        self.start = Some(start);
        self.end = Some(end);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}
