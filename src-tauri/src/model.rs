use serde::{Deserialize, Serialize};

/// Everything the dashboard renders. Cached on disk so the UI opens instantly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Dashboard {
    pub this_device: Option<DeviceSnapshot>,
    pub local: Option<LocalReport>,
    pub icloud: Option<CloudAccount>,
    pub google: Option<CloudAccount>,
    /// Devices reported by other OmniStore installs (synced through the iCloud Drive /
    /// OneDrive / Dropbox folder) plus Apple devices listed on the iCloud account.
    pub devices: Vec<DeviceSnapshot>,
    pub large_files: Vec<FileEntry>,
    pub findings: Vec<Finding>,
    pub totals: Totals,
    pub history: Vec<HistoryPoint>,
    pub updated_at: Option<String>,
    /// Folder used to share device snapshots between installs, if one exists.
    pub sync_folder: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub cloud_used: u64,
    pub cloud_capacity: u64,
    pub local_used: u64,
    pub local_capacity: u64,
    pub monthly_spend_eur: f64,
    pub reclaimable_bytes: u64,
    pub potential_savings_eur: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAccount {
    pub provider: Provider,
    pub account_label: String,
    pub used: u64,
    pub limit: u64,
    pub plan: PlanMatch,
    /// "ok" | "near" | "critical" | "over"
    pub status: String,
    pub over_by: u64,
    pub breakdown: Vec<UsageSlice>,
    pub trash_bytes: u64,
    pub family_shared: bool,
    pub fetched_at: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Icloud,
    Google,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSlice {
    pub key: String,
    pub label: String,
    pub bytes: u64,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanMatch {
    pub id: String,
    pub name: String,
    pub bytes: u64,
    pub monthly_eur: f64,
    pub price_verified: bool,
    /// true when the provider told us the plan, false when inferred from the quota size
    pub exact: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSnapshot {
    pub id: String,
    pub name: String,
    /// windows | macos | ios | ipados | android | apple (seen on iCloud only) | backup (iOS backup on this PC)
    pub kind: String,
    pub model: Option<String>,
    pub os: Option<String>,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub volumes: Vec<Volume>,
    pub source: String,
    pub updated_at: String,
    /// What fills the device: apps, photos, videos, screenshots, backups, cloud files, system.
    #[serde(default)]
    pub breakdown: Vec<UsageSlice>,
    /// Largest apps (iPhone/iPad: app + documents and data).
    #[serde(default)]
    pub top_apps: Vec<AppUsage>,
    #[serde(default)]
    pub media: Option<MediaStats>,
    /// How this device uses the cloud: sync folders, iCloud Backup, etc.
    #[serde(default)]
    pub cloud: Vec<CloudLink>,
    /// USB / Wi-Fi for iPhone and iPad read from a computer.
    #[serde(default)]
    pub connection: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUsage {
    pub name: String,
    pub bundle_id: String,
    pub app_bytes: u64,
    pub data_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaStats {
    pub photos: u64,
    pub photos_bytes: u64,
    pub videos: u64,
    pub videos_bytes: u64,
    pub screenshots: u64,
    pub screenshots_bytes: u64,
    /// true when the library was too large to measure every file and sizes are extrapolated
    pub estimated: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudLink {
    /// iCloud Drive, iCloud Photos, iCloud Backup, OneDrive, Google Drive, Dropbox
    pub name: String,
    pub detail: String,
    /// stored on this device
    pub local_bytes: u64,
    /// in the cloud only (not downloaded)
    pub cloud_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub name: String,
    pub mount: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    /// ssd | hdd | unknown
    pub kind: String,
    pub removable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalReport {
    pub volumes: Vec<Volume>,
    pub physical_disks: Vec<PhysicalDisk>,
    pub sync_clients: Vec<SyncClient>,
    pub reclaimable: Vec<Reclaimable>,
    pub ios_backups: Vec<IosBackup>,
    pub apple_usb_devices: Vec<String>,
    pub scanned_at: String,
    /// ok | no_service (Apple Devices / iTunes not installed) | none (no iPhone/iPad found)
    #[serde(default)]
    pub ios_status: String,
    #[serde(default)]
    pub ios_errors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalDisk {
    pub name: String,
    pub media_type: String,
    pub bus_type: String,
    pub health: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncClient {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub sync_root: Option<String>,
    /// bytes of synced files physically on this disk
    pub local_bytes: u64,
    /// bytes of files that are cloud-only placeholders
    pub cloud_only_bytes: u64,
    /// app cache outside the sync root (e.g. Google DriveFS cache)
    pub cache_bytes: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reclaimable {
    pub id: String,
    pub label: String,
    pub path: Option<String>,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IosBackup {
    pub device_name: String,
    pub product_type: Option<String>,
    pub ios_version: Option<String>,
    pub last_backup: Option<String>,
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub bytes: u64,
    pub modified: Option<String>,
    pub cloud_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    /// critical | warn | info
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub bytes: u64,
    pub monthly_eur: f64,
    pub action: Option<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub label: String,
    /// open_url | open_path | empty_recycle_bin | icloud_signin
    pub kind: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub date: String,
    pub icloud_used: Option<u64>,
    pub google_used: Option<u64>,
    pub local_used: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub device_id: String,
    pub device_name: String,
    pub icloud_connected: bool,
    pub google_email: Option<String>,
    pub large_file_min_mb: u64,
}
