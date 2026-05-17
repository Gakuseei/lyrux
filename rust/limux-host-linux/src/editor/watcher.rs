#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::editor::buffer::{self, FileEtag};
use crate::editor::strings;
use crate::editor::tab_state::{DirtyMarkerCb, EditorTabState};

#[derive(Clone)]
struct WatcherCtx {
    path: Rc<RefCell<PathBuf>>,
    buffer: sourceview5::Buffer,
    scrolled: gtk4::ScrolledWindow,
    dirty: Rc<Cell<bool>>,
    saved_etag: Rc<Cell<Option<FileEtag>>>,
    suppress_dirty: Rc<Cell<bool>>,
    banner: gtk4::Revealer,
    dirty_marker_cb: DirtyMarkerCb,
}

impl WatcherCtx {
    fn from_state(state: &EditorTabState) -> Self {
        Self {
            path: state.path.clone(),
            buffer: state.buffer.clone(),
            scrolled: state.scrolled.clone(),
            dirty: state.dirty.clone(),
            saved_etag: state.saved_etag.clone(),
            suppress_dirty: state.suppress_dirty.clone(),
            banner: state.banner.clone(),
            dirty_marker_cb: state.dirty_marker_cb.clone(),
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    fn snapshot_text(&self) -> String {
        let (start, end) = self.buffer.bounds();
        self.buffer.text(&start, &end, false).to_string()
    }

    fn mark_clean(&self, etag: FileEtag) {
        self.dirty.set(false);
        self.saved_etag.set(Some(etag));
        if let Some(cb) = self.dirty_marker_cb.borrow().as_ref() {
            cb(false);
        }
    }
}

pub fn install(state: &EditorTabState) -> Option<gtk4::gio::FileMonitor> {
    let path = state.path.borrow().clone();
    let file = gtk4::gio::File::for_path(&path);
    let monitor = file
        .monitor_file(
            gtk4::gio::FileMonitorFlags::WATCH_HARD_LINKS,
            gtk4::gio::Cancellable::NONE,
        )
        .ok()?;
    let ctx = WatcherCtx::from_state(state);
    monitor.connect_changed(move |_m, _f, _other, event| {
        if !matches!(
            event,
            gtk4::gio::FileMonitorEvent::ChangesDoneHint
                | gtk4::gio::FileMonitorEvent::Created
                | gtk4::gio::FileMonitorEvent::Deleted
        ) {
            return;
        }
        let path_now = ctx.path.borrow().clone();
        let current = FileEtag::for_path(&path_now).ok();
        let saved = ctx.saved_etag.get();
        if current == saved {
            return;
        }
        match (current, ctx.is_dirty()) {
            (Some(_), false) => reload_clean(&ctx),
            (Some(_), true) => show_banner(&ctx, false),
            (None, _) => show_banner_deleted(&ctx),
        }
    });
    Some(monitor)
}

fn reload_clean(ctx: &WatcherCtx) {
    let path = ctx.path.borrow().clone();
    if let buffer::LoadResult::Text { contents, etag } = buffer::load(&path) {
        let cursor_line;
        let cursor_col;
        {
            let mark = ctx.buffer.get_insert();
            let iter = ctx.buffer.iter_at_mark(&mark);
            cursor_line = iter.line();
            cursor_col = iter.line_offset();
        }
        let scroll = ctx.scrolled.vadjustment().value();

        ctx.suppress_dirty.set(true);
        ctx.buffer.set_text(&contents);
        ctx.suppress_dirty.set(false);
        ctx.mark_clean(etag);

        if let Some(iter) = ctx.buffer.iter_at_line_offset(cursor_line, cursor_col) {
            ctx.buffer.place_cursor(&iter);
        }
        ctx.scrolled.vadjustment().set_value(scroll);
    }
}

fn show_banner(ctx: &WatcherCtx, deleted: bool) {
    let path_display = ctx.path.borrow().display().to_string();
    let banner = build_banner_widget(
        if deleted {
            strings::BANNER_FILE_DELETED_PREFIX
        } else {
            strings::BANNER_FILE_CHANGED_PREFIX
        },
        &path_display,
        deleted,
        ctx.clone(),
    );
    ctx.banner.set_child(Some(&banner));
    ctx.banner.set_reveal_child(true);
}

fn show_banner_deleted(ctx: &WatcherCtx) {
    show_banner(ctx, true);
}

fn build_banner_widget(
    prefix: &str,
    path_display: &str,
    deleted: bool,
    ctx: WatcherCtx,
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
    let c = ctx.clone();
    action_btn.connect_clicked(move |_| {
        if deleted {
            let path = c.path.borrow().clone();
            let _ = buffer::save_atomic(&path, &c.snapshot_text());
            if let Ok(etag) = FileEtag::for_path(&path) {
                c.mark_clean(etag);
            }
            c.banner.set_reveal_child(false);
        } else {
            reload_clean(&c);
            c.banner.set_reveal_child(false);
        }
    });
    bar.append(&action_btn);

    let dismiss = gtk4::Button::with_label(if deleted {
        strings::BANNER_CLOSE_TAB
    } else {
        strings::BANNER_KEEP_MINE
    });
    let banner_weak = ctx.banner.downgrade();
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

pub fn show_error_banner(state: &EditorTabState, message: &str) {
    let bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    bar.set_margin_top(6);
    bar.set_margin_bottom(6);
    bar.set_margin_start(12);
    bar.set_margin_end(12);
    let label = gtk4::Label::new(Some(message));
    label.set_hexpand(true);
    label.set_halign(gtk4::Align::Start);
    label.set_wrap(true);
    bar.append(&label);

    let dismiss_btn = gtk4::Button::with_label(strings::BANNER_DISMISS);
    let banner_weak = state.banner.downgrade();
    dismiss_btn.connect_clicked(move |_| {
        if let Some(b) = banner_weak.upgrade() {
            b.set_reveal_child(false);
        }
    });
    bar.append(&dismiss_btn);

    state.banner.set_child(Some(&bar));
    state.banner.set_reveal_child(true);
}
