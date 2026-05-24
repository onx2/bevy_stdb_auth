use crate::{TOKEN_ENDPOINT, error::StdbAuthError};
use std::time::Duration;

const TOKEN_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Builds a native blocking token request client.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn token_client() -> Result<reqwest::blocking::Client, StdbAuthError> {
    reqwest::blocking::Client::builder()
        .timeout(TOKEN_REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(StdbAuthError::from)
}

/// Builds a browser token request client.
#[cfg(target_arch = "wasm32")]
pub(crate) fn token_client() -> Result<reqwest::Client, StdbAuthError> {
    reqwest::Client::builder()
        .build()
        .map_err(StdbAuthError::from)
}

/// Builds a native blocking token endpoint request.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn token_endpoint_request(
    client: &reqwest::blocking::Client,
) -> reqwest::blocking::RequestBuilder {
    client.post(TOKEN_ENDPOINT).timeout(TOKEN_REQUEST_TIMEOUT)
}

/// Builds a browser token endpoint request.
#[cfg(target_arch = "wasm32")]
pub(crate) fn token_endpoint_request(client: &reqwest::Client) -> reqwest::RequestBuilder {
    client.post(TOKEN_ENDPOINT).timeout(TOKEN_REQUEST_TIMEOUT)
}
