//! iCloud: the user signs in on Apple's own icloud.com page inside an OmniStore window.
//! OmniStore never sees the password. Once the session exists, a small script running
//! on icloud.com reads what the icloud.com web apps themselves read (storage, Photos
//! library index, iCloud Drive folder list) and hands compact totals back by navigating
//! to a fake host that the app intercepts and cancels. No cable, nothing on the devices.

use crate::model::*;
use crate::plans;
use crate::store::AppState;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW: &str = "icloud";
const REPORT_HOST: &str = "omnistore.invalid";
static SCAN_GEN: AtomicU64 = AtomicU64::new(0);

const SCRIPT: &str = r#"
(function () {
  if (window.__omnistore) return;
  window.__omnistore = true;
  if (!/(^|\.)icloud\.com$/.test(location.hostname)) return;
  var Q = 'clientBuildNumber=2534Project66&clientMasteringNumber=2534B22&clientId=__CLIENT_ID__';
  var SETUP = 'https://setup.icloud.com/setup/ws/1';
  var PREV = __PREV__;
  var askedLogin = false, done = false, lastSend = 0, chain = Promise.resolve();

  function sleep(ms) { return new Promise(function (r) { setTimeout(r, ms); }); }
  // Reports leave one at a time: two navigations started back to back cancel each other.
  function send(obj) {
    chain = chain.then(async function () {
      var wait = lastSend + 1200 - Date.now();
      if (wait > 0) await sleep(wait);
      lastSend = Date.now();
      window.location.assign('https://__HOST__/report?d=' + encodeURIComponent(JSON.stringify(obj)));
    });
    return chain;
  }
  async function getJson(url, opts) {
    try {
      var r = await fetch(url, Object.assign({ credentials: 'include' }, opts || {}));
      if (!r.ok) return { __error: r.status };
      return await r.json();
    } catch (e) { return { __error: String(e) }; }
  }
  async function post(url, body) {
    for (var i = 0; i < 4; i++) {
      try {
        var r = await fetch(url, { method: 'POST', credentials: 'include', headers: { 'Content-Type': 'text/plain' }, body: JSON.stringify(body) });
        if (r.status === 429 || r.status === 503) { await sleep(1500 * (i + 1)); continue; }
        if (!r.ok) return { __error: r.status };
        return await r.json();
      } catch (e) {
        if (i === 3) return { __error: String(e).slice(0, 80) };
        await sleep(800 * (i + 1));
      }
    }
    return { __error: 'busy' };
  }
  function text(b64) {
    try {
      var s = decodeURIComponent(escape(atob(b64)));
      return /^[^\u0000-\u001f�]{1,40}$/.test(s) ? s : null;
    } catch (e) { return null; }
  }

  // ---------------------------------------------------------------- iCloud Drive
  async function readDrive(ws, q, diag) {
    var base = ws.drivews && ws.drivews.url;
    if (!base) { diag.drive = 'off'; return null; }
    var url = base + '/retrieveItemDetailsInFolders?' + q;
    var root = await post(url, [{ drivewsid: 'FOLDER::com.apple.CloudDocs::root', partialData: false }]);
    if (!root || root.__error || !root[0]) { diag.drive = 'error ' + (root && root.__error); return null; }
    root = root[0];
    var items = (root.items || []).map(function (it) {
      var isFile = it.type === 'FILE';
      return {
        id: it.drivewsid, type: it.type,
        name: String(it.name || '') + (isFile && it.extension ? '.' + it.extension : ''),
        bytes: isFile ? (it.size || 0) : (typeof it.assetQuota === 'number' ? it.assetQuota : null),
        files: isFile ? 1 : (it.fileCount || 0)
      };
    });
    // App folders come back without a size: ask for their details in one batch.
    var missing = items.filter(function (it) { return it.bytes === null && it.id; }).slice(0, 60);
    if (missing.length) {
      var det = await post(url, missing.map(function (it) { return { drivewsid: it.id, partialData: false }; }));
      if (det && !det.__error && det.forEach) {
        det.forEach(function (d) {
          var it = items.find(function (x) { return x.id === d.drivewsid; });
          if (!it) return;
          if (typeof d.assetQuota === 'number') { it.bytes = d.assetQuota; it.files = d.fileCount || it.files; return; }
          var sum = 0, n = 0;
          (d.items || []).forEach(function (c) {
            sum += c.type === 'FILE' ? (c.size || 0) : (c.assetQuota || 0);
            n += c.type === 'FILE' ? 1 : (c.fileCount || 0);
          });
          it.bytes = sum; it.files = n;
        });
      }
    }
    var trashBytes = 0, trashCount = 0;
    var trash = await post(url, [{ drivewsid: 'FOLDER::com.apple.CloudDocs::TRASH_ROOT', partialData: false }]);
    if (trash && !trash.__error && trash[0]) {
      var t = trash[0];
      (t.items || []).forEach(function (c) { trashBytes += c.type === 'FILE' ? (c.size || 0) : (c.assetQuota || 0); });
      if (!trashBytes && typeof t.assetQuota === 'number') trashBytes = t.assetQuota;
      trashCount = t.numberOfItems || (t.items || []).length;
    }
    items = items.filter(function (it) { return it.bytes; }).sort(function (a, b) { return b.bytes - a.bytes; });
    var sumItems = items.reduce(function (s, it) { return s + it.bytes; }, 0);
    diag.drive = 'ok ' + items.length;
    return {
      total: Math.max(typeof root.assetQuota === 'number' ? root.assetQuota : 0, sumItems),
      files: root.fileCount || items.reduce(function (s, it) { return s + it.files; }, 0),
      items: items.slice(0, 14).map(function (it) { return { name: it.name.slice(0, 48), type: it.type, bytes: it.bytes, files: it.files }; }),
      more: items.slice(14).reduce(function (s, it) { return s + it.bytes; }, 0),
      trashBytes: trashBytes, trashCount: trashCount
    };
  }

  // ---------------------------------------------------------------- iCloud Photos
  var ZONE = { zoneName: 'PrimarySync', zoneType: 'REGULAR_CUSTOM_ZONE' };
  var COUNT_IDS = {
    all: 'CPLAssetByAssetDateWithoutHiddenOrDeleted',
    videos: 'CPLAssetInSmartAlbumByAssetDate:Video',
    screenshots: 'CPLAssetInSmartAlbumByAssetDate:Screenshot',
    live: 'CPLAssetInSmartAlbumByAssetDate:Live',
    hidden: 'CPLAssetHiddenByAssetDate',
    deleted: 'CPLAssetDeletedByExpungedDate'
  };
  var KEYS = ['resOriginalRes', 'resOriginalVidComplRes', 'resOriginalAltRes', 'resJPEGFullRes', 'resVidFullRes', 'itemType',
    'resOriginalFileType', 'masterRef', 'assetSubtypeV2', 'recordName', 'recordType', 'recordChangeTag'];
  var KEYS_X = KEYS.concat(['importedByDisplayNameEnc', 'importedByBundleIdentifierEnc', 'importedBy']);
  var BUNDLES = {
    'com.apple.camera': 'Camera', 'com.apple.mobileslideshow': 'Photos', 'com.apple.mobilesafari': 'Safari',
    'com.apple.MobileSMS': 'Messages', 'com.apple.DocumentsApp': 'Files', 'net.whatsapp.WhatsApp': 'WhatsApp',
    'com.burbn.instagram': 'Instagram', 'ph.telegra.Telegraph': 'Telegram', 'com.facebook.Messenger': 'Messenger',
    'com.facebook.Facebook': 'Facebook', 'com.toyopagroup.picaboo': 'Snapchat', 'com.viber': 'Viber',
    'com.zhiliaoapp.musically': 'TikTok', 'com.google.photos': 'Google Photos', 'com.google.chrome.ios': 'Chrome'
  };

  async function countOf(base, pq, id, zone) {
    var r = await post(base + '/internal/records/query/batch?' + pq, { batch: [{
      resultsLimit: 1,
      query: { recordType: 'HyperionIndexCountLookup', filterBy: { fieldName: 'indexCountID', comparator: 'IN', fieldValue: { type: 'STRING_LIST', value: [id] } } },
      zoneWide: true, zoneID: zone
    }] });
    try { return r.batch[0].records[0].fields.itemCount.value; } catch (e) { return null; }
  }

  async function photoCounts(ws, q, diag) {
    var root = ws.ckdatabasews && ws.ckdatabasews.url;
    if (!root) { diag.photos = 'off'; return null; }
    var base = root + '/database/1/com.apple.photos.cloud/production/private';
    var pq = q + '&remapEnums=true&getCurrentSyncToken=true';
    var keys = Object.keys(COUNT_IDS);
    var vals = await Promise.all(keys.map(function (k) { return countOf(base, pq, COUNT_IDS[k], ZONE); }));
    var counts = {};
    keys.forEach(function (k, i) { if (typeof vals[i] === 'number') counts[k] = vals[i]; });
    if (typeof counts.all !== 'number') { diag.photos = 'no library'; return null; }
    // A Shared Library you own counts against your storage too.
    var zones = [{ zone: ZONE, count: counts.all }];
    var zl = await post(base + '/zones/list?' + pq, {});
    if (zl && zl.zones) {
      for (var i = 0; i < zl.zones.length; i++) {
        var z = zl.zones[i];
        if (z.deleted || !z.zoneID || !/^SharedSync-/.test(z.zoneID.zoneName || '')) continue;
        var n = await countOf(base, pq, COUNT_IDS.all, z.zoneID);
        if (n) { zones.push({ zone: z.zoneID, count: n }); counts.shared = (counts.shared || 0) + n; }
      }
    }
    diag.photos = 'counts ok';
    return { base: base, pq: pq, counts: counts, zones: zones };
  }

  async function scanPhotos(ph, diag) {
    var agg = {};
    ['photos', 'live', 'videos', 'screenshots', 'edits', 'hidden', 'deleted'].forEach(function (k) { agg[k] = { n: 0, b: 0 }; });
    var apps = {}, unknown = { n: 0, b: 0 };
    var seen = new Set();
    var scanned = 0, failed = 0, partial = false, keys = KEYS_X, triedPlain = false;
    var started = Date.now(), lastReport = Date.now();

    var jobs = ph.zones.map(function (z) { return { type: 'CPLAssetAndMasterByAssetDateWithoutHiddenOrDeleted', zone: z.zone, count: z.count, bucket: null }; });
    if (ph.counts.hidden) jobs.push({ type: 'CPLAssetAndMasterHiddenByAssetDate', zone: ZONE, count: ph.counts.hidden, bucket: 'hidden' });
    if (ph.counts.deleted) jobs.push({ type: 'CPLAssetAndMasterDeletedByExpungedDate', zone: ZONE, count: ph.counts.deleted, bucket: 'deleted' });
    var total = jobs.reduce(function (s, j) { return s + j.count; }, 0);

    function size(rec, k) { try { return rec.fields[k].value.size || 0; } catch (e) { return 0; } }
    function field(m, a, k) {
      var f = (m.fields && m.fields[k]) || (a && a.fields && a.fields[k]);
      return f ? f.value : null;
    }
    function add(job, m, a) {
      var orig = size(m, 'resOriginalRes') + size(m, 'resOriginalAltRes');
      var live = size(m, 'resOriginalVidComplRes');
      var edits = a ? size(a, 'resJPEGFullRes') + size(a, 'resVidFullRes') : 0;
      scanned++;
      if (job.bucket) { agg[job.bucket].n++; agg[job.bucket].b += orig + live + edits; return; }
      var type = String(field(m, a, 'itemType') || field(m, a, 'resOriginalFileType') || '');
      var isVideo = /movie|video|mpeg|quicktime|3gpp|avi/i.test(type);
      var shot = field(m, a, 'assetSubtypeV2') === 3;
      var bucket = isVideo ? 'videos' : (shot ? 'screenshots' : 'photos');
      agg[bucket].n++; agg[bucket].b += orig;
      if (live) { agg.live.n++; agg.live.b += live; }
      if (edits) { agg.edits.n++; agg.edits.b += edits; }
      var name = null, dn = field(m, a, 'importedByDisplayNameEnc'), bid = field(m, a, 'importedByBundleIdentifierEnc');
      if (typeof dn === 'string') name = text(dn);
      if (typeof bid === 'string') { var b = text(bid); if (b && BUNDLES[b]) name = BUNDLES[b]; else if (!name && b && b.indexOf('.') > 0) name = b.split('.').pop(); }
      if (!name) { var by = field(m, a, 'importedBy'); if (by === 1 || by === 2) name = 'Camera'; }
      if (!name && shot) name = 'Screenshots';
      var tot = orig + live + edits;
      if (name) { apps[name] = apps[name] || { n: 0, b: 0 }; apps[name].n++; apps[name].b += tot; }
      else { unknown.n++; unknown.b += tot; }
    }
    async function page(job, offset) {
      function body() {
        return {
          query: { recordType: job.type, filterBy: [
            { fieldName: 'direction', comparator: 'EQUALS', fieldValue: { type: 'STRING', value: 'ASCENDING' } },
            { fieldName: 'startRank', comparator: 'EQUALS', fieldValue: { type: 'INT64', value: offset } }
          ] },
          zoneID: job.zone, desiredKeys: keys, resultsLimit: 200
        };
      }
      var r = await post(ph.base + '/records/query?' + ph.pq, body());
      if (r && r.__error === 400 && !triedPlain) { triedPlain = true; keys = KEYS; r = await post(ph.base + '/records/query?' + ph.pq, body()); }
      if (!r || r.__error || !r.records) { failed++; return; }
      var assets = {};
      r.records.forEach(function (rec) {
        if (rec.recordType === 'CPLAsset' && rec.fields && rec.fields.masterRef && rec.fields.masterRef.value) assets[rec.fields.masterRef.value.recordName] = rec;
      });
      r.records.forEach(function (rec) {
        if (rec.recordType !== 'CPLMaster' || seen.has(rec.recordName)) return;
        seen.add(rec.recordName);
        add(job, rec, assets[rec.recordName]);
      });
    }
    function report(final) {
      var list = Object.keys(apps).map(function (k) { return { name: k, n: apps[k].n, b: apps[k].b }; }).sort(function (a, b) { return b.b - a.b; });
      var rest = list.slice(12).reduce(function (s, x) { return { n: s.n + x.n, b: s.b + x.b }; }, { n: 0, b: 0 });
      return send({ status: 'photos', done: final, partial: partial || failed > 0, scanned: scanned, total: total,
        agg: agg, apps: list.slice(0, 12), appsOther: rest, appsUnknown: unknown, counts: ph.counts });
    }
    for (var j = 0; j < jobs.length; j++) {
      var job = jobs[j], next = 0;
      var worker = async function () {
        while (next < job.count) {
          if (Date.now() - started > 8 * 60 * 1000 || failed > 25) { partial = true; return; }
          var offset = next; next += 100;
          await page(job, offset);
          if (Date.now() - lastReport > 5000) { lastReport = Date.now(); report(false); }
        }
      };
      await Promise.all([worker(), worker(), worker(), worker()]);
    }
    diag.scan = scanned + '/' + total + (failed ? ' failed ' + failed : '');
    await report(true);
  }

  // ---------------------------------------------------------------- main
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
    var ds = v.dsInfo, ws = v.webservices || {};
    var q = Q + '&dsid=' + encodeURIComponent(ds.dsid);
    var storage = await getJson(SETUP + '/storageUsageInfo?' + q, { method: 'POST', body: 'null', headers: { 'Content-Type': 'text/plain' } });
    if (storage.__error) storage = await getJson(SETUP + '/storageUsageInfo?' + q);
    var plan = await getJson('https://gatewayws.icloud.com/acsegateway/v3/accounts/' + ds.dsid + '/subscriptions/features/cloud.storage/plan-summary?' + q);
    var devs = await getJson('https://setup.icloud.com/setup/web/device/getDevices?' + q);
    var devices = ((devs && devs.devices) || []).map(function (d) {
      return { name: d.name, model: d.modelDisplayName || d.model, os: d.osVersion, deviceClass: d.deviceClass };
    });
    var diag = {}, drive = null, ph = null;
    if (!storage.__error) {
      try { drive = await readDrive(ws, q, diag); } catch (e) { diag.drive = 'failed'; }
      try { ph = await photoCounts(ws, q, diag); } catch (e) { diag.photos = 'failed'; }
    }
    var fresh = PREV && PREV.count === (ph && ph.counts.all) && PREV.at && (Date.now() - PREV.at) < 7 * 864e5;
    var deep = !!(ph && ph.counts.all > 0 && !fresh);
    await send({
      status: storage.__error ? 'error' : 'ok',
      error: storage.__error ? ('storage ' + storage.__error) : null,
      account: { fullName: ds.fullName, appleId: ds.appleId || ds.primaryEmail },
      storage: storage, plan: plan, devices: devices,
      drive: drive, photos: ph ? { counts: ph.counts } : null, deep: deep, diag: diag
    });
    if (deep) {
      try { await scanPhotos(ph, diag); }
      catch (e) { send({ status: 'photos', done: true, partial: true, failedHard: true, counts: ph.counts }); }
    }
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', function () { setTimeout(tick, 1500); });
  else setTimeout(tick, 1500);
})();
"#;

