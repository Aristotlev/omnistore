mod google;
mod icloud;
mod ios;
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
        dedupe_devices(&mut d);
        rules::recompute(&mut d);
    }
    state.record_history();
    state.save_dashboard();
    let d = state.dashboard.lock().unwrap().clone();
    let _ = app.emit("dashboard://updated", &d);
    d
}

/// The same device can arrive through Google sync and the iCloud Drive / OneDrive folder.
/// Keep one entry per device id (the newest), and never list this device twice.
fn dedupe_devices(d: &mut Dashboard) {
    let me = d.this_device.as_ref().map(|t| t.id.clone());
    let mut best: std::collections::HashMap<String, DeviceSnapshot> = std::collections::HashMap::new();
    for dev in d.devices.drain(..) {
        if Some(&dev.id) == me.as_ref() {
            continue;
        }
        match best.get(&dev.id) {
            Some(old) if old.updated_at >= dev.updated_at => {}
            _ => {
                best.insert(dev.id.clone(), dev);
            }
        }
    }
    let mut list: Vec<DeviceSnapshot> = best.into_values().collect();
    list.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes).then(a.name.cmp(&b.name)));
    d.devices = list;
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
    state.settings()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsInput {
    device_name: Option<String>,
    large_file_min_mb: Option<u64>,
}

#[tauri::command]
fn save_settings(state: State<'_, AppState>, input: SettingsInput) -> Settings {
    {
        let mut s = state.settings.lock().unwrap();
        if let Some(v) = input.device_name.filter(|v| !v.trim().is_empty()) {
            s.device_name = v.trim().into();
        }
        if let Some(v) = input.large_file_min_mb.filter(|v| *v >= 50) {
            s.large_file_min_mb = v;
        }
    }
    state.save_settings();
    state.settings()
}

#[tauri::command]
async fn scan_local(app: AppHandle) -> Res<Dashboard> {
    let settings = app.state::<AppState>().settings();
    let (mut report, device) = tauri::async_runtime::spawn_blocking(move || local::scan(&settings))
        .await
        .map_err(|e| e.to_string())?;

    // iPhone / iPad plugged in or on the same Wi-Fi: read them from here, no app needed on them.
    let phones = ios::scan().await;
    report.ios_status = phones.status.clone();
    report.ios_errors = phones.errors.clone();

    let publish: Vec<DeviceSnapshot> = std::iter::once(device.clone()).chain(phones.devices.iter().cloned()).collect();
    let sync = tauri::async_runtime::spawn_blocking(move || local::sync_devices(&publish))
        .await
        .map_err(|e| e.to_string())?;
    {
        let state = app.state::<AppState>();
        let mut d = state.dashboard.lock().unwrap();
        d.local = Some(report);
        d.this_device = Some(device);
        d.sync_folder = sync.0;
        merge_devices(&mut d, "OmniStore sync", sync.1);
        // Keep the last reading of phones that are not connected right now.
        d.devices.retain(|x| !phones.devices.iter().any(|p| p.id == x.id));
        d.devices.extend(phones.devices);
    }
    Ok(commit(&app))
}

#[tauri::command]
async fn deep_scan(app: AppHandle) -> Res<Dashboard> {
    let min = app.state::<AppState>().settings().large_file_min_mb * 1_000_000;
    let files = tauri::async_runtime::spawn_blocking(move || {
        let home = dirs::home_dir().unwrap_or_default();
        scan::large_files(&home, min, 200, std::time::Duration::from_secs(120))
    })
    .await
    .map_err(|e| e.to_string())?;
    app.state::<AppState>().dashboard.lock().unwrap().large_files = files;
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
    let token = google::access_token().await?;
    let account = google::fetch_account(&token).await?;
    // This computer plus every iPhone/iPad it has read.
    let state = app.state::<AppState>();
    let publish: Vec<DeviceSnapshot> = {
        let d = state.dashboard.lock().unwrap();
        d.this_device
            .iter()
            .cloned()
            .chain(d.devices.iter().filter(|x| x.source == "Read over USB / Wi-Fi").cloned())
            .collect()
    };
    let others = if publish.is_empty() { vec![] } else { google::sync_devices(&token, &publish).await.unwrap_or_default() };
    let mut d = state.dashboard.lock().unwrap();
    d.google = Some(account);
    merge_devices(&mut d, "Google sync", others);
    Ok(())
}

#[tauri::command]
fn google_available() -> bool {
    google::available()
}

#[tauri::command]
async fn google_connect(app: AppHandle) -> Res<Dashboard> {
    let email = google::connect(&app).await?;
    {
        let state = app.state::<AppState>();
        state.settings.lock().unwrap().google_email = Some(email);
        state.save_settings();
    }
    google_pull(&app).await?;
    let _ = app.emit("toast", "Google connected.");
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
        merge_devices(&mut d, "Google sync", vec![]);
    }
    state.settings.lock().unwrap().google_email = None;
    state.save_settings();
    commit(&app)
}

/// Rescan this device, then refresh every signed-in account. Errors are reported, not fatal.
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

/// Empties the Recycle Bin, then re-reads only its size so the UI updates instantly.
#[tauri::command]
async fn empty_recycle_bin(app: AppHandle) -> Res<Dashboard> {
    let freed = tauri::async_runtime::spawn_blocking(|| {
        let before = local::recycle_bin_bytes();
        local::empty_recycle_bin().map(|_| before.saturating_sub(local::recycle_bin_bytes()))
    })
    .await
    .map_err(|e| e.to_string())??;
    {
        let state = app.state::<AppState>();
        let mut d = state.dashboard.lock().unwrap();
        if let Some(l) = d.local.as_mut() {
            let now = local::recycle_bin_bytes();
            for r in l.reclaimable.iter_mut().filter(|r| r.id == "recycle_bin" || r.id == "trash") {
                r.bytes = now;
            }
        }
        // Free space on the drives changed too.
        let vols = local::volumes();
        if let Some(l) = d.local.as_mut() {
            l.volumes = vols.clone();
        }
        if let Some(t) = d.this_device.as_mut() {
            let internal: Vec<&Volume> = vols.iter().filter(|v| !v.removable).collect();
            t.free_bytes = internal.iter().map(|v| v.free_bytes).sum();
            t.volumes = vols.clone();
        }
    }
    let _ = app.emit("toast", format!("Recycle Bin emptied. {} freed.", rules::fmt_bytes(freed)));
    Ok(commit(&app))
}

#[tauri::command]
async fn run_action(app: AppHandle, action: Action) -> Res<Option<Dashboard>> {
    match action.kind.as_str() {
        "open_url" => {
            let t = action.target.ok_or("missing url")?;
            app.opener().open_url(t, None::<&str>).map_err(|e| e.to_string())?;
            Ok(None)
        }
        "open_path" => {
            let t = action.target.ok_or("missing path")?;
            app.opener().open_path(t, None::<&str>).map_err(|e| e.to_string())?;
            Ok(None)
        }
        "empty_recycle_bin" => empty_recycle_bin(app).await.map(Some),
        "icloud_signin" => icloud::open(&app, true).map(|_| None),
        other => Err(format!("unknown action {other}")),
    }
}

#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build());
    builder
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
            google_available,
            google_connect,
            google_refresh,
            google_disconnect,
            refresh_all,
            empty_recycle_bin,
            run_action,
            app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running OmniStore");
}
