//! Shared OIDC request construction and token normalization.

use super::StdbOidcAuthOptions;
use crate::{AUTHORIZATION_CODE_GRANT_TYPE, AUTHORIZATION_ENDPOINT, error::StdbAuthError};
use oauth2::{CsrfToken, PkceCodeChallenge};
use std::collections::BTreeMap;
use url::Url;

/// An OIDC authorization request and its local validation state.
pub(super) struct StdbOidcAuthorizationRequest {
    pub(super) authorization_url: Url,
    pub(super) state: String,
    pub(super) pkce_verifier: String,
}

/// An authorization-code callback returned by SpacetimeAuth.
pub(super) struct StdbOidcAuthorizationCode {
    pub(super) code: String,
}

/// A token endpoint form request.
pub(super) struct StdbOidcTokenRequestForm {
    pub(super) params: BTreeMap<String, String>,
}

/// Builds a SpacetimeAuth OIDC authorization URL.
pub(super) fn build_authorization_request(
    options: &StdbOidcAuthOptions,
) -> Result<StdbOidcAuthorizationRequest, StdbAuthError> {
    let client_id = require_non_empty(&options.client_id, "client_id")?;
    let redirect_uri = validate_redirect_uri(&options.redirect_uri)?;
    let scopes = normalized_scopes(&options.scopes);
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let state = CsrfToken::new_random();
    let mut authorization_url = Url::parse(AUTHORIZATION_ENDPOINT)
        .expect("static SpacetimeAuth authorization endpoint must be valid");

    {
        let mut query = authorization_url.query_pairs_mut();
        query.append_pair("response_type", "code");
        query.append_pair("client_id", &client_id);
        query.append_pair("redirect_uri", redirect_uri.as_str());
        query.append_pair("state", state.secret());
        query.append_pair("code_challenge", pkce_challenge.as_str());
        query.append_pair("code_challenge_method", pkce_challenge.method().as_str());

        if !scopes.is_empty() {
            query.append_pair("scope", &scopes.join(" "));
        }

        if let Some(prompt) = options.prompt.as_param() {
            query.append_pair("prompt", prompt);
        }
    }

    Ok(StdbOidcAuthorizationRequest {
        authorization_url,
        state: state.into_secret(),
        pkce_verifier: pkce_verifier.into_secret(),
    })
}

/// Parses a SpacetimeAuth OIDC callback URL.
pub(super) fn parse_callback_url(
    callback_url: &str,
    expected_state: &str,
) -> Result<StdbOidcAuthorizationCode, StdbAuthError> {
    let callback_url = Url::parse(callback_url).map_err(|error| {
        StdbAuthError::InvalidOidcCallback(format!("callback URL is invalid: {error}"))
    })?;
    let state = query_param(&callback_url, "state").ok_or_else(|| {
        StdbAuthError::InvalidOidcCallback("callback is missing `state`".to_string())
    })?;

    if state != expected_state {
        return Err(StdbAuthError::InvalidOidcCallback(
            "callback `state` does not match the pending authorization".to_string(),
        ));
    }

    if let Some(error) = query_param(&callback_url, "error") {
        let description = query_param(&callback_url, "error_description");
        return Err(StdbAuthError::Provider(format_provider_error(
            &error,
            description.as_deref(),
        )));
    }

    let code = query_param(&callback_url, "code").ok_or_else(|| {
        StdbAuthError::InvalidOidcCallback("callback is missing `code`".to_string())
    })?;

    if code.trim().is_empty() {
        return Err(StdbAuthError::InvalidOidcCallback(
            "callback `code` must not be empty".to_string(),
        ));
    }

    Ok(StdbOidcAuthorizationCode { code })
}

