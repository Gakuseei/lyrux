#![allow(dead_code)]

use gtk4 as gtk;
use sourceview5::prelude::*;

use crate::editor::themes;

#[derive(Clone)]
pub struct ViewConfig {
    pub theme_id: String,
    pub font_family: String,
    pub font_size: i32,
    pub tab_width: u32,
    pub insert_spaces: bool,
    pub show_line_numbers: bool,
    pub show_whitespace: bool,
    pub wrap_lines: bool,
    pub auto_indent: bool,
    pub highlight_current_line: bool,
    pub highlight_matching_brackets: bool,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            theme_id: themes::default_id().to_string(),
            font_family: "monospace".into(),
            font_size: 13,
            tab_width: 4,
            insert_spaces: true,
            show_line_numbers: true,
            show_whitespace: false,
            wrap_lines: false,
            auto_indent: true,
            highlight_current_line: true,
            highlight_matching_brackets: true,
        }
    }
}

pub fn build(buffer: &sourceview5::Buffer, cfg: &ViewConfig) -> sourceview5::View {
    let view = sourceview5::View::with_buffer(buffer);
    apply_to_view(&view, cfg);
    apply_to_buffer(buffer, cfg);
    view
}

pub fn apply_to_view(view: &sourceview5::View, cfg: &ViewConfig) {
    view.set_show_line_numbers(cfg.show_line_numbers);
    view.set_highlight_current_line(cfg.highlight_current_line);
    view.set_tab_width(cfg.tab_width);
    view.set_insert_spaces_instead_of_tabs(cfg.insert_spaces);
    view.set_auto_indent(cfg.auto_indent);
    view.set_indent_on_tab(true);
    view.set_show_line_marks(true);
    view.set_monospace(true);
    if cfg.wrap_lines {
        view.set_wrap_mode(gtk::WrapMode::WordChar);
    } else {
        view.set_wrap_mode(gtk::WrapMode::None);
    }
    let mut whitespace = sourceview5::SpaceTypeFlags::empty();
    if cfg.show_whitespace {
        whitespace |= sourceview5::SpaceTypeFlags::SPACE
            | sourceview5::SpaceTypeFlags::TAB
            | sourceview5::SpaceTypeFlags::NEWLINE;
    }
    let drawer = view.space_drawer();
    drawer.set_types_for_locations(sourceview5::SpaceLocationFlags::ALL, whitespace);
    drawer.set_enable_matrix(cfg.show_whitespace);
    let css = format!(
        ".sourceview, .sourceview text {{ font-family: \"{}\", monospace; font-size: {}pt; }}",
        cfg.font_family.replace('"', ""),
        cfg.font_size
    );
    let provider = gtk::CssProvider::new();
    provider.load_from_data(&css);
    view.add_css_class("sourceview");
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

pub fn apply_to_buffer(buffer: &sourceview5::Buffer, cfg: &ViewConfig) {
    buffer.set_highlight_matching_brackets(cfg.highlight_matching_brackets);
    let manager = sourceview5::StyleSchemeManager::default();
    themes::register_all(&manager);
    if let Some(scheme) = manager.scheme(&cfg.theme_id) {
        buffer.set_style_scheme(Some(&scheme));
    }
}
