//! Order execution (create/cancel) for Bluefin trading bot.
//!
//! Provides:
//! - Single and batch order creation
//! - Single, batch, and market-wide cancellation
//! - TIF support (GTC, IOC, FOK)
//! - ACK vs confirmation separation

mod bluefin_executor;

pub use bluefin_executor::BluefinOrderExecutor;

use bf_core::{CancelRequest, OrderExecutor, OrderRequest};
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum ExecError {
    #[error("Order creation failed: {0}")]
    CreateFailed(String),
    #[error("Order cancellation failed: {0}")]
    CancelFailed(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Order execution result with ACK/confirmation separation
#[derive(Debug, Clone)]
pub enum ExecResult {
    /// Order was accepted (ACK received)
    /// Final confirmation will come via WebSocket
    Acked {
        order_hash: String,
        ack_time: chrono::DateTime<chrono::Utc>,
    },
    /// Order was rejected immediately
    Rejected { reason: String },
}

/// Cancel execution result
#[derive(Debug, Clone)]
pub enum CancelResult {
    /// Cancel request was accepted (ACK received)
    /// Final confirmation will come via WebSocket
    Acked {
        order_hashes: Vec<String>,
        ack_time: chrono::DateTime<chrono::Utc>,
    },
    /// Cancel request failed
    Failed { reason: String },
}

/// Order executor service
pub struct OrderExecService<E: OrderExecutor> {
    executor: Arc<E>,
}

impl<E: OrderExecutor> OrderExecService<E> {
    pub fn new(executor: Arc<E>) -> Self {
        Self { executor }
    }

    /// Create a single order
    /// Returns ACK result; final confirmation comes via WS
    pub async fn create_order(&self, request: OrderRequest) -> Result<ExecResult, ExecError> {
        // Validate request
        self.validate_order(&request)?;

        info!(
            "Creating {} {} order for {} @ {:?}",
            request.time_in_force,
            request.side,
            request.market,
            request.price
        );

        match self.executor.create_order(request).await {
            Ok(order) => Ok(ExecResult::Acked {
                order_hash: order.order_hash,
                ack_time: chrono::Utc::now(),
            }),
            Err(e) => Ok(ExecResult::Rejected {
                reason: e.to_string(),
            }),
        }
    }

    /// Create multiple orders in batch
    pub async fn create_orders(&self, requests: Vec<OrderRequest>) -> Vec<Result<ExecResult, ExecError>> {
        let mut results = Vec::with_capacity(requests.len());

        for request in requests {
            results.push(self.create_order(request).await);
        }

        results
    }

    /// Cancel order(s)
    /// Returns ACK result; final confirmation comes via WS
    pub async fn cancel(&self, request: CancelRequest) -> Result<CancelResult, ExecError> {
        let description = match &request {
            CancelRequest::Single { market, order_hash } => {
                format!("single order {} on {}", order_hash, market)
            }
            CancelRequest::Batch { market, order_hashes } => {
                format!("{} orders on {}", order_hashes.len(), market)
            }
            CancelRequest::AllForMarket { market } => format!("all orders for {}", market),
        };

        info!("Cancelling {}", description);

        match self.executor.cancel(request).await {
            Ok(hashes) => Ok(CancelResult::Acked {
                order_hashes: hashes,
                ack_time: chrono::Utc::now(),
            }),
            Err(e) => Ok(CancelResult::Failed {
                reason: e.to_string(),
            }),
        }
    }

    /// Cancel a single order by hash
    pub async fn cancel_single(
        &self,
        market: &str,
        order_hash: &str,
    ) -> Result<CancelResult, ExecError> {
        self.cancel(CancelRequest::Single {
            market: market.to_string(),
            order_hash: order_hash.to_string(),
        })
        .await
    }

    /// Cancel multiple orders by hash
    pub async fn cancel_batch(
        &self,
        market: &str,
        order_hashes: Vec<String>,
    ) -> Result<CancelResult, ExecError> {
        self.cancel(CancelRequest::Batch {
            market: market.to_string(),
            order_hashes,
        })
        .await
    }

    /// Cancel all orders for a market
    pub async fn cancel_all_for_market(&self, market: &str) -> Result<CancelResult, ExecError> {
        self.cancel(CancelRequest::AllForMarket {
            market: market.to_string(),
        })
        .await
    }

    /// Validate order request before submission
    fn validate_order(&self, request: &OrderRequest) -> Result<(), ExecError> {
        if request.size <= rust_decimal::Decimal::ZERO {
            return Err(ExecError::ValidationError("Size must be positive".to_string()));
        }

        if request.order_type == bf_core::OrderType::Limit && request.price.is_none() {
            return Err(ExecError::ValidationError(
                "Limit orders require a price".to_string(),
            ));
        }

        if request.post_only && request.time_in_force != bf_core::TimeInForce::Gtc {
            warn!("Post-only orders typically use GTC time-in-force");
        }

        Ok(())
    }
}