/// The last finished photo scan, so the next refresh can skip an unchanged library.
fn previous_scan(app: &AppHandle) -> Value {
    let state = app.state::<AppState>();
    let d = state.dashboard.lock().unwrap();
    let done = d
        .icloud
        .as_ref()
        .and_then(|a| a.insights.iter().find(|g| g.key == "photos_types" && !g.estimated));
    match done {
        Some(g) => serde_json::json!({
            "count": g.index_count,
            "at": chrono::DateTime::parse_from_rfc3339(&g.updated_at).map(|t| t.timestamp_millis()).unwrap_or(0),
        }),
        None => Value::Null,
    }
}

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
        .replace("__HOST__", REPORT_HOST)
        .replace("__PREV__", &previous_scan(app).to_string());
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

/// Apple's sign-in cookies are session cookies unless "Keep me signed in" is ticked, so they
/// vanish whenever OmniStore restarts. OmniStore promises "signed in until you sign out", so
/// keep them for 60 days. Apple still ends the session on its side whenever it wants to.
fn persist_session(app: &AppHandle) {
    use tauri::webview::cookie::{time, Expiration};
    let Some(w) = app.get_webview_window(WINDOW) else { return };
    let Ok(url) = "https://www.icloud.com/".parse::<Url>() else { return };
    let Ok(cookies) = w.cookies_for_url(url) else { return };
    let until = time::OffsetDateTime::now_utc() + time::Duration::days(60);
    for mut c in cookies {
        let name = c.name().to_ascii_uppercase();
        let session_only = matches!(c.expires(), None | Some(Expiration::Session));
        if session_only && (name.starts_with("X-APPLE") || name.starts_with("X_APPLE")) {
            c.set_expires(until);
            let _ = w.set_cookie(c);
        }
    }
}

