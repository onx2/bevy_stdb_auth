use crate::message::{
    StdbAuthCommandRejectedMessage, StdbAuthFailedMessage, StdbAuthLogoutFailedMessage,
    StdbAuthLogoutSucceededMessage, StdbAuthRefreshFailedMessage, StdbAuthSucceededMessage,
    StdbAuthTokenRefreshedMessage,
};
use bevy_ecs::prelude::MessageReader;

/// A [`MessageReader`] for [`StdbAuthSucceededMessage`].
pub type ReadStdbAuthSucceededMessage<'w, 's> = MessageReader<'w, 's, StdbAuthSucceededMessage>;

/// A [`MessageReader`] for [`StdbAuthFailedMessage`].
pub type ReadStdbAuthFailedMessage<'w, 's> = MessageReader<'w, 's, StdbAuthFailedMessage>;

/// A [`MessageReader`] for [`StdbAuthCommandRejectedMessage`].
pub type ReadStdbAuthCommandRejectedMessage<'w, 's> =
    MessageReader<'w, 's, StdbAuthCommandRejectedMessage>;

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
