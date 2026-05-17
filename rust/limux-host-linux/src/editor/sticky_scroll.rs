use std::cell::Cell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk4::prelude::*;
use sourceview5::prelude::*;

type RefreshFn = Rc<dyn Fn()>;

#[derive(Clone)]
pub struct StickyController {
    label: gtk::Label,
    overlay: gtk::Overlay,
    enabled: Rc<Cell<bool>>,
    last_header_line: Rc<Cell<i32>>,
    refresh: RefreshFn,
}

impl StickyController {
    pub fn overlay(&self) -> &gtk::Overlay {
        &self.overlay
    }

    pub fn set_enabled(&self, on: bool) {
        self.enabled.set(on);
        if !on {
            self.label.set_visible(false);
            self.last_header_line.set(-1);
            return;
        }
        (self.refresh)();
    }
}

pub fn install(
    view: &sourceview5::View,
    buffer: &sourceview5::Buffer,
    scrolled: &gtk::ScrolledWindow,
    enabled: bool,
) -> StickyController {
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(scrolled));
    overlay.set_hexpand(true);
    overlay.set_vexpand(true);

    let label = gtk::Label::new(None);
    label.set_xalign(0.0);
    label.set_halign(gtk::Align::Fill);
    label.set_valign(gtk::Align::Start);
    label.set_hexpand(true);
    label.set_can_target(true);
    label.set_can_focus(false);
    label.set_visible(false);
    label.set_single_line_mode(true);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_cursor_from_name(Some("pointer"));
    label.add_css_class("lyrux-sticky-header");

    overlay.add_overlay(&label);

    let enabled_rc = Rc::new(Cell::new(enabled));
    let last_header = Rc::new(Cell::new(-1i32));

    let click = gtk::GestureClick::new();
    click.set_button(gtk::gdk::BUTTON_PRIMARY);
    let view_for_click = view.downgrade();
    let buffer_for_click = buffer.downgrade();
    let last_for_click = last_header.clone();
    click.connect_released(move |gesture, _n_press, _x, _y| {
        let line = last_for_click.get();
        if line < 0 {
            return;
        }
        let (Some(view), Some(buffer)) = (view_for_click.upgrade(), buffer_for_click.upgrade())
        else {
            return;
        };
        let Some(mut iter) = buffer.iter_at_line(line) else {
            return;
        };
        view.scroll_to_iter(&mut iter, 0.1, true, 0.0, 0.1);
        buffer.place_cursor(&iter);
        view.grab_focus();
        gesture.set_state(gtk::EventSequenceState::Claimed);
    });
    label.add_controller(click);

    let label_for_cb = label.clone();
    let view_weak = view.downgrade();
    let buffer_weak = buffer.downgrade();
    let enabled_for_cb = enabled_rc.clone();
    let last_for_cb = last_header.clone();
    let refresh_fn: RefreshFn = Rc::new(move || {
        let (Some(view), Some(buffer)) = (view_weak.upgrade(), buffer_weak.upgrade()) else {
            return;
        };
        update(&view, &buffer, &label_for_cb, &enabled_for_cb, &last_for_cb);
    });

    let vadj = scrolled.vadjustment();
    let refresh_for_scroll = refresh_fn.clone();
    vadj.connect_value_changed(move |_| {
        refresh_for_scroll();
    });

    let refresh_for_buffer = refresh_fn.clone();
    buffer.connect_changed(move |_| {
        refresh_for_buffer();
    });

    StickyController {
        label,
        overlay,
        enabled: enabled_rc,
        last_header_line: last_header,
        refresh: refresh_fn,
    }
}

