//! "Continue with Google": the user signs in on Google's own page in their default browser
//! (account picker, passkey, or whatever they normally use) and approves OmniStore.
//! Google blocks sign-in inside embedded app webviews, so the system browser is the only path.
//! OmniStore asks for one non-sensitive scope and only reads the storage quota.

use crate::model::*;
use crate::plans;
use crate::store::secrets;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";
const ABOUT_URL: &str = "https://www.googleapis.com/drive/v3/about";
const REFRESH_KEY: &str = "google_refresh_token";

/// Baked in at build time from GitHub secrets. Users never see or type these.
fn client() -> Option<(&'static str, &'static str)> {
    let id = option_env!("OMNISTORE_GOOGLE_CLIENT_ID").filter(|s| !s.is_empty())?;
    let secret = option_env!("OMNISTORE_GOOGLE_CLIENT_SECRET").unwrap_or("");
    Some((id, secret))
}

pub fn available() -> bool {
    client().is_some()
}

fn http() -> reqwest::Client {
    reqwest::Client::builder().timeout(Duration::from_secs(30)).build().expect("http client")
}

fn random_b64(n: usize) -> String {
    let mut b = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut b);
    URL_SAFE_NO_PAD.encode(b)
}

const DONE_PAGE: &str = "<!doctype html><meta charset=utf-8><title>OmniStore</title>\
<body style=\"font-family:system-ui;background:#0b1020;color:#e6e9f2;display:grid;place-items:center;height:100vh;margin:0\">\
<div style=\"text-align:center\"><h2>Google connected</h2><p>You can close this tab and go back to OmniStore.</p></div>\
<script>setTimeout(function(){window.close()},1500)</script>";

/// Opens Google's sign-in in the browser and waits for the approval. Returns the account email.
pub async fn connect(app: &AppHandle) -> Result<String, String> {
    let (client_id, client_secret) = client().ok_or("Google sign-in is not included in this build.")?;
    let verifier = random_b64(64);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_b64(16);

    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");

    let mut auth = url::Url::parse(AUTH_URL).unwrap();
    auth.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", &redirect)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPE)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "select_account consent");
    app.opener().open_url(auth.as_str(), None::<&str>).map_err(|e| e.to_string())?;

    let code = tokio::time::timeout(Duration::from_secs(300), wait_for_code(listener, &state))
        .await
        .map_err(|_| "Google sign-in timed out.".to_string())??;

    let mut form = vec![
        ("code", code.as_str()),
        ("client_id", client_id),
        ("redirect_uri", redirect.as_str()),
        ("grant_type", "authorization_code"),
        ("code_verifier", verifier.as_str()),
    ];
    if !client_secret.is_empty() {
        form.push(("client_secret", client_secret));
    }
    let tok: Value = http()
        .post(TOKEN_URL)
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let refresh = tok["refresh_token"]
        .as_str()
        .ok_or_else(|| format!("Google did not finish sign-in: {}", tok["error_description"].as_str().unwrap_or("no token")))?;
    secrets::set(REFRESH_KEY, refresh)?;
    let access = tok["access_token"].as_str().unwrap_or_default().to_string();
    let about = about(&access).await?;
    Ok(about["user"]["emailAddress"].as_str().unwrap_or("Google account").to_string())
}

async fn wait_for_code(listener: TcpListener, expected_state: &str) -> Result<String, String> {
    loop {
        let (mut sock, _) = listener.accept().await.map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await.map_err(|e| e.to_string())?;
        let req = String::from_utf8_lossy(&buf[..n]).to_string();
        let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
        let parsed = url::Url::parse(&format!("http://127.0.0.1{path}")).ok();
        let get = |k: &str| parsed.as_ref().and_then(|u| u.query_pairs().find(|(q, _)| q == k).map(|(_, v)| v.to_string()));
        if let Some(err) = get("error") {
            let _ = respond(&mut sock, "<h2>Google sign-in cancelled</h2>").await;
            return Err(format!("Google sign-in cancelled ({err})"));
        }
        if let Some(code) = get("code") {
            if get("state").as_deref() != Some(expected_state) {
                let _ = respond(&mut sock, "<h2>Sign-in check failed</h2>").await;
                return Err("Google sign-in check failed. Try again.".into());
            }
            let _ = respond(&mut sock, DONE_PAGE).await;
            return Ok(code);
        }
        let _ = respond(&mut sock, "").await; // favicon and other noise
    }
}

async fn respond(sock: &mut tokio::net::TcpStream, body: &str) -> std::io::Result<()> {
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    sock.write_all(resp.as_bytes()).await?;
    sock.shutdown().await
}

pub fn is_connected() -> bool {
    secrets::get(REFRESH_KEY).is_some()
}

pub fn disconnect() {
    secrets::delete(REFRESH_KEY);
}

pub async fn access_token() -> Result<String, String> {
    let (client_id, client_secret) = client().ok_or("Google sign-in is not included in this build.")?;
    let refresh = secrets::get(REFRESH_KEY).ok_or("Google is not connected")?;
    let mut form = vec![("client_id", client_id.to_string()), ("refresh_token", refresh), ("grant_type", "refresh_token".to_string())];
    if !client_secret.is_empty() {
        form.push(("client_secret", client_secret.to_string()));
    }
    let tok: Value = http()
        .post(TOKEN_URL)
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    tok["access_token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| "Google sign-in expired. Click Continue with Google again.".to_string())
}

async fn about(token: &str) -> Result<Value, String> {
    let r = http()
        .get(ABOUT_URL)
        .query(&[("fields", "user(displayName,emailAddress),storageQuota")])
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(format!("Google storage read failed ({})", r.status()));
    }
    r.json().await.map_err(|e| e.to_string())
}

