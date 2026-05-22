//! Bevy integration for SpacetimeAuth token sessions.
//!
//! `bevy_stdb_auth` acquires and maintains authentication tokens for applications
//! that use SpacetimeAuth. Applications decide how to use those tokens.

#[cfg(not(any(feature = "oidc", feature = "steam")))]
compile_error!("enable at least one auth source feature: `oidc` or `steam`.");

#[cfg(all(target_arch = "wasm32", feature = "oidc", not(feature = "browser")))]
compile_error!("enable the `browser` feature when using `oidc` on `wasm32` targets.");

#[cfg(all(target_arch = "wasm32", feature = "steam"))]
compile_error!("the `steam` feature is native-only and cannot be enabled on `wasm32` targets.");

#[cfg(all(target_arch = "wasm32", feature = "persistence"))]
compile_error!(
    "the `persistence` feature is native-only and cannot be enabled on `wasm32` targets."
);

mod alias;
mod commands;
mod error;
mod message;
mod plugin;
mod refresh;
mod session;
mod source;
mod token;
mod transport;

#[cfg(feature = "oidc")]
mod oidc;
#[cfg(all(feature = "steam", not(target_arch = "wasm32")))]
mod steam;

/// Common imports for `bevy_stdb_auth`.
pub mod prelude {
    #[cfg(feature = "oidc")]
    pub use crate::oidc::{StdbOidcAuthOptions, StdbOidcPrompt};
    #[cfg(all(feature = "steam", not(target_arch = "wasm32")))]
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
        plugin::{StdbAuthPlugin, StdbAutoRefreshOptions},
        session::{StdbAuthSession, StdbAuthSessionSource},
        source::StdbAuthSource,
        transport::StdbAuthTransportConfig,
    };
}