/// A photo scan keeps the hidden window alive. Never leave it open for good.
fn watchdog(app: AppHandle) {
    let generation = SCAN_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(600)).await;
        if SCAN_GEN.load(Ordering::SeqCst) != generation {
            return;
        }
        if let Some(w) = app.get_webview_window(WINDOW) {
            if !w.is_visible().unwrap_or(true) {
                let _ = w.close();
            }
        }
    });
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
            persist_session(app);
            let deep = p["deep"].as_bool().unwrap_or(false);
            let mut account = parse_account(&p);
            account.insights = parse_insights(&p, deep);
            let devices = parse_devices(&p);
            crate::apply_icloud(app, account, devices, !deep);
            let _ = app.emit("icloud://status", "connected");
            if deep {
                // The sign-in window has done its job. The scan continues out of sight.
                if let Some(w) = app.get_webview_window(WINDOW) {
                    let _ = w.hide();
                }
                watchdog(app.clone());
            } else {
                close(app);
            }
        }
        "photos" => {
            let done = p["done"].as_bool().unwrap_or(false);
            crate::apply_icloud_photos(app, parse_photo_scan(&p));
            if done {
                SCAN_GEN.fetch_add(1, Ordering::SeqCst);
                close(app);
            }
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
        insights: vec![],
    }
}

