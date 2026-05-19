//! Native keyring persistence for refresh credentials.

#![allow(dead_code)]

const SERVICE: &str = "bevy_stdb_auth";

/// Configures the platform credential store for [`keyring_core::Entry`].
#[cfg(target_os = "linux")]
pub(crate) fn initialize_keyring_store() -> keyring_core::Result<()> {
    keyring_core::set_default_store(zbus_secret_service_keyring_store::Store::new()?);
    Ok(())
}

/// Configures the platform credential store for [`keyring_core::Entry`].
#[cfg(target_os = "macos")]
pub(crate) fn initialize_keyring_store() -> keyring_core::Result<()> {
    keyring_core::set_default_store(apple_native_keyring_store::keychain::Store::new()?);
    Ok(())
}

/// Configures the platform credential store for [`keyring_core::Entry`].
#[cfg(target_os = "windows")]
pub(crate) fn initialize_keyring_store() -> keyring_core::Result<()> {
    keyring_core::set_default_store(windows_native_keyring_store::Store::new()?);
    Ok(())
}

/// Stores the refresh token associated with `client_id`.
pub(crate) fn store_refresh_token(
    client_id: &str,
    refresh_token: &str,
) -> keyring_core::Result<()> {
    keyring_core::Entry::new(SERVICE, client_id)?.set_password(refresh_token)
}

/// Returns the refresh token associated with `client_id`.
pub(crate) fn stored_refresh_token(client_id: &str) -> keyring_core::Result<String> {
    keyring_core::Entry::new(SERVICE, client_id)?.get_password()
}

/// Clears the refresh token associated with `client_id`.
pub(crate) fn clear_refresh_token(client_id: &str) -> keyring_core::Result<()> {
    keyring_core::Entry::new(SERVICE, client_id)?.delete_credential()
}
