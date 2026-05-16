#![allow(dead_code)]

use gtk4::prelude::*;

use crate::editor::buffer::{self, FileEtag};
use crate::editor::strings;
use crate::editor::tab_state::EditorTabState;

pub fn install(state: &EditorTabState) -> Option<gtk4::gio::FileMonitor> {
    let file = gtk4::gio::File::for_path(&state.path);
    let monitor = file
        .monitor_file(
            gtk4::gio::FileMonitorFlags::WATCH_HARD_LINKS,
            gtk4::gio::Cancellable::NONE,
        )
        .ok()?;
    let state = state.clone();
    monitor.connect_changed(move |_m, _f, _other, event| {
        if !matches!(
            event,
            gtk4::gio::FileMonitorEvent::ChangesDoneHint
                | gtk4::gio::FileMonitorEvent::Created
                | gtk4::gio::FileMonitorEvent::Deleted
        ) {
            return;
        }
        let current = FileEtag::for_path(&state.path).ok();
        let saved = state.saved_etag.get();
        if current == saved {
            return;
        }
        match (current, state.is_dirty()) {
            (Some(_), false) => reload_clean(&state),
            (Some(_), true) => show_banner(&state, false),
            (None, _) => show_banner_deleted(&state),
        }
    });
    Some(monitor)
}

fn reload_clean(state: &EditorTabState) {
    if let buffer::LoadResult::Text { contents, etag } = buffer::load(&state.path) {
        state.suppress_dirty.set(true);
        state.buffer.set_text(&contents);
        state.suppress_dirty.set(false);
        state.mark_clean(etag);
    }
}

fn show_banner(state: &EditorTabState, deleted: bool) {
    let banner = build_banner_widget(
        if deleted {
            strings::BANNER_FILE_DELETED_PREFIX
        } else {
            strings::BANNER_FILE_CHANGED_PREFIX
        },
        &state.path.display().to_string(),
        deleted,
        state.clone(),
    );
    state.banner.set_child(Some(&banner));
    state.banner.set_reveal_child(true);
}

fn show_banner_deleted(state: &EditorTabState) {
    show_banner(state, true);
}

fn build_banner_widget(
    prefix: &str,
    path_display: &str,
    deleted: bool,
    state: EditorTabState,
) -> gtk4::Box {
    let bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    bar.set_margin_top(6);
    bar.set_margin_bottom(6);
    bar.set_margin_start(12);
    bar.set_margin_end(12);
    let label = gtk4::Label::new(Some(&format!("{prefix}{path_display}")));
    label.set_hexpand(true);
    label.set_halign(gtk4::Align::Start);
    bar.append(&label);

    let action_label = if deleted {
        strings::BANNER_SAVE_AS_NEW
    } else {
        strings::BANNER_RELOAD
    };
    let action_btn = gtk4::Button::with_label(action_label);
    let s = state.clone();
    action_btn.connect_clicked(move |_| {
        if deleted {
            let _ = buffer::save_atomic(&s.path, &s.snapshot_text());
            if let Ok(etag) = FileEtag::for_path(&s.path) {
                s.mark_clean(etag);
            }
            s.banner.set_reveal_child(false);
        } else {
            reload_clean(&s);
            s.banner.set_reveal_child(false);
        }
    });
    bar.append(&action_btn);

    let dismiss = gtk4::Button::with_label(if deleted {
        strings::BANNER_CLOSE_TAB
    } else {
        strings::BANNER_KEEP_MINE
    });
    let banner_weak = state.banner.downgrade();
    dismiss.connect_clicked(move |_| {
        if let Some(b) = banner_weak.upgrade() {
            b.set_reveal_child(false);
        }
    });
    bar.append(&dismiss);

    bar
}

pub fn dismiss(state: &EditorTabState) {
    state.banner.set_reveal_child(false);
}
