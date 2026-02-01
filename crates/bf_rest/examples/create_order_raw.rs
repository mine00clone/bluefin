//! Raw create order requester using bluefin-pro SDK
//!
//! Tests:
//! - POST /trade/orders
//!
//! Usage:
//!   BLUEFIN_ALLOW_TRADING=1 cargo run --example create_order_raw -p bf_rest

use bf_config::{AppConfig, Environment as BfEnvironment};
use bluefin_api::apis::{configuration::Configuration, trade_api::post_create_order};
use bluefin_api::models::{
    CreateOrderRequest, CreateOrderRequestSignedFields, LoginRequest, OrderSide,
    OrderTimeInForce, OrderType as SdkOrderType, SelfTradePreventionType,
};
use bluefin_pro::prelude::*;
use chrono::Utc;
use hex::FromHex;
use rand::random;
use rust_decimal::Decimal;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use sui_sdk_types::SignatureScheme;

const E9: u64 = 1_000_000_000;

fn to_sdk_env(env: &BfEnvironment) -> Environment {
    match env {
        BfEnvironment::Staging => Environment::Staging,
        BfEnvironment::Prod => Environment::Production,
    }
}

fn decimal_to_e9(value: Decimal) -> String {
    let scaled = value * Decimal::from(E9);
    scaled.trunc().to_string()
}

fn parse_decimal(var_name: &str, default: &str) -> anyhow::Result<Decimal> {
    let raw = std::env::var(var_name).unwrap_or_else(|_| default.to_string());
    raw.parse::<Decimal>()
        .map_err(|e| anyhow::anyhow!("Invalid {}: {}", var_name, e))
}

fn env_bool(var_name: &str) -> Option<bool> {
    std::env::var(var_name).ok().map(|raw| {
        raw == "1" || raw.eq_ignore_ascii_case("true") || raw.eq_ignore_ascii_case("yes")
    })
}

fn parse_stp(raw: &str) -> anyhow::Result<SelfTradePreventionType> {
    match raw.to_uppercase().as_str() {
        "TAKER" => Ok(SelfTradePreventionType::Taker),
        "MAKER" => Ok(SelfTradePreventionType::Maker),
        "BOTH" => Ok(SelfTradePreventionType::Both),
        "UNSPECIFIED" => Ok(SelfTradePreventionType::Unspecified),
        _ => Err(anyhow::anyhow!("Invalid self_trade_prevention_type: {}", raw)),
    }
}

fn parse_side(raw: &str) -> anyhow::Result<OrderSide> {
    match raw.to_uppercase().as_str() {
        "BUY" => Ok(OrderSide::Long),
        "SELL" => Ok(OrderSide::Short),
        _ => Err(anyhow::anyhow!("Invalid BLUEFIN_ORDER_SIDE: {}", raw)),
    }
}

fn parse_order_type(raw: &str) -> anyhow::Result<SdkOrderType> {
    match raw.to_uppercase().as_str() {
        "LIMIT" => Ok(SdkOrderType::Limit),
        "MARKET" => Ok(SdkOrderType::Market),
        _ => Err(anyhow::anyhow!("Invalid BLUEFIN_ORDER_TYPE: {}", raw)),
    }
}

fn parse_tif(raw: &str) -> anyhow::Result<OrderTimeInForce> {
    match raw.to_uppercase().as_str() {
        "GTC" => Ok(OrderTimeInForce::Gtt),
        "GTT" => Ok(OrderTimeInForce::Gtt),
        "IOC" => Ok(OrderTimeInForce::Ioc),
        "FOK" => Ok(OrderTimeInForce::Fok),
        _ => Err(anyhow::anyhow!("Invalid BLUEFIN_ORDER_TIF: {}", raw)),
    }
}

fn parse_u64_env(var_name: &str) -> Option<u64> {
    std::env::var(var_name)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
}

