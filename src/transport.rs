use crate::error::StdbAuthError;
use bevy_ecs::prelude::Resource;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(feature = "oidc")]
const SPACETIMEAUTH_AUTHORIZATION_ENDPOINT: &str = "https://auth.spacetimedb.com/oidc/auth";
const SPACETIMEAUTH_TOKEN_ENDPOINT: &str = "https://auth.spacetimedb.com/oidc/token";
#[cfg(feature = "oidc")]
const SPACETIMEAUTH_END_SESSION_ENDPOINT: &str = "https://auth.spacetimedb.com/oidc/session/end";

/// Configures HTTP transport for SpacetimeAuth provider requests.
#[cfg_attr(target_arch = "wasm32", derive(Default))]
#[derive(Clone, Debug, Resource)]
pub(crate) struct StdbAuthTransportConfig {
    #[cfg(not(target_arch = "wasm32"))]
    token_request_timeout: Option<Duration>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for StdbAuthTransportConfig {
    fn default() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            token_request_timeout: Some(Duration::from_secs(10)),
        }
    }
}

impl StdbAuthTransportConfig {
    /// Creates a validated [`StdbAuthTransportConfig`].
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn try_new(token_request_timeout: Option<Duration>) -> Result<Self, StdbAuthError> {
        if token_request_timeout.is_some_and(|v| v.is_zero()) {
            return Err(StdbAuthError::InvalidConfig(
                "token request timeout must be greater than zero".to_string(),
            ));
        }

        Ok(Self {
            token_request_timeout,
        })
    }

    /// Returns the SpacetimeAuth authorization endpoint URL.
    #[cfg(feature = "oidc")]
    pub(crate) fn authorization_endpoint_url(&self) -> &'static str {
        SPACETIMEAUTH_AUTHORIZATION_ENDPOINT
    }

    /// Returns the SpacetimeAuth token endpoint URL.
    pub(crate) fn token_endpoint_url(&self) -> &'static str {
        SPACETIMEAUTH_TOKEN_ENDPOINT
    }

    /// Returns the SpacetimeAuth end-session endpoint URL.
    #[cfg(feature = "oidc")]
    pub(crate) fn end_session_endpoint_url(&self) -> &'static str {
        SPACETIMEAUTH_END_SESSION_ENDPOINT
    }

    /// Returns the token request timeout duration.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn token_request_timeout(&self) -> Option<Duration> {
        self.token_request_timeout
    }

    /// Builds a native blocking token request client.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn token_client(&self) -> Result<reqwest::blocking::Client, StdbAuthError> {
        reqwest::blocking::Client::builder()
            .timeout(self.token_request_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(StdbAuthError::from)
    }

    /// Builds a browser token request client.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn token_client(&self) -> Result<reqwest::Client, StdbAuthError> {
        reqwest::Client::builder()
            .build()
            .map_err(StdbAuthError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_spacetimeauth_endpoints_are_used() {
        let config = StdbAuthTransportConfig::default();

        #[cfg(feature = "oidc")]
        assert_eq!(
            config.authorization_endpoint_url(),
            "https://auth.spacetimedb.com/oidc/auth"
        );
        assert_eq!(
            config.token_endpoint_url(),
            "https://auth.spacetimedb.com/oidc/token"
        );
        #[cfg(feature = "oidc")]
        assert_eq!(
            config.end_session_endpoint_url(),
            "https://auth.spacetimedb.com/oidc/session/end"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn zero_token_request_timeout_is_rejected() {
        let error = StdbAuthTransportConfig::try_new(Some(Duration::ZERO))
            .expect_err("zero timeout should be rejected");

        assert!(matches!(error, StdbAuthError::InvalidConfig(_)));
    }
}
