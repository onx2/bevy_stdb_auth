# Bevy SpacetimeAuth

A [Bevy](https://bevy.org/) integration for [SpacetimeAuth](https://spacetimedb.com/docs/core-concepts/authentication/spacetimeauth/) token sessions.

[![crates.io](https://img.shields.io/crates/v/bevy_stdb_auth)](https://crates.io/crates/bevy_stdb_auth)
![Dependabot](https://img.shields.io/badge/dependabot-enabled-brightgreen.svg)
[![docs.rs](https://docs.rs/bevy_stdb_auth/badge.svg)](https://docs.rs/bevy_stdb_auth)
[![CI](https://github.com/onx2/bevy_stdb_auth/actions/workflows/ci.yml/badge.svg)](https://github.com/onx2/bevy_stdb_auth/actions/workflows/ci.yml?query=branch%3Amain)
[![CodeQL](https://github.com/onx2/bevy_stdb_auth/actions/workflows/github-code-scanning/codeql/badge.svg)](https://github.com/onx2/bevy_stdb_auth/actions/workflows/github-code-scanning/codeql)

![Useless AI generated image that kind of looks cool](https://github.com/user-attachments/assets/141a4b22-1cd7-4340-8c88-277fd6af6a19)
_Please enjoy this useless AI generated image based on the README contents of this repo._

## Overview

`bevy_stdb_auth` adapts SpacetimeAuth token acquisition and session lifecycle state into Bevy-style resources, systems, commands, and messages.

This crate is intentionally token-focused. It does not depend on `bevy_stdb`, and it does not manage SpacetimeDB connections directly. Applications decide how to use auth tokens, including passing them to `bevy_stdb`, HTTP clients, backend APIs, or game services.

> Current status: the core auth session shell is implemented. OIDC and Steam SpacetimeAuth flows are planned next.

## Features

- **Plugin setup** via `StdbAuthPlugin`
- **Command interface** via `StdbAuthCommands`
- **Current auth state** through `StdbAuthSession`
- **Lifecycle messages** for login, refresh, logout, and local session clearing
- **Existing-token sessions** through `StdbAuthSource::Token`
- **Planned SpacetimeAuth OIDC support** for native and browser clients
- **Planned SpacetimeAuth Steam support** for native Steam ticket exchange
- **No hard dependency on `bevy_stdb`**

## Example

```rust
use bevy::prelude::*;
use bevy_stdb_auth::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(StdbAuthPlugin::default())
        .add_systems(Startup, login_with_existing_token)
        .add_systems(Update, on_auth_succeeded)
        .run();
}

fn login_with_existing_token(mut auth: StdbAuthCommands) {
    auth.login(StdbLoginOptions::new(StdbAuthSource::Token(
        StdbTokenAuthOptions::new("json.web.token"),
    )));
}

fn on_auth_succeeded(mut messages: MessageReader<StdbAuthSucceededMessage>) {
    for message in messages.read() {
        info!("authenticated with token: {}", message.session.access_token);
    }
}
```

## Commands

Use `StdbAuthCommands` from normal Bevy systems to manage auth state.

| Method | Behavior |
|---|---|
| `login` | Starts a login flow using `StdbLoginOptions` |
| `logout` | Clears the current session and runs provider logout when supported |
| `clear_session` | Clears local auth state without contacting SpacetimeAuth |
| `refresh_now` | Requests an immediate token refresh when a refresh token is available |
| `cancel_pending` | Clears local pending auth task state when possible |

```rust
use bevy_stdb_auth::prelude::*;

fn logout(mut auth: StdbAuthCommands) {
    auth.logout(StdbLogoutOptions::default());
}

fn clear_local_session(mut auth: StdbAuthCommands) {
    auth.clear_session();
}
```

`StdbAuthCommands` is conservative: login/logout requests are ignored while another auth operation is pending, unless a command explicitly clears pending state.

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

## Messages

`bevy_stdb_auth` emits Bevy messages for auth lifecycle events:

- `StdbAuthSucceededMessage`
- `StdbAuthFailedMessage`
- `StdbAuthTokenRefreshedMessage`
- `StdbAuthRefreshFailedMessage`
- `StdbAuthLogoutSucceededMessage`
- `StdbAuthLogoutFailedMessage`
- `StdbAuthSessionClearedMessage`

Applications can listen to these messages to route UI, update connection tokens, reconnect clients, or clear local game state.

## Integrating with `bevy_stdb`

`bevy_stdb_auth` does not directly depend on [`bevy_stdb`](https://github.com/onx2/bevy_stdb). Connect the two crates with a small glue system.

```rust
use bevy::prelude::*;
use bevy_stdb::prelude::*;
use bevy_stdb_auth::prelude::*;
use crate::module_bindings::{DbConnection, RemoteModule};

pub type StdbCmds<'w, 's> = StdbCommands<'w, 's, DbConnection, RemoteModule>;

fn connect_on_auth_success(
    mut messages: MessageReader<StdbAuthSucceededMessage>,
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

## Planned SpacetimeAuth OIDC

OIDC support will target the SpacetimeAuth authorization-code-with-PKCE flow.

Native clients will use:

- loopback redirect listener
- system browser open
- PKCE authorization code flow
- blocking HTTP inside Bevy's IO task pool

Browser clients will use:

- browser redirects
- `sessionStorage` pending auth state
- automatic callback resume after reload
- browser URL cleanup after callback handling

## Planned SpacetimeAuth Steam

Steam support will be scoped specifically to SpacetimeAuth's Steam ticket exchange flow:

1. request a Steam Web API ticket through Steamworks
2. exchange the ticket with SpacetimeAuth
3. normalize the token response into `StdbAuthSession`

This is planned as a native-only flow initially.

## Feature flags

Current placeholder feature flags:

| Feature | Purpose |
|---|---|
| `browser` | Browser runtime support |
| `oidc` | Planned SpacetimeAuth OIDC support |
| `steam` | Planned SpacetimeAuth Steam support |

The feature model may change while the crate is still experimental.

## Compatibility

| bevy_stdb_auth | bevy |
|---|---|
| 0.1 | 0.18 |

## Notes

This crate is currently scoped to SpacetimeAuth. A more generic `bevy_auth` crate may be extracted later after the SpacetimeAuth API proves itself in real projects.

Special thanks to [`bevy_stdb`](https://github.com/onx2/bevy_stdb) for the command and message integration patterns.
