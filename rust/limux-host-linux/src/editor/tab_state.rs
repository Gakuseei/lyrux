#![allow(dead_code)]

use gtk4::prelude::*;
use sourceview5::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use crate::editor::buffer::{self, FileEtag, LoadResult};
use crate::editor::langs;
use crate::editor::view::{self, ViewConfig};

pub type DirtyMarkerCb = Rc<RefCell<Option<Rc<dyn Fn(bool)>>>>;
pub type ViewCssProvider = Rc<RefCell<Option<gtk4::CssProvider>>>;

#[derive(Clone)]
pub struct EditorTabState {
    pub path: PathBuf,
    pub scrolled: gtk4::ScrolledWindow,
    pub view: sourceview5::View,
    pub buffer: sourceview5::Buffer,
    pub dirty: Rc<Cell<bool>>,
    pub saved_etag: Rc<Cell<Option<FileEtag>>>,
    pub banner: gtk4::Revealer,
    pub root: gtk4::Box,
    pub monitor: Rc<RefCell<Option<gtk4::gio::FileMonitor>>>,
    pub suppress_dirty: Rc<Cell<bool>>,
    pub dirty_marker_cb: DirtyMarkerCb,
    pub css_provider: ViewCssProvider,
}

pub enum BuildOutcome {
    Ok(EditorTabState),
    TooLarge(u64),
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
    let scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .child(&view)
        .hexpand(true)
        .vexpand(true)
        .build();

    let banner = gtk4::Revealer::builder()
        .reveal_child(false)
        .transition_type(gtk4::RevealerTransitionType::SlideDown)
        .build();

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root.append(&banner);
    root.append(&scrolled);
    root.set_hexpand(true);
    root.set_vexpand(true);

    let state = EditorTabState {
        path,
        scrolled,
        view,
        buffer: buffer.clone(),
        dirty: Rc::new(Cell::new(false)),
        saved_etag: Rc::new(Cell::new(Some(etag))),
        banner,
        root,
        monitor: Rc::new(RefCell::new(None)),
        suppress_dirty: Rc::new(Cell::new(false)),
        dirty_marker_cb: Rc::new(RefCell::new(None)),
        css_provider: Rc::new(RefCell::new(None)),
    };
    view::apply_css(&state.view, cfg, &state.css_provider);

    let dirty = state.dirty.clone();
    let suppress = state.suppress_dirty.clone();
    buffer.connect_changed(move |_| {
        if suppress.get() {
            return;
        }
        dirty.set(true);
    });

    BuildOutcome::Ok(state)
}

impl EditorTabState {
    pub fn snapshot_text(&self) -> String {
        let (start, end) = self.buffer.bounds();
        self.buffer.text(&start, &end, false).to_string()
    }

    pub fn mark_clean(&self, etag: FileEtag) {
        self.dirty.set(false);
        self.saved_etag.set(Some(etag));
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
        *self.dirty_marker_cb.borrow_mut() = Some(cb.clone());
        let suppress = self.suppress_dirty.clone();
        self.buffer.connect_changed(move |_| {
            if suppress.get() {
                return;
            }
            cb(true);
        });
    }
}
