//! Bluefin SDK implementation of the OrderExecutor trait.

use bf_auth::TokenManager;
use bf_core::{
    BoxFuture, CancelAck, CancelRequest, CoreError, Order, OrderExecutor, OrderRequest,
    OrderStatus, OrderType, Side, TimeInForce,
};
use bluefin_api::apis::configuration::Configuration;
use bluefin_api::apis::trade_api::{cancel_orders, post_create_order};
use bluefin_api::models::{
    CancelOrdersRequest, CreateOrderRequest, CreateOrderRequestSignedFields, OrderSide,
    OrderTimeInForce, OrderType as SdkOrderType, SelfTradePreventionType,
};
use bluefin_pro::prelude::*;
use chrono::{TimeDelta, Utc};
use hex::FromHex;
use rand::random;
use rust_decimal::Decimal;
use std::ops::Add;
use std::sync::Arc;
use sui_sdk_types::SignatureScheme;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// E9 multiplier constant (1 billion)
const E9: u64 = 1_000_000_000;

/// Bluefin SDK implementation of OrderExecutor
pub struct BluefinOrderExecutor {
    token_manager: Arc<TokenManager>,
    ids_id: Arc<RwLock<Option<String>>>,
    leverage_e9: String,
}

impl BluefinOrderExecutor {
    /// Create a new BluefinOrderExecutor
    pub fn new(token_manager: Arc<TokenManager>, leverage: u32) -> Self {
        info!("Using leverage (e9): {}", (leverage as u64) * E9);
        Self {
            token_manager,
            ids_id: Arc::new(RwLock::new(None)),
            leverage_e9: ((leverage as u64) * E9).to_string(),
        }
    }

    /// Get the IDS ID from exchange config (cached)
    async fn get_ids_id(&self) -> Result<String, CoreError> {
        // Check cache first
        {
            let cache = self.ids_id.read().await;
            if let Some(ids_id) = cache.as_ref() {
                return Ok(ids_id.clone());
            }
        }

        // Fetch from exchange
        let environment = self.token_manager.environment();
        let contracts_config = exchange::info::contracts_config(environment)
            .await
            .map_err(|e| CoreError::ApiError(format!("Failed to get exchange config: {}", e)))?;

        // Cache and return
        let ids_id = contracts_config.ids_id;
        {
            let mut cache = self.ids_id.write().await;
            *cache = Some(ids_id.clone());
        }

        Ok(ids_id)
    }

    /// Create API configuration with auth token
    async fn get_trade_config(&self) -> Result<Configuration, CoreError> {
        let token = self.token_manager.get_token().await.map_err(|e| {
            CoreError::AuthError(format!("Failed to get auth token: {}", e))
        })?;

        let environment = self.token_manager.environment();
        Ok(Configuration {
            base_path: trade::url(environment).into(),
            bearer_access_token: Some(token),
            ..Configuration::new()
        })
    }

    /// Convert internal Side to SDK OrderSide
    fn convert_side(side: Side) -> OrderSide {
        match side {
            Side::Buy => OrderSide::Long,
            Side::Sell => OrderSide::Short,
        }
    }

    /// Convert internal TimeInForce to SDK OrderTimeInForce
    /// Note: SDK uses GTT (Good-Til-Time) instead of GTC
    fn convert_tif(tif: TimeInForce) -> OrderTimeInForce {
        match tif {
            TimeInForce::Gtc => OrderTimeInForce::Gtt, // GTC maps to GTT in Bluefin
            TimeInForce::Ioc => OrderTimeInForce::Ioc,
            TimeInForce::Fok => OrderTimeInForce::Fok,
        }
    }

    /// Convert internal OrderType to SDK OrderType
    fn convert_order_type(order_type: OrderType) -> SdkOrderType {
        match order_type {
            OrderType::Limit => SdkOrderType::Limit,
            OrderType::Market => SdkOrderType::Market,
        }
    }

