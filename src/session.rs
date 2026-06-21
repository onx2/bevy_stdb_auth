use bevy_ecs::prelude::{Resource, World};
use std::time::Instant;

/// The source that produced a [`StdbAuthSession`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StdbAuthSessionSource {
    /// The session was created with the SpacetimeAuth OIDC flow.
    Oidc,
    /// The session was created with the SpacetimeAuth Steam flow.
    Steam,
}

/// The current SpacetimeAuth session.
#[derive(Clone, Resource)]
pub struct StdbAuthSession {
    /// The access token used by authenticated clients.
    pub access_token: String,
    /// The token type, such as `Bearer`.
    pub token_type: String,
    /// The instant when the access token expires.
    pub expires_at: Option<Instant>,
    /// Whether this session has refresh credentials.
    pub can_refresh: bool,
    /// The granted OAuth scopes.
    pub scope: Option<String>,
    /// The optional client ID associated with this session.
    pub client_id: Option<String>,
    /// The source that produced this session.
    pub source: StdbAuthSessionSource,
    /// The optional URI returned to after provider logout.
    pub post_logout_redirect_uri: Option<String>,
    /// The OIDC ID token.
    pub id_token: String,
}

/// Stores credential material for the active [`StdbAuthSession`].
#[derive(Clone, Resource)]
pub(crate) struct StdbAuthCredentialMaterial {
    pub(crate) refresh_token: Option<String>,
    pub(crate) id_token: String,
}

impl Default for StdbAuthCredentialMaterial {
    fn default() -> Self {
        Self {
            refresh_token: None,
            id_token: String::new(),
        }
    }
}

impl StdbAuthCredentialMaterial {
    pub(crate) fn new(refresh_token: Option<String>, id_token: String) -> Self {
        Self {
            refresh_token,
            id_token,
        }
    }

    pub(crate) fn has_refresh_token(&self) -> bool {
        self.refresh_token.is_some()
    }
}

/// Groups a [`StdbAuthSession`] with its credential material.
pub(crate) struct StdbAuthSessionParts {
    pub(crate) session: StdbAuthSession,
    pub(crate) credentials: StdbAuthCredentialMaterial,
}

impl StdbAuthSessionParts {
    pub(crate) fn new(session: StdbAuthSession, credentials: StdbAuthCredentialMaterial) -> Self {
        Self {
            session,
            credentials,
        }
    }
}

/// Removes the active [`StdbAuthSession`] and its credential material.
pub(crate) fn clear_session(world: &mut World) {
    world.remove_resource::<StdbAuthSession>();
    world.remove_resource::<StdbAuthCredentialMaterial>();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> StdbAuthSession {
        StdbAuthSession {
            access_token: "access".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: None,
            can_refresh: true,
            scope: None,
            client_id: Some("client".to_string()),
            source: StdbAuthSessionSource::Oidc,
            post_logout_redirect_uri: None,
            id_token: "id".to_string(),
        }
    }

    #[test]
    fn clear_session_removes_session_and_credentials() {
        let mut world = World::new();
        world.insert_resource(session());
        world.insert_resource(StdbAuthCredentialMaterial::new(
            Some("refresh".to_string()),
            "id".to_string(),
        ));

        clear_session(&mut world);

        assert!(!world.contains_resource::<StdbAuthSession>());
        assert!(!world.contains_resource::<StdbAuthCredentialMaterial>());
    }
}
