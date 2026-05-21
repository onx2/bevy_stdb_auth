//! Browser OIDC redirect and callback resume support.
use super::StdbOidcAuthOptions;
use crate::{StdbAuthError, StdbAuthSession};

pub(crate) async fn acquire_session(
    options: StdbOidcAuthOptions,
) -> Result<StdbAuthSession, StdbAuthError> {
    unimplemented!()
}
