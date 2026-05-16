use serde::{Deserialize, Serialize};

use crate::editor::view::ViewConfig;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EditorSettings {
    #[serde(default = "default_theme")]
    pub theme_id: String,
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
    #[serde(default = "default_true")]
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
    #[serde(default)]
    pub vim_mode: bool,
}

fn default_theme() -> String {
    "lyrux-dark".to_string()
}
fn default_font_family() -> String {
    "JetBrains Mono".to_string()
}
fn default_font_size() -> i32 {
    13
}
fn default_tab_width() -> u32 {
    4
}
fn default_true() -> bool {
    true
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            theme_id: default_theme(),
            font_family: default_font_family(),
            font_size: default_font_size(),
            tab_width: default_tab_width(),
            insert_spaces: true,
            show_line_numbers: true,
            show_whitespace: false,
            wrap_lines: true,
            auto_indent: true,
            highlight_current_line: true,
            highlight_matching_brackets: true,
            strip_trailing_whitespace: true,
            ensure_final_newline: true,
            show_indent_guides: true,
            highlight_word_at_cursor: true,
            vim_mode: false,
        }
    }
}

impl EditorSettings {
    pub fn to_view_config(&self) -> ViewConfig {
        ViewConfig {
            theme_id: self.theme_id.clone(),
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
        }
    }
}
