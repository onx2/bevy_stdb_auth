//! Steam ticket exchange support for SpacetimeAuth.

use crate::{
    AUTH_URI_BASE,
    error::StdbAuthError,
    session::{StdbAuthSession, StdbAuthSessionSource},
    token::StdbTokenResponse,
};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use steamworks::{Client, TicketForWebApiResponse};

const STEAM_TOKEN_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

/// Options for authenticating with a Steam Web API ticket.
#[derive(Clone, Debug)]
pub struct StdbSteamAuthOptions {
    /// The OAuth client identifier.
    pub client_id: String,
    /// The unique identifier for the Steam application.
    pub app_id: u32,
}

pub(crate) fn acquire_session(
    options: StdbSteamAuthOptions,
) -> Result<StdbAuthSession, StdbAuthError> {
    let token = acquire_token_response(&options)?;

    Ok(token.into_session(Some(options.client_id), StdbAuthSessionSource::Steam, None))
}

/// Acquires a token response using Steam authentication.
fn acquire_token_response(
    options: &StdbSteamAuthOptions,
) -> Result<StdbTokenResponse, StdbAuthError> {
    let steam_client = Client::init_app(options.app_id).map_err(|error| {
        StdbAuthError::Internal(format!("failed to initialize Steam client: {error}"))
    })?;

    let ticket = request_steam_webapi_ticket(&steam_client)?;

    exchange_steam_ticket_request(&options.client_id, &ticket)
}

/// Exchanges a Steam Web API ticket for a token response.
fn exchange_steam_ticket_request(
    client_id: &str,
    steam_ticket: &[u8],
) -> Result<StdbTokenResponse, StdbAuthError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(STEAM_TOKEN_EXCHANGE_TIMEOUT)
        .build()
        .map_err(StdbAuthError::from)?;
    let response = client
        .post(format!("{AUTH_URI_BASE}/token"))
        .form(&[
            ("grant_type", "urn:spacetimeauth:steam-ticket"),
            ("steam_ticket", hex::encode(steam_ticket).as_str()),
            ("client_id", client_id),
        ])
        .send()
        .map_err(StdbAuthError::from)?
        .error_for_status()
        .map_err(StdbAuthError::from)?;

    response
        .json::<StdbTokenResponse>()
        .map_err(StdbAuthError::from)
}

/// Requests a Steam Web API ticket.
fn request_steam_webapi_ticket(client: &Client) -> Result<Vec<u8>, StdbAuthError> {
    let (tx, rx) = mpsc::sync_channel(1);

    let requested_handle = client
        .user()
        .authentication_session_ticket_for_webapi("spacetimeauth");

    let _callback = client.register_callback(move |response: TicketForWebApiResponse| {
        if response.ticket_handle == requested_handle {
            let _ = tx.send(response.result.map(|()| response.ticket));
        }
    });

    let timeout = Duration::from_secs(5);
    let start = Instant::now();

    loop {
        client.run_callbacks();

        if let Ok(result) = rx.try_recv() {
            return result.map_err(|error| StdbAuthError::Internal(error.to_string()));
        }

        if start.elapsed() >= timeout {
            return Err(StdbAuthError::Timeout);
        }

        thread::sleep(Duration::from_millis(10));
    }
}
