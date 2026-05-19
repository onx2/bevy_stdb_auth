//! Optional refresh credential persistence backends.

#[cfg(all(target_arch = "wasm32", feature = "browser"))]
mod browser;
#[cfg(not(target_arch = "wasm32"))]
mod keyring;

/// Determines how refresh credentials are persisted.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StdbAuthPersistence {
    /// Does not persist refresh credentials.
    #[default]
    None,
    /// Stores refresh credentials in the native OS keyring.
    Keyring,
    /// Stores refresh credentials in browser `localStorage`.
    ///
    /// `localStorage` is not secure against cross-site scripting attacks.
    LocalStorage,
}
