//! Browser OIDC redirect and callback resume support.
use super::StdbOidcAuthOptions;
use crate::{error::StdbAuthError, session::StdbAuthSession};

pub(crate) async fn acquire_session(
    _options: StdbOidcAuthOptions,
) -> Result<StdbAuthSession, StdbAuthError> {
    unimplemented!()
}
