use bevy_ecs::prelude::SystemSet;

/// Labels systems that manage SpacetimeAuth lifecycle work.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum StdbAuthSet {
    /// Runs application systems that submit authentication commands.
    Command,
    /// Resumes browser OIDC callbacks when running on browser targets.
    BrowserCallback,
    /// Requests automatic refresh operations for expiring sessions.
    AutoRefresh,
    /// Polls pending authentication operations and applies their results.
    Poll,
}
