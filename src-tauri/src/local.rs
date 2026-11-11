//! This device: volumes, physical disks, installed sync clients, reclaimable space,
//! iPhone/iPad backups stored here, and Apple devices plugged in over USB.

use crate::model::*;
use crate::scan::{dir_size, DirSize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use sysinfo::{DiskKind, Disks, System};

#[allow(dead_code)]
const SYNC_BUDGET: Duration = Duration::from_secs(45);
const SMALL_BUDGET: Duration = Duration::from_secs(15);

pub fn device_kind() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else if cfg!(target_os = "android") {
        "android"
    } else {
        "linux"
    }
}

pub fn volumes() -> Vec<Volume> {
    let disks = Disks::new_with_refreshed_list();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for d in disks.list() {
        let name = d.name().to_string_lossy().to_string();
        let mount = d.mount_point().display().to_string();
        // Google Drive for desktop mounts a virtual FAT32 drive that mirrors the cloud. Not local storage.
        if name.eq_ignore_ascii_case("Google Drive") {
            continue;
        }
        if d.total_space() == 0 || !seen.insert(mount.clone()) {
            continue;
        }
        out.push(Volume {
            name: if name.is_empty() { mount.clone() } else { name },
            mount,
            file_system: d.file_system().to_string_lossy().to_string(),
            total_bytes: d.total_space(),
            free_bytes: d.available_space(),
            kind: match d.kind() {
                DiskKind::SSD => "ssd".into(),
                DiskKind::HDD => "hdd".into(),
                _ => "unknown".into(),
            },
            removable: d.is_removable(),
        });
    }
    out
}

pub fn this_device(settings: &Settings, vols: &[Volume], model: Option<String>) -> DeviceSnapshot {
    let internal: Vec<&Volume> = vols.iter().filter(|v| !v.removable).collect();
    DeviceSnapshot {
        id: settings.device_id.clone(),
        name: settings.device_name.clone(),
        kind: device_kind().into(),
        model,
        os: System::long_os_version(),
        total_bytes: internal.iter().map(|v| v.total_bytes).sum(),
        free_bytes: internal.iter().map(|v| v.free_bytes).sum(),
        volumes: vols.to_vec(),
        source: "this device".into(),
        updated_at: now(),
    }
}

pub fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn sized(path: &Path, budget: Duration) -> DirSize {
    dir_size(path, budget)
}

/// Full local report. Blocking: call from spawn_blocking.
pub fn scan(settings: &Settings) -> (LocalReport, DeviceSnapshot) {
    let vols = volumes();
    let physical = physical_disks();
    let model = machine_model();
    let report = LocalReport {
        sync_clients: sync_clients(),
        reclaimable: reclaimable(&vols),
        ios_backups: ios_backups(),
        apple_usb_devices: apple_usb_devices(),
        physical_disks: physical,
        volumes: vols.clone(),
        scanned_at: now(),
    };
    let device = this_device(settings, &vols, model);
    (report, device)
}

// ---------------------------------------------------------------- iOS backups

fn backup_roots() -> Vec<PathBuf> {
    let h = home();
    let mut v = vec![];
    if cfg!(windows) {
        v.push(h.join("Apple").join("MobileSync").join("Backup")); // Apple Devices app / Store iTunes
        if let Some(roaming) = dirs::config_dir() {
            v.push(roaming.join("Apple Computer").join("MobileSync").join("Backup")); // classic iTunes
        }
    } else if cfg!(target_os = "macos") {
        v.push(h.join("Library").join("Application Support").join("MobileSync").join("Backup"));
    }
    v
}

