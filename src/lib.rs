//! Bevy integration for SpacetimeAuth token sessions.
//!
//! `bevy_stdb_auth` acquires and maintains authentication tokens for applications
//! that use SpacetimeAuth. Applications decide how to use those tokens.

#[cfg(not(any(feature = "oidc", feature = "steam")))]
compile_error!("enable at least one auth source feature: `oidc` or `steam`.");

#[allow(dead_code)]
pub(crate) const AUTH_URI_BASE: &str = "https://auth.spacetimedb.com/oidc";

mod alias;
mod commands;
mod error;
mod message;
mod plugin;
mod refresh;
mod session;
mod source;
mod token;

#[cfg(feature = "oidc")]
mod oidc;
#[cfg(feature = "steam")]
mod steam;

/// Common imports for `bevy_stdb_auth`.
pub mod prelude {
    #[cfg(feature = "oidc")]
    pub use crate::oidc::{StdbOidcAuthOptions, StdbOidcPrompt};
    #[cfg(feature = "steam")]
    pub use crate::steam::StdbSteamAuthOptions;
    pub use crate::{
        alias::{
            ReadStdbAuthCommandRejectedMessage, ReadStdbAuthFailedMessage,
            ReadStdbAuthLogoutFailedMessage, ReadStdbAuthLogoutSucceededMessage,
            ReadStdbAuthRefreshFailedMessage, ReadStdbAuthSucceededMessage,
            ReadStdbAuthTokenRefreshedMessage,
        },
        commands::{
            StdbAuthCommandError, StdbAuthCommands, StdbAuthOperationKind, StdbLoginOptions,
            StdbLogoutOptions,
        },
        error::StdbAuthError,
        message::{
            StdbAuthCommandRejectedMessage, StdbAuthFailedMessage, StdbAuthLogoutFailedMessage,
            StdbAuthLogoutSucceededMessage, StdbAuthRefreshFailedMessage, StdbAuthSucceededMessage,
            StdbAuthTokenRefreshedMessage,
        },
        plugin::StdbAuthPlugin,
        session::{StdbAuthSession, StdbAuthSessionSource},
        source::StdbAuthSource,
        token::StdbTokenResponse,
    };
}
