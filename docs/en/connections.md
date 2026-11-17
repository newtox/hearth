# Connections and role sync

[Deutsch](../de/verbindungen.md) · [All docs](../README.md)

Users link outside accounts under **Settings → Connections**. Linked accounts show on their profile. A platform only appears once its credentials are set, and `PUBLIC_URL` (set automatically by the stack) must be the public chat address.

| Platform | What it does | Variables | Redirect URI to register |
|---|---|---|---|
| Spotify | Shows the current track as activity | `SPOTIFY_CLIENT_ID`, `SPOTIFY_CLIENT_SECRET` | `https://<chat-domain>/api/spotify/callback` |
| Steam | Shows the Steam profile | `STEAM_API_KEY` | none, Steam uses OpenID |
| Twitch | Shows the channel, subscriber roles | `CONNECT_TWITCH_ID`, `CONNECT_TWITCH_SECRET` | `https://<chat-domain>/api/connections/twitch/callback` |
| YouTube | Shows the channel, member roles | `CONNECT_YOUTUBE_ID`, `CONNECT_YOUTUBE_SECRET` | `https://<chat-domain>/api/connections/youtube/callback` |

After adding variables, redeploy the stack.

## Where to get the credentials

- **Spotify**: [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard), create an app with the "Web API". In development mode only accounts on the app's user list can link.
- **Steam**: [steamcommunity.com/dev/apikey](https://steamcommunity.com/dev/apikey)
- **Twitch**: [dev.twitch.tv/console/apps](https://dev.twitch.tv/console/apps), category "Chat Bot" or "Website Integration", client type "Confidential".
- **YouTube**: [console.cloud.google.com](https://console.cloud.google.com), enable "YouTube Data API v3", create an OAuth client of type "Web application". While the consent screen is in testing mode, only test users can link.

## Twitch and YouTube roles

Anyone with **Manage server** links a channel under **Server settings → Integrations**. Hearth creates a role, purple "Twitch Subscriber" or red "YouTube Member", and gives it to every member who

1. has linked their own Twitch or YouTube account under Settings, and
2. subscribes to that Twitch channel or is a member of that YouTube channel.

When the subscription ends, the role goes away again. Syncing runs every 10 minutes, right away when someone links or unlinks an account or joins the server, and when you press **Sync now**.

The role can be renamed, recoloured and given permissions. It cannot be given or taken by hand and it stays until you unlink the integration.

- Twitch subscriptions only exist for Affiliate and Partner channels.
- YouTube only shares the member list with creators it has allowlisted for the memberships API, in practice channels with a YouTube Partner Manager. Without that access the integration links but shows an error instead of giving roles.
