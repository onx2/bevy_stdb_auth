# bevy_stdb_auth

A [Bevy](https://bevy.org/) integration for [SpacetimeAuth](https://spacetimedb.com/docs/core-concepts/authentication/spacetimeauth/) token sessions.

[![crates.io](https://img.shields.io/crates/v/bevy_stdb_auth)](https://crates.io/crates/bevy_stdb_auth)
![Dependabot](https://img.shields.io/badge/dependabot-enabled-brightgreen.svg)
[![docs.rs](https://docs.rs/bevy_stdb_auth/badge.svg)](https://docs.rs/bevy_stdb_auth)
[![CI](https://github.com/onx2/bevy_stdb_auth/actions/workflows/ci.yml/badge.svg)](https://github.com/onx2/bevy_stdb_auth/actions/workflows/ci.yml?query=branch%3Amain)
[![CodeQL](https://github.com/onx2/bevy_stdb_auth/actions/workflows/github-code-scanning/codeql/badge.svg)](https://github.com/onx2/bevy_stdb_auth/actions/workflows/github-code-scanning/codeql)

![Useless AI generated image that kind of looks cool](https://github.com/user-attachments/assets/141a4b22-1cd7-4340-8c88-277fd6af6a19)
_Please enjoy this useless AI generated image based on the README contents of this repo._



## Overview

`bevy_stdb_auth` adapts SpacetimeAuth login, refresh, logout, and session lifecycle state into Bevy-style resources, systems, plugins, commands, and messages.

This crate is intentionally scoped to SpacetimeAuth. It does not depend on `bevy_stdb`, and it does not manage SpacetimeDB connections directly. Applications decide how to use auth tokens, including passing them to `bevy_stdb`, HTTP clients, backend APIs, or game services.

## Features

- **Plugin setup** via `StdbAuthPlugin`
- **Command interface** for login, logout, manual refresh requests, and pending-operation cancellation through `StdbAuthCommands`
- **Current auth state** through `StdbAuthSession`
- **Lifecycle messages** for login, refresh, and logout
- **SpacetimeAuth OIDC support** for native and browser clients through the `oidc` feature
- **Native OIDC refresh-token persistence** through the `persistence` feature
- **SpacetimeAuth Steam support** for native Steam ticket exchange through the `steam` feature

## Example

```rust
use bevy::prelude::*;
use bevy_stdb_auth::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(StdbAuthPlugin::default())
        .add_systems(Startup, login_with_steam)
        .add_systems(Update, on_auth_succeeded)
        .run();
}

fn login_with_steam(mut auth: StdbAuthCommands) {
    auth.login(StdbLoginOptions::new(StdbAuthSource::Steam(
        StdbSteamAuthOptions {
            client_id: "my-client-id".to_string(),
            app_id: 480,
        },
    )));
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

Native OIDC uses the system browser and a loopback redirect listener:

- build authorization URL with PKCE and CSRF state
- open the system browser
- listen for the redirect on a local callback URL
- exchange the authorization code for a SpacetimeAuth token response
- normalize the response into `StdbAuthSession`

When the `persistence` feature is enabled on native targets, OIDC refresh tokens are stored in the native OS keyring. On the next login attempt, the crate can try the stored refresh token before opening a browser.

### Browser OIDC

Browser OIDC uses browser redirects:

- store temporary OIDC pending state in `sessionStorage`
- redirect with `window.location`
- resume the callback after reload
- exchange the authorization code for a token response
- clean callback parameters from the browser URL

Persistent browser refresh-token storage is intentionally not exposed yet. This is because it is insecure to store refresh tokens in via browser Storage APIs.

### Steam

Steam support is native-only and scoped to SpacetimeAuth's Steam ticket exchange flow:

1. request a Steam Web API ticket through Steamworks
2. hex-encode the ticket
3. exchange the ticket with the SpacetimeAuth token endpoint
4. normalize the token response into `StdbAuthSession`

Steam does not use persisted refresh-token recovery. This is because it is native and doesn't require a web browser callback loop to work.

## Commands

Use `StdbAuthCommands` from normal Bevy systems to manage auth state.

| Method | Behavior |
|---|---|
| `login` | Starts a login flow using `StdbLoginOptions` |
| `logout` | Clears the current session and runs provider logout when supported |
| `refresh_now` | Requests an immediate token refresh for the current session |
| `cancel_pending` | Clears local pending auth task state when possible |

```rust
use bevy_stdb_auth::prelude::*;

fn logout(mut auth: StdbAuthCommands) {
    auth.logout(StdbLogoutOptions::default());
}
```

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
- optional refresh token
- optional scope string
- optional ID token
- optional client ID
- session source kind
- optional post-logout redirect URI

## Messages

`bevy_stdb_auth` emits Bevy messages for auth lifecycle events:

- `StdbAuthSucceededMessage`
- `StdbAuthFailedMessage`
- `StdbAuthTokenRefreshedMessage`
- `StdbAuthRefreshFailedMessage`
- `StdbAuthLogoutSucceededMessage`
- `StdbAuthLogoutFailedMessage`

Applications can listen to these messages to route UI, update connection tokens, reconnect clients, or clear local game state.

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
| `oidc` | SpacetimeAuth OIDC authorization-code flow support |
| `steam` | Native SpacetimeAuth Steam ticket exchange support |
| `browser` | Browser runtime support for OIDC redirects and callback resume |
| `persistence` | Native OIDC refresh-token persistence using the OS keyring |

For apps targeting both native and browser, configure features per target so native builds can include Steam and keyring persistence without enabling those dependencies for WASM:

```toml
[dependencies]
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
bevy_stdb_auth = { version = "*", default-features = false, features = ["oidc", "persistence", "steam"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
bevy_stdb_auth = { version = "*", default-features = false, features = ["oidc", "browser"] }
```

The `steam` and `persistence` features are native-only. Browser builds should not enable them.

## Compatibility

| bevy_stdb_auth | bevy |
|---|---|
| 0.1 | 0.18 |

## Notes

This crate focuses on SpacetimeAuth session lifecycle management. It intentionally does not manage SpacetimeDB connections directly.

Use `bevy_stdb_auth` when you want Bevy-native authentication state and lifecycle messages. Use `bevy_stdb` or the SpacetimeDB SDK directly to decide how those tokens are applied to your app's connections.
