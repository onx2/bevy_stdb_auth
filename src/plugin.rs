use crate::{
    error::StdbAuthError,
    message::{
        StdbAuthCommandRejectedMessage, StdbAuthFailedMessage, StdbAuthLogoutFailedMessage,
        StdbAuthLogoutSucceededMessage, StdbAuthRefreshFailedMessage, StdbAuthSucceededMessage,
        StdbAuthTokenRefreshedMessage,
    },
    session::{StdbAuthCredentialMaterial, StdbAuthSession, StdbAuthSessionParts, clear_session},
    set::StdbAuthSet,
};
use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::prelude::{IntoScheduleConfigs, Resource, World, resource_exists};
use bevy_tasks::{Task, block_on, poll_once};
use bevy_time::{Time, Timer, TimerMode};
use std::time::{Duration, Instant};

/// Controls automatic refresh for sessions with refresh credentials.
#[derive(Clone, Debug)]
pub struct StdbAutoRefreshOptions {
    /// How long before expiration an automatic refresh should be requested.
    pub refresh_buffer: Duration,
    /// Delay before retrying after a failed automatic refresh.
    pub initial_retry_delay: Duration,
    /// Maximum number of failed automatic refresh attempts.
    pub max_attempts: u32,
    /// Multiplier applied to retry delay after each failure.
    pub backoff_factor: f32,
    /// Maximum delay between automatic refresh retries.
    pub max_retry_delay: Duration,
}

impl Default for StdbAutoRefreshOptions {
    fn default() -> Self {
        Self {
            refresh_buffer: Duration::from_secs(60),
            initial_retry_delay: Duration::from_secs(5),
            max_attempts: 3,
            backoff_factor: 2.0,
            max_retry_delay: Duration::from_secs(60),
        }
    }
}

/// Adds SpacetimeAuth session systems and messages to an [`App`].
#[derive(Clone, Debug)]
pub struct StdbAuthPlugin {
    /// Automatic refresh behavior for sessions with refresh credentials.
    pub auto_refresh: Option<StdbAutoRefreshOptions>,
}

impl Default for StdbAuthPlugin {
    fn default() -> Self {
        Self {
            auto_refresh: Some(StdbAutoRefreshOptions::default()),
        }
    }
}

impl Plugin for StdbAuthPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StdbAuthRefreshConfig {
            options: self.auto_refresh.clone(),
        });
        app.init_resource::<StdbAutoRefreshBackoff>();

        #[cfg(all(feature = "oidc", feature = "persistence", not(target_arch = "wasm32")))]
        crate::oidc::persistence::initialize_keyring_store_best_effort();

        app.add_message::<StdbAuthSucceededMessage>();
        app.add_message::<StdbAuthFailedMessage>();
        app.add_message::<StdbAuthCommandRejectedMessage>();
        app.add_message::<StdbAuthTokenRefreshedMessage>();
        app.add_message::<StdbAuthRefreshFailedMessage>();
        app.add_message::<StdbAuthLogoutSucceededMessage>();
        app.add_message::<StdbAuthLogoutFailedMessage>();

        app.configure_sets(
            PreUpdate,
            (
                StdbAuthSet::Command,
                StdbAuthSet::BrowserCallback,
                StdbAuthSet::AutoRefresh,
                StdbAuthSet::Poll,
            )
                .chain(),
        );

        #[cfg(all(feature = "oidc", feature = "browser", target_arch = "wasm32"))]
        app.add_systems(
            PreUpdate,
            request_browser_callback_resume.in_set(StdbAuthSet::BrowserCallback),
        );

        app.add_systems(
            PreUpdate,
            (
                request_auto_refresh.in_set(StdbAuthSet::AutoRefresh),
                poll_pending_auth
                    .run_if(resource_exists::<PendingAuthOperation>)
                    .in_set(StdbAuthSet::Poll),
            ),
        );
    }
}

#[derive(Clone, Resource)]
pub(crate) struct StdbAuthRefreshConfig {
    pub(crate) options: Option<StdbAutoRefreshOptions>,
}

#[derive(Default, Resource)]
pub(crate) struct StdbAutoRefreshBackoff {
    attempts: u32,
    current_delay: Duration,
    timer: Option<Timer>,
}

#[derive(Resource)]
pub(crate) enum PendingAuthOperation {
    Login(Task<Result<StdbAuthSessionParts, StdbAuthError>>),
    Logout(Task<Result<(), StdbAuthError>>),
    Refresh {
        task: Task<Result<StdbAuthSessionParts, StdbAuthError>>,
        automatic: bool,
    },
}

#[cfg(all(feature = "oidc", feature = "browser", target_arch = "wasm32"))]
fn request_browser_callback_resume(world: &mut World) {
    if world.contains_resource::<PendingAuthOperation>()
        || world.contains_resource::<StdbAuthSession>()
    {
        return;
    }

    if !crate::oidc::browser::pending_callback_available() {
        return;
    }

    let task = bevy_tasks::IoTaskPool::get_or_init(bevy_tasks::TaskPool::default)
        .spawn(async move { crate::oidc::browser::resume_session().await });
    world.insert_resource(PendingAuthOperation::Login(task));
}

