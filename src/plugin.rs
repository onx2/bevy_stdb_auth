use crate::{
    error::StdbAuthError,
    message::{
        StdbAuthCommandRejectedMessage, StdbAuthFailedMessage, StdbAuthLogoutFailedMessage,
        StdbAuthLogoutSucceededMessage, StdbAuthRefreshFailedMessage, StdbAuthSucceededMessage,
        StdbAuthTokenRefreshedMessage,
    },
    session::{StdbAuthCredentialMaterial, StdbAuthSession, StdbAuthSessionParts, clear_session},
    transport::StdbAuthTransportConfig,
};
use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::prelude::{IntoScheduleConfigs, Resource, World, resource_exists};
use bevy_tasks::{Task, block_on, poll_once};
use std::time::{Duration, Instant};

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
    /// How long token endpoint requests may run before timing out.
    pub token_request_timeout: Duration,
}

impl Default for StdbAuthPlugin {
    fn default() -> Self {
        Self {
            auto_refresh: true,
            refresh_buffer: Duration::from_secs(DEFAULT_REFRESH_BUFFER_SECS),
            auto_resume_browser_callback: true,
            token_request_timeout: StdbAuthTransportConfig::default_token_request_timeout(),
        }
    }
}

impl Plugin for StdbAuthPlugin {
    fn build(&self, app: &mut App) {
        let transport_config = StdbAuthTransportConfig::try_new(self.token_request_timeout)
            .expect("invalid SpacetimeAuth transport configuration");
        app.insert_resource(transport_config);
        app.insert_resource(StdbAuthRefreshConfig {
            auto_refresh: self.auto_refresh,
            refresh_buffer: self.refresh_buffer,
        });

        #[cfg(all(feature = "oidc", feature = "browser", target_arch = "wasm32"))]
        app.insert_resource(StdbAuthBrowserConfig {
            auto_resume_browser_callback: self.auto_resume_browser_callback,
        });

        #[cfg(all(feature = "oidc", feature = "persistence", not(target_arch = "wasm32")))]
        crate::oidc::persistence::initialize_keyring_store_best_effort();

        app.add_message::<StdbAuthSucceededMessage>();
        app.add_message::<StdbAuthFailedMessage>();
        app.add_message::<StdbAuthCommandRejectedMessage>();
        app.add_message::<StdbAuthTokenRefreshedMessage>();
        app.add_message::<StdbAuthRefreshFailedMessage>();
        app.add_message::<StdbAuthLogoutSucceededMessage>();
        app.add_message::<StdbAuthLogoutFailedMessage>();

        app.add_systems(
            PreUpdate,
            (
                request_browser_callback_resume,
                request_auto_refresh,
                poll_pending_auth.run_if(resource_exists::<PendingAuthOperation>),
            )
                .chain(),
        );
    }
}

#[derive(Clone, Copy, Resource)]
pub(crate) struct StdbAuthRefreshConfig {
    pub(crate) auto_refresh: bool,
    pub(crate) refresh_buffer: Duration,
}

#[cfg(all(feature = "oidc", feature = "browser", target_arch = "wasm32"))]
#[derive(Clone, Copy, Resource)]
pub(crate) struct StdbAuthBrowserConfig {
    pub(crate) auto_resume_browser_callback: bool,
}

#[derive(Resource)]
pub(crate) enum PendingAuthOperation {
    Login(Task<Result<StdbAuthSessionParts, StdbAuthError>>),
    Logout(Task<Result<(), StdbAuthError>>),
    Refresh(Task<Result<StdbAuthSessionParts, StdbAuthError>>),
}

#[cfg(all(feature = "oidc", feature = "browser", target_arch = "wasm32"))]
fn request_browser_callback_resume(world: &mut World) {
    if world.contains_resource::<PendingAuthOperation>()
        || world.contains_resource::<StdbAuthSession>()
    {
        return;
    }

    let Some(config) = world.get_resource::<StdbAuthBrowserConfig>().copied() else {
        return;
    };

    if !config.auto_resume_browser_callback || !crate::oidc::browser::pending_callback_available() {
        return;
    }

    let transport_config = world
        .get_resource::<StdbAuthTransportConfig>()
        .cloned()
        .unwrap_or_default();
    let task = bevy_tasks::IoTaskPool::get_or_init(bevy_tasks::TaskPool::default)
        .spawn(async move { crate::oidc::browser::resume_session(transport_config).await });
    world.insert_resource(PendingAuthOperation::Login(task));
}