fn update(
    view: &sourceview5::View,
    buffer: &sourceview5::Buffer,
    label: &gtk::Label,
    enabled: &Rc<Cell<bool>>,
    last_header: &Rc<Cell<i32>>,
) {
    if !enabled.get() {
        label.set_visible(false);
        last_header.set(-1);
        return;
    }
    let (_, buffer_top_y) = view.window_to_buffer_coords(gtk::TextWindowType::Widget, 0, 0);
    let (visible_iter, _) = view.line_at_y(buffer_top_y);
    let visible_line = visible_iter.line();

    let lang_id = buffer
        .language()
        .map(|l| l.id().to_string())
        .unwrap_or_default();
    let kind = LangKind::from_id(&lang_id);
    if matches!(kind, LangKind::Unsupported) {
        label.set_visible(false);
        last_header.set(-1);
        return;
    }

    let header_line = find_enclosing_header(buffer, visible_line, kind);
    match header_line {
        Some(h) if h != visible_line => {
            if last_header.get() == h {
                label.set_visible(true);
                return;
            }
            let text = line_text(buffer, h);
            let trimmed = text.trim_end_matches(['\r', '\n']).to_string();
            label.set_text(&trimmed);
            label.set_visible(true);
            last_header.set(h);
        }
        _ => {
            label.set_visible(false);
            last_header.set(-1);
        }
    }
}

fn line_text(buffer: &sourceview5::Buffer, line: i32) -> String {
    let start = buffer.iter_at_line(line).unwrap_or(buffer.start_iter());
    let end_line = line + 1;
    let end = buffer.iter_at_line(end_line).unwrap_or(buffer.end_iter());
    buffer.text(&start, &end, false).to_string()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LangKind {
    Rust,
    Typescript,
    Python,
    Markdown,
    Go,
    Unsupported,
}

impl LangKind {
    pub fn from_id(id: &str) -> Self {
        match id {
            "rust" => LangKind::Rust,
            "typescript" | "javascript" => LangKind::Typescript,
            "python" | "python3" => LangKind::Python,
            "markdown" => LangKind::Markdown,
            "go" => LangKind::Go,
            _ => LangKind::Unsupported,
        }
    }
}

pub fn find_enclosing_header(
    buffer: &sourceview5::Buffer,
    from_line: i32,
    kind: LangKind,
) -> Option<i32> {
    if matches!(kind, LangKind::Unsupported) {
        return None;
    }
    let mut line = from_line;
    while line >= 0 {
        let text = line_text(buffer, line);
        if is_header_line(&text, kind) {
            return Some(line);
        }
        line -= 1;
    }
    None
}

pub fn is_header_line(text: &str, kind: LangKind) -> bool {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return false;
    }
    match kind {
        LangKind::Rust => is_rust_header(trimmed),
        LangKind::Typescript => is_ts_header(trimmed),
        LangKind::Python => is_python_header(trimmed),
        LangKind::Markdown => is_markdown_header(trimmed),
        LangKind::Go => is_go_header(trimmed),
        LangKind::Unsupported => false,
    }
}

fn strip_visibility_rust(s: &str) -> &str {
    let mut rest = s;
    if let Some(stripped) = rest.strip_prefix("pub") {
        let after = stripped.trim_start();
        if let Some(after_paren) = after.strip_prefix('(') {
            if let Some(idx) = after_paren.find(')') {
                rest = after_paren[idx + 1..].trim_start();
            } else {
                rest = after.trim_start();
            }
        } else {
            rest = after;
        }
    }
    if let Some(stripped) = rest.strip_prefix("async ") {
        rest = stripped.trim_start();
    }
    if let Some(stripped) = rest.strip_prefix("unsafe ") {
        rest = stripped.trim_start();
    }
    if let Some(stripped) = rest.strip_prefix("default ") {
        rest = stripped.trim_start();
    }
    rest
}

fn starts_with_keyword(rest: &str, keyword: &str) -> bool {
    if let Some(after) = rest.strip_prefix(keyword) {
        match after.chars().next() {
            Some(c) => !c.is_alphanumeric() && c != '_',
            None => true,
        }
    } else {
        false
    }
}

fn is_rust_header(trimmed: &str) -> bool {
    let rest = strip_visibility_rust(trimmed);
    for kw in ["fn", "struct", "enum", "trait", "impl", "mod"] {
        if starts_with_keyword(rest, kw) {
            return true;
        }
    }
    false
}