pub fn ios_backups() -> Vec<IosBackup> {
    let mut out = vec![];
    for root in backup_roots() {
        let Ok(rd) = std::fs::read_dir(&root) else { continue };
        for entry in rd.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let info = plist::Value::from_file(path.join("Info.plist")).ok();
            let dict = info.as_ref().and_then(|v| v.as_dictionary());
            let s = |k: &str| dict.and_then(|d| d.get(k)).and_then(|v| v.as_string()).map(String::from);
            let last = dict
                .and_then(|d| d.get("Last Backup Date"))
                .and_then(|v| v.as_date())
                .map(|d| {
                    let st: std::time::SystemTime = d.into();
                    chrono::DateTime::<chrono::Local>::from(st).format("%Y-%m-%d").to_string()
                });
            out.push(IosBackup {
                device_name: s("Device Name").or_else(|| s("Display Name")).unwrap_or_else(|| "iOS device".into()),
                product_type: s("Product Type"),
                ios_version: s("Product Version"),
                last_backup: last,
                bytes: sized(&path, SMALL_BUDGET).local_bytes,
                path: path.display().to_string(),
            });
        }
    }
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    out
}

// ---------------------------------------------------------------- reclaimable

fn reclaimable(vols: &[Volume]) -> Vec<Reclaimable> {
    let mut out = vec![];
    let temp = std::env::temp_dir();
    out.push(Reclaimable {
        id: "temp".into(),
        label: "Temporary files".into(),
        bytes: sized(&temp, SMALL_BUDGET).local_bytes,
        path: Some(temp.display().to_string()),
    });

    if cfg!(windows) {
        let mut bin = 0u64;
        for v in vols.iter().filter(|v| !v.removable) {
            let p = PathBuf::from(&v.mount).join("$Recycle.Bin");
            bin += sized(&p, SMALL_BUDGET).local_bytes;
        }
        out.push(Reclaimable { id: "recycle_bin".into(), label: "Recycle Bin".into(), bytes: bin, path: None });

        let old = PathBuf::from("C:\\Windows.old");
        if old.exists() {
            out.push(Reclaimable {
                id: "windows_old".into(),
                label: "Previous Windows install (Windows.old)".into(),
                bytes: sized(&old, SMALL_BUDGET).local_bytes,
                path: Some(old.display().to_string()),
            });
        }
    }
    if cfg!(target_os = "macos") {
        let trash = home().join(".Trash");
        out.push(Reclaimable {
            id: "trash".into(),
            label: "Trash".into(),
            bytes: sized(&trash, SMALL_BUDGET).local_bytes,
            path: Some(trash.display().to_string()),
        });
        let derived = home().join("Library/Developer/Xcode/DerivedData");
        if derived.exists() {
            out.push(Reclaimable {
                id: "xcode_derived".into(),
                label: "Xcode DerivedData".into(),
                bytes: sized(&derived, SMALL_BUDGET).local_bytes,
                path: Some(derived.display().to_string()),
            });
        }
    }
    if let Some(dl) = dirs::download_dir() {
        out.push(Reclaimable {
            id: "downloads".into(),
            label: "Downloads folder".into(),
            bytes: sized(&dl, SMALL_BUDGET).local_bytes,
            path: Some(dl.display().to_string()),
        });
    }
    out
}

// ---------------------------------------------------------------- sync clients

#[allow(dead_code)]
fn client(id: &str, name: &str, installed: bool, root: Option<PathBuf>, cache: Option<PathBuf>) -> SyncClient {
    let size = root.as_ref().map(|r| sized(r, SYNC_BUDGET)).unwrap_or_default();
    let cache_bytes = cache.as_ref().map(|c| sized(c, SMALL_BUDGET).local_bytes).unwrap_or(0);
    SyncClient {
        id: id.into(),
        name: name.into(),
        installed: installed || root.as_ref().map(|r| r.exists()).unwrap_or(false),
        sync_root: root.filter(|r| r.exists()).map(|r| r.display().to_string()),
        local_bytes: size.local_bytes,
        cloud_only_bytes: size.cloud_only_bytes,
        cache_bytes,
        file_count: size.files,
    }
}

