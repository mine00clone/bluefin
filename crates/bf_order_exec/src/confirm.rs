//! Confirmation helpers using WS account events.

use bf_core::{AccountEvent, ConfirmStatus, OrderStatus};
use tokio::sync::mpsc::Receiver;

pub async fn wait_confirm(
    receiver: &mut Receiver<AccountEvent>,
    order_hash: &str,
    timeout: std::time::Duration,
) -> ConfirmStatus {
    let result = tokio::time::timeout(timeout, async {
        while let Some(event) = receiver.recv().await {
            if let AccountEvent::OrderUpdate(update) = event {
                if update.order.order_hash == order_hash {
                    return ConfirmStatus::from(update.order.status);
                }
            }
        }
        ConfirmStatus::TimedOut
    })
    .await;

    match result {
        Ok(status) => status,
        Err(_) => ConfirmStatus::TimedOut,
    }
}

pub fn confirm_status_from_order_status(status: OrderStatus) -> ConfirmStatus {
    ConfirmStatus::from(status)
}
