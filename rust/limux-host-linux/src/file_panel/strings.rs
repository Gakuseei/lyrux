pub const PROMPT_NEW_FILE_TITLE: &str = "New File";
pub const PROMPT_NEW_FOLDER_TITLE: &str = "New Folder";
pub const PROMPT_RENAME_TITLE: &str = "Rename";
pub const PROMPT_PLACEHOLDER_FILE: &str = "filename.ext";
pub const PROMPT_PLACEHOLDER_FOLDER: &str = "folder-name";
pub const DIALOG_BTN_CREATE: &str = "Create";
pub const DIALOG_BTN_RENAME: &str = "Rename";
pub const DIALOG_BTN_CANCEL: &str = "Cancel";

#[allow(dead_code)]
pub const SETTING_FP_SHOW_SIZE: &str = "Show file sizes";
#[allow(dead_code)]
pub const SETTING_FP_SHOW_MTIME: &str = "Show modified time";
#[allow(dead_code)]
pub const STATUS_RELATIVE_NOW: &str = "now";

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
}
