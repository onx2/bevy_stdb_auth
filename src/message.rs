use crate::{
    commands::{StdbAuthCommandError, StdbAuthOperationKind},
    session::StdbAuthSession,
};
use bevy_ecs::prelude::Message;

/// A message sent when authentication succeeds.
#[derive(Clone, Message)]
pub struct StdbAuthSucceededMessage {
    /// The active authentication session.
    pub session: StdbAuthSession,
}

/// A message sent when authentication fails.
#[derive(Clone, Message)]
pub struct StdbAuthFailedMessage {
    /// The failure message.
    pub message: String,
}

/// A message sent when an authentication command is rejected.
#[derive(Clone, Message)]
pub struct StdbAuthCommandRejectedMessage {
    /// The rejected authentication operation.
    pub operation: StdbAuthOperationKind,
    /// The reason the authentication command was rejected.
    pub error: StdbAuthCommandError,
}

/// A message sent when the access token is refreshed.
#[derive(Clone, Message)]
pub struct StdbAuthTokenRefreshedMessage {
    /// The active authentication session.
    pub session: StdbAuthSession,
}

/// A message sent when token refresh fails.
#[derive(Clone, Message)]
pub struct StdbAuthRefreshFailedMessage {
    /// The failure message.
    pub message: String,
}

/// A message sent when logout succeeds.
#[derive(Clone, Default, Message)]
pub struct StdbAuthLogoutSucceededMessage;

/// A message sent when logout fails.
#[derive(Clone, Message)]
pub struct StdbAuthLogoutFailedMessage {
    /// The failure message.
    pub message: String,
}
