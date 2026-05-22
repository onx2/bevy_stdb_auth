//! Native OIDC authorization-code flow support.

use super::{StdbOidcAuthOptions, common};
use crate::{
    error::StdbAuthError,
    session::{StdbAuthSessionParts, StdbAuthSessionSource},
    token::StdbTokenResponse,
    transport::StdbAuthTransportConfig,
};
use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};
use url::Url;

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(120);
const CALLBACK_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CALLBACK_READ_TIMEOUT: Duration = Duration::from_secs(5);
const CALLBACK_REQUEST_BUFFER_SIZE: usize = 8192;

pub(crate) fn acquire_session(
    options: StdbOidcAuthOptions,
    transport_config: &StdbAuthTransportConfig,
) -> Result<StdbAuthSessionParts, StdbAuthError> {
    let redirect_uri = validate_native_redirect_uri(&options.redirect_uri)?;

    #[cfg(feature = "persistence")]
    if let Some(parts) = try_refresh_stored_session(&options, transport_config) {
        return Ok(parts);
    }

    let listener = bind_loopback_listener(&redirect_uri)?;
    let authorization_request = common::build_authorization_request(&options, transport_config)?;

    webbrowser::open(authorization_request.authorization_url.as_str()).map_err(|error| {
        StdbAuthError::Internal(format!("failed to open system browser: {error}"))
    })?;

    let authorization_code = receive_authorization_code(
        &listener,
        &redirect_uri,
        &authorization_request.state,
        CALLBACK_TIMEOUT,
    )?;
    let token_form = common::authorization_code_token_form(
        &options,
        &authorization_code.code,
        &authorization_request.pkce_verifier,
    )?;
    let token = exchange_authorization_code(transport_config, token_form)?;

    token.into_session_parts(
        Some(options.client_id),
        StdbAuthSessionSource::Oidc,
        options.post_logout_redirect_uri,
    )
}

#[cfg(feature = "persistence")]
fn try_refresh_stored_session(
    options: &StdbOidcAuthOptions,
    transport_config: &StdbAuthTransportConfig,
) -> Option<StdbAuthSessionParts> {
    let refresh_token = super::persistence::stored_refresh_token_best_effort(&options.client_id)?;
    let session = crate::session::StdbAuthSession {
        access_token: String::new(),
        token_type: "Bearer".to_string(),
        expires_at: None,
        can_refresh: true,
        scope: None,
        client_id: Some(options.client_id.clone()),
        source: StdbAuthSessionSource::Oidc,
        post_logout_redirect_uri: options.post_logout_redirect_uri.clone(),
    };

    bevy_tasks::block_on(crate::refresh::refresh_session(
        session,
        refresh_token,
        transport_config.clone(),
    ))
    .ok()
}

fn bind_loopback_listener(redirect_uri: &Url) -> Result<TcpListener, StdbAuthError> {
    let bind_addr = loopback_bind_addr(redirect_uri)?;
    let listener = TcpListener::bind(bind_addr).map_err(|error| {
        StdbAuthError::Internal(format!("failed to bind OIDC callback listener: {error}"))
    })?;
    listener.set_nonblocking(true).map_err(|error| {
        StdbAuthError::Internal(format!(
            "failed to configure OIDC callback listener: {error}"
        ))
    })?;

    Ok(listener)
}

fn receive_authorization_code(
    listener: &TcpListener,
    redirect_uri: &Url,
    expected_state: &str,
    timeout: Duration,
) -> Result<common::StdbOidcAuthorizationCode, StdbAuthError> {
    let started_at = Instant::now();

    loop {
        match listener.accept() {
            Ok((mut stream, _remote_addr)) => {
                let callback_url = match read_callback_url(&mut stream, redirect_uri) {
                    Ok(callback_url) => callback_url,
                    Err(error) => {
                        let _ = write_callback_response(&mut stream, false);
                        return Err(error);
                    }
                };
                let result = common::parse_callback_url(callback_url.as_str(), expected_state);

                let _ = write_callback_response(&mut stream, result.is_ok());
                return result;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if started_at.elapsed() >= timeout {
                    return Err(StdbAuthError::Timeout);
                }

                thread::sleep(CALLBACK_POLL_INTERVAL);
            }
            Err(error) => {
                return Err(StdbAuthError::Internal(format!(
                    "failed to accept OIDC callback: {error}"
                )));
            }
        }
    }
}

