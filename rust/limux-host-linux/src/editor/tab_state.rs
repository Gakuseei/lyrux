use gtk4::prelude::*;
use sourceview5::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use crate::editor::buffer::{self, FileEtag, LoadResult};
use crate::editor::highlight::{self, HighlightController};
use crate::editor::indent;
use crate::editor::langs;
use crate::editor::pair;
use crate::editor::status_bar;
use crate::editor::sticky_scroll::{self, StickyController};
use crate::editor::view::{self, ViewConfig};

pub type DirtyMarkerCb = Rc<RefCell<Option<Rc<dyn Fn(bool)>>>>;
pub type TitleCb = Rc<RefCell<Option<Rc<dyn Fn(&str)>>>>;
pub type ViewCssProvider = Rc<RefCell<Option<gtk4::CssProvider>>>;
pub type ActionCb = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

#[derive(Clone)]
pub struct EditorTabState {
    pub path: Rc<RefCell<PathBuf>>,
    pub scrolled: gtk4::ScrolledWindow,
    pub view: sourceview5::View,
    pub buffer: sourceview5::Buffer,
    pub dirty: Rc<Cell<bool>>,
    pub saved_etag: Rc<Cell<Option<FileEtag>>>,
    pub saved_text: Rc<RefCell<String>>,
    pub banner: gtk4::Revealer,
    pub root: gtk4::Box,
    pub monitor: Rc<RefCell<Option<gtk4::gio::FileMonitor>>>,
    pub suppress_dirty: Rc<Cell<bool>>,
    pub dirty_marker_cb: DirtyMarkerCb,
    pub title_cb: TitleCb,
    pub css_provider: ViewCssProvider,
    pub swap_path: Rc<RefCell<Option<PathBuf>>>,
    pub highlight: HighlightController,
    pub sticky: StickyController,
    pub minimap: sourceview5::Map,
    pub save_action: ActionCb,
    pub close_action: ActionCb,
}

pub enum BuildOutcome {
    Ok(EditorTabState),
    TooLarge(#[allow(dead_code)] u64),
    Binary,
    NotFound,
    Io(String),
}

pub fn build(path: PathBuf, cfg: &ViewConfig) -> BuildOutcome {
    let load = buffer::load(&path);
    let (text, etag) = match load {
        LoadResult::Text { contents, etag } => (contents, etag),
        LoadResult::Binary { .. } => return BuildOutcome::Binary,
        LoadResult::TooLarge { size } => return BuildOutcome::TooLarge(size),
        LoadResult::NotFound => return BuildOutcome::NotFound,
        LoadResult::Io(e) => return BuildOutcome::Io(e),
    };
    let buffer = sourceview5::Buffer::new(None);
    buffer.set_text(&text);
    if let Some(lang) = langs::language_for_path(&path) {
        buffer.set_language(Some(&lang));
    }
    let view = view::build(&buffer, cfg);
    pair::install(&view, &buffer);
    indent::install(&view, &buffer);
    let highlight_ctrl = highlight::install(&buffer, cfg.highlight_word_at_cursor);
    let scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .child(&view)
        .hexpand(true)
        .vexpand(true)
        .build();
    let sticky = sticky_scroll::install(&view, &buffer, &scrolled, cfg.show_sticky_scroll);

    let minimap = build_minimap(&view, cfg.show_minimap);
    let editor_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    editor_row.set_hexpand(true);
    editor_row.set_vexpand(true);
    editor_row.append(sticky.overlay());
    editor_row.append(&minimap);

    let banner = gtk4::Revealer::builder()
        .reveal_child(false)
        .transition_type(gtk4::RevealerTransitionType::SlideDown)
        .build();

    let status = status_bar::build(&buffer, cfg);

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root.append(&banner);
    root.append(&editor_row);
    root.append(&status);
    root.set_hexpand(true);
    root.set_vexpand(true);

    let state = EditorTabState {
        path: Rc::new(RefCell::new(path)),
        scrolled,
        view,
        buffer: buffer.clone(),
        dirty: Rc::new(Cell::new(false)),
        saved_etag: Rc::new(Cell::new(Some(etag))),
        saved_text: Rc::new(RefCell::new(text.clone())),
        banner,
        root,
        monitor: Rc::new(RefCell::new(None)),
        suppress_dirty: Rc::new(Cell::new(false)),
        dirty_marker_cb: Rc::new(RefCell::new(None)),
        title_cb: Rc::new(RefCell::new(None)),
        css_provider: Rc::new(RefCell::new(None)),
        swap_path: Rc::new(RefCell::new(None)),
        highlight: highlight_ctrl,
        sticky,
        minimap,
        save_action: Rc::new(RefCell::new(None)),
        close_action: Rc::new(RefCell::new(None)),
    };
    view::apply_css(&state.view, cfg, &state.css_provider);

    install_dirty_tracker(
        &buffer,
        &state.dirty,
        &state.suppress_dirty,
        &state.saved_text,
        &state.dirty_marker_cb,
    );

    BuildOutcome::Ok(state)
}

fn install_dirty_tracker(
    buffer: &sourceview5::Buffer,
    dirty: &Rc<Cell<bool>>,
    suppress: &Rc<Cell<bool>>,
    saved_text: &Rc<RefCell<String>>,
    dirty_marker_cb: &DirtyMarkerCb,
) {
    let dirty = dirty.clone();
    let suppress = suppress.clone();
    let saved_text = saved_text.clone();
    let dirty_marker_cb = dirty_marker_cb.clone();
    buffer.connect_changed(move |buf| {
        if suppress.get() {
            return;
        }
        let (s, e) = buf.bounds();
        let now = buf.text(&s, &e, false).to_string();
        let saved = saved_text.borrow();
        let is_dirty = compute_dirty(&now, &saved);
        if dirty.get() != is_dirty {
            dirty.set(is_dirty);
            if let Some(cb) = dirty_marker_cb.borrow().as_ref() {
                cb(is_dirty);
            }
        }
    });
}

pub fn compute_dirty(current: &str, saved: &str) -> bool {
    current != saved
}

pub fn build_minimap(view: &sourceview5::View, visible: bool) -> sourceview5::Map {
    let map = sourceview5::Map::new();
    map.set_view(view);
    map.set_width_request(120);
    map.set_visible(visible);
    map
}

impl EditorTabState {
    pub fn snapshot_text(&self) -> String {
        let (start, end) = self.buffer.bounds();
        self.buffer.text(&start, &end, false).to_string()
    }

