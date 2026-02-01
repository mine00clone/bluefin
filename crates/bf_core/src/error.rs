//! Core error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Invalid order state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Order not found: {0}")]
    OrderNotFound(String),

    #[error("Invalid price: {0}")]
    InvalidPrice(String),

    #[error("Invalid quantity: {0}")]
    InvalidQuantity(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),
}