#[cfg(windows)]
pub fn sync_clients() -> Vec<SyncClient> {
    let h = home();
    let local = dirs::data_local_dir().unwrap_or_else(|| h.join("AppData").join("Local"));
    let roaming = dirs::config_dir().unwrap_or_else(|| h.join("AppData").join("Roaming"));
    let pf = PathBuf::from(std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into()));
    let pf86 = PathBuf::from(std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".into()));

    let icloud_installed = std::fs::read_dir(local.join("Packages"))
        .map(|rd| rd.filter_map(Result::ok).any(|e| e.file_name().to_string_lossy().starts_with("AppleInc.iCloud")))
        .unwrap_or(false)
        || pf.join("Common Files\\Apple\\Internet Services\\iCloud.exe").exists()
        || pf86.join("Common Files\\Apple\\Internet Services\\iCloud.exe").exists();

    let onedrive_root = std::env::var("OneDriveConsumer").or_else(|_| std::env::var("OneDrive")).ok().map(PathBuf::from);
    let onedrive_installed = local.join("Microsoft\\OneDrive\\OneDrive.exe").exists() || pf.join("Microsoft OneDrive\\OneDrive.exe").exists();

    let gdrive_installed = pf.join("Google\\Drive File Stream").exists();
    let gdrive_mirror = h.join("My Drive");

    let dropbox_root = [local.join("Dropbox\\info.json"), roaming.join("Dropbox\\info.json")]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v["personal"]["path"].as_str().or(v["business"]["path"].as_str()).map(PathBuf::from));

    vec![
        client("icloud_drive", "iCloud Drive", icloud_installed, Some(h.join("iCloudDrive")), None),
        client("icloud_photos", "iCloud Photos", icloud_installed, Some(h.join("Pictures").join("iCloud Photos")), None),
        client(
            "google_drive",
            "Google Drive for desktop",
            gdrive_installed,
            gdrive_mirror.exists().then_some(gdrive_mirror),
            Some(local.join("Google\\DriveFS")),
        ),
        client("onedrive", "OneDrive", onedrive_installed, onedrive_root, None),
        client("dropbox", "Dropbox", false, dropbox_root, None),
    ]
}

#[cfg(target_os = "macos")]
pub fn sync_clients() -> Vec<SyncClient> {
    let h = home();
    let mut v = vec![client(
        "icloud_drive",
        "iCloud Drive",
        true,
        Some(h.join("Library/Mobile Documents/com~apple~CloudDocs")),
        None,
    )];
    // File Provider clients (Google Drive, OneDrive, Dropbox, Box) all live here on modern macOS.
    if let Ok(rd) = std::fs::read_dir(h.join("Library/CloudStorage")) {
        for e in rd.filter_map(Result::ok) {
            let n = e.file_name().to_string_lossy().to_string();
            let (id, name) = if n.starts_with("GoogleDrive") {
                ("google_drive", "Google Drive")
            } else if n.starts_with("OneDrive") {
                ("onedrive", "OneDrive")
            } else if n.starts_with("Dropbox") {
                ("dropbox", "Dropbox")
            } else if n.starts_with("Box") {
                ("box", "Box")
            } else {
                ("other", n.as_str())
            };
            v.push(client(id, name, true, Some(e.path()), None));
        }
    }
    v
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn sync_clients() -> Vec<SyncClient> {
    vec![]
}

// ---------------------------------------------------------------- hardware (PowerShell on Windows)

#[cfg(windows)]
fn powershell(script: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let full = format!("[Console]::OutputEncoding=[Text.Encoding]::UTF8; {script}");
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &full])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(not(windows))]
fn powershell(_script: &str) -> Option<String> {
    None
}

fn json_list(raw: &str) -> Vec<serde_json::Value> {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Array(a)) => a,
        Ok(v @ serde_json::Value::Object(_)) => vec![v],
        _ => vec![],
    }
}

