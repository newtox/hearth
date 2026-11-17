# Configuration

[Deutsch](../de/einstellungen.md) · [All docs](../README.md)

## Stack variables

Set in Portainer or in `deploy/.env` when using [`deploy/docker-compose.yml`](../../deploy/docker-compose.yml).

| Variable | Required | Meaning |
|---|---|---|
| `CHAT_DOMAIN` | yes | Domain of the chat, without `https://` |
| `LIVEKIT_DOMAIN` | yes | Domain of the voice server |
| `DB_PASSWORD` | yes | Database password |
| `LIVEKIT_SECRET` | yes | Shared secret of app and LiveKit, at least 32 characters |
| `APP_PORT` | no | Local port of the app, default `8090` |
| `LIVEKIT_PORT` | no | Local port of LiveKit, default `7880` |
| `HEARTH_IMAGE` | no | Image to run, default `ghcr.io/newtox/hearth:latest`. Use a version such as `ghcr.io/newtox/hearth:0.2.0` to stay on it. |
| `SPOTIFY_CLIENT_ID`, `SPOTIFY_CLIENT_SECRET` | no | Spotify activity, see [connections](connections.md) |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASSWORD`, `SMTP_FROM`, `SMTP_TLS` | no | Email for “Forgot your password?”, see below |

## Server variables

The app reads these directly. The stack sets the first group for you.

| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | | PostgreSQL connection |
| `BIND` | `0.0.0.0:8080` | Address the app listens on |
| `PUBLIC_URL` | | Public address, needed for account connections |
| `TRUST_PROXY` | `0` | `1` behind a reverse proxy, so rate limits use the real client address |
| `UPLOAD_DIR` | `uploads` | Folder for uploaded files |
| `WEB_DIR` | `../web/dist` | Folder of the built web interface |
| `LIVEKIT_URL`, `LIVEKIT_API_KEY`, `LIVEKIT_API_SECRET` | | Voice server. Without them voice channels are visible but cannot be joined. |
| `SPOTIFY_POLL_SECS` | `15` | How often the current Spotify track is fetched |

## Email for password resets

Without these variables the sign-in page still offers “Forgot your password?”, but the server answers that resets are not set up.

| Variable | Example | Meaning |
|---|---|---|
| `SMTP_HOST` | `smtp.example.com` | Mail server |
| `SMTP_PORT` | `587` | `587` with STARTTLS or `465` with TLS |
| `SMTP_USER`, `SMTP_PASSWORD` | | Login at the mail server |
| `SMTP_FROM` | `Hearth <chat@example.com>` | Sender address |
| `SMTP_TLS` | | `starttls` (default), `tls` (default on port 465) or `none` |

The reset link is valid for one hour and works once. After a reset, every device is signed out.

## Limits built into the server

- Attachments up to 25 MB, avatars and other images up to 8 MB, emojis up to 1 MB, stickers up to 2 MB
- 100 emojis and 50 stickers per server
- Messages up to 4000 characters
- 100 roles per server
- Rate limits on login, registration, messages, reactions and uploads
