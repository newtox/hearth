# Apps

[Deutsch](../de/apps.md) · [All docs](../README.md)

The apps are the Hearth web interface inside a native shell: Tauri on Windows and Android, Electron on Linux. Electron brings its own Chromium, so voice, screen sharing and the sound of a shared screen work on every Linux distribution, like Discord. On first start the apps ask for the server address, for example `https://chat.example.com`.

## Download

Every version is on the [Releases](https://github.com/newtox/hearth/releases) page:

| File | For | Install |
|---|---|---|
| `Hearth_<version>_windows_x64_setup.exe` | Windows 10 and 11 | Run it. The app is not signed, so Windows shows "Windows protected your PC": **More info → Run anyway**. |
| `Hearth_<version>_linux_x86_64.flatpak` | Linux | `flatpak install --user Hearth_<version>_linux_x86_64.flatpak`, then `flatpak run de.newtox.hearth`. Needs the Flathub remote for the Freedesktop runtime and the Electron base. |
| `Hearth_<version>_linux_x86_64.AppImage` | Linux | `chmod +x` the file and start it. |
| `Hearth_<version>_linux_amd64.deb` | Debian, Ubuntu | `sudo apt install ./Hearth_<version>_linux_amd64.deb` |
| `Hearth_<version>_linux_x86_64.rpm` | Fedora, Nobara, openSUSE | `sudo dnf install ./Hearth_<version>_linux_x86_64.rpm` |
| `Hearth_<version>_android.apk` | Android 7 and newer | Open the file on the phone and allow installing from this source once. |

## Publishing a new version

```sh
git tag v0.2.0
git push origin v0.2.0
```

The "Apps" workflow builds every file and attaches them to a new release named after the tag. It takes about 20 to 30 minutes. The same tag also publishes the server image as `ghcr.io/newtox/hearth:0.2.0`. Under **Actions → Apps → Run workflow** you can build without a release; the files are then only in the run's artifacts.

## Updates

Hearth checks for a new version at startup and regularly afterwards, and shows a notice at the top. **Settings → Help** shows your version and lets you check by hand.

| Where | What happens |
| --- | --- |
| Browser | "Reload" loads the version that is running on the server. |
| Linux (AppImage, deb, rpm) | "Update now" downloads the new version and restarts Hearth. For deb and rpm the system asks for your password once. |
| Linux (Flatpak) | "Update now" downloads the new Flatpak, installs it through the system and restarts Hearth. For this the app may talk to Flatpak on your system (`org.freedesktop.Flatpak`). |
| Windows | "Update now" installs the new version once signing is set up (see below). Otherwise the release page opens. |
| Android | "Download" fetches the new APK for you to open. |

### Setting up Windows updates

Windows only installs updates signed with your key. Set this up once:

```sh
cd web
npx tauri signer generate -w ~/.tauri/hearth.key
```

Then in the repository under **Settings → Secrets and variables → Actions**:

- Secret `TAURI_SIGNING_PRIVATE_KEY`: contents of `~/.tauri/hearth.key`
- Secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the password you chose
- Variable `TAURI_UPDATER_PUBKEY`: contents of `~/.tauri/hearth.key.pub`

From the next tag on, the release contains a `latest.json`, and Windows apps from that version on update themselves. Keep the key safe: with a new key everyone has to reinstall by hand once.

## Limits

- The APK is a debug build. It installs fine but cannot go to the Play Store without a signing key.
- Android only connects to HTTPS servers.
- Screen sharing is not available on Android because the system WebView does not offer it. Watching a share works.
- On Linux the sound of a shared screen needs PipeWire, which current distributions use by default. All sound except Hearth's own is shared.
- Windows and Linux builds are not code-signed.

## Building yourself

Linux, needs Node 22, plus `rpm` for the RPM package:

```sh
cd desktop
npm install
npm run web
npx electron-builder --linux AppImage deb rpm
```

The files land in `desktop/dist/`.

Windows, needs Rust and Node 22:

```sh
cd web
npm install
npx tauri build
```

The installer lands in `web/src-tauri/target/release/bundle/nsis/`.
