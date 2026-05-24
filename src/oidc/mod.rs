//! OIDC authorization-code support for SpacetimeAuth.
mod common;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
pub(crate) mod persistence;

#[cfg(all(feature = "browser", target_arch = "wasm32"))]
pub(crate) mod browser;
#[cfg(not(target_arch = "wasm32"))]
mod native;

use crate::error::StdbAuthError;
use crate::session::StdbAuthSessionParts;
use crate::transport::StdbAuthTransportConfig;

/// Controls the OIDC `prompt` authorization parameter.
#[derive(Clone, Debug, Default)]
pub enum StdbOidcPrompt {
    /// Allows the provider to decide whether user interaction is required.
    #[default]
    None,
    /// Requests that the provider force user authentication.
    Login,
    /// Requests that the provider prompt for account selection.
    SelectAccount,
}

impl StdbOidcPrompt {
    /// Returns the OIDC `prompt` parameter value.
    fn as_param(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Login => Some("login"),
            Self::SelectAccount => Some("select_account"),
        }
    }
}

/// Options for the SpacetimeAuth OIDC authorization-code flow.
#[derive(Clone, Debug)]
pub struct StdbOidcAuthOptions {
    /// The OAuth client identifier.
    pub client_id: String,
    /// The redirect URI used by the client.
    pub redirect_uri: String,
    /// The URI returned to after provider logout.
    pub post_logout_redirect_uri: Option<String>,
    /// The requested OAuth scopes.
    pub scopes: Vec<String>,
    /// The prompt behavior for interactive authorization.
    pub prompt: StdbOidcPrompt,
}

#[cfg(all(feature = "browser", target_arch = "wasm32"))]
pub(crate) async fn acquire_session(
    options: StdbOidcAuthOptions,
    transport_config: StdbAuthTransportConfig,
) -> Result<StdbAuthSessionParts, StdbAuthError> {
    browser::acquire_session(options, transport_config).await
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn acquire_session(
    options: StdbOidcAuthOptions,
    transport_config: StdbAuthTransportConfig,
) -> Result<StdbAuthSessionParts, StdbAuthError> {
    native::acquire_session(options, &transport_config).await
}
