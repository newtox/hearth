# Einstellungen

[English](../en/configuration.md) · [Alle Dokus](../README.md)

## Variablen für den Stack

In Portainer oder in `deploy/.env` setzen, wenn du [`deploy/docker-compose.yml`](../../deploy/docker-compose.yml) benutzt.

| Variable | Pflicht | Bedeutung |
|---|---|---|
| `CHAT_DOMAIN` | ja | Domain des Chats, ohne `https://` |
| `LIVEKIT_DOMAIN` | ja | Domain des Sprachservers |
| `DB_PASSWORD` | ja | Passwort der Datenbank |
| `LIVEKIT_SECRET` | ja | Gemeinsames Geheimnis von App und LiveKit, mindestens 32 Zeichen |
| `APP_PORT` | nein | Lokaler Port der App, Standard `8090` |
| `LIVEKIT_PORT` | nein | Lokaler Port von LiveKit, Standard `7880` |
| `HEARTH_IMAGE` | nein | Welches Image läuft, Standard `ghcr.io/newtox/hearth:latest`. Mit einer Version wie `ghcr.io/newtox/hearth:0.2.0` bleibst du auf dieser. |
| `SPOTIFY_CLIENT_ID`, `SPOTIFY_CLIENT_SECRET` | nein | Spotify-Aktivität, siehe [Verbindungen](verbindungen.md) |
| `CONNECT_TWITCH_ID`, `CONNECT_TWITCH_SECRET` | nein | Twitch-Konten und Abonnenten-Rollen |
| `CONNECT_YOUTUBE_ID`, `CONNECT_YOUTUBE_SECRET` | nein | YouTube-Konten und Mitglieder-Rollen |
| `STEAM_API_KEY` | nein | Steam-Konten |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASSWORD`, `SMTP_FROM`, `SMTP_TLS` | nein | E-Mail für „Passwort vergessen?“, siehe unten |

## Variablen des Servers

Die App liest diese direkt. Die erste Gruppe setzt der Stack für dich.

| Variable | Standard | Bedeutung |
|---|---|---|
| `DATABASE_URL` | | Verbindung zu PostgreSQL |
| `BIND` | `0.0.0.0:8080` | Adresse, auf der die App lauscht |
| `PUBLIC_URL` | | Öffentliche Adresse, nötig für Kontoverbindungen |
| `TRUST_PROXY` | `0` | `1` hinter einem Reverse Proxy, damit Ratenlimits die echte Adresse sehen |
| `UPLOAD_DIR` | `uploads` | Ordner für hochgeladene Dateien |
| `WEB_DIR` | `../web/dist` | Ordner der gebauten Weboberfläche |
| `LIVEKIT_URL`, `LIVEKIT_API_KEY`, `LIVEKIT_API_SECRET` | | Sprachserver. Ohne sie sind Sprachkanäle sichtbar, aber nicht betretbar. |
| `SPOTIFY_POLL_SECS` | `15` | Wie oft der aktuelle Spotify-Titel abgefragt wird |
| `INTEGRATION_SYNC_SECS` | `600` | Wie oft Twitch- und YouTube-Rollen abgeglichen werden |

## E-Mail für Passwort-Resets

Ohne diese Variablen zeigt die Anmeldung „Passwort vergessen?“ trotzdem an, der Server antwortet dann aber, dass das nicht eingerichtet ist.

| Variable | Beispiel | Bedeutung |
|---|---|---|
| `SMTP_HOST` | `smtp.example.com` | Mailserver |
| `SMTP_PORT` | `587` | `587` mit STARTTLS oder `465` mit TLS |
| `SMTP_USER`, `SMTP_PASSWORD` | | Anmeldung am Mailserver |
| `SMTP_FROM` | `Hearth <chat@example.com>` | Absenderadresse |
| `SMTP_TLS` | | `starttls` (Standard), `tls` (Standard bei Port 465) oder `none` |

Der Link gilt eine Stunde und nur einmal. Nach dem Zurücksetzen werden alle Geräte abgemeldet.

## Feste Grenzen im Server

- Anhänge bis 25 MB, Avatare und andere Bilder bis 8 MB, Emojis bis 1 MB, Sticker bis 2 MB
- 100 Emojis und 50 Sticker pro Server
- Nachrichten bis 4000 Zeichen
- 100 Rollen pro Server
- Ratenlimits für Anmelden, Registrieren, Nachrichten, Reaktionen und Uploads
