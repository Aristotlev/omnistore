//! Directory sizing that understands cloud placeholders (OneDrive / iCloud / Dropbox
//! Files On-Demand on Windows, dataless files on macOS). Reading metadata never
//! downloads a file.

use crate::model::FileEntry;
use std::path::Path;
use std::time::{Duration, Instant};
use walkdir::WalkDir;

#[derive(Debug, Default, Clone, Copy)]
pub struct DirSize {
    pub local_bytes: u64,
    pub cloud_only_bytes: u64,
    pub files: u64,
}

#[cfg(windows)]
pub fn is_cloud_only(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const OFFLINE: u32 = 0x0000_1000;
    const RECALL_ON_OPEN: u32 = 0x0004_0000;
    const RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
    meta.file_attributes() & (OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS) != 0
}

#[cfg(target_os = "macos")]
pub fn is_cloud_only(meta: &std::fs::Metadata) -> bool {
    use std::os::macos::fs::MetadataExt;
    const SF_DATALESS: u32 = 0x4000_0000;
    meta.st_flags() & SF_DATALESS != 0
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn is_cloud_only(_meta: &std::fs::Metadata) -> bool {
    false
}

/// Walks `root` without following links or junctions. Stops after `budget`.
pub fn dir_size(root: &Path, budget: Duration) -> DirSize {
    let mut out = DirSize::default();
    if !root.exists() {
        return out;
    }
    let started = Instant::now();
    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            out.files += 1;
            if is_cloud_only(&meta) {
                out.cloud_only_bytes += meta.len();
            } else {
                out.local_bytes += meta.len();
            }
        }
        if out.files % 4096 == 0 && started.elapsed() > budget {
            break;
        }
    }
    out
}

/// Folders that are huge, system-owned, or already measured elsewhere.
fn skip_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | ".git" | "$Recycle.Bin" | "System Volume Information" | "WindowsApps" | "WinSxS"
    )
}

/// Largest files under `root`, biggest first.
pub fn large_files(root: &Path, min_bytes: u64, limit: usize, budget: Duration) -> Vec<FileEntry> {
    let started = Instant::now();
    let mut found: Vec<FileEntry> = Vec::new();
    let mut seen: u64 = 0;
    let walker = WalkDir::new(root).follow_links(false).into_iter().filter_entry(|e| {
        !(e.file_type().is_dir() && e.file_name().to_str().map(skip_dir).unwrap_or(false))
    });
    for entry in walker.filter_map(Result::ok) {
        seen += 1;
        if seen % 4096 == 0 && started.elapsed() > budget {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.len() < min_bytes {
            continue;
        }
        let modified = meta
            .modified()
            .ok()
            .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d").to_string());
        found.push(FileEntry {
            path: entry.path().display().to_string(),
            bytes: meta.len(),
            modified,
            cloud_only: is_cloud_only(&meta),
        });
    }
    found.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    found.truncate(limit);
    found
}
