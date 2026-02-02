//! Strategy implementations (intent-only).

use bf_core::{
    Intent, IntentPriceSpec, IntentSizeSpec, MarketEvent, OrderType, Side, Strategy,
    StrategyContext, TimeInForce,
};
use uuid::Uuid;

/// Example strategy that emits a single intent on any market event.
pub struct ExampleStrategy {
    id: String,
    market: String,
    side: Side,
    order_type: OrderType,
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
            tif: TimeInForce::Ioc,
            post_only: false,
            reduce_only: false,
        }
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
            price_spec: IntentPriceSpec::Market,
            size_spec: IntentSizeSpec::Quantity(rust_decimal::Decimal::ONE),
            time_in_force: self.tif,
            post_only: self.post_only,
            reduce_only: self.reduce_only,
            cancel_after_ms: None,
            tags: vec!["example".to_string()],
            created_at: ctx.now,
        }]
    }
}
