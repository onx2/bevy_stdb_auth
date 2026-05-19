//! Optional credential storage backends.

#[cfg(all(feature = "oidc", not(feature = "browser")))]
mod keyring;
