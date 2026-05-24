use bevy_app::{App, AppExit, ScheduleRunnerPlugin, Startup, Update};
use bevy_ecs::prelude::{MessageWriter, ResMut, Resource};
use bevy_stdb_auth::prelude::*;
use bevy_time::{Time, TimePlugin};
use std::{env, time::Duration};

const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:3000/callback";
const DEFAULT_SCOPES: &str = "openid profile email";
const EXAMPLE_TIMEOUT: Duration = Duration::from_secs(180);
const FRAME_DELAY: Duration = Duration::from_millis(16);

#[derive(Resource)]
struct ExampleTimeout(Duration);

fn main() {
    App::new()
        .add_plugins(ScheduleRunnerPlugin::run_loop(FRAME_DELAY))
        .add_plugins(TimePlugin)
        .add_plugins(StdbAuthPlugin::default())
        .insert_resource(ExampleTimeout(EXAMPLE_TIMEOUT))
        .add_systems(Startup, start_oidc_login)
        .add_systems(Update, (observe_auth_messages, exit_on_timeout))
        .run();
}

fn start_oidc_login(mut auth: StdbAuthCommands, mut exit: MessageWriter<AppExit>) {
    let client_id = match env::var("STDB_AUTH_CLIENT_ID") {
        Ok(client_id) => client_id,
        Err(_) => {
            eprintln!("set STDB_AUTH_CLIENT_ID before running this example");
            exit.write(AppExit::error());
            return;
        }
    };
    let redirect_uri =
        env::var("STDB_AUTH_REDIRECT_URI").unwrap_or_else(|_| DEFAULT_REDIRECT_URI.to_string());
    let post_logout_redirect_uri = env::var("STDB_AUTH_POST_LOGOUT_REDIRECT_URI").ok();
    let scopes = env::var("STDB_AUTH_SCOPES")
        .unwrap_or_else(|_| DEFAULT_SCOPES.to_string())
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let options = StdbOidcAuthOptions {
        client_id,
        redirect_uri,
        post_logout_redirect_uri,
        scopes,
        prompt: StdbOidcPrompt::None,
    };

    println!("Opening your browser for SpacetimeAuth. Check your browser to continue login.");

    if let Err(error) = auth.login(StdbLoginOptions::new(StdbAuthSource::Oidc(options))) {
        eprintln!("login request rejected: {error}");
        exit.write(AppExit::error());
    }
}

fn observe_auth_messages(
    mut succeeded: ReadStdbAuthSucceededMessage,
    mut failed: ReadStdbAuthFailedMessage,
    mut refreshed: ReadStdbAuthTokenRefreshedMessage,
    mut refresh_failed: ReadStdbAuthRefreshFailedMessage,
    mut rejected: ReadStdbAuthCommandRejectedMessage,
    mut exit: MessageWriter<AppExit>,
) {
    for message in succeeded.read() {
        println!(
            "authenticated with {} token; refresh available: {}",
            message.session.token_type, message.session.can_refresh
        );
        exit.write(AppExit::Success);
    }

    for message in failed.read() {
        eprintln!("authentication failed: {}", message.message);
        exit.write(AppExit::error());
    }

    for message in refreshed.read() {
        println!(
            "token refreshed; refresh available: {}",
            message.session.can_refresh
        );
    }

    for message in refresh_failed.read() {
        eprintln!("token refresh failed: {}", message.message);
    }

    for message in rejected.read() {
        eprintln!("auth command rejected: {}", message.error);
        exit.write(AppExit::error());
    }
}

fn exit_on_timeout(
    time: ResMut<Time>,
    timeout: ResMut<ExampleTimeout>,
    mut exit: MessageWriter<AppExit>,
) {
    if time.elapsed() >= timeout.0 {
        eprintln!("example timed out before authentication completed");
        exit.write(AppExit::error());
    }
}
