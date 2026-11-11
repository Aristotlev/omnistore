mod google;
mod icloud;
mod local;
mod model;
mod plans;
mod rules;
mod scan;
mod store;

use model::*;
use serde::Deserialize;
use store::AppState;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

type Res<T> = Result<T, String>;

/// Recompute findings and totals, record today's history point, persist, notify the UI.
fn commit(app: &AppHandle) -> Dashboard {
    let state = app.state::<AppState>();
    {
        let mut d = state.dashboard.lock().unwrap();
        d.updated_at = Some(local::now());
        rules::recompute(&mut d);
    }
    state.record_history();
    state.save_dashboard();
    let d = state.dashboard.lock().unwrap().clone();
    let _ = app.emit("dashboard://updated", &d);
    d
}

fn merge_devices(d: &mut Dashboard, source: &str, incoming: Vec<DeviceSnapshot>) {
    d.devices.retain(|x| x.source != source);
    d.devices.extend(incoming);
}

/// Called by the iCloud window once icloud.com hands back the storage report.
pub(crate) fn apply_icloud(app: &AppHandle, account: CloudAccount, devices: Vec<DeviceSnapshot>) {
    let state = app.state::<AppState>();
    {
        let mut d = state.dashboard.lock().unwrap();
        d.icloud = Some(account);
        merge_devices(&mut d, "iCloud account", devices);
    }
    state.settings.lock().unwrap().icloud_connected = true;
    state.save_settings();
    commit(app);
}

#[tauri::command]
fn get_dashboard(state: State<'_, AppState>) -> Dashboard {
    state.dashboard.lock().unwrap().clone()
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    masked(&state)
}

fn masked(state: &AppState) -> Settings {
    let mut s = state.settings();
    // never send the secret back to the UI
    if !s.google_client_secret.is_empty() {
        s.google_client_secret = "••••••••".into();
    }
    s
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsInput {
    device_name: Option<String>,
    google_client_id: Option<String>,
    google_client_secret: Option<String>,
    large_file_min_mb: Option<u64>,
}

#[tauri::command]
fn save_settings(state: State<'_, AppState>, input: SettingsInput) -> Settings {
    {
        let mut s = state.settings.lock().unwrap();
        if let Some(v) = input.device_name.filter(|v| !v.trim().is_empty()) {
            s.device_name = v.trim().into();
        }
        if let Some(v) = input.google_client_id {
            s.google_client_id = v.trim().into();
        }
        if let Some(v) = input.google_client_secret.filter(|v| !v.contains('•')) {
            s.google_client_secret = v.trim().into();
        }
        if let Some(v) = input.large_file_min_mb.filter(|v| *v >= 50) {
            s.large_file_min_mb = v;
        }
    }
    state.save_settings();
    masked(&state)
}

#[tauri::command]
async fn scan_local(app: AppHandle) -> Res<Dashboard> {
    let settings = app.state::<AppState>().settings();
    let (report, device) = tauri::async_runtime::spawn_blocking(move || local::scan(&settings))
        .await
        .map_err(|e| e.to_string())?;
    {
        let state = app.state::<AppState>();
        let mut d = state.dashboard.lock().unwrap();
        d.local = Some(report);
        d.this_device = Some(device);
    }
    Ok(commit(&app))
}

#[tauri::command]
async fn deep_scan(app: AppHandle) -> Res<Dashboard> {
    let min = app.state::<AppState>().settings().large_file_min_mb * 1_000_000;
    let _ = app.emit("scan://progress", "Scanning your user folder for large files…");
    let files = tauri::async_runtime::spawn_blocking(move || {
        let home = dirs::home_dir().unwrap_or_default();
        scan::large_files(&home, min, 200, std::time::Duration::from_secs(120))
    })
    .await
    .map_err(|e| e.to_string())?;
    app.state::<AppState>().dashboard.lock().unwrap().large_files = files;
    let _ = app.emit("scan://progress", "");
    Ok(commit(&app))
}

#[tauri::command]
async fn icloud_connect(app: AppHandle) -> Res<()> {
    icloud::open(&app, true)
}

#[tauri::command]
async fn icloud_refresh(app: AppHandle) -> Res<()> {
    icloud::open(&app, false)
}

#[tauri::command]
async fn icloud_disconnect(app: AppHandle) -> Res<Dashboard> {
    icloud::close(&app);
    // icloud.com cookies live in the shared webview profile. The main window only loads
    // bundled assets, so clearing it signs the user out of Apple without side effects.
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.clear_all_browsing_data();
    }
    let state = app.state::<AppState>();
    {
        let mut d = state.dashboard.lock().unwrap();
        d.icloud = None;
        merge_devices(&mut d, "iCloud account", vec![]);
    }
    state.settings.lock().unwrap().icloud_connected = false;
    state.save_settings();
    Ok(commit(&app))
}

