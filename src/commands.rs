use crate::{
    error::StdbAuthError,
    message::StdbAuthCommandRejectedMessage,
    plugin::PendingAuthOperation,
    session::{StdbAuthCredentialMaterial, StdbAuthSession},
    source::StdbAuthSource,
    transport::StdbAuthTransportConfig,
};
use bevy_ecs::{
    message::Messages,
    prelude::{Commands, Res, World},
    system::{Command, SystemParam},
};
use bevy_tasks::{IoTaskPool, TaskPool};
use thiserror::Error;
use url::Url;

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
    /// The active session does not include a client ID.
    #[error("the active authentication session does not include a client ID")]
    MissingClientId,
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
#[derive(Clone, Debug)]
pub struct StdbLogoutOptions {
    /// Whether the SpacetimeAuth provider session should be ended.
    pub end_provider_session: bool,
    /// Whether provider-backed local credentials should be cleared.
    pub clear_stored_credentials: bool,
}

impl Default for StdbLogoutOptions {
    fn default() -> Self {
        Self {
            end_provider_session: true,
            clear_stored_credentials: false,
        }
    }
}

/// Sends authentication commands from Bevy systems.
#[derive(SystemParam)]
pub struct StdbAuthCommands<'w, 's> {
    commands: Commands<'w, 's>,
    pending_auth: Option<Res<'w, PendingAuthOperation>>,
    session: Option<Res<'w, StdbAuthSession>>,
    credentials: Option<Res<'w, StdbAuthCredentialMaterial>>,
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

        if self.session.is_none() {
            return Err(StdbAuthCommandError::NoSession);
        }

        let can_refresh = self
            .credentials
            .as_deref()
            .is_some_and(StdbAuthCredentialMaterial::has_refresh_token);

        if !can_refresh {
            return Err(StdbAuthCommandError::MissingRefreshToken);
        }

        if self
            .session
            .as_deref()
            .and_then(|session| session.client_id.as_ref())
            .is_none()
        {
            return Err(StdbAuthCommandError::MissingClientId);
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
        let transport_config = world
            .get_resource::<StdbAuthTransportConfig>()
            .cloned()
            .unwrap_or_default();
        let task = IoTaskPool::get_or_init(TaskPool::default)
            .spawn(async move { source.acquire_session(transport_config).await });
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

        let Some(session) = world.get_resource::<StdbAuthSession>().cloned() else {
            reject_auth_command(
                world,
                StdbAuthOperationKind::Logout,
                StdbAuthCommandError::NoSession,
            );
            return;
        };

        let id_token_hint = world
            .get_resource::<StdbAuthCredentialMaterial>()
            .and_then(|credentials| credentials.id_token.clone());
        let transport_config = world
            .get_resource::<StdbAuthTransportConfig>()
            .cloned()
            .unwrap_or_default();
        let options = self.options;
        let task = IoTaskPool::get_or_init(TaskPool::default).spawn(async move {
            if options.clear_stored_credentials {
                clear_persisted_credentials_best_effort(&session);
            }

            if options.end_provider_session {
                end_provider_session(&session, id_token_hint.as_deref(), &transport_config)?;
            }

            Ok::<(), StdbAuthError>(())
        });
        world.insert_resource(PendingAuthOperation::Logout(task));
    }
}

struct StartRefreshCommand;

