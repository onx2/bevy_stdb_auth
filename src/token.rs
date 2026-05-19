use crate::session::{StdbAuthSession, StdbAuthSessionSource};
use std::time::{Duration, Instant};

/// Options for creating a session from an existing token.
#[derive(Clone, Debug)]
pub struct StdbTokenAuthOptions {
    /// The access token used by authenticated clients.
    pub access_token: String,
    /// The token type, such as `Bearer`.
    pub token_type: String,
    /// The number of seconds before the access token expires.
    pub expires_in: Option<u64>,
    /// The optional refresh token used to acquire a new access token.
    pub refresh_token: Option<String>,
    /// The granted OAuth scopes.
    pub scope: Option<String>,
    /// The optional OIDC ID token.
    pub id_token: Option<String>,
    /// The optional client ID associated with this token.
    pub client_id: Option<String>,
}

impl StdbTokenAuthOptions {
    /// Creates [`StdbTokenAuthOptions`] with an access token.
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            token_type: "Bearer".to_string(),
            expires_in: None,
            refresh_token: None,
            scope: None,
            id_token: None,
            client_id: None,
        }
    }
}

impl From<StdbTokenAuthOptions> for StdbAuthSession {
    fn from(options: StdbTokenAuthOptions) -> Self {
        Self {
            access_token: options.access_token,
            token_type: options.token_type,
            expires_at: options
                .expires_in
                .map(|seconds| Instant::now() + Duration::from_secs(seconds)),
            refresh_token: options.refresh_token,
            scope: options.scope,
            id_token: options.id_token,
            client_id: options.client_id,
            source: StdbAuthSessionSource::Token,
            post_logout_redirect_uri: None,
        }
    }
}

/// A normalized SpacetimeAuth token endpoint response.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct StdbTokenResponse {
    /// The access token used by authenticated clients.
    pub access_token: String,
    /// The token type, such as `Bearer`.
    pub token_type: String,
    /// The number of seconds before the access token expires.
    pub expires_in: Option<u64>,
    /// The optional refresh token used to acquire a new access token.
    pub refresh_token: Option<String>,
    /// The granted OAuth scopes.
    pub scope: Option<String>,
    /// The optional OIDC ID token.
    pub id_token: Option<String>,
}
