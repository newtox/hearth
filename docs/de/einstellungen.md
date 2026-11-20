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
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASSWORD`, `SMTP_FROM`, `SMTP_TLS` | nein | E-Mail für „Passwort vergessen?“, siehe unten |

## Variablen des Servers

Die App liest diese direkt. Die erste Gruppe setzt der Stack für dich.

| Variable | Standard | Bedeutung |
|---|---|---|
| `DATABASE_URL` | | Verbindung zu PostgreSQL |
| `BIND` | `0.0.0.0:8080` | Adresse, auf der die App lauscht |
| `PUBLIC_URL` | | Öffentliche Adresse. Dient für Links in E-Mails, Linkvorschauen und die erlaubten Herkünfte. Ohne sie ist jede Herkunft erlaubt, das ist nur für die lokale Entwicklung gedacht. |
| `CORS_ORIGINS` | | Weitere erlaubte Herkünfte, durch Komma getrennt. Die Web-Adresse und die Apps sind immer erlaubt. |
| `EMAIL_VERIFICATION` | `on` | Mit eingerichteter E-Mail müssen neue Konten ihre Adresse bestätigen, bevor sie schreiben können. `off` schaltet das ab. |
| `DEEPL_API_KEY` | | Schlüssel für die DeepL API. Damit werden englische Namen und Beschreibungen globaler Abzeichen automatisch übersetzt. Ein kostenloser Schlüssel (Endung `:fx`) reicht. |
| `TRUST_PROXY` | `0` | `1` hinter einem Reverse Proxy, damit Ratenlimits die echte Adresse sehen |
| `UPLOAD_DIR` | `uploads` | Ordner für hochgeladene Dateien |
| `WEB_DIR` | `../web/dist` | Ordner der gebauten Weboberfläche |
| `LIVEKIT_URL`, `LIVEKIT_API_KEY`, `LIVEKIT_API_SECRET` | | Sprachserver. Ohne sie sind Sprachkanäle sichtbar, aber nicht betretbar. |

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

Mit eingerichteter E-Mail bekommen neue Konten und geänderte Adressen einen Bestätigungslink, der 24 Stunden gilt. Bis dahin kann das Konto lesen, aber nicht schreiben, keinen Servern beitreten und keine Freunde hinzufügen. Bestehende Konten gelten als bestätigt.

Ohne E-Mail kann ein Admin auf dem Server einen Reset-Link erzeugen. Er gilt 24 Stunden:

```bash
docker exec hearth /app/hearth-server reset-link "Nutzername oder E-Mail"
```

## Feste Grenzen im Server

- Anhänge bis 25 MB, Avatare und andere Bilder bis 8 MB, Emojis bis 1 MB, Sticker bis 2 MB
- 100 Emojis und 50 Sticker pro Server
- Nachrichten bis 4000 Zeichen
- 100 Rollen pro Server
- Ratenlimits für Anmelden, Registrieren, Nachrichten, Reaktionen und Uploads
