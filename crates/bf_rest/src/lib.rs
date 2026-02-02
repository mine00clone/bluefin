//! REST API client for Bluefin exchange.
//!
//! Provides methods for:
//! - Order creation
//! - Order cancellation
//! - Open orders retrieval
//! - Account information

use bf_auth::TokenManager;
use bf_config::AppConfig;
use bf_core::{CancelAck, CancelRequest, CoreError, Order, OrderRequest};
use std::sync::Arc;
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum RestError {
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Authentication error: {0}")]
    AuthError(#[from] bf_auth::AuthError),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },
}

impl From<RestError> for CoreError {
    fn from(e: RestError) -> Self {
        CoreError::SerializationError(e.to_string())
    }
}

/// REST API client
pub struct RestClient {
    config: AppConfig,
    token_manager: Arc<TokenManager>,
    client: reqwest::Client,
}

impl RestClient {
    pub fn new(config: AppConfig, token_manager: Arc<TokenManager>) -> Self {
        Self {
            config,
            token_manager,
            client: reqwest::Client::new(),
        }
    }

    /// Create a new order
    pub async fn create_order(&self, _request: OrderRequest) -> Result<Order, RestError> {
        debug!("Creating order");

        // TODO: Implement using bluefin-pro SDK
        // POST /api/v1/trade/orders

        Err(RestError::RequestFailed("Order creation not yet implemented - waiting for raw response analysis".to_string()))
    }

    /// Cancel orders
    pub async fn cancel_orders(&self, _request: CancelRequest) -> Result<CancelAck, RestError> {
        debug!("Cancelling orders");

        // TODO: Implement using bluefin-pro SDK
        // PUT /api/v1/trade/orders/cancel
        // Supports: single hash, batch hashes, or all for market

        Err(RestError::RequestFailed(
            "Order cancellation not yet implemented - waiting for raw response analysis".to_string(),
        ))
    }

    /// Get open orders
    pub async fn get_open_orders(&self, market: Option<&str>) -> Result<Vec<Order>, RestError> {
        debug!("Getting open orders for market: {:?}", market);

        // TODO: Implement using bluefin-pro SDK
        // GET /api/v1/trade/openOrders

        Err(RestError::RequestFailed("Open orders retrieval not yet implemented - waiting for raw response analysis".to_string()))
    }

    /// Get account information
    pub async fn get_account(&self) -> Result<serde_json::Value, RestError> {
        debug!("Getting account info");

        // TODO: Implement using bluefin-pro SDK
        // GET /api/v1/account

        Err(RestError::RequestFailed("Account info not yet implemented - waiting for raw response analysis".to_string()))
    }

    /// Get trade history
    pub async fn get_trades(
        &self,
        _market: Option<&str>,
        _start: Option<chrono::DateTime<chrono::Utc>>,
        _end: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<bf_core::TradeRecord>, RestError> {
        debug!("Getting trade history");

        // TODO: Implement using bluefin-pro SDK
        // GET /api/v1/account/trades

        Err(RestError::RequestFailed("Trade history not yet implemented - waiting for raw response analysis".to_string()))
    }

    /// Save raw response to file for analysis
    pub async fn save_raw_response(path: &str, response: &serde_json::Value) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(response)?;
        tokio::fs::write(path, content).await
    }
}

/// Trait implementation for OrderExecutor
impl bf_core::OrderExecutor for RestClient {
    fn create_order(&self, request: OrderRequest) -> bf_core::BoxFuture<'_, Result<Order, CoreError>> {
        Box::pin(async move {
            self.create_order(request).await.map_err(Into::into)
        })
    }

    fn create_orders(&self, requests: Vec<OrderRequest>) -> bf_core::BoxFuture<'_, Result<Vec<Order>, CoreError>> {
        Box::pin(async move {
            let mut orders = Vec::with_capacity(requests.len());
            for request in requests {
                orders.push(self.create_order(request).await.map_err(CoreError::from)?);
            }
            Ok(orders)
        })
    }

    fn cancel(&self, request: CancelRequest) -> bf_core::BoxFuture<'_, Result<CancelAck, CoreError>> {
        Box::pin(async move {
            self.cancel_orders(request).await.map_err(Into::into)
        })
    }
}
