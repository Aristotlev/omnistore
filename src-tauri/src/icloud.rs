//! iCloud: the user signs in on Apple's own icloud.com page inside an OmniStore window.
//! OmniStore never sees the password. Once the session exists, a small script running
//! on icloud.com reads the same storage data the icloud.com Settings page shows and
//! hands it back by navigating to a fake host that the app intercepts and cancels.

use crate::model::*;
use crate::plans;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW: &str = "icloud";
const REPORT_HOST: &str = "omnistore.invalid";

const SCRIPT: &str = r#"
(function () {
  if (window.__omnistore) return;
  window.__omnistore = true;
  if (!/(^|\.)icloud\.com$/.test(location.hostname)) return;
  var Q = 'clientBuildNumber=2534Project66&clientMasteringNumber=2534B22&clientId=__CLIENT_ID__';
  var SETUP = 'https://setup.icloud.com/setup/ws/1';
  var askedLogin = false, done = false;
  function send(obj) {
    window.location.assign('https://__HOST__/report?d=' + encodeURIComponent(JSON.stringify(obj)));
  }
  async function getJson(url, opts) {
    try {
      var r = await fetch(url, Object.assign({ credentials: 'include' }, opts || {}));
      if (!r.ok) return { __error: r.status };
      return await r.json();
    } catch (e) { return { __error: String(e) }; }
  }
  async function tick() {
    if (done) return;
    var v = await getJson(SETUP + '/validate?' + Q, { method: 'POST', body: 'null', headers: { 'Content-Type': 'text/plain' } });
    if (!v || v.__error || !v.dsInfo) {
      // Skip Apple's landing page and go straight to the sign-in form (passkey / iPhone / password).
      var b = Array.from(document.querySelectorAll('button, a, ui-button')).find(function (e) {
        return /^\s*(sign in|σύνδεση)\s*$/i.test(e.textContent || '');
      });
      if (b) b.click();
      if (!askedLogin) { askedLogin = true; send({ status: 'needs_login' }); }
      setTimeout(tick, 3000);
      return;
    }
    done = true;
    var ds = v.dsInfo;
    var q = Q + '&dsid=' + encodeURIComponent(ds.dsid);
    var storage = await getJson(SETUP + '/storageUsageInfo?' + q, { method: 'POST', body: 'null', headers: { 'Content-Type': 'text/plain' } });
    if (storage.__error) storage = await getJson(SETUP + '/storageUsageInfo?' + q);
    var plan = await getJson('https://gatewayws.icloud.com/acsegateway/v3/accounts/' + ds.dsid + '/subscriptions/features/cloud.storage/plan-summary?' + q);
    var devs = await getJson('https://setup.icloud.com/setup/web/device/getDevices?' + q);
    var devices = ((devs && devs.devices) || []).map(function (d) {
      return { name: d.name, model: d.modelDisplayName || d.model, os: d.osVersion, deviceClass: d.deviceClass };
    });
    send({
      status: storage.__error ? 'error' : 'ok',
      error: storage.__error ? ('storage ' + storage.__error) : null,
      account: { fullName: ds.fullName, appleId: ds.appleId || ds.primaryEmail },
      storage: storage, plan: plan, devices: devices
    });
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', function () { setTimeout(tick, 1500); });
  else setTimeout(tick, 1500);
})();
"#;

/// Opens (or reuses) the iCloud window. `interactive` shows it immediately.
/// A silent refresh only shows the window when Apple asks the user to sign in again.
pub fn open(app: &AppHandle, interactive: bool) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(WINDOW) {
        if interactive {
            let _ = w.show();
            let _ = w.set_focus();
        }
        let _ = w.eval("window.__omnistore=false;location.reload();");
        return Ok(());
    }
    let script = SCRIPT
        .replace("__CLIENT_ID__", &uuid::Uuid::new_v4().to_string().to_uppercase())
        .replace("__HOST__", REPORT_HOST);
    let handle = app.clone();
    let url: Url = "https://www.icloud.com/".parse().map_err(|e| format!("{e}"))?;
    let builder = WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::External(url))
        .initialization_script(&script)
        .on_navigation(move |url| {
            if url.host_str() == Some(REPORT_HOST) {
                let payload = url
                    .query_pairs()
                    .find(|(k, _)| k == "d")
                    .and_then(|(_, v)| serde_json::from_str::<Value>(&v).ok());
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(p) = payload {
                        handle_report(&h, p);
                    }
                });
                return false;
            }
            true
        });
    #[cfg(desktop)]
    let builder = builder.title("Sign in to iCloud").inner_size(600.0, 780.0).min_inner_size(420.0, 600.0).center().visible(interactive);
    builder.build().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn close(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WINDOW) {
        let _ = w.close();
    }
}

