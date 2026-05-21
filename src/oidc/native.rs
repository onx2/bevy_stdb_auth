//! Native OIDC authorization-code flow support.
use super::StdbOidcAuthOptions;
use crate::{error::StdbAuthError, session::StdbAuthSession};

pub(crate) fn acquire_session(
    options: StdbOidcAuthOptions,
) -> Result<StdbAuthSession, StdbAuthError> {
    unimplemented!()
}
