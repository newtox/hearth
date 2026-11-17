# Installation auf einem Server

[English](../en/installation.md) · [Alle Dokus](../README.md)

Hearth läuft als drei Container: die App (`ghcr.io/newtox/hearth`), PostgreSQL und LiveKit für Sprache. GitHub baut das App-Image bei jedem Push auf `main`, auf dem Server wird nichts kompiliert.

Du brauchst:

- Docker mit dem Compose-Plugin
- Zwei Domains, die auf den Server zeigen, zum Beispiel `chat.example.com` und `livekit.example.com`. Bei Cloudflare die LiveKit-Domain auf „Nur DNS“ stellen (graue Wolke), über den Proxy funktioniert Sprache nicht.
- Einen Reverse Proxy mit HTTPS, zum Beispiel Caddy
- Die Ports 7881/TCP und 7882/UDP offen für Sprache, in der Server-Firewall und beim Hoster

## Weg A: Portainer

1. In Portainer **Stacks → Add stack** öffnen und `hearth` als Namen eintragen.
2. **Repository** wählen, URL `https://github.com/newtox/hearth`, Compose-Pfad `deploy/docker-compose.yml`. Alternativ **Web editor** wählen und den Inhalt von [`deploy/docker-compose.yml`](../../deploy/docker-compose.yml) einfügen.
3. Diese Umgebungsvariablen eintragen:

   | Variable | Wert |
   |---|---|
   | `CHAT_DOMAIN` | `chat.example.com` |
   | `LIVEKIT_DOMAIN` | `livekit.example.com` |
   | `DB_PASSWORD` | lange Zufallszeichenkette, `openssl rand -hex 24` |
   | `LIVEKIT_SECRET` | lange Zufallszeichenkette mit mindestens 32 Zeichen, `openssl rand -hex 32` |
   | `APP_PORT` | freier lokaler Port für die App, Standard `8090` |
   | `LIVEKIT_PORT` | freier lokaler Port für LiveKit, Standard `7880` |

   Optionale Variablen für E-Mail stehen unter [Einstellungen](einstellungen.md).
4. **Deploy the stack** klicken.
5. Die Domains im Reverse Proxy eintragen, siehe [Caddy](#caddy).
6. `https://chat.example.com` öffnen. Das erste Konto, das sich registriert, wird Admin der Instanz.

`DB_PASSWORD` und `LIVEKIT_SECRET` sicher aufheben. Wenn du `DB_PASSWORD` später änderst, ändert das nicht das Passwort in einer schon angelegten Datenbank.

## Weg B: Script

```sh
sudo git clone https://github.com/newtox/hearth.git /opt/hearth
cd /opt/hearth/deploy
sudo ./deploy.sh chat.example.com livekit.example.com
```

Das Script legt `deploy/.env` mit zufälligen Geheimnissen an, lädt die Images, startet den Stack, trägt beide Domains in Caddy ein, wenn Caddy auf dem Server läuft, und öffnet die Sprach-Ports in `ufw`. Ist ein Port belegt, bricht es ab und sagt, welcher Container ihn nutzt. Dann mit anderem Port nochmal starten:

```sh
sudo APP_PORT=8097 ./deploy.sh chat.example.com livekit.example.com
```

Der Port wird in `deploy/.env` gemerkt, spätere Läufe brauchen ihn nicht mehr. Der Stack taucht auch in Portainer als `hearth` auf.

## Caddy

```
chat.example.com {
	encode zstd gzip
	request_body {
		max_size 30MB
	}
	reverse_proxy 127.0.0.1:8090
}

livekit.example.com {
	reverse_proxy 127.0.0.1:7880
}
```

Die Ports nehmen, die du für `APP_PORT` und `LIVEKIT_PORT` gewählt hast, dann `systemctl reload caddy`.

## Privates Repository oder Image

Ist das Repository privat, ist auch das Image in GitHub Packages privat. Entweder das Paket öffentlich machen (GitHub → dein Profil → **Packages → hearth → Package settings → Change visibility**) oder dich auf dem Server anmelden:

```sh
echo <token> | docker login ghcr.io -u newtox --password-stdin
```

Der Token ist ein GitHub Personal Access Token mit dem Recht `read:packages`. In Portainer trägst du ihn unter **Registries → Add registry → Custom** ein, URL `ghcr.io`.

## Updates

Der App-Container trägt das Label `com.centurylinklabs.watchtower.enable=true`, Watchtower holt neue Images also von selbst. Ohne Watchtower: in Portainer **Stacks → hearth → Pull and redeploy**, oder `deploy.sh` nochmal ausführen. Änderungen an der Datenbank spielt die neue Version beim Start selbst ein.

## Stoppen, starten, entfernen

| Was | Portainer | Kommandozeile in `/opt/hearth/deploy` |
|---|---|---|
| Stoppen, alle Daten bleiben | Stacks → hearth → **Stop this stack** | `docker compose down` |
| Wieder starten | **Start this stack** | `docker compose --env-file .env up -d` |
| Entfernen samt allen Nachrichten, Konten und Uploads | Stack löschen, dann unter **Volumes** `hearth_db` und `hearth_uploads` löschen | `docker compose down -v` |

Wenn du alles entfernst und Hearth nicht neu einrichtest, auch die beiden Domain-Blöcke aus der Caddyfile löschen.

## Sicherungen

```sh
docker exec hearth-db pg_dump -U hearth hearth > hearth.sql
docker run --rm -v hearth_uploads:/data -v "$PWD":/out debian tar czf /out/uploads.tar.gz -C /data .
```