fn handle_report(app: &AppHandle, p: Value) {
    let status = p["status"].as_str().unwrap_or("error");
    match status {
        "needs_login" => {
            if let Some(w) = app.get_webview_window(WINDOW) {
                let _ = w.show();
                let _ = w.set_focus();
            }
            let _ = app.emit("icloud://status", "needs_login");
        }
        "ok" => {
            let account = parse_account(&p);
            let devices = parse_devices(&p);
            crate::apply_icloud(app, account, devices);
            close(app);
            let _ = app.emit("icloud://status", "connected");
        }
        _ => {
            let msg = p["error"].as_str().unwrap_or("unknown error").to_string();
            let _ = app.emit("icloud://status", format!("error: {msg}"));
        }
    }
}

fn u(v: &Value) -> u64 {
    v.as_u64().or_else(|| v.as_f64().map(|f| f.max(0.0) as u64)).unwrap_or(0)
}

/// Finds `{ limit, limitUnits }` anywhere in the plan-summary payload.
fn plan_limit_bytes(v: &Value) -> Option<u64> {
    match v {
        Value::Object(map) => {
            if let (Some(limit), Some(units)) = (map.get("limit").and_then(|l| l.as_f64()), map.get("limitUnits").and_then(|u| u.as_str())) {
                let mult: f64 = match units.to_ascii_uppercase().as_str() {
                    "TIB" => 1024f64.powi(4),
                    "TB" => 1e12,
                    "GB" => 1e9,
                    "MIB" => 1024f64.powi(2),
                    _ => 1024f64.powi(3),
                };
                return Some((limit * mult) as u64);
            }
            map.values().find_map(plan_limit_bytes)
        }
        Value::Array(a) => a.iter().find_map(plan_limit_bytes),
        _ => None,
    }
}

pub fn parse_account(p: &Value) -> CloudAccount {
    let s = &p["storage"];
    let info = &s["storageUsageInfo"];
    let used = u(&info["usedStorageInBytes"]);
    let total = u(&info["totalStorageInBytes"]);
    let commerce = u(&info["commerceStorageInBytes"]);
    let comp = u(&info["compStorageInBytes"]);

    let breakdown: Vec<UsageSlice> = s["storageUsageByMedia"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|m| UsageSlice {
                    key: m["mediaKey"].as_str().unwrap_or("other").to_string(),
                    label: m["displayLabel"].as_str().unwrap_or("Other").to_string(),
                    bytes: u(&m["usageInBytes"]),
                    color: m["displayColor"].as_str().map(|c| if c.starts_with('#') { c.to_string() } else { format!("#{c}") }),
                })
                .filter(|m| m.bytes > 0)
                .collect()
        })
        .unwrap_or_default();

    // When an account is over quota (for example after an iCloud+ plan lapsed), Apple caps
    // usedStorageInBytes at the quota while the per-category numbers stay real. Use the larger.
    let by_media: u64 = breakdown.iter().map(|b: &UsageSlice| b.bytes).sum();
    let used = used.max(by_media);

    let plan = match plan_limit_bytes(&p["plan"]) {
        Some(bytes) => {
            let mut m = plans::match_by_quota(Provider::Icloud, bytes);
            m.exact = true;
            m
        }
        None => {
            // commerce = purchased storage, comp = the free 5 GB. Free accounts have commerce 0.
            let basis = if commerce > 0 { commerce } else if total > 0 { total } else { comp };
            plans::match_by_quota(Provider::Icloud, basis)
        }
    };
    let limit = if total > 0 { total } else { plan.bytes };
    let over_quota = s["quotaStatus"]["overQuota"].as_bool().unwrap_or(false) || used > limit;
    let label = p["account"]["appleId"]
        .as_str()
        .or(p["account"]["fullName"].as_str())
        .unwrap_or("Apple Account")
        .to_string();

    CloudAccount {
        provider: Provider::Icloud,
        account_label: label,
        used,
        limit,
        status: crate::rules::status_for(used, limit, over_quota),
        over_by: used.saturating_sub(limit),
        plan,
        breakdown,
        trash_bytes: 0,
        family_shared: !s["familyStorageUsageInfo"].is_null(),
        fetched_at: crate::local::now(),
        note: None,
    }
}

