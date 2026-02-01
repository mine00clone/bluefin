//! SQLite persistence layer for Bluefin trading bot.
//!
//! Provides:
//! - Schema management and migrations
//! - Order, fill, and balance persistence
//! - Event storage for replay

use bf_core::{
    Balance, BalanceRepository, BoxFuture, CoreError, Fill, Order, OrderFilter,
    OrderRepository, TradeHistoryRepository,
};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    MigrationError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<StorageError> for CoreError {
    fn from(e: StorageError) -> Self {
        CoreError::SerializationError(e.to_string())
    }
}

/// SQLite storage implementation
pub struct SqliteStorage {
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    /// Create new storage with database path
    pub async fn new(db_path: &str) -> Result<Self, StorageError> {
        // Create directory if needed
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::MigrationError(e.to_string()))?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite:{}?mode=rwc", db_path))
            .await?;

        let storage = Self { pool };
        storage.run_migrations().await?;

        info!("SQLite storage initialized at {}", db_path);
        Ok(storage)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<(), StorageError> {
        sqlx::query(SCHEMA)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::MigrationError(e.to_string()))?;

        info!("Database migrations completed");
        Ok(())
    }

    /// Get the connection pool
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}

/// Database schema
const SCHEMA: &str = r#"
-- Orders table: current state of orders
CREATE TABLE IF NOT EXISTS orders (
    order_hash TEXT PRIMARY KEY,
    market TEXT NOT NULL,
    side TEXT NOT NULL,
    order_type TEXT NOT NULL,
    price TEXT,
    size TEXT NOT NULL,
    filled_size TEXT DEFAULT '0',
    status TEXT NOT NULL,
    time_in_force TEXT,
    reduce_only INTEGER DEFAULT 0,
    post_only INTEGER DEFAULT 0,
    client_order_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Order events table: WS events for replay
CREATE TABLE IF NOT EXISTS order_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_hash TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    received_at TEXT NOT NULL
);

-- Fills table: trade executions
CREATE TABLE IF NOT EXISTS fills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fill_id TEXT,
    order_hash TEXT NOT NULL,
    market TEXT NOT NULL,
    side TEXT NOT NULL,
    price TEXT NOT NULL,
    size TEXT NOT NULL,
    fee TEXT NOT NULL,
    fee_asset TEXT NOT NULL,
    filled_at TEXT NOT NULL,
    UNIQUE(fill_id)
);

-- Balances table: balance snapshots
CREATE TABLE IF NOT EXISTS balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset TEXT NOT NULL,
    total TEXT NOT NULL,
    available TEXT NOT NULL,
    locked TEXT NOT NULL,
    snapshot_at TEXT NOT NULL
);

