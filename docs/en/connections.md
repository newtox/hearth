# Spotify

[Deutsch](../de/verbindungen.md) · [All docs](../README.md)

Users link Spotify under **Settings → Connections**. The track they are playing then shows as their activity, and the account appears on their profile. A game or a manually set activity takes precedence.

1. Create an app at [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard) with the "Web API".
2. Add `https://<chat-domain>/api/spotify/callback` as a redirect URI.
3. Set `SPOTIFY_CLIENT_ID` and `SPOTIFY_CLIENT_SECRET` in Portainer or in `deploy/.env` and redeploy.

While the Spotify app is in development mode, only accounts on the app's user list can link.
