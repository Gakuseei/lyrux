pub mod actions;
pub mod clipboard;
pub mod config;
pub mod dnd;
pub mod git;
pub mod model;
pub mod ops;
pub mod row_object;
pub mod view;
pub mod watcher;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;

use crate::file_panel::clipboard::{ClipMode, Clipboard};
use crate::file_panel::model::TreeModel;
use crate::file_panel::row_object::RowObject;
use crate::file_panel::view::{
    apply_model_to_store, build_header, build_list_view, file_panel_css, HeaderHandle, ViewState,
};
use crate::file_panel::watcher::WatcherHandle;

pub type WorkspaceId = String;

pub struct PerWorkspace {
    pub model: TreeModel,
    // Held to keep the filesystem watcher thread alive for this workspace's lifetime.
    #[allow(dead_code)]
    pub watcher: Option<WatcherHandle>,
}

pub struct Inner {
    pub root_box: gtk::Box,
    pub header: HeaderHandle,
    // Kept on Inner so future code (DnD autoscroll, scroll restoration) can reach it.
    #[allow(dead_code)]
    pub scrolled: gtk::ScrolledWindow,
    pub view: ViewState,
    // Owned by Inner to keep the sticky-overlay widget alive in the tree.
    #[allow(dead_code)]
    pub sticky: gtk::Box,
    pub clipboard: Clipboard,
    pub cache: HashMap<WorkspaceId, PerWorkspace>,
    pub active: Option<WorkspaceId>,
    pub visible: bool,
    // Bug 5: snapshot of expanded_paths captured on collapse-all so the next
    // press can restore the previously-open folders.
    pub last_expanded_snapshot: HashMap<WorkspaceId, HashSet<PathBuf>>,
    // Layout-change subscriber (window.rs). Invoked with the new desired
    // width whenever the visible tree changes (workspace switch, expand /
    // collapse toggle, hidden-files toggle, git/watcher refresh). The
    // callback receives only an `i32` so it is safe to invoke while any
    // borrow on `Inner` is held, provided the callback never re-borrows
    // `Inner` itself — the window-level callback only touches the
    // GTK paned and window handles.
    pub on_layout_changed: Option<Box<dyn Fn(i32)>>,
}

#[derive(Clone)]
pub struct FilePanelHandle {
    inner: Rc<RefCell<Inner>>,
    untitled_counter: Rc<AtomicU32>,
}

impl FilePanelHandle {
    pub fn new() -> Self {
        let root_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root_box.add_css_class("limux-fp-root");
        let header = build_header();
        let view = build_list_view();
        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_child(Some(&view.list_view));
        let sticky =
            crate::file_panel::view::build_sticky_overlay(&view.list_view, &scrolled, &view.store);
        root_box.append(&header.root);
        root_box.append(&sticky);
        root_box.append(&scrolled);
        Self {
            inner: Rc::new(RefCell::new(Inner {
                root_box,
                header,
                scrolled,
                view,
                sticky,
                clipboard: Clipboard::default(),
                cache: HashMap::new(),
                active: None,
                visible: false,
                last_expanded_snapshot: HashMap::new(),
                on_layout_changed: None,
            })),
            untitled_counter: Rc::new(AtomicU32::new(0)),
        }
    }

    pub fn widget(&self) -> gtk::Widget {
        self.inner.borrow().root_box.clone().upcast()
    }

    pub fn install_css(provider_data: &mut String) {
        provider_data.push_str(file_panel_css());
    }