    pub fn mark_clean(&self, etag: FileEtag) {
        self.dirty.set(false);
        self.saved_etag.set(Some(etag));
        *self.saved_text.borrow_mut() = self.snapshot_text();
        if let Some(cb) = self.dirty_marker_cb.borrow().as_ref() {
            cb(false);
        }
    }

    pub fn set_monitor(&self, m: Option<gtk4::gio::FileMonitor>) {
        *self.monitor.borrow_mut() = m;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    pub fn on_dirty_changed<F: Fn(bool) + 'static>(&self, f: F) {
        let cb: Rc<dyn Fn(bool)> = Rc::new(f);
        *self.dirty_marker_cb.borrow_mut() = Some(cb);
    }

    pub fn on_title_changed<F: Fn(&str) + 'static>(&self, f: F) {
        let cb: Rc<dyn Fn(&str)> = Rc::new(f);
        *self.title_cb.borrow_mut() = Some(cb);
    }
}

#[cfg(test)]
mod tests {
    use super::compute_dirty;

    #[derive(Default)]
    struct DirtySim {
        saved: String,
        current: String,
        dirty: bool,
        emissions: Vec<bool>,
    }

    impl DirtySim {
        fn new(initial: &str) -> Self {
            Self {
                saved: initial.to_string(),
                current: initial.to_string(),
                dirty: false,
                emissions: Vec::new(),
            }
        }

        fn edit(&mut self, next: &str) {
            self.current = next.to_string();
            let is_dirty = compute_dirty(&self.current, &self.saved);
            if self.dirty != is_dirty {
                self.dirty = is_dirty;
                self.emissions.push(is_dirty);
            }
        }
    }

    #[test]
    fn compute_dirty_string_equality() {
        assert!(!compute_dirty("hello", "hello"));
        assert!(compute_dirty("hello ", "hello"));
        assert!(compute_dirty("", "x"));
        assert!(!compute_dirty("", ""));
    }

    #[test]
    fn type_then_backspace_reverts_dirty() {
        let mut sim = DirtySim::new("hello");
        sim.edit("hello ");
        assert!(sim.dirty);
        sim.edit("hello");
        assert!(!sim.dirty);
        assert_eq!(sim.emissions, vec![true, false]);
    }

    #[test]
    fn repeated_typing_emits_dirty_once() {
        let mut sim = DirtySim::new("a");
        sim.edit("ab");
        sim.edit("abc");
        sim.edit("abcd");
        assert_eq!(sim.emissions, vec![true]);
    }

    #[test]
    fn empty_buffer_round_trip() {
        let mut sim = DirtySim::new("");
        sim.edit("x");
        sim.edit("");
        assert!(!sim.dirty);
        assert_eq!(sim.emissions, vec![true, false]);
    }
}