    /// Convert Decimal to e9 string format
    fn decimal_to_e9(value: Decimal) -> String {
        // Convert decimal to e9 format (multiply by 10^9)
        // Use string parsing to handle arbitrary precision
        let scaled = value * Decimal::from(E9);
        // Truncate to integer for e9 format
        scaled.trunc().to_string()
    }

    /// Build and sign a CreateOrderRequest
    async fn build_order_request(
        &self,
        request: &OrderRequest,
    ) -> Result<CreateOrderRequest, CoreError> {
        if request.size <= Decimal::ZERO {
            return Err(CoreError::InvalidQuantity(
                "Order size must be positive".to_string(),
            ));
        }
        if request.order_type == OrderType::Limit && request.price.is_none() {
            return Err(CoreError::InvalidPrice(
                "Limit order requires price".to_string(),
            ));
        }
        if request.post_only && request.time_in_force != TimeInForce::Gtc {
            return Err(CoreError::InvalidRequest(
                "post_only requires GTC time_in_force".to_string(),
            ));
        }

        let ids_id = self.get_ids_id().await?;
        let account_address = self.token_manager.account_address().to_string();

        // Build price (required for limit orders, use 0 for market)
        let price_e9 = match request.price {
            Some(p) => Self::decimal_to_e9(p),
            None => "0".to_string(), // Market orders don't use price
        };

        // Build signed fields
        let signed_fields = CreateOrderRequestSignedFields {
            symbol: request.market.clone(),
            account_address,
            price_e9,
            quantity_e9: Self::decimal_to_e9(request.size),
            side: Self::convert_side(request.side),
            leverage_e9: self.leverage_e9.clone(),
            is_isolated: false,
            salt: random::<u64>().to_string(),
            ids_id,
            expires_at_millis: Utc::now().add(TimeDelta::minutes(6)).timestamp_millis(),
            signed_at_millis: Utc::now().timestamp_millis(),
        };

        // Build order request
        let order_request = CreateOrderRequest {
            signed_fields,
            client_order_id: request.client_order_id.clone(),
            r#type: Self::convert_order_type(request.order_type),
            reduce_only: request.reduce_only,
            post_only: if request.post_only { Some(true) } else { None },
            time_in_force: Some(Self::convert_tif(request.time_in_force)),
            trigger_price_e9: None,
            self_trade_prevention_type: Some(SelfTradePreventionType::Maker),
            ..Default::default()
        };

        // Sign the request
        let private_key = PrivateKey::from_hex(self.token_manager.private_key_hex())
            .map_err(|e| CoreError::AuthError(format!("Invalid private key: {}", e)))?;

        let signed_request = order_request
            .sign(private_key, SignatureScheme::Ed25519)
            .map_err(|e| CoreError::ApiError(format!("Failed to sign order: {}", e)))?;

        Ok(signed_request)
    }

    /// Execute the actual order creation API call
    async fn execute_create_order(
        &self,
        signed_request: CreateOrderRequest,
    ) -> Result<Order, CoreError> {
        let config = self.get_trade_config().await?;

        debug!("Submitting order to Bluefin API");
        let response = post_create_order(&config, signed_request.clone())
            .await
            .map_err(|e| CoreError::ApiError(format!("Order creation failed: {}", e)))?;

        info!("Order submitted: {}", response.order_hash);

        // Build Order from the response
        let signed_fields = &signed_request.signed_fields;
        let side = match signed_fields.side {
            OrderSide::Long => Side::Buy,
            OrderSide::Short => Side::Sell,
            OrderSide::Unspecified => {
                return Err(CoreError::InvalidRequest(
                    "OrderSide is unspecified; check raw capture for signed_fields.side".to_string(),
                ));
            }
        };
        let order_type = match signed_request.r#type {
            SdkOrderType::Limit => OrderType::Limit,
            SdkOrderType::Market => OrderType::Market,
            _ => {
                return Err(CoreError::InvalidRequest(
                    "Unknown order type in signed request; check raw capture for type".to_string(),
                ));
            }
        };
        let time_in_force = signed_request
            .time_in_force
            .ok_or_else(|| {
                CoreError::InvalidRequest(
                    "time_in_force is missing; check raw capture for time_in_force".to_string(),
                )
            })?;
        let time_in_force = match time_in_force {
            OrderTimeInForce::Gtt => TimeInForce::Gtc,
            OrderTimeInForce::Ioc => TimeInForce::Ioc,
            OrderTimeInForce::Fok => TimeInForce::Fok,
            _ => {
                return Err(CoreError::InvalidRequest(
                    "Unknown time_in_force in signed request; check raw capture for time_in_force"
                        .to_string(),
                ));
            }
        };