fn n(v: &Value) -> u64 {
    v.as_str().and_then(|s| s.parse().ok()).or_else(|| v.as_u64()).unwrap_or(0)
}

pub async fn fetch_account(token: &str) -> Result<CloudAccount, String> {
    let a = about(token).await?;
    Ok(parse_account(&a))
}

pub fn parse_account(a: &Value) -> CloudAccount {
    let q = &a["storageQuota"];
    let limit = n(&q["limit"]);
    let used = n(&q["usage"]);
    let in_drive = n(&q["usageInDrive"]);
    let trash = n(&q["usageInDriveTrash"]);
    let plan = if limit == 0 {
        PlanMatch {
            id: "google-unlimited".into(),
            name: "Workspace / unlimited".into(),
            bytes: 0,
            monthly_eur: 0.0,
            price_verified: false,
            exact: false,
        }
    } else {
        plans::match_by_quota(Provider::Google, limit)
    };
    let breakdown = vec![
        UsageSlice { key: "drive".into(), label: "Drive".into(), bytes: in_drive.saturating_sub(trash), color: Some("#4285F4".into()) },
        UsageSlice { key: "trash".into(), label: "Drive trash".into(), bytes: trash, color: Some("#9AA0A6".into()) },
        UsageSlice { key: "gmail_photos".into(), label: "Gmail + Google Photos".into(), bytes: used.saturating_sub(in_drive), color: Some("#EA4335".into()) },
    ];
    CloudAccount {
        provider: Provider::Google,
        account_label: a["user"]["emailAddress"].as_str().unwrap_or("Google account").to_string(),
        used,
        limit,
        status: crate::rules::status_for(used, limit, limit > 0 && used > limit),
        over_by: if limit > 0 { used.saturating_sub(limit) } else { 0 },
        plan,
        breakdown: breakdown.into_iter().filter(|s| s.bytes > 0).collect(),
        trash_bytes: trash,
        family_shared: false,
        fetched_at: crate::local::now(),
        note: Some("Google reports Gmail and Photos as one combined figure.".into()),
        insights: vec![],
    }
}

const FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";

/// Every OmniStore signed in to the same Google account keeps small device snapshots in
/// OmniStore's hidden app folder in Google Drive (invisible in Drive, only OmniStore can read it).
/// Uploads `publish` (this computer plus any iPhone/iPad it read) and returns everything else.
pub async fn sync_devices(token: &str, publish: &[DeviceSnapshot]) -> Result<Vec<DeviceSnapshot>, String> {
    let c = http();
    let list: Value = c
        .get(FILES_URL)
        .query(&[("spaces", "appDataFolder"), ("fields", "files(id,name)"), ("pageSize", "200")])
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let files = list["files"].as_array().cloned().unwrap_or_default();
    let mine: Vec<String> = publish.iter().map(|d| format!("device-{}.json", d.id)).collect();

    for (dev, name) in publish.iter().zip(mine.iter()) {
        let body = serde_json::to_vec(dev).map_err(|e| e.to_string())?;
        let existing = files.iter().find(|f| f["name"].as_str() == Some(name.as_str())).and_then(|f| f["id"].as_str()).map(String::from);
        let file_id = match existing {
            Some(id) => id,
            None => {
                let created: Value = c
                    .post(FILES_URL)
                    .bearer_auth(token)
                    .json(&serde_json::json!({ "name": name, "parents": ["appDataFolder"], "mimeType": "application/json" }))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .json()
                    .await
                    .map_err(|e| e.to_string())?;
                match created["id"].as_str() {
                    Some(id) => id.to_string(),
                    None => continue,
                }
            }
        };
        let _ = c
            .patch(format!("{UPLOAD_URL}/{file_id}"))
            .query(&[("uploadType", "media")])
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await;
    }

    let mut others = vec![];
    for f in &files {
        let name = f["name"].as_str().unwrap_or("");
        if !name.starts_with("device-") || mine.iter().any(|m| m == name) {
            continue;
        }
        let Some(id) = f["id"].as_str() else { continue };
        if let Ok(resp) = c.get(format!("{FILES_URL}/{id}")).query(&[("alt", "media")]).bearer_auth(token).send().await {
            if let Ok(mut d) = resp.json::<DeviceSnapshot>().await {
                d.source = "Google sync".into();
                others.push(d);
            }
        }
    }
    Ok(others)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_quota() {
        let v: Value = serde_json::from_str(
            r#"{"user":{"emailAddress":"a@gmail.com"},"storageQuota":{"limit":"107374182400","usage":"68500000000","usageInDrive":"23000000000","usageInDriveTrash":"3300000000"}}"#,
        )
        .unwrap();
        let a = parse_account(&v);
        assert_eq!(a.plan.id, "google-100");
        assert_eq!(a.trash_bytes, 3_300_000_000);
        assert_eq!(a.breakdown.len(), 3);
        assert_eq!(a.status, "ok");
    }
}
