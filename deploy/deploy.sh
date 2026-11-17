#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<TXT
Usage: sudo ./deploy.sh <chat-domain> <livekit-domain>
Example: sudo ./deploy.sh chat.example.com livekit.example.com

Run it from the folder that contains docker-compose.yml. It pulls the
published image, so nothing is built on the server.

Optional environment variables (remembered in .env after the first run):
  APP_PORT      Local port of the app     (default: 8090)
  LIVEKIT_PORT  Local port of LiveKit     (default: 7880)
  HEARTH_IMAGE  Image to run              (default: ghcr.io/newtox/hearth:latest)
  CADDYFILE     Path to the Caddyfile     (default: /etc/caddy/Caddyfile)
TXT
  exit 1
}

[ $# -eq 2 ] || usage
CHAT_DOMAIN=$1
LIVEKIT_DOMAIN=$2
CADDYFILE=${CADDYFILE:-/etc/caddy/Caddyfile}
cd "$(dirname "$0")"

say() { printf '\n==> %s\n' "$*"; }
die() { printf 'Error: %s\n' "$*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || die "Please run as root or with sudo."
for tool in docker openssl curl; do
  command -v "$tool" >/dev/null || die "$tool is not installed."
done
docker compose version >/dev/null 2>&1 || die "The Docker Compose plugin is missing (docker compose)."
[ -f docker-compose.yml ] || die "docker-compose.yml is missing in $(pwd)."

if [ -d ../.git ] && command -v git >/dev/null; then
  say "Updating the stack files"
  git -C .. pull --ff-only || echo "Could not pull, continuing with the files on disk."
fi

say "Configuration"
umask 077
touch .env
get_env() { grep "^$1=" .env | tail -n1 | cut -d= -f2- || true; }
set_env() {
  if grep -q "^$1=" .env; then
    sed -i "s|^$1=.*|$1=$2|" .env
  else
    echo "$1=$2" >> .env
  fi
}
[ -n "$(get_env DB_PASSWORD)" ] || set_env DB_PASSWORD "$(openssl rand -hex 24)"
[ -n "$(get_env LIVEKIT_SECRET)" ] || set_env LIVEKIT_SECRET "$(openssl rand -hex 32)"
APP_PORT=${APP_PORT:-$(get_env APP_PORT)}
APP_PORT=${APP_PORT:-8090}
LIVEKIT_PORT=${LIVEKIT_PORT:-$(get_env LIVEKIT_PORT)}
LIVEKIT_PORT=${LIVEKIT_PORT:-7880}
set_env CHAT_DOMAIN "$CHAT_DOMAIN"
set_env LIVEKIT_DOMAIN "$LIVEKIT_DOMAIN"
set_env APP_PORT "$APP_PORT"
set_env LIVEKIT_PORT "$LIVEKIT_PORT"
[ -z "${HEARTH_IMAGE:-}" ] || set_env HEARTH_IMAGE "$HEARTH_IMAGE"
chmod 600 .env

busy() {
  local port=$1 owner
  owner=$(docker ps --filter "publish=$port" --format '{{.Names}}' | grep -v '^hearth-' | head -n1 || true)
  if [ -n "$owner" ]; then
    echo "$owner"
  elif ss -ltn "( sport = :$port )" 2>/dev/null | grep -q LISTEN && ! docker ps --filter "publish=$port" --format '{{.Names}}' | grep -q '^hearth-'; then
    echo "another program"
  fi
}
for pair in "APP_PORT:$APP_PORT" "LIVEKIT_PORT:$LIVEKIT_PORT"; do
  name=${pair%%:*}
  port=${pair#*:}
  owner=$(busy "$port")
  [ -z "$owner" ] || die "Port $port is used by $owner. Run again with a free port, e.g. $name=8097 ./deploy.sh $CHAT_DOMAIN $LIVEKIT_DOMAIN"
done

say "Pulling images and starting containers"
docker compose --env-file .env pull
docker compose --env-file .env up -d --remove-orphans

say "Waiting for the app to respond"
ok=0
for _ in $(seq 1 60); do
  if curl -fsS -o /dev/null "http://127.0.0.1:$APP_PORT/" 2>/dev/null; then
    ok=1
    break
  fi
  sleep 2
done
[ "$ok" -eq 1 ] || { docker compose --env-file .env logs --tail 40 app; die "The app is not responding on port $APP_PORT."; }
echo "App is running on 127.0.0.1:$APP_PORT."

BLOCK=$(cat <<CADDY

$CHAT_DOMAIN {
	encode zstd gzip
	request_body {
		max_size 30MB
	}
	reverse_proxy 127.0.0.1:$APP_PORT
}

$LIVEKIT_DOMAIN {
	reverse_proxy 127.0.0.1:$LIVEKIT_PORT
}
CADDY
)

say "Caddy"
if command -v caddy >/dev/null && [ -f "$CADDYFILE" ]; then
  if grep -qE "^$CHAT_DOMAIN[ ,{]" "$CADDYFILE"; then
    echo "$CHAT_DOMAIN is already in $CADDYFILE, leaving Caddy unchanged."
  else
    backup="$CADDYFILE.bak.$(date +%Y%m%d%H%M%S)"
    cp "$CADDYFILE" "$backup"
    printf '%s\n' "$BLOCK" >> "$CADDYFILE"
    if caddy validate --config "$CADDYFILE" --adapter caddyfile >/dev/null 2>&1; then
      systemctl reload caddy
      echo "Caddy reloaded. Backup of the previous file: $backup"
    else
      cp "$backup" "$CADDYFILE"
      die "The new Caddyfile was invalid and has been restored. Please add this block by hand:$BLOCK"
    fi
  fi
else
  echo "No Caddy found on the host. Add this block to your Caddy configuration:"
  printf '%s\n' "$BLOCK"
fi

if command -v ufw >/dev/null && ufw status | grep -q "Status: active"; then
  say "Firewall"
  ufw allow 7881/tcp >/dev/null
  ufw allow 7882/udp >/dev/null
  echo "Opened 7881/TCP and 7882/UDP for voice channels."
fi

say "Done"
cat <<TXT
Chat:         https://$CHAT_DOMAIN
Voice server: wss://$LIVEKIT_DOMAIN
The first account to register becomes the instance admin.

DNS: Both domains must point to this server. On Cloudflare, add the
LiveKit domain without the proxy (grey cloud) and open 7881/TCP and
7882/UDP in your provider's firewall.

New app versions arrive through Watchtower or by running this script again.
In Portainer the stack shows up as "hearth".
TXT
