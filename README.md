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

Download the artifact for your platform from [GitHub Releases](../../releases):

| Platform | Artifacts |
|---|---|
| macOS (Apple Silicon and Intel) | `.dmg` or `.app` |
| Windows | `.msi` or `.exe` |
| Linux | `.AppImage` or `.deb` |

The app includes a signed auto-updater. When a new release is published, it prompts you to upgrade.

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
