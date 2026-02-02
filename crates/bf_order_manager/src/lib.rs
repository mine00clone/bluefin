//! Order state management (OMS) for Bluefin trading bot.
//!
//! Provides:
//! - Order state tracking (open, partial, filled, cancelled)
//! - WS event-driven state updates
//! - List/filter/sort operations
//! - REST reconciliation

use bf_core::{
    Order, OrderFilter, OrderRepository, OrderSortField, OrderStatus,
    OrderUpdateEvent,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Error)]
pub enum OmsError {
    #[error("Order not found: {0}")]
    OrderNotFound(String),
    #[error("Invalid state transition: {from} -> {to}")]
    InvalidTransition { from: String, to: String },
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Order Management System
pub struct OrderManager<R: OrderRepository> {
    /// In-memory order state
    orders: Arc<RwLock<HashMap<String, Order>>>,
    /// Repository for persistence
    repository: Arc<R>,
    /// Last REST reconciliation time
    last_reconcile: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl<R: OrderRepository> OrderManager<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            orders: Arc::new(RwLock::new(HashMap::new())),
            repository,
            last_reconcile: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize from REST snapshot (startup)
    pub async fn init_from_rest(&self, orders: Vec<Order>) -> Result<(), OmsError> {
        let mut state = self.orders.write().await;
        state.clear();

        for order in orders {
            self.repository
                .save_order(&order)
                .await
                .map_err(|e| OmsError::StorageError(e.to_string()))?;

            state.insert(order.order_hash.clone(), order);
        }

        *self.last_reconcile.write().await = Some(Utc::now());
        info!("Initialized {} orders from REST", state.len());
        Ok(())
    }

    /// Apply a WS order update (source of truth)
    pub async fn apply_update(&self, event: OrderUpdateEvent) -> Result<(), OmsError> {
        let mut state = self.orders.write().await;
        let order = event.order;

        // Validate state transition if order exists
        if let Some(existing) = state.get(&order.order_hash) {
            if !self.is_valid_transition(existing.status, order.status) {
                warn!(
                    "Invalid state transition for {}: {:?} -> {:?}",
                    order.order_hash, existing.status, order.status
                );
                return Err(OmsError::InvalidTransition {
                    from: existing.status.to_string(),
                    to: order.status.to_string(),
                });
            }
        }

        // Save to repository
        self.repository
            .save_order(&order)
            .await
            .map_err(|e| OmsError::StorageError(e.to_string()))?;

        // Update or remove based on status
        if order.is_terminal() {
            state.remove(&order.order_hash);
            debug!("Order {} reached terminal state: {:?}", order.order_hash, order.status);
        } else {
            state.insert(order.order_hash.clone(), order);
        }

        Ok(())
    }

    /// Get a single order by hash
    pub async fn get_order(&self, order_hash: &str) -> Option<Order> {
        let state = self.orders.read().await;
        state.get(order_hash).cloned()
    }

    /// Get all active orders
    pub async fn get_all_active(&self) -> Vec<Order> {
        let state = self.orders.read().await;
        state
            .values()
            .filter(|o| o.is_active())
            .cloned()
            .collect()
    }

    /// Get orders with filter and sort
    pub async fn get_orders(&self, filter: OrderFilter) -> Vec<Order> {
        let state = self.orders.read().await;
        let mut orders: Vec<Order> = state
            .values()
            .filter(|o| {
                // Apply market filter
                if let Some(ref market) = filter.market {
                    if &o.market != market {
                        return false;
                    }
                }

                // Apply status filter
                if let Some(ref statuses) = filter.status {
                    if !statuses.contains(&o.status) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Apply sorting
        if let Some(sort_field) = filter.sort_by {
            orders.sort_by(|a, b| {
                let cmp = match sort_field {
                    OrderSortField::CreatedAt => a.created_at.cmp(&b.created_at),
                    OrderSortField::UpdatedAt => a.updated_at.cmp(&b.updated_at),
                    OrderSortField::Price => a.price.cmp(&b.price),
                    OrderSortField::Size => a.size.cmp(&b.size),
                };
                if filter.sort_desc {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        }

        // Apply pagination
        let offset = filter.offset.unwrap_or(0);
        let limit = filter.limit.unwrap_or(usize::MAX);
        orders.into_iter().skip(offset).take(limit).collect()
    }

    /// Get open orders for a market
    pub async fn get_open_orders(&self, market: Option<&str>) -> Vec<Order> {
        self.get_orders(OrderFilter {
            market: market.map(String::from),
            status: Some(vec![OrderStatus::Open, OrderStatus::Partial]),
            ..Default::default()
        })
        .await
    }

    /// Reconcile with REST snapshot
    pub async fn reconcile(&self, rest_orders: Vec<Order>) -> Result<ReconcileResult, OmsError> {
        let mut state = self.orders.write().await;
        let mut added = 0;
        let mut updated = 0;
        let mut removed = 0;

        let rest_hashes: std::collections::HashSet<_> =
            rest_orders.iter().map(|o| o.order_hash.clone()).collect();

        // Update/add from REST
        for order in rest_orders {
            self.repository
                .save_order(&order)
                .await
                .map_err(|e| OmsError::StorageError(e.to_string()))?;

            if state.contains_key(&order.order_hash) {
                updated += 1;
            } else {
                added += 1;
            }
            state.insert(order.order_hash.clone(), order);
        }

        // Remove orders not in REST (they've been filled/cancelled)
        let to_remove: Vec<_> = state
            .keys()
            .filter(|hash| !rest_hashes.contains(*hash))
            .cloned()
            .collect();

        for hash in to_remove {
            state.remove(&hash);
            removed += 1;
        }

        *self.last_reconcile.write().await = Some(Utc::now());
        info!(
            "Reconciled orders: {} added, {} updated, {} removed",
            added, updated, removed
        );

        Ok(ReconcileResult {
            added,
            updated,
            removed,
        })
    }

    /// Check if state transition is valid
    fn is_valid_transition(&self, from: OrderStatus, to: OrderStatus) -> bool {
        use OrderStatus::*;
        matches!(
            (from, to),
            (Open, Partial)
                | (Open, Filled)
                | (Open, Cancelled)
                | (Open, Expired)
                | (Partial, Partial)
                | (Partial, Filled)
                | (Partial, Cancelled)
                | (Partial, Expired)
        )
    }

    /// Get last reconciliation time
    pub async fn last_reconcile_time(&self) -> Option<DateTime<Utc>> {
        *self.last_reconcile.read().await
    }

    /// Get count of active orders
    pub async fn active_count(&self) -> usize {
        let state = self.orders.read().await;
        state.values().filter(|o| o.is_active()).count()
    }
}

/// Result of reconciliation
#[derive(Debug, Clone)]
pub struct ReconcileResult {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}