    pub fn show_workspace(&self, workspace_id: WorkspaceId, root: PathBuf, expanded: Vec<PathBuf>) {
        {
            let inner = self.inner.borrow();
            if inner.active.as_ref() == Some(&workspace_id) {
                return;
            }
        }
        if !self.inner.borrow().cache.contains_key(&workspace_id) {
            let mut model = TreeModel::new(root.clone());
            for p in &expanded {
                model.expanded_paths.insert(p.clone());
            }
            model.rebuild_visible();
            let expanded_paths: Vec<PathBuf> = expanded.clone();
            for p in expanded_paths {
                if let Some(idx) = model.find_row(&p) {
                    if !model.rows[idx].expanded {
                        model.toggle_expand(idx);
                    }
                }
            }
            let (tx, rx) = mpsc::channel::<Vec<PathBuf>>();
            let watcher_handle = watcher::spawn(root.clone(), tx);

            // Bridge watcher events into the GTK main loop. The watcher
            // thread sends `Vec<PathBuf>` batches on `rx`; a main-thread
            // timeout source drains the channel and dispatches into
            // `on_watcher_event`. `timeout_add_local` accepts non-Send
            // closures, so we can safely capture both `self` (Rc) and the
            // receiver here without crossing thread boundaries.
            let self_clone = self.clone();
            let id_clone = workspace_id.clone();
            glib::timeout_add_local(Duration::from_millis(100), move || {
                let mut batches: Vec<Vec<PathBuf>> = Vec::new();
                loop {
                    match rx.try_recv() {
                        Ok(paths) => batches.push(paths),
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            for paths in batches {
                                self_clone.on_watcher_event(&id_clone, paths);
                            }
                            return glib::ControlFlow::Break;
                        }
                    }
                }
                for paths in batches {
                    self_clone.on_watcher_event(&id_clone, paths);
                }
                glib::ControlFlow::Continue
            });

            let mut inner = self.inner.borrow_mut();
            inner.cache.insert(
                workspace_id.clone(),
                PerWorkspace {
                    model,
                    watcher: watcher_handle,
                },
            );
        }
        {
            let inner = self.inner.borrow();
            let per = inner.cache.get(&workspace_id).unwrap();
            apply_model_to_store(&per.model, &inner.view.store);
            let title = root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| root.display().to_string());
            inner.header.title.set_text(&title);
        }
        eprintln!(
            "limux: fp-show-workspace id={} root={}",
            workspace_id,
            root.display()
        );
        self.inner.borrow_mut().active = Some(workspace_id.clone());

        self.refresh_git_for(workspace_id);
        self.notify_layout_changed();
    }

    pub fn toggle_visible(&self) {
        let visible = {
            let mut inner = self.inner.borrow_mut();
            inner.visible = !inner.visible;
            inner.visible
        };
        self.inner.borrow().root_box.set_visible(visible);
    }

    pub fn set_visible(&self, v: bool) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.visible = v;
        }
        self.inner.borrow().root_box.set_visible(v);
    }

    #[allow(dead_code)]
    pub fn is_visible(&self) -> bool {
        self.inner.borrow().visible
    }

    /// Width (px) needed so the longest visible filename in the active
    /// workspace fits without truncation. Falls back to 200 when no
    /// workspace is active or the cache entry is missing.
    pub fn desired_width(&self) -> i32 {
        let inner = self.inner.borrow();
        let active = match inner.active.as_ref() {
            Some(a) => a,
            None => return 200,
        };
        let per = match inner.cache.get(active) {
            Some(p) => p,
            None => return 200,
        };
        compute_desired_width(&per.model)
    }

    /// Register a callback fired whenever the visible tree changes and
    /// the desired panel width may have shifted. The callback is invoked
    /// on the main thread with the new width (already clamped).
    pub fn set_on_layout_changed<F: Fn(i32) + 'static>(&self, cb: F) {
        self.inner.borrow_mut().on_layout_changed = Some(Box::new(cb));
    }

    fn notify_layout_changed(&self) {
        let width = self.desired_width();
        let inner = self.inner.borrow();
        if let Some(cb) = inner.on_layout_changed.as_ref() {
            cb(width);
        }
    }

    fn on_watcher_event(&self, workspace_id: &str, paths: Vec<PathBuf>) {
        let mut touched_git = false;
        let mut should_apply = false;
        {
            let mut inner = self.inner.borrow_mut();
            let active_now = inner.active.as_deref() == Some(workspace_id);
            if let Some(per) = inner.cache.get_mut(workspace_id) {
                for p in &paths {
                    if let Some(parent) = p.parent() {
                        per.model.refresh_subtree(parent);
                    }
                    if p.components().any(|c| c.as_os_str() == ".git") {
                        touched_git = true;
                    }
                }
                if active_now {
                    should_apply = true;
                }
            }
        }
        if should_apply {
            let inner = self.inner.borrow();
            if let Some(per) = inner.cache.get(workspace_id) {
                apply_model_to_store(&per.model, &inner.view.store);
            }
        }
        if touched_git {
            self.refresh_git_for(workspace_id.to_string());
        }
    }

    // NOTE: T32 deviation — `git status` runs synchronously on the main
    // thread. The plan called for a tokio offload, but limux does not
    // depend on tokio. A `std::thread::spawn` bridge would also need a
    // thread→main hop, but every cross-thread glib API requires `Send`
    // closures, while `FilePanelHandle` wraps `Rc<RefCell<_>>` and is not
    // `Send`. `git status` is typically <100 ms; async offload is
    // deferred to T35 (production wiring), where a thread-local registry
    // or `async_channel`-based bridge can be introduced cleanly.
    fn refresh_git_for(&self, workspace_id: WorkspaceId) {
        let root = match self
            .inner
            .borrow()
            .cache
            .get(&workspace_id)
            .map(|p| p.model.root.clone())
        {
            Some(r) => r,
            None => return,
        };
        let map = match crate::file_panel::git::run_status(&root) {
            Ok(m) => m,
            Err(_) => return,
        };
        {
            let mut inner = self.inner.borrow_mut();
            let active = inner.active.as_deref() == Some(workspace_id.as_str());
            let store = inner.view.store.clone();
            if let Some(per) = inner.cache.get_mut(&workspace_id) {
                per.model.set_git_status_map(map);
                per.model.rebuild_visible();
                if active {
                    let rows: Vec<crate::file_panel::row_object::RowObject> = per
                        .model
                        .rows
                        .iter()
                        .map(crate::file_panel::row_object::RowObject::from_row)
                        .collect();
                    store.remove_all();
                    for r in &rows {
                        store.append(r);
                    }
                }
            }
        }
        self.notify_layout_changed();
    }
}

