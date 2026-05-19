use crate::{error::StdbAuthError, session::StdbAuthSession, token::StdbTokenAuthOptions};

#[cfg(feature = "oidc")]
use crate::oidc::StdbOidcAuthOptions;
#[cfg(feature = "steam")]
use crate::steam::StdbSteamAuthOptions;

/// The source used to acquire a [`StdbAuthSession`].
#[derive(Clone, Debug)]
pub enum StdbAuthSource {
    /// Uses an existing token as a local auth session.
    Token(StdbTokenAuthOptions),
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
            Self::Token(options) => Ok(options.into()),
            #[cfg(feature = "oidc")]
            Self::Oidc(_) => Err(StdbAuthError::Unsupported(
                "OIDC authentication is not implemented yet".to_string(),
            )),
            #[cfg(feature = "steam")]
            Self::Steam(_) => Err(StdbAuthError::Unsupported(
                "Steam authentication is not implemented yet".to_string(),
            )),
        }
    }
}
