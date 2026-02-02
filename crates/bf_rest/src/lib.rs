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
use chrono::TimeZone;
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
    trade_url: String,
}

impl RestClient {
    pub fn new(config: AppConfig, token_manager: Arc<TokenManager>) -> Self {
        let trade_url = config.rest.trade_url.clone();
        Self {
            config,
            token_manager,
            client: reqwest::Client::new(),
            trade_url,
        }
    }

    pub fn new_with_trade_url(
        config: AppConfig,
        token_manager: Arc<TokenManager>,
        trade_url: String,
    ) -> Self {
        Self {
            config,
            token_manager,
            client: reqwest::Client::new(),
            trade_url,
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

        let token = self.token_manager.get_token().await?;
        let url = format!("{}/api/v1/trade/openOrders", self.trade_url);
        let mut request = self.client.get(&url).bearer_auth(token);
        if let Some(symbol) = market {
            request = request.query(&[("symbol", symbol)]);
        }

        let response = request
            .send()
            .await
            .map_err(|e| RestError::RequestFailed(e.to_string()))?;
        let status = response.status();
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RestError::ParseError(e.to_string()))?;

        if !status.is_success() {
            return Err(RestError::ApiError {
                status: status.as_u16(),
                message: value.to_string(),
            });
        }

        let array = value.as_array().ok_or_else(|| {
            RestError::ParseError("openOrders response is not an array".to_string())
        })?;
        let mut orders = Vec::with_capacity(array.len());
        for item in array {
            orders.push(parse_open_order(item)?);
        }
        Ok(orders)
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

fn parse_open_order(value: &serde_json::Value) -> Result<Order, RestError> {
    let order_hash = value
        .get("orderHash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RestError::ParseError("orderHash missing".to_string()))?
        .to_string();
    let market = value
        .get("symbol")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RestError::ParseError("symbol missing".to_string()))?
        .to_string();
    let side = match value.get("side").and_then(|v| v.as_str()) {
        Some("LONG") => bf_core::Side::Buy,
        Some("SHORT") => bf_core::Side::Sell,
        Some(other) => {
            return Err(RestError::ParseError(format!(
                "unknown side: {}",
                other
            )))
        }
        None => return Err(RestError::ParseError("side missing".to_string())),
    };
    let order_type = match value.get("type").and_then(|v| v.as_str()) {
        Some("LIMIT") => bf_core::OrderType::Limit,
        Some("MARKET") => bf_core::OrderType::Market,
        Some(other) => {
            return Err(RestError::ParseError(format!(
                "unknown order type: {}",
                other
            )))
        }
        None => return Err(RestError::ParseError("type missing".to_string())),
    };
    let time_in_force = match value.get("timeInForce").and_then(|v| v.as_str()) {
        Some("GTT") => bf_core::TimeInForce::Gtc,
        Some("IOC") => bf_core::TimeInForce::Ioc,
        Some("FOK") => bf_core::TimeInForce::Fok,
        Some(other) => {
            return Err(RestError::ParseError(format!(
                "unknown timeInForce: {}",
                other
            )))
        }
        None => return Err(RestError::ParseError("timeInForce missing".to_string())),
    };
    let status = match value.get("status").and_then(|v| v.as_str()) {
        Some("OPEN") => bf_core::OrderStatus::Open,
        Some("PARTIAL") | Some("PARTIALLY_FILLED") => bf_core::OrderStatus::Partial,
        Some("FILLED") => bf_core::OrderStatus::Filled,
        Some("CANCELED") | Some("CANCELLED") => bf_core::OrderStatus::Cancelled,
        Some("EXPIRED") => bf_core::OrderStatus::Expired,
        Some("REJECTED") => bf_core::OrderStatus::Rejected,
        Some(other) => {
            return Err(RestError::ParseError(format!(
                "unknown status: {}",
                other
            )))
        }
        None => return Err(RestError::ParseError("status missing".to_string())),
    };

    let price = parse_e9_decimal(value.get("priceE9"))?;
    let size = parse_e9_decimal(value.get("quantityE9"))?;
    let filled_size = parse_e9_decimal(value.get("filledQuantityE9"))?;
    let created_at = parse_millis(value.get("orderTimeAtMillis"))
        .or_else(|| parse_millis(value.get("createdAtMillis")))
        .or_else(|| parse_millis(value.get("createTime")))
        .unwrap_or_else(chrono::Utc::now);
    let updated_at = parse_millis(value.get("updatedAtMillis")).unwrap_or(created_at);

    Ok(Order {
        order_hash,
        market,
        side,
        order_type,
        price: Some(price),
        size,
        filled_size,
        status,
        time_in_force,
        reduce_only: value.get("reduceOnly").and_then(|v| v.as_bool()).unwrap_or(false),
        post_only: value.get("postOnly").and_then(|v| v.as_bool()).unwrap_or(false),
        client_order_id: value
            .get("clientOrderId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        created_at,
        updated_at,
    })
}

fn parse_e9_decimal(value: Option<&serde_json::Value>) -> Result<rust_decimal::Decimal, RestError> {
    let raw = value.ok_or_else(|| RestError::ParseError("e9 value missing".to_string()))?;
    let raw_str = if let Some(s) = raw.as_str() {
        s.to_string()
    } else if let Some(n) = raw.as_i64() {
        n.to_string()
    } else if let Some(n) = raw.as_u64() {
        n.to_string()
    } else {
        return Err(RestError::ParseError("invalid e9 value".to_string()));
    };
    let parsed = rust_decimal::Decimal::from_str_exact(&raw_str)
        .map_err(|e| RestError::ParseError(e.to_string()))?;
    Ok(parsed / rust_decimal::Decimal::from(1_000_000_000u64))
}

fn parse_millis(value: Option<&serde_json::Value>) -> Option<chrono::DateTime<chrono::Utc>> {
    let raw = value?;
    if let Some(ms) = raw.as_i64() {
        return chrono::Utc.timestamp_millis_opt(ms).single();
    }
    if let Some(s) = raw.as_str() {
        if let Ok(ms) = s.parse::<i64>() {
            return chrono::Utc.timestamp_millis_opt(ms).single();
        }
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Some(dt.with_timezone(&chrono::Utc));
        }
    }
    None
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