async fn google_pull(app: &AppHandle) -> Res<()> {
    let settings = app.state::<AppState>().settings();
    let token = google::access_token(&settings).await?;
    let account = google::fetch_account(&token).await?;
    let this = app.state::<AppState>().dashboard.lock().unwrap().this_device.clone();
    let others = match this {
        Some(t) => google::sync_devices(&token, &t).await.unwrap_or_default(),
        None => vec![],
    };
    let state = app.state::<AppState>();
    {
        let mut d = state.dashboard.lock().unwrap();
        d.google = Some(account);
        merge_devices(&mut d, "OmniStore sync", others);
    }
    Ok(())
}

#[tauri::command]
async fn google_connect(app: AppHandle) -> Res<Dashboard> {
    let settings = app.state::<AppState>().settings();
    let email = google::connect(&app, &settings).await?;
    {
        let state = app.state::<AppState>();
        state.settings.lock().unwrap().google_email = Some(email);
        state.save_settings();
    }
    google_pull(&app).await?;
    Ok(commit(&app))
}

#[tauri::command]
async fn google_refresh(app: AppHandle) -> Res<Dashboard> {
    google_pull(&app).await?;
    Ok(commit(&app))
}

#[tauri::command]
fn google_disconnect(app: AppHandle) -> Dashboard {
    google::disconnect();
    let state = app.state::<AppState>();
    {
        let mut d = state.dashboard.lock().unwrap();
        d.google = None;
        merge_devices(&mut d, "OmniStore sync", vec![]);
    }
    state.settings.lock().unwrap().google_email = None;
    state.save_settings();
    commit(&app)
}

/// Rescan this device, then refresh every connected account. Errors per source are reported, not fatal.
#[tauri::command]
async fn refresh_all(app: AppHandle) -> Res<Vec<String>> {
    let mut errors = vec![];
    if let Err(e) = scan_local(app.clone()).await {
        errors.push(format!("This device: {e}"));
    }
    if google::is_connected() {
        if let Err(e) = google_pull(&app).await {
            errors.push(format!("Google: {e}"));
        }
        commit(&app);
    }
    if app.state::<AppState>().settings().icloud_connected {
        if let Err(e) = icloud::open(&app, false) {
            errors.push(format!("iCloud: {e}"));
        }
    }
    Ok(errors)
}

#[tauri::command]
async fn run_action(app: AppHandle, action: Action) -> Res<()> {
    match action.kind.as_str() {
        "open_url" => {
            let t = action.target.ok_or("missing url")?;
            app.opener().open_url(t, None::<&str>).map_err(|e| e.to_string())
        }
        "open_path" => {
            let t = action.target.ok_or("missing path")?;
            app.opener().open_path(t, None::<&str>).map_err(|e| e.to_string())
        }
        "empty_recycle_bin" => {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                std::process::Command::new("powershell.exe")
                    .args(["-NoProfile", "-NonInteractive", "-Command", "Clear-RecycleBin -Force -ErrorAction SilentlyContinue"])
                    .creation_flags(0x0800_0000)
                    .status()
                    .map_err(|e| e.to_string())?;
            }
            scan_local(app).await.map(|_| ())
        }
        other => Err(format!("unknown action {other}")),
    }
}

#[tauri::command]
fn google_status() -> bool {
    google::is_connected()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = AppState::load(app.handle());
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_dashboard,
            get_settings,
            save_settings,
            scan_local,
            deep_scan,
            icloud_connect,
            icloud_refresh,
            icloud_disconnect,
            google_connect,
            google_refresh,
            google_disconnect,
            google_status,
            refresh_all,
            run_action
        ])
        .run(tauri::generate_context!())
        .expect("error while running OmniStore");
}
