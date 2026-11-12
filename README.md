# OmniStore

**All your storage in one place.** See your iCloud and Google plans, how full they are, what fills them, and what's on every device you own: PC, Mac, iPhone and iPad. Then clean up in a click.

<p>
  <a href="https://github.com/Aristotlev/omnistore/releases/latest/download/OmniStore-Windows.exe"><b>⬇ Download for Windows</b></a> &nbsp;·&nbsp;
  <a href="https://github.com/Aristotlev/omnistore/releases/latest/download/OmniStore-Mac.dmg"><b>⬇ Download for Mac</b></a>
</p>

Windows 10 / 11 and macOS 11 or later (Apple silicon and Intel). Free. Once installed, OmniStore updates itself.

## What you get

- **Your cloud plans at a glance.** Which iCloud+ and Google One plan you're on, what it costs, how much is free, and how far over you are.
- **What fills your iCloud.** Photos, videos, screenshots, Live Photos and edits, with counts and sizes. Which apps saved them (Camera, WhatsApp, Instagram…). Your iCloud Drive folders, Recently Deleted, and backups.
- **Every device.** Each PC, Mac, iPhone and iPad on your account, with its storage and what it keeps in the cloud. You don't install anything on your iPhone or iPad.
- **This computer.** Drives and disks, cloud app folders (iCloud, Google Drive, OneDrive, Dropbox), large files, old iPhone backups, the Recycle Bin.
- **What to fix first.** Clear recommendations, such as "Recently Deleted photos still use 7 GB" or "You could drop to a cheaper plan", each with a button that does it or takes you there.

## Sign in

| | What you do | What OmniStore sees |
|---|---|---|
| **Apple** | Click *Sign in to iCloud* and sign in on Apple's own page: scan the passkey QR code with your iPhone, or approve the sign-in on your iPhone. | Your plan, storage by category, your photo library and iCloud Drive sizes, and the devices on your Apple Account. |
| **Google** | Click *Continue with Google*, pick your account in your browser and click Continue. | Your Google One plan and how it splits between Drive, Gmail and Photos. |

You only do this once. OmniStore never sees your Apple password, and it can't open, change or delete your files, mail or photos.

## First launch

OmniStore is new and not yet signed with a paid Apple or Microsoft certificate, so your computer asks once before opening it:

- **Windows:** if you see "Windows protected your PC", click **More info**, then **Run anyway**.
- **Mac:** open the .dmg and drag OmniStore to Applications. The first time you open it, macOS may say it can't verify the developer. Go to **System Settings → Privacy & Security**, scroll down and click **Open Anyway**.

## Privacy

Everything stays on your computer. OmniStore has no accounts, no servers and no analytics. Your Google sign-in is kept in your system's password store. If you install OmniStore on more than one computer, each writes a small summary (device name, model, disk sizes) to your own iCloud Drive, OneDrive or Google account so your other devices can show it.

[Privacy policy](https://aristotlev.github.io/omnistore/privacy.html) · [Terms](https://aristotlev.github.io/omnistore/terms.html) · [Report a problem](https://github.com/Aristotlev/omnistore/issues)

## FAQ

**Do I need to install OmniStore on my iPhone or iPad?**
No. What your iPhone and iPad store in iCloud is read from your Apple Account. Apple doesn't share an iPhone's own free space or per-app sizes online, so to see those, connect the iPhone to your computer once (on Windows this needs Apple's free *Apple Devices* app). After that, it can be read over Wi-Fi.

**Is there an iPhone app?**
Not yet. iPhone and iPad apps can only be installed from the App Store.

**How do updates work?**
OmniStore checks for a new version when it opens and every few hours, and installs it in one click.

---

<details>
<summary><b>For developers</b></summary>

Built with Tauri 2 (Rust) and React + TypeScript.

```powershell
npm install
npm run tauri dev        # run the app
npm run dev              # browser preview with demo data at http://localhost:1420
cd src-tauri; cargo test # Rust tests
```

**Release.** Double-click `Publish OmniStore.cmd`. It bumps the version, tags and pushes; GitHub Actions (`ci/release.yml`, copied to `.github/workflows`) builds the Windows installer and the universal Mac .dmg, publishes the release with `latest.json` for the updater, and attaches the fixed-name downloads `OmniStore-Windows.exe` and `OmniStore-Mac.dmg`. `release-keys\` holds the update signing key and Google client (git-ignored). Back it up: without it, installed copies can't verify new updates.

**Layout**

```
src/                      React dashboard (demo data outside Tauri)
src-tauri/src/
  lib.rs                  commands, refresh pipeline, updater
  icloud.rs               icloud.com sign-in window; storage, Photos and iCloud Drive reader
  google.rs               Google sign-in (PKCE, system browser) and quota
  ios.rs                  iPhone / iPad over USB or Wi-Fi via Apple's device service
  local.rs, scan.rs       drives, disks, sync apps, Recycle Bin, backups, large files, device sync
  plans.rs, rules.rs      plan matching, overage, forecast, recommendations
  store.rs                settings and dashboard cache
src-tauri/resources/plans.json   plan catalogue (EUR)
docs/                     website: https://aristotlev.github.io/omnistore/
```

</details>
