use serde::{Deserialize, Serialize};

use crate::editor::themes;
use crate::editor::view::ViewConfig;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    System,
    #[default]
    Manual,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EditorSettings {
    #[serde(default)]
    pub theme_mode: ThemeMode,
    #[serde(default = "default_theme")]
    pub theme_id: String,
    #[serde(default = "default_theme_dark")]
    pub theme_id_dark: String,
    #[serde(default = "default_theme_light")]
    pub theme_id_light: String,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: i32,
    #[serde(default = "default_tab_width")]
    pub tab_width: u32,
    #[serde(default = "default_true")]
    pub insert_spaces: bool,
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    #[serde(default)]
    pub show_whitespace: bool,
    #[serde(default = "default_wrap_lines")]
    pub wrap_lines: bool,
    #[serde(default = "default_true")]
    pub auto_indent: bool,
    #[serde(default = "default_true")]
    pub highlight_current_line: bool,
    #[serde(default = "default_true")]
    pub highlight_matching_brackets: bool,
    #[serde(default = "default_true")]
    pub strip_trailing_whitespace: bool,
    #[serde(default = "default_true")]
    pub ensure_final_newline: bool,
    #[serde(default = "default_true")]
    pub show_indent_guides: bool,
    #[serde(default = "default_true")]
    pub highlight_word_at_cursor: bool,
    #[serde(default = "default_true")]
    pub show_sticky_scroll: bool,
    #[serde(default = "default_true")]
    pub show_minimap: bool,
    #[serde(default)]
    pub vim_mode: bool,
    #[serde(default)]
    pub fp_show_size: bool,
    #[serde(default)]
    pub fp_show_mtime: bool,
}

fn default_theme() -> String {
    themes::default_id().to_string()
}
fn default_theme_dark() -> String {
    themes::default_dark_id().to_string()
}
fn default_theme_light() -> String {
    themes::default_light_id().to_string()
}
fn default_font_family() -> String {
    "Lilex".to_string()
}
fn default_font_size() -> i32 {
    12
}
fn default_tab_width() -> u32 {
    4
}
fn default_true() -> bool {
    true
}
fn default_wrap_lines() -> bool {
    false
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::default(),
            theme_id: default_theme(),
            theme_id_dark: default_theme_dark(),
            theme_id_light: default_theme_light(),
            font_family: default_font_family(),
            font_size: default_font_size(),
            tab_width: default_tab_width(),
            insert_spaces: true,
            show_line_numbers: true,
            show_whitespace: false,
            wrap_lines: false,
            auto_indent: true,
            highlight_current_line: true,
            highlight_matching_brackets: true,
            strip_trailing_whitespace: true,
            ensure_final_newline: true,
            show_indent_guides: true,
            highlight_word_at_cursor: true,
            show_sticky_scroll: true,
            show_minimap: true,
            vim_mode: false,
            fp_show_size: false,
            fp_show_mtime: false,
        }
    }
}

impl EditorSettings {
    pub fn effective_theme_id(&self, system_prefers_dark: Option<bool>) -> String {
        match self.theme_mode {
            ThemeMode::Manual => self.theme_id.clone(),
            ThemeMode::System => {
                if system_prefers_dark.unwrap_or(true) {
                    self.theme_id_dark.clone()
                } else {
                    self.theme_id_light.clone()
                }
            }
        }
    }

    pub fn to_view_config(&self) -> ViewConfig {
        self.to_view_config_with_system_pref(None)
    }

    pub fn to_view_config_with_system_pref(&self, system_prefers_dark: Option<bool>) -> ViewConfig {
        ViewConfig {
            theme_id: self.effective_theme_id(system_prefers_dark),
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            tab_width: self.tab_width,
            insert_spaces: self.insert_spaces,
            show_line_numbers: self.show_line_numbers,
            show_whitespace: self.show_whitespace,
            wrap_lines: self.wrap_lines,
            auto_indent: self.auto_indent,
            highlight_current_line: self.highlight_current_line,
            highlight_matching_brackets: self.highlight_matching_brackets,
            show_indent_guides: self.show_indent_guides,
            highlight_word_at_cursor: self.highlight_word_at_cursor,
            show_sticky_scroll: self.show_sticky_scroll,
            show_minimap: self.show_minimap,
            vim_mode: self.vim_mode,
        }
    }
}
