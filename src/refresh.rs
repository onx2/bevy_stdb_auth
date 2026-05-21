//! Token refresh scheduling and refresh request handling.

use crate::{
    error::StdbAuthError,
    session::{StdbAuthSession, StdbAuthSessionParts},
    transport::StdbAuthTransportConfig,
};
use bevy_tasks::{IoTaskPool, Task, TaskPool};

/// Spawns a token refresh request for the active [`StdbAuthSession`].
pub(crate) fn spawn_refresh_session_task(
    session: StdbAuthSession,
    refresh_token: String,
    transport_config: StdbAuthTransportConfig,
) -> Task<Result<StdbAuthSessionParts, StdbAuthError>> {
    IoTaskPool::get_or_init(TaskPool::default)
        .spawn(async move { refresh_session(session, refresh_token, transport_config).await })
}

#[cfg(feature = "oidc")]
pub(crate) async fn refresh_session(
    session: StdbAuthSession,
    refresh_token: String,
    transport_config: StdbAuthTransportConfig,
) -> Result<StdbAuthSessionParts, StdbAuthError> {
    let client_id = session.client_id.clone().ok_or_else(|| {
        StdbAuthError::InvalidConfig("refresh requires a session client ID".to_string())
    })?;
    let token_form = crate::oidc::common::refresh_token_form(&client_id, &refresh_token)?;
    let token = exchange_refresh_token(&transport_config, token_form).await?;
    let parts = token.into_session_parts(
        Some(client_id),
        session.source,
        session.post_logout_redirect_uri.clone(),
    )?;

    Ok(retain_refresh_context(session, refresh_token, parts))
}

#[cfg(not(feature = "oidc"))]
pub(crate) async fn refresh_session(
    _session: StdbAuthSession,
    _refresh_token: String,
    _transport_config: StdbAuthTransportConfig,
) -> Result<StdbAuthSessionParts, StdbAuthError> {
    Err(StdbAuthError::Unsupported(
        "token refresh requires the `oidc` feature".to_string(),
    ))
}

#[cfg(all(feature = "oidc", not(target_arch = "wasm32")))]
async fn exchange_refresh_token(
    transport_config: &StdbAuthTransportConfig,
    token_form: crate::oidc::common::StdbOidcTokenRequestForm,
) -> Result<crate::token::StdbTokenResponse, StdbAuthError> {
    let client = transport_config.blocking_token_client()?;
    let response = client
        .post(transport_config.token_endpoint_url())
        .form(&token_form.params)
        .send()
        .map_err(StdbAuthError::from)?
        .error_for_status()
        .map_err(StdbAuthError::from)?;

    response
        .json::<crate::token::StdbTokenResponse>()
        .map_err(StdbAuthError::from)
}

#[cfg(all(feature = "oidc", target_arch = "wasm32"))]
async fn exchange_refresh_token(
    transport_config: &StdbAuthTransportConfig,
    token_form: crate::oidc::common::StdbOidcTokenRequestForm,
) -> Result<crate::token::StdbTokenResponse, StdbAuthError> {
    let client = transport_config.token_client()?;
    let response = client
        .post(transport_config.token_endpoint_url())
        .form(&token_form.params)
        .send()
        .await
        .map_err(StdbAuthError::from)?
        .error_for_status()
        .map_err(StdbAuthError::from)?;

    response
        .json::<crate::token::StdbTokenResponse>()
        .await
        .map_err(StdbAuthError::from)
}

#[cfg(feature = "oidc")]
fn retain_refresh_context(
    previous_session: StdbAuthSession,
    previous_refresh_token: String,
    mut parts: StdbAuthSessionParts,
) -> StdbAuthSessionParts {
    if parts.credentials.refresh_token.is_none() {
        parts.credentials.refresh_token = Some(previous_refresh_token);
    }

    if parts.session.scope.is_none() {
        parts.session.scope = previous_session.scope;
    }

    parts.session.can_refresh = parts.credentials.has_refresh_token();
    parts
}

#[cfg(all(test, feature = "oidc"))]
mod tests {
    use super::*;
    use crate::{
        session::{StdbAuthCredentialMaterial, StdbAuthSessionSource},
        token::StdbTokenResponse,
    };

    fn previous_session() -> StdbAuthSession {
        StdbAuthSession {
            access_token: "old_access".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: None,
            can_refresh: true,
            scope: Some("openid".to_string()),
            client_id: Some("client".to_string()),
            source: StdbAuthSessionSource::Oidc,
            post_logout_redirect_uri: None,
        }
    }

    #[test]
    fn refresh_context_retains_refresh_token_when_not_rotated() {
        let previous_refresh_token = "old_refresh".to_string();
        let parts = StdbTokenResponse {
            access_token: "new_access".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(60),
            refresh_token: None,
            scope: None,
            id_token: None,
        }
        .into_session_parts(
            Some("client".to_string()),
            StdbAuthSessionSource::Oidc,
            None,
        )
        .expect("token response should be valid");

        let parts = retain_refresh_context(previous_session(), previous_refresh_token, parts);

        assert_eq!(parts.session.access_token, "new_access");
        assert_eq!(parts.session.scope.as_deref(), Some("openid"));
        assert!(parts.session.can_refresh);
        assert_eq!(
            parts.credentials.refresh_token.as_deref(),
            Some("old_refresh")
        );
    }

    #[test]
    fn refresh_context_uses_rotated_refresh_token() {
        let parts = StdbAuthSessionParts::new(
            StdbAuthSession {
                access_token: "new_access".to_string(),
                token_type: "Bearer".to_string(),
                expires_at: None,
                can_refresh: true,
                scope: Some("openid email".to_string()),
                client_id: Some("client".to_string()),
                source: StdbAuthSessionSource::Oidc,
                post_logout_redirect_uri: None,
            },
            StdbAuthCredentialMaterial::new(Some("new_refresh".to_string()), None),
        );

        let parts = retain_refresh_context(previous_session(), "old_refresh".to_string(), parts);

        assert_eq!(
            parts.credentials.refresh_token.as_deref(),
            Some("new_refresh")
        );
        assert_eq!(parts.session.scope.as_deref(), Some("openid email"));
    }
}
