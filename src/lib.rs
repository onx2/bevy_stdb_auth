//! Bevy integration for SpacetimeAuth token sessions.
//!
//! `bevy_stdb_auth` acquires and maintains authentication tokens for applications
//! that use SpacetimeAuth. Applications decide how to use those tokens.

mod alias;
mod commands;
mod error;
mod message;
#[cfg(feature = "persistence")]
mod persistence;
mod plugin;
mod refresh;
mod session;
mod source;
mod token;

#[cfg(feature = "oidc")]
pub mod oidc;
#[cfg(feature = "steam")]
pub mod steam;

pub use commands::{StdbAuthCommands, StdbLoginOptions, StdbLogoutOptions};
pub use error::StdbAuthError;
pub use message::{
    StdbAuthFailedMessage, StdbAuthLogoutFailedMessage, StdbAuthLogoutSucceededMessage,
    StdbAuthRefreshFailedMessage, StdbAuthSessionClearedMessage, StdbAuthSucceededMessage,
    StdbAuthTokenRefreshedMessage,
};
#[cfg(feature = "persistence")]
pub use persistence::StdbAuthPersistence;
pub use plugin::StdbAuthPlugin;
pub use session::{StdbAuthSession, StdbAuthSessionSource};
pub use source::StdbAuthSource;
pub use token::{StdbTokenAuthOptions, StdbTokenResponse};

/// Common imports for `bevy_stdb_auth`.
pub mod prelude {
    pub use crate::{
        StdbAuthCommands, StdbAuthError, StdbAuthPlugin, StdbAuthSession, StdbAuthSessionSource,
        StdbAuthSource, StdbLoginOptions, StdbLogoutOptions, StdbTokenAuthOptions,
        StdbTokenResponse,
        alias::{
            ReadStdbAuthFailedMessage, ReadStdbAuthLogoutFailedMessage,
            ReadStdbAuthLogoutSucceededMessage, ReadStdbAuthRefreshFailedMessage,
            ReadStdbAuthSessionClearedMessage, ReadStdbAuthSucceededMessage,
            ReadStdbAuthTokenRefreshedMessage,
        },
        message::{
            StdbAuthFailedMessage, StdbAuthLogoutFailedMessage, StdbAuthLogoutSucceededMessage,
            StdbAuthRefreshFailedMessage, StdbAuthSessionClearedMessage, StdbAuthSucceededMessage,
            StdbAuthTokenRefreshedMessage,
        },
    };

    #[cfg(feature = "persistence")]
    pub use crate::StdbAuthPersistence;

    #[cfg(feature = "oidc")]
    pub use crate::oidc::{StdbOidcAuthOptions, StdbOidcPrompt};

    #[cfg(feature = "steam")]
    pub use crate::steam::StdbSteamAuthOptions;
}
