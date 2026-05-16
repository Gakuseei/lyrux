pub mod buffer;
pub mod image_viewer;
pub mod keymap;
pub mod langs;
pub mod pair;
pub mod session;
pub mod settings;
pub mod settings_panel;
pub mod strings;
pub mod swap;
pub mod tab_state;
pub mod themes;
pub mod view;
pub mod watcher;

pub use settings::EditorSettings;

#[allow(unused_imports)]
pub use image_viewer::ImageViewerTabState;
pub use tab_state::EditorTabState;
pub use view::ViewConfig;

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::prelude::*;

use crate::editor::tab_state::BuildOutcome;

pub enum FileKind {
    Editable,
    Image,
}

pub fn classify_file(path: &Path) -> FileKind {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp" => FileKind::Image,
        _ => FileKind::Editable,
    }
}

pub fn spawn_empty(cfg: &ViewConfig) -> EditorTabState {
    let buffer = sourceview5::Buffer::new(None);
    let view = view::build(&buffer, cfg);
    pair::install(&view, &buffer);
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
        path: Rc::new(RefCell::new(PathBuf::new())),
        scrolled,
        view,
        buffer: buffer.clone(),
        dirty: Rc::new(Cell::new(false)),
        saved_etag: Rc::new(Cell::new(None)),
        banner,
        root,
        monitor: Rc::new(RefCell::new(None)),
        suppress_dirty: Rc::new(Cell::new(false)),
        dirty_marker_cb: Rc::new(RefCell::new(None)),
        title_cb: Rc::new(RefCell::new(None)),
        css_provider: Rc::new(RefCell::new(None)),
        swap_path: Rc::new(RefCell::new(None)),
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

    state
}

pub fn spawn_from_path(path: PathBuf, cfg: &ViewConfig) -> BuildOutcome {
    tab_state::build(path, cfg)
}