fn row(key: &str, label: &str, bytes: u64, count: Option<u64>) -> InsightRow {
    InsightRow { key: key.into(), label: label.into(), bytes, count }
}

/// Groups available right away: photo counts (sizes follow from the scan) and iCloud Drive.
pub fn parse_insights(p: &Value, scanning: bool) -> Vec<InsightGroup> {
    let mut out = vec![];
    let counts = &p["photos"]["counts"];
    if counts["all"].is_number() {
        let all = u(&counts["all"]) + u(&counts["shared"]);
        let videos = u(&counts["videos"]);
        let shots = u(&counts["screenshots"]);
        let mut rows = vec![
            row("photos", "Photos", 0, Some(all.saturating_sub(videos + shots))),
            row("videos", "Videos", 0, Some(videos)),
            row("screenshots", "Screenshots", 0, Some(shots)),
            row("hidden", "Hidden", 0, Some(u(&counts["hidden"]))),
            row("deleted", "Recently Deleted", 0, Some(u(&counts["deleted"]))),
        ];
        rows.retain(|r| r.count.unwrap_or(0) > 0);
        out.push(InsightGroup {
            key: "photos_types".into(),
            title: "Photos and videos".into(),
            note: None,
            total_count: all + u(&counts["hidden"]) + u(&counts["deleted"]),
            rows,
            total_bytes: 0,
            estimated: true,
            progress: scanning.then(|| "Measuring sizes…".to_string()),
            index_count: u(&counts["all"]),
            updated_at: crate::local::now(),
        });
    }

    let drive = &p["drive"];
    if drive["total"].is_number() {
        let total = u(&drive["total"]);
        let mut rows: Vec<InsightRow> = drive["items"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|it| {
                let name = it["name"].as_str().unwrap_or("Folder");
                row(&format!("item:{name}"), name, u(&it["bytes"]), Some(u(&it["files"])).filter(|n| *n > 0))
            })
            .collect();
        if u(&drive["more"]) > 0 {
            rows.push(row("more", "Other folders", u(&drive["more"]), None));
        }
        if u(&drive["trashBytes"]) > 0 {
            rows.push(row("trash", "Recently Deleted", u(&drive["trashBytes"]), Some(u(&drive["trashCount"])).filter(|n| *n > 0)));
        }
        // Apple's "Docs" number also holds app data that never shows up as files in iCloud Drive.
        let docs = p["storage"]["storageUsageByMedia"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|m| m["mediaKey"].as_str() == Some("docs"))
            .map(|m| u(&m["usageInBytes"]))
            .unwrap_or(0);
        let visible = total + u(&drive["trashBytes"]);
        if docs > visible + docs / 50 {
            rows.push(row("hidden_app_data", "App data not shown as files", docs - visible, None));
        }
        out.push(InsightGroup {
            key: "drive".into(),
            title: "iCloud Drive".into(),
            note: None,
            total_bytes: rows.iter().map(|r| r.bytes).sum(),
            total_count: u(&drive["files"]),
            rows,
            estimated: false,
            progress: None,
            index_count: 0,
            updated_at: crate::local::now(),
        });
    }
    out
}

