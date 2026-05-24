//! Steam ticket exchange support for SpacetimeAuth.

use crate::{
    error::StdbAuthError,
    session::{StdbAuthSessionParts, StdbAuthSessionSource},
    token::StdbTokenResponse,
    transport::StdbAuthTransportConfig,
};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use steamworks::{Client, TicketForWebApiResponse};

const CALLBACK_READ_TIMEOUT: Duration = Duration::from_secs(5);
const CALLBACK_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Options for authenticating with a Steam Web API ticket.
#[derive(Clone, Debug)]
pub struct StdbSteamAuthOptions {
    /// The OAuth client identifier.
    pub client_id: String,
    /// The unique identifier for the Steam application.
    pub app_id: u32,
}

pub(crate) async fn acquire_session(
    options: StdbSteamAuthOptions,
    transport_config: &StdbAuthTransportConfig,
) -> Result<StdbAuthSessionParts, StdbAuthError> {
    let token = acquire_token_response(&options, transport_config)?;

    token.into_session_parts(Some(options.client_id), StdbAuthSessionSource::Steam, None)
}

/// Acquires a token response using Steam authentication.
fn acquire_token_response(
    options: &StdbSteamAuthOptions,
    transport_config: &StdbAuthTransportConfig,
) -> Result<StdbTokenResponse, StdbAuthError> {
    let steam_client = Client::init_app(options.app_id).map_err(|error| {
        StdbAuthError::Internal(format!("failed to initialize Steam client: {error}"))
    })?;

    let ticket = request_steam_webapi_ticket(&steam_client)?;

    exchange_steam_ticket_request(&options.client_id, &ticket, transport_config)
}

/// Exchanges a Steam Web API ticket for a token response.
fn exchange_steam_ticket_request(
    client_id: &str,
    steam_ticket: &[u8],
    transport_config: &StdbAuthTransportConfig,
) -> Result<StdbTokenResponse, StdbAuthError> {
    let client = transport_config.token_client()?;
    let response = client
        .post(transport_config.token_endpoint_url())
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

    let start = Instant::now();

    loop {
        client.run_callbacks();

        if let Ok(result) = rx.try_recv() {
            return result.map_err(|error| StdbAuthError::Internal(error.to_string()));
        }

        if start.elapsed() >= CALLBACK_READ_TIMEOUT {
            return Err(StdbAuthError::Timeout);
        }

        thread::sleep(CALLBACK_POLL_INTERVAL);
    }
}
