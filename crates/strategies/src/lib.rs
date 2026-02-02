//! Strategy implementations (intent-only).

use bf_core::{
    Intent, IntentPriceSpec, IntentSizeSpec, MarketEvent, OrderType, Side, Strategy,
    StrategyContext, TimeInForce,
};
use bf_config::StrategyConfig;
use uuid::Uuid;

/// Example strategy that emits a single intent on any market event.
pub struct ExampleStrategy {
    id: String,
    market: String,
    side: Side,
    order_type: OrderType,
    price_offset_bps: i64,
    size_quantity: rust_decimal::Decimal,
    tif: TimeInForce,
    post_only: bool,
    reduce_only: bool,
}

impl ExampleStrategy {
    pub fn new(market: &str) -> Self {
        Self {
            id: "example".to_string(),
            market: market.to_string(),
            side: Side::Buy,
            order_type: OrderType::Market,
            price_offset_bps: 0,
            size_quantity: rust_decimal::Decimal::ONE,
            tif: TimeInForce::Ioc,
            post_only: false,
            reduce_only: false,
        }
    }

    pub fn from_config(config: &StrategyConfig) -> anyhow::Result<Self> {
        let s = &config.strategy;
        let side = match s.side.as_str() {
            "BUY" => Side::Buy,
            "SELL" => Side::Sell,
            other => {
                return Err(anyhow::anyhow!("Invalid strategy.side: {}", other));
            }
        };
        let order_type = match s.order_type.as_str() {
            "LIMIT" => OrderType::Limit,
            "MARKET" => OrderType::Market,
            other => {
                return Err(anyhow::anyhow!("Invalid strategy.order_type: {}", other));
            }
        };
        let tif = match s.tif.as_str() {
            "GTC" => TimeInForce::Gtc,
            "IOC" => TimeInForce::Ioc,
            "FOK" => TimeInForce::Fok,
            other => {
                return Err(anyhow::anyhow!("Invalid strategy.tif: {}", other));
            }
        };
        let size_quantity = s
            .size_quantity
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| anyhow::anyhow!("Invalid strategy.size_quantity: {}", e))?;

        Ok(Self {
            id: s.id.clone(),
            market: s.market.clone(),
            side,
            order_type,
            price_offset_bps: s.price_offset_bps,
            size_quantity,
            tif,
            post_only: s.post_only,
            reduce_only: s.reduce_only,
        })
    }
}

impl Strategy for ExampleStrategy {
    fn strategy_id(&self) -> &str {
        &self.id
    }

    fn on_event(&mut self, ctx: &StrategyContext, event: MarketEvent) -> Vec<Intent> {
        let market = match event {
            MarketEvent::OrderBook(e) => e.market,
            MarketEvent::Trade(e) => e.market,
            MarketEvent::Ticker(e) => e.market,
        };
        if market != self.market {
            return Vec::new();
        }

        vec![Intent {
            intent_id: Uuid::new_v4().to_string(),
            strategy_id: self.id.clone(),
            market: market.to_string(),
            side: self.side,
            order_type: self.order_type,
            price_spec: if self.order_type == OrderType::Market {
                IntentPriceSpec::Market
            } else {
                IntentPriceSpec::OffsetBps(self.price_offset_bps)
            },
            size_spec: IntentSizeSpec::Quantity(self.size_quantity),
            time_in_force: self.tif,
            post_only: self.post_only,
            reduce_only: self.reduce_only,
            cancel_after_ms: None,
            tags: vec!["example".to_string()],
            created_at: ctx.now,
        }]
    }
}
