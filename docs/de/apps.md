# Apps

[English](../en/apps.md) · [Alle Dokus](../README.md)

Die Apps sind die Hearth-Weboberfläche in einer kleinen nativen Hülle (Tauri). Beim ersten Start fragen sie nach der Serveradresse, zum Beispiel `https://chat.example.com`.

## Herunterladen

Jede Version liegt unter [Releases](https://github.com/newtox/hearth/releases):

| Datei | Für | Installieren |
|---|---|---|
| `Hearth_<version>_windows_x64_setup.exe` | Windows 10 und 11 | Ausführen. Die App ist nicht signiert, Windows zeigt „Der Computer wurde durch Windows geschützt“: **Weitere Informationen → Trotzdem ausführen**. |
| `Hearth_<version>_linux_x86_64.flatpak` | Linux | `flatpak install --user Hearth_<version>_linux_x86_64.flatpak`, dann `flatpak run de.newtox.hearth`. Braucht Flathub für die GNOME-Laufzeit. |
| `Hearth_<version>_linux_x86_64.AppImage` | Linux | Datei mit `chmod +x` ausführbar machen und starten. |
| `Hearth_<version>_linux_amd64.deb` | Debian, Ubuntu | `sudo apt install ./Hearth_<version>_linux_amd64.deb` |
| `Hearth_<version>_android.apk` | Android 7 und neuer | Datei auf dem Handy öffnen und einmal erlauben, Apps aus dieser Quelle zu installieren. |

## Neue Version veröffentlichen

```sh
git tag v0.2.0
git push origin v0.2.0
```

Der Workflow „Apps“ baut alle Dateien und hängt sie an ein neues Release mit dem Namen des Tags. Das dauert etwa 20 bis 30 Minuten. Derselbe Tag veröffentlicht auch das Server-Image als `ghcr.io/newtox/hearth:0.2.0`. Unter **Actions → Apps → Run workflow** kannst du ohne Release bauen, die Dateien liegen dann nur bei den Artifacts des Laufs.

## Grenzen

- Die APK ist ein Debug-Build. Sie lässt sich normal installieren, für den Play Store bräuchte sie einen Signaturschlüssel.
- Android verbindet sich nur mit HTTPS-Servern.
- Bildschirm teilen gibt es auf Android nicht, weil die System-WebView es nicht anbietet. Zuschauen geht.
- Unter Linux braucht Sprache eine WebKitGTK mit WebRTC. Ubuntu und Debian bauen sie ohne, deshalb zeigt die App dann einen Hinweis und öffnet Hearth im Browser. In Chrome lässt sich die Seite als App installieren, die dann wie eine Desktop-App mit Sprache und Bildschirm teilen funktioniert.
- Die Windows- und Linux-Builds sind nicht signiert.

## Selbst bauen

Linux, braucht Rust, Node 22 und `libwebkit2gtk-4.1-dev`:

```sh
cd web
npm install
npx tauri build
```

Die Dateien landen in `web/src-tauri/target/release/bundle/`.
