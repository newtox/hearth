# Development

[Deutsch](../de/entwicklung.md) · [All docs](../README.md)

Requirements: Rust, Node 22, Docker.

```sh
docker compose up -d
cp .env.example server/.env
cd web && npm install && npm run build
cd ../server && cargo run --release
```

The app runs at http://localhost:8080 and creates its tables on start. For hot reload, keep the server running and start `npm run dev` in `web`.

## Layout

- `server/` Rust backend: Axum, WebSocket gateway, PostgreSQL through sqlx, migrations in `server/migrations`
- `web/` Svelte 5 interface, translations in `web/src/lib/i18n.svelte.ts` (German keys, English values)
- `web/src-tauri/` Tauri 2 shell for the apps
- `deploy/` stack for servers
- `.github/workflows/image.yml` server image, `.github/workflows/apps.yml` apps and releases

## Tests

```sh
pip install requests websockets
python3 scripts/smoke.py http://127.0.0.1:8080
```

Role sync against a fake platform: start `python3 scripts/mock_platform.py`, run the server with `PUBLIC_URL=http://127.0.0.1:8080 CONNECT_MOCK_URL=http://127.0.0.1:9099 INTEGRATION_SYNC_SECS=5`, then `python3 scripts/integrations.py`.

Password reset by email: `pip install aiosmtpd`, start `python3 scripts/mock_smtp.py`, run the server with `PUBLIC_URL=http://127.0.0.1:8080 SMTP_HOST=127.0.0.1 SMTP_PORT=2525 SMTP_TLS=none SMTP_FROM=chat@example.com`, then `python3 scripts/password_reset.py`.
