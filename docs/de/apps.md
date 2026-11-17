# Apps

[English](../en/apps.md) · [Alle Dokus](../README.md)

Die Apps sind die Hearth-Weboberfläche in einer nativen Hülle: Tauri unter Windows und Android, Electron unter Linux. Electron bringt sein eigenes Chromium mit, deshalb funktionieren Sprache, Bildschirm teilen und der Ton eines geteilten Bildschirms auf jeder Linux-Distribution, wie bei Discord. Beim ersten Start fragen die Apps nach der Serveradresse, zum Beispiel `https://chat.example.com`.

## Herunterladen

Jede Version liegt unter [Releases](https://github.com/newtox/hearth/releases):

| Datei | Für | Installieren |
|---|---|---|
| `Hearth_<version>_windows_x64_setup.exe` | Windows 10 und 11 | Ausführen. Die App ist nicht signiert, Windows zeigt „Der Computer wurde durch Windows geschützt“: **Weitere Informationen → Trotzdem ausführen**. |
| `Hearth_<version>_linux_x86_64.flatpak` | Linux | `flatpak install --user Hearth_<version>_linux_x86_64.flatpak`, dann `flatpak run de.newtox.hearth`. Braucht Flathub für die Freedesktop-Laufzeit und die Electron-Basis. |
| `Hearth_<version>_linux_x86_64.AppImage` | Linux | Datei mit `chmod +x` ausführbar machen und starten. |
| `Hearth_<version>_linux_amd64.deb` | Debian, Ubuntu | `sudo apt install ./Hearth_<version>_linux_amd64.deb` |
| `Hearth_<version>_linux_x86_64.rpm` | Fedora, Nobara, openSUSE | `sudo dnf install ./Hearth_<version>_linux_x86_64.rpm` |
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
- Unter Linux braucht der Ton eines geteilten Bildschirms PipeWire, das aktuelle Distributionen standardmäßig nutzen. Geteilt wird jeder Ton außer dem von Hearth selbst.
- Die Windows- und Linux-Builds sind nicht signiert.

## Selbst bauen

Linux, braucht Node 22 und für das RPM-Paket `rpm`:

```sh
cd desktop
npm install
npm run web
npx electron-builder --linux AppImage deb rpm
```

Die Dateien landen in `desktop/dist/`.

Windows, braucht Rust und Node 22:

```sh
cd web
npm install
npx tauri build
```

Der Installer landet in `web/src-tauri/target/release/bundle/nsis/`.