/// Groups built from the photo scan: sizes by type, and by the app that saved each item.
pub fn parse_photo_scan(p: &Value) -> Vec<InsightGroup> {
    let done = p["done"].as_bool().unwrap_or(false);
    let partial = p["partial"].as_bool().unwrap_or(false);
    let agg = &p["agg"];
    let get = |k: &str| (u(&agg[k]["b"]), u(&agg[k]["n"]));
    let mut out = vec![];
    if agg.is_object() {
        let labels = [
            ("photos", "Photos"),
            ("live", "Live Photo motion"),
            ("videos", "Videos"),
            ("screenshots", "Screenshots"),
            ("edits", "Edited versions"),
            ("hidden", "Hidden"),
            ("deleted", "Recently Deleted"),
        ];
        let mut rows: Vec<InsightRow> = labels
            .iter()
            .map(|(k, label)| {
                let (b, n) = get(k);
                row(k, label, b, Some(n))
            })
            .filter(|r| r.bytes > 0 || r.count.unwrap_or(0) > 0)
            .collect();
        // Biggest first; Hidden and Recently Deleted stay at the bottom.
        let tail = |k: &str| matches!(k, "hidden" | "deleted");
        rows.sort_by(|a, b| tail(&a.key).cmp(&tail(&b.key)).then(b.bytes.cmp(&a.bytes)));
        let items = ["photos", "videos", "screenshots", "hidden", "deleted"].iter().map(|k| get(k).1).sum();
        out.push(InsightGroup {
            key: "photos_types".into(),
            title: "Photos and videos".into(),
            note: (done && partial).then(|| "Large library: part of it could not be measured this time.".to_string()),
            total_bytes: rows.iter().map(|r| r.bytes).sum(),
            total_count: items,
            rows,
            estimated: !done || partial,
            progress: (!done).then(|| format!("Measured {} of {} items", u(&p["scanned"]), u(&p["total"]))),
            index_count: u(&p["counts"]["all"]),
            updated_at: crate::local::now(),
        });

        let mut rows: Vec<InsightRow> = p["apps"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|a| {
                let name = a["name"].as_str().unwrap_or("App");
                row(&format!("app:{name}"), name, u(&a["b"]), Some(u(&a["n"])))
            })
            .collect();
        if !rows.is_empty() {
            if u(&p["appsOther"]["b"]) > 0 {
                rows.push(row("other", "Other apps", u(&p["appsOther"]["b"]), Some(u(&p["appsOther"]["n"]))));
            }
            if u(&p["appsUnknown"]["b"]) > 0 {
                rows.push(row("unknown", "Source not recorded", u(&p["appsUnknown"]["b"]), Some(u(&p["appsUnknown"]["n"]))));
            }
            out.push(InsightGroup {
                key: "photos_apps".into(),
                title: "Saved by".into(),
                note: None,
                total_bytes: rows.iter().map(|r| r.bytes).sum(),
                total_count: rows.iter().filter_map(|r| r.count).sum(),
                rows,
                estimated: !done || partial,
                progress: None,
                index_count: 0,
                updated_at: crate::local::now(),
            });
        }
    }
    out
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
        assert!(parse_insights(&p, false).is_empty());
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

    #[test]
    fn drive_and_photo_insights() {
        let p: Value = serde_json::from_str(
            r#"{"status":"ok","deep":true,
            "storage":{"storageUsageByMedia":[{"mediaKey":"docs","usageInBytes":138100000000}]},
            "photos":{"counts":{"all":12000,"videos":900,"screenshots":2100,"hidden":10,"deleted":40}},
            "drive":{"total":90000000000,"files":5200,"more":1000000000,"trashBytes":2000000000,"trashCount":31,
              "items":[{"name":"Desktop","type":"FOLDER","bytes":60000000000,"files":3000},{"name":"Keynote","type":"APP_LIBRARY","bytes":29000000000,"files":12}]}}"#,
        )
        .unwrap();
        let g = parse_insights(&p, true);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].key, "photos_types");
        assert_eq!(g[0].rows[0].count, Some(9000));
        assert!(g[0].estimated && g[0].progress.is_some());
        let drive = &g[1];
        assert_eq!(drive.rows[0].label, "Desktop");
        assert!(drive.rows.iter().any(|r| r.key == "trash" && r.bytes == 2_000_000_000));
        // 138.1 GB of Docs minus 92 GB visible in Drive (files + trash)
        assert_eq!(drive.rows.iter().find(|r| r.key == "hidden_app_data").unwrap().bytes, 46_100_000_000);

        let scan: Value = serde_json::from_str(
            r#"{"status":"photos","done":true,"partial":false,"scanned":12050,"total":12050,"counts":{"all":12000},
            "agg":{"photos":{"n":9000,"b":30000000000},"live":{"n":4000,"b":6000000000},"videos":{"n":900,"b":18000000000},
                   "screenshots":{"n":2100,"b":2500000000},"edits":{"n":0,"b":0},"hidden":{"n":10,"b":50000000},"deleted":{"n":40,"b":900000000}},
            "apps":[{"name":"Camera","n":8000,"b":40000000000},{"name":"WhatsApp","n":3000,"b":9000000000}],
            "appsOther":{"n":0,"b":0},"appsUnknown":{"n":1000,"b":7500000000}}"#,
        )
        .unwrap();
        let g = parse_photo_scan(&scan);
        assert_eq!(g.len(), 2);
        assert!(!g[0].estimated);
        assert_eq!(g[0].index_count, 12000);
        assert_eq!(g[0].total_count, 12050);
        assert_eq!(g[0].total_bytes, 57_450_000_000);
        assert!(g[0].rows.iter().all(|r| r.key != "edits"));
        assert_eq!(g[1].rows.last().unwrap().key, "unknown");
    }
}
