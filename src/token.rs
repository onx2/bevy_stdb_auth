use crate::{
    error::StdbAuthError,
    session::{
        StdbAuthCredentialMaterial, StdbAuthSession, StdbAuthSessionParts, StdbAuthSessionSource,
    },
};
use std::time::{Duration, Instant};

/// A normalized SpacetimeAuth token endpoint response.
#[derive(Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct StdbTokenResponse {
    /// The access token used by authenticated clients.
    pub(crate) access_token: String,
    /// The token type, such as `Bearer`.
    pub(crate) token_type: String,
    /// The number of seconds before the access token expires.
    pub(crate) expires_in: Option<u64>,
    /// The optional refresh token used to acquire a new access token.
    pub(crate) refresh_token: Option<String>,
    /// The granted OAuth scopes.
    pub(crate) scope: Option<String>,
    /// The OIDC ID token.
    pub(crate) id_token: String,
}

impl StdbTokenResponse {
    pub(crate) fn into_session_parts(
        self,
        client_id: Option<String>,
        source: StdbAuthSessionSource,
        post_logout_redirect_uri: Option<String>,
    ) -> Result<StdbAuthSessionParts, StdbAuthError> {
        let access_token = require_non_empty(self.access_token, "access_token")?;
        let token_type = require_bearer_token_type(self.token_type)?;
        let expires_in = validate_expires_in(self.expires_in)?;
        let id_token = require_non_empty(self.id_token, "id_token")?;

        let credentials = StdbAuthCredentialMaterial::new(
            optional_non_empty(self.refresh_token),
            id_token.clone(),
        );
        let session = StdbAuthSession {
            access_token,
            token_type,
            expires_at: expires_in.map(|seconds| Instant::now() + Duration::from_secs(seconds)),
            can_refresh: credentials.has_refresh_token(),
            scope: optional_non_empty(self.scope),
            client_id,
            source,
            post_logout_redirect_uri,
            id_token,
        };

        Ok(StdbAuthSessionParts::new(session, credentials))
    }
}

fn require_non_empty(value: String, field: &'static str) -> Result<String, StdbAuthError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(StdbAuthError::InvalidTokenResponse(format!(
            "`{field}` must not be empty"
        )));
    }

    Ok(value)
}

fn require_bearer_token_type(value: String) -> Result<String, StdbAuthError> {
    let value = require_non_empty(value, "token_type")?;
    if !value.eq_ignore_ascii_case("Bearer") {
        return Err(StdbAuthError::InvalidTokenResponse(
            "`token_type` must be `Bearer`".to_string(),
        ));
    }

    Ok(value)
}

fn validate_expires_in(expires_in: Option<u64>) -> Result<Option<u64>, StdbAuthError> {
    if expires_in == Some(0) {
        return Err(StdbAuthError::InvalidTokenResponse(
            "`expires_in` must be greater than zero".to_string(),
        ));
    }

    Ok(expires_in)
}

fn optional_non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_token_response() -> StdbTokenResponse {
        StdbTokenResponse {
            access_token: "access".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(60),
            refresh_token: Some("refresh".to_string()),
            scope: Some("openid".to_string()),
            id_token: "id".to_string(),
        }
    }

    #[test]
    fn token_response_splits_session_from_credentials() {
        let parts = valid_token_response()
            .into_session_parts(
                Some("client".to_string()),
                StdbAuthSessionSource::Oidc,
                None,
            )
            .expect("valid token response should produce session parts");

        assert_eq!(parts.session.access_token, "access");
        assert!(parts.session.can_refresh);
        assert_eq!(parts.credentials.refresh_token.as_deref(), Some("refresh"));
        assert_eq!(parts.credentials.id_token, "id");
        assert_eq!(parts.session.id_token, "id");
    }

    #[test]
    fn token_response_rejects_empty_access_token() {
        let mut response = valid_token_response();
        response.access_token.clear();

        let result = response.into_session_parts(None, StdbAuthSessionSource::Oidc, None);

        assert!(matches!(
            result,
            Err(StdbAuthError::InvalidTokenResponse(_))
        ));
    }

    #[test]
    fn token_response_rejects_non_bearer_token_type() {
        let mut response = valid_token_response();
        response.token_type = "mac".to_string();

        let result = response.into_session_parts(None, StdbAuthSessionSource::Oidc, None);

        assert!(matches!(
            result,
            Err(StdbAuthError::InvalidTokenResponse(_))
        ));
    }

    #[test]
    fn token_response_rejects_zero_expiration() {
        let mut response = valid_token_response();
        response.expires_in = Some(0);

        let result = response.into_session_parts(None, StdbAuthSessionSource::Oidc, None);

        assert!(matches!(
            result,
            Err(StdbAuthError::InvalidTokenResponse(_))
        ));
    }

    #[test]
    fn token_response_normalizes_empty_optional_fields() {
        let mut response = valid_token_response();
        response.refresh_token = Some("  ".to_string());
        response.scope = Some("  ".to_string());

        let parts = response
            .into_session_parts(None, StdbAuthSessionSource::Oidc, None)
            .expect("empty optional fields should be normalized");

        assert!(!parts.session.can_refresh);
        assert!(parts.session.scope.is_none());
        assert!(parts.credentials.refresh_token.is_none());
        assert_eq!(parts.credentials.id_token, "id");
        assert_eq!(parts.session.id_token, "id");
    }
}
