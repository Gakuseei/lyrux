pub const SORT_MENU_TOOLTIP: &str = "Sort by";
pub const SORT_NAME_ASC: &str = "Name (A-Z)";
pub const SORT_NAME_DESC: &str = "Name (Z-A)";
pub const SORT_MODIFIED_DESC: &str = "Modified (newest)";
pub const SORT_SIZE_DESC: &str = "Size (largest)";
pub const SORT_FOLDERS_FIRST: &str = "Folders first, name";

pub const PROMPT_NEW_FILE_TITLE: &str = "New File";
pub const PROMPT_NEW_FOLDER_TITLE: &str = "New Folder";
pub const PROMPT_RENAME_TITLE: &str = "Rename";
pub const PROMPT_PLACEHOLDER_FILE: &str = "filename.ext";
pub const PROMPT_PLACEHOLDER_FOLDER: &str = "folder-name";
pub const DIALOG_BTN_CREATE: &str = "Create";
pub const DIALOG_BTN_RENAME: &str = "Rename";
pub const DIALOG_BTN_CANCEL: &str = "Cancel";

pub const SETTING_FP_SHOW_SIZE: &str = "Show file sizes";
pub const SETTING_FP_SHOW_MTIME: &str = "Show modified time";
pub const SETTINGS_SECTION_FILE_PANEL: &str = "File Panel";
pub const STATUS_RELATIVE_NOW: &str = "now";

pub fn relative_minutes(m: i64) -> String {
    format!("{m}m")
}

pub fn relative_hours(h: i64) -> String {
    format!("{h}h")
}

pub fn relative_days(d: i64) -> String {
    format!("{d}d")
}

pub fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    if bytes < KB {
        return format!("{bytes}B");
    }
    if bytes < MB {
        return format!("{:.0}K", bytes as f64 / KB as f64);
    }
    if bytes < GB {
        return format!("{:.1}M", bytes as f64 / MB as f64);
    }
    if bytes < TB {
        return format!("{:.1}G", bytes as f64 / GB as f64);
    }
    format!("{:.1}T", bytes as f64 / TB as f64)
}

pub fn relative_time(mtime: i64, now: i64) -> String {
    if mtime <= 0 {
        return String::new();
    }
    let diff = now.saturating_sub(mtime);
    if diff < 60 {
        return STATUS_RELATIVE_NOW.to_string();
    }
    if diff < 3600 {
        return relative_minutes(diff / 60);
    }
    if diff < 86_400 {
        return relative_hours(diff / 3600);
    }
    if diff < 86_400 * 365 {
        return relative_days(diff / 86_400);
    }
    relative_days(diff / 86_400)
}

pub fn split_basename_ext(name: &str) -> (usize, usize) {
    if let Some(idx) = name.rfind('.') {
        if idx > 0 && idx < name.len() {
            return (idx, name.len());
        }
    }
    (name.len(), name.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_keeps_full_for_no_ext() {
        let (sel, _) = split_basename_ext("README");
        assert_eq!(sel, "README".len());
    }

    #[test]
    fn split_excludes_extension() {
        let (sel, _) = split_basename_ext("main.rs");
        assert_eq!(sel, 4);
    }

    #[test]
    fn split_keeps_full_for_hidden() {
        let (sel, _) = split_basename_ext(".gitignore");
        assert_eq!(sel, ".gitignore".len());
    }

    #[test]
    fn human_size_bytes_under_kb() {
        assert_eq!(human_size(0), "0B");
        assert_eq!(human_size(512), "512B");
    }

    #[test]
    fn human_size_kb_rounds() {
        assert_eq!(human_size(1024), "1K");
        assert_eq!(human_size(2048), "2K");
    }

    #[test]
    fn human_size_mb_with_one_decimal() {
        assert_eq!(human_size(1024 * 1024), "1.0M");
    }

    #[test]
    fn human_size_gb_with_one_decimal() {
        assert_eq!(human_size(1024_u64.pow(3)), "1.0G");
    }

    #[test]
    fn relative_time_now_under_60_sec() {
        assert_eq!(relative_time(100, 110), "now");
    }

    #[test]
    fn relative_time_minutes() {
        assert_eq!(relative_time(1, 301), "5m");
    }

    #[test]
    fn relative_time_hours() {
        assert_eq!(relative_time(1, 1 + 3600 * 5), "5h");
    }

    #[test]
    fn relative_time_days() {
        assert_eq!(relative_time(1, 1 + 86_400 * 3), "3d");
    }

    #[test]
    fn relative_time_empty_for_zero_mtime() {
        assert_eq!(relative_time(0, 1000), "");
    }
}
