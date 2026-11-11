# OmniStore

Every cloud plan, device and drive in one dashboard. Windows first, macOS and iOS from the same codebase (Tauri 2 + React + Rust).

## Run on Windows

Prerequisites (skip what you already have from OmniBoost):

```powershell
winget install --id Rustlang.Rustup -e
winget install --id OpenJS.NodeJS.LTS -e
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Run:

```powershell
cd "$HOME\Documents\OmniStore"
npm install
npm run tauri dev
```

Installer (`src-tauri\target\release\bundle\nsis\OmniStore_0.1.0_x64-setup.exe`):

```powershell
npm run tauri build
```

Browser preview with demo data (no Rust needed): `npm run dev` then open http://localhost:1420

Rust tests: `cd src-tauri` then `cargo test`

## How sign-in works

| Account | What the user does | What OmniStore reads |
|---|---|---|
| Apple | Signs in on icloud.com inside an OmniStore window. Password and 2FA go to Apple only. | Plan, usage by category (Photos, Backups, Docs, Mail, Messages), quota status, Apple devices on the account |
| Google | Signs in on Google's page in the default browser and approves access | Storage quota (total, Drive, trash, Gmail + Photos combined). A hidden app folder holds device snapshots for cross-device sync |

Google blocks sign-in inside embedded webviews, so Google always uses the system browser. Apple has no consent screen for storage data, so OmniStore reads it from the icloud.com session the user created.

## Google client ID (one time)

1. console.cloud.google.com → new project `OmniStore`
2. APIs & Services → Library → enable Google Drive API
3. Google Auth Platform → Branding (app name, email) → Audience: External, add yourself as test user
4. Data Access → add scope `https://www.googleapis.com/auth/drive.appdata` (non-sensitive, no verification)
5. Clients → Create client → Desktop app → paste ID and secret in OmniStore Settings
6. Audience → Publish app. Apps left in Testing lose their sign-in after 7 days. With only a non-sensitive scope, publishing needs no Google review.

To ship without users pasting anything, build with:

```powershell
$env:OMNISTORE_GOOGLE_CLIENT_ID="xxxx.apps.googleusercontent.com"
$env:OMNISTORE_GOOGLE_CLIENT_SECRET="GOCSPX-xxxx"
npm run tauri build
```

## What the Windows build scans

- Volumes (SSD/HDD, external), physical disks via `Get-PhysicalDisk` (media, bus, health)
- iCloud Drive, iCloud Photos, Google Drive for desktop (cache), OneDrive, Dropbox: bytes on disk vs online-only placeholders
- Recycle Bin, Temp, Windows.old, Downloads
- iPhone/iPad backups stored on the PC (Apple Devices app and iTunes paths) with device name, iOS version, last backup date
- Apple devices plugged in over USB
- Largest files in the user folder (on demand)

## Layout

```
src/                     React dashboard (demo data when run outside Tauri)
src-tauri/src/
  lib.rs                 commands, state, refresh pipeline
  local.rs               volumes, hardware, sync clients, reclaimable, iOS backups, USB devices
  scan.rs                placeholder-aware folder sizing, large files
  icloud.rs              icloud.com sign-in window + storage reader
  google.rs              browser sign-in (PKCE loopback), quota, device sync
  plans.rs               plan matching from plans.json
  rules.rs               status, overage, forecast, optimisation findings
  store.rs               settings/dashboard JSON, OS credential store
src-tauri/resources/plans.json   EUR plan catalogue (iCloud verified, Google to verify)
```

Data lives in `%APPDATA%\app.omnistore`. The Google token lives in Windows Credential Manager.

## Next: macOS and iOS

macOS: on the MacBook, same commands. `local.rs` already handles `~/Library/CloudStorage`, iCloud Drive, dataless files, `system_profiler` and local iPhone backups.

iOS/iPadOS (on the Mac with Xcode):

```bash
npm run tauri ios init
npm run tauri ios dev
```

iOS work still to do: Google sign-in through a custom URL scheme (loopback is not allowed on mobile), iCloud sign-in in the single mobile webview, and a Swift plugin for device capacity and Photos library analysis.
