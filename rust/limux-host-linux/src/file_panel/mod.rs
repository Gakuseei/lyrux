// In release builds the `perf_log!` macro expands to nothing, leaving the
// timer locals (`t0`, `t_refresh`, ...) and splice/descendant counters
// without readers. They are intentional debug-only instrumentation; suppress
// the resulting warnings only for non-debug builds.
#![cfg_attr(not(debug_assertions), allow(unused_variables, unused_assignments))]

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

// Perf instrumentation. Stripped in release builds (`cfg(debug_assertions)`
// gates the expansion to a no-op when optimizations are on). Use in place of
// raw `eprintln!("limux-perf: ...")` so production AppImage stays clean.
macro_rules! perf_log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            eprintln!($($arg)*);
        }
    };
}
pub(crate) use perf_log;

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
    apply_changes_to_store, apply_model_to_store, build_header, build_list_view, file_panel_css,
    HeaderHandle, ViewState,
};
use crate::file_panel::watcher::WatcherHandle;

pub type WorkspaceId = String;

pub(crate) const MAX_PATHS_PER_TICK: usize = 10_000;

pub struct PerWorkspace {
    pub model: TreeModel,
    // Held to keep the filesystem watcher thread alive for this workspace's lifetime.
    #[allow(dead_code)]
    pub watcher: Option<WatcherHandle>,
    // Removing this source drops the timeout closure which owns the watcher's
    // `rx`. The next watcher send then fails, the watcher thread exits, and the
    // debouncer is dropped on unwind. This is the only handle that stops the
    // watcher; `WatcherHandle` carries no state.
    pub timeout_source: Option<glib::SourceId>,
    // Coalescing flags for the async `git status` job. `git_in_flight` is
    // set when a worker thread is currently running `git status` for this
    // workspace; further refresh requests during that window flip
    // `git_rerun_pending` instead of spawning duplicate jobs. When the in-
    // flight job completes, the apply path checks `git_rerun_pending` and
    // kicks one fresh run.
    pub git_in_flight: bool,
    pub git_rerun_pending: bool,
    // Polling source for the async `git status` worker's result channel.
    // Stored so `hibernate_workspace` / `forget_workspace` can remove it
    // explicitly when the workspace goes away before the worker delivers.
    // Cleared back to `None` when the closure itself returns `Break`.
    pub git_poll_source: Option<glib::SourceId>,
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
    pub gitignore_cache: HashMap<WorkspaceId, std::rc::Rc<ignore::gitignore::Gitignore>>,
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
                gitignore_cache: HashMap::new(),
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
            let gitignore = {
                let mut inner = self.inner.borrow_mut();
                if let Some(existing) = inner.gitignore_cache.get(&workspace_id) {
                    Rc::clone(existing)
                } else {
                    let mut builder = ignore::gitignore::GitignoreBuilder::new(&root);
                    let _ = builder.add(root.join(".gitignore"));
                    let built = builder
                        .build()
                        .unwrap_or_else(|_| ignore::gitignore::Gitignore::empty());
                    let rc = Rc::new(built);
                    inner
                        .gitignore_cache
                        .insert(workspace_id.clone(), Rc::clone(&rc));
                    rc
                }
            };

