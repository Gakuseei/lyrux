#![allow(dead_code)]

pub const SECTION_EDITOR: &str = "Editor";
pub const SETTING_THEME: &str = "Theme";
pub const SETTING_FONT: &str = "Font family";
pub const SETTING_FONT_SIZE: &str = "Font size";
pub const SETTING_TAB_WIDTH: &str = "Tab width";
pub const SETTING_INSERT_SPACES: &str = "Insert spaces (not tabs)";
pub const SETTING_LINE_NUMBERS: &str = "Show line numbers";
pub const SETTING_SHOW_WHITESPACE: &str = "Show whitespace";
pub const SETTING_WRAP_LINES: &str = "Wrap long lines";
pub const SETTING_AUTO_INDENT: &str = "Auto-indent";
pub const SETTING_HIGHLIGHT_LINE: &str = "Highlight current line";
pub const SETTING_HIGHLIGHT_BRACKETS: &str = "Highlight matching brackets";
pub const SETTING_VIM: &str = "Vim mode (coming soon)";

pub const DIALOG_UNSAVED_TITLE: &str = "Unsaved changes";
pub const DIALOG_UNSAVED_BODY_PREFIX: &str = "Save changes to ";
pub const DIALOG_UNSAVED_BODY_SUFFIX: &str = " before closing?";
pub const DIALOG_BTN_SAVE: &str = "Save";
pub const DIALOG_BTN_DISCARD: &str = "Discard";
pub const DIALOG_BTN_CANCEL: &str = "Cancel";

pub const BANNER_FILE_CHANGED_PREFIX: &str = "File changed on disk: ";
pub const BANNER_FILE_DELETED_PREFIX: &str = "File deleted on disk: ";
pub const BANNER_RELOAD: &str = "Reload";
pub const BANNER_KEEP_MINE: &str = "Keep mine";
pub const BANNER_SAVE_AS_NEW: &str = "Save";
pub const BANNER_CLOSE_TAB: &str = "Close tab";

pub const TOAST_RELOADED_PREFIX: &str = "Reloaded ";

pub const ERROR_FILE_TOO_LARGE: &str = "File is larger than 10 MB; refusing to open.";
pub const ERROR_FILE_BINARY: &str = "Binary file; use the image viewer or a hex tool.";
pub const ERROR_OUTSIDE_WORKSPACE: &str = "Refusing to save outside the workspace root.";
pub const ERROR_WRITE_FAILED_PREFIX: &str = "Save failed: ";

pub const THEME_LYRUX_DARK: &str = "Lyrux Dark";
pub const THEME_LYRUX_LIGHT: &str = "Lyrux Light";
pub const THEME_CATPPUCCIN_LATTE: &str = "Catppuccin Latte";
pub const THEME_CATPPUCCIN_FRAPPE: &str = "Catppuccin Frappé";
pub const THEME_CATPPUCCIN_MACCHIATO: &str = "Catppuccin Macchiato";
pub const THEME_CATPPUCCIN_MOCHA: &str = "Catppuccin Mocha";
pub const THEME_TOKYO_NIGHT: &str = "Tokyo Night";
pub const THEME_TOKYO_NIGHT_STORM: &str = "Tokyo Night Storm";
pub const THEME_ONE_DARK: &str = "One Dark";
pub const THEME_ONE_LIGHT: &str = "One Light";

pub const ICON_EDITOR_TOOLTIP: &str = "New editor tab";
pub const TAB_TITLE_UNTITLED: &str = "untitled";
pub const RECOVER_PROMPT_PREFIX: &str = "Recover unsaved changes in ";
pub const RECOVER_BTN_RECOVER: &str = "Recover";
pub const RECOVER_BTN_DISCARD: &str = "Discard";
