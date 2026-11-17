# Verbindungen und Rollen-Abgleich

[English](../en/connections.md) · [Alle Dokus](../README.md)

Unter **Einstellungen → Verbindungen** verknüpfen Nutzer andere Konten, die dann im Profil stehen. Eine Plattform erscheint erst, wenn ihre Zugangsdaten gesetzt sind, und `PUBLIC_URL` (setzt der Stack selbst) muss die öffentliche Chat-Adresse sein.

| Plattform | Was sie macht | Variablen | Redirect-URI zum Eintragen |
|---|---|---|---|
| Spotify | Zeigt den laufenden Titel als Aktivität | `SPOTIFY_CLIENT_ID`, `SPOTIFY_CLIENT_SECRET` | `https://<chat-domain>/api/spotify/callback` |
| Steam | Zeigt das Steam-Profil | `STEAM_API_KEY` | keine, Steam nutzt OpenID |
| Twitch | Zeigt den Kanal, Abonnenten-Rollen | `CONNECT_TWITCH_ID`, `CONNECT_TWITCH_SECRET` | `https://<chat-domain>/api/connections/twitch/callback` |
| YouTube | Zeigt den Kanal, Mitglieder-Rollen | `CONNECT_YOUTUBE_ID`, `CONNECT_YOUTUBE_SECRET` | `https://<chat-domain>/api/connections/youtube/callback` |

Nach dem Eintragen den Stack neu deployen.

## Woher die Zugangsdaten kommen

- **Spotify**: [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard), App mit „Web API“ anlegen. Im Entwicklungsmodus können nur Konten verbinden, die in der Nutzerliste der App stehen.
- **Steam**: [steamcommunity.com/dev/apikey](https://steamcommunity.com/dev/apikey)
- **Twitch**: [dev.twitch.tv/console/apps](https://dev.twitch.tv/console/apps), Kategorie „Chat Bot“ oder „Website Integration“, Client-Typ „Confidential“.
- **YouTube**: [console.cloud.google.com](https://console.cloud.google.com), „YouTube Data API v3“ aktivieren, OAuth-Client vom Typ „Webanwendung“ anlegen. Solange der Zustimmungsbildschirm im Testmodus ist, können nur Testnutzer verbinden.

## Twitch- und YouTube-Rollen

Wer **Server verwalten** darf, verbindet unter **Servereinstellungen → Integrationen** einen Kanal. Hearth legt eine Rolle an, lila „Twitch Subscriber“ oder rot „YouTube Member“, und gibt sie jedem Mitglied, das

1. sein eigenes Twitch- oder YouTube-Konto in den Einstellungen verbunden hat und
2. den Twitch-Kanal abonniert hat oder Mitglied des YouTube-Kanals ist.

Endet das Abo, verschwindet die Rolle wieder. Abgeglichen wird alle 10 Minuten, sofort, wenn jemand ein Konto verbindet, trennt oder dem Server beitritt, und wenn du **Jetzt abgleichen** drückst.

Name, Farbe und Rechte der Rolle lassen sich ändern. Von Hand vergeben oder entziehen kann man sie nicht, und sie bleibt, bis du die Integration trennst.

- Twitch-Abos gibt es nur für Affiliate- und Partner-Kanäle.
- YouTube gibt die Mitgliederliste nur an Creator heraus, die für die Memberships-API freigeschaltet sind, praktisch Kanäle mit einem YouTube Partner Manager. Ohne diese Freigabe lässt sich die Integration verbinden, zeigt aber einen Fehler an, statt Rollen zu vergeben.
