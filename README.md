# OmniStore

Every cloud plan, device and drive in one dashboard. Windows first, macOS and iOS from the same codebase (Tauri 2 + React + Rust). No accounts and no servers. Users never see setup or keys.

## Download

Latest installer: https://github.com/Aristotlev/omnistore/releases/latest

Installed copies check for updates on launch and every 6 hours, then update in one click.

## Publish a new version

Double-click **Publish OmniStore.cmd**. It:

1. bumps the version if the current one is already released
2. commits, tags and pushes to GitHub
3. GitHub's Windows servers build and sign the installer and publish the release with `latest.json`
4. every installed OmniStore sees the update
5. the new installer lands in `Desktop\OmniStore` and opens

`release-keys\` holds the update signing key. It is git-ignored. Back it up: without it, installed copies cannot verify new updates.

## How sign-in works

| Account | What the user does | What OmniStore shows |
|---|---|---|
| Apple | Signs in once on Apple's own icloud.com page inside OmniStore (passkey QR with iPhone, iPhone approval, or password + code) | Plan, usage by category, over-quota status, Apple devices on the account |
| Google | Clicks Continue with Google, picks their account in the browser (passkey or already signed in), clicks Continue | Storage plan, total, Drive, Drive trash, Gmail + Photos combined |

## Device sync

Each install writes a small snapshot to `iCloud Drive\OmniStore\Devices` (or OneDrive). The user's own sync app carries it to their other devices. No server.

## Local development

```powershell
npm install
npm run tauri dev
```

Browser preview with demo data: `npm run dev`, open http://localhost:1420. Rust tests: `cd src-tauri` then `cargo test`.

## Layout

```
src/                     React dashboard (demo data when run outside Tauri)
src-tauri/src/
  lib.rs                 commands, refresh pipeline, updater
  local.rs               volumes, hardware, sync apps, Recycle Bin, iOS backups, USB devices, device sync
  scan.rs                placeholder-aware folder sizing, large files
  icloud.rs              icloud.com sign-in window + storage reader
  plans.rs / rules.rs    plan matching, overage, forecast, recommendations
  store.rs               settings and dashboard cache
src-tauri/resources/plans.json   EUR plan catalogue
ci/release.yml           GitHub build + release workflow (copied to .github/workflows by publish.ps1)
```

## Google sign-in registration

Registered in Google Cloud project `omnistore-514414` (Google Auth Platform, Desktop client "OmniStore Windows", scope `drive.appdata`, non-sensitive). The client ID and secret live in `release-keys\google.json` (git-ignored) and in GitHub Actions secrets, and are baked into release builds. Website for the consent screen: https://aristotlev.github.io/omnistore/ (served from `docs/`).
