use crate::model::{Dashboard, HistoryPoint, Settings};
use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub dir: PathBuf,
    pub settings: Mutex<Settings>,
    pub dashboard: Mutex<Dashboard>,
}

impl AppState {
    pub fn load(app: &AppHandle) -> Self {
        let dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("omnistore"));
        let _ = std::fs::create_dir_all(&dir);

        let mut settings: Settings = read_json(&dir.join("settings.json")).unwrap_or_default();
        if settings.device_id.is_empty() {
            settings.device_id = uuid::Uuid::new_v4().to_string();
        }
        if settings.device_name.is_empty() {
            settings.device_name = sysinfo::System::host_name().unwrap_or_else(|| "This device".into());
        }
        if settings.large_file_min_mb == 0 {
            settings.large_file_min_mb = 500;
        }
        let dashboard: Dashboard = read_json(&dir.join("dashboard.json")).unwrap_or_default();
        let state = Self { dir, settings: Mutex::new(settings), dashboard: Mutex::new(dashboard) };
        state.save_settings();
        state
    }

    pub fn save_settings(&self) {
        let s = self.settings.lock().unwrap().clone();
        write_json(&self.dir.join("settings.json"), &s);
    }

    pub fn save_dashboard(&self) {
        let d = self.dashboard.lock().unwrap().clone();
        write_json(&self.dir.join("dashboard.json"), &d);
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// One history point per day, newest value wins. Keeps 400 days.
    pub fn record_history(&self) {
        let mut d = self.dashboard.lock().unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let point = HistoryPoint {
            date: today.clone(),
            icloud_used: d.icloud.as_ref().map(|a| a.used),
            google_used: d.google.as_ref().map(|a| a.used),
            local_used: d.local.as_ref().map(|l| l.volumes.iter().map(|v| v.total_bytes.saturating_sub(v.free_bytes)).sum()),
        };
        if let Some(last) = d.history.last_mut() {
            if last.date == today {
                *last = point;
                return;
            }
        }
        d.history.push(point);
        let len = d.history.len();
        if len > 400 {
            d.history.drain(0..len - 400);
        }
    }
}

pub fn read_json<T: DeserializeOwned>(path: &PathBuf) -> Option<T> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn write_json<T: Serialize>(path: &PathBuf, value: &T) {
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, raw).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

/// The Google refresh token lives in the OS credential store:
/// Windows Credential Manager, macOS/iOS Keychain.
pub mod secrets {
    const SERVICE: &str = "app.omnistore";

    pub fn set(key: &str, value: &str) -> Result<(), String> {
        keyring::Entry::new(SERVICE, key)
            .and_then(|e| e.set_password(value))
            .map_err(|e| e.to_string())
    }

    pub fn get(key: &str) -> Option<String> {
        keyring::Entry::new(SERVICE, key).ok()?.get_password().ok()
    }

    pub fn delete(key: &str) {
        if let Ok(e) = keyring::Entry::new(SERVICE, key) {
            let _ = e.delete_credential();
        }
    }
}
