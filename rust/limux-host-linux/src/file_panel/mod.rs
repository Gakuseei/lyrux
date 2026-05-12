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
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;

use crate::file_panel::clipboard::Clipboard;
use crate::file_panel::model::TreeModel;
use crate::file_panel::view::{
    apply_model_to_store, build_header, build_list_view, file_panel_css, HeaderHandle, ViewState,
};
use crate::file_panel::watcher::WatcherHandle;

pub type WorkspaceId = String;

#[allow(dead_code)]
pub struct PerWorkspace {
    pub model: TreeModel,
    pub watcher: Option<WatcherHandle>,
}

#[allow(dead_code)]
pub struct Inner {
    pub root_box: gtk::Box,
    pub header: HeaderHandle,
    pub scrolled: gtk::ScrolledWindow,
    pub view: ViewState,
    pub sticky: gtk::Box,
    pub clipboard: Clipboard,
    pub cache: HashMap<WorkspaceId, PerWorkspace>,
    pub active: Option<WorkspaceId>,
    pub visible: bool,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct FilePanelHandle {
    inner: Rc<RefCell<Inner>>,
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
            })),
        }
    }

    #[allow(dead_code)]
    pub fn widget(&self) -> gtk::Widget {
        self.inner.borrow().root_box.clone().upcast()
    }

    #[allow(dead_code)]
    pub fn install_css(provider_data: &mut String) {
        provider_data.push_str(file_panel_css());
    }

    #[allow(dead_code)]
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
            inner.header.title.set_text(&workspace_id);
        }
        self.inner.borrow_mut().active = Some(workspace_id.clone());

        self.refresh_git_for(workspace_id);
    }

    #[allow(dead_code)]
    pub fn toggle_visible(&self) {
        let visible = {
            let mut inner = self.inner.borrow_mut();
            inner.visible = !inner.visible;
            inner.visible
        };
        self.inner.borrow().root_box.set_visible(visible);
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
}

impl Default for FilePanelHandle {
    fn default() -> Self {
        Self::new()
    }
}
