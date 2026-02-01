//! Trait definitions for dependency injection and boundary separation.

use std::future::Future;
use std::pin::Pin;

use crate::{
    Balance, CancelRequest, CoreError, Fill, Order, OrderRequest, OrderStatus,
    AccountEvent, MarketEvent,
};

/// Result type for async trait methods
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Order execution interface (create/cancel orders)
pub trait OrderExecutor: Send + Sync {
    /// Create a new order
    fn create_order(&self, request: OrderRequest) -> BoxFuture<'_, Result<Order, CoreError>>;

    /// Create multiple orders in batch
    fn create_orders(&self, requests: Vec<OrderRequest>) -> BoxFuture<'_, Result<Vec<Order>, CoreError>>;

    /// Cancel order(s)
    fn cancel(&self, request: CancelRequest) -> BoxFuture<'_, Result<Vec<String>, CoreError>>;
}

/// Order repository interface (persistence)
pub trait OrderRepository: Send + Sync {
    /// Save or update an order
    fn save_order(&self, order: &Order) -> BoxFuture<'_, Result<(), CoreError>>;

    /// Get order by hash
    fn get_order(&self, order_hash: &str) -> BoxFuture<'_, Result<Option<Order>, CoreError>>;

    /// Get all orders matching filter
    fn get_orders(&self, filter: OrderFilter) -> BoxFuture<'_, Result<Vec<Order>, CoreError>>;

    /// Get open orders for a market
    fn get_open_orders(&self, market: Option<&str>) -> BoxFuture<'_, Result<Vec<Order>, CoreError>>;
}

/// Order filter for queries
#[derive(Debug, Clone, Default)]
pub struct OrderFilter {
    pub market: Option<String>,
    pub status: Option<Vec<OrderStatus>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<OrderSortField>,
    pub sort_desc: bool,
}

/// Fields to sort orders by
#[derive(Debug, Clone, Copy)]
pub enum OrderSortField {
    CreatedAt,
    UpdatedAt,
    Price,
    Size,
}

/// Balance repository interface
pub trait BalanceRepository: Send + Sync {
    /// Save balance snapshot
    fn save_balance(&self, balance: &Balance) -> BoxFuture<'_, Result<(), CoreError>>;

    /// Get latest balance for asset
    fn get_balance(&self, asset: &str) -> BoxFuture<'_, Result<Option<Balance>, CoreError>>;

    /// Get all balances
    fn get_all_balances(&self) -> BoxFuture<'_, Result<Vec<Balance>, CoreError>>;
}

/// Trade history repository interface
pub trait TradeHistoryRepository: Send + Sync {
    /// Save a fill
    fn save_fill(&self, fill: &Fill) -> BoxFuture<'_, Result<(), CoreError>>;

    /// Get fills for an order
    fn get_fills_for_order(&self, order_hash: &str) -> BoxFuture<'_, Result<Vec<Fill>, CoreError>>;

    /// Get fills in a time range
    fn get_fills_in_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
        market: Option<&str>,
    ) -> BoxFuture<'_, Result<Vec<Fill>, CoreError>>;
}

/// WebSocket stream source interface
pub trait StreamSource: Send + Sync {
    /// Subscribe to account events
    fn subscribe_account(&self) -> BoxFuture<'_, Result<tokio::sync::mpsc::Receiver<AccountEvent>, CoreError>>;

    /// Subscribe to market events
    fn subscribe_market(&self, markets: Vec<String>) -> BoxFuture<'_, Result<tokio::sync::mpsc::Receiver<MarketEvent>, CoreError>>;
}
