use crate::message::{
    StdbAuthCommandRejectedMessage, StdbAuthFailedMessage, StdbAuthLogoutFailedMessage,
    StdbAuthLogoutSucceededMessage, StdbAuthRefreshFailedMessage, StdbAuthSucceededMessage,
    StdbAuthTokenRefreshedMessage,
};
use bevy_ecs::prelude::MessageReader;

/// Reads successful authentication messages.
pub type ReadStdbAuthSucceededMessage<'w, 's> = MessageReader<'w, 's, StdbAuthSucceededMessage>;

/// Reads authentication failure messages.
pub type ReadStdbAuthFailedMessage<'w, 's> = MessageReader<'w, 's, StdbAuthFailedMessage>;

/// Reads rejected authentication command messages.
pub type ReadStdbAuthCommandRejectedMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthCommandRejectedMessage>;

/// Reads successful token refresh messages.
pub type ReadStdbAuthTokenRefreshedMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthTokenRefreshedMessage>;

/// Reads token refresh failure messages.
pub type ReadStdbAuthRefreshFailedMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthRefreshFailedMessage>;

/// Reads successful logout messages.
pub type ReadStdbAuthLogoutSucceededMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthLogoutSucceededMessage>;

/// Reads logout failure messages.
pub type ReadStdbAuthLogoutFailedMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthLogoutFailedMessage>;
