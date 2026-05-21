use crate::{
    error::StdbAuthError, message::StdbAuthCommandRejectedMessage, plugin::PendingAuthOperation,
    session::StdbAuthSession, source::StdbAuthSource,
};
use bevy_ecs::{
    message::Messages,
    prelude::{Commands, Res, World},
    system::{Command, SystemParam},
};
use bevy_tasks::{IoTaskPool, TaskPool};
use thiserror::Error;

/// The kind of authentication operation requested by [`StdbAuthCommands`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StdbAuthOperationKind {
    /// A login operation.
    Login,
    /// A logout operation.
    Logout,
    /// A token refresh operation.
    Refresh,
    /// A pending-operation cancellation.
    Cancel,
}

/// An error returned when an authentication command cannot be accepted.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum StdbAuthCommandError {
    /// Another authentication operation is already pending.
    #[error("another authentication operation is already pending")]
    PendingOperation,
    /// No authentication session is active.
    #[error("no authentication session is active")]
    NoSession,
    /// No authentication operation is pending.
    #[error("no authentication operation is pending")]
    NoPendingOperation,
    /// The active session cannot be refreshed.
    #[error("the active authentication session cannot be refreshed")]
    MissingRefreshToken,
    /// The command is not supported by the active authentication source.
    #[error("unsupported authentication command: {0}")]
    Unsupported(String),
}

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
    /// Requests a login flow using [`StdbLoginOptions`].
    pub fn login(&mut self, options: StdbLoginOptions) -> Result<(), StdbAuthCommandError> {
        self.ensure_no_visible_pending_operation()?;
        self.commands.queue(StartLoginCommand { options });
        Ok(())
    }

    /// Requests a logout flow for the current [`StdbAuthSession`].
    pub fn logout(&mut self, options: StdbLogoutOptions) -> Result<(), StdbAuthCommandError> {
        self.ensure_no_visible_pending_operation()?;

        if self.session.is_none() {
            return Err(StdbAuthCommandError::NoSession);
        }

        self.commands.queue(StartLogoutCommand { options });
        Ok(())
    }

    /// Requests an immediate token refresh for the current [`StdbAuthSession`].
    pub fn refresh_now(&mut self) -> Result<(), StdbAuthCommandError> {
        self.ensure_no_visible_pending_operation()?;

        let Some(session) = self.session.as_deref() else {
            return Err(StdbAuthCommandError::NoSession);
        };

        if session.refresh_token.is_none() {
            return Err(StdbAuthCommandError::MissingRefreshToken);
        }

        self.commands.queue(StartRefreshCommand);
        Ok(())
    }

    /// Requests cancellation of the current authentication operation.
    pub fn cancel_pending(&mut self) -> Result<(), StdbAuthCommandError> {
        if self.pending_auth.is_none() {
            return Err(StdbAuthCommandError::NoPendingOperation);
        }

        self.commands.queue(CancelPendingAuthCommand);
        Ok(())
    }

    fn ensure_no_visible_pending_operation(&self) -> Result<(), StdbAuthCommandError> {
        if self.pending_auth.is_some() {
            return Err(StdbAuthCommandError::PendingOperation);
        }

        Ok(())
    }
}

struct StartLoginCommand {
    options: StdbLoginOptions,
}

impl Command for StartLoginCommand {
    fn apply(self, world: &mut World) {
        if reject_if_pending(world, StdbAuthOperationKind::Login) {
            return;
        }

        let source = self.options.source;
        let task = IoTaskPool::get_or_init(TaskPool::default)
            .spawn(async move { source.acquire_session().await });
        world.insert_resource(PendingAuthOperation::Login(task));
    }
}

struct StartLogoutCommand {
    options: StdbLogoutOptions,
}