-- Raw dumps index: metadata for raw response files
CREATE TABLE IF NOT EXISTS raw_dumps_index (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    endpoint TEXT NOT NULL,
    file_path TEXT NOT NULL,
    is_masked INTEGER DEFAULT 0,
    created_at TEXT NOT NULL
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_orders_market_status ON orders(market, status);
CREATE INDEX IF NOT EXISTS idx_order_events_order_hash ON order_events(order_hash);
CREATE INDEX IF NOT EXISTS idx_order_events_received_at ON order_events(received_at);
CREATE INDEX IF NOT EXISTS idx_fills_order_hash ON fills(order_hash);
CREATE INDEX IF NOT EXISTS idx_fills_filled_at ON fills(filled_at);
CREATE INDEX IF NOT EXISTS idx_fills_market ON fills(market);
CREATE INDEX IF NOT EXISTS idx_balances_asset ON balances(asset);
CREATE INDEX IF NOT EXISTS idx_balances_snapshot_at ON balances(snapshot_at);
"#;

// OrderRepository implementation
impl OrderRepository for SqliteStorage {
    fn save_order(&self, order: &Order) -> BoxFuture<'_, Result<(), CoreError>> {
        let order = order.clone();
        Box::pin(async move {
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO orders
                (order_hash, market, side, order_type, price, size, filled_size, status,
                 time_in_force, reduce_only, post_only, client_order_id, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&order.order_hash)
            .bind(&order.market)
            .bind(serde_json::to_string(&order.side).unwrap())
            .bind(serde_json::to_string(&order.order_type).unwrap())
            .bind(order.price.map(|p| p.to_string()))
            .bind(order.size.to_string())
            .bind(order.filled_size.to_string())
            .bind(serde_json::to_string(&order.status).unwrap())
            .bind(serde_json::to_string(&order.time_in_force).unwrap())
            .bind(order.reduce_only)
            .bind(order.post_only)
            .bind(&order.client_order_id)
            .bind(order.created_at.to_rfc3339())
            .bind(order.updated_at.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| CoreError::SerializationError(e.to_string()))?;

            Ok(())
        })
    }

    fn get_order(&self, order_hash: &str) -> BoxFuture<'_, Result<Option<Order>, CoreError>> {
        let _order_hash = order_hash.to_string();
        Box::pin(async move {
            // TODO: Implement full deserialization
            Ok(None)
        })
    }

    fn get_orders(&self, _filter: OrderFilter) -> BoxFuture<'_, Result<Vec<Order>, CoreError>> {
        Box::pin(async move {
            // TODO: Implement with filter
            Ok(vec![])
        })
    }

    fn get_open_orders(&self, _market: Option<&str>) -> BoxFuture<'_, Result<Vec<Order>, CoreError>> {
        Box::pin(async move {
            // TODO: Implement
            Ok(vec![])
        })
    }
}

// BalanceRepository implementation
impl BalanceRepository for SqliteStorage {
    fn save_balance(&self, balance: &Balance) -> BoxFuture<'_, Result<(), CoreError>> {
        let balance = balance.clone();
        Box::pin(async move {
            sqlx::query(
                r#"
                INSERT INTO balances (asset, total, available, locked, snapshot_at)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&balance.asset)
            .bind(balance.total.to_string())
            .bind(balance.available.to_string())
            .bind(balance.locked.to_string())
            .bind(balance.snapshot_at.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| CoreError::SerializationError(e.to_string()))?;

            Ok(())
        })
    }

    fn get_balance(&self, _asset: &str) -> BoxFuture<'_, Result<Option<Balance>, CoreError>> {
        Box::pin(async move {
            // TODO: Implement
            Ok(None)
        })
    }

    fn get_all_balances(&self) -> BoxFuture<'_, Result<Vec<Balance>, CoreError>> {
        Box::pin(async move {
            // TODO: Implement
            Ok(vec![])
        })
    }
}

// TradeHistoryRepository implementation
impl TradeHistoryRepository for SqliteStorage {
    fn save_fill(&self, fill: &Fill) -> BoxFuture<'_, Result<(), CoreError>> {
        let fill = fill.clone();
        Box::pin(async move {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO fills
                (fill_id, order_hash, market, side, price, size, fee, fee_asset, filled_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&fill.fill_id)
            .bind(&fill.order_hash)
            .bind(&fill.market)
            .bind(serde_json::to_string(&fill.side).unwrap())
            .bind(fill.price.to_string())
            .bind(fill.size.to_string())
            .bind(fill.fee.to_string())
            .bind(&fill.fee_asset)
            .bind(fill.filled_at.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| CoreError::SerializationError(e.to_string()))?;

            Ok(())
        })
    }

    fn get_fills_for_order(&self, _order_hash: &str) -> BoxFuture<'_, Result<Vec<Fill>, CoreError>> {
        Box::pin(async move {
            // TODO: Implement
            Ok(vec![])
        })
    }

    fn get_fills_in_range(
        &self,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
        _market: Option<&str>,
    ) -> BoxFuture<'_, Result<Vec<Fill>, CoreError>> {
        Box::pin(async move {
            // TODO: Implement
            Ok(vec![])
        })
    }
}
