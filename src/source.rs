use crate::{
    error::StdbAuthError, session::StdbAuthSessionParts, transport::StdbAuthTransportConfig,
};

#[cfg(feature = "oidc")]
use crate::oidc::{StdbOidcAuthOptions, acquire_session as acquire_oidc_session};
#[cfg(all(feature = "steam", not(target_arch = "wasm32")))]
use crate::steam::{StdbSteamAuthOptions, acquire_session as acquire_steam_session};

/// The source used to acquire a SpacetimeAuth session.
#[derive(Clone, Debug)]
pub enum StdbAuthSource {
    /// Uses the SpacetimeAuth OIDC authorization-code flow.
    #[cfg(feature = "oidc")]
    Oidc(StdbOidcAuthOptions),
    /// Uses a Steam Web API ticket exchange through SpacetimeAuth.
    #[cfg(all(feature = "steam", not(target_arch = "wasm32")))]
    Steam(StdbSteamAuthOptions),
}

impl StdbAuthSource {
    pub(crate) async fn acquire_session(
        self,
        transport_config: StdbAuthTransportConfig,
    ) -> Result<StdbAuthSessionParts, StdbAuthError> {
        match self {
            #[cfg(feature = "oidc")]
            Self::Oidc(options) => acquire_oidc_session(options, transport_config).await,
            #[cfg(all(feature = "steam", not(target_arch = "wasm32")))]
            Self::Steam(options) => acquire_steam_session(options, &transport_config),
        }
    }
}
