//! Intent normalization (strategy -> order request).

use bf_core::{Intent, IntentPriceSpec, IntentSizeSpec, MarketSnapshot, OrderRequest, OrderType};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::ExecError;

#[derive(Debug, Clone, Copy)]
pub enum PolicyMode {
    Snap,
    Reject,
}

#[derive(Debug, Clone, Copy)]
pub enum RoundingMode {
    Down,
    Up,
    Nearest,
}

#[derive(Debug, Clone)]
pub struct NormalizationPolicy {
    pub price_policy: PolicyMode,
    pub size_policy: PolicyMode,
    pub price_rounding: RoundingMode,
    pub size_rounding: RoundingMode,
    pub max_deviation_bps: i64,
}

fn decimal_to_e9(value: Decimal) -> Result<i128, ExecError> {
    let scaled = value * Decimal::from(1_000_000_000u64);
    scaled
        .round()
        .to_i128()
        .ok_or_else(|| ExecError::ValidationError("Failed to convert decimal to e9".to_string()))
}

fn e9_to_decimal(value: i128) -> Decimal {
    Decimal::from(value) / Decimal::from(1_000_000_000u64)
}

fn snap_value(value_e9: i128, step_e9: i128, mode: RoundingMode) -> i128 {
    if step_e9 <= 0 {
        return value_e9;
    }
    let remainder = value_e9 % step_e9;
    if remainder == 0 {
        return value_e9;
    }
    match mode {
        RoundingMode::Down => value_e9 - remainder,
        RoundingMode::Up => value_e9 + (step_e9 - remainder),
        RoundingMode::Nearest => {
            if remainder * 2 >= step_e9 {
                value_e9 + (step_e9 - remainder)
            } else {
                value_e9 - remainder
            }
        }
    }
}

fn enforce_policy(
    value_e9: i128,
    step_e9: i128,
    policy: PolicyMode,
    rounding: RoundingMode,
    max_deviation_bps: i64,
    label: &str,
) -> Result<i128, ExecError> {
    if step_e9 <= 0 {
        return Ok(value_e9);
    }
    if value_e9 % step_e9 == 0 {
        return Ok(value_e9);
    }
    match policy {
        PolicyMode::Reject => Err(ExecError::ValidationError(format!(
            "{} is not aligned with step",
            label
        ))),
        PolicyMode::Snap => {
            let snapped = snap_value(value_e9, step_e9, rounding);
            if value_e9 > 0 && max_deviation_bps > 0 {
                let original = e9_to_decimal(value_e9);
                let adjusted = e9_to_decimal(snapped);
                let diff = (original - adjusted).abs();
                let bps = (diff / original) * Decimal::from(10_000);
                let bps_i64 = bps
                    .round()
                    .to_i64()
                    .ok_or_else(|| ExecError::ValidationError("Failed to compute bps".to_string()))?;
                if bps_i64 > max_deviation_bps {
                    return Err(ExecError::ValidationError(format!(
                        "{} deviation {} bps exceeds max {} bps",
                        label, bps_i64, max_deviation_bps
                    )));
                }
            }
            Ok(snapped)
        }
    }
}

pub fn normalize_intent(
    intent: &Intent,
    snapshot: &MarketSnapshot,
    policy: &NormalizationPolicy,
    last_price: Option<Decimal>,
) -> Result<OrderRequest, ExecError> {
    let meta = snapshot.get(&intent.market).ok_or_else(|| {
        ExecError::ValidationError(format!(
            "Unknown market in snapshot: {}",
            intent.market
        ))
    })?;

    let price = match (&intent.price_spec, intent.order_type) {
        (IntentPriceSpec::Market, _) => None,
        (IntentPriceSpec::Absolute(p), _) => Some(*p),
        (IntentPriceSpec::OffsetBps(bps), OrderType::Limit) => {
            let last = last_price.ok_or_else(|| {
                ExecError::ValidationError(
                    "Missing last_price for offset_bps intent; check raw capture".to_string(),
                )
            })?;
            let factor = Decimal::ONE + (Decimal::from(*bps) / Decimal::from(10_000));
            Some(last * factor)
        }
        (IntentPriceSpec::OffsetBps(_), OrderType::Market) => None,
    };

    let size = match &intent.size_spec {
        IntentSizeSpec::Quantity(q) => *q,
    };

    let mut price = price;
    let mut size = size;

    if let Some(p) = price {
        let price_e9 = decimal_to_e9(p)?;
        let snapped = enforce_policy(
            price_e9,
            meta.tick_size_e9 as i128,
            policy.price_policy,
            policy.price_rounding,
            policy.max_deviation_bps,
            "price",
        )?;
        if snapped <= 0 {
            return Err(ExecError::ValidationError("Price snaps to 0".to_string()));
        }
        price = Some(e9_to_decimal(snapped));
    }

    let size_e9 = decimal_to_e9(size)?;
    let snapped_size = enforce_policy(
        size_e9,
        meta.step_size_e9 as i128,
        policy.size_policy,
        policy.size_rounding,
        policy.max_deviation_bps,
        "size",
    )?;
    let snapped_size = snapped_size.max(0);
    size = e9_to_decimal(snapped_size);

    if snapped_size < meta.min_order_quantity_e9 as i128 {
        return Err(ExecError::ValidationError(
            "Order size below minimum".to_string(),
        ));
    }

    Ok(OrderRequest {
        market: intent.market.clone(),
        side: intent.side,
        order_type: intent.order_type,
        price,
        size,
        time_in_force: intent.time_in_force,
        reduce_only: intent.reduce_only,
        post_only: intent.post_only,
        client_order_id: Some(format!("{}:{}", intent.strategy_id, intent.intent_id)),
    })
}
