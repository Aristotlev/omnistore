//! iPhone and iPad storage, read from this computer over USB or Wi-Fi through Apple's own
//! device service (installed with Apple Devices / iTunes on Windows, built into macOS).
//! Same channel Finder and Apple Devices use: exact capacity and free space, every app's size,
//! and the photos, videos and screenshots stored on the device.

use crate::model::*;
use idevice::afc::AfcClient;
use idevice::installation_proxy::InstallationProxyClient;
use idevice::lockdown::LockdownClient;
use idevice::provider::{IdeviceProvider, UsbmuxdProvider};
use idevice::usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdConnection, UsbmuxdDevice};
use idevice::IdeviceService;
use plist::Value;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

pub struct IosScan {
    pub devices: Vec<DeviceSnapshot>,
    /// ok | no_service | none
    pub status: String,
    pub errors: Vec<String>,
}

pub async fn scan() -> IosScan {
    let mut mux = match UsbmuxdConnection::default().await {
        Ok(m) => m,
        Err(_) => return IosScan { devices: vec![], status: "no_service".into(), errors: vec![] },
    };
    let listed = mux.get_devices().await.unwrap_or_default();
    // A device can show up twice (USB and Wi-Fi). Prefer USB.
    let mut by_udid: HashMap<String, UsbmuxdDevice> = HashMap::new();
    for d in listed {
        let usb = d.connection_type == Connection::Usb;
        match by_udid.get(&d.udid) {
            Some(prev) if prev.connection_type == Connection::Usb || !usb => {}
            _ => {
                by_udid.insert(d.udid.clone(), d);
            }
        }
    }
    if by_udid.is_empty() {
        return IosScan { devices: vec![], status: "none".into(), errors: vec![] };
    }
    let mut devices = vec![];
    let mut errors = vec![];
    for dev in by_udid.values() {
        match tokio::time::timeout(Duration::from_secs(150), read_device(dev)).await {
            Ok(Ok(s)) => devices.push(s),
            Ok(Err(e)) => errors.push(e),
            Err(_) => errors.push("An iPhone or iPad took too long to answer. Keep it unlocked and try again.".into()),
        }
    }
    IosScan { devices, status: "ok".into(), errors }
}

fn s(v: &Option<Value>) -> Option<String> {
    v.as_ref().and_then(|x| x.as_string()).map(String::from)
}

fn num(d: &plist::Dictionary, k: &str) -> u64 {
    d.get(k)
        .and_then(|v| v.as_unsigned_integer().or_else(|| v.as_signed_integer().map(|i| i.max(0) as u64)))
        .unwrap_or(0)
}

async fn pairing(provider: &UsbmuxdProvider, udid: &str) -> Result<idevice::pairing_file::PairingFile, String> {
    if let Ok(p) = provider.get_pairing_file().await {
        return Ok(p);
    }
    // Older Apple device services on Windows keep pair records only on disk.
    let path = std::path::PathBuf::from(std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into()))
        .join("Apple")
        .join("Lockdown")
        .join(format!("{udid}.plist"));
    idevice::pairing_file::PairingFile::read_from_file(&path)
        .map_err(|_| "This iPhone or iPad has not trusted this computer yet. Unlock it, plug it in and tap Trust.".to_string())
}