impl Command for StartLogoutCommand {
    fn apply(self, world: &mut World) {
        if reject_if_pending(world, StdbAuthOperationKind::Logout) {
            return;
        }

        if !world.contains_resource::<StdbAuthSession>() {
            reject_auth_command(
                world,
                StdbAuthOperationKind::Logout,
                StdbAuthCommandError::NoSession,
            );
            return;
        }

        let _options = self.options;
        let task =
            IoTaskPool::get_or_init(TaskPool::default).spawn(async { Ok::<(), StdbAuthError>(()) });
        world.insert_resource(PendingAuthOperation::Logout(task));
    }
}

struct StartRefreshCommand;

impl Command for StartRefreshCommand {
    fn apply(self, world: &mut World) {
        if reject_if_pending(world, StdbAuthOperationKind::Refresh) {
            return;
        }

        let Some(session) = world.get_resource::<StdbAuthSession>() else {
            reject_auth_command(
                world,
                StdbAuthOperationKind::Refresh,
                StdbAuthCommandError::NoSession,
            );
            return;
        };

        if session.refresh_token.is_none() {
            reject_auth_command(
                world,
                StdbAuthOperationKind::Refresh,
                StdbAuthCommandError::MissingRefreshToken,
            );
            return;
        }

        let task = IoTaskPool::get_or_init(TaskPool::default).spawn(async {
            Err::<StdbAuthSession, StdbAuthError>(StdbAuthError::Unsupported(
                "token refresh is not implemented for this auth source".to_string(),
            ))
        });
        world.insert_resource(PendingAuthOperation::Refresh(task));
    }
}

struct CancelPendingAuthCommand;

impl Command for CancelPendingAuthCommand {
    fn apply(self, world: &mut World) {
        if world.remove_resource::<PendingAuthOperation>().is_none() {
            reject_auth_command(
                world,
                StdbAuthOperationKind::Cancel,
                StdbAuthCommandError::NoPendingOperation,
            );
        }
    }
}

fn reject_if_pending(world: &mut World, operation: StdbAuthOperationKind) -> bool {
    if world.contains_resource::<PendingAuthOperation>() {
        reject_auth_command(world, operation, StdbAuthCommandError::PendingOperation);
        return true;
    }

    false
}

fn reject_auth_command(
    world: &mut World,
    operation: StdbAuthOperationKind,
    error: StdbAuthCommandError,
) {
    if let Some(mut messages) = world.get_resource_mut::<Messages<StdbAuthCommandRejectedMessage>>()
    {
        messages.write(StdbAuthCommandRejectedMessage { operation, error });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::StdbAuthSessionSource;

    fn session_with_refresh_token() -> StdbAuthSession {
        StdbAuthSession {
            access_token: "access".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: None,
            refresh_token: Some("refresh".to_string()),
            scope: None,
            id_token: None,
            client_id: Some("client".to_string()),
            source: StdbAuthSessionSource::Oidc,
            post_logout_redirect_uri: None,
        }
    }

    fn world_with_rejection_messages() -> World {
        let mut world = World::new();
        world.init_resource::<Messages<StdbAuthCommandRejectedMessage>>();
        world
    }

    #[test]
    fn refresh_admission_rejects_second_same_frame_request() {
        let mut world = world_with_rejection_messages();
        world.insert_resource(session_with_refresh_token());

        StartRefreshCommand.apply(&mut world);
        StartRefreshCommand.apply(&mut world);

        assert!(world.contains_resource::<PendingAuthOperation>());

        let messages = world.resource::<Messages<StdbAuthCommandRejectedMessage>>();
        let rejected = messages.iter_current_update_messages().collect::<Vec<_>>();

        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].operation, StdbAuthOperationKind::Refresh);
        assert_eq!(rejected[0].error, StdbAuthCommandError::PendingOperation);
    }

    #[test]
    fn refresh_admission_rejects_missing_session() {
        let mut world = world_with_rejection_messages();

        StartRefreshCommand.apply(&mut world);

        let messages = world.resource::<Messages<StdbAuthCommandRejectedMessage>>();
        let rejected = messages.iter_current_update_messages().collect::<Vec<_>>();

        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].operation, StdbAuthOperationKind::Refresh);
        assert_eq!(rejected[0].error, StdbAuthCommandError::NoSession);
    }
}