#[cfg(not(all(feature = "oidc", feature = "browser", target_arch = "wasm32")))]
fn request_browser_callback_resume(_world: &mut World) {}

fn request_auto_refresh(world: &mut World) {
    if world.contains_resource::<PendingAuthOperation>() {
        return;
    }

    let Some(config) = world.get_resource::<StdbAuthRefreshConfig>().copied() else {
        return;
    };

    if !config.auto_refresh {
        return;
    }

    let Some(session) = world.get_resource::<StdbAuthSession>().cloned() else {
        return;
    };

    if !should_refresh_session(&session, config.refresh_buffer) {
        return;
    }

    let Some(refresh_token) = world
        .get_resource::<StdbAuthCredentialMaterial>()
        .and_then(|credentials| credentials.refresh_token.clone())
    else {
        return;
    };

    if session.client_id.is_none() {
        return;
    }

    let transport_config = world
        .get_resource::<StdbAuthTransportConfig>()
        .cloned()
        .unwrap_or_default();
    let task = crate::refresh::spawn_refresh_session_task(session, refresh_token, transport_config);
    world.insert_resource(PendingAuthOperation::Refresh(task));
}

fn should_refresh_session(session: &StdbAuthSession, refresh_buffer: Duration) -> bool {
    let Some(expires_at) = session.expires_at else {
        return false;
    };

    expires_at <= Instant::now() + refresh_buffer
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
                Ok(parts) => apply_login_success(world, parts),
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
                Ok(parts) => {
                    persist_refresh_token_best_effort(&parts);
                    let session = parts.session.clone();
                    world.insert_resource(parts.credentials);
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
    }
}

fn apply_login_success(world: &mut World, parts: StdbAuthSessionParts) {
    persist_refresh_token_best_effort(&parts);
    let session = parts.session.clone();
    world.insert_resource(parts.credentials);
    world.insert_resource(session.clone());
    world.write_message(StdbAuthSucceededMessage { session });
}

fn persist_refresh_token_best_effort(parts: &StdbAuthSessionParts) {
    #[cfg(not(all(feature = "oidc", feature = "persistence", not(target_arch = "wasm32"))))]
    let _ = parts;

    #[cfg(all(feature = "oidc", feature = "persistence", not(target_arch = "wasm32")))]
    if parts.session.source == crate::session::StdbAuthSessionSource::Oidc
        && let (Some(client_id), Some(refresh_token)) = (
            parts.session.client_id.as_deref(),
            parts.credentials.refresh_token.as_deref(),
        )
    {
        crate::oidc::persistence::store_refresh_token_best_effort(client_id, refresh_token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::StdbAuthSessionSource;

    fn session(expires_at: Option<Instant>) -> StdbAuthSession {
        StdbAuthSession {
            access_token: "access".to_string(),
            token_type: "Bearer".to_string(),
            expires_at,
            can_refresh: true,
            scope: None,
            client_id: Some("client".to_string()),
            source: StdbAuthSessionSource::Oidc,
            post_logout_redirect_uri: None,
        }
    }

    #[test]
    fn should_refresh_session_inside_buffer() {
        let session = session(Some(Instant::now() + Duration::from_secs(30)));

        assert!(should_refresh_session(&session, Duration::from_secs(60)));
    }

    #[test]
    fn should_not_refresh_session_outside_buffer() {
        let session = session(Some(Instant::now() + Duration::from_secs(120)));

        assert!(!should_refresh_session(&session, Duration::from_secs(60)));
    }

    #[test]
    fn should_not_refresh_session_without_expiration() {
        let session = session(None);

        assert!(!should_refresh_session(&session, Duration::from_secs(60)));
    }
}
