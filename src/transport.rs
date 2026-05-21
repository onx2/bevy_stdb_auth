use crate::error::StdbAuthError;
use bevy_ecs::prelude::Resource;
use std::time::Duration;

const SPACETIMEAUTH_AUTHORIZATION_ENDPOINT: &str = "https://auth.spacetimedb.com/oidc/auth";
const SPACETIMEAUTH_TOKEN_ENDPOINT: &str = "https://auth.spacetimedb.com/oidc/token";
const SPACETIMEAUTH_END_SESSION_ENDPOINT: &str = "https://auth.spacetimedb.com/oidc/session/end";
const DEFAULT_TOKEN_REQUEST_TIMEOUT_SECS: u64 = 10;

/// Configures HTTP transport for SpacetimeAuth provider requests.
#[derive(Clone, Debug, Resource)]
pub struct StdbAuthTransportConfig {
    token_request_timeout: Duration,
}

impl StdbAuthTransportConfig {
    /// Creates a validated [`StdbAuthTransportConfig`].
    pub fn try_new(token_request_timeout: Duration) -> Result<Self, StdbAuthError> {
        if token_request_timeout.is_zero() {
            return Err(StdbAuthError::InvalidConfig(
                "token request timeout must be greater than zero".to_string(),
            ));
        }

        Ok(Self {
            token_request_timeout,
        })
    }

    /// Returns the SpacetimeAuth authorization endpoint URL.
    pub fn authorization_endpoint_url(&self) -> &'static str {
        SPACETIMEAUTH_AUTHORIZATION_ENDPOINT
    }

    /// Returns the SpacetimeAuth token endpoint URL.
    pub fn token_endpoint_url(&self) -> &'static str {
        SPACETIMEAUTH_TOKEN_ENDPOINT
    }

    /// Returns the SpacetimeAuth end-session endpoint URL.
    pub fn end_session_endpoint_url(&self) -> &'static str {
        SPACETIMEAUTH_END_SESSION_ENDPOINT
    }

    /// Returns the default token request timeout.
    pub fn default_token_request_timeout() -> Duration {
        Duration::from_secs(DEFAULT_TOKEN_REQUEST_TIMEOUT_SECS)
    }

    /// Returns the configured token request timeout.
    pub fn token_request_timeout(&self) -> Duration {
        self.token_request_timeout
    }

    /// Builds a native blocking token request client.
    #[allow(dead_code)]
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn blocking_token_client(&self) -> Result<reqwest::blocking::Client, StdbAuthError> {
        reqwest::blocking::Client::builder()
            .timeout(self.token_request_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(StdbAuthError::from)
    }

    /// Builds a browser token request client.
    #[allow(dead_code)]
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn token_client(&self) -> Result<reqwest::Client, StdbAuthError> {
        reqwest::Client::builder()
            .build()
            .map_err(StdbAuthError::from)
    }
}

impl Default for StdbAuthTransportConfig {
    fn default() -> Self {
        Self::try_new(Self::default_token_request_timeout())
            .expect("default SpacetimeAuth transport configuration must be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_spacetimeauth_endpoints_are_used() {
        let config = StdbAuthTransportConfig::default();

        assert_eq!(
            config.authorization_endpoint_url(),
            "https://auth.spacetimedb.com/oidc/auth"
        );
        assert_eq!(
            config.token_endpoint_url(),
            "https://auth.spacetimedb.com/oidc/token"
        );
        assert_eq!(
            config.end_session_endpoint_url(),
            "https://auth.spacetimedb.com/oidc/session/end"
        );
    }

    #[test]
    fn zero_token_request_timeout_is_rejected() {
        let error = StdbAuthTransportConfig::try_new(Duration::ZERO)
            .expect_err("zero timeout should be rejected");

        assert!(matches!(error, StdbAuthError::InvalidConfig(_)));
    }
}