impl Command for StartRefreshCommand {
    fn apply(self, world: &mut World) {
        if reject_if_pending(world, StdbAuthOperationKind::Refresh) {
            return;
        }

        if !world.contains_resource::<StdbAuthSession>() {
            reject_auth_command(
                world,
                StdbAuthOperationKind::Refresh,
                StdbAuthCommandError::NoSession,
            );
            return;
        }

        let Some(refresh_token) = world
            .get_resource::<StdbAuthCredentialMaterial>()
            .and_then(|credentials| credentials.refresh_token.clone())
        else {
            reject_auth_command(
                world,
                StdbAuthOperationKind::Refresh,
                StdbAuthCommandError::MissingRefreshToken,
            );
            return;
        };

        let session = world.resource::<StdbAuthSession>().clone();
        if session.client_id.is_none() {
            reject_auth_command(
                world,
                StdbAuthOperationKind::Refresh,
                StdbAuthCommandError::MissingClientId,
            );
            return;
        }

        let transport_config = world
            .get_resource::<StdbAuthTransportConfig>()
            .cloned()
            .unwrap_or_default();
        let task =
            crate::refresh::spawn_refresh_session_task(session, refresh_token, transport_config);
        world.insert_resource(PendingAuthOperation::Refresh {
            task,
            automatic: false,
        });
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

fn end_provider_session(
    session: &StdbAuthSession,
    id_token_hint: Option<&str>,
    transport_config: &StdbAuthTransportConfig,
) -> Result<(), StdbAuthError> {
    if session.source != crate::session::StdbAuthSessionSource::Oidc {
        return Ok(());
    }

    let end_session_url = build_end_session_url(session, id_token_hint, transport_config);

    #[cfg(all(feature = "oidc", feature = "browser", target_arch = "wasm32"))]
    {
        web_sys::window()
            .ok_or_else(|| StdbAuthError::Internal("browser window is unavailable".to_string()))?
            .location()
            .assign(end_session_url.as_str())
            .map_err(|error| {
                StdbAuthError::Internal(format!(
                    "failed to redirect to SpacetimeAuth logout: {error:?}"
                ))
            })?;
    }

    #[cfg(all(feature = "oidc", not(target_arch = "wasm32")))]
    {
        webbrowser::open(end_session_url.as_str()).map_err(|error| {
            StdbAuthError::Internal(format!("failed to open SpacetimeAuth logout URL: {error}"))
        })?;
    }

    #[cfg(not(feature = "oidc"))]
    let _ = end_session_url;

    Ok(())
}

fn build_end_session_url(
    session: &StdbAuthSession,
    id_token_hint: Option<&str>,
    transport_config: &StdbAuthTransportConfig,
) -> Url {
    let mut end_session_url = Url::parse(transport_config.end_session_endpoint_url())
        .expect("static SpacetimeAuth end-session endpoint must be valid");

    let mut params = Vec::new();

    if let Some(id_token_hint) = id_token_hint.filter(|token| !token.trim().is_empty()) {
        params.push(("id_token_hint", id_token_hint));
    }

    if let Some(post_logout_redirect_uri) = session
        .post_logout_redirect_uri
        .as_deref()
        .filter(|uri| !uri.trim().is_empty())
    {
        params.push(("post_logout_redirect_uri", post_logout_redirect_uri));
    }

    if let Some(client_id) = session
        .client_id
        .as_deref()
        .filter(|client_id| !client_id.trim().is_empty())
    {
        params.push(("client_id", client_id));
    }

    if !params.is_empty() {
        end_session_url.query_pairs_mut().extend_pairs(params);
    }

    end_session_url
}

fn clear_persisted_credentials_best_effort(session: &StdbAuthSession) {
    #[cfg(all(feature = "oidc", feature = "persistence", not(target_arch = "wasm32")))]
    if session.source == crate::session::StdbAuthSessionSource::Oidc
        && let Some(client_id) = session.client_id.as_deref()
    {
        crate::oidc::persistence::clear_refresh_token_best_effort(client_id);
    }

    #[cfg(not(all(feature = "oidc", feature = "persistence", not(target_arch = "wasm32"))))]
    let _ = session;
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

    fn session_with_refresh_credentials() -> StdbAuthSession {
        StdbAuthSession {
            access_token: "access".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: None,
            can_refresh: true,
            scope: None,
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
    fn logout_options_end_provider_session_by_default() {
        let options = StdbLogoutOptions::default();

        assert!(options.end_provider_session);
        assert!(!options.clear_stored_credentials);
    }

    #[test]
    fn end_session_url_contains_logout_context() {
        let mut session = session_with_refresh_credentials();
        session.post_logout_redirect_uri = Some("http://127.0.0.1:3000/logged-out".to_string());
        let end_session_url = build_end_session_url(
            &session,
            Some("id-token"),
            &StdbAuthTransportConfig::default(),
        );
        let params = end_session_url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect::<std::collections::BTreeMap<_, _>>();

        assert_eq!(
            end_session_url.as_str().split('?').next(),
            Some("https://auth.spacetimedb.com/oidc/session/end")
        );
        assert_eq!(params.get("client_id").map(String::as_str), Some("client"));
        assert_eq!(
            params.get("id_token_hint").map(String::as_str),
            Some("id-token")
        );
        assert_eq!(
            params.get("post_logout_redirect_uri").map(String::as_str),
            Some("http://127.0.0.1:3000/logged-out")
        );
    }

    #[test]
    fn end_session_url_omits_empty_optional_context() {
        let mut session = session_with_refresh_credentials();
        session.client_id = Some("  ".to_string());
        session.post_logout_redirect_uri = Some("  ".to_string());
        let end_session_url =
            build_end_session_url(&session, Some("  "), &StdbAuthTransportConfig::default());

        assert!(end_session_url.query().is_none());
    }

    #[test]
    fn refresh_admission_rejects_second_same_frame_request() {
        let mut world = world_with_rejection_messages();
        let task =
            IoTaskPool::get_or_init(TaskPool::default).spawn(async { Ok::<(), StdbAuthError>(()) });
        world.insert_resource(PendingAuthOperation::Logout(task));

        StartRefreshCommand.apply(&mut world);

        assert!(world.contains_resource::<PendingAuthOperation>());

        let messages = world.resource::<Messages<StdbAuthCommandRejectedMessage>>();
        let rejected = messages.iter_current_update_messages().collect::<Vec<_>>();

        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].operation, StdbAuthOperationKind::Refresh);
        assert_eq!(rejected[0].error, StdbAuthCommandError::PendingOperation);
    }

    #[test]
    fn refresh_admission_rejects_missing_credentials() {
        let mut world = world_with_rejection_messages();
        world.insert_resource(session_with_refresh_credentials());

        StartRefreshCommand.apply(&mut world);

        let messages = world.resource::<Messages<StdbAuthCommandRejectedMessage>>();
        let rejected = messages.iter_current_update_messages().collect::<Vec<_>>();

        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].operation, StdbAuthOperationKind::Refresh);
        assert_eq!(rejected[0].error, StdbAuthCommandError::MissingRefreshToken);
    }

    #[test]
    fn refresh_admission_rejects_missing_client_id() {
        let mut world = world_with_rejection_messages();
        let mut session = session_with_refresh_credentials();
        session.client_id = None;
        world.insert_resource(session);
        world.insert_resource(StdbAuthCredentialMaterial::new(
            Some("refresh".to_string()),
            None,
        ));

        StartRefreshCommand.apply(&mut world);

        let messages = world.resource::<Messages<StdbAuthCommandRejectedMessage>>();
        let rejected = messages.iter_current_update_messages().collect::<Vec<_>>();

        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].operation, StdbAuthOperationKind::Refresh);
        assert_eq!(rejected[0].error, StdbAuthCommandError::MissingClientId);
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
