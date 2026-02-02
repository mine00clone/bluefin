//! Raw create order requester using bluefin-pro SDK
//!
//! Tests:
//! - POST /trade/orders
//!
//! Usage:
//!   cargo run --example create_order_raw -p bf_rest -- --run config/run/run_live.toml

use bf_config::{load_market_snapshot, load_run_config, Environment as BfEnvironment};
use bluefin_api::apis::{
    configuration::Configuration, exchange_api::get_market_ticker, trade_api::post_create_order,
};
use bluefin_api::models::{
    CreateOrderRequest, CreateOrderRequestSignedFields, LoginRequest, OrderSide,
    OrderTimeInForce, OrderType as SdkOrderType, SelfTradePreventionType,
};
use bluefin_pro::prelude::*;
use chrono::Utc;
use hex::FromHex;
use rand::random;
use rust_decimal::Decimal;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
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
        _ => Err(anyhow::anyhow!("Invalid side: {}", raw)),
    }
}

fn parse_order_type(raw: &str) -> anyhow::Result<SdkOrderType> {
    match raw.to_uppercase().as_str() {
        "LIMIT" => Ok(SdkOrderType::Limit),
        "MARKET" => Ok(SdkOrderType::Market),
        _ => Err(anyhow::anyhow!("Invalid order_type: {}", raw)),
    }
}

fn parse_tif(raw: &str) -> anyhow::Result<OrderTimeInForce> {
    match raw.to_uppercase().as_str() {
        "GTC" => Ok(OrderTimeInForce::Gtt),
        "GTT" => Ok(OrderTimeInForce::Gtt),
        "IOC" => Ok(OrderTimeInForce::Ioc),
        "FOK" => Ok(OrderTimeInForce::Fok),
        _ => Err(anyhow::anyhow!("Invalid tif: {}", raw)),
    }
}

fn snap_to_step(value_e9: u64, step_e9: u64) -> u64 {
    if step_e9 == 0 {
        return value_e9;
    }
    (value_e9 / step_e9) * step_e9
}

