//! Steam ticket exchange support for SpacetimeAuth.

/// Options for authenticating with a Steam Web API ticket.
#[derive(Clone, Debug)]
pub struct StdbSteamAuthOptions {
    /// The OAuth client identifier.
    pub client_id: String,
    /// The unique identifier for the Steam application.
    pub app_id: u32,
}
