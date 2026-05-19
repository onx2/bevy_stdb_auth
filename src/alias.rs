use crate::message::{
    StdbAuthFailedMessage, StdbAuthLogoutFailedMessage, StdbAuthLogoutSucceededMessage,
    StdbAuthRefreshFailedMessage, StdbAuthSessionClearedMessage, StdbAuthSucceededMessage,
    StdbAuthTokenRefreshedMessage,
};
use bevy_ecs::prelude::MessageReader;

/// A [`MessageReader`] for  authentication succeeds.
pub type ReadStdbAuthSucceededMessage<'w, 's> = MessageReader<'w, 's, StdbAuthSucceededMessage>;

/// A [`MessageReader`] for [`StdbAuthFailedMessage`].
pub type ReadStdbAuthFailedMessage<'w, 's> = MessageReader<'w, 's, StdbAuthFailedMessage>;

/// A [`MessageReader`] for [`StdbAuthTokenRefreshedMessage`].
pub type ReadStdbAuthTokenRefreshedMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthTokenRefreshedMessage>;

/// A [`MessageReader`] for [`StdbAuthRefreshFailedMessage`].
pub type ReadStdbAuthRefreshFailedMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthRefreshFailedMessage>;

/// A [`MessageReader`] for [`StdbAuthLogoutSucceededMessage`].
pub type ReadStdbAuthLogoutSucceededMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthLogoutSucceededMessage>;

/// A [`MessageReader`] for [`StdbAuthLogoutFailedMessage`].
pub type ReadStdbAuthLogoutFailedMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthLogoutFailedMessage>;

/// A [`MessageReader`] for [`StdbAuthSessionClearedMessage`].
pub type ReadStdbAuthSessionClearedMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthSessionClearedMessage>;
