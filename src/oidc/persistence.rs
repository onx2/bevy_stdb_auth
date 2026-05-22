//! Native keyring persistence for refresh credentials.

const SERVICE: &str = "bevy_stdb_auth";

/// Configures the platform credential store for [`keyring_core::Entry`].
#[cfg(target_os = "linux")]
fn initialize_keyring_store() -> keyring_core::Result<()> {
    keyring_core::set_default_store(zbus_secret_service_keyring_store::Store::new()?);
    Ok(())
}

/// Configures the platform credential store for [`keyring_core::Entry`].
#[cfg(target_os = "macos")]
fn initialize_keyring_store() -> keyring_core::Result<()> {
    keyring_core::set_default_store(apple_native_keyring_store::keychain::Store::new()?);
    Ok(())
}

/// Configures the platform credential store for [`keyring_core::Entry`].
#[cfg(target_os = "windows")]
fn initialize_keyring_store() -> keyring_core::Result<()> {
    keyring_core::set_default_store(windows_native_keyring_store::Store::new()?);
    Ok(())
}

/// Configures the platform credential store for [`keyring_core::Entry`].
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn initialize_keyring_store() -> keyring_core::Result<()> {
    Err(keyring_core::Error::NotSupportedByStore(
        "native refresh-token persistence is supported only on Linux, macOS, and Windows"
            .to_string(),
    ))
}

/// Configures the platform credential store when available.
pub(crate) fn initialize_keyring_store_best_effort() {
    let _ = initialize_keyring_store();
}

/// Stores the refresh token associated with `client_id`.
fn store_refresh_token(client_id: &str, refresh_token: &str) -> keyring_core::Result<()> {
    keyring_core::Entry::new(SERVICE, client_id)?.set_password(refresh_token)
}

/// Stores the refresh token associated with `client_id` when persistence is available.
pub(crate) fn store_refresh_token_best_effort(client_id: &str, refresh_token: &str) {
    initialize_keyring_store_best_effort();
    let _ = store_refresh_token(client_id, refresh_token);
}

/// Returns the refresh token associated with `client_id`.
fn stored_refresh_token(client_id: &str) -> keyring_core::Result<String> {
    keyring_core::Entry::new(SERVICE, client_id)?.get_password()
}

/// Returns the refresh token associated with `client_id` when persistence is available.
pub(crate) fn stored_refresh_token_best_effort(client_id: &str) -> Option<String> {
    initialize_keyring_store_best_effort();
    stored_refresh_token(client_id).ok().and_then(|token| {
        let token = token.trim().to_string();
        (!token.is_empty()).then_some(token)
    })
}

/// Clears the refresh token associated with `client_id`.
fn clear_refresh_token(client_id: &str) -> keyring_core::Result<()> {
    keyring_core::Entry::new(SERVICE, client_id)?.delete_credential()
}

/// Clears the refresh token associated with `client_id` when persistence is available.
pub(crate) fn clear_refresh_token_best_effort(client_id: &str) {
    initialize_keyring_store_best_effort();
    let _ = clear_refresh_token(client_id);
}