fn parse_devices(p: &Value) -> Vec<DeviceSnapshot> {
    p["devices"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|d| {
                    let name = d["name"].as_str().unwrap_or("Apple device").to_string();
                    let model = d["model"].as_str().map(String::from);
                    let m = model.clone().unwrap_or_default().to_lowercase();
                    let class = d["deviceClass"].as_str().unwrap_or("").to_lowercase();
                    let kind = if m.contains("ipad") || class.contains("ipad") {
                        "ipados"
                    } else if m.contains("iphone") || class.contains("iphone") {
                        "ios"
                    } else if m.contains("mac") || class.contains("mac") {
                        "macos"
                    } else {
                        "apple"
                    };
                    DeviceSnapshot {
                        id: format!("icloud:{name}:{}", model.clone().unwrap_or_default()),
                        name,
                        kind: kind.into(),
                        model,
                        os: d["os"].as_str().map(String::from),
                        source: "iCloud account".into(),
                        updated_at: crate::local::now(),
                        ..Default::default()
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_storage_payload() {
        let p: Value = serde_json::from_str(
            r#"{"status":"ok","account":{"appleId":"a@b.c"},
            "storage":{"storageUsageInfo":{"usedStorageInBytes":60000000000,"totalStorageInBytes":53687091200,"commerceStorageInBytes":53687091200,"compStorageInBytes":5368709120},
            "quotaStatus":{"overQuota":true},
            "storageUsageByMedia":[{"mediaKey":"photos","displayLabel":"Photos","displayColor":"ff9500","usageInBytes":40000000000}]},
            "plan":{"__error":403},"devices":[{"name":"Aris iPhone","model":"iPhone 15 Pro"}]}"#,
        )
        .unwrap();
        let a = parse_account(&p);
        assert_eq!(a.plan.id, "icloud-50");
        assert_eq!(a.status, "over");
        assert_eq!(a.breakdown[0].color.as_deref(), Some("#ff9500"));
        assert_eq!(parse_devices(&p)[0].kind, "ios");
    }

    #[test]
    fn over_quota_uses_category_totals() {
        let p: Value = serde_json::from_str(
            r#"{"status":"ok","account":{"appleId":"a@b.c"},
            "storage":{"storageUsageInfo":{"usedStorageInBytes":5368709120,"totalStorageInBytes":5368709120,"commerceStorageInBytes":0,"compStorageInBytes":5368709120},
            "quotaStatus":{"overQuota":true},
            "storageUsageByMedia":[{"mediaKey":"photos","usageInBytes":57100000000},{"mediaKey":"backup","usageInBytes":19000000000},{"mediaKey":"docs","usageInBytes":138100000000}]},
            "plan":{"__error":403}}"#,
        )
        .unwrap();
        let a = parse_account(&p);
        assert_eq!(a.plan.id, "icloud-free");
        assert_eq!(a.used, 214_200_000_000);
        assert_eq!(a.status, "over");
        assert!(a.over_by > 208_000_000_000);
    }
}