fn request_auto_refresh(world: &mut World) {
    if world.contains_resource::<PendingAuthOperation>() {
        return;
    }

    let Some(options) = world
        .get_resource::<StdbAuthRefreshConfig>()
        .and_then(|config| config.options.clone())
    else {
        return;
    };

    if !auto_refresh_backoff_ready(world, &options) {
        return;
    }

    let Some(session) = world.get_resource::<StdbAuthSession>().cloned() else {
        return;
    };

    if !should_refresh_session(&session, options.refresh_buffer) {
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

    let task = crate::refresh::spawn_refresh_session_task(session, refresh_token);
    world.insert_resource(PendingAuthOperation::Refresh {
        task,
        automatic: true,
    });
}

fn auto_refresh_backoff_ready(world: &mut World, options: &StdbAutoRefreshOptions) -> bool {
    let delta = world
        .get_resource::<Time>()
        .map(Time::delta)
        .unwrap_or_default();
    let mut backoff = world
        .get_resource_mut::<StdbAutoRefreshBackoff>()
        .expect("StdbAutoRefreshBackoff should be inserted when StdbAuthPlugin is built");

    if options.max_attempts > 0 && backoff.attempts >= options.max_attempts {
        return false;
    }

    let Some(timer) = backoff.timer.as_mut() else {
        return true;
    };

    timer.tick(delta);

    if !timer.just_finished() {
        return false;
    }

    backoff.timer = None;
    true
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
        PendingAuthOperation::Refresh {
            mut task,
            automatic,
        } => {
            let Some(result) = block_on(poll_once(&mut task)) else {
                world.insert_resource(PendingAuthOperation::Refresh { task, automatic });
                return;
            };

            match result {
                Ok(parts) => {
                    reset_auto_refresh_backoff(world);
                    persist_refresh_token_best_effort(&parts);
                    let session = parts.session.clone();
                    world.insert_resource(parts.credentials);
                    world.insert_resource(session.clone());
                    world.write_message(StdbAuthTokenRefreshedMessage { session });
                }
                Err(error) => {
                    if automatic {
                        arm_auto_refresh_backoff(world);
                    }
                    world.write_message(StdbAuthRefreshFailedMessage {
                        message: error.to_string(),
                    });
                }
            }
        }
    }
}

fn reset_auto_refresh_backoff(world: &mut World) {
    if let Some(mut backoff) = world.get_resource_mut::<StdbAutoRefreshBackoff>() {
        backoff.attempts = 0;
        backoff.current_delay = Duration::ZERO;
        backoff.timer = None;
    }
}

fn arm_auto_refresh_backoff(world: &mut World) {
    let Some(options) = world
        .get_resource::<StdbAuthRefreshConfig>()
        .and_then(|config| config.options.clone())
    else {
        return;
    };
    let mut backoff = world
        .get_resource_mut::<StdbAutoRefreshBackoff>()
        .expect("StdbAutoRefreshBackoff should be inserted when StdbAuthPlugin is built");

    backoff.attempts = backoff.attempts.saturating_add(1);

    if options.max_attempts > 0 && backoff.attempts >= options.max_attempts {
        backoff.timer = None;
        return;
    }

    if backoff.current_delay.is_zero() {
        backoff.current_delay = options.initial_retry_delay;
    }

    let delay = backoff.current_delay.min(options.max_retry_delay);
    backoff.timer = Some(Timer::new(delay, TimerMode::Once));
    backoff.current_delay = delay
        .mul_f32(options.backoff_factor.max(1.0))
        .min(options.max_retry_delay);
}

fn apply_login_success(world: &mut World, parts: StdbAuthSessionParts) {
    reset_auto_refresh_backoff(world);
    persist_refresh_token_best_effort(&parts);
    let session = parts.session.clone();
    world.insert_resource(parts.credentials);
    world.insert_resource(session.clone());
    world.write_message(StdbAuthSucceededMessage { session });
}

#[cfg(all(feature = "oidc", feature = "persistence", not(target_arch = "wasm32")))]
fn persist_refresh_token_best_effort(parts: &StdbAuthSessionParts) {
    if parts.session.source == crate::session::StdbAuthSessionSource::Oidc
        && let (Some(client_id), Some(refresh_token)) = (
            parts.session.client_id.as_deref(),
            parts.credentials.refresh_token.as_deref(),
        )
    {
        crate::oidc::persistence::store_refresh_token_best_effort(client_id, refresh_token);
    }
}

#[cfg(not(all(feature = "oidc", feature = "persistence", not(target_arch = "wasm32"))))]
fn persist_refresh_token_best_effort(_parts: &StdbAuthSessionParts) {}

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

    fn world_with_auto_refresh(options: StdbAutoRefreshOptions) -> World {
        let mut world = World::new();
        world.insert_resource(StdbAuthRefreshConfig {
            options: Some(options),
        });
        world.init_resource::<StdbAutoRefreshBackoff>();
        world
    }

    #[test]
    fn default_plugin_enables_auto_refresh() {
        let plugin = StdbAuthPlugin::default();

        assert!(plugin.auto_refresh.is_some());
    }

    #[test]
    fn auto_refresh_backoff_stops_after_max_attempts() {
        let mut world = world_with_auto_refresh(StdbAutoRefreshOptions {
            max_attempts: 1,
            ..Default::default()
        });

        arm_auto_refresh_backoff(&mut world);

        assert!(!auto_refresh_backoff_ready(
            &mut world,
            &StdbAutoRefreshOptions {
                max_attempts: 1,
                ..Default::default()
            }
        ));
    }

    #[test]
    fn auto_refresh_backoff_resets() {
        let mut world = world_with_auto_refresh(StdbAutoRefreshOptions::default());

        arm_auto_refresh_backoff(&mut world);
        reset_auto_refresh_backoff(&mut world);

        let backoff = world.resource::<StdbAutoRefreshBackoff>();
        assert_eq!(backoff.attempts, 0);
        assert!(backoff.current_delay.is_zero());
        assert!(backoff.timer.is_none());
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
