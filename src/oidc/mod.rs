//! OIDC authorization-code support for SpacetimeAuth.

mod common;

#[cfg(all(feature = "oidc", feature = "browser"))]
mod browser;
#[cfg(all(feature = "oidc", not(feature = "browser")))]
mod loopback;
#[cfg(all(feature = "oidc", not(feature = "browser")))]
mod native;

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
    pub fn as_param(&self) -> Option<&'static str> {
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
