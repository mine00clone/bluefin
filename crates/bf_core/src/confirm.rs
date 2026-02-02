//! Confirmation status for order execution.

use crate::OrderStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmStatus {
    Active,
    Filled,
    PartiallyFilled,
    Canceled,
    Expired,
    TimedOut,
}

impl From<OrderStatus> for ConfirmStatus {
    fn from(status: OrderStatus) -> Self {
        match status {
            OrderStatus::Open => ConfirmStatus::Active,
            OrderStatus::Partial => ConfirmStatus::PartiallyFilled,
            OrderStatus::Filled => ConfirmStatus::Filled,
            OrderStatus::Cancelled => ConfirmStatus::Canceled,
            OrderStatus::Expired => ConfirmStatus::Expired,
            OrderStatus::Rejected => ConfirmStatus::TimedOut,
        }
    }
}