async fn read_device(dev: &UsbmuxdDevice) -> Result<DeviceSnapshot, String> {
    let provider = dev.to_provider(UsbmuxdAddr::default(), "omnistore");
    let pair = pairing(&provider, &dev.udid).await?;
    let mut lockdown = LockdownClient::connect(&provider).await.map_err(|e| format!("Could not reach the device: {e}"))?;
    lockdown
        .start_session(&pair)
        .await
        .map_err(|_| "Unlock your iPhone or iPad and tap Trust, then refresh.".to_string())?;

    let name = s(&lockdown.get_value(Some("DeviceName"), None).await.ok()).unwrap_or_else(|| "iPhone".into());
    let product = s(&lockdown.get_value(Some("ProductType"), None).await.ok());
    let version = s(&lockdown.get_value(Some("ProductVersion"), None).await.ok());
    let class = s(&lockdown.get_value(Some("DeviceClass"), None).await.ok()).unwrap_or_default();
    let disk = lockdown
        .get_value(None, Some("com.apple.disk_usage"))
        .await
        .ok()
        .and_then(|v| v.into_dictionary())
        .unwrap_or_default();
    let backup = lockdown
        .get_value(None, Some("com.apple.mobile.backup"))
        .await
        .ok()
        .and_then(|v| v.into_dictionary())
        .unwrap_or_default();

    let total = num(&disk, "TotalDiskCapacity").max(num(&disk, "TotalDataCapacity"));
    let free = {
        let a = num(&disk, "AmountDataAvailable");
        if a > 0 { a } else { num(&disk, "TotalDataAvailable") }
    };
    let used = total.saturating_sub(free);

    let top_apps = apps(&provider).await.unwrap_or_default();
    let media = media(&provider).await.ok();

    let apps_total: u64 = top_apps.iter().map(|a| a.app_bytes + a.data_bytes).sum();
    let mut breakdown = vec![];
    if apps_total > 0 {
        breakdown.push(UsageSlice { key: "apps".into(), label: "Apps and app data".into(), bytes: apps_total, color: Some("#6c8cff".into()) });
    }
    if let Some(m) = &media {
        breakdown.push(UsageSlice { key: "photos".into(), label: "Photos".into(), bytes: m.photos_bytes, color: Some("#ffb547".into()) });
        breakdown.push(UsageSlice { key: "videos".into(), label: "Videos".into(), bytes: m.videos_bytes, color: Some("#ff5c7a".into()) });
        breakdown.push(UsageSlice { key: "screenshots".into(), label: "Screenshots".into(), bytes: m.screenshots_bytes, color: Some("#b28cff".into()) });
    }
    let known: u64 = breakdown.iter().map(|b| b.bytes).sum();
    breakdown.push(UsageSlice {
        key: "system".into(),
        label: "iOS, system and other".into(),
        bytes: used.saturating_sub(known),
        color: Some("#9aa3bd".into()),
    });
    breakdown.retain(|b| b.bytes > 0);

    let mut cloud = vec![];
    let cloud_on = backup.get("CloudBackupEnabled").and_then(|v| v.as_boolean());
    let last_cloud = backup.get("LastCloudBackupDate").and_then(|v| v.as_date()).map(fmt_date);
    if cloud_on.is_some() || last_cloud.is_some() {
        cloud.push(CloudLink {
            name: "iCloud Backup".into(),
            detail: match (cloud_on, &last_cloud) {
                (Some(false), _) => "Off".into(),
                (_, Some(d)) => format!("On, last backup {d}"),
                _ => "On".into(),
            },
            local_bytes: 0,
            cloud_bytes: 0,
        });
    }
    if let Some(d) = backup.get("LastiTunesBackupDate").and_then(|v| v.as_date()).map(fmt_date) {
        cloud.push(CloudLink { name: "Computer backup".into(), detail: format!("Last backup {d}"), local_bytes: 0, cloud_bytes: 0 });
    }

    let kind = if class.eq_ignore_ascii_case("iPad") { "ipados" } else { "ios" };
    Ok(DeviceSnapshot {
        id: format!("ios-{}", dev.udid),
        name,
        kind: kind.into(),
        model: product.map(|p| model_name(&p)),
        os: version.map(|v| format!("{} {v}", if kind == "ipados" { "iPadOS" } else { "iOS" })),
        total_bytes: total,
        free_bytes: free,
        volumes: vec![],
        source: "Read over USB / Wi-Fi".into(),
        updated_at: crate::local::now(),
        breakdown,
        top_apps,
        media,
        cloud,
        connection: Some(if dev.connection_type == Connection::Usb { "USB".into() } else { "Wi-Fi".into() }),
    })
}

fn fmt_date(d: plist::Date) -> String {
    let st: std::time::SystemTime = d.into();
    chrono::DateTime::<chrono::Local>::from(st).format("%Y-%m-%d").to_string()
}

async fn apps(provider: &UsbmuxdProvider) -> Result<Vec<AppUsage>, String> {
    let mut ip = InstallationProxyClient::connect(provider).await.map_err(|e| e.to_string())?;
    let mut opts = plist::Dictionary::new();
    opts.insert("ApplicationType".into(), Value::String("User".into()));
    opts.insert(
        "ReturnAttributes".into(),
        Value::Array(
            ["CFBundleIdentifier", "CFBundleDisplayName", "CFBundleName", "StaticDiskUsage", "DynamicDiskUsage"]
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        ),
    );
    let list = ip.browse(Some(Value::Dictionary(opts))).await.map_err(|e| e.to_string())?;
    let mut out: Vec<AppUsage> = list
        .into_iter()
        .filter_map(|v| v.into_dictionary())
        .map(|d| {
            let get = |k: &str| d.get(k).and_then(|v| v.as_string()).map(String::from);
            AppUsage {
                name: get("CFBundleDisplayName").or_else(|| get("CFBundleName")).unwrap_or_else(|| get("CFBundleIdentifier").unwrap_or_default()),
                bundle_id: get("CFBundleIdentifier").unwrap_or_default(),
                app_bytes: num(&d, "StaticDiskUsage"),
                data_bytes: num(&d, "DynamicDiskUsage"),
            }
        })
        .collect();
    out.sort_by(|a, b| (b.app_bytes + b.data_bytes).cmp(&(a.app_bytes + a.data_bytes)));
    Ok(out)
}

