# Installing on a server

[Deutsch](../de/installation.md) · [All docs](../README.md)

Hearth runs as three containers: the app (`ghcr.io/newtox/hearth`), PostgreSQL and LiveKit for voice. GitHub builds the app image on every push to `main`, so nothing is compiled on your server.

You need:

- Docker with the Compose plugin
- Two domains pointing at the server, for example `chat.example.com` and `livekit.example.com`. On Cloudflare, set the LiveKit domain to "DNS only" (grey cloud), voice does not work through the proxy.
- A reverse proxy with HTTPS, for example Caddy
- Ports 7881/TCP and 7882/UDP open for voice, in the server firewall and at your hosting provider

## Option A: Portainer

1. In Portainer open **Stacks → Add stack** and name it `hearth`.
2. Choose **Repository**, URL `https://github.com/newtox/hearth`, compose path `deploy/docker-compose.yml`. Alternatively choose **Web editor** and paste the content of [`deploy/docker-compose.yml`](../../deploy/docker-compose.yml).
3. Add these environment variables:

   | Variable | Value |
   |---|---|
   | `CHAT_DOMAIN` | `chat.example.com` |
   | `LIVEKIT_DOMAIN` | `livekit.example.com` |
   | `DB_PASSWORD` | a long random string, `openssl rand -hex 24` |
   | `LIVEKIT_SECRET` | a long random string with at least 32 characters, `openssl rand -hex 32` |
   | `APP_PORT` | a free local port for the app, default `8090` |
   | `LIVEKIT_PORT` | a free local port for LiveKit, default `7880` |

   Optional variables for Spotify, Twitch, YouTube and Steam are listed in [configuration](configuration.md).
4. Click **Deploy the stack**.
5. Add the domains to your reverse proxy, see [Caddy](#caddy).
6. Open `https://chat.example.com`. The first account that registers becomes the instance admin.

Keep `DB_PASSWORD` and `LIVEKIT_SECRET` somewhere safe. Changing `DB_PASSWORD` later does not change the password inside an existing database.

## Option B: script

```sh
sudo git clone https://github.com/newtox/hearth.git /opt/hearth
cd /opt/hearth/deploy
sudo ./deploy.sh chat.example.com livekit.example.com
```

The script creates `deploy/.env` with random secrets, pulls the images, starts the stack, adds both domains to Caddy if Caddy runs on the host and opens the voice ports in `ufw`. If a port is taken it stops and tells you which container uses it; run it again with another port:

```sh
sudo APP_PORT=8097 ./deploy.sh chat.example.com livekit.example.com
```

The port is remembered in `deploy/.env`, so later runs need no prefix. The stack also appears in Portainer as `hearth`.

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

Use the ports you chose for `APP_PORT` and `LIVEKIT_PORT`, then `systemctl reload caddy`.

## Private repository or image

If the repository is private, the image on GitHub Packages is private too. Either make the package public (GitHub → your profile → **Packages → hearth → Package settings → Change visibility**) or log in on the server:

```sh
echo <token> | docker login ghcr.io -u newtox --password-stdin
```

The token is a GitHub personal access token with the `read:packages` scope. In Portainer add it under **Registries → Add registry → Custom**, URL `ghcr.io`.

## Updates

The app container carries the label `com.centurylinklabs.watchtower.enable=true`, so Watchtower picks up new images on its own. Without Watchtower: **Stacks → hearth → Pull and redeploy** in Portainer, or run `deploy.sh` again. Database changes are applied automatically when the new version starts.

## Stopping, starting, removing

| What | Portainer | Command line in `/opt/hearth/deploy` |
|---|---|---|
| Stop, keep all data | Stacks → hearth → **Stop this stack** | `docker compose down` |
| Start again | **Start this stack** | `docker compose --env-file .env up -d` |
| Remove including all messages, accounts and uploads | Delete the stack, then delete the volumes `hearth_db` and `hearth_uploads` under **Volumes** | `docker compose down -v` |

After removing everything, also delete the two domain blocks from your Caddyfile if you do not set Hearth up again.

## Backups

```sh
docker exec hearth-db-1 pg_dump -U hearth hearth > hearth.sql
docker run --rm -v hearth_uploads:/data -v "$PWD":/out debian tar czf /out/uploads.tar.gz -C /data .
```
