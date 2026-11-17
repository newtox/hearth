# Apps

[Deutsch](../de/apps.md) · [All docs](../README.md)

The apps are the Hearth web interface inside a small native shell (Tauri). On first start they ask for the server address, for example `https://chat.example.com`.

## Download

Every version is on the [Releases](https://github.com/newtox/hearth/releases) page:

| File | For | Install |
|---|---|---|
| `Hearth_<version>_windows_x64_setup.exe` | Windows 10 and 11 | Run it. The app is not signed, so Windows shows "Windows protected your PC": **More info → Run anyway**. |
| `Hearth_<version>_linux_x86_64.flatpak` | Linux | `flatpak install --user Hearth_<version>_linux_x86_64.flatpak`, then `flatpak run de.newtox.hearth`. Needs the Flathub remote for the GNOME runtime. |
| `Hearth_<version>_linux_x86_64.AppImage` | Linux | `chmod +x` the file and start it. |
| `Hearth_<version>_linux_amd64.deb` | Debian, Ubuntu | `sudo apt install ./Hearth_<version>_linux_amd64.deb` |
| `Hearth_<version>_android.apk` | Android 7 and newer | Open the file on the phone and allow installing from this source once. |

## Publishing a new version

```sh
git tag v0.2.0
git push origin v0.2.0
```

The "Apps" workflow builds every file and attaches them to a new release named after the tag. It takes about 20 to 30 minutes. The same tag also publishes the server image as `ghcr.io/newtox/hearth:0.2.0`. Under **Actions → Apps → Run workflow** you can build without a release; the files are then only in the run's artifacts.

## Limits

- The APK is a debug build. It installs fine but cannot go to the Play Store without a signing key.
- Android only connects to HTTPS servers.
- Screen sharing is not available on Android because the system WebView does not offer it. Watching a share works.
- On Linux, voice needs a WebKitGTK built with WebRTC. Ubuntu and Debian build it without, so the app shows a note and opens Hearth in the browser instead. In Chrome you can install the web page as an app, which then works like a desktop app including voice and screen sharing.
- Windows and Linux builds are not code-signed.

## Building yourself

Linux, needs Rust, Node 22 and `libwebkit2gtk-4.1-dev`:

```sh
cd web
npm install
npx tauri build
```

The files land in `web/src-tauri/target/release/bundle/`.
