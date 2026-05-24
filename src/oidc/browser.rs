//! Browser OIDC redirect and callback resume support.

use super::{StdbOidcAuthOptions, common};
use crate::{
    error::StdbAuthError,
    session::{StdbAuthSessionParts, StdbAuthSessionSource},
    token::StdbTokenResponse,
    transport::StdbAuthTransportConfig,
};
use serde::{Deserialize, Serialize};
use url::Url;

/// The SessionStorage key used to store pending OIDC authorization state in the browser.
const PENDING_AUTH_STORAGE_KEY: &str = "bevy_stdb_auth.oidc.pending";

/// The time-to-live for pending OIDC authorization state in the browser, in milliseconds.
const PENDING_AUTH_TTL_MS: f64 = 5.0 * 60.0 * 1000.0; // 5 minutes

#[derive(Deserialize, Serialize)]
struct BrowserPendingAuthorization {
    client_id: String,
    redirect_uri: String,
    post_logout_redirect_uri: Option<String>,
    state: String,
    pkce_verifier: String,
    created_at_ms: f64,
}

pub(crate) async fn acquire_session(
    options: StdbOidcAuthOptions,
    transport_config: StdbAuthTransportConfig,
) -> Result<StdbAuthSessionParts, StdbAuthError> {
    if pending_callback_available() {
        return resume_session(transport_config).await;
    }

    start_authorization_redirect(&options, &transport_config)?;
    std::future::pending::<Result<StdbAuthSessionParts, StdbAuthError>>().await
}

pub(crate) fn pending_callback_available() -> bool {
    let Ok(href) = browser_location_href() else {
        return false;
    };
    let Ok(url) = Url::parse(&href) else {
        return false;
    };
    let has_callback_params = url
        .query_pairs()
        .any(|(key, _value)| key == "code" || key == "error");

    has_callback_params && load_pending_authorization().is_ok_and(|pending| pending.is_some())
}

pub(crate) async fn resume_session(
    transport_config: StdbAuthTransportConfig,
) -> Result<StdbAuthSessionParts, StdbAuthError> {
    let pending = load_pending_authorization()?.ok_or_else(|| {
        StdbAuthError::InvalidOidcCallback("missing browser OIDC pending state".to_string())
    })?;
    let callback_url = browser_location_href()?;
    let authorization_code = common::parse_callback_url(&callback_url, &pending.state);

    remove_pending_authorization();
    let _ = clean_browser_callback_url();

    let authorization_code = authorization_code?;
    let options = StdbOidcAuthOptions {
        client_id: pending.client_id.clone(),
        redirect_uri: pending.redirect_uri,
        post_logout_redirect_uri: pending.post_logout_redirect_uri.clone(),
        scopes: Vec::new(),
        prompt: super::StdbOidcPrompt::None,
    };
    let token_form = common::authorization_code_token_form(
        &options,
        &authorization_code.code,
        &pending.pkce_verifier,
    )?;
    let token = exchange_authorization_code(&transport_config, token_form).await?;

    token.into_session_parts(
        Some(pending.client_id),
        StdbAuthSessionSource::Oidc,
        pending.post_logout_redirect_uri,
    )
}

fn start_authorization_redirect(
    options: &StdbOidcAuthOptions,
    transport_config: &StdbAuthTransportConfig,
) -> Result<(), StdbAuthError> {
    let authorization_request = common::build_authorization_request(options, transport_config)?;
    let pending = BrowserPendingAuthorization {
        client_id: options.client_id.clone(),
        redirect_uri: options.redirect_uri.clone(),
        post_logout_redirect_uri: options.post_logout_redirect_uri.clone(),
        state: authorization_request.state,
        pkce_verifier: authorization_request.pkce_verifier,
        created_at_ms: js_sys::Date::now(),
    };

    store_pending_authorization(&pending)?;
    browser_window()?
        .location()
        .assign(authorization_request.authorization_url.as_str())
        .map_err(|error| {
            StdbAuthError::Internal(format_js_error("failed to redirect browser", error))
        })
}

