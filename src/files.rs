use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct NodeModulesEntry {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub size_bytes: u64,
}

fn dir_size(path: &Path) -> io::Result<u64> {
    let size: u64 = jwalk::WalkDir::new(path)
        .skip_hidden(false) // Ensure we count .bin and .cache folders
        .into_iter()
        .filter_map(|result| result.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.metadata().map(|m| m.len()).unwrap_or(0))
        .sum();

    Ok(size)
}

pub fn format_size(size_bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size = size_bytes as f64;

    match size {
        s if s >= GB => format!("{:.2} GB", s / GB),
        s if s >= MB => format!("{:.2} MB", s / MB),
        s if s >= KB => format!("{:.2} KB", s / KB),
        _ => format!("{} B", size_bytes),
    }
}

pub fn format_modified(time: SystemTime) -> String {
    let now = SystemTime::now();
    match now.duration_since(time) {
        Ok(delta) => humanize_duration(delta),
        Err(_) => "back to the future?".into(),
    }
}

fn humanize_duration(delta: Duration) -> String {
    let secs = delta.as_secs();
    let days = secs / 86400;
    let hours = secs / 3600;
    let mins = secs / 60;

    match (days, hours, mins) {
        (1, _, _) => "1 day ago".into(),
        (2.., _, _) => format!("{days} days ago"),
        (_, 1, _) => "1 hour ago".into(),
        (_, 2.., _) => format!("{hours} hours ago"),
        (_, _, 1..) => format!("{mins} min ago"),
        _ => "just now".into(),
    }
}

pub fn scan_node_modules(root: PathBuf, tx: Sender<NodeModulesEntry>) {
    let mut it = WalkDir::new(root).into_iter();

    while let Some(entry) = it.next() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.file_type().is_dir() && entry.file_name().to_string_lossy().starts_with('.') {
            it.skip_current_dir();
            continue;
        }

        if entry.file_type().is_dir() && entry.file_name() == "node_modules" {
            let path = entry.path().to_path_buf();
            let tx = tx.clone();

            rayon::spawn(move || {
                if let Ok(metadata) = fs::metadata(&path) {
                    let modified = metadata.modified().unwrap_or(UNIX_EPOCH);

                    if let Ok(size_bytes) = dir_size(&path) {
                        let entry = NodeModulesEntry {
                            path,
                            modified,
                            size_bytes,
                        };

                        let _ = tx.send(entry);
                    }
                }
            });

            it.skip_current_dir();
        }
    }
}
