import type { Dashboard, Settings } from "./types";

const G = 1e9;
const now = () => new Date().toISOString();

export function demoSettings(): Settings {
  return {
    deviceId: "demo-pc",
    deviceName: "DESKTOP-DEMO",
    icloudConnected: true,
    largeFileMinMb: 500,
  };
}

export function demoDashboard(): Dashboard {
  const history = Array.from({ length: 30 }, (_, i) => {
    const d = new Date(Date.now() - (29 - i) * 864e5);
    return {
      date: d.toISOString().slice(0, 10),
      icloudUsed: (196 + i * 0.55) * G,
      googleUsed: (60 + i * 0.13) * G,
            localUsed: (690 + i * 1.4) * G,
    };
  });
  return {
    updatedAt: now(),
    thisDevice: {
      id: "demo-pc",
      name: "DESKTOP-DEMO",
      kind: "windows",
      model: "Custom desktop",
      os: "Windows 11 Pro 24H2",
      totalBytes: 2000 * G,
      freeBytes: 268 * G,
      volumes: [],
      source: "this device",
      updatedAt: now(),
    },
    icloud: {
      provider: "icloud",
      accountLabel: "you@icloud.com",
      used: 212.4 * G,
      limit: 214.7 * G,
      plan: { id: "icloud-200", name: "iCloud+ 200 GB", bytes: 214.7 * G, monthlyEur: 2.99, priceVerified: true, exact: true },
      status: "critical",
      overBy: 0,
      breakdown: [
        { key: "photos", label: "Photos and Videos", bytes: 148.2 * G, color: "#FF9F0A" },
        { key: "backup", label: "Backups", bytes: 41.7 * G, color: "#BF5AF2" },
        { key: "docs", label: "Documents", bytes: 12.9 * G, color: "#0A84FF" },
        { key: "messages", label: "Messages", bytes: 6.3 * G, color: "#30D158" },
        { key: "mail", label: "Mail", bytes: 3.3 * G, color: "#64D2FF" },
      ],
      trashBytes: 0,
      familyShared: false,
      fetchedAt: now(),
      insights: [
        { key: "photos_types", title: "Photos and videos", totalBytes: 148.2 * G, totalCount: 21480, estimated: false, indexCount: 21480, updatedAt: now(),
          rows: [
            { key: "videos", label: "Videos", bytes: 71.6 * G, count: 1240 },
            { key: "photos", label: "Photos", bytes: 52.3 * G, count: 16210 },
            { key: "live", label: "Live Photo motion", bytes: 9.8 * G, count: 6120 },
            { key: "screenshots", label: "Screenshots", bytes: 4.1 * G, count: 3730 },
            { key: "edits", label: "Edited versions", bytes: 2.9 * G, count: 410 },
            { key: "hidden", label: "Hidden", bytes: 0.4 * G, count: 52 },
            { key: "deleted", label: "Recently Deleted", bytes: 7.1 * G, count: 248 },
          ] },
        { key: "photos_apps", title: "Saved by", totalBytes: 148.2 * G, totalCount: 21480, estimated: false, indexCount: 0, updatedAt: now(),
          rows: [
            { key: "app:Camera", label: "Camera", bytes: 118.4 * G, count: 14920 },
            { key: "app:WhatsApp", label: "WhatsApp", bytes: 14.7 * G, count: 3310 },
            { key: "app:Screenshots", label: "Screenshots", bytes: 4.1 * G, count: 3730 },
            { key: "app:Instagram", label: "Instagram", bytes: 2.2 * G, count: 290 },
            { key: "other", label: "Other apps", bytes: 3.5 * G, count: 610 },
            { key: "unknown", label: "Source not recorded", bytes: 5.3 * G, count: 620 },
          ] },
        { key: "drive", title: "iCloud Drive", totalBytes: 12.9 * G, totalCount: 2140, estimated: false, indexCount: 0, updatedAt: now(),
          rows: [
            { key: "item:Desktop", label: "Desktop", bytes: 4.6 * G, count: 812 },
            { key: "item:Documents", label: "Documents", bytes: 3.9 * G, count: 1104 },
            { key: "item:Downloads", label: "Downloads", bytes: 1.8 * G, count: 96 },
            { key: "item:Pages", label: "Pages", bytes: 0.3 * G, count: 41 },
            { key: "more", label: "Other folders", bytes: 0.6 * G },
            { key: "trash", label: "Recently Deleted", bytes: 0.9 * G, count: 31 },
            { key: "hidden_app_data", label: "App data not shown as files", bytes: 0.8 * G },
          ] },
      ],
    },
    google: {
      provider: "google",
      accountLabel: "you@gmail.com",
      used: 63.8 * G,
      limit: 107.4 * G,
      plan: { id: "google-100", name: "Google One Basic 100 GB", bytes: 107.4 * G, monthlyEur: 1.99, priceVerified: false, exact: false },
      status: "ok",
      overBy: 0,
      breakdown: [
        { key: "drive", label: "Drive", bytes: 21.4 * G, color: "#4285F4" },
        { key: "trash", label: "Drive trash", bytes: 3.1 * G, color: "#9AA0A6" },
        { key: "gmail_photos", label: "Gmail + Google Photos", bytes: 39.3 * G, color: "#EA4335" },
      ],
      trashBytes: 3.1 * G,
      familyShared: false,
      fetchedAt: now(),
      note: "Google reports Gmail and Photos as one combined figure.",
    },
    syncFolder: "C:\\Users\\you\\iCloudDrive\\OmniStore\\Devices",
    devices: [
      { id: "mac", name: "MacBook", kind: "macos", model: "MacBook Pro", os: "macOS 26", totalBytes: 1000 * G, freeBytes: 211 * G, volumes: [], source: "OmniStore sync", updatedAt: now() },
      { id: "iphone", name: "Aris iPhone", kind: "ios", model: "iPhone 15 Pro", os: "iOS 26.1", totalBytes: 128 * G, freeBytes: 6.2 * G, volumes: [], source: "Read over USB / Wi-Fi", connection: "USB", updatedAt: now(),
        breakdown: [
          { key: "apps", label: "Apps and app data", bytes: 41.3 * G, color: "#6c8cff" },
          { key: "photos", label: "Photos", bytes: 38.9 * G, color: "#ffb547" },
          { key: "videos", label: "Videos", bytes: 21.7 * G, color: "#ff5c7a" },
          { key: "screenshots", label: "Screenshots", bytes: 2.4 * G, color: "#b28cff" },
          { key: "system", label: "iOS, system and other", bytes: 17.5 * G, color: "#9aa3bd" },
        ],
        media: { photos: 14212, photosBytes: 38.9 * G, videos: 611, videosBytes: 21.7 * G, screenshots: 2380, screenshotsBytes: 2.4 * G, estimated: false },
        topApps: [
          { name: "WhatsApp", bundleId: "net.whatsapp.WhatsApp", appBytes: 0.3 * G, dataBytes: 9.8 * G },
          { name: "Instagram", bundleId: "com.burbn.instagram", appBytes: 0.5 * G, dataBytes: 3.1 * G },
          { name: "Spotify", bundleId: "com.spotify.client", appBytes: 0.2 * G, dataBytes: 6.4 * G },
          { name: "TikTok", bundleId: "com.zhiliaoapp.musically", appBytes: 0.6 * G, dataBytes: 2.2 * G },
        ],
        cloud: [{ name: "iCloud Backup", detail: "On, last backup 2026-11-10", localBytes: 0, cloudBytes: 0 }],
      },
      { id: "ipad", name: "Aris iPad", kind: "ipados", model: "iPad Air", os: "iPadOS 26.1", totalBytes: 64 * G, freeBytes: 9.8 * G, volumes: [], source: "Read over USB / Wi-Fi", connection: "Wi-Fi", updatedAt: now(),
        breakdown: [
          { key: "apps", label: "Apps and app data", bytes: 24.1 * G, color: "#6c8cff" },
          { key: "photos", label: "Photos", bytes: 9.6 * G, color: "#ffb547" },
          { key: "videos", label: "Videos", bytes: 6.3 * G, color: "#ff5c7a" },
          { key: "screenshots", label: "Screenshots", bytes: 0.8 * G, color: "#b28cff" },
          { key: "system", label: "iOS, system and other", bytes: 13.4 * G, color: "#9aa3bd" },
        ],
        media: { photos: 3120, photosBytes: 9.6 * G, videos: 140, videosBytes: 6.3 * G, screenshots: 870, screenshotsBytes: 0.8 * G, estimated: false },
        cloud: [{ name: "iCloud Backup", detail: "On, last backup 2026-11-09", localBytes: 0, cloudBytes: 0 }],
      },
      { id: "icloud:watch", name: "Apple Watch", kind: "apple", model: "Apple Watch", os: "watchOS 26", totalBytes: 0, freeBytes: 0, volumes: [], source: "iCloud account", updatedAt: now() },
    ],
    local: {
      scannedAt: now(),
      volumes: [
        { name: "Windows", mount: "C:\\", fileSystem: "NTFS", totalBytes: 1000 * G, freeBytes: 92 * G, kind: "ssd", removable: false },
        { name: "Data", mount: "D:\\", fileSystem: "NTFS", totalBytes: 1000 * G, freeBytes: 176 * G, kind: "hdd", removable: false },
        { name: "Backup", mount: "E:\\", fileSystem: "exFAT", totalBytes: 2000 * G, freeBytes: 1430 * G, kind: "hdd", removable: true },
      ],
      physicalDisks: [
        { name: "Samsung SSD 990 PRO 1TB", mediaType: "SSD", busType: "NVMe", health: "Healthy", sizeBytes: 1000 * G },
        { name: "WDC WD10EZEX", mediaType: "HDD", busType: "SATA", health: "Healthy", sizeBytes: 1000 * G },
        { name: "Seagate Expansion", mediaType: "HDD", busType: "USB", health: "Healthy", sizeBytes: 2000 * G },
      ],
      syncClients: [
        { id: "icloud_drive", name: "iCloud Drive", installed: true, syncRoot: "C:\\Users\\you\\iCloudDrive", localBytes: 11.2 * G, cloudOnlyBytes: 1.7 * G, cacheBytes: 0, fileCount: 2140 },
        { id: "icloud_photos", name: "iCloud Photos", installed: true, syncRoot: "C:\\Users\\you\\Pictures\\iCloud Photos", localBytes: 38.5 * G, cloudOnlyBytes: 109.7 * G, cacheBytes: 0, fileCount: 18422 },
        { id: "google_drive", name: "Google Drive for desktop", installed: true, syncRoot: null, localBytes: 0, cloudOnlyBytes: 0, cacheBytes: 6.4 * G, fileCount: 0 },
        { id: "onedrive", name: "OneDrive", installed: true, syncRoot: "C:\\Users\\you\\OneDrive", localBytes: 2.1 * G, cloudOnlyBytes: 0.4 * G, cacheBytes: 0, fileCount: 811 },
        { id: "dropbox", name: "Dropbox", installed: false, syncRoot: null, localBytes: 0, cloudOnlyBytes: 0, cacheBytes: 0, fileCount: 0 },
      ],
      reclaimable: [
        { id: "recycle_bin", label: "Recycle Bin", bytes: 4.8 * G, path: null },
        { id: "temp", label: "Temporary files", bytes: 2.3 * G, path: "C:\\Users\\you\\AppData\\Local\\Temp" },
        { id: "downloads", label: "Downloads folder", bytes: 27.6 * G, path: "C:\\Users\\you\\Downloads" },
      ],
      iosBackups: [
        { deviceName: "iPhone", productType: "iPhone15,2", iosVersion: "17.4", lastBackup: "2024-03-02", path: "C:\\Users\\you\\Apple\\MobileSync\\Backup\\00008110-demo", bytes: 58.3 * G },
      ],
      appleUsbDevices: ["Apple iPhone"],
    },
    largeFiles: [
      { path: "C:\\Users\\you\\Videos\\2025-render-final.mov", bytes: 14.2 * G, modified: "2025-06-11", cloudOnly: false },
      { path: "C:\\Users\\you\\Downloads\\Win11_24H2.iso", bytes: 5.8 * G, modified: "2025-02-20", cloudOnly: false },
      { path: "C:\\Users\\you\\Documents\\VMs\\ubuntu.vhdx", bytes: 21.7 * G, modified: "2026-01-04", cloudOnly: false },
    ],
    findings: [
      { id: "icloud-photos-deleted", severity: "warn", title: "Recently Deleted photos still use 7.1 GB", detail: "248 deleted photos and videos count against iCloud for 30 days. Empty Recently Deleted in Photos to get the space back now.", bytes: 7.1 * G, monthlyEur: 0, action: { label: "Open iCloud Photos", kind: "open_url", target: "https://www.icloud.com/photos/" } },
      { id: "Icloud-near", severity: "critical", title: "iCloud+ is 99% full", detail: "2.3 GB free of 214.7 GB. At the current rate it is full in about 4 days.", bytes: 0, monthlyEur: 0, action: { label: "Manage plan", kind: "open_url", target: "https://www.icloud.com/settings/" } },
      { id: "iosbackup", severity: "warn", title: "iPhone backup on this PC: 58.3 GB", detail: "Last backup 2024-03-02. Older than 6 months, likely safe to delete if iCloud Backup is on.", bytes: 58.3 * G, monthlyEur: 0, action: { label: "Show in folder", kind: "open_path", target: "C:\\" } },
      { id: "vol-C", severity: "warn", title: "C:\\ is 91% full", detail: "92 GB free of 1.00 TB.", bytes: 0, monthlyEur: 0, action: { label: "Storage settings", kind: "open_url", target: "ms-settings:storagesense" } },
      { id: "reclaim-recycle", severity: "warn", title: "Recycle Bin: 4.8 GB", detail: "Safe to clear.", bytes: 4.8 * G, monthlyEur: 0, action: { label: "Empty Recycle Bin", kind: "empty_recycle_bin" } },
      { id: "dev-iphone", severity: "warn", title: "iPhone has only 6.2 GB free", detail: "128 GB total. Turn on Optimize Storage for Photos and offload unused apps.", bytes: 0, monthlyEur: 0 },
      { id: "sync-icloud_photos", severity: "info", title: "iCloud Photos keeps 38.5 GB downloaded on this PC", detail: "Right-click folders you rarely open and choose \"Free up space\" (online-only).", bytes: 38.5 * G, monthlyEur: 0, action: { label: "Open folder", kind: "open_path", target: "C:\\" } },
      { id: "cache-google_drive", severity: "info", title: "Google Drive for desktop cache uses 6.4 GB", detail: "Lower the cache or switch folders to streaming in the app's preferences.", bytes: 6.4 * G, monthlyEur: 0 },
      { id: "large-files", severity: "info", title: "3 large files hold 41.7 GB", detail: "Review them on the This device tab. Move archives to an external drive.", bytes: 41.7 * G, monthlyEur: 0 },
    ],
    totals: {
      cloudUsed: 276.2 * G,
      cloudCapacity: 322.1 * G,
      localUsed: 1732 * G,
      localCapacity: 2000 * G,
      monthlySpendEur: 4.98,
      reclaimableBytes: 66.2 * G,
      potentialSavingsEur: 0,
    },
    history,
  };
}
