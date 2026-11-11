//! Plan status, overage, forecast and optimisation findings.

use crate::model::*;
use crate::plans;

const GB: u64 = 1_000_000_000;

pub fn status_for(used: u64, limit: u64, over: bool) -> String {
    if limit == 0 {
        return "ok".into();
    }
    let pct = used as f64 / limit as f64;
    if over || pct > 1.0 {
        "over"
    } else if pct >= 0.95 {
        "critical"
    } else if pct >= 0.85 {
        "near"
    } else {
        "ok"
    }
    .into()
}

pub fn fmt_bytes(b: u64) -> String {
    let b = b as f64;
    if b >= 1e12 {
        format!("{:.2} TB", b / 1e12)
    } else if b >= 1e9 {
        format!("{:.1} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{:.0} MB", b / 1e6)
    } else {
        format!("{:.0} KB", b / 1e3)
    }
}

fn eur(x: f64) -> String {
    format!("€{:.2}", x)
}

/// Days until full from a linear fit over the last 90 history points.
pub fn days_until_full(points: &[(f64, f64)], used: u64, limit: u64) -> Option<f64> {
    if points.len() < 7 || limit == 0 || used >= limit {
        return None;
    }
    let n = points.len() as f64;
    let (sx, sy) = points.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    let (mx, my) = (sx / n, sy / n);
    let (num, den) = points.iter().fold((0.0, 0.0), |a, p| (a.0 + (p.0 - mx) * (p.1 - my), a.1 + (p.0 - mx).powi(2)));
    if den == 0.0 {
        return None;
    }
    let slope = num / den; // bytes per day
    (slope > 1e6).then(|| (limit - used) as f64 / slope)
}

fn series(history: &[HistoryPoint], pick: fn(&HistoryPoint) -> Option<u64>) -> Vec<(f64, f64)> {
    history
        .iter()
        .rev()
        .take(90)
        .filter_map(|h| {
            let d = chrono::NaiveDate::parse_from_str(&h.date, "%Y-%m-%d").ok()?;
            Some((d.num_days_from_ce() as f64, pick(h)? as f64))
        })
        .collect()
}

use chrono::Datelike;

fn act(label: &str, kind: &str, target: Option<String>) -> Option<Action> {
    Some(Action { label: label.into(), kind: kind.into(), target })
}

fn slice(a: &CloudAccount, key: &str) -> u64 {
    a.breakdown.iter().filter(|s| s.key == key).map(|s| s.bytes).sum()
}

