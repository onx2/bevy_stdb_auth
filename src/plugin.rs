use crate::{
    error::StdbAuthError,
    message::{
        StdbAuthFailedMessage, StdbAuthLogoutFailedMessage, StdbAuthLogoutSucceededMessage,
        StdbAuthRefreshFailedMessage, StdbAuthSessionClearedMessage, StdbAuthSucceededMessage,
        StdbAuthTokenRefreshedMessage,
    },
    session::StdbAuthSession,
};
use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::prelude::{IntoScheduleConfigs, Resource, World, resource_exists};
use bevy_tasks::{Task, block_on, poll_once};
use std::time::Duration;

const DEFAULT_REFRESH_BUFFER_SECS: u64 = 60;

/// Adds SpacetimeAuth session systems and messages to an [`App`].
#[derive(Clone, Debug)]
pub struct StdbAuthPlugin {
    /// Whether sessions with refresh tokens should be refreshed automatically.
    pub auto_refresh: bool,
    /// How long before expiration an automatic refresh should be requested.
    pub refresh_buffer: Duration,
    /// Whether browser callback URLs should be resumed automatically.
    pub auto_resume_browser_callback: bool,
}

impl Default for StdbAuthPlugin {
    fn default() -> Self {
        Self {
            auto_refresh: true,
            refresh_buffer: Duration::from_secs(DEFAULT_REFRESH_BUFFER_SECS),
            auto_resume_browser_callback: true,
        }
    }
}

impl Plugin for StdbAuthPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StdbAuthSucceededMessage>();
        app.add_message::<StdbAuthFailedMessage>();
        app.add_message::<StdbAuthTokenRefreshedMessage>();
        app.add_message::<StdbAuthRefreshFailedMessage>();
        app.add_message::<StdbAuthLogoutSucceededMessage>();
        app.add_message::<StdbAuthLogoutFailedMessage>();
        app.add_message::<StdbAuthSessionClearedMessage>();

        app.add_systems(
            PreUpdate,
            poll_pending_auth.run_if(resource_exists::<PendingAuthOperation>),
        );
    }
}

#[derive(Resource)]
pub(crate) enum PendingAuthOperation {
    Login(Task<Result<StdbAuthSession, StdbAuthError>>),
    Logout(Task<Result<(), StdbAuthError>>),
    Refresh(Task<Result<StdbAuthSession, StdbAuthError>>),
    Clear,
}

fn poll_pending_auth(world: &mut World) {
    let Some(pending) = world.remove_resource::<PendingAuthOperation>() else {
        return;
    };

    match pending {
        PendingAuthOperation::Login(mut task) => {
            let Some(result) = block_on(poll_once(&mut task)) else {
                world.insert_resource(PendingAuthOperation::Login(task));
                return;
            };

            match result {
                Ok(session) => apply_login_success(world, session),
                Err(error) => {
                    world.write_message(StdbAuthFailedMessage {
                        message: error.to_string(),
                    });
                }
            }
        }
        PendingAuthOperation::Logout(mut task) => {
            let Some(result) = block_on(poll_once(&mut task)) else {
                world.insert_resource(PendingAuthOperation::Logout(task));
                return;
            };

            clear_session(world);

            match result {
                Ok(()) => {
                    world.write_message_default::<StdbAuthLogoutSucceededMessage>();
                }
                Err(error) => {
                    world.write_message(StdbAuthLogoutFailedMessage {
                        message: error.to_string(),
                    });
                }
            }
        }
        PendingAuthOperation::Refresh(mut task) => {
            let Some(result) = block_on(poll_once(&mut task)) else {
                world.insert_resource(PendingAuthOperation::Refresh(task));
                return;
            };

            match result {
                Ok(session) => {
                    world.insert_resource(session.clone());
                    world.write_message(StdbAuthTokenRefreshedMessage { session });
                }
                Err(error) => {
                    world.write_message(StdbAuthRefreshFailedMessage {
                        message: error.to_string(),
                    });
                }
            }
        }
        PendingAuthOperation::Clear => {
            clear_session(world);
            world.write_message_default::<StdbAuthSessionClearedMessage>();
        }
    }
}

fn apply_login_success(world: &mut World, session: StdbAuthSession) {
    world.insert_resource(session.clone());
    world.write_message(StdbAuthSucceededMessage { session });
}

fn clear_session(world: &mut World) {
    world.remove_resource::<StdbAuthSession>();
}
