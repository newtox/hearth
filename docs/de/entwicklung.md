# Entwicklung

[English](../en/development.md) · [Alle Dokus](../README.md)

Voraussetzungen: Rust, Node 22, Docker.

```sh
docker compose up -d
cp .env.example server/.env
cd web && npm install && npm run build
cd ../server && cargo run --release
```

Die App läuft unter http://localhost:8080 und legt ihre Tabellen beim Start an. Für Hot Reload den Server laufen lassen und in `web` `npm run dev` starten.

## Aufbau

- `server/` Rust-Backend: Axum, WebSocket-Gateway, PostgreSQL über sqlx, Migrationen in `server/migrations`
- `web/` Svelte-5-Oberfläche, Übersetzungen in `web/src/lib/i18n.svelte.ts` (deutsche Schlüssel, englische Werte)
- `web/src-tauri/` Tauri-2-Hülle für die Apps
- `deploy/` Stack für Server
- `.github/workflows/image.yml` Server-Image, `.github/workflows/apps.yml` Apps und Releases

## Tests

```sh
pip install requests websockets
python3 scripts/smoke.py http://127.0.0.1:8080
```

Rollen-Abgleich gegen eine nachgebaute Plattform: `python3 scripts/mock_platform.py` starten, den Server mit `PUBLIC_URL=http://127.0.0.1:8080 CONNECT_MOCK_URL=http://127.0.0.1:9099 INTEGRATION_SYNC_SECS=5` laufen lassen, dann `python3 scripts/integrations.py`.

Passwort-Reset per E-Mail: `pip install aiosmtpd`, `python3 scripts/mock_smtp.py` starten, den Server mit `PUBLIC_URL=http://127.0.0.1:8080 SMTP_HOST=127.0.0.1 SMTP_PORT=2525 SMTP_TLS=none SMTP_FROM=chat@example.com` laufen lassen, dann `python3 scripts/password_reset.py`.