fn snap_to_step(value_e9: u64, step_e9: u64) -> u64 {
    if step_e9 == 0 {
        return value_e9;
    }
    (value_e9 / step_e9) * step_e9
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin Create Order Raw ===\n");

    if std::env::var("BLUEFIN_ALLOW_TRADING").ok().as_deref() != Some("1") {
        println!("Set BLUEFIN_ALLOW_TRADING=1 to allow order creation.");
        return Ok(());
    }

    let config = AppConfig::load("config")?;
    let environment = to_sdk_env(&config.env.name);

    // Load .env for secrets
    dotenvy::dotenv().ok();
    let use_test_keys = std::env::var("BLUEFIN_USE_TEST_KEYS").ok().as_deref() == Some("1");
    let is_staging = matches!(environment, Environment::Staging);
    let (private_key_hex, account_address) = if use_test_keys && is_staging {
        let test_keys = environment
            .test_keys()
            .expect("Test keys not available for this environment");
        (test_keys.private_key.to_string(), test_keys.address.to_string())
    } else {
        let secrets = AppConfig::load_secrets()?;
        let account_address = secrets
            .account_address
            .expect("BLUEFIN_ACCOUNT_ADDRESS not set in .env");
        (secrets.private_key, account_address)
    };

    if private_key_hex.starts_with("suiprivk") || private_key_hex.len() != 64 {
        println!("ERROR: Private key format issue.");
        println!("Expected 64-char hex private key.");
        return Ok(());
    }

    let market = std::env::var("BLUEFIN_ORDER_MARKET")
        .ok()
        .or_else(|| config.markets.symbols.first().cloned())
        .unwrap_or_else(|| "BTC-PERP".to_string());
    let side = parse_side(&std::env::var("BLUEFIN_ORDER_SIDE").unwrap_or_else(|_| "BUY".into()))?;
    let order_type = parse_order_type(&std::env::var("BLUEFIN_ORDER_TYPE").unwrap_or_else(|_| "LIMIT".into()))?;
    let size = parse_decimal("BLUEFIN_ORDER_SIZE", "1")?;
    let tif = parse_tif(&std::env::var("BLUEFIN_ORDER_TIF").unwrap_or_else(|_| "GTC".into()))?;
    let price = if matches!(order_type, SdkOrderType::Limit) {
        Some(parse_decimal("BLUEFIN_ORDER_PRICE", "0")?)
    } else {
        None
    };

    if matches!(order_type, SdkOrderType::Limit) && price.as_ref().map(|p| p.is_zero()).unwrap_or(true) {
        println!("LIMIT order requires BLUEFIN_ORDER_PRICE > 0");
        return Ok(());
    }

    println!("Account: {}", account_address);
    println!("Environment: {:?}", config.env.name);
    println!("Market: {}", market);

    // Create raw directory
    let raw_dir = Path::new("data/raw/rest");
    fs::create_dir_all(raw_dir)?;

    // Authenticate
    let login_request = LoginRequest::new(
        account_address.clone(),
        Utc::now().timestamp_millis(),
        auth::audience(environment).into(),
    );
    let private_key = PrivateKey::from_hex(&private_key_hex)?;
    let signature = login_request.signature(SignatureScheme::Ed25519, private_key)?;
    let token_response = login_request.authenticate(&signature, environment).await?;

    let trade_config = Configuration {
        base_path: trade::url(environment).into(),
        bearer_access_token: Some(token_response.access_token.clone()),
        ..Configuration::new()
    };

    let contracts_config = exchange::info::contracts_config(environment)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let markets = exchange::info::markets(environment)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let market_info = markets
        .iter()
        .find(|m| m.symbol == market)
        .ok_or_else(|| anyhow::anyhow!("Market not found: {}", market))?;

    let tick_size_e9: u64 = market_info.tick_size_e9.parse().unwrap_or(0);
    let step_size_e9: u64 = market_info.step_size_e9.parse().unwrap_or(0);
    let min_order_qty_e9: u64 = market_info.min_order_quantity_e9.parse().unwrap_or(0);

    let mut price_e9_u64 = price
        .map(decimal_to_e9)
        .unwrap_or_else(|| "0".to_string())
        .parse::<u64>()
        .unwrap_or(0);
    if matches!(order_type, SdkOrderType::Limit) {
        price_e9_u64 = snap_to_step(price_e9_u64, tick_size_e9);
        if price_e9_u64 == 0 {
            return Err(anyhow::anyhow!("Price snaps to 0 (tick size too large)"));
        }
    }

    let mut size_e9_u64 = decimal_to_e9(size).parse::<u64>().unwrap_or(0);
    size_e9_u64 = snap_to_step(size_e9_u64, step_size_e9);
    if size_e9_u64 < min_order_qty_e9 {
        return Err(anyhow::anyhow!(
            "Order size below minimum: {} < {} (e9)",
            size_e9_u64,
            min_order_qty_e9
        ));
    }

    let signed_at_millis = parse_u64_env("BLUEFIN_SIGNED_AT_MILLIS")
        .unwrap_or_else(|| Utc::now().timestamp_millis() as u64);
    let expires_at_millis = parse_u64_env("BLUEFIN_EXPIRES_AT_MILLIS")
        .unwrap_or(signed_at_millis + 6 * 60 * 1000);

    let leverage = std::env::var("BLUEFIN_ORDER_LEVERAGE")
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(config.orders.default_leverage);

    let signed_fields = CreateOrderRequestSignedFields {
        symbol: market.clone(),
        account_address: account_address.clone(),
        price_e9: price_e9_u64.to_string(),
        quantity_e9: size_e9_u64.to_string(),
        side,
        leverage_e9: ((leverage as u64) * E9).to_string(),
        is_isolated: false,
        salt: random::<u64>().to_string(),
        ids_id: contracts_config.ids_id,
        expires_at_millis: expires_at_millis as i64,
        signed_at_millis: signed_at_millis as i64,
    };

    let post_only = env_bool("BLUEFIN_ORDER_POST_ONLY").unwrap_or(config.orders.post_only);
    let reduce_only = env_bool("BLUEFIN_ORDER_REDUCE_ONLY").unwrap_or(config.orders.reduce_only);
    let stp_raw = std::env::var("BLUEFIN_SELF_TRADE_PREVENTION_TYPE")
        .ok()
        .unwrap_or_else(|| config.orders.self_trade_prevention_type.clone());
    let stp = parse_stp(&stp_raw)?;

    let order_request = CreateOrderRequest {
        signed_fields,
        client_order_id: None,
        r#type: order_type,
        reduce_only,
        post_only: if post_only { Some(true) } else { None },
        time_in_force: Some(tif),
        trigger_price_e9: None,
        self_trade_prevention_type: Some(stp),
        ..Default::default()
    };

    let private_key = PrivateKey::from_hex(&private_key_hex)?;
    let signed_request = order_request.sign(private_key, SignatureScheme::Ed25519)?;

    println!("\n--- Submitting Order ---");
    let response = post_create_order(&trade_config, signed_request.clone()).await;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = raw_dir.join(format!("create_order_{}.json", timestamp));
    let output = match &response {
        Ok(ok) => serde_json::json!({
            "request": signed_request,
            "response": ok,
            "timestamp": Utc::now().to_rfc3339(),
        }),
        Err(e) => {
            let mut err = serde_json::json!({
                "request": signed_request,
                "error": e.to_string(),
                "timestamp": Utc::now().to_rfc3339(),
            });
            if let bluefin_api::apis::Error::ResponseError(resp) = e {
                err["response_status"] = serde_json::json!(resp.status.as_u16());
                err["response_content"] = serde_json::json!(resp.content.clone());
            }
            err
        }
    };
    let mut file = File::create(&output_path)?;
    writeln!(file, "{}", serde_json::to_string_pretty(&output)?)?;
    println!("Saved to: {}", output_path.display());

    match response {
        Ok(ok) => {
            println!("Order submitted: {}", ok.order_hash);
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
