# Tokscale UI

A lightweight desktop companion for viewing your AI coding usage, cost, and quotas from the menubar, tray, or a dashboard.

![Screenshot placeholder](docs/screenshot.png)

## Features

- **Menubar readout**: show cost (for example `$48.12`), tokens (for example `61.4M`), or icon only. Windows uses a tooltip.
- **Tray popover**: a hidden 360 px WebView that appears in under 16 ms.
- **Dashboard**: 10 views with a 1280×820 default window.
  - Overview
  - Usage
  - Models
  - Daily
  - Hourly
  - Monthly
  - Sessions, with a details drawer
  - Projects
  - Stats
  - Agents
- **Fast startup**: the cache is read in under 5 ms. If the cache schema changes, the app falls back to the forward-compatible `tokscale` CLI JSON output.
- **Sync**: offline background sync runs on a timer, and you can trigger a manual sync with credentialed providers.
- **Harness support**: harnesses are discovered dynamically. Known harnesses get curated colors and icons. Unknown harnesses get a generated color and a two-character monogram.
- **Subscription quotas**: quota meters show your plan usage.
- **Notifications**: optional OS notifications for quota at or above 85% or a daily budget threshold. Off by default.
- **Low memory**: about 20–30 MB idle. The dashboard WebView is destroyed when closed and recreated from Rust state in about 100 ms.

## Requirements

Tokscale UI needs the `tokscale` CLI. Install it with one of these commands:

```bash
bun add -g tokscale@latest
# or
npm i -g tokscale@latest
```

You can also use the one-click install in the app's Quick Settings.

**Linux (GNOME):** system tray support requires the AppIndicator extension and `libayatana-appindicator`.

## Install

### macOS (no Apple Developer ID required)

Recommended (clears Gatekeeper quarantine automatically on install; future updates happen seamlessly in-app):

```bash
brew trust --cask munim/tap/tokscale-ui   # once for third-party tap on Homebrew 6+
brew install --cask munim/tap/tokscale-ui
```

Or without Homebrew:

```bash
curl -fsSL https://raw.githubusercontent.com/munim/tokscale-ui/main/scripts/install.sh | sh
```

If you download the `.dmg` manually from [GitHub Releases](../../releases) in a browser, clear quarantine once after dragging `Tokscale UI.app` to `/Applications`:

```bash
xattr -dr com.apple.quarantine "/Applications/Tokscale UI.app"
```

### Windows

Recommended (per-user install without browser SmartScreen Mark of the Web):

```powershell
# Scoop
scoop bucket add munim https://github.com/munim/scoop-bucket
scoop install tokscale-ui

# Or PowerShell installer (silent per-user NSIS)
irm https://raw.githubusercontent.com/munim/tokscale-ui/main/scripts/install.ps1 | iex
```

You can also download `Tokscale UI_<version>_x64-setup.exe` directly from [GitHub Releases](../../releases).

### Linux

Recommended (`.AppImage` with automatic in-app self-updates and desktop entry):

```bash
curl -fsSL https://raw.githubusercontent.com/munim/tokscale-ui/main/scripts/install.sh | sh
```

Or download `.AppImage` (requires FUSE: `libfuse2` / `libfuse2t64` on Ubuntu/Debian) or `.deb` (`sudo apt install ./tokscale-ui_*.deb`) from [GitHub Releases](../../releases).

### Automatic Updates

Tokscale UI checks GitHub Releases silently on startup and every 6 hours (verified via minisign, independent of OS code-signing). Self-updating installs (macOS `.app`, Windows NSIS per-user, Linux `.AppImage`) download and apply updates in-app. Read-only, Scoop, or `.deb` installs notify you with the exact upgrade command.

## Privacy

- Tokscale UI reads only the local `tokscale` cache (`~/.config/tokscale/cache/`) and local harness session logs.
- UI settings are stored in an isolated `preferences.json` in the app config directory.
- Tokscale UI never modifies `~/.config/tokscale/settings.json`.
- Session title enrichment uses `tokscale`'s free local summarizer only. No paid cloud LLM is called.

## Development

```bash
bun install
bun run tauri dev                      # run the desktop app
VITE_USE_MOCKS=1 bun run dev           # run the frontend with mock data
bun run check                          # type and Svelte checks
bun run test                           # frontend tests
cd src-tauri && cargo test             # Rust tests
cd src-tauri && cargo clippy -- -D warnings
```

## Release

See [docs/release.md](docs/release.md) for the full release runbook, including signing keys, versioning, tagging, and verifying `latest.json`.

## Credits

Tokscale UI is built on top of [`@junhoyeo/tokscale`](https://github.com/junhoyeo/tokscale) by Junho Yeo. It is an upgrade-compatible companion to `tokscale`, not a fork or replacement. Huge thanks to the upstream project for the CLI and data model that make this app possible.

## License

MIT (to be confirmed by maintainer).