impl FilePanelHandle {
    pub fn wire_interactions(&self, window: &gtk::ApplicationWindow) {
        self.wire_list_activate();
        self.wire_context_menu();
        self.wire_header_buttons();
        self.wire_actions(window);
    }

    fn wire_list_activate(&self) {
        let store = self.inner.borrow().view.store.clone();
        let list_view = self.inner.borrow().view.list_view.clone();
        let handle = self.clone();
        list_view.connect_activate(move |_, position| {
            let row_obj = match store
                .item(position)
                .and_then(|o| o.downcast::<RowObject>().ok())
            {
                Some(o) => o,
                None => return,
            };
            handle.toggle_expand_path(&row_obj.path());
        });
    }

    fn toggle_expand_path(&self, path: &Path) {
        {
            let mut inner = self.inner.borrow_mut();
            let active = match inner.active.clone() {
                Some(a) => a,
                None => return,
            };
            let store = inner.view.store.clone();
            if let Some(per) = inner.cache.get_mut(&active) {
                if let Some(idx) = per.model.find_row(path) {
                    per.model.toggle_expand(idx);
                    apply_model_to_store(&per.model, &store);
                }
            }
        }
        self.notify_layout_changed();
    }

    fn wire_context_menu(&self) {
        let menu_model = crate::file_panel::actions::build_context_menu();
        let list_view = self.inner.borrow().view.list_view.clone();
        let popover = gtk::PopoverMenu::from_model(Some(&menu_model));
        popover.set_has_arrow(false);
        popover.set_parent(&list_view);
        let popover_clone = popover.clone();
        let gesture = gtk::GestureClick::new();
        gesture.set_button(gtk4::gdk::BUTTON_SECONDARY);
        gesture.connect_pressed(move |g, _, x, y| {
            let rect = gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1);
            popover_clone.set_pointing_to(Some(&rect));
            popover_clone.popup();
            g.set_state(gtk::EventSequenceState::Claimed);
        });
        list_view.add_controller(gesture);
    }

    fn wire_header_buttons(&self) {
        let inner = self.inner.borrow();
        inner
            .header
            .new_file
            .set_action_name(Some("win.fp-new-file"));
        inner
            .header
            .new_folder
            .set_action_name(Some("win.fp-new-folder"));
        inner
            .header
            .collapse_all
            .set_action_name(Some("win.fp-collapse-all"));
        inner
            .header
            .toggle_hidden
            .set_action_name(Some("win.fp-toggle-hidden"));
    }

    fn wire_actions(&self, window: &gtk::ApplicationWindow) {
        let handle = self.clone();
        let dispatch = move |name: &str| handle.dispatch_action(name);
        crate::file_panel::actions::register_all(window, dispatch);
    }

    fn dispatch_action(&self, name: &str) {
        match name {
            "fp-new-file" => self.do_new_file(),
            "fp-new-folder" => self.do_new_folder(),
            "fp-rename" => self.do_rename(),
            "fp-delete" => self.do_delete(false),
            "fp-delete-permanent" => self.do_delete(true),
            "fp-duplicate" => self.do_duplicate(),
            "fp-cut" => self.do_clip(ClipMode::Cut),
            "fp-copy" => self.do_clip(ClipMode::Copy),
            "fp-paste" => self.do_paste(),
            "fp-reveal-in-fm" => self.do_reveal(),
            "fp-open-in-terminal" => {}
            "fp-copy-path" => {}
            "fp-copy-relative-path" => {}
            "fp-collapse-all" => self.do_collapse_all_or_restore(),
            "fp-expand-all" => {}
            "fp-toggle-hidden" => self.do_toggle_hidden(),
            "fp-refresh" => self.do_refresh(),
            _ => {}
        }
    }

    fn selected_paths(&self) -> Vec<PathBuf> {
        let inner = self.inner.borrow();
        let sel = &inner.view.selection;
        let n = inner.view.store.n_items();
        let mut paths = Vec::new();
        for i in 0..n {
            if sel.is_selected(i) {
                if let Some(obj) = inner
                    .view
                    .store
                    .item(i)
                    .and_then(|o| o.downcast::<RowObject>().ok())
                {
                    paths.push(obj.path());
                }
            }
        }
        paths
    }

    fn current_root(&self) -> Option<PathBuf> {
        let inner = self.inner.borrow();
        let active = inner.active.as_ref()?;
        inner.cache.get(active).map(|p| p.model.root.clone())
    }

    fn target_parent(&self, root: &Path) -> PathBuf {
        self.selected_paths()
            .first()
            .cloned()
            .map(|p| {
                if p.is_dir() {
                    p
                } else {
                    p.parent().unwrap_or(root).to_path_buf()
                }
            })
            .unwrap_or_else(|| root.to_path_buf())
    }

    fn prompt_name(&self, _title: &str, default: &str) -> Option<String> {
        let n = self.untitled_counter.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        Some(format!("{default}-{n}-{nanos}"))
    }

    fn do_new_file(&self) {
        let root = match self.current_root() {
            Some(r) => r,
            None => return,
        };
        let parent = self.target_parent(&root);
        let name = match self.prompt_name("New file", "untitled.txt") {
            Some(n) => n,
            None => return,
        };
        let _ = crate::file_panel::ops::new_file(&root, &parent, &name);
    }

    fn do_new_folder(&self) {
        let root = match self.current_root() {
            Some(r) => r,
            None => return,
        };
        let parent = self.target_parent(&root);
        let name = match self.prompt_name("New folder", "untitled") {
            Some(n) => n,
            None => return,
        };
        let _ = crate::file_panel::ops::new_folder(&root, &parent, &name);
    }

    fn do_rename(&self) {
        let root = match self.current_root() {
            Some(r) => r,
            None => return,
        };
        let paths = self.selected_paths();
        if paths.len() != 1 {
            return;
        }
        let current = paths[0].file_name().and_then(|s| s.to_str()).unwrap_or("");
        let new = match self.prompt_name("Rename", current) {
            Some(n) => n,
            None => return,
        };
        let _ = crate::file_panel::ops::rename(&root, &paths[0], &new);
    }

    fn do_delete(&self, permanent: bool) {
        let root = match self.current_root() {
            Some(r) => r,
            None => return,
        };
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        let _ = if permanent {
            crate::file_panel::ops::delete_permanent(&root, &paths)
        } else {
            crate::file_panel::ops::delete(&root, &paths)
        };
    }

    fn do_duplicate(&self) {
        let root = match self.current_root() {
            Some(r) => r,
            None => return,
        };
        for p in self.selected_paths() {
            let _ = crate::file_panel::ops::duplicate(&root, &p);
        }
    }

    fn do_clip(&self, mode: ClipMode) {
        let paths = self.selected_paths();
        self.inner.borrow_mut().clipboard.set(paths, mode);
    }

    fn do_paste(&self) {
        let root = match self.current_root() {
            Some(r) => r,
            None => return,
        };
        let dst = self.target_parent(&root);
        let (sources, mode) = self.inner.borrow_mut().clipboard.take_for_paste();
        if let Some(mode) = mode {
            let _ = crate::file_panel::ops::paste(&root, &sources, mode, &dst);
        }
    }

    fn do_reveal(&self) {
        for p in self.selected_paths() {
            let _ = crate::file_panel::ops::reveal_in_fm(&p);
        }
    }

    fn do_collapse_all_or_restore(&self) {
        // Inspect current state under a shared borrow, then take a mutable
        // borrow for the write. Avoids overlapping borrows on `inner`.
        let plan = {
            let inner = self.inner.borrow();
            let active = match inner.active.clone() {
                Some(a) => a,
                None => return,
            };
            let per = match inner.cache.get(&active) {
                Some(p) => p,
                None => return,
            };
            let is_collapsed = per.model.expanded_paths.is_empty();
            if is_collapsed {
                let snapshot = inner.last_expanded_snapshot.get(&active).cloned();
                (active, true, snapshot)
            } else {
                let current = per.model.expanded_paths.clone();
                (active, false, Some(current))
            }
        };
        let (active, is_restore, payload) = plan;
        let count_before;
        let count_after;
        {
            let mut inner = self.inner.borrow_mut();
            let store = inner.view.store.clone();
            if is_restore {
                let Some(snapshot) = payload else {
                    eprintln!("limux: fp-collapse-all restore skipped (no snapshot)");
                    return;
                };
                let Some(per) = inner.cache.get_mut(&active) else {
                    return;
                };
                count_before = per.model.expanded_paths.len();
                per.model.expanded_paths = snapshot;
                per.model.rebuild_visible();
                count_after = per.model.expanded_paths.len();
                apply_model_to_store(&per.model, &store);
            } else {
                if let Some(snapshot) = payload.clone() {
                    inner
                        .last_expanded_snapshot
                        .insert(active.clone(), snapshot);
                }
                let Some(per) = inner.cache.get_mut(&active) else {
                    return;
                };
                count_before = per.model.expanded_paths.len();
                per.model.expanded_paths.clear();
                per.model.rebuild_visible();
                count_after = per.model.expanded_paths.len();
                apply_model_to_store(&per.model, &store);
            }
        }
        eprintln!(
            "limux: fp-collapse-all active={active} restore={is_restore} expanded_before={count_before} after={count_after}"
        );
        self.notify_layout_changed();
    }

    fn do_toggle_hidden(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            let active = match inner.active.clone() {
                Some(a) => a,
                None => return,
            };
            let store = inner.view.store.clone();
            if let Some(per) = inner.cache.get_mut(&active) {
                let new = !per.model.hidden_visible;
                per.model.set_hidden_visible(new);
                per.model.rebuild_visible();
                apply_model_to_store(&per.model, &store);
            }
        }
        self.notify_layout_changed();
    }

    fn do_refresh(&self) {
        let active = self.inner.borrow().active.clone();
        if let Some(active) = active {
            self.refresh_git_for(active);
        }
    }
}

impl Default for FilePanelHandle {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_desired_width(model: &crate::file_panel::model::TreeModel) -> i32 {
    const INDENT_PX: i32 = 16;
    const CHEVRON_PX: i32 = 18;
    const ICON_PX: i32 = 22;
    const GIT_MARKER_PX: i32 = 22;
    const PADDING_PX: i32 = 28;
    const CHAR_PX: i32 = 8;
    const MIN_WIDTH: i32 = 200;
    const MAX_WIDTH: i32 = 600;
    let mut max_text = 0;
    for row in &model.rows {
        let name_chars = row
            .path
            .file_name()
            .map(|s| s.to_string_lossy().chars().count() as i32)
            .unwrap_or(0);
        let depth = row.depth as i32;
        let row_text_w = depth * INDENT_PX + name_chars * CHAR_PX;
        if row_text_w > max_text {
            max_text = row_text_w;
        }
    }
    let total = max_text + CHEVRON_PX + ICON_PX + GIT_MARKER_PX + PADDING_PX;
    total.clamp(MIN_WIDTH, MAX_WIDTH)
}