pub fn recompute(d: &mut Dashboard) {
    let mut f: Vec<Finding> = vec![];
    let mut plan_savings: Vec<f64> = vec![];

    // ---------------------------------------------------------- cloud plans
    for acc in [d.icloud.as_ref(), d.google.as_ref()].into_iter().flatten() {
        let p = acc.provider;
        let name = plans::provider(p).name.clone();
        let manage = Some(plans::manage_url(p));
        let pick: fn(&HistoryPoint) -> Option<u64> = match p {
            Provider::Icloud => |h| h.icloud_used,
            Provider::Google => |h| h.google_used,
        };
        let forecast = days_until_full(&series(&d.history, pick), acc.used, acc.limit);

        if acc.status == "over" {
            let up = plans::cheapest_fit(p, acc.used, 1.15);
            let detail = match &up {
                Some(t) => format!(
                    "{} used of {} on {}. Syncing and backups stop until you free {} or move to {} ({}/mo, +{}).",
                    fmt_bytes(acc.used), fmt_bytes(acc.limit), acc.plan.name, fmt_bytes(acc.over_by),
                    t.name, eur(t.monthly_eur), eur(t.monthly_eur - acc.plan.monthly_eur)
                ),
                None => format!("{} used of {}.", fmt_bytes(acc.used), fmt_bytes(acc.limit)),
            };
            f.push(Finding {
                id: format!("{p:?}-over"),
                severity: "critical".into(),
                title: format!("{name} is over its plan by {}", fmt_bytes(acc.over_by)),
                detail,
                bytes: acc.over_by,
                monthly_eur: 0.0,
                action: act("Manage plan", "open_url", manage.clone()),
            });
        } else if acc.status == "near" || acc.status == "critical" {
            let when = forecast.map(|d| format!(" At the current rate it is full in about {:.0} days.", d)).unwrap_or_default();
            f.push(Finding {
                id: format!("{p:?}-near"),
                severity: if acc.status == "critical" { "critical" } else { "warn" }.into(),
                title: format!("{name} is {:.0}% full", acc.used as f64 * 100.0 / acc.limit as f64),
                detail: format!("{} free of {}.{when}", fmt_bytes(acc.limit - acc.used), fmt_bytes(acc.limit)),
                bytes: 0,
                monthly_eur: 0.0,
                action: act("Manage plan", "open_url", manage.clone()),
            });
        } else if let Some(days) = forecast.filter(|d| *d < 120.0) {
            f.push(Finding {
                id: format!("{p:?}-forecast"),
                severity: "info".into(),
                title: format!("{name} fills up in about {:.0} days", days),
                detail: "Based on your daily usage trend.".into(),
                bytes: 0,
                monthly_eur: 0.0,
                action: None,
            });
        }

        // Paying for more than needed
        if acc.plan.monthly_eur > 0.0 && acc.status == "ok" {
            if let Some(t) = plans::cheapest_fit(p, acc.used, 1.25) {
                let save = acc.plan.monthly_eur - t.monthly_eur;
                if save >= 0.5 {
                    plan_savings.push(save);
                    f.push(Finding {
                        id: format!("{p:?}-downgrade"),
                        severity: "info".into(),
                        title: format!("Downgrade {name} to {} and save {}/mo", t.name, eur(save)),
                        detail: format!("You use {} of {}. {} still leaves 25% headroom.", fmt_bytes(acc.used), fmt_bytes(acc.limit), t.name),
                        bytes: 0,
                        monthly_eur: save,
                        action: act("Manage plan", "open_url", manage.clone()),
                    });
                }
            }
        }

        if acc.trash_bytes > 200_000_000 {
            f.push(Finding {
                id: format!("{p:?}-trash"),
                severity: "warn".into(),
                title: format!("Empty Google Drive trash ({})", fmt_bytes(acc.trash_bytes)),
                detail: "Trashed files still count against your Google storage.".into(),
                bytes: acc.trash_bytes,
                monthly_eur: 0.0,
                action: act("Open trash", "open_url", Some("https://drive.google.com/drive/trash".into())),
            });
        }

        let backups = slice(acc, "backup");
        if p == Provider::Icloud && acc.used > 0 && backups as f64 / acc.used as f64 > 0.2 && backups > 5 * GB {
            f.push(Finding {
                id: "icloud-backups".into(),
                severity: "info".into(),
                title: format!("Device backups use {} of iCloud", fmt_bytes(backups)),
                detail: "Delete backups of devices you no longer use: iPhone Settings > [your name] > iCloud > Manage Account Storage > Backups.".into(),
                bytes: backups,
                monthly_eur: 0.0,
                action: None,
            });
        }
    }

    // ---------------------------------------------------------- consolidation (both paid)
    if let (Some(ic), Some(go)) = (d.icloud.as_ref(), d.google.as_ref()) {
        let current = ic.plan.monthly_eur + go.plan.monthly_eur;
        let mut best: Option<(f64, String, String)> = None;
        // A) Move Google Drive files to iCloud, Google drops to free 15 GB (Gmail + Photos must fit).
        let go_drive = slice(go, "drive");
        let go_rest = go.used.saturating_sub(go_drive).saturating_sub(go.trash_bytes);
        if go.plan.monthly_eur > 0.0 && (go_rest as f64) < 15.0 * 1.0737e9 * 0.85 {
            if let Some(t) = plans::cheapest_fit(Provider::Icloud, ic.used + go_drive, 1.15) {
                let cost = t.monthly_eur;
                let save = current - cost;
                if save >= 0.5 {
                    best = Some((save, format!("Move {} of Google Drive files to iCloud Drive", fmt_bytes(go_drive)),
                        format!("Switch iCloud to {} ({}/mo) and Google to the free plan. Gmail + Photos ({}) still fit in the free 15 GB.", t.name, eur(cost), fmt_bytes(go_rest))));
                }
            }
        }
        // B) Move iCloud Drive docs to Google, iCloud keeps photos/backups/messages on a smaller tier.
        let ic_docs = slice(ic, "docs");
        let ic_rest = ic.used.saturating_sub(ic_docs);
        if ic.plan.monthly_eur > 0.0 {
            let ic_t = plans::cheapest_fit(Provider::Icloud, ic_rest, 1.15);
            let go_t = plans::cheapest_fit(Provider::Google, go.used + ic_docs, 1.15);
            if let (Some(a), Some(b)) = (ic_t, go_t) {
                let save = current - a.monthly_eur - b.monthly_eur;
                if save >= 0.5 && best.as_ref().map(|x| save > x.0).unwrap_or(true) {
                    best = Some((save, format!("Move {} of iCloud Drive files to Google Drive", fmt_bytes(ic_docs)),
                        format!("Then iCloud fits {} and Google fits {}.", a.name, b.name)));
                }
            }
        }
        if let Some((save, title, detail)) = best {
            plan_savings.push(save);
            f.push(Finding {
                id: "consolidate".into(),
                severity: "info".into(),
                title: format!("{title}: save {}/mo", eur(save)),
                detail: format!("You pay {}/mo for two clouds. {detail}", eur(current)),
                bytes: 0,
                monthly_eur: save,
                action: None,
            });
        }
    }

    // ---------------------------------------------------------- this device
    let mut reclaim_total = 0u64;
    if let Some(local) = d.local.as_ref() {
        for v in local.volumes.iter().filter(|v| v.total_bytes > 0) {
            let used = v.total_bytes - v.free_bytes.min(v.total_bytes);
            let pct = used as f64 / v.total_bytes as f64;
            if pct >= 0.9 {
                f.push(Finding {
                    id: format!("vol-{}", v.mount),
                    severity: if pct >= 0.95 { "critical" } else { "warn" }.into(),
                    title: format!("{} is {:.0}% full", v.mount, pct * 100.0),
                    detail: format!("{} free of {}.", fmt_bytes(v.free_bytes), fmt_bytes(v.total_bytes)),
                    bytes: 0,
                    monthly_eur: 0.0,
                    action: if cfg!(windows) { act("Storage settings", "open_url", Some("ms-settings:storagesense".into())) } else { None },
                });
            }
        }
        for r in &local.reclaimable {
            let (min, sev, action) = match r.id.as_str() {
                "recycle_bin" => (500_000_000, "warn", act("Empty Recycle Bin", "empty_recycle_bin", None)),
                "trash" => (500_000_000, "warn", act("Open Trash", "open_path", r.path.clone())),
                "temp" => (1_000_000_000, "info", act("Open folder", "open_path", r.path.clone())),
                "windows_old" => (1_000_000_000, "warn", act("Storage settings", "open_url", Some("ms-settings:storagesense".into()))),
                "xcode_derived" => (2 * GB, "info", act("Open folder", "open_path", r.path.clone())),
                "downloads" => (5 * GB, "info", act("Open folder", "open_path", r.path.clone())),
                _ => (GB, "info", None),
            };
            if r.bytes >= min {
                if r.id != "downloads" {
                    reclaim_total += r.bytes;
                }
                f.push(Finding {
                    id: format!("reclaim-{}", r.id),
                    severity: sev.into(),
                    title: format!("{}: {}", r.label, fmt_bytes(r.bytes)),
                    detail: if r.id == "downloads" { "Review and delete old installers and archives.".into() } else { "Safe to clear.".into() },
                    bytes: r.bytes,
                    monthly_eur: 0.0,
                    action,
                });
            }
        }
        let six_months_ago = (chrono::Local::now() - chrono::Duration::days(180)).format("%Y-%m-%d").to_string();
        for b in &local.ios_backups {
            let old = b.last_backup.as_deref().map(|d| d < six_months_ago.as_str()).unwrap_or(false);
            if old || b.bytes > 20 * GB {
                reclaim_total += b.bytes;
                f.push(Finding {
                    id: format!("iosbackup-{}", b.path),
                    severity: if old { "warn" } else { "info" }.into(),
                    title: format!("{} backup on this PC: {}", b.device_name, fmt_bytes(b.bytes)),
                    detail: format!(
                        "Last backup {}. {}",
                        b.last_backup.clone().unwrap_or_else(|| "unknown".into()),
                        if old { "Older than 6 months, likely safe to delete if iCloud Backup is on." } else { "Keep only the newest backup per device." }
                    ),
                    bytes: b.bytes,
                    monthly_eur: 0.0,
                    action: act("Show in folder", "open_path", Some(b.path.clone())),
                });
            }
        }
        for c in &local.sync_clients {
            if c.local_bytes > 10 * GB && matches!(c.id.as_str(), "icloud_drive" | "icloud_photos" | "onedrive" | "dropbox" | "google_drive") {
                f.push(Finding {
                    id: format!("sync-{}", c.id),
                    severity: "info".into(),
                    title: format!("{} keeps {} downloaded on this PC", c.name, fmt_bytes(c.local_bytes)),
                    detail: "Right-click folders you rarely open and choose \"Free up space\" (online-only). Files stay in the cloud.".into(),
                    bytes: c.local_bytes,
                    monthly_eur: 0.0,
                    action: act("Open folder", "open_path", c.sync_root.clone()),
                });
            }
            if c.cache_bytes > 5 * GB {
                f.push(Finding {
                    id: format!("cache-{}", c.id),
                    severity: "info".into(),
                    title: format!("{} cache uses {}", c.name, fmt_bytes(c.cache_bytes)),
                    detail: "Lower the cache or switch folders to streaming in the app's preferences.".into(),
                    bytes: c.cache_bytes,
                    monthly_eur: 0.0,
                    action: None,
                });
            }
        }
    }

    if !d.large_files.is_empty() {
        let total: u64 = d.large_files.iter().filter(|x| !x.cloud_only).map(|x| x.bytes).sum();
        f.push(Finding {
            id: "large-files".into(),
            severity: "info".into(),
            title: format!("{} large files hold {}", d.large_files.len(), fmt_bytes(total)),
            detail: "Review them on the This device tab. Move archives to an external drive.".into(),
            bytes: total,
            monthly_eur: 0.0,
            action: None,
        });
    }

    for dev in d.devices.iter().filter(|x| x.total_bytes > 0) {
        if (dev.free_bytes as f64) < dev.total_bytes as f64 * 0.1 {
            f.push(Finding {
                id: format!("dev-{}", dev.id),
                severity: "warn".into(),
                title: format!("{} has only {} free", dev.name, fmt_bytes(dev.free_bytes)),
                detail: format!("{} total. Turn on Optimize Storage for Photos and offload unused apps.", fmt_bytes(dev.total_bytes)),
                bytes: 0,
                monthly_eur: 0.0,
                action: None,
            });
        }
    }

    let rank = |s: &str| match s {
        "critical" => 0,
        "warn" => 1,
        _ => 2,
    };
    f.sort_by(|a, b| rank(&a.severity).cmp(&rank(&b.severity)).then(b.monthly_eur.partial_cmp(&a.monthly_eur).unwrap()).then(b.bytes.cmp(&a.bytes)));

    // ---------------------------------------------------------- totals
    let clouds: Vec<&CloudAccount> = [d.icloud.as_ref(), d.google.as_ref()].into_iter().flatten().collect();
    let (lu, lc) = d
        .this_device
        .as_ref()
        .map(|t| (t.total_bytes.saturating_sub(t.free_bytes), t.total_bytes))
        .unwrap_or((0, 0));
    d.totals = Totals {
        cloud_used: clouds.iter().map(|c| c.used).sum(),
        cloud_capacity: clouds.iter().map(|c| c.limit).sum(),
        local_used: lu,
        local_capacity: lc,
        monthly_spend_eur: clouds.iter().map(|c| c.plan.monthly_eur).sum(),
        reclaimable_bytes: reclaim_total + clouds.iter().map(|c| c.trash_bytes).sum::<u64>(),
        // downgrade and consolidation are alternatives: count the best one
        potential_savings_eur: plan_savings.iter().cloned().fold(0.0, f64::max),
    };
    d.findings = f;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(p: Provider, used: u64, limit: u64, plan: &str, eur: f64, breakdown: Vec<(&str, u64)>) -> CloudAccount {
        CloudAccount {
            provider: p,
            account_label: "t".into(),
            used,
            limit,
            plan: PlanMatch { id: plan.into(), name: plan.into(), bytes: limit, monthly_eur: eur, price_verified: true, exact: true },
            status: status_for(used, limit, false),
            over_by: used.saturating_sub(limit),
            breakdown: breakdown.into_iter().map(|(k, b)| UsageSlice { key: k.into(), label: k.into(), bytes: b, color: None }).collect(),
            trash_bytes: 0,
            family_shared: false,
            fetched_at: String::new(),
            note: None,
        }
    }

    #[test]
    fn over_plan_and_downgrade_and_consolidation() {
        let gib = 1024u64.pow(3);
        let mut d = Dashboard::default();
        // iCloud 2 TB with only 150 GB used -> downgrade to 200 GB
        d.icloud = Some(acc(Provider::Icloud, 150 * gib, 2000 * gib, "icloud-2t", 9.99, vec![("photos", 120 * gib), ("docs", 30 * gib)]));
        // Google 100 GB paid, Drive 40 GB, Gmail+Photos 5 GB -> can drop to free if Drive moves to iCloud
        d.google = Some(acc(Provider::Google, 45 * gib, 100 * gib, "google-100", 1.99, vec![("drive", 40 * gib), ("gmail_photos", 5 * gib)]));
        recompute(&mut d);
        let ids: Vec<&str> = d.findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"Icloud-downgrade"), "{ids:?}");
        assert!(ids.contains(&"consolidate"), "{ids:?}");
        assert!(d.totals.potential_savings_eur > 6.0);
        assert!((d.totals.monthly_spend_eur - 11.98).abs() < 0.01);

        // over quota
        d.icloud = Some(acc(Provider::Icloud, 230 * gib, 200 * gib, "icloud-200", 2.99, vec![]));
        recompute(&mut d);
        assert_eq!(d.findings[0].severity, "critical");
        assert!(d.findings[0].title.contains("over its plan"));
    }

    #[test]
    fn forecast() {
        let pts: Vec<(f64, f64)> = (0..30).map(|i| (i as f64, 100e9 + i as f64 * 1e9)).collect();
        let days = days_until_full(&pts, 130e9 as u64, 160e9 as u64).unwrap();
        assert!((days - 30.0).abs() < 0.5);
    }
}
