#![allow(dead_code)]

use std::cell::RefCell;

use gtk4 as gtk;
use sourceview5::prelude::*;

use crate::editor::snippets;
use crate::editor::strings;
use crate::editor::themes;

thread_local! {
    static EDITOR_FONT_PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

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
    pub show_indent_guides: bool,
    pub highlight_word_at_cursor: bool,
    pub show_sticky_scroll: bool,
    pub show_minimap: bool,
    pub vim_mode: bool,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            theme_id: themes::default_id().to_string(),
            font_family: "Lilex".into(),
            font_size: 12,
            tab_width: 4,
            insert_spaces: true,
            show_line_numbers: true,
            show_whitespace: false,
            wrap_lines: false,
            auto_indent: true,
            highlight_current_line: true,
            highlight_matching_brackets: true,
            show_indent_guides: true,
            highlight_word_at_cursor: true,
            show_sticky_scroll: true,
            show_minimap: true,
            vim_mode: false,
        }
    }
}

pub fn build(buffer: &sourceview5::Buffer, cfg: &ViewConfig) -> sourceview5::View {
    let view = sourceview5::View::with_buffer(buffer);
    apply_to_view(&view, cfg);
    apply_to_buffer(buffer, cfg);
    install_completion(&view, buffer);
    view
}

fn install_completion(view: &sourceview5::View, buffer: &sourceview5::Buffer) {
    let snippet_manager = sourceview5::SnippetManager::default();
    snippets::register_bundled(&snippet_manager);
    view.set_enable_snippets(true);
    let completion = view.completion();
    let snippet_provider = sourceview5::CompletionSnippets::new();
    snippet_provider.set_priority(200);
    completion.add_provider(&snippet_provider);
    let words = sourceview5::CompletionWords::new(Some(strings::COMPLETION_WORDS_TITLE));
    words.set_priority(100);
    words.register(buffer);
    completion.add_provider(&words);
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
    let pattern = if cfg.show_indent_guides {
        sourceview5::BackgroundPatternType::Grid
    } else {
        sourceview5::BackgroundPatternType::None
    };
    view.set_background_pattern(pattern);
    view.add_css_class("sourceview");
}

pub fn apply_css(view: &sourceview5::View, cfg: &ViewConfig) {
    let css = format!(
        ".sourceview, .sourceview text {{ font-family: \"{0}\", \"Lilex\", \"JetBrains Mono\", \"JetBrainsMono Nerd Font\", \"Cascadia Mono\", \"Fira Code\", \"Iosevka\", \"DejaVu Sans Mono\", monospace; font-size: {1}pt; padding: 6px 12px; line-height: 1.3; letter-spacing: 0; }} .lyrux-sticky-header {{ font-family: \"{0}\", \"Lilex\", \"JetBrains Mono\", monospace; font-size: {1}pt; padding: 2px 12px; background: alpha(@theme_bg_color, 0.92); color: @theme_fg_color; border-bottom: 1px solid alpha(@theme_fg_color, 0.18); font-weight: 600; }}",
        cfg.font_family.replace('"', ""),
        cfg.font_size
    );
    view.add_css_class("sourceview");
    EDITOR_FONT_PROVIDER.with(|slot| {
        let mut slot_ref = slot.borrow_mut();
        if let Some(provider) = slot_ref.as_ref() {
            provider.load_from_data(&css);
            return;
        }
        let provider = gtk::CssProvider::new();
        provider.load_from_data(&css);
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        *slot_ref = Some(provider);
    });
}

pub fn apply_to_buffer(buffer: &sourceview5::Buffer, cfg: &ViewConfig) {
    buffer.set_highlight_matching_brackets(cfg.highlight_matching_brackets);
    let manager = sourceview5::StyleSchemeManager::default();
    themes::register_all(&manager);
    if let Some(scheme) = manager.scheme(&cfg.theme_id) {
        buffer.set_style_scheme(Some(&scheme));
    }
}
