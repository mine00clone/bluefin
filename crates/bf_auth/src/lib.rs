//! Authentication and token management for Bluefin API.
//!
//! Handles:
//! - Token acquisition via /auth/v2/token
//! - Automatic token refresh (5-minute expiry)

use bluefin_api::apis::{auth_api::auth_token_refresh_put, configuration::Configuration};
use bluefin_api::models::{LoginRequest, RefreshTokenRequest};
use bluefin_pro::prelude::*;
use chrono::{DateTime, Duration, Utc};
use hex::FromHex;
use std::sync::Arc;
use sui_sdk_types::SignatureScheme;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
    #[error("Token refresh failed: {0}")]
    RefreshFailed(String),
    #[error("No valid token available")]
    NoToken,
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("Invalid private key: {0}")]
    InvalidKey(String),
}

/// Authentication token with expiry
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
}

impl AuthToken {
    /// Check if token is expired or about to expire (within 30 seconds)
    pub fn is_expired(&self) -> bool {
        Utc::now() + Duration::seconds(30) >= self.expires_at
    }
}

/// Re-export Environment for convenience
pub use bluefin_pro::prelude::Environment as BluefinEnvironment;

/// Token manager for automatic token refresh
pub struct TokenManager {
    token: Arc<RwLock<Option<AuthToken>>>,
    private_key_hex: String,
    account_address: String,
    environment: BluefinEnvironment,
}

impl TokenManager {
    /// Create a new TokenManager with the given credentials
    pub fn new(
        private_key_hex: String,
        account_address: String,
        environment: BluefinEnvironment,
    ) -> Result<Self, AuthError> {
        // Validate private key format
        if private_key_hex.len() != 64 {
            return Err(AuthError::InvalidKey(format!(
                "Private key must be 64 hex characters, got {}",
                private_key_hex.len()
            )));
        }

        // Verify the key is valid hex
        if PrivateKey::from_hex(&private_key_hex).is_err() {
            return Err(AuthError::InvalidKey(
                "Private key is not valid hexadecimal".to_string(),
            ));
        }

        Ok(Self {
            token: Arc::new(RwLock::new(None)),
            private_key_hex,
            account_address,
            environment,
        })
    }

    /// Create a TokenManager using SDK test keys (staging only)
    pub fn with_test_keys(environment: BluefinEnvironment) -> Result<Self, AuthError> {
        let test_keys = environment.test_keys().ok_or_else(|| {
            AuthError::InvalidKey("Test keys not available for production environment".to_string())
        })?;

        Ok(Self {
            token: Arc::new(RwLock::new(None)),
            private_key_hex: test_keys.private_key.to_string(),
            account_address: test_keys.address.to_string(),
            environment,
        })
    }

    /// Get the account address
    pub fn account_address(&self) -> &str {
        &self.account_address
    }

    /// Get the environment
    pub fn environment(&self) -> BluefinEnvironment {
        self.environment
    }

    /// Get the private key (for order signing)
    pub fn private_key_hex(&self) -> &str {
        &self.private_key_hex
    }

    /// Get a valid token, refreshing if necessary
    pub async fn get_token(&self) -> Result<String, AuthError> {
        // Check if we have a valid token
        {
            let token = self.token.read().await;
            if let Some(t) = token.as_ref() {
                if !t.is_expired() {
                    return Ok(t.access_token.clone());
                }
            }
        }

        // Need to refresh or acquire new token
        self.refresh_or_acquire().await
    }

    /// Force refresh the token
    async fn refresh_or_acquire(&self) -> Result<String, AuthError> {
        let mut token = self.token.write().await;

        // Double-check after acquiring write lock
        if let Some(t) = token.as_ref() {
            if !t.is_expired() {
                return Ok(t.access_token.clone());
            }

            // Try refresh first if we have a refresh token
            if let Some(refresh_token) = &t.refresh_token {
                match self.refresh_token(refresh_token).await {
                    Ok(new_token) => {
                        info!("Token refreshed successfully");
                        let access = new_token.access_token.clone();
                        *token = Some(new_token);
                        return Ok(access);
                    }
                    Err(e) => {
                        warn!("Token refresh failed: {}, acquiring new token", e);
                    }
                }
            }
        }

        // Acquire new token
        let new_token = self.acquire_token().await?;
        info!("New token acquired successfully");
        let access = new_token.access_token.clone();
        *token = Some(new_token);
        Ok(access)
    }

    /// Acquire a new token via /auth/v2/token
    async fn acquire_token(&self) -> Result<AuthToken, AuthError> {
        debug!("Acquiring new auth token for {}", self.account_address);

        // Create login request
        let login_request = LoginRequest::new(
            self.account_address.clone(),
            Utc::now().timestamp_millis(),
            auth::audience(self.environment).into(),
        );

        // Sign the request
        let private_key = PrivateKey::from_hex(&self.private_key_hex)
            .map_err(|e| AuthError::InvalidKey(e.to_string()))?;

        let signature = login_request
            .signature(SignatureScheme::Ed25519, private_key)
            .map_err(|e| AuthError::AuthFailed(format!("Failed to sign login request: {}", e)))?;

        // Authenticate
        let response = login_request
            .authenticate(&signature, self.environment)
            .await
            .map_err(|e| AuthError::AuthFailed(format!("Authentication request failed: {}", e)))?;

        // Calculate expiry (access token is valid for N seconds)
        let expires_at =
            Utc::now() + Duration::seconds(response.access_token_valid_for_seconds as i64);

        Ok(AuthToken {
            access_token: response.access_token,
            refresh_token: Some(response.refresh_token),
            expires_at,
        })
    }

    /// Refresh token via /auth/token/refresh
    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, AuthError> {
        debug!("Refreshing auth token");

        let configuration = Configuration {
            base_path: auth::url(self.environment).into(),
            ..Configuration::new()
        };

        let request = RefreshTokenRequest::new(refresh_token.to_string());
        let response = auth_token_refresh_put(&configuration, request)
            .await
            .map_err(|e| AuthError::RefreshFailed(e.to_string()))?;

        let expires_at =
            Utc::now() + Duration::seconds(response.access_token_valid_for_seconds as i64);

        Ok(AuthToken {
            access_token: response.access_token,
            refresh_token: Some(response.refresh_token),
            expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_expiry() {
        let token = AuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: Utc::now() + Duration::seconds(60),
        };
        assert!(!token.is_expired());

        let expired_token = AuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: Utc::now() - Duration::seconds(60),
        };
        assert!(expired_token.is_expired());
    }

    #[test]
    fn test_invalid_key_length() {
        let result = TokenManager::new(
            "abc123".to_string(), // too short
            "0x123".to_string(),
            BluefinEnvironment::Staging,
        );
        assert!(result.is_err());
    }
}
