//! Token refresh scheduling and refresh request handling.

use crate::{
    error::StdbAuthError,
    session::{StdbAuthSession, StdbAuthSessionParts},
    transport::StdbAuthTransportConfig,
};
use bevy_tasks::{IoTaskPool, Task, TaskPool};
use std::collections::BTreeMap;

/// Spawns a token refresh request for the active [`StdbAuthSession`].
pub(crate) fn spawn_refresh_session_task(
    session: StdbAuthSession,
    refresh_token: String,
    transport_config: StdbAuthTransportConfig,
) -> Task<Result<StdbAuthSessionParts, StdbAuthError>> {
    IoTaskPool::get_or_init(TaskPool::default)
        .spawn(async move { refresh_session(session, refresh_token, transport_config).await })
}

pub(crate) async fn refresh_session(
    session: StdbAuthSession,
    refresh_token: String,
    transport_config: StdbAuthTransportConfig,
) -> Result<StdbAuthSessionParts, StdbAuthError> {
    let client_id = session.client_id.clone().ok_or_else(|| {
        StdbAuthError::InvalidConfig("refresh requires a session client ID".to_string())
    })?;
    let token_form = refresh_token_form(&client_id, &refresh_token)?;
    let token = exchange_refresh_token(&transport_config, token_form).await?;
    let parts = token.into_session_parts(
        Some(client_id),
        session.source,
        session.post_logout_redirect_uri.clone(),
    )?;

    Ok(retain_refresh_context(session, refresh_token, parts))
}

struct RefreshTokenRequestForm {
    params: BTreeMap<String, String>,
}

fn refresh_token_form(
    client_id: &str,
    refresh_token: &str,
) -> Result<RefreshTokenRequestForm, StdbAuthError> {
    let mut params = BTreeMap::new();
    params.insert("grant_type".to_string(), "refresh_token".to_string());
    params.insert(
        "refresh_token".to_string(),
        require_non_empty(refresh_token, "refresh_token")?,
    );
    params.insert(
        "client_id".to_string(),
        require_non_empty(client_id, "client_id")?,
    );

    Ok(RefreshTokenRequestForm { params })
}

fn require_non_empty(value: &str, field: &'static str) -> Result<String, StdbAuthError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(StdbAuthError::InvalidConfig(format!(
            "`{field}` must not be empty"
        )));
    }

    Ok(value)
}

#[cfg(not(target_arch = "wasm32"))]
async fn exchange_refresh_token(
    transport_config: &StdbAuthTransportConfig,
    token_form: RefreshTokenRequestForm,
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

#[cfg(target_arch = "wasm32")]
async fn exchange_refresh_token(
    transport_config: &StdbAuthTransportConfig,
    token_form: RefreshTokenRequestForm,
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

#[cfg(test)]
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
    fn refresh_token_form_contains_required_fields() {
        let form =
            refresh_token_form("client", "refresh").expect("refresh token form should be valid");

        assert_eq!(
            form.params.get("grant_type").map(String::as_str),
            Some("refresh_token")
        );
        assert_eq!(
            form.params.get("client_id").map(String::as_str),
            Some("client")
        );
        assert_eq!(
            form.params.get("refresh_token").map(String::as_str),
            Some("refresh")
        );
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
