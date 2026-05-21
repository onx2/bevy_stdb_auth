use crate::{error::StdbAuthError, session::StdbAuthSession};

#[cfg(feature = "oidc")]
use crate::oidc::{StdbOidcAuthOptions, acquire_session as acquire_oidc_session};
#[cfg(feature = "steam")]
use crate::steam::{StdbSteamAuthOptions, acquire_session as acquire_steam_session};

/// The source used to acquire a [`StdbAuthSession`].
#[derive(Clone, Debug)]
pub enum StdbAuthSource {
    /// Uses the SpacetimeAuth OIDC authorization-code flow.
    #[cfg(feature = "oidc")]
    Oidc(StdbOidcAuthOptions),
    /// Uses a Steam Web API ticket exchange through SpacetimeAuth.
    #[cfg(feature = "steam")]
    Steam(StdbSteamAuthOptions),
}

impl StdbAuthSource {
    pub(crate) async fn acquire_session(self) -> Result<StdbAuthSession, StdbAuthError> {
        match self {
            #[cfg(feature = "oidc")]
            Self::Oidc(options) => acquire_oidc_session(options).await,
            #[cfg(feature = "steam")]
            Self::Steam(options) => acquire_steam_session(options),
        }
    }
}
