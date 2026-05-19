use bevy_ecs::prelude::Resource;
use std::time::Instant;

/// The source that produced a [`StdbAuthSession`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StdbAuthSessionSource {
    /// The session was created from an existing token.
    Token,
}

/// The current SpacetimeAuth session.
#[derive(Clone, Debug, Resource)]
pub struct StdbAuthSession {
    /// The access token used by authenticated clients.
    pub access_token: String,
    /// The token type, such as `Bearer`.
    pub token_type: String,
    /// The instant when the access token expires.
    pub expires_at: Option<Instant>,
    /// The optional refresh token used to acquire a new access token.
    pub refresh_token: Option<String>,
    /// The granted OAuth scopes.
    pub scope: Option<String>,
    /// The optional OIDC ID token.
    pub id_token: Option<String>,
    /// The optional client ID associated with this session.
    pub client_id: Option<String>,
    /// The source that produced this session.
    pub source: StdbAuthSessionSource,
}