fn is_ts_header(trimmed: &str) -> bool {
    let mut rest = trimmed;
    if let Some(stripped) = rest.strip_prefix("export ") {
        rest = stripped.trim_start();
    }
    if let Some(stripped) = rest.strip_prefix("default ") {
        rest = stripped.trim_start();
    }
    if let Some(stripped) = rest.strip_prefix("async ") {
        rest = stripped.trim_start();
    }
    for kw in [
        "function",
        "class",
        "interface",
        "type",
        "enum",
        "namespace",
    ] {
        if starts_with_keyword(rest, kw) {
            return true;
        }
    }
    false
}

fn is_python_header(trimmed: &str) -> bool {
    for kw in ["def", "class", "async def"] {
        if starts_with_keyword(trimmed, kw) {
            return true;
        }
    }
    false
}

fn is_markdown_header(trimmed: &str) -> bool {
    if !trimmed.starts_with('#') {
        return false;
    }
    let after_hashes = trimmed.trim_start_matches('#');
    let hash_count = trimmed.len() - after_hashes.len();
    if !(1..=6).contains(&hash_count) {
        return false;
    }
    matches!(after_hashes.chars().next(), Some(' '))
}

fn is_go_header(trimmed: &str) -> bool {
    for kw in ["func", "type"] {
        if starts_with_keyword(trimmed, kw) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_header_simple_fn() {
        assert!(is_header_line("fn main() {", LangKind::Rust));
    }

    #[test]
    fn rust_header_pub_fn() {
        assert!(is_header_line("pub fn foo() {", LangKind::Rust));
    }

    #[test]
    fn rust_header_pub_crate_fn() {
        assert!(is_header_line("pub(crate) fn foo() {", LangKind::Rust));
    }

    #[test]
    fn rust_header_indented_impl() {
        assert!(is_header_line("    impl Bar for Foo {", LangKind::Rust));
    }

    #[test]
    fn rust_header_async_fn() {
        assert!(is_header_line("pub async fn foo() {", LangKind::Rust));
    }

    #[test]
    fn rust_header_not_function_body() {
        assert!(!is_header_line("    let x = 5;", LangKind::Rust));
    }

    #[test]
    fn rust_header_not_fnord_identifier() {
        assert!(!is_header_line("fnord();", LangKind::Rust));
    }

    #[test]
    fn ts_header_function() {
        assert!(is_header_line(
            "export async function foo() {",
            LangKind::Typescript
        ));
    }

    #[test]
    fn ts_header_class() {
        assert!(is_header_line("class Foo {", LangKind::Typescript));
    }

    #[test]
    fn ts_header_interface() {
        assert!(is_header_line(
            "export interface Bar {",
            LangKind::Typescript
        ));
    }

    #[test]
    fn ts_header_not_arrow_assign() {
        assert!(!is_header_line("const x = () => 5;", LangKind::Typescript));
    }

    #[test]
    fn python_header_def() {
        assert!(is_header_line("def foo():", LangKind::Python));
    }

    #[test]
    fn python_header_indented_class() {
        assert!(is_header_line("    class Inner:", LangKind::Python));
    }

    #[test]
    fn python_header_not_call() {
        assert!(!is_header_line("foo(bar)", LangKind::Python));
    }

    #[test]
    fn markdown_header_h1() {
        assert!(is_header_line("# Title", LangKind::Markdown));
    }

    #[test]
    fn markdown_header_h3() {
        assert!(is_header_line("### Sub", LangKind::Markdown));
    }

    #[test]
    fn markdown_header_too_many_hashes() {
        assert!(!is_header_line("####### Bad", LangKind::Markdown));
    }

    #[test]
    fn markdown_header_no_space() {
        assert!(!is_header_line("#hashtag", LangKind::Markdown));
    }

    #[test]
    fn go_header_func() {
        assert!(is_header_line("func main() {", LangKind::Go));
    }

    #[test]
    fn go_header_type() {
        assert!(is_header_line("type Foo struct {", LangKind::Go));
    }

    #[test]
    fn unsupported_lang_no_header() {
        assert!(!is_header_line("fn main() {", LangKind::Unsupported));
    }
}