fn exchange_authorization_code(
    transport_config: &StdbAuthTransportConfig,
    token_form: common::StdbOidcTokenRequestForm,
) -> Result<StdbTokenResponse, StdbAuthError> {
    let client = transport_config.blocking_token_client()?;
    let response = client
        .post(transport_config.token_endpoint_url())
        .form(&token_form.params)
        .send()
        .map_err(StdbAuthError::from)?
        .error_for_status()
        .map_err(StdbAuthError::from)?;

    response
        .json::<StdbTokenResponse>()
        .map_err(StdbAuthError::from)
}

fn read_callback_url(stream: &mut TcpStream, redirect_uri: &Url) -> Result<Url, StdbAuthError> {
    stream
        .set_read_timeout(Some(CALLBACK_READ_TIMEOUT))
        .map_err(|error| {
            StdbAuthError::Internal(format!("failed to configure OIDC callback stream: {error}"))
        })?;

    let mut buffer = [0; CALLBACK_REQUEST_BUFFER_SIZE];
    let bytes_read = stream.read(&mut buffer).map_err(|error| {
        StdbAuthError::InvalidOidcCallback(format!("failed to read callback request: {error}"))
    })?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request_line = request.lines().next().ok_or_else(|| {
        StdbAuthError::InvalidOidcCallback("callback request is empty".to_string())
    })?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or_else(|| {
        StdbAuthError::InvalidOidcCallback("callback request is missing method".to_string())
    })?;
    let request_target = parts.next().ok_or_else(|| {
        StdbAuthError::InvalidOidcCallback("callback request is missing target".to_string())
    })?;

    if method != "GET" {
        return Err(StdbAuthError::InvalidOidcCallback(
            "callback request method must be GET".to_string(),
        ));
    }

    callback_url_from_request_target(redirect_uri, request_target)
}

fn callback_url_from_request_target(
    redirect_uri: &Url,
    request_target: &str,
) -> Result<Url, StdbAuthError> {
    let callback_url = if request_target.starts_with("http://")
        || request_target.starts_with("https://")
    {
        Url::parse(request_target)
    } else {
        redirect_uri.join(request_target)
    }
    .map_err(|error| {
        StdbAuthError::InvalidOidcCallback(format!("callback request target is invalid: {error}"))
    })?;

    validate_callback_url_matches_redirect_uri(&callback_url, redirect_uri)?;

    Ok(callback_url)
}

fn validate_callback_url_matches_redirect_uri(
    callback_url: &Url,
    redirect_uri: &Url,
) -> Result<(), StdbAuthError> {
    let matches_redirect = callback_url.scheme() == redirect_uri.scheme()
        && callback_url.host_str() == redirect_uri.host_str()
        && callback_url.port_or_known_default() == redirect_uri.port_or_known_default()
        && callback_url.path() == redirect_uri.path();

    if !matches_redirect {
        return Err(StdbAuthError::InvalidOidcCallback(
            "callback URL does not match the configured redirect URI".to_string(),
        ));
    }

    Ok(())
}

