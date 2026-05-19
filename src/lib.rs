//! Bevy integration for SpacetimeAuth token sessions.
//!
//! `bevy_stdb_auth` acquires and maintains authentication tokens for applications
//! that use SpacetimeAuth. Applications decide how to use those tokens.

mod alias;
mod commands;
mod message;
mod session;

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::prelude::{IntoScheduleConfigs, Resource, World, resource_exists};
use bevy_tasks::{Task, block_on, poll_once};
use message::{
    StdbAuthFailedMessage, StdbAuthLogoutFailedMessage, StdbAuthLogoutSucceededMessage,
    StdbAuthRefreshFailedMessage, StdbAuthSessionClearedMessage, StdbAuthSucceededMessage,
    StdbAuthTokenRefreshedMessage,
};
use session::{StdbAuthSession, StdbAuthSessionSource};
use std::{fmt::Display, time::Duration, time::Instant};

const DEFAULT_REFRESH_BUFFER_SECS: u64 = 60;

/// Adds SpacetimeAuth session systems and messages to an [`App`].
#[derive(Clone, Debug)]
pub struct StdbAuthPlugin {
    /// Whether sessions with refresh tokens should be refreshed automatically.
    pub auto_refresh: bool,
    /// How long before expiration an automatic refresh should be requested.
    pub refresh_buffer: Duration,
    /// Whether browser callback URLs should be resumed automatically.
    pub auto_resume_browser_callback: bool,
}

impl Default for StdbAuthPlugin {
    fn default() -> Self {
        Self {
            auto_refresh: true,
            refresh_buffer: Duration::from_secs(DEFAULT_REFRESH_BUFFER_SECS),
            auto_resume_browser_callback: true,
        }
    }
}

impl Plugin for StdbAuthPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StdbAuthSucceededMessage>();
        app.add_message::<StdbAuthFailedMessage>();
        app.add_message::<StdbAuthTokenRefreshedMessage>();
        app.add_message::<StdbAuthRefreshFailedMessage>();
        app.add_message::<StdbAuthLogoutSucceededMessage>();
        app.add_message::<StdbAuthLogoutFailedMessage>();
        app.add_message::<StdbAuthSessionClearedMessage>();

        app.add_systems(
            PreUpdate,
            poll_pending_auth.run_if(resource_exists::<PendingAuthOperation>),
        );
    }
}

/// Options for creating a session from an existing token.
#[derive(Clone, Debug)]
pub struct StdbTokenAuthOptions {
    /// The access token used by authenticated clients.
    pub access_token: String,
    /// The token type, such as `Bearer`.
    pub token_type: String,
    /// The number of seconds before the access token expires.
    pub expires_in: Option<u64>,
    /// The optional refresh token used to acquire a new access token.
    pub refresh_token: Option<String>,
    /// The granted OAuth scopes.
    pub scope: Option<String>,
    /// The optional OIDC ID token.
    pub id_token: Option<String>,
    /// The optional client ID associated with this token.
    pub client_id: Option<String>,
}

impl StdbTokenAuthOptions {
    /// Creates [`StdbTokenAuthOptions`] with an access token.
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            token_type: "Bearer".to_string(),
            expires_in: None,
            refresh_token: None,
            scope: None,
            id_token: None,
            client_id: None,
        }
    }

    fn into_session(self) -> StdbAuthSession {
        StdbAuthSession {
            access_token: self.access_token,
            token_type: self.token_type,
            expires_at: self
                .expires_in
                .map(|seconds| Instant::now() + Duration::from_secs(seconds)),
            refresh_token: self.refresh_token,
            scope: self.scope,
            id_token: self.id_token,
            client_id: self.client_id,
            source: StdbAuthSessionSource::Token,
        }
    }
}

/// An authentication lifecycle error.
#[derive(Clone, Debug)]
pub enum StdbAuthError {
    /// The requested operation is not supported by the current auth source.
    Unsupported(String),
    /// An internal authentication operation failed.
    Internal(String),
}

impl Display for StdbAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(message) => write!(f, "unsupported auth operation: {message}"),
            Self::Internal(message) => write!(f, "auth operation failed: {message}"),
        }
    }
}

impl std::error::Error for StdbAuthError {}

#[derive(Resource)]
enum PendingAuthOperation {
    Login(Task<Result<StdbAuthSession, StdbAuthError>>),
    Logout(Task<Result<(), StdbAuthError>>),
    Refresh(Task<Result<StdbAuthSession, StdbAuthError>>),
    Clear,
}

fn poll_pending_auth(world: &mut World) {
    let Some(pending) = world.remove_resource::<PendingAuthOperation>() else {
        return;
    };

    match pending {
        PendingAuthOperation::Login(mut task) => {
            let Some(result) = block_on(poll_once(&mut task)) else {
                world.insert_resource(PendingAuthOperation::Login(task));
                return;
            };

            match result {
                Ok(session) => apply_login_success(world, session),
                Err(error) => {
                    world.write_message(StdbAuthFailedMessage {
                        message: error.to_string(),
                    });
                }
            }
        }
        PendingAuthOperation::Logout(mut task) => {
            let Some(result) = block_on(poll_once(&mut task)) else {
                world.insert_resource(PendingAuthOperation::Logout(task));
                return;
            };

            clear_session(world);

            match result {
                Ok(()) => {
                    world.write_message_default::<StdbAuthLogoutSucceededMessage>();
                }
                Err(error) => {
                    world.write_message(StdbAuthLogoutFailedMessage {
                        message: error.to_string(),
                    });
                }
            }
        }
        PendingAuthOperation::Refresh(mut task) => {
            let Some(result) = block_on(poll_once(&mut task)) else {
                world.insert_resource(PendingAuthOperation::Refresh(task));
                return;
            };

            match result {
                Ok(session) => {
                    world.insert_resource(session.clone());
                    world.write_message(StdbAuthTokenRefreshedMessage { session });
                }
                Err(error) => {
                    world.write_message(StdbAuthRefreshFailedMessage {
                        message: error.to_string(),
                    });
                }
            }
        }
        PendingAuthOperation::Clear => {
            clear_session(world);
            world.write_message_default::<StdbAuthSessionClearedMessage>();
        }
    }
}

fn apply_login_success(world: &mut World, session: StdbAuthSession) {
    world.insert_resource(session.clone());
    world.write_message(StdbAuthSucceededMessage { session });
}

fn clear_session(world: &mut World) {
    world.remove_resource::<StdbAuthSession>();
}

/// Common imports for `bevy_stdb_auth`.
pub mod prelude {
    pub use crate::{
        StdbAuthPlugin, StdbTokenAuthOptions,
        alias::{
            ReadStdbAuthFailedMessage, ReadStdbAuthLogoutFailedMessage,
            ReadStdbAuthLogoutSucceededMessage, ReadStdbAuthRefreshFailedMessage,
            ReadStdbAuthSessionClearedMessage, ReadStdbAuthSucceededMessage,
            ReadStdbAuthTokenRefreshedMessage,
        },
        commands::{StdbAuthCommands, StdbAuthSource, StdbLoginOptions, StdbLogoutOptions},
        message::{
            StdbAuthFailedMessage, StdbAuthLogoutFailedMessage, StdbAuthLogoutSucceededMessage,
            StdbAuthRefreshFailedMessage, StdbAuthSessionClearedMessage, StdbAuthSucceededMessage,
            StdbAuthTokenRefreshedMessage,
        },
    };
}
