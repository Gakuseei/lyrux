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
use std::rc::Rc;

use gtk4 as gtk;
use gtk4::prelude::*;

use crate::file_panel::clipboard::Clipboard;
use crate::file_panel::model::TreeModel;
use crate::file_panel::view::{
    build_header, build_list_view, file_panel_css, HeaderHandle, ViewState,
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
}

impl Default for FilePanelHandle {
    fn default() -> Self {
        Self::new()
    }
}