fn write_callback_response(stream: &mut TcpStream, succeeded: bool) -> std::io::Result<()> {
    let (status, body) = if succeeded {
        (
            "200 OK",
            "<!doctype html><title>Authenticated</title><p>Authentication completed. You can close this window.</p>",
        )
    } else {
        (
            "400 Bad Request",
            "<!doctype html><title>Authentication failed</title><p>Authentication failed. You can close this window.</p>",
        )
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    stream.write_all(response.as_bytes())
}

fn validate_native_redirect_uri(redirect_uri: &str) -> Result<Url, StdbAuthError> {
    let redirect_uri = Url::parse(redirect_uri).map_err(|error| {
        StdbAuthError::InvalidConfig(format!("`redirect_uri` is invalid: {error}"))
    })?;

    if redirect_uri.scheme() != "http" {
        return Err(StdbAuthError::InvalidConfig(
            "native OIDC `redirect_uri` must use the `http` scheme".to_string(),
        ));
    }

    if redirect_uri.query().is_some() {
        return Err(StdbAuthError::InvalidConfig(
            "native OIDC `redirect_uri` must not include a query string".to_string(),
        ));
    }

    if redirect_uri.fragment().is_some() {
        return Err(StdbAuthError::InvalidConfig(
            "native OIDC `redirect_uri` must not include a fragment".to_string(),
        ));
    }

    let host = redirect_uri.host_str().ok_or_else(|| {
        StdbAuthError::InvalidConfig("native OIDC `redirect_uri` must include a host".to_string())
    })?;

    if !is_loopback_host(host) {
        return Err(StdbAuthError::InvalidConfig(
            "native OIDC `redirect_uri` must use a loopback host".to_string(),
        ));
    }

    if redirect_uri.port().is_none_or(|port| port == 0) {
        return Err(StdbAuthError::InvalidConfig(
            "native OIDC `redirect_uri` must include a non-zero explicit port".to_string(),
        ));
    }

    Ok(redirect_uri)
}

fn loopback_bind_addr(redirect_uri: &Url) -> Result<SocketAddr, StdbAuthError> {
    let host = redirect_uri.host_str().ok_or_else(|| {
        StdbAuthError::InvalidConfig("native OIDC `redirect_uri` must include a host".to_string())
    })?;
    let port = redirect_uri.port().ok_or_else(|| {
        StdbAuthError::InvalidConfig(
            "native OIDC `redirect_uri` must include an explicit port".to_string(),
        )
    })?;
    let ip = if host.eq_ignore_ascii_case("localhost") {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    } else {
        host.parse::<IpAddr>().map_err(|error| {
            StdbAuthError::InvalidConfig(format!("native OIDC loopback host is invalid: {error}"))
        })?
    };

    if !ip.is_loopback() {
        return Err(StdbAuthError::InvalidConfig(
            "native OIDC `redirect_uri` must use a loopback host".to_string(),
        ));
    }

    Ok(SocketAddr::new(ip, port))
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_redirect_uri_accepts_loopback_http_with_port() {
        let redirect_uri = validate_native_redirect_uri("http://127.0.0.1:3000/callback")
            .expect("loopback redirect URI should be valid");

        assert_eq!(redirect_uri.as_str(), "http://127.0.0.1:3000/callback");
    }

    #[test]
    fn native_redirect_uri_rejects_non_loopback_hosts() {
        let result = validate_native_redirect_uri("http://example.com:3000/callback");

        assert!(matches!(result, Err(StdbAuthError::InvalidConfig(_))));
    }

    #[test]
    fn native_redirect_uri_rejects_query_string() {
        let result = validate_native_redirect_uri("http://127.0.0.1:3000/callback?route=auth");

        assert!(matches!(result, Err(StdbAuthError::InvalidConfig(_))));
    }

    #[test]
    fn native_redirect_uri_rejects_missing_port() {
        let result = validate_native_redirect_uri("http://127.0.0.1/callback");

        assert!(matches!(result, Err(StdbAuthError::InvalidConfig(_))));
    }

    #[test]
    fn native_redirect_uri_rejects_zero_port() {
        let result = validate_native_redirect_uri("http://127.0.0.1:0/callback");

        assert!(matches!(result, Err(StdbAuthError::InvalidConfig(_))));
    }

    #[test]
    fn callback_target_uses_redirect_origin() {
        let redirect_uri = Url::parse("http://127.0.0.1:3000/callback").unwrap();
        let callback_url =
            callback_url_from_request_target(&redirect_uri, "/callback?code=abc&state=state")
                .expect("callback target should be valid");

        assert_eq!(
            callback_url.as_str(),
            "http://127.0.0.1:3000/callback?code=abc&state=state"
        );
    }

    #[test]
    fn callback_target_rejects_wrong_path() {
        let redirect_uri = Url::parse("http://127.0.0.1:3000/callback").unwrap();
        let result = callback_url_from_request_target(&redirect_uri, "/other?code=abc&state=state");

        assert!(matches!(result, Err(StdbAuthError::InvalidOidcCallback(_))));
    }
}
