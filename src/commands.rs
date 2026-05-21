use crate::{
    error::StdbAuthError, plugin::PendingAuthOperation, session::StdbAuthSession,
    source::StdbAuthSource,
};
use bevy_ecs::{
    prelude::{Commands, Res},
    system::SystemParam,
};
use bevy_tasks::IoTaskPool;

/// Options for starting an authentication flow.
#[derive(Clone, Debug)]
pub struct StdbLoginOptions {
    /// The authentication source used to acquire a session.
    pub source: StdbAuthSource,
}

impl StdbLoginOptions {
    /// Creates [`StdbLoginOptions`] with the given [`StdbAuthSource`].
    pub fn new(source: StdbAuthSource) -> Self {
        Self { source }
    }
}

/// Options for logging out of the current authentication session.
#[derive(Clone, Debug, Default)]
pub struct StdbLogoutOptions {
    /// Whether provider-backed local credentials should be cleared.
    pub clear_stored_credentials: bool,
}

/// Sends authentication commands from Bevy systems.
#[derive(SystemParam)]
pub struct StdbAuthCommands<'w, 's> {
    commands: Commands<'w, 's>,
    pending_auth: Option<Res<'w, PendingAuthOperation>>,
    session: Option<Res<'w, StdbAuthSession>>,
}

impl StdbAuthCommands<'_, '_> {
    /// Starts a login flow using [`StdbLoginOptions`].
    pub fn login(&mut self, options: StdbLoginOptions) {
        if self.pending_auth.is_some() {
            return;
        }

        let source = options.source;
        let task = IoTaskPool::get().spawn(async move { source.acquire_session().await });
        self.commands
            .insert_resource(PendingAuthOperation::Login(task));
    }

    /// Starts a logout flow for the current [`StdbAuthSession`].
    pub fn logout(&mut self, _options: StdbLogoutOptions) {
        if self.pending_auth.is_some() {
            return;
        }

        let task = IoTaskPool::get().spawn(async { Ok(()) });
        self.commands
            .insert_resource(PendingAuthOperation::Logout(task));
    }

    /// Requests an immediate token refresh for the current [`StdbAuthSession`].
    pub fn refresh_now(&mut self) {
        if self.pending_auth.is_some() {
            return;
        }

        let Some(session) = self.session.as_deref() else {
            return;
        };

        if session.refresh_token.is_none() {
            let task = IoTaskPool::get().spawn(async {
                Err(StdbAuthError::Unsupported(
                    "the current auth session does not include a refresh token".to_string(),
                ))
            });
            self.commands
                .insert_resource(PendingAuthOperation::Refresh(task));
            return;
        }

        let task = IoTaskPool::get().spawn(async {
            Err(StdbAuthError::Unsupported(
                "token refresh is not implemented for this auth source".to_string(),
            ))
        });
        self.commands
            .insert_resource(PendingAuthOperation::Refresh(task));
    }

    /// Clears any local pending authentication operation.
    pub fn cancel_pending(&mut self) {
        self.commands.remove_resource::<PendingAuthOperation>();
    }
}
