# Release Runbook

This document describes how to ship a signed Tokscale UI release. Releases are built by `.github/workflows/release.yml` using `tauri-apps/tauri-action@v0`, which publishes platform artifacts and a signed `latest.json` to GitHub Releases.

## 1. Generate the updater signing key pair

Run this once, from the repository root:

```bash
bunx tauri signer generate -w ~/.tauri/tokscale-ui.key
```

The command prints a public key and asks for a password. Keep both safe.

- The **public key** goes into `src-tauri/tauri.conf.json` (see step 2).
- The **private key file** and **password** are secrets. Never commit them.

## 2. Configure the public key

Set `plugins.updater.pubkey` in `src-tauri/tauri.conf.json` to the public key printed in step 1:

```json
{
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": ["https://github.com/munim/tokscale-ui/releases/latest/download/latest.json"],
      "pubkey": "<PUBLIC_KEY_FROM_tauri_signer_generate>",
      "windows": { "installMode": "passive" }
    }
  }
}
```

Confirm that `bundle.createUpdaterArtifacts` is `true`. Only change the `bundle` and `plugins.updater` sections.

## 3. Store the signing secrets in GitHub

In the repository, go to **Settings → Secrets and variables → Actions → New repository secret** and add:

| Secret name | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of the private key file (`~/.tauri/tokscale-ui.key`) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The password chosen in step 1 |

`GITHUB_TOKEN` is provided automatically. The workflow needs `contents: write`, which is already set in `release.yml`.

To check that no secrets were committed, run:

```bash
git grep -i "PRIVATE_KEY"
```

The only matches should be secret references such as `${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}`.

## 4. Bump the version

Update the version in all three files so they match:

- `package.json` → `"version"`
- `src-tauri/Cargo.toml` → `[package] version`
- `src-tauri/tauri.conf.json` → `"version"`

Commit the change on `main` after the PR is merged.

## 5. Tag and push

Create an annotated tag that matches the version, then push it:

```bash
git tag -a vX.Y.Z -m "Tokscale UI vX.Y.Z"
git push origin vX.Y.Z
```

Pushing a tag that matches `v*.*.*` triggers `.github/workflows/release.yml`. The workflow builds this matrix:

| Runner | Target | Artifacts |
|---|---|---|
| `macos-latest` | `aarch64-apple-darwin` | `.dmg`, `.app.tar.gz` |
| `macos-latest` | `x86_64-apple-darwin` | `.dmg`, `.app.tar.gz` |
| `windows-latest` | default | `.msi`, `.exe` (NSIS) |
| `ubuntu-22.04` | default | `.AppImage`, `.deb` |

The release is created as a published, non-draft, non-prerelease. Its title is `Tokscale UI vX.Y.Z`.

To test the pipeline before a real release, push a throwaway tag such as `v0.0.0-rc.1` on a fork.

## 6. Verify the release

Once the workflow is green, download `latest.json` from the release:

```bash
curl -L -o latest.json https://github.com/munim/tokscale-ui/releases/latest/download/latest.json
```

Check each of the following:

1. `version`, `notes`, and `pub_date` are present.
2. `platforms` contains all four keys:
   - `darwin-aarch64`
   - `darwin-x86_64`
   - `windows-x86_64`
   - `linux-x86_64`
3. Each platform entry has a non-empty `signature` and a `url` that points to an artifact in the release.

The release workflow fails if any platform key is missing. If that happens, re-run the failed job and check the matrix logs before publishing.

Finally, confirm the updater end to end: install an older build, launch it, and check that the update prompt appears and upgrades successfully on macOS and Windows.

## Troubleshooting

- **Signature errors in `latest.json`**: the private key or password secret does not match the `pubkey` in `tauri.conf.json`. Regenerate the key pair, update both the config and the secrets, and tag a new version.
- **Missing platform key**: a matrix job failed or was skipped. Re-run it and confirm `latest.json` is regenerated.
- **Updater does not prompt**: confirm the endpoint URL and that the installed build's version is lower than the release version.
