# Spotify

[English](../en/connections.md) · [Alle Dokus](../README.md)

Unter **Einstellungen → Verbindungen** verknüpfen Nutzer ihr Spotify-Konto. Der laufende Titel erscheint dann als Aktivität und das Konto im Profil. Ein Spiel oder eine von Hand gesetzte Aktivität geht vor.

1. Unter [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard) eine App mit „Web API“ anlegen.
2. `https://<chat-domain>/api/spotify/callback` als Redirect-URI eintragen.
3. `SPOTIFY_CLIENT_ID` und `SPOTIFY_CLIENT_SECRET` in Portainer oder in `deploy/.env` setzen und neu deployen.

Solange die Spotify-App im Entwicklungsmodus ist, können nur Konten verbinden, die in der Nutzerliste der App stehen.
