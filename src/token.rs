use crate::session::{StdbAuthSession, StdbAuthSessionSource};
use std::time::{Duration, Instant};

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

impl StdbTokenResponse {
    #[allow(dead_code)]
    pub(crate) fn into_session(
        self,
        client_id: Option<String>,
        source: StdbAuthSessionSource,
        post_logout_redirect_uri: Option<String>,
    ) -> StdbAuthSession {
        StdbAuthSession {
            access_token: self.access_token,
            token_type: self.token_type,
            expires_at: self
                .expires_in
                .map(|seconds| Instant::now() + Duration::from_secs(seconds)),
            refresh_token: self.refresh_token,
            scope: self.scope,
            id_token: self.id_token,
            client_id,
            source,
            post_logout_redirect_uri,
        }
    }
}
