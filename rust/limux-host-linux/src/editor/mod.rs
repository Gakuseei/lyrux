pub mod buffer;
pub mod closed_tabs;
pub mod command_palette;
pub mod editorconfig;
pub mod find_bar;
pub mod find_in_files;
pub mod highlight;
pub mod image_viewer;
pub mod indent;
pub mod keymap;
pub mod langs;
pub mod pair;
pub mod quick_open;
pub mod recent_files;
pub mod session;
pub mod settings;
pub mod settings_panel;
pub mod snippets;
pub mod status_bar;
pub mod sticky_scroll;
pub mod strings;
pub mod swap;
pub mod tab_state;
pub mod themes;
pub mod view;
pub mod vim;
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
    let minimap = tab_state::build_minimap(&view, cfg.show_minimap);
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
        path: Rc::new(RefCell::new(PathBuf::new())),
        scrolled,
        view,
        buffer: buffer.clone(),
        dirty: Rc::new(Cell::new(false)),
        saved_etag: Rc::new(Cell::new(None)),
        saved_text: Rc::new(RefCell::new(String::new())),
        saved_char_count: Rc::new(Cell::new(0)),
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
        wrap_button,
        vim_label,
        vim_im_context: Rc::new(RefCell::new(None)),
        vim_key_controller: Rc::new(RefCell::new(None)),
        save_action: Rc::new(RefCell::new(None)),
        close_action: Rc::new(RefCell::new(None)),
    };
    view::apply_css(&state.view, cfg);
    vim::apply_to_tab(&state, cfg.vim_mode);

    tab_state::install_dirty_tracker(
        &buffer,
        &state.dirty,
        &state.suppress_dirty,
        &state.saved_text,
        &state.saved_char_count,
        &state.dirty_marker_cb,
    );

    state
}

pub fn spawn_from_path(path: PathBuf, cfg: &ViewConfig) -> BuildOutcome {
    tab_state::build(path, cfg)
}
