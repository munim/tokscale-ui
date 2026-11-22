# AGENTS.md

Tauri v2, Rust, and Svelte 5 (runes) desktop companion for `@junhoyeo/tokscale` CLI.

## Cost-Efficient Exploration

- Never scan or read `src-tauri/target/`, `dist/`, `node_modules/`, `src-tauri/gen/`, `bun.lock`, `src-tauri/Cargo.lock`, or `src-tauri/tests/fixtures/`.
- Never glob repo root unfiltered; `src-tauri/target/` holds thousands of build artifacts.
- Search symbols first (`grep` or `fsearch`) scoped to `src/` or `src-tauri/src/`.
- Read bounded line slices (`offset` and `limit`) on large files: `src-tauri/src/data/mod.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/contracts.rs`.
- Jump straight to owner module; skip broad repo walks:
  - IPC contracts and mocks: `src-tauri/src/contracts.rs`, `src/lib/contracts/ipc.ts`, `src/lib/contracts/mocks.ts`
  - Tauri commands, `AppState`, startup hydration: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`
  - Cache schema, range slicing, session-model join: `src-tauri/src/data/`
  - `tokscale` locator, serialized subprocess queue, CLI args, provider sync: `src-tauri/src/cli/`
  - Tray icon, popover, dashboard window lifecycle, `preferences.json`: `src-tauri/src/shell/`
  - Frontend apps: `src/tray/` (`tray.html`) and `src/dashboard/` (`dashboard.html`); shared code in `src/lib/` (`$lib`).
- Never cross-import between `src/tray/` and `src/dashboard/`.
- Delegate searches, small edits, and diff checks to `cavecrew` (`cavecrew-investigator`, `cavecrew-builder`, `cavecrew-reviewer`) for compressed `path:line` output.
- If using `explore`, set thoroughness `quick` and scope to one directory.
- Run single targeted tests while iterating; run full suite once before finish.

## Commands

Toolchain: Bun `1.4.1` (`bun install --frozen-lockfile`), Rust stable. No ESLint or Prettier; `svelte-check` (`noUnusedLocals`, `noUnusedParameters`) and `clippy -D warnings` gate CI.

```bash
bun run tauri dev                                                            # full desktop app
VITE_USE_MOCKS=1 bun run dev                                                 # browser UI on :1420 (strictPort)
bun run check                                                                # svelte-check + TypeScript
bun run test                                                                 # all Vitest tests
bunx vitest run src/tray/tray.test.ts -t "pattern"                           # single frontend test
cargo test --manifest-path src-tauri/Cargo.toml                              # all Rust unit + integration tests
cargo test --manifest-path src-tauri/Cargo.toml <test_name>                  # single Rust test
cargo test --manifest-path src-tauri/Cargo.toml --test live_sync_regression  # single Rust integration file
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings             # Rust lint gate
bun run tauri build --debug --no-sign                                        # CI smoke build
```

- Dual HTML entrypoints (`tray.html`, `dashboard.html`); no `index.html`.
- Browser dev `/` returns 404. Open `http://localhost:1420/tray.html` or `http://localhost:1420/dashboard.html`.

## Architecture & Lockstep Invariants

- **Manual IPC parity (no codegen)**: Update these files together for any command, `tokscale://*` event, or payload change:
  1. `src-tauri/src/contracts.rs` (preserve serde `camelCase`/`snake_case` and explicit renames `"7d"`, `"30d"`, `"retryClient"`, `"msPer1KTokens"`).
  2. `src-tauri/src/commands.rs` and `tauri::generate_handler![...]` in `src-tauri/src/lib.rs`.
  3. `src/lib/contracts/ipc.ts` (`COMMANDS`, `EVENTS`, types, mock fallback) and `src/lib/contracts/mocks.ts`.
- **Mirrored harness hashing**: `src-tauri/src/data/harness.rs` and `src/lib/harnesses/registry.ts` duplicate 64-bit FNV-1a (`hue = hash % 360`) and curated palettes (`claude`, `codex`, `opencode`, `gemini`, `other`). Keep both identical.
- **Window lifecycle**: `tray` WebView (`tray.html`, 360px) stays hidden in background and resizes via `resize_tray`.
- `dashboard` WebView (`dashboard.html`) is absent from `tauri.conf.json`. `src-tauri/src/shell/dashboard.rs` creates it on demand and destroys it on close.
- Reopening dashboard serves state from in-memory `DataService` with zero extra disk reads.
- Cold dashboard open passes target route via `DashboardSnapshot.pendingHandoff`; warm open emits `tokscale://dashboard-navigate`.
- macOS switches `ActivationPolicy::Accessory` (tray only) and `Regular` (dashboard open); requires `macOSPrivateApi: true`.
- **Cache vs CLI state**: Startup reads `~/.config/tokscale/cache/tui-data-cache.json` (`EXPECTED_SCHEMA_VERSION = 12`) and `subscription-usage-cache.json` (`TOKSCALE_CONFIG_DIR` overrides `~/.config/tokscale`).
- Missing, corrupt, or drifted cache (or missing today data) triggers CLI fallback. Later `hydrate_from_disk()` calls must not clobber newer `cli_synced_at` state.

## `tokscale` CLI & Privacy Rules

- `tokscale` has **no** `daily` subcommand (exits 2). Read daily and live today usage via `graph --no-spinner --offline` and `hourly --json --no-spinner --offline --week`.
- `tokscale report` rejects `--no-spinner` (`debug_assert!` in `src-tauri/src/cli/commands.rs`).
- Session title enrichment (`report --json`) never passes `--summarizer` (free local summarizer only, no cloud LLM) and runs only for `today` or `7d`.
- Route all `tokscale` and installer (`bun`/`npm`) subprocesses through `CliService`. `CliQueue` serializes runs, deduplicates identical in-flight argv, and sets `NO_COLOR=1`.
- Never modify `~/.config/tokscale/settings.json`. Write UI preferences only to app config `preferences.json` via `shell::prefs::patch_preferences` (atomic `.json.tmp` rename; preserves unknown keys).
- Never read or log provider credential files (`cursor`, `trae`, `warp`, `capy`, `antigravity`). Check file existence with `std::fs::metadata` only (`src-tauri/src/cli/providers.rs`).

## Testing Gotchas

- **Vitest mock mode**: `isMockMode()` in `src/lib/contracts/ipc.ts` returns `true` when `window.__TAURI_INTERNALS__` is missing or `VITE_USE_MOCKS=1`.
- Component tests for real `invoke`/`listen` wiring must set `window.__TAURI_INTERNALS__ = {}` in `beforeEach`, delete it in `afterEach`, and clear `document.documentElement.dataset.theme`.
- **Hermetic Rust tests**: Use `ProbeEnv::isolated(...)`, `DataService::with_paths(...)`, and `AppState::new_for_test(...)`. Never touch real `~/.config/tokscale` or host `PATH`.

## Commits & Releases

- **Commits**: Use `caveman-commit` skill. Write Conventional Commits `<type>(<scope>): <imperative summary>` with lowercase summary after colon, <=50 chars target (72 max), no period, no AI attribution. Add body only for non-obvious *why* or breaking changes.
- **Releases**: Bump version identically in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Follow `docs/release.md` for signing keys and `v*.*.*` tag builds.
