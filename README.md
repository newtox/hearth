# Hearth

Self-hosted chat for communities: servers, channels, roles, direct messages, group chats, voice channels with screen sharing. Runs in the browser and as an app for Windows, Linux and Android. Interface in German, English and Danish.

**Documentation / Dokumentation:** [docs](docs/README.md)

| | English | Deutsch |
|---|---|---|
| Install on a server with Portainer or a script | [installation](docs/en/installation.md) | [Installation](docs/de/installation.md) |
| Download and install the apps | [apps](docs/en/apps.md) | [Apps](docs/de/apps.md) |
| All settings | [configuration](docs/en/configuration.md) | [Einstellungen](docs/de/einstellungen.md) |
| Roles and permissions | [permissions](docs/en/permissions.md) | [Rechte](docs/de/rechte.md) |
| Using Hearth | [guide](docs/en/guide.md) | [Anleitung](docs/de/anleitung.md) |
| Working on the code | [development](docs/en/development.md) | [Entwicklung](docs/de/entwicklung.md) |

## Quick start on a server

The server image is published as `ghcr.io/newtox/hearth`, the apps under [Releases](https://github.com/newtox/hearth/releases).

```sh
sudo git clone https://github.com/newtox/hearth.git /opt/hearth
cd /opt/hearth/deploy
sudo ./deploy.sh chat.example.com livekit.example.com
```

Or add [`deploy/docker-compose.yml`](deploy/docker-compose.yml) as a stack in Portainer, see [installation](docs/en/installation.md).

## Features

- Servers with categories, text and voice channels, polls, invites with expiry and usage limit
- Roles with hierarchy, colour, 21 permissions, channel rules per role and per member
- Right-click menus for members, messages, channels and servers, a server menu on the server name, and a moderator view per member
- Replies, pinned messages, search across a server with an author filter, desktop notifications
- Real-time chat: Markdown, mentions, attachments up to 25 MB, reactions, custom emojis and stickers, emoji search in German and English, typing indicator, unread markers
- Friends, direct messages, group chats with up to 10 people, three group roles and per-group permissions, blocking
- Voice channels and calls with screen sharing, watched on demand with a viewer list, and sounds
- Profiles with avatar, banner, bio, accent colour, badges with built-in icons and linked accounts
- Status online, idle (also automatic), do not disturb and invisible
- Activity status set by hand: playing, listening, watching or a custom text
- Badges earned from stream time, voice time and account age in ten colour levels
- Moderation: timeouts, kicks, bans, audit log, rate limits
- Six themes: Light, Ash, Dark, Onyx, Midnight and Parchment

## Known limits

- The Linux app needs a WebKitGTK with WebRTC for voice. Ubuntu and Debian build it without, so the app offers to open the browser instead.
- Every member of a server is loaded at once. That is fine for friend groups, not for servers with many thousands of members.

## License

[CC BY-NC-SA 4.0](LICENSE)
