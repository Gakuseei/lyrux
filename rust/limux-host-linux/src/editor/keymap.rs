use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;

use crate::editor::buffer;
use crate::editor::strings;
use crate::editor::tab_state::EditorTabState;
use crate::file_panel::model::is_within_root;

pub fn install(
    view: &sourceview5::View,
    state: &EditorTabState,
    workspace_root: Option<PathBuf>,
    on_clean: Rc<dyn Fn()>,
) {
    let ctrl = gtk4::EventControllerKey::new();
    let state = state.clone();
    ctrl.connect_key_pressed(move |_, key, _, mods| {
        let ctrl_held = mods.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
        if !ctrl_held {
            return glib::Propagation::Proceed;
        }
        match key {
            gtk4::gdk::Key::s => {
                save_tab(&state, workspace_root.as_deref(), &on_clean);
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
    view.add_controller(ctrl);
}

pub fn save_tab(state: &EditorTabState, workspace_root: Option<&Path>, on_clean: &Rc<dyn Fn()>) {
    if state.path.as_os_str().is_empty() {
        return;
    }
    if let Some(root) = workspace_root {
        if !is_within_root(&state.path, root) {
            eprintln!("lyrux: {}", strings::ERROR_OUTSIDE_WORKSPACE);
            return;
        }
    }
    let text = state.snapshot_text();
    match buffer::save_atomic(&state.path, &text) {
        Ok(etag) => {
            state.mark_clean(etag);
            if let Some(sp) = state.swap_path.borrow_mut().take() {
                let _ = crate::editor::swap::discard(&sp);
            }
            on_clean();
        }
        Err(e) => eprintln!("lyrux: {}{e}", strings::ERROR_WRITE_FAILED_PREFIX),
    }
}
