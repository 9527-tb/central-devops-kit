# Central DevOps Kit

A desktop app. The stack matches DevKit: Tauri 2, Vue 3, Vite, and antdv-next. It runs on **macOS, Windows, and Linux**. The interface is available in Simplified Chinese, Traditional Chinese, English, and Japanese.

It currently includes GitLab variable comparison, port lookup, settings and a tray icon, and in-app updates. DevKit’s workbench, run plans, and project scan are not included.

## Features

- **GitLab variables.** Enter a group URL and an access token, then load variables and compare `connect-staging` with `connect-production` side by side. Missing keys and differing values are visible, and names or values can be copied.
- **Port lookup.** Find the process listening on a port. An exact query matches 1–65535. With fuzzy match, `80` also matches `8080` and `18080`. After confirmation, that process can be stopped. macOS and Linux use `lsof`. Windows uses `netstat`.
- **Settings.** Theme, language, launch at login, close to tray, and check for updates on startup. Closing the window can hide it in the tray. The window uses the system title bar.
- **In-app updates.** The app checks about 3 seconds after launch. The same check is available from the left rail and from Settings. A newer version opens a dialog; confirming it downloads, installs, and restarts the app.

The left rail lists GitLab variables and port lookup, then Settings at the bottom, then Check for updates. Hovering an icon shows its name. The current page stays selected.

## Start

Node.js, pnpm, and Rust are required. The desktop build also needs the Tauri system dependencies for the host platform.

```bash
pnpm install
pnpm tauri dev
```

Frontend only (no tray, settings file, or port lookup):

```bash
pnpm dev
```

Settings are stored at:

- macOS / Linux: `~/.central-devops-kit/settings.json`
- Windows: `%APPDATA%\central-devops-kit\settings.json`

## Add a feature

- Put the page in `src/views/`.
- Put the feature in `src/features/<name>/`.
- Put state in `src/stores/`.
- Add a left-rail item in `items` inside `src/features/app/AppRail.vue` (do not set `bottom`), add `goXxx` in `src/stores/appNav.js`, and mount the view in `src/App.vue`.
- Add copy for all four languages in `src/i18n/locales/`.
- Put Rust logic in its own module under `src-tauri/src/`. `commands.rs` only forwards IPC.

## Release

Versions start at **0.0.1**. `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` stay on the same version.

A push to `master` publishes through GitHub Actions on Node.js 24. With no existing tags, the configured version is published. If the configured version is already higher than the latest tag, that configured version is published. Otherwise the patch number of the latest tag is incremented. A commit message containing `[skip ci]` or `[ci skip]` does not publish.

Build targets:

- macOS Apple Silicon and macOS Intel
- Windows
- Linux (deb, rpm, and AppImage)

After the build, the workflow renames Tauri’s default asset names so the operating system and file extension stay visible. A matching `.sig` file is the signature. `macOS_*.app.tar.gz` is the in-app updater package, not the everyday installer.

| File | Description |
| --- | --- |
| `Central-DevOps-Kit_<version>_macOS_AppleSilicon.dmg` | macOS Apple Silicon installer |
| `Central-DevOps-Kit_<version>_macOS_Intel.dmg` | macOS Intel installer |
| `Central-DevOps-Kit_<version>_Windows_x64_setup.exe` | Windows installer |
| `Central-DevOps-Kit_<version>_Windows_x64.msi` | Windows MSI installer |
| `Central-DevOps-Kit_<version>_Linux_x64.deb` | Linux Debian/Ubuntu package |
| `Central-DevOps-Kit_<version>_Linux_x64.rpm` | Linux RPM package |
| `Central-DevOps-Kit_<version>_Linux_x64.AppImage` | Linux AppImage |

## In-app updates

The client reads these manifests in order:

- `https://ghfast.top/https://github.com/9527-tb/central-devops-kit/releases/latest/download/latest.json`
- `https://cdn.jsdelivr.net/gh/9527-tb/central-devops-kit@master/updater/latest.json`
- `https://github.com/9527-tb/central-devops-kit/releases/latest/download/latest.json`

Packages are verified with minisign. The repository secret `TAURI_SIGNING_PRIVATE_KEY` is the full contents of `~/.tauri/central-devops-kit.key`. The private key has no password. After a successful publish, the workflow rebuilds `latest.json` from the release assets and writes it back to `updater/latest.json`.
