# Bevy SpacetimeAuth Plan

## Goal

Build a Bevy-native integration for SpacetimeAuth that acquires, refreshes, exposes, and clears SpacetimeDB-compatible auth tokens.

`bevy_stdb_auth` is specific to SpacetimeAuth and Bevy. It supports the SpacetimeAuth OIDC flow on native and browser targets, and the SpacetimeAuth Steam ticket exchange flow on native targets.

## Core boundary

`bevy_stdb_auth` owns authentication state and token lifecycle. Applications decide how to use the tokens.

The crate should not manage SpacetimeDB connections directly. A typical `bevy_stdb` integration is:

- on `StdbAuthSucceededMessage`, connect with `StdbConnectOptions::from_token(...)`
- on `StdbAuthTokenRefreshedMessage`, update or reconnect with the new token
- on `StdbAuthLogoutSucceededMessage`, disconnect and clear local connection state

The exact glue should live in the application or examples.

## Supported auth sources

Supported sources:

- `StdbAuthSource::Oidc(StdbOidcAuthOptions)`
- `StdbAuthSource::Steam(StdbSteamAuthOptions)`
- `StdbAuthSource::Token(StdbTokenAuthOptions)` for existing-token sessions and tests

`Steam` means Steam ticket exchange through SpacetimeAuth. It is not a generic Steam identity provider.

## Public API

### Plugin

`StdbAuthPlugin` installs login, logout, browser callback resume, and token refresh systems.

Configuration:

- auto-refresh enabled by default
- refresh buffer duration
- browser callback auto-resume enabled by default when browser OIDC is enabled

### Commands

Use a Bevy `SystemParam` named `StdbAuthCommands`.

Methods:

- `login(StdbLoginOptions)` starts an auth flow
- `logout(StdbLogoutOptions)` clears the current session and runs SpacetimeAuth logout when supported
- `clear_session()` clears local auth state without contacting SpacetimeAuth
- `refresh_now()` requests an immediate refresh when a refresh token is available
- `cancel_pending()` clears local pending auth task state when possible

`StdbAuthCommands` should:

- no-op when login/logout is already pending unless a command explicitly cancels or replaces it
- update only auth crate resources
- emit lifecycle messages through plugin systems
- never directly connect or disconnect `bevy_stdb`

### Resources

Public resources:

- `StdbAuthSession`: current auth state

`StdbAuthSession` should contain:

- access token
- token type
- optional ID token
- optional refresh token
- optional expires-at time
- granted scopes
- client ID
- auth source kind
- optional post-logout redirect URI

Internal resources:

- pending login task
- pending logout task
- pending refresh task
- refresh timer state
- browser callback pending state

### Messages

Lifecycle messages:

- `StdbAuthSucceededMessage`
- `StdbAuthFailedMessage`
- `StdbAuthTokenRefreshedMessage`
- `StdbAuthRefreshFailedMessage`
- `StdbAuthLogoutSucceededMessage`
- `StdbAuthLogoutFailedMessage`
- `StdbAuthSessionClearedMessage`

Applications listen to these messages to route UI, update tokens, reconnect clients, or clear local game state.

## Feature strategy

Keep features scoped to the supported SpacetimeAuth flows.

Candidate features:

- `browser`: browser runtime support and browser APIs
- `oidc`: SpacetimeAuth OIDC support
- `steam`: SpacetimeAuth Steam ticket exchange support

Expected combinations:

- native OIDC: `oidc`
- browser OIDC: `browser`, `oidc`
- native Steam: `steam`
- native OIDC + Steam: `oidc`, `steam`

The `steam` feature is native-only. Browser builds should not enable `steam`.

## Shared token model

OIDC and Steam should normalize responses into one token/session shape.

Shared token response fields:

- access token
- token type
- expires-in
- refresh token
- scope
- ID token

## OIDC flow

SpacetimeAuth OIDC uses authorization code flow with PKCE.

Shared OIDC behavior:

- PKCE challenge/verifier generation
- CSRF state generation and validation
- authorization URL construction
- callback parsing
- authorization code exchange
- token response normalization
- refresh token exchange
- end-session handling

### Native OIDC

Native OIDC uses:

- loopback redirect listener
- system browser open
- PKCE authorization code flow
- blocking HTTP inside Bevy `IoTaskPool`

Native OIDC should not block Bevy's main schedule.

### Browser OIDC

Browser OIDC uses:

- `window.location` redirects
- `sessionStorage` pending context
- automatic callback detection after reload
- async browser HTTP
- browser URL cleanup after callback processing

Browser callback resume must not require the app to call login again after returning from SpacetimeAuth.

## Steam flow

SpacetimeAuth Steam support is native-only.

Flow:

1. request Steam Web API ticket from Steamworks
2. hex-encode the ticket
3. exchange the ticket with SpacetimeAuth token endpoint
4. normalize token response into `StdbAuthSession`

The Steam flow should run off the main Bevy schedule through auth tasks.

## Refresh strategy

If a token response includes `refresh_token` and `expires_in`, insert refresh state and tick a timer.

Refresh should happen before expiration using a configurable buffer.

On refresh success:

- update `StdbAuthSession`
- reset refresh timer
- emit `StdbAuthTokenRefreshedMessage`

On refresh failure:

- emit `StdbAuthRefreshFailedMessage`
- keep or clear the current session according to the configured policy

## Logout strategy

Logout should always clear local auth state.

For OIDC sessions:

- browser may redirect to the SpacetimeAuth end-session endpoint
- native may open or call the SpacetimeAuth end-session endpoint, depending on final endpoint behavior

For Steam sessions:

- clear local session state
- do not call OIDC end-session unless SpacetimeAuth requires it for Steam-issued sessions

SpacetimeAuth logout failures should not prevent local session clearing.

## Integration with `bevy_stdb`

`bevy_stdb_auth` should not depend on `bevy_stdb`.

Example integration behavior:

- on `StdbAuthSucceededMessage`, call `stdb.connect(StdbConnectOptions::from_token(session.access_token.clone()))`
- on `StdbAuthTokenRefreshedMessage`, update or reconnect with the new token
- on `StdbAuthLogoutSucceededMessage`, call `stdb.disconnect()`

## Implementation phases

### Phase 1: Core auth session shell

- Define `StdbAuthSession`
- Define token response/session metadata structs
- Define lifecycle messages
- Add `StdbAuthPlugin`
- Add `StdbAuthCommands`
- Add pending login/logout polling
- Add `StdbAuthSource::Token` for local validation

### Phase 2: Shared SpacetimeAuth OIDC core

- Define `StdbOidcAuthOptions`
- Build authorization URL with PKCE and state
- Normalize token responses
- Implement refresh token exchange
- Implement callback parsing helpers

### Phase 3: Native OIDC

- Add loopback listener
- Open browser
- Exchange authorization code
- Emit session messages
- Add native manual test example

### Phase 4: Browser OIDC

- Store pending context in `sessionStorage`
- Redirect to SpacetimeAuth
- Auto-resume callback after reload
- Clear callback URL
- Add browser manual test example

### Phase 5: Refresh and logout

- Add refresh timer systems
- Add `refresh_now()`
- Add logout/end-session support
- Add local `clear_session()` behavior

### Phase 6: Steam exchange

- Add `StdbSteamAuthOptions`
- Add Steamworks ticket request
- Exchange Steam ticket with SpacetimeAuth
- Normalize token response
- Add native manual test example

### Phase 7: `bevy_stdb` integration example

- Add example showing auth success connecting to SpacetimeDB
- Add example showing refresh token propagation
- Add example showing logout disconnecting from SpacetimeDB

## Testing plan

Automated checks:

- default features
- `oidc`
- `browser`, `oidc`
- `steam`
- `oidc`, `steam`

Manual checks:

- native OIDC login success
- native OIDC cancellation/timeout
- browser OIDC redirect/resume success
- browser OIDC provider error callback
- token refresh success/failure
- OIDC logout behavior
- Steam ticket exchange success/failure

## Open decisions

- Whether native token persistence belongs in this crate.
- Whether native OIDC logout should POST, open browser, or be local-only.
- Whether refresh failure should keep or clear the current session by default.
- Whether `StdbAuthCommands` should be command-only or also mirrored by request messages.