        Ok(Order {
            order_hash: response.order_hash,
            market: signed_fields.symbol.clone(),
            side,
            order_type,
            price: None, // Price in e9, we'd need to convert back
            size: Decimal::ZERO, // Size in e9, we'd need to convert back
            filled_size: Decimal::ZERO,
            status: OrderStatus::Open,
            time_in_force,
            reduce_only: signed_request.reduce_only,
            post_only: signed_request.post_only.unwrap_or(false),
            client_order_id: signed_request.client_order_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Execute the actual cancel API call
    async fn execute_cancel(
        &self,
        market: &str,
        order_hashes: Option<Vec<String>>,
    ) -> Result<(), CoreError> {
        let config = self.get_trade_config().await?;

        let request = CancelOrdersRequest {
            symbol: market.to_string(),
            order_hashes: order_hashes.clone(),
        };

        debug!("Submitting cancel request to Bluefin API");
        cancel_orders(&config, request)
            .await
            .map_err(|e| CoreError::ApiError(format!("Cancel request failed: {}", e)))?;

        Ok(())
    }
}

impl OrderExecutor for BluefinOrderExecutor {
    fn create_order(&self, request: OrderRequest) -> BoxFuture<'_, Result<Order, CoreError>> {
        Box::pin(async move {
            let signed_request = self.build_order_request(&request).await?;
            self.execute_create_order(signed_request).await
        })
    }

    fn create_orders(
        &self,
        requests: Vec<OrderRequest>,
    ) -> BoxFuture<'_, Result<Vec<Order>, CoreError>> {
        Box::pin(async move {
            let mut orders = Vec::with_capacity(requests.len());

            for request in requests {
                let signed_request = self.build_order_request(&request).await?;
                let order = self.execute_create_order(signed_request).await?;
                orders.push(order);
            }

            Ok(orders)
        })
    }

    fn cancel(&self, request: CancelRequest) -> BoxFuture<'_, Result<CancelAck, CoreError>> {
        Box::pin(async move {
            match request {
                CancelRequest::Single { market, order_hash } => {
                    self.execute_cancel(&market, Some(vec![order_hash.clone()]))
                        .await?;
                    Ok(CancelAck::Single { market, order_hash })
                }
                CancelRequest::Batch { market, order_hashes } => {
                    self.execute_cancel(&market, Some(order_hashes.clone()))
                        .await?;
                    Ok(CancelAck::Batch { market, order_hashes })
                }
                CancelRequest::AllForMarket { market } => {
                    self.execute_cancel(&market, None).await?;
                    Ok(CancelAck::AllForMarket { market })
                }
            }
        })
    }
}

/// Extended cancel methods that include market context
impl BluefinOrderExecutor {
    /// Cancel a single order by hash with market context
    pub async fn cancel_order(&self, market: &str, order_hash: &str) -> Result<(), CoreError> {
        self.execute_cancel(market, Some(vec![order_hash.to_string()]))
            .await?;
        Ok(())
    }

    /// Cancel multiple orders by hash with market context
    pub async fn cancel_orders_batch(
        &self,
        market: &str,
        order_hashes: Vec<String>,
    ) -> Result<(), CoreError> {
        self.execute_cancel(market, Some(order_hashes)).await?;
        Ok(())
    }

    /// Cancel all orders for a market
    pub async fn cancel_all(&self, market: &str) -> Result<(), CoreError> {
        self.execute_cancel(market, None).await?;
        Ok(())
    }
}
