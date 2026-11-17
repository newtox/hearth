# Hearth

Self-hosted chat for communities: servers, channels, roles, direct messages, voice channels with screen sharing.
The name is a working title and can be changed at any time.

- `server/` Rust backend (Axum, WebSocket gateway, PostgreSQL via sqlx)
- `web/` Svelte 5 frontend (Vite), German and English, six themes: Light, Ash, Dark, Onyx, Midnight and Parchment (RPG)
- `web/src-tauri/` Tauri 2 shell for Windows, Linux and Android
- `deploy/` Docker stack and setup script for your own server
- `scripts/smoke.py` end-to-end test against a running server
- `scripts/integrations.py` and `scripts/mock_platform.py` test for the Twitch and YouTube roles against a fake platform
- `.github/workflows/apps.yml` builds the apps on GitHub

## Getting started

Requirements: Rust, Node 22, Docker (or your own PostgreSQL 16).

```sh
docker compose up -d
cp .env.example server/.env
cd web && npm install && npm run build
cd ../server && cargo run --release
```

Everything then runs at http://localhost:8080. The server creates its tables on startup.
The first account to register becomes the instance admin and can create and grant global badges.

For frontend development with hot reload, start the server as above and run `cd web && npm run dev`.

Test: `pip install requests websockets && python3 scripts/smoke.py http://127.0.0.1:8080`

Role sync test: start `python3 scripts/mock_platform.py`, run the server with `PUBLIC_URL=http://127.0.0.1:8080 CONNECT_MOCK_URL=http://127.0.0.1:9099 INTEGRATION_SYNC_SECS=5`, then `python3 scripts/integrations.py`.

## Deploying to a server

`deploy/deploy.sh` fetches the code, builds the Docker image, starts the app, PostgreSQL and LiveKit, and adds both domains to Caddy:

```sh
sudo ./deploy.sh chat.example.com livekit.example.com
```

The script generates its own secrets and stores them in `deploy/.env`. Run it again to update.

## Voice channels

Voice and screen sharing run through LiveKit, which `docker compose up -d` starts as well.
Before going live, set the secret in `livekit.yaml` and `LIVEKIT_API_SECRET` in `server/.env` to the same random value.
`LIVEKIT_URL` is the address clients use to reach LiveKit, so `wss://…` on the public internet.
Open 7880 (through the reverse proxy), 7881/TCP and 7882/UDP.
Without the `LIVEKIT_*` variables, voice channels stay visible but cannot be joined.

## Spotify

Optional. Create an app at https://developer.spotify.com/dashboard, add `https://<your-chat-domain>/api/spotify/callback` as a redirect URI, and set `SPOTIFY_CLIENT_ID`, `SPOTIFY_CLIENT_SECRET` and `PUBLIC_URL` (on a deployed server: add the first two to `deploy/.env` and run `deploy.sh` again).
Users then link their account under Settings, and the track they are playing shows as their activity. A game or a manually set activity takes precedence.
While a Spotify app is in development mode, Spotify only lets accounts you have added to the app's user list link.

## Account connections

Optional. Users can link outside accounts under Settings and show them on their profile. Each platform appears once its credentials are set; `PUBLIC_URL` must be set too.

| Platform | Variables | Redirect URI to register |
|---|---|---|
| Twitch | `CONNECT_TWITCH_ID`, `CONNECT_TWITCH_SECRET` | `<PUBLIC_URL>/api/connections/twitch/callback` |
| YouTube | `CONNECT_YOUTUBE_ID`, `CONNECT_YOUTUBE_SECRET` | `<PUBLIC_URL>/api/connections/youtube/callback` |
| Steam | `STEAM_API_KEY` | none, Steam uses OpenID |
| Spotify | see above | see above |

On a deployed server, add the variables to `deploy/.env` and run `deploy.sh` again.

## Twitch and YouTube roles

Server managers can link their Twitch or YouTube channel under Server settings, Integrations. Hearth then creates a role and gives it to every member who has linked their own account under Settings and subscribes to that Twitch channel or is a member of that YouTube channel. When the subscription or membership ends, the role is removed again.

Roles are synced every 10 minutes (change with `INTEGRATION_SYNC_SECS`), right away when someone links or unlinks an account, and when someone joins the server. The role can be renamed, recoloured and given permissions like any other role, but it cannot be assigned by hand or deleted while the integration exists.

It uses the same apps and redirect URIs as the account connections above, no extra setup is needed.

- Twitch: subscriptions only exist for Affiliate and Partner channels.
- YouTube: Google only gives the channel member list to creators it has allowlisted, which in practice means channels with a YouTube Partner Manager. Without that access the integration links but shows an error instead of assigning roles. While the Google app is in testing mode, only accounts added as test users can link.

## Apps

The apps are the same interface inside a Tauri shell. They ask for the server address at sign-in.

On GitHub, run the "Apps" workflow under Actions or push a `v…` tag. It produces `.exe`, `.AppImage`, `.deb`, `.flatpak` and `.apk` artifacts.

Locally on Linux (needs `libwebkit2gtk-4.1-dev`):

```sh
cd web && npm install && npx tauri build
```

Limitations:

- The APK is a debug build. A release needs its own signing key.
- Android only allows HTTPS servers.
- Screen sharing does not work on Android (the WebView does not offer it); watching does.
- In the Linux app, voice depends on whether the installed WebKitGTK supports WebRTC. In the browser it works regardless.

## Features

- Accounts with Argon2 passwords and revocable sessions
- Servers with categories, text and voice channels, icon, description, ownership transfer
- Roles with hierarchy, colour, 20 permissions, per-role channel rules
- Threads started from any message in a text channel
- Roles for Twitch subscribers and YouTube channel members, kept in sync automatically
- Polls with single or multiple choice and an optional end time
- Real-time chat: Markdown, attachments up to 25 MB, reactions, custom emojis and stickers, typing indicator, unread markers
- Friends list with requests, direct messages between friends or members of shared servers
- Group chats with up to 10 friends, a group picture, three roles (administrator, moderator, member) and per-group permission settings
- Blocking users
- Sounds for messages, calls and voice channels
- Calls with screen sharing in direct messages and groups, with an incoming-call banner
- Screen shares are watched on demand, with a viewer list
- Profiles with avatar, banner, bio, accent colour, global badges with built-in icons, linked accounts
- Activity status (playing, listening, watching, custom text), set by hand, detected from running games in the desktop app, or taken from Spotify
- Badges earned automatically from tracked playtime, stream time, voice time and account age, in ten colour levels with a progress view
- Moderation: timeouts, kicks, bans, audit log, rate limits
- Invites with expiry and usage limit
- Interface in German and English, switchable in the settings

## Known limits

- No email verification and no password reset.
- CORS is open so the apps can connect. Sign-in uses bearer tokens, not cookies.
- On structural changes every client reloads the server state. Very large servers will need something finer.
- TLS is handled by a reverse proxy (Caddy or nginx). `/gateway` needs WebSocket forwarding, and behind the proxy set `TRUST_PROXY=1` so rate limits see the real IP.
- The server keeps track of who is in a voice channel in memory only.
- Activity and playtime are reported by the client, so they can be faked.
