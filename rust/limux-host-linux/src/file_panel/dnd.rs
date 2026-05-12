use gtk4 as gtk;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::file_panel::row_object::RowObject;

// Phase-2: drag-and-drop wiring. Not invoked yet in Phase 1; helpers kept
// here so the follow-up task can plug them in without rebuilding from
// scratch. Suppress dead-code warnings until then.

#[allow(dead_code)]
pub fn install_drag_source(
    widget: &impl IsA<gtk::Widget>,
    get_row: impl Fn() -> Option<RowObject> + 'static,
) {
    let source = gtk::DragSource::new();
    source.set_actions(gdk::DragAction::COPY | gdk::DragAction::MOVE);
    source.connect_prepare(move |_, _, _| {
        let row = get_row()?;
        let file = gtk4::gio::File::for_path(row.path());
        let value = glib::Value::from(&file);
        Some(gdk::ContentProvider::for_value(&value))
    });
    widget.add_controller(source);
}

#[allow(dead_code)]
pub struct DropContext {
    pub root: PathBuf,
    pub hover_since: RefCell<Option<(PathBuf, Instant)>>,
}

#[allow(dead_code)]
pub fn install_drop_target(
    widget: &impl IsA<gtk::Widget>,
    ctx: Rc<DropContext>,
    target_dir_for: impl Fn() -> Option<PathBuf> + 'static + Clone,
    on_drop: impl Fn(Vec<PathBuf>, PathBuf) + 'static,
    on_hover_expand: impl Fn(PathBuf) + 'static,
) {
    let target = gtk::DropTarget::new(
        gtk4::gio::File::static_type(),
        gdk::DragAction::COPY | gdk::DragAction::MOVE,
    );
    let ctx_motion = ctx.clone();
    let target_dir_motion = target_dir_for.clone();
    target.connect_motion(move |_, _, _| {
        let dir = match target_dir_motion() {
            Some(p) => p,
            None => return gdk::DragAction::empty(),
        };
        let mut slot = ctx_motion.hover_since.borrow_mut();
        match &*slot {
            Some((p, _)) if *p == dir => {}
            _ => *slot = Some((dir.clone(), Instant::now())),
        }
        if let Some((p, started)) = &*slot {
            if started.elapsed() >= Duration::from_millis(600) {
                on_hover_expand(p.clone());
            }
        }
        gdk::DragAction::MOVE
    });
    target.connect_drop(move |_, value, _, _| {
        let dir = match target_dir_for() {
            Some(p) => p,
            None => return false,
        };
        let file = match value.get::<gtk4::gio::File>() {
            Ok(f) => f,
            Err(_) => return false,
        };
        let src = match file.path() {
            Some(p) => p,
            None => return false,
        };
        if dir.starts_with(&src) {
            return false;
        }
        if !crate::file_panel::model::is_within_root(&src, &ctx.root) {
            return false;
        }
        if !crate::file_panel::model::is_within_root(&dir, &ctx.root) {
            return false;
        }
        on_drop(vec![src], dir);
        true
    });
    widget.add_controller(target);
}

#[allow(dead_code)]
pub fn install_edge_autoscroll(scrolled: &gtk::ScrolledWindow) {
    let motion = gtk::EventControllerMotion::new();
    let scrolled_ref = scrolled.clone();
    motion.connect_motion(move |_, _, y| {
        let h = scrolled_ref.allocated_height() as f64;
        let edge = 32.0;
        let adj = scrolled_ref.vadjustment();
        if y < edge {
            adj.set_value(adj.value() - 12.0);
        } else if y > h - edge {
            adj.set_value(adj.value() + 12.0);
        }
    });
    scrolled.add_controller(motion);
}
