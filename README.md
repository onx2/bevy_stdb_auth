# bevy_stdb_auth

A [Bevy](https://bevy.org/)-native integration for the [SpacetimeAuth](https://spacetimedb.com/docs/core-concepts/authentication/spacetimeauth/) issuer.

[![crates.io](https://img.shields.io/crates/v/bevy_stdb_auth)](https://crates.io/crates/bevy_stdb_auth)
![Dependabot](https://img.shields.io/badge/dependabot-enabled-brightgreen.svg)
[![docs.rs](https://docs.rs/bevy_stdb_auth/badge.svg)](https://docs.rs/bevy_stdb_auth)
[![CI](https://github.com/onx2/bevy_stdb_auth/actions/workflows/ci.yml/badge.svg)](https://github.com/onx2/bevy_stdb_auth/actions/workflows/ci.yml?query=branch%3Amain)
[![CodeQL](https://github.com/onx2/bevy_stdb_auth/actions/workflows/github-code-scanning/codeql/badge.svg)](https://github.com/onx2/bevy_stdb_auth/actions/workflows/github-code-scanning/codeql)

## Overview

`bevy_stdb_auth` adapts SpacetimeAuth login, refresh, logout, and session lifecycle state into Bevy-style resources, systems, plugins, commands, and messages.

This crate is intentionally scoped to SpacetimeAuth and does not manage SpacetimeDB connections directly. Applications decide how to use auth tokens, including passing them to [bevy_stdb](https://github.com/onx2/bevy_stdb), directly with the SpacetimeDB SDK, HTTP clients, or some other recipient.

## Features

- **Command interface** for login, logout, manual refresh requests, and pending-operation cancellation through `StdbAuthCommands`
- **Current auth state** through `StdbAuthSession`
- **Lifecycle messages** for login, refresh, and logout
- **SpacetimeAuth OIDC support** for native and browser clients through the default `oidc` feature
- **Native OIDC refresh-token persistence** through the opt-in `persistence` feature
- **SpacetimeAuth Steam support** for native Steam ticket exchange through the opt-in `steam` feature

## Example

```rust
use bevy::prelude::*;
use bevy_stdb_auth::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(StdbAuthPlugin::default())
        .add_systems(Startup, login_with_oidc)
        .add_systems(Update, on_auth_succeeded)
        .run();
}

fn login_with_oidc(mut auth: StdbAuthCommands) {
    // These options should match your SpacetimeAuth client configuration
    let options = StdbOidcAuthOptions {
        client_id: "my-client-id".to_string(),
        redirect_uri: "http://127.0.0.1:3000/callback".to_string(),
        post_logout_redirect_uri: None,
        scopes: vec![String::from("openid"), String::from("profile"), String::from("email")],
        prompt: StdbOidcPrompt::None,
    };

    if let Err(error) = auth.login(StdbLoginOptions::new(StdbAuthSource::Oidc(options))) {
        warn!("login request rejected: {error}");
    }
}

fn on_auth_succeeded(mut messages: ReadStdbAuthSucceededMessage) {
    for message in messages.read() {
        info!("authenticated with token type: {}", message.session.token_type);
    }
}
```

## Authentication flows

`bevy_stdb_auth` supports the SpacetimeAuth login methods directly. It is not a generic authentication abstraction.

| Source | Feature | Targets | Behavior |
|---|---|---|---|
| `StdbAuthSource::Oidc` | `oidc` | native, browser | SpacetimeAuth OIDC authorization-code-with-PKCE flow |
| `StdbAuthSource::Steam` | `steam` | native | Steam Web API ticket exchange through SpacetimeAuth |

### Native OIDC

Native OIDC uses the system browser and a loopback redirect listener. The configured redirect URI must use `http`, a loopback host, a non-zero explicit port, and no query string.

- build authorization URL with PKCE and CSRF state
- open the system browser
- listen for the redirect on a local callback URL
- exchange the authorization code for a SpacetimeAuth token response
- normalize the response into `StdbAuthSession`

When the `persistence` feature is enabled on native targets, OIDC refresh tokens are stored in the native OS keyring on a best-effort basis. On the next login attempt, the crate tries the stored refresh token before opening a browser.

### Browser OIDC

Browser OIDC uses browser redirects:

- store temporary OIDC pending state in `sessionStorage` with a short TTL
- redirect with `window.location`
- resume the callback after reload
- exchange the authorization code for a token response
- clean callback parameters from the browser URL
- resume callbacks automatically when a pending browser authorization is detected

Persistent browser refresh-token storage is intentionally not exposed yet. This is because it is insecure to store refresh tokens in browser Storage APIs.

### Steam

Steam support is native-only and scoped to SpacetimeAuth's Steam ticket exchange flow:

1. request a Steam Web API ticket through Steamworks
2. hex-encode the ticket
3. exchange the ticket with the SpacetimeAuth token endpoint
4. normalize the token response into `StdbAuthSession`

Steam does not use persisted refresh-token recovery. This is because it is native and doesn't require a web browser callback loop to work.

## HTTP transport

`StdbAuthPlugin::default()` uses the fixed SpacetimeAuth endpoints. Token endpoint requests use an internal 10-second timeout. The endpoint URLs are not configurable. For more information, see the [SpacetimeAuth API docs](https://spacetimedb.com/docs/core-concepts/authentication/spacetimeauth/).

| Endpoint | URL |
|---|---|
| Authorization | `https://auth.spacetimedb.com/oidc/auth` |
| Token | `https://auth.spacetimedb.com/oidc/token` |
| End session | `https://auth.spacetimedb.com/oidc/session/end` |

## Commands

Use `StdbAuthCommands` from normal Bevy systems to manage auth state.

Command methods return `Result<(), StdbAuthCommandError>` when a request cannot be accepted against the currently visible world state. Broader auth lifecycle failures use `StdbAuthError`, and command rejections convert into `StdbAuthError::Command` when a unified error type is needed. Deferred same-frame rejections are available through `ReadStdbAuthCommandRejectedMessage`.

| Method | Behavior |
|---|---|
| `login` | Starts a login flow using `StdbLoginOptions` |
| `logout` | Clears the current session and ends the SpacetimeAuth provider session by default |
| `refresh_now` | Requests an immediate token refresh when refresh credentials are available |
| `cancel_pending` | Clears local pending auth task state when possible |

```rust
use bevy_stdb_auth::prelude::*;

fn logout(mut auth: StdbAuthCommands) {
    if let Err(error) = auth.logout(StdbLogoutOptions::default()) {
        warn!("logout request rejected: {error}");
    }
}
```

`StdbLogoutOptions::default()` ends the SpacetimeAuth provider session through `https://auth.spacetimedb.com/oidc/session/end` and retains persisted refresh credentials. Set `end_provider_session` to `false` for local-only logout. Set `forget_device` to remove persisted refresh credentials from this device when the `persistence` feature is enabled.

## Token refresh

Sessions with refresh credentials can be refreshed manually through `StdbAuthCommands::refresh_now`. When `StdbAuthPlugin::auto_refresh` is `Some`, the plugin requests a refresh before expiration using `StdbAutoRefreshOptions::refresh_buffer`.

`StdbAuthPlugin::default()` enables automatic refresh with retry backoff. Set `auto_refresh` to `None` to disable it, or provide `StdbAutoRefreshOptions` to configure the refresh buffer, retry delay, max attempts, backoff factor, and max retry delay.

If SpacetimeAuth returns a rotated refresh token, the crate replaces the stored credential material and updates native keyring persistence when enabled. If the refresh response omits a refresh token, the existing refresh token is retained.

## Session resource

A successful login inserts `StdbAuthSession` as a Bevy resource.

```rust
use bevy::prelude::*;
use bevy_stdb_auth::prelude::*;

fn read_auth_session(session: Option<Res<StdbAuthSession>>) {
    if let Some(session) = session {
        info!("token type: {}", session.token_type);
    }
}
```

`StdbAuthSession` stores:

- access token
- token type
- optional expiration instant
- whether refresh credentials are available
- optional scope string
- optional client ID
- session source kind
- optional post-logout redirect URI

Refresh tokens and raw OIDC ID tokens are kept in internal credential resources instead of `StdbAuthSession` or lifecycle messages.

## Lifecycle readers

`bevy_stdb_auth` emits internal Bevy messages for auth lifecycle events. Applications read them through public reader aliases:

- `ReadStdbAuthSucceededMessage`
- `ReadStdbAuthFailedMessage`
- `ReadStdbAuthCommandRejectedMessage`
- `ReadStdbAuthTokenRefreshedMessage`
- `ReadStdbAuthRefreshFailedMessage`
- `ReadStdbAuthLogoutSucceededMessage`
- `ReadStdbAuthLogoutFailedMessage`

Applications can use these readers to route UI, update connection tokens, reconnect clients, or clear local game state. The concrete message types are intentionally not exported; auth state changes should be requested through `StdbAuthCommands`.

## Integrating with `bevy_stdb`

`bevy_stdb_auth` does not directly depend on [`bevy_stdb`](https://github.com/onx2/bevy_stdb). Connect the two crates with small glue systems.

```rust
use bevy::prelude::*;
use bevy_stdb::prelude::*;
use bevy_stdb_auth::prelude::*;
use crate::module_bindings::{DbConnection, RemoteModule};

pub type StdbCmds<'w, 's> = StdbCommands<'w, 's, DbConnection, RemoteModule>;

fn connect_on_auth_success(
    mut messages: ReadStdbAuthSucceededMessage,
    mut stdb: StdbCmds,
) {
    for message in messages.read() {
        stdb.connect(StdbConnectOptions::from_token(
            message.session.access_token.clone(),
        ));
    }
}
```

Token refresh can be handled the same way by listening for `StdbAuthTokenRefreshedMessage` and updating or reconnecting the SpacetimeDB client according to your app's policy.

## Feature flags

| Feature | Purpose |
|---|---|
| `oidc` | SpacetimeAuth OIDC authorization-code flow support; enabled by default |
| `steam` | Native SpacetimeAuth Steam ticket exchange support |
| `browser` | Browser runtime support for OIDC redirects and callback resume |
| `persistence` | Native OIDC refresh-token persistence using the OS keyring |

The default feature set is `oidc` only. Enable `steam` and `persistence` explicitly for native apps that need them.

For apps targeting both native and browser, configure features per target so native builds can include Steam and keyring persistence without enabling those dependencies for WASM:

```toml
[dependencies]
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
bevy_stdb_auth = { version = "0.1", default-features = false, features = ["oidc", "persistence", "steam"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
bevy_stdb_auth = { version = "0.1", default-features = false, features = ["oidc", "browser"] }
```

The `steam` and `persistence` features are native-only and are rejected on `wasm32` targets. Browser OIDC builds must enable `oidc` and `browser` together with `default-features = false`. Native all-feature builds still use the native OIDC implementation even when the `browser` feature is enabled.

## Compatibility

| bevy_stdb_auth | Bevy | MSRV |
|---|---|---|
| 0.1 | 0.18 | 1.89 |
| 0.2 | 0.19 | 1.95 |

## Notes

This crate focuses on SpacetimeAuth session lifecycle management. It intentionally does not manage SpacetimeDB connections directly.

Use `bevy_stdb_auth` when you want Bevy-native authentication state and lifecycle messages. Use `bevy_stdb` or the SpacetimeDB SDK directly to decide how those tokens are applied to your app's connections.
