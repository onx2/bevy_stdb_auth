use crate::session::StdbAuthSession;
use bevy_ecs::prelude::Message;

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