async fn exchange_authorization_code(
    transport_config: &StdbAuthTransportConfig,
    token_form: common::StdbOidcTokenRequestForm,
) -> Result<StdbTokenResponse, StdbAuthError> {
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
        .json::<StdbTokenResponse>()
        .await
        .map_err(StdbAuthError::from)
}

fn load_pending_authorization() -> Result<Option<BrowserPendingAuthorization>, StdbAuthError> {
    let Some(storage) = browser_session_storage()? else {
        return Ok(None);
    };
    let Some(serialized) = storage
        .get_item(PENDING_AUTH_STORAGE_KEY)
        .map_err(|error| {
            StdbAuthError::Internal(format_js_error(
                "failed to read browser sessionStorage",
                error,
            ))
        })?
    else {
        return Ok(None);
    };
    let pending = serde_json::from_str::<BrowserPendingAuthorization>(&serialized)?;

    if js_sys::Date::now() - pending.created_at_ms > PENDING_AUTH_TTL_MS {
        remove_pending_authorization();
        return Err(StdbAuthError::InvalidOidcCallback(
            "browser OIDC pending state expired".to_string(),
        ));
    }

    Ok(Some(pending))
}

fn store_pending_authorization(pending: &BrowserPendingAuthorization) -> Result<(), StdbAuthError> {
    let storage = browser_session_storage()?.ok_or_else(|| {
        StdbAuthError::Internal("browser sessionStorage is unavailable".to_string())
    })?;
    let serialized = serde_json::to_string(pending)?;

    storage
        .set_item(PENDING_AUTH_STORAGE_KEY, &serialized)
        .map_err(|error| {
            StdbAuthError::Internal(format_js_error(
                "failed to write browser sessionStorage",
                error,
            ))
        })
}

fn remove_pending_authorization() {
    if let Ok(Some(storage)) = browser_session_storage() {
        let _ = storage.remove_item(PENDING_AUTH_STORAGE_KEY);
    }
}

fn clean_browser_callback_url() -> Result<(), StdbAuthError> {
    let href = browser_location_href()?;
    let mut url = Url::parse(&href).map_err(|error| {
        StdbAuthError::InvalidOidcCallback(format!("browser callback URL is invalid: {error}"))
    })?;
    let preserved_query_params = url
        .query_pairs()
        .filter(|(key, _value)| !is_oidc_callback_param(key))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    url.set_query(None);

    if !preserved_query_params.is_empty() {
        url.query_pairs_mut().extend_pairs(
            preserved_query_params
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
    }

    let history_state = js_sys::Object::new();

    browser_window()?
        .history()
        .map_err(|error| {
            StdbAuthError::Internal(format_js_error("failed to access browser history", error))
        })?
        .replace_state_with_url(history_state.as_ref(), "", Some(url.as_str()))
        .map_err(|error| {
            StdbAuthError::Internal(format_js_error("failed to clean callback URL", error))
        })
}

fn is_oidc_callback_param(param: &str) -> bool {
    matches!(
        param,
        "code" | "state" | "error" | "error_description" | "error_uri" | "session_state" | "iss"
    )
}

fn browser_location_href() -> Result<String, StdbAuthError> {
    browser_window()?.location().href().map_err(|error| {
        StdbAuthError::Internal(format_js_error("failed to read browser URL", error))
    })
}

fn browser_session_storage() -> Result<Option<web_sys::Storage>, StdbAuthError> {
    browser_window()?.session_storage().map_err(|error| {
        StdbAuthError::Internal(format_js_error(
            "failed to access browser sessionStorage",
            error,
        ))
    })
}

fn browser_window() -> Result<web_sys::Window, StdbAuthError> {
    web_sys::window()
        .ok_or_else(|| StdbAuthError::Internal("browser window is unavailable".to_string()))
}

fn format_js_error(context: &str, error: impl core::fmt::Debug) -> String {
    format!("{context}: {error:?}")
}
