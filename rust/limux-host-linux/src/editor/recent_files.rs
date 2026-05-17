use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CONFIG_DIR_NAME: &str = "limux";
pub const RECENT_FILES_FILE_NAME: &str = "recent-files.json";
pub const MAX_RECENT_FILES: usize = 50;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RecentFiles {
    #[serde(default)]
    pub entries: Vec<PathBuf>,
}

impl RecentFiles {
    pub fn push(&mut self, path: PathBuf) {
        self.entries.retain(|p| p != &path);
        self.entries.insert(0, path);
        self.entries.truncate(MAX_RECENT_FILES);
    }

    #[allow(dead_code)]
    pub fn top(&self, n: usize) -> Vec<PathBuf> {
        self.entries.iter().take(n).cloned().collect()
    }
}

pub fn recent_files_path() -> Option<PathBuf> {
    dirs::config_dir().map(|base| base.join(CONFIG_DIR_NAME).join(RECENT_FILES_FILE_NAME))
}

pub fn load() -> RecentFiles {
    let Some(path) = recent_files_path() else {
        return RecentFiles::default();
    };
    load_from_path(&path)
}

pub fn load_from_path(path: &Path) -> RecentFiles {
    if !path.exists() {
        return RecentFiles::default();
    }
    let Ok(raw) = fs::read_to_string(path) else {
        return RecentFiles::default();
    };
    serde_json::from_str::<RecentFiles>(&raw).unwrap_or_default()
}

pub fn save(recent: &RecentFiles) -> Result<(), String> {
    let Some(path) = recent_files_path() else {
        return Err("config_dir unavailable; cannot save recent files".to_string());
    };
    save_to_path(&path, recent)
}

pub fn save_to_path(path: &Path, recent: &RecentFiles) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let serialized = serde_json::to_string_pretty(recent).map_err(|err| err.to_string())?;
    fs::write(path, format!("{serialized}\n")).map_err(|err| err.to_string())
}

pub fn push_path(path: &Path) {
    let mut recent = load();
    recent.push(path.to_path_buf());
    if let Err(err) = save(&recent) {
        eprintln!("lyrux: failed to save recent files: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn push_moves_existing_to_front_and_dedupes() {
        let mut recent = RecentFiles::default();
        recent.push(PathBuf::from("/a/one.rs"));
        recent.push(PathBuf::from("/a/two.rs"));
        recent.push(PathBuf::from("/a/one.rs"));
        assert_eq!(recent.entries.len(), 2);
        assert_eq!(recent.entries[0], PathBuf::from("/a/one.rs"));
        assert_eq!(recent.entries[1], PathBuf::from("/a/two.rs"));
    }

    #[test]
    fn push_caps_at_max_limit() {
        let mut recent = RecentFiles::default();
        for i in 0..(MAX_RECENT_FILES + 10) {
            recent.push(PathBuf::from(format!("/a/file{i}.rs")));
        }
        assert_eq!(recent.entries.len(), MAX_RECENT_FILES);
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("recent-files.json");
        let mut recent = RecentFiles::default();
        recent.push(PathBuf::from("/a/one.rs"));
        recent.push(PathBuf::from("/a/two.rs"));
        save_to_path(&path, &recent).expect("save");
        let loaded = load_from_path(&path);
        assert_eq!(loaded.entries, recent.entries);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("missing.json");
        let loaded = load_from_path(&path);
        assert!(loaded.entries.is_empty());
    }

    #[test]
    fn top_returns_first_n_entries() {
        let mut recent = RecentFiles::default();
        for i in 0..5 {
            recent.push(PathBuf::from(format!("/a/file{i}.rs")));
        }
        let top = recent.top(3);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0], PathBuf::from("/a/file4.rs"));
    }
}
