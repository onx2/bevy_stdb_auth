//! Bevy integration for SpacetimeAuth token sessions.
//!
//! `bevy_stdb_auth` acquires and maintains authentication tokens for applications
//! that use SpacetimeAuth. Applications decide how to use those tokens.

mod alias;
mod commands;
mod error;
mod message;
mod plugin;
pub mod prelude;
mod refresh;
mod session;
mod source;
mod storage;
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
pub use plugin::StdbAuthPlugin;
pub use session::{StdbAuthSession, StdbAuthSessionSource};
pub use source::StdbAuthSource;
pub use token::{StdbTokenAuthOptions, StdbTokenResponse};