            let mut model = TreeModel::new(root.clone());
            model.set_gitignore(Rc::clone(&gitignore));
            for p in &expanded {
                crate::file_panel::perf_log!(
                    "limux-perf: expanded_paths::insert(show_workspace seed) {:?}",
                    p
                );
                model.expanded_paths.insert(p.clone());
            }
            // Restore expansion captured by a prior `hibernate_workspace` call
            // so re-showing this workspace feels stateful. Explicit `expanded`
            // (from layout_state) and snapshot paths are unioned; duplicates
            // are idempotent because `HashSet::insert` and the toggle loop
            // below both no-op on already-expanded entries.
            let snapshot_expanded: Vec<PathBuf> = self
                .inner
                .borrow()
                .last_expanded_snapshot
                .get(&workspace_id)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default();
            for p in &snapshot_expanded {
                model.expanded_paths.insert(p.clone());
            }
            model.rebuild_visible();
            let mut all_expanded: Vec<PathBuf> = expanded.clone();
            all_expanded.extend(snapshot_expanded.iter().cloned());
            for p in all_expanded {
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
            let timeout_source = glib::timeout_add_local(Duration::from_millis(100), move || {
                let mut merged: Vec<PathBuf> = Vec::with_capacity(1024);
                loop {
                    match rx.try_recv() {
                        Ok(paths) => {
                            merged.extend(paths);
                            if merged.len() >= MAX_PATHS_PER_TICK {
                                self_clone.on_watcher_event(&id_clone, std::mem::take(&mut merged));
                            }
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            if !merged.is_empty() {
                                self_clone.on_watcher_event(&id_clone, merged);
                            }
                            return glib::ControlFlow::Break;
                        }
                    }
                }
                if !merged.is_empty() {
                    self_clone.on_watcher_event(&id_clone, merged);
                }
                glib::ControlFlow::Continue
            });

            let mut inner = self.inner.borrow_mut();
            inner.cache.insert(
                workspace_id.clone(),
                PerWorkspace {
                    model,
                    watcher: watcher_handle,
                    timeout_source: Some(timeout_source),
                    git_in_flight: false,
                    git_rerun_pending: false,
                    git_poll_source: None,
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
        self.inner.borrow_mut().active = Some(workspace_id.clone());

        // Evict every other cached workspace. The cache previously grew
        // unbounded — switching between workspaces left their TreeModel,
        // watcher thread, debouncer, gitignore matcher, and timeout sources
        // resident forever. `hibernate_workspace` drops the heavy state but
        // keeps a snapshot of expanded paths so a future re-show restores
        // the open folders.
        let to_hibernate: Vec<WorkspaceId> = self
            .inner
            .borrow()
            .cache
            .keys()
            .filter(|k| *k != &workspace_id)
            .cloned()
            .collect();
        for id in to_hibernate {
            self.hibernate_workspace(&id);
        }

        self.refresh_git_for(workspace_id);
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

    fn on_watcher_event(&self, workspace_id: &str, paths: Vec<PathBuf>) {
        let active_now = self.inner.borrow().active.as_deref() == Some(workspace_id);
        if !active_now {
            return;
        }
        // Drop paths matched by the workspace's `.gitignore`. Aria-class
        // workspaces write to many ignored dirs (`.superpowers/`,
        // `.playwright/`, `.todos/`, ...) and would otherwise drive a
        // refresh storm. Cheap component-level excludes still run earlier
        // in `watcher.rs`; this filter handles per-workspace ignores.
        let paths: Vec<PathBuf> = {
            let inner = self.inner.borrow();
            match inner.cache.get(workspace_id) {
                Some(per) => paths
                    .into_iter()
                    .filter(|p| !per.model.ignored_cache.contains(p))
                    .collect(),
                None => paths,
            }
        };
        if paths.is_empty() {
            return;
        }
        let mut touched_git = false;
        let mut parents: HashSet<PathBuf> = HashSet::new();
        for p in &paths {
            if let Some(parent) = p.parent() {
                parents.insert(parent.to_path_buf());
            }
            if p.components().any(|c| c.as_os_str() == ".git") {
                touched_git = true;
            }
        }
        // Visibility filter: only refresh parents that are currently visible
        // in the tree — i.e. the workspace root, or a path the user has
        // expanded. Watcher events for collapsed/never-opened subtrees have
        // nothing to update in the UI; refreshing them just burns main-thread
        // cycles and fights user clicks. Mirrors VSCode's behavior.
        let (visible_parents, root) = {
            let inner = self.inner.borrow();
            match inner.cache.get(workspace_id) {
                Some(per) => {
                    let root = per.model.root.clone();
                    let visible: Vec<PathBuf> = parents
                        .into_iter()
                        .filter(|p| p == &root || per.model.expanded_paths.contains(p))
                        .collect();
                    (visible, root)
                }
                None => return,
            }
        };
        if visible_parents.is_empty() {
            if touched_git {
                self.refresh_git_for(workspace_id.to_string());
            }
            return;
        }
        // Collapse threshold: many visible parents → one root refresh is
        // cheaper than N parent refreshes, and refresh_subtree(root) already
        // re-expands the saved set depth-first.
        let any_changed = {
            let mut inner = self.inner.borrow_mut();
            if let Some(per) = inner.cache.get_mut(workspace_id) {
                if visible_parents.len() > 5 {
                    per.model.refresh_subtree(&root)
                } else {
                    let mut changed = false;
                    for parent in &visible_parents {
                        changed |= per.model.refresh_subtree(parent);
                    }
                    changed
                }
            } else {
                false
            }
        };
        if any_changed {
            let inner = self.inner.borrow();
            if let Some(per) = inner.cache.get(workspace_id) {
                apply_model_to_store(&per.model, &inner.view.store);
            }
        } else {
            crate::file_panel::perf_log!(
                "limux-perf: on_watcher_event SKIP apply_model_to_store (no-op refresh)"
            );
        }
        if touched_git {
            self.refresh_git_for(workspace_id.to_string());
        }
    }

    /// Stop the watcher and drop the cached state for `workspace_id`. Removes
    /// the 100ms timeout source first (its closure owns the watcher channel
    /// receiver), then drops the cache entry, which releases the watcher
    /// thread and debouncer.
    pub fn forget_workspace(&self, workspace_id: &WorkspaceId) {
        let entry = self.inner.borrow_mut().cache.remove(workspace_id);
        if let Some(mut per) = entry {
            if let Some(src) = per.timeout_source.take() {
                src.remove();
            }
            if let Some(src) = per.git_poll_source.take() {
                src.remove();
            }
        }
        let mut inner = self.inner.borrow_mut();
        if inner.active.as_ref() == Some(workspace_id) {
            inner.active = None;
        }
        inner.last_expanded_snapshot.remove(workspace_id);
        inner.gitignore_cache.remove(workspace_id);
    }

    /// Switch-time cleanup: drop the heavy per-workspace state for
    /// `workspace_id` but keep a snapshot of its expanded paths so a future
    /// `show_workspace` call can restore the open folders. Unlike
    /// `forget_workspace` (close-time) this deliberately does NOT clear
    /// `inner.active` — it is called from inside `show_workspace` after a
    /// new active has already been set.
    ///
    /// Watcher shutdown chain: removing `timeout_source` drops the closure
    /// that owns the watcher channel's `rx`. The next watcher `send` fails,
    /// the watcher thread exits, and the debouncer drops on unwind.
    pub fn hibernate_workspace(&self, workspace_id: &WorkspaceId) {
        let entry = self.inner.borrow_mut().cache.remove(workspace_id);
        let Some(mut per) = entry else {
            return;
        };
        // Capture the expanded set BEFORE dropping `per` so a future re-show
        // can restore the tree. Replaces any prior snapshot (e.g. an empty
        // one written by collapse-all) — accepted trade per the leak fix.
        let snapshot = per.model.expanded_paths.clone();
        self.inner
            .borrow_mut()
            .last_expanded_snapshot
            .insert(workspace_id.clone(), snapshot);
        if let Some(src) = per.timeout_source.take() {
            src.remove();
        }
        if let Some(src) = per.git_poll_source.take() {
            src.remove();
        }
    }

    // `git status` runs on a worker thread spawned via `std::thread::spawn`.
    // The worker sends the resulting `HashMap<PathBuf, GitStatus>` back to
    // the main thread over an `mpsc::channel`. A short-lived
    // `glib::timeout_add_local` polls the receiver and dispatches into
    // `apply_git_result` once the map arrives. Coalescing (`git_in_flight`
    // + `git_rerun_pending`) prevents duplicate jobs from piling up during
    // a watcher burst.
    fn refresh_git_for(&self, workspace_id: WorkspaceId) {
        let t0 = std::time::Instant::now();
        let root = {
            let mut inner = self.inner.borrow_mut();
            let per = match inner.cache.get_mut(&workspace_id) {
                Some(p) => p,
                None => return,
            };
            if per.git_in_flight {
                per.git_rerun_pending = true;
                return;
            }
            per.git_in_flight = true;
            per.model.root.clone()
        };

        let (tx, rx) = mpsc::channel::<HashMap<PathBuf, crate::file_panel::model::GitStatus>>();
        let root_for_thread = root.clone();
        std::thread::spawn(move || {
            let t1 = std::time::Instant::now();
            if let Ok(map) = crate::file_panel::git::run_status(&root_for_thread) {
                crate::file_panel::perf_log!(
                    "limux-perf: run_status (worker thread) took {:?}",
                    t1.elapsed()
                );
                let _ = tx.send(map);
            } else {
                crate::file_panel::perf_log!(
                    "limux-perf: run_status (worker thread, err) took {:?}",
                    t1.elapsed()
                );
            }
        });

        let self_clone = self.clone();
        let id = workspace_id.clone();
        let src = glib::timeout_add_local(Duration::from_millis(50), move || match rx.try_recv() {
            Ok(map) => {
                crate::file_panel::perf_log!(
                    "limux-perf: refresh_git_for (spawn to result delivered) took {:?}",
                    t0.elapsed()
                );
                // Clear our stored SourceId BEFORE calling `apply_git_result`.
                // That call may synchronously re-enter `refresh_git_for` (if
                // `git_rerun_pending` is set) and assign a NEW SourceId; we
                // must not clobber it on the way out.
                if let Some(per) = self_clone.inner.borrow_mut().cache.get_mut(&id) {
                    per.git_poll_source = None;
                }
                self_clone.apply_git_result(&id, map);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                // Worker exited without sending (git status failed, or the
                // workspace was hibernated/forgotten while the worker was
                // running). Clear our state so future refreshes can run.
                let mut inner = self_clone.inner.borrow_mut();
                if let Some(per) = inner.cache.get_mut(&id) {
                    per.git_in_flight = false;
                    per.git_rerun_pending = false;
                    per.git_poll_source = None;
                }
                glib::ControlFlow::Break
            }
        });
        if let Some(per) = self.inner.borrow_mut().cache.get_mut(&workspace_id) {
            per.git_poll_source = Some(src);
        }
    }

    /// Apply a freshly-computed git status map to the workspace's model and,
    /// if it is active, the visible store. Runs on the GTK main thread.
    /// Uses `refresh_subtree(&root)` (not `rebuild_visible`) so previously-
    /// expanded folders survive the refresh — `rebuild_visible` alone only
    /// re-populates depth-0 rows and would collapse the tree.
    fn apply_git_result(
        &self,
        workspace_id: &str,
        map: HashMap<PathBuf, crate::file_panel::model::GitStatus>,
    ) {
        let t0 = std::time::Instant::now();
        let rerun;
        {
            let mut inner = self.inner.borrow_mut();
            let active = inner.active.as_deref() == Some(workspace_id);
            let store = inner.view.store.clone();
            if let Some(per) = inner.cache.get_mut(workspace_id) {
                per.model.set_git_status_map(map);
                let root = per.model.root.clone();
                let t_refresh = std::time::Instant::now();
                crate::file_panel::perf_log!(
                    "limux-perf: apply_git_result calling refresh_subtree ws={} root={:?}",
                    workspace_id,
                    root
                );
                let changed = per.model.refresh_subtree(&root);
                crate::file_panel::perf_log!(
                    "limux-perf: apply_git_result refresh_subtree took {:?} changed={}",
                    t_refresh.elapsed(),
                    changed
                );
                if active && changed {
                    let t_apply = std::time::Instant::now();
                    apply_model_to_store(&per.model, &store);
                    crate::file_panel::perf_log!(
                        "limux-perf: apply_git_result apply_model_to_store took {:?}",
                        t_apply.elapsed()
                    );
                }
                per.git_in_flight = false;
                rerun = std::mem::take(&mut per.git_rerun_pending);
            } else {
                return;
            }
        }
        crate::file_panel::perf_log!("limux-perf: apply_git_result total took {:?}", t0.elapsed());
        if rerun {
            self.refresh_git_for(workspace_id.to_string());
        }
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
            let t0 = std::time::Instant::now();
            let row_obj = match store
                .item(position)
                .and_then(|o| o.downcast::<RowObject>().ok())
            {
                Some(o) => o,
                None => return,
            };
            handle.toggle_expand_path(&row_obj.path());
            crate::file_panel::perf_log!(
                "limux-perf: list_view connect_activate (full click) took {:?}",
                t0.elapsed()
            );
        });
    }

    fn toggle_expand_path(&self, path: &Path) {
        let t0 = std::time::Instant::now();
        {
            let mut inner = self.inner.borrow_mut();
            let active = match inner.active.clone() {
                Some(a) => a,
                None => return,
            };
            let store = inner.view.store.clone();
            if let Some(per) = inner.cache.get_mut(&active) {
                if let Some(idx) = per.model.find_row(path) {
                    let t_toggle = std::time::Instant::now();
                    let change = per.model.toggle_expand(idx);
                    crate::file_panel::perf_log!(
                        "limux-perf: model.toggle_expand took {:?}",
                        t_toggle.elapsed()
                    );
                    let t_apply = std::time::Instant::now();
                    if let Some(change) = change {
                        apply_changes_to_store(&[change], &store);
                    }
                    crate::file_panel::perf_log!(
                        "limux-perf: apply_changes_to_store (after toggle) took {:?}",
                        t_apply.elapsed()
                    );
                }
            }
        }
        crate::file_panel::perf_log!(
            "limux-perf: toggle_expand_path total took {:?}",
            t0.elapsed()
        );
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
        {
            let mut inner = self.inner.borrow_mut();
            let store = inner.view.store.clone();
            if is_restore {
                let Some(snapshot) = payload else {
                    return;
                };
                let Some(per) = inner.cache.get_mut(&active) else {
                    return;
                };
                crate::file_panel::perf_log!(
                    "limux-perf: expanded_paths::assign(collapse_all restore) {:?}",
                    snapshot
                );
                per.model.expanded_paths = snapshot;
                per.model.rebuild_visible();
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
                crate::file_panel::perf_log!("limux-perf: expanded_paths::clear(collapse_all)");
                per.model.expanded_paths.clear();
                per.model.rebuild_visible();
                apply_model_to_store(&per.model, &store);
            }
        }
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
    const CHEVRON_PX: i32 = 20;
    const ICON_PX: i32 = 24;
    const GIT_MARKER_PX: i32 = 24;
    const PADDING_PX: i32 = 40;
    const CHAR_PX: i32 = 9;
    const BREATHING_PX: i32 = 24;
    const MIN_WIDTH: i32 = 260;
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
    let total = max_text + CHEVRON_PX + ICON_PX + GIT_MARKER_PX + PADDING_PX + BREATHING_PX;
    total.clamp(MIN_WIDTH, MAX_WIDTH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_panel::model::TreeModel;

    #[test]
    fn compute_desired_width_empty_model_returns_min() {
        let m = TreeModel::new(PathBuf::from("/tmp"));
        assert_eq!(compute_desired_width(&m), 260);
    }

    #[test]
    fn watcher_drain_dispatches_in_batches_of_at_most_10k() {
        assert_eq!(super::MAX_PATHS_PER_TICK, 10_000);
    }

    #[test]
    fn compute_desired_width_clamps_at_max() {
        let mut m = TreeModel::new(PathBuf::from("/tmp"));
        let long = "a".repeat(200);
        m.rows.push(crate::file_panel::model::Row {
            path: PathBuf::from(format!("/tmp/{long}")),
            depth: 0,
            kind: crate::file_panel::model::Kind::File,
            expanded: false,
            git_status: crate::file_panel::model::GitStatus::Clean,
            parent_idx: None,
            ignored: false,
        });
        assert_eq!(compute_desired_width(&m), 600);
    }

    // Tests below exercise the cache/hibernate machinery. They build a real
    // `FilePanelHandle`, which constructs `gtk::Box` and therefore needs GTK
    // initialized. The pattern matches the integration tests in
    // `tests/file_panel_*.rs`: skip cleanly when no display is available.
    use std::fs;
    use tempfile::TempDir;

    fn try_init_gtk() -> bool {
        gtk::init().is_ok()
    }

    #[test]
    fn hibernate_workspace_saves_expanded_paths_to_snapshot() {
        if !try_init_gtk() {
            eprintln!("skipping: gtk init failed (no display)");
            return;
        }
        let ws = TempDir::new().unwrap();
        let sub = ws.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("inner.txt"), b"x").unwrap();
        let h = FilePanelHandle::new();
        h.show_workspace("A".into(), ws.path().to_path_buf(), vec![sub.clone()]);

        h.hibernate_workspace(&"A".to_string());

        let inner = h.inner.borrow();
        let snap = inner
            .last_expanded_snapshot
            .get("A")
            .expect("snapshot present");
        assert!(snap.contains(&sub), "snapshot should contain expanded path");
    }

    #[test]
    fn hibernate_workspace_drops_cache_entry() {
        if !try_init_gtk() {
            eprintln!("skipping: gtk init failed (no display)");
            return;
        }
        let ws = TempDir::new().unwrap();
        let h = FilePanelHandle::new();
        h.show_workspace("A".into(), ws.path().to_path_buf(), Vec::new());
        assert!(h.inner.borrow().cache.contains_key("A"));

        h.hibernate_workspace(&"A".to_string());

        assert!(!h.inner.borrow().cache.contains_key("A"));
    }

    #[test]
    fn show_workspace_hibernates_previously_active() {
        if !try_init_gtk() {
            eprintln!("skipping: gtk init failed (no display)");
            return;
        }
        let a = TempDir::new().unwrap();
        let a_sub = a.path().join("sub");
        fs::create_dir(&a_sub).unwrap();
        fs::write(a_sub.join("f.txt"), b"a").unwrap();
        let b = TempDir::new().unwrap();
        fs::write(b.path().join("ib.txt"), b"b").unwrap();
        let h = FilePanelHandle::new();
        h.show_workspace("A".into(), a.path().to_path_buf(), vec![a_sub.clone()]);
        h.show_workspace("B".into(), b.path().to_path_buf(), Vec::new());

        let inner = h.inner.borrow();
        assert!(!inner.cache.contains_key("A"), "A should be hibernated");
        assert!(inner.cache.contains_key("B"), "B should be active");
        let snap_a = inner
            .last_expanded_snapshot
            .get("A")
            .expect("A snapshot present");
        assert!(
            snap_a.contains(&a_sub),
            "A's expanded path should survive in snapshot"
        );
    }

    #[test]
    fn show_workspace_restores_expansion_from_snapshot() {
        if !try_init_gtk() {
            eprintln!("skipping: gtk init failed (no display)");
            return;
        }
        let a = TempDir::new().unwrap();
        let sub = a.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("inner.txt"), b"x").unwrap();
        let h = FilePanelHandle::new();
        // First show with explicit expansion → snapshot picks it up via
        // hibernate (triggered by the second show below).
        h.show_workspace("A".into(), a.path().to_path_buf(), vec![sub.clone()]);
        let other = TempDir::new().unwrap();
        h.show_workspace("B".into(), other.path().to_path_buf(), Vec::new());
        // Re-show A with an EMPTY explicit expansion list — the snapshot
        // captured during hibernation must restore `sub`.
        h.show_workspace("A".into(), a.path().to_path_buf(), Vec::new());

        let inner = h.inner.borrow();
        let per = inner.cache.get("A").expect("A re-cached");
        assert!(
            per.model.expanded_paths.contains(&sub),
            "expansion should be restored from snapshot"
        );
    }
}
