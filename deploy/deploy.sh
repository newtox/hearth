#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<TXT
Usage: sudo ./deploy.sh <chat-domain> <livekit-domain>
Example: sudo ./deploy.sh chat.example.com livekit.example.com

Optional environment variables:
  REPO          Git URL of the repository (default: https://github.com/newtox/hearth.git)
  BRANCH        Branch                    (default: main)
  DIR           Target directory          (default: /opt/hearth)
  APP_PORT      Local port of the app     (default: 8090)
  LIVEKIT_PORT  Local port of LiveKit     (default: 7880)
  CADDYFILE     Path to the Caddyfile     (default: /etc/caddy/Caddyfile)
TXT
  exit 1
}

[ $# -eq 2 ] || usage
CHAT_DOMAIN=$1
LIVEKIT_DOMAIN=$2
REPO=${REPO:-https://github.com/newtox/hearth.git}
BRANCH=${BRANCH:-main}
DIR=${DIR:-/opt/hearth}
APP_PORT=${APP_PORT:-8090}
LIVEKIT_PORT=${LIVEKIT_PORT:-7880}
CADDYFILE=${CADDYFILE:-/etc/caddy/Caddyfile}

say() { printf '\n==> %s\n' "$*"; }
die() { printf 'Error: %s\n' "$*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || die "Please run as root or with sudo."
for tool in git docker openssl curl; do
  command -v "$tool" >/dev/null || die "$tool is not installed."
done
docker compose version >/dev/null 2>&1 || die "The Docker Compose plugin is missing (docker compose)."

say "Fetching code"
if [ -d "$DIR/.git" ]; then
  git -C "$DIR" fetch origin "$BRANCH"
  git -C "$DIR" checkout "$BRANCH"
  git -C "$DIR" pull --ff-only origin "$BRANCH"
else
  git clone --branch "$BRANCH" "$REPO" "$DIR"
fi
cd "$DIR/deploy"

say "Configuration"
if [ ! -f .env ]; then
  umask 077
  cat > .env <<ENV
DB_PASSWORD=$(openssl rand -hex 24)
LIVEKIT_SECRET=$(openssl rand -hex 32)
ENV
  echo "Generated new secrets in $DIR/deploy/.env."
fi
set_env() {
  if grep -q "^$1=" .env; then
    sed -i "s|^$1=.*|$1=$2|" .env
  else
    echo "$1=$2" >> .env
  fi
}
set_env CHAT_DOMAIN "$CHAT_DOMAIN"
set_env LIVEKIT_DOMAIN "$LIVEKIT_DOMAIN"
set_env APP_PORT "$APP_PORT"
set_env LIVEKIT_PORT "$LIVEKIT_PORT"
chmod 600 .env
LIVEKIT_SECRET=$(grep '^LIVEKIT_SECRET=' .env | cut -d= -f2)

cat > livekit.generated.yaml <<YAML
port: 7880
rtc:
  tcp_port: 7881
  udp_port: 7882
  use_external_ip: true
keys:
  hearth: $LIVEKIT_SECRET
YAML
chmod 644 livekit.generated.yaml

say "Building and starting containers (the first build takes a few minutes)"
docker compose --env-file .env up -d --build --remove-orphans

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

To update, run this script again.
In Portainer the whole thing shows up as the stack "hearth".
TXT
