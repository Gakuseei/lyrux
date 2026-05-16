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
pub const SETTING_STRIP_WS: &str = "Strip trailing whitespace on save";
pub const SETTING_FINAL_NEWLINE: &str = "Ensure final newline on save";
pub const SETTING_INDENT_GUIDES: &str = "Show indent guides";
pub const SETTING_HIGHLIGHT_WORD: &str = "Highlight matching word at cursor";
pub const SETTING_STICKY_SCROLL: &str = "Sticky scroll (pin function/class headers)";
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
pub const BANNER_KEEP_MINE: &str = "Keep my version";
pub const BANNER_SAVE_AS_NEW: &str = "Save";
pub const BANNER_CLOSE_TAB: &str = "Close tab";
pub const BANNER_DISMISS: &str = "Dismiss";
pub const SAVE_AS_DIALOG_TITLE: &str = "Save As";
pub const SAVE_AS_BINARY_WARN_BODY: &str =
    "This filename looks like an image or binary file. Save text content anyway?";
pub const SAVE_AS_BINARY_WARN_PROCEED: &str = "Save anyway";

pub const TOAST_RELOADED_PREFIX: &str = "Reloaded ";

pub const ERROR_FILE_TOO_LARGE: &str = "File too large to open (over 10 MB).";
pub const ERROR_FILE_BINARY: &str = "Binary file; use the image viewer or a hex tool.";
pub const ERROR_OUTSIDE_WORKSPACE: &str = "Can't save here — file is outside the workspace.";
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
pub const TAB_TITLE_UNTITLED: &str = "Untitled";
pub const RECOVER_PROMPT_PREFIX: &str = "Recover unsaved changes in ";
pub const RECOVER_BTN_RECOVER: &str = "Recover";
pub const RECOVER_BTN_DISCARD: &str = "Discard";

pub const FIND_PLACEHOLDER: &str = "Find";
pub const REPLACE_PLACEHOLDER: &str = "Replace";
pub const GOTO_LINE_TITLE: &str = "Go to line";
pub const GOTO_LINE_PROMPT: &str = "Line number";
pub const REPLACE_BTN: &str = "Replace";
pub const REPLACE_ALL_BTN: &str = "Replace all";
pub const GOTO_BTN: &str = "Go";

pub const SHORTCUT_LABEL_EDITOR_TOGGLE_PANE: &str = "Open editor in focused pane";

pub const QUICK_OPEN_TITLE: &str = "Open File";
pub const QUICK_OPEN_PLACEHOLDER: &str = "Search workspace files…";
pub const QUICK_OPEN_EMPTY: &str = "No matches.";
pub const SHORTCUT_LABEL_EDITOR_QUICK_OPEN: &str = "Quick-open file";

pub const FIF_TITLE: &str = "Find in Files";
pub const FIF_PLACEHOLDER: &str = "Find in files…";
pub const FIF_NO_RG: &str = "ripgrep (rg) not installed.";
pub const FIF_NO_MATCHES: &str = "No matches.";
pub const FIF_NO_ROOT: &str = "No workspace folder is open.";
pub const FIF_RG_FAILED_PREFIX: &str = "ripgrep failed: ";
pub const SHORTCUT_LABEL_EDITOR_FIND_IN_FILES: &str = "Find in files";

pub fn fif_results_label(matches: usize, files: usize) -> String {
    format!("{matches} matches in {files} files")
}

pub const CMD_PALETTE_TITLE: &str = "Command Palette";
pub const CMD_PALETTE_PLACEHOLDER: &str = "Search commands…";
pub const CMD_PALETTE_EMPTY: &str = "No matching commands.";
pub const SHORTCUT_LABEL_EDITOR_COMMAND_PALETTE: &str = "Command palette";

pub const CMD_NEW_WORKSPACE: &str = "Workspace: New";
pub const CMD_CLOSE_WORKSPACE: &str = "Workspace: Close";
pub const CMD_NEXT_WORKSPACE: &str = "Workspace: Next";
pub const CMD_PREV_WORKSPACE: &str = "Workspace: Previous";
pub const CMD_SPLIT_RIGHT: &str = "Pane: Split Right";
pub const CMD_SPLIT_DOWN: &str = "Pane: Split Down";
pub const CMD_CLOSE_FOCUSED_PANE: &str = "Pane: Close Focused";
pub const CMD_FOCUS_LEFT: &str = "Pane: Focus Left";
pub const CMD_FOCUS_RIGHT: &str = "Pane: Focus Right";
pub const CMD_FOCUS_UP: &str = "Pane: Focus Up";
pub const CMD_FOCUS_DOWN: &str = "Pane: Focus Down";
pub const CMD_CYCLE_TAB_NEXT: &str = "Tab: Next";
pub const CMD_CYCLE_TAB_PREV: &str = "Tab: Previous";
pub const CMD_TOGGLE_SIDEBAR: &str = "View: Toggle Sidebar";
pub const CMD_TOGGLE_FILE_PANEL: &str = "View: Toggle File Panel";
pub const CMD_TOGGLE_TOP_BAR: &str = "View: Toggle Top Bar";
pub const CMD_TOGGLE_FULLSCREEN: &str = "View: Toggle Fullscreen";
pub const CMD_EDITOR_TOGGLE_CURRENT_PANE: &str = "Editor: Open in Focused Pane";
pub const CMD_EDITOR_QUICK_OPEN: &str = "Editor: Quick Open File";
pub const CMD_NEW_TERMINAL: &str = "Terminal: New";
pub const CMD_NEW_TERMINAL_IN_PANE: &str = "Terminal: New in Focused Pane";
pub const CMD_OPEN_BROWSER_IN_SPLIT: &str = "Browser: Open in Split";
pub const CMD_QUIT_APP: &str = "Application: Quit";

pub const STATUS_LINE_PREFIX: &str = "Ln ";
pub const STATUS_COL_PREFIX: &str = "Col ";
pub const STATUS_SPACES: &str = "Spaces:";
pub const STATUS_TAB_WIDTH: &str = "Tab Width:";
pub const STATUS_ENCODING_UTF8: &str = "UTF-8";
pub const STATUS_EOL_LF: &str = "LF";
pub const STATUS_LANG_PLAIN_TEXT: &str = "Plain Text";
