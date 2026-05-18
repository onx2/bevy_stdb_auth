//! Bevy integration for SpacetimeAuth token sessions.
//!
//! `bevy_stdb_auth` acquires and maintains authentication tokens for applications
//! that use SpacetimeAuth. Applications decide how to use those tokens.

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    prelude::{Commands, IntoScheduleConfigs, Message, Res, Resource, World, resource_exists},
    system::SystemParam,
};
use bevy_tasks::{IoTaskPool, Task, block_on, poll_once};
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

impl StdbAuthPlugin {
    /// Creates a [`StdbAuthPlugin`] with default settings.
    pub fn new() -> Self {
        Self::default()
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

/// Sends authentication commands from Bevy systems.
#[derive(SystemParam)]
pub struct StdbAuthCommands<'w, 's> {
    commands: Commands<'w, 's>,
    pending_auth: Option<Res<'w, PendingAuthOperation>>,
    session: Option<Res<'w, StdbAuthSession>>,
}

impl StdbAuthCommands<'_, '_> {
    /// Starts a login flow using [`StdbLoginOptions`].
    pub fn login(&mut self, options: StdbLoginOptions) {
        if self.pending_auth.is_some() {
            return;
        }

        let source = options.source;
        let task = IoTaskPool::get().spawn(async move { source.acquire_session().await });
        self.commands
            .insert_resource(PendingAuthOperation::Login(task));
    }

    /// Starts a logout flow for the current [`StdbAuthSession`].
    pub fn logout(&mut self, _options: StdbLogoutOptions) {
        if self.pending_auth.is_some() {
            return;
        }

        if self.session.is_none() {
            self.commands.insert_resource(PendingAuthOperation::Clear);
            return;
        }

        let task = IoTaskPool::get().spawn(async { Ok(()) });
        self.commands
            .insert_resource(PendingAuthOperation::Logout(task));
    }

    /// Clears local authentication state without contacting SpacetimeAuth.
    pub fn clear_session(&mut self) {
        self.commands.insert_resource(PendingAuthOperation::Clear);
    }

    /// Requests an immediate token refresh for the current [`StdbAuthSession`].
    pub fn refresh_now(&mut self) {
        if self.pending_auth.is_some() {
            return;
        }

        let Some(session) = self.session.as_deref() else {
            return;
        };

        if session.refresh_token.is_none() {
            let task = IoTaskPool::get().spawn(async {
                Err(StdbAuthError::Unsupported(
                    "the current auth session does not include a refresh token".to_string(),
                ))
            });
            self.commands
                .insert_resource(PendingAuthOperation::Refresh(task));
            return;
        }

        let task = IoTaskPool::get().spawn(async {
            Err(StdbAuthError::Unsupported(
                "token refresh is not implemented for this auth source".to_string(),
            ))
        });
        self.commands
            .insert_resource(PendingAuthOperation::Refresh(task));
    }

    /// Clears any local pending authentication operation.
    pub fn cancel_pending(&mut self) {
        self.commands.remove_resource::<PendingAuthOperation>();
    }
}

/// Options for starting an authentication flow.
#[derive(Clone, Debug)]
pub struct StdbLoginOptions {
    /// The authentication source used to acquire a session.
    pub source: StdbAuthSource,
}

impl StdbLoginOptions {
    /// Creates [`StdbLoginOptions`] with the given [`StdbAuthSource`].
    pub fn new(source: StdbAuthSource) -> Self {
        Self { source }
    }
}

/// Options for logging out of the current authentication session.
#[derive(Clone, Debug, Default)]
pub struct StdbLogoutOptions {
    /// Whether provider-backed local credentials should be cleared.
    pub clear_stored_credentials: bool,
}

/// The source used to acquire a [`StdbAuthSession`].
#[derive(Clone, Debug)]
pub enum StdbAuthSource {
    /// Uses an existing token as a local auth session.
    Token(StdbTokenAuthOptions),
}

impl StdbAuthSource {
    async fn acquire_session(self) -> Result<StdbAuthSession, StdbAuthError> {
        match self {
            Self::Token(options) => Ok(options.into_session()),
        }
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

/// The current SpacetimeAuth session.
#[derive(Clone, Debug, Resource)]
pub struct StdbAuthSession {
    /// The access token used by authenticated clients.
    pub access_token: String,
    /// The token type, such as `Bearer`.
    pub token_type: String,
    /// The instant when the access token expires.
    pub expires_at: Option<Instant>,
    /// The optional refresh token used to acquire a new access token.
    pub refresh_token: Option<String>,
    /// The granted OAuth scopes.
    pub scope: Option<String>,
    /// The optional OIDC ID token.
    pub id_token: Option<String>,
    /// The optional client ID associated with this session.
    pub client_id: Option<String>,
    /// The source that produced this session.
    pub source: StdbAuthSessionSource,
}

/// The source that produced a [`StdbAuthSession`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StdbAuthSessionSource {
    /// The session was created from an existing token.
    Token,
}

/// A message sent when authentication succeeds.
#[derive(Clone, Debug, Message)]
pub struct StdbAuthSucceededMessage {
    /// The active authentication session.
    pub session: StdbAuthSession,
}

/// A message sent when authentication fails.
#[derive(Clone, Debug, Message)]
pub struct StdbAuthFailedMessage {
    /// The failure message.
    pub message: String,
}

/// A message sent when the access token is refreshed.
#[derive(Clone, Debug, Message)]
pub struct StdbAuthTokenRefreshedMessage {
    /// The active authentication session.
    pub session: StdbAuthSession,
}

/// A message sent when token refresh fails.
#[derive(Clone, Debug, Message)]
pub struct StdbAuthRefreshFailedMessage {
    /// The failure message.
    pub message: String,
}

/// A message sent when logout succeeds.
#[derive(Clone, Debug, Default, Message)]
pub struct StdbAuthLogoutSucceededMessage;

/// A message sent when logout fails.
#[derive(Clone, Debug, Message)]
pub struct StdbAuthLogoutFailedMessage {
    /// The failure message.
    pub message: String,
}

/// A message sent when local auth session state is cleared.
#[derive(Clone, Debug, Default, Message)]
pub struct StdbAuthSessionClearedMessage;

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
        StdbAuthCommands, StdbAuthFailedMessage, StdbAuthLogoutFailedMessage,
        StdbAuthLogoutSucceededMessage, StdbAuthPlugin, StdbAuthRefreshFailedMessage,
        StdbAuthSession, StdbAuthSessionClearedMessage, StdbAuthSessionSource, StdbAuthSource,
        StdbAuthSucceededMessage, StdbAuthTokenRefreshedMessage, StdbLoginOptions,
        StdbLogoutOptions, StdbTokenAuthOptions,
    };
}
