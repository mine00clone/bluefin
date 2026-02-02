//! Core domain types and traits for Bluefin trading bot.
//!
//! This crate defines:
//! - Domain types (Order, Fill, Balance, Event)
//! - Trait boundaries (OrderExecutor, OrderRepository, etc.)
//! - No I/O dependencies (pure domain logic)

pub mod error;
pub mod event;
pub mod order;
pub mod balance;
pub mod fill;
pub mod traits;
pub mod market_meta;
pub mod plan;
pub mod redact;

pub use error::CoreError;
pub use event::*;
pub use order::*;
pub use balance::*;
pub use fill::*;
pub use traits::*;
pub use market_meta::*;
pub use plan::*;
pub use redact::*;
