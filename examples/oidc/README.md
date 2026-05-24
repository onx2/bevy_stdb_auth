# Example: OIDC integration

This example runs a minimal Bevy app that starts a SpacetimeAuth OIDC login and prints the lifecycle messages emitted by `bevy_stdb_auth`.

## SpacetimeAuth setup

Create or configure a project/client in the SpacetimeAuth dashboard before running the example.

Use this native redirect URI unless you change `STDB_AUTH_REDIRECT_URI`:

```text
http://127.0.0.1:3000/callback
```

## Environment

| Variable | Required | Default | Purpose |
|---|---:|---|---|
| `STDB_AUTH_CLIENT_ID` | yes | | SpacetimeAuth OAuth client ID |
| `STDB_AUTH_REDIRECT_URI` | no | `http://127.0.0.1:3000/callback` | Native loopback callback URI |
| `STDB_AUTH_POST_LOGOUT_REDIRECT_URI` | no | | URI used after provider logout |
| `STDB_AUTH_SCOPES` | no | `openid profile email` | Space-separated OAuth scopes |

## Run

```sh
STDB_AUTH_CLIENT_ID="your-client-id" cargo run --example oidc --no-default-features --features oidc
```

For native keyring persistence:

```sh
STDB_AUTH_CLIENT_ID="your-client-id" cargo run --example oidc --no-default-features --features oidc,persistence
```

Linux keyring persistence may require Secret Service support through your desktop session.