fn enum_or_string(v: &serde_json::Value, table: &[(u64, &str)]) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    let n = v.as_u64().unwrap_or(u64::MAX);
    table.iter().find(|(k, _)| *k == n).map(|(_, s)| s.to_string()).unwrap_or_else(|| "Unknown".into())
}

pub fn physical_disks() -> Vec<PhysicalDisk> {
    if cfg!(target_os = "macos") {
        return mac_physical_disks();
    }
    let Some(raw) = powershell(
        "Get-PhysicalDisk | Select-Object FriendlyName,MediaType,BusType,HealthStatus,Size | ConvertTo-Json -Compress",
    ) else {
        return vec![];
    };
    const MEDIA: &[(u64, &str)] = &[(0, "Unspecified"), (3, "HDD"), (4, "SSD"), (5, "SCM")];
    const BUS: &[(u64, &str)] = &[
        (1, "SCSI"), (2, "ATAPI"), (3, "ATA"), (4, "1394"), (5, "SSA"), (6, "Fibre Channel"), (7, "USB"),
        (8, "RAID"), (9, "iSCSI"), (10, "SAS"), (11, "SATA"), (12, "SD"), (13, "MMC"), (15, "File Backed Virtual"),
        (16, "Storage Spaces"), (17, "NVMe"),
    ];
    const HEALTH: &[(u64, &str)] = &[(0, "Healthy"), (1, "Warning"), (2, "Unhealthy"), (5, "Unknown")];
    json_list(&raw)
        .iter()
        .map(|d| PhysicalDisk {
            name: d["FriendlyName"].as_str().unwrap_or("Disk").trim().to_string(),
            media_type: enum_or_string(&d["MediaType"], MEDIA),
            bus_type: enum_or_string(&d["BusType"], BUS),
            health: enum_or_string(&d["HealthStatus"], HEALTH),
            size_bytes: d["Size"].as_u64().unwrap_or(0),
        })
        .collect()
}

fn mac_physical_disks() -> Vec<PhysicalDisk> {
    let Ok(out) = std::process::Command::new("system_profiler")
        .args(["SPNVMeDataType", "SPSerialATADataType", "SPUSBDataType", "-json"])
        .output()
    else {
        return vec![];
    };
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let mut disks = vec![];
    for (key, bus) in [("SPNVMeDataType", "NVMe"), ("SPSerialATADataType", "SATA")] {
        for ctrl in v[key].as_array().into_iter().flatten() {
            for item in ctrl["_items"].as_array().into_iter().flatten() {
                disks.push(PhysicalDisk {
                    name: item["device_model"].as_str().or(item["_name"].as_str()).unwrap_or("Disk").into(),
                    media_type: if item["spsata_medium_type"].as_str() == Some("Rotational") { "HDD".into() } else { "SSD".into() },
                    bus_type: bus.into(),
                    health: item["smart_status"].as_str().unwrap_or("Unknown").into(),
                    size_bytes: item["size_in_bytes"].as_u64().unwrap_or(0),
                });
            }
        }
    }
    disks
}

pub fn machine_model() -> Option<String> {
    if cfg!(windows) {
        return powershell("$c=Get-CimInstance Win32_ComputerSystem; \"$($c.Manufacturer) $($c.Model)\"")
            .filter(|s| !s.trim().is_empty());
    }
    if cfg!(target_os = "macos") {
        let out = std::process::Command::new("sysctl").args(["-n", "hw.model"]).output().ok()?;
        return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    None
}

/// iPhone / iPad plugged in over USB (Apple vendor id 05AC).
pub fn apple_usb_devices() -> Vec<String> {
    let Some(raw) = powershell(
        "Get-PnpDevice -PresentOnly | Where-Object { $_.InstanceId -like 'USB\\VID_05AC*' -and $_.FriendlyName -match 'iPhone|iPad|iPod' } | Select-Object -ExpandProperty FriendlyName",
    ) else {
        return vec![];
    };
    let mut v: Vec<String> = raw.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    v.sort();
    v.dedup();
    v
}