/// Builds an authorization-code token request form.
pub(super) fn authorization_code_token_form(
    options: &StdbOidcAuthOptions,
    code: &str,
    pkce_verifier: &str,
) -> Result<StdbOidcTokenRequestForm, StdbAuthError> {
    let mut params = BTreeMap::new();
    params.insert(
        "grant_type".to_string(),
        AUTHORIZATION_CODE_GRANT_TYPE.to_string(),
    );
    params.insert("code".to_string(), require_non_empty(code, "code")?);
    params.insert(
        "redirect_uri".to_string(),
        validate_redirect_uri(&options.redirect_uri)?.to_string(),
    );
    params.insert(
        "client_id".to_string(),
        require_non_empty(&options.client_id, "client_id")?,
    );
    params.insert(
        "code_verifier".to_string(),
        require_non_empty(pkce_verifier, "code_verifier")?,
    );

    Ok(StdbOidcTokenRequestForm { params })
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

fn validate_redirect_uri(redirect_uri: &str) -> Result<Url, StdbAuthError> {
    let redirect_uri = require_non_empty(redirect_uri, "redirect_uri")?;
    let redirect_uri = Url::parse(&redirect_uri).map_err(|error| {
        StdbAuthError::InvalidConfig(format!("`redirect_uri` is invalid: {error}"))
    })?;

    if redirect_uri.fragment().is_some() {
        return Err(StdbAuthError::InvalidConfig(
            "`redirect_uri` must not include a fragment".to_string(),
        ));
    }

    Ok(redirect_uri)
}

fn normalized_scopes(scopes: &[String]) -> Vec<String> {
    scopes
        .iter()
        .filter_map(|scope| {
            let scope = scope.trim();
            (!scope.is_empty()).then(|| scope.to_string())
        })
        .collect()
}

fn query_param(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find_map(|(key, value)| (key == name).then(|| value.into_owned()))
}

fn format_provider_error(error: &str, description: Option<&str>) -> String {
    match description.filter(|description| !description.trim().is_empty()) {
        Some(description) => format!("{error}: {description}"),
        None => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oidc::StdbOidcPrompt;

    fn auth_options() -> StdbOidcAuthOptions {
        StdbOidcAuthOptions {
            client_id: "client".to_string(),
            redirect_uri: "http://127.0.0.1:3000/callback".to_string(),
            post_logout_redirect_uri: None,
            scopes: vec!["openid".to_string(), "email".to_string()],
            prompt: StdbOidcPrompt::Login,
        }
    }

    fn query_map(url: &Url) -> BTreeMap<String, String> {
        url.query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect()
    }

    fn form_map(form: StdbOidcTokenRequestForm) -> BTreeMap<String, String> {
        form.params
    }

    #[test]
    fn authorization_request_contains_oidc_parameters() {
        let request = build_authorization_request(&auth_options())
            .expect("authorization request should be valid");
        let query = query_map(&request.authorization_url);

        assert_eq!(
            request.authorization_url.as_str().split('?').next(),
            Some("https://auth.spacetimedb.com/oidc/auth")
        );
        assert_eq!(query.get("response_type").map(String::as_str), Some("code"));
        assert_eq!(query.get("client_id").map(String::as_str), Some("client"));
        assert_eq!(
            query.get("redirect_uri").map(String::as_str),
            Some("http://127.0.0.1:3000/callback")
        );
        assert_eq!(query.get("scope").map(String::as_str), Some("openid email"));
        assert_eq!(query.get("prompt").map(String::as_str), Some("login"));
        assert_eq!(
            query.get("code_challenge_method").map(String::as_str),
            Some("S256")
        );
        assert_eq!(query.get("state"), Some(&request.state));
        assert!(query.contains_key("code_challenge"));
        assert!(request.pkce_verifier.len() >= 43);
    }

    #[test]
    fn callback_url_returns_authorization_code() {
        let callback = parse_callback_url(
            "http://127.0.0.1:3000/callback?code=abc&state=state",
            "state",
        )
        .expect("callback should be valid");

        assert_eq!(callback.code, "abc");
    }

    #[test]
    fn callback_url_rejects_state_mismatch() {
        let result = parse_callback_url(
            "http://127.0.0.1:3000/callback?code=abc&state=other",
            "state",
        );

        assert!(matches!(result, Err(StdbAuthError::InvalidOidcCallback(_))));
    }

    #[test]
    fn callback_url_returns_provider_error() {
        let result = parse_callback_url(
            "http://127.0.0.1:3000/callback?error=access_denied&error_description=nope&state=state",
            "state",
        );

        assert!(matches!(result, Err(StdbAuthError::Provider(_))));
    }

    #[test]
    fn authorization_code_token_form_contains_required_fields() {
        let form = authorization_code_token_form(&auth_options(), "code", "verifier")
            .expect("authorization-code token form should be valid");
        let form = form_map(form);

        assert_eq!(
            form.get("grant_type").map(String::as_str),
            Some("authorization_code")
        );
        assert_eq!(form.get("code").map(String::as_str), Some("code"));
        assert_eq!(form.get("client_id").map(String::as_str), Some("client"));
        assert_eq!(
            form.get("code_verifier").map(String::as_str),
            Some("verifier")
        );
        assert_eq!(
            form.get("redirect_uri").map(String::as_str),
            Some("http://127.0.0.1:3000/callback")
        );
    }
}
