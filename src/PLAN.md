# Bevy SpacetimeDB Auth Plan

## Goal

Build a Bevy-native SpacetimeAuth companion crate that acquires, refreshes, exposes, and clears SpacetimeDB-compatible auth tokens.

The crate should focus on SpacetimeAuth first. It should be a proving ground for the auth API before generalizing into a broader `bevy_auth` crate.

## Non-goals

- Do not support Auth0, AWS Cognito, Firebase, or other token issuers yet.
- Do not depend on `bevy_stdb` mechanically.
- Do not directly manage SpacetimeDB connections.
- Do not design a universal provider abstraction before the SpacetimeAuth implementation is stable.

## Core boundary

`bevy_stdb_auth` produces and maintains auth sessions. Applications decide how to use the tokens.

Typical `bevy_stdb` integration:

- on auth success, connect with `StdbConnectOptions::from_token(...)`
- on token refresh, update or reconnect with the new token
- on logout, disconnect and clear local connection state

If useful, `bevy_stdb` can later expose `set_token` and `clear_token` helpers to make token refresh integration easier.

## Supported auth sources

Initial sources:

- SpacetimeAuth OIDC
- SpacetimeAuth Steam ticket exchange

Suggested API shape:

- `StdbAuthSource::Oidc(StdbOidcAuthOptions)`
- `StdbAuthSource::Steam(StdbSteamAuthOptions)`

Steam is scoped specifically as a SpacetimeAuth exchange flow, not as a generic Steam identity provider.

## Public API shape

### Plugin

`StdbAuthPlugin` installs login, logout, browser callback resume, and token refresh systems.

Plugin configuration should include:

- auto-refresh enabled by default
- refresh buffer duration
- optional browser callback auto-resume

### Commands

Use a Bevy `SystemParam` named `StdbAuthCommands`, following the `bevy_stdb` command pattern.

Initial methods:

- `login(StdbLoginOptions)` starts an auth flow.
- `logout(StdbLogoutOptions)` clears the current session and runs provider logout when supported.
- `clear_session()` clears local auth state without contacting SpacetimeAuth.
- `refresh_now()` requests an immediate refresh when a refresh token is available.
- `cancel_pending()` clears local pending auth task state when possible.

`StdbAuthCommands` should be conservative:

- no-op when login/logout is already pending unless the command explicitly cancels or replaces it
- update only auth crate resources
- never directly connect or disconnect `bevy_stdb`

### Resources

Public resources:

- `StdbAuthSession`: current auth state

Suggested fields:

- access token
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

Applications can listen to these messages and decide how to route UI or update other clients.

## Feature strategy

Start pragmatic and keep features minimal.

Candidate features:

- `browser`: enables browser runtime support and browser APIs
- `oidc`: enables SpacetimeAuth OIDC support
- `steam`: enables SpacetimeAuth Steam ticket exchange

The feature model can be revisited after implementation. The priority is to avoid blocking the auth architecture on feature-flag design.

## Shared SpacetimeAuth token model

Both OIDC and Steam should normalize responses into one token/session shape.

Shared token response fields:

- access token
- token type
- expires-in
- refresh token
- scope
- ID token

## OIDC flow

Shared OIDC behavior:

- PKCE challenge/verifier generation
- CSRF state
- authorization URL construction
- callback parsing
- authorization code exchange
- refresh token exchange
- optional end-session handling

### Native OIDC

Native OIDC should use:

- loopback redirect listener
- system browser open
- PKCE authorization code flow
- blocking HTTP inside Bevy `IoTaskPool`

### Browser OIDC

Browser OIDC should use:

- `window.location` redirects
- `sessionStorage` pending context
- automatic callback detection after reload
- async browser HTTP
- URL cleanup after callback processing

Browser callback resume must not require the app to call login again after returning from SpacetimeAuth.

## Steam flow

SpacetimeAuth Steam flow:

1. request Steam Web API ticket from Steamworks
2. hex-encode the ticket
3. exchange the ticket with SpacetimeAuth token endpoint
4. normalize token response into `StdbAuthSession`

This flow is native-only for now.

## Refresh strategy

If a token response includes `refresh_token` and `expires_in`, insert refresh state and tick a timer.

Refresh should happen before expiration using a configurable buffer.

On refresh success:

- update `StdbAuthSession`
- reset refresh timer
- emit `StdbAuthTokenRefreshedMessage`

On refresh failure:

- emit `StdbAuthRefreshFailedMessage`
- keep or clear the session based on a configurable policy

## Logout strategy

Logout should always clear local session state.

For OIDC sessions:

- browser may redirect to SpacetimeAuth end-session endpoint
- native may open/call the end-session endpoint depending on final behavior

For Steam sessions:

- clear local session state
- no OIDC end-session call unless SpacetimeAuth requires one for the issued token

Provider logout failures should not prevent local session clearing.

## Integration with `bevy_stdb`

`bevy_stdb_auth` should not depend on `bevy_stdb`.

Example integration behavior:

- on `StdbAuthSucceededMessage`, call `stdb.connect(StdbConnectOptions::from_token(session.access_token.clone()))`
- on `StdbAuthTokenRefreshedMessage`, update the token or reconnect
- on `StdbAuthLogoutSucceededMessage`, call `stdb.disconnect()`

The exact glue should live in the application or an optional integration example.

## Implementation phases

### Phase 1: Core auth session shell

- Define `StdbAuthSession`
- Define token response/session metadata structs
- Define lifecycle messages
- Add `StdbAuthPlugin`
- Add `StdbAuthCommands`
- Add pending login/logout polling
- Add fake/test auth source for local testing if useful

### Phase 2: Shared SpacetimeAuth OIDC core

- Define OIDC options
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
- OIDC feature
- browser OIDC feature combination
- Steam feature
- OIDC + Steam feature combination

Manual checks:

- native OIDC login success
- native OIDC cancellation/timeout
- browser OIDC redirect/resume success
- browser OIDC provider error callback
- token refresh success/failure
- OIDC logout behavior
- Steam ticket exchange success/failure

## Future generalization

After `bevy_stdb_auth` is stable, consider extracting a more generic `bevy_auth` core.

Potential future issuers:

- Auth0
- AWS Cognito
- Custom OIDC
- Firebase / Google Identity Platform
- Keycloak
- Okta
- Microsoft Entra ID

Do not generalize until the SpacetimeAuth API proves itself in real projects.

## Open decisions

- Final crate name: `bevy_stdb_auth` vs keeping `bevy_auth` during experimentation.
- Whether native token persistence belongs in this crate.
- Whether native OIDC logout should POST, open browser, or be local-only.
- Whether refresh failure should keep or clear the current session by default.
- Whether `StdbAuthCommands` should be command-only or also mirrored by request messages.