/// Walks /DCIM on the device's media partition. Screenshots are PNGs, Live Photo movies
/// (same name as a photo) count as photos.
async fn media(provider: &UsbmuxdProvider) -> Result<MediaStats, String> {
    let mut afc = AfcClient::connect(provider).await.map_err(|e| e.to_string())?;
    let started = Instant::now();
    let budget = Duration::from_secs(90);

    let mut files: Vec<String> = vec![];
    let folders = afc.list_dir("/DCIM").await.map_err(|e| e.to_string())?;
    for f in folders.into_iter().filter(|f| f != "." && f != "..") {
        let dir = format!("/DCIM/{f}");
        if let Ok(list) = afc.list_dir(&dir).await {
            files.extend(list.into_iter().filter(|x| x != "." && x != "..").map(|x| format!("{dir}/{x}")));
        }
    }

    let stems: HashSet<String> = files
        .iter()
        .filter(|p| matches!(ext(p).as_str(), "heic" | "jpg" | "jpeg" | "dng" | "png"))
        .map(|p| stem(p))
        .collect();

    let mut m = MediaStats::default();
    let mut sized = 0usize;
    let mut sampled = [0u64; 3]; // bytes measured per class
    let mut counted = [0u64; 3]; // files measured per class
    for p in &files {
        let class = match ext(p).as_str() {
            "png" => 2,
            "heic" | "jpg" | "jpeg" | "dng" | "gif" | "webp" => 0,
            "mov" | "mp4" | "m4v" => {
                if stems.contains(&stem(p)) { 0 } else { 1 }
            }
            _ => continue,
        };
        match class {
            0 => m.photos += 1,
            1 => m.videos += 1,
            _ => m.screenshots += 1,
        }
        if started.elapsed() < budget {
            if let Ok(info) = afc.get_file_info(p).await {
                sampled[class] += info.size as u64;
                counted[class] += 1;
                sized += 1;
            }
        }
    }
    let totals = [m.photos, m.videos, m.screenshots];
    let mut bytes = [0u64; 3];
    for i in 0..3 {
        bytes[i] = if counted[i] == 0 { 0 } else { sampled[i] / counted[i] * totals[i] };
        if counted[i] == totals[i] {
            bytes[i] = sampled[i];
        }
    }
    m.photos_bytes = bytes[0];
    m.videos_bytes = bytes[1];
    m.screenshots_bytes = bytes[2];
    m.estimated = sized < (m.photos + m.videos + m.screenshots) as usize;
    Ok(m)
}

fn ext(p: &str) -> String {
    p.rsplit('.').next().unwrap_or("").to_ascii_lowercase()
}

fn stem(p: &str) -> String {
    let file = p.rsplit('/').next().unwrap_or(p);
    file.rsplit_once('.').map(|(a, _)| a).unwrap_or(file).to_ascii_uppercase()
}

/// Friendly names for common models. Unknown identifiers are shown as-is.
fn model_name(id: &str) -> String {
    let n = match id {
        "iPhone12,1" => "iPhone 11",
        "iPhone12,3" => "iPhone 11 Pro",
        "iPhone12,5" => "iPhone 11 Pro Max",
        "iPhone13,2" => "iPhone 12",
        "iPhone13,3" => "iPhone 12 Pro",
        "iPhone13,4" => "iPhone 12 Pro Max",
        "iPhone14,5" => "iPhone 13",
        "iPhone14,2" => "iPhone 13 Pro",
        "iPhone14,3" => "iPhone 13 Pro Max",
        "iPhone14,7" => "iPhone 14",
        "iPhone14,8" => "iPhone 14 Plus",
        "iPhone15,2" => "iPhone 14 Pro",
        "iPhone15,3" => "iPhone 14 Pro Max",
        "iPhone15,4" => "iPhone 15",
        "iPhone15,5" => "iPhone 15 Plus",
        "iPhone16,1" => "iPhone 15 Pro",
        "iPhone16,2" => "iPhone 15 Pro Max",
        "iPhone17,3" => "iPhone 16",
        "iPhone17,4" => "iPhone 16 Plus",
        "iPhone17,1" => "iPhone 16 Pro",
        "iPhone17,2" => "iPhone 16 Pro Max",
        "iPhone17,5" => "iPhone 16e",
        _ => "",
    };
    if n.is_empty() { id.to_string() } else { n.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helpers() {
        assert_eq!(ext("/DCIM/100APPLE/IMG_0001.HEIC"), "heic");
        assert_eq!(stem("/DCIM/100APPLE/IMG_0001.MOV"), "IMG_0001");
        assert_eq!(model_name("iPhone16,1"), "iPhone 15 Pro");
        assert_eq!(model_name("iPad13,1"), "iPad13,1");
    }
}
