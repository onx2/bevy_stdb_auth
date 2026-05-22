# Example: Steam integration

This example runs a minimal Bevy app that requests a Steam Web API ticket through Steamworks and exchanges it with SpacetimeAuth.

## SpacetimeAuth setup

Create or configure a project/client in the SpacetimeAuth dashboard before running the example. The client must be configured for the Steam auth flow.

## Steam setup

Run the example from an environment where Steam is running and the current user owns or can run the configured app ID.

The default app ID is `480` for Steamworks Spacewar. Use your own app ID for real testing.

## Environment

| Variable | Required | Default | Purpose |
|---|---:|---|---|
| `STDB_AUTH_CLIENT_ID` | yes | | SpacetimeAuth OAuth client ID |
| `STEAM_APP_ID` | no | `480` | Steam app ID used to initialize Steamworks |

## Run

```sh
STDB_AUTH_CLIENT_ID="your-client-id" STEAM_APP_ID="480" cargo run --example steam --no-default-features --features steam
```

Steam refresh-token persistence is intentionally not used. If SpacetimeAuth returns a refresh token for the Steam flow, `bevy_stdb_auth` can refresh the access token in memory while the app is running.
