use gtk4::prelude::*;
use sourceview5::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use crate::editor::buffer::{self, FileEtag, LoadResult};
use crate::editor::highlight::{self, HighlightController};
use crate::editor::indent;
use crate::editor::langs;
use crate::editor::minimap_overlay;
use crate::editor::pair;
use crate::editor::status_bar;
use crate::editor::sticky_scroll::{self, StickyController};
use crate::editor::view::{self, ViewConfig};

pub type DirtyMarkerCb = Rc<RefCell<Option<Rc<dyn Fn(bool)>>>>;
pub type TitleCb = Rc<RefCell<Option<Rc<dyn Fn(&str)>>>>;
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
    pub saved_char_count: Rc<Cell<i32>>,
    pub banner: gtk4::Revealer,
    pub root: gtk4::Box,
    pub monitor: Rc<RefCell<Option<gtk4::gio::FileMonitor>>>,
    pub suppress_dirty: Rc<Cell<bool>>,
    pub dirty_marker_cb: DirtyMarkerCb,
    pub title_cb: TitleCb,
    pub swap_path: Rc<RefCell<Option<PathBuf>>>,
    pub highlight: HighlightController,
    pub sticky: StickyController,
    pub minimap: sourceview5::Map,
    pub minimap_container: gtk4::Overlay,
    pub wrap_button: gtk4::Button,
    pub vim_label: gtk4::Label,
    pub vim_im_context: Rc<RefCell<Option<sourceview5::VimIMContext>>>,
    pub vim_key_controller: Rc<RefCell<Option<gtk4::EventControllerKey>>>,
    pub save_action: ActionCb,
    pub close_action: ActionCb,
}

#[allow(clippy::large_enum_variant)]
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
    view::install_file_drop_reject(&scrolled);
    let sticky = sticky_scroll::install(&view, &buffer, &scrolled, cfg.show_sticky_scroll);

    let minimap = build_minimap(&view, cfg.show_minimap);
    let editor_row_built = build_editor_row(&view, sticky.overlay(), &minimap, cfg.show_minimap);
    let editor_row = editor_row_built.root;
    let minimap_container = editor_row_built.minimap_container;
    minimap_overlay::apply_reservation(&scrolled, cfg.show_minimap);

    let banner = gtk4::Revealer::builder()
        .reveal_child(false)
        .transition_type(gtk4::RevealerTransitionType::SlideDown)
        .build();

    let status = status_bar::build(&buffer, cfg);
    let wrap_button = status.wrap_button.clone();
    let vim_label = status.vim_label.clone();
    let status_root = status.root;

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root.append(&banner);
    root.append(&editor_row);
    root.append(&status_root);
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
        saved_char_count: Rc::new(Cell::new(buffer.char_count())),
        banner,
        root,
        monitor: Rc::new(RefCell::new(None)),
        suppress_dirty: Rc::new(Cell::new(false)),
        dirty_marker_cb: Rc::new(RefCell::new(None)),
        title_cb: Rc::new(RefCell::new(None)),
        swap_path: Rc::new(RefCell::new(None)),
        highlight: highlight_ctrl,
        sticky,
        minimap,
        minimap_container,
        wrap_button,
        vim_label,
        vim_im_context: Rc::new(RefCell::new(None)),
        vim_key_controller: Rc::new(RefCell::new(None)),
        save_action: Rc::new(RefCell::new(None)),
        close_action: Rc::new(RefCell::new(None)),
    };
    view::apply_css(&state.view, cfg);
    crate::editor::vim::apply_to_tab(&state, cfg.vim_mode);

    install_dirty_tracker(
        &buffer,
        &state.dirty,
        &state.suppress_dirty,
        &state.saved_text,
        &state.saved_char_count,
        &state.dirty_marker_cb,
    );

    BuildOutcome::Ok(state)
}

pub fn install_dirty_tracker(
    buffer: &sourceview5::Buffer,
    dirty: &Rc<Cell<bool>>,
    suppress: &Rc<Cell<bool>>,
    saved_text: &Rc<RefCell<String>>,
    saved_char_count: &Rc<Cell<i32>>,
    dirty_marker_cb: &DirtyMarkerCb,
) {
    let dirty = dirty.clone();
    let suppress = suppress.clone();
    let saved_text = saved_text.clone();
    let saved_char_count = saved_char_count.clone();
    let dirty_marker_cb = dirty_marker_cb.clone();
    buffer.connect_changed(move |buf| {
        if suppress.get() {
            return;
        }
        let cur_count = buf.char_count();
        let saved_count = saved_char_count.get();
        let is_dirty = if cur_count != saved_count {
            true
        } else {
            let (s, e) = buf.bounds();
            let now = buf.text(&s, &e, false).to_string();
            let saved = saved_text.borrow();
            compute_dirty(&now, &saved)
        };
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
    map.set_visible(visible);
    map
}

pub struct EditorRow {
    pub root: gtk4::Overlay,
    pub minimap_container: gtk4::Overlay,
}

pub fn build_editor_row(
    view: &sourceview5::View,
    sticky_overlay: &gtk4::Overlay,
    minimap: &sourceview5::Map,
    show_minimap: bool,
) -> EditorRow {
    let row = gtk4::Overlay::new();
    row.set_hexpand(true);
    row.set_vexpand(true);
    row.set_child(Some(sticky_overlay));

    let container = minimap_overlay::build(view, minimap);
    container.root.set_halign(gtk4::Align::End);
    container.root.set_valign(gtk4::Align::Fill);
    container.root.set_visible(show_minimap);
    row.add_overlay(&container.root);
    EditorRow {
        root: row,
        minimap_container: container.root,
    }
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
        self.saved_char_count.set(self.buffer.char_count());
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