fn parse_run_arg() -> String {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--run" {
            if let Some(val) = args.next() {
                return val;
            }
        }
    }
    "config/run/run_live.toml".to_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bluefin Create Order Raw ===\n");

    let run_path = parse_run_arg();
    let runtime = load_run_config(Path::new(&run_path))?;
    let environment = to_sdk_env(&runtime.profile.env.name);

    // Load .env for secrets (secrets only)
    dotenvy::dotenv().ok();
    let use_test_keys = std::env::var("BLUEFIN_USE_TEST_KEYS").ok().as_deref() == Some("1");
    let is_staging = matches!(environment, Environment::Staging);
    let (private_key_hex, account_address) = if use_test_keys && is_staging {
        let test_keys = environment
            .test_keys()
            .expect("Test keys not available for this environment");
        (test_keys.private_key.to_string(), test_keys.address.to_string())
    } else {
        let secrets = bf_config::AppConfig::load_secrets()?;
        let account_address = secrets
            .account_address
            .expect("BLUEFIN_ACCOUNT_ADDRESS not set in .env");
        (secrets.private_key, account_address)
    };

    if private_key_hex.starts_with("suiprivk") || private_key_hex.len() != 64 {
        return Err(anyhow::anyhow!("Private key must be 64-char hex"));
    }

    let exchange_config = Configuration {
        base_path: runtime.profile.rest.exchange_url.clone(),
        ..Configuration::new()
    };

    let snapshot = load_market_snapshot(&runtime.markets_snapshot_path)?;

    let raw_dir = Path::new(&runtime.app.paths.raw_path).join("rest");
    fs::create_dir_all(&raw_dir)?;

    // Authenticate once
    let login_request = LoginRequest::new(
        account_address.clone(),
        Utc::now().timestamp_millis(),
        auth::audience(environment).into(),
    );
    let private_key = PrivateKey::from_hex(&private_key_hex)?;
    let signature = login_request.signature(SignatureScheme::Ed25519, private_key)?;
    let token_response = login_request.authenticate(&signature, environment).await?;

    let trade_config = Configuration {
        base_path: runtime.profile.rest.trade_url.clone(),
        bearer_access_token: Some(token_response.access_token.clone()),
        ..Configuration::new()
    };

    // Fetch contracts config once (ids_id)
    let contracts_config = exchange::info::contracts_config(environment)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    for plan_order in &runtime.plan.orders {
        let market = plan_order.market.clone();
        let meta = snapshot
            .get(&market)
            .ok_or_else(|| anyhow::anyhow!("Market not found in snapshot: {}", market))?;

        let side = parse_side(&plan_order.side)?;
        let order_type = parse_order_type(&plan_order.order_type)?;
        let tif = parse_tif(&plan_order.tif)?;

        let ticker = get_market_ticker(&exchange_config, &market).await?;
        let last_price_e9: u64 = ticker.last_price_e9.parse().unwrap_or(0);
        let signed_at_millis = ticker.updated_at_millis as u64;
        let expires_at_millis = signed_at_millis + 6 * 60 * 1000;

        let (mut price_e9_u64, price_is_set) = if matches!(order_type, SdkOrderType::Limit) {
            match plan_order.price_mode.as_str() {
                "absolute" => {
                    let p = plan_order
                        .price_e9
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("price_e9 required for absolute"))?;
                    (p.parse::<u64>()?, true)
                }
                "offset_bps" => {
                    let bps = plan_order
                        .price_bps
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("price_bps required for offset_bps"))?;
                    let bps_i64: i64 = bps.parse()?;
                    let last_price = Decimal::from(last_price_e9) / Decimal::from(E9);
                    let factor = Decimal::ONE + (Decimal::from(bps_i64) / Decimal::from(10_000));
                    let price = last_price * factor;
                    (decimal_to_e9(price).parse::<u64>()?, true)
                }
                other => return Err(anyhow::anyhow!("Invalid price_mode: {}", other)),
            }
        } else {
            (0, false)
        };

        let size_e9_str = match plan_order.size_mode.as_str() {
            "quantity" => plan_order
                .quantity
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("quantity required for size_mode=quantity"))?,
            other => return Err(anyhow::anyhow!("Invalid size_mode: {}", other)),
        };

        if matches!(order_type, SdkOrderType::Limit) && price_is_set {
            price_e9_u64 = snap_to_step(price_e9_u64, meta.tick_size_e9 as u64);
            if price_e9_u64 == 0 {
                return Err(anyhow::anyhow!("Price snaps to 0"));
            }
        }

        let mut size_e9_u64: u64 = size_e9_str.parse()?;
        size_e9_u64 = snap_to_step(size_e9_u64, meta.step_size_e9 as u64);
        if size_e9_u64 < meta.min_order_quantity_e9 as u64 {
            return Err(anyhow::anyhow!("Order size below minimum"));
        }

        let signed_fields = CreateOrderRequestSignedFields {
            symbol: market.clone(),
            account_address: account_address.clone(),
            price_e9: price_e9_u64.to_string(),
            quantity_e9: size_e9_u64.to_string(),
            side,
            leverage_e9: (10u64 * E9).to_string(),
            is_isolated: false,
            salt: random::<u64>().to_string(),
            ids_id: contracts_config.ids_id.clone(),
            expires_at_millis: expires_at_millis as i64,
            signed_at_millis: signed_at_millis as i64,
        };

        let stp = parse_stp("MAKER")?;
        let order_request = CreateOrderRequest {
            signed_fields,
            client_order_id: Some(plan_order.id.clone()),
            r#type: order_type,
            reduce_only: plan_order.reduce_only,
            post_only: if plan_order.post_only { Some(true) } else { None },
            time_in_force: Some(tif),
            trigger_price_e9: None,
            self_trade_prevention_type: Some(stp),
            ..Default::default()
        };

        let private_key = PrivateKey::from_hex(&private_key_hex)?;
        let signed_request = order_request.sign(private_key, SignatureScheme::Ed25519)?;

        let response = post_create_order(&trade_config, signed_request.clone()).await;
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let output_path = raw_dir.join(format!("create_order_{}_{}.json", plan_order.id, timestamp));
        let output = match &response {
            Ok(ok) => serde_json::json!({
                "request": signed_request,
                "response": ok,
                "timestamp": Utc::now().to_rfc3339(),
            }),
            Err(e) => serde_json::json!({
                "request": signed_request,
                "error": e.to_string(),
                "timestamp": Utc::now().to_rfc3339(),
            }),
        };
        let mut file = File::create(&output_path)?;
        writeln!(file, "{}", serde_json::to_string_pretty(&output)?)?;
        println!("Saved to: {}", output_path.display());

        if let Err(e) = response {
            return Err(e.into());
        }
    }

    Ok(())
}
