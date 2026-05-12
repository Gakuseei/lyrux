use std::cell::RefCell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk4::prelude::*;

#[allow(dead_code)]
pub struct HeaderHandle {
    pub root: gtk::Box,
    pub title: gtk::Label,
    pub new_file: gtk::Button,
    pub new_folder: gtk::Button,
    pub collapse_all: gtk::Button,
    pub toggle_hidden: gtk::Button,
}

#[allow(dead_code)]
pub fn build_header() -> HeaderHandle {
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    root.set_margin_start(10);
    root.set_margin_end(8);
    root.set_margin_top(8);
    root.set_margin_bottom(6);
    root.add_css_class("limux-fp-header");

    let title = gtk::Label::new(None);
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("limux-fp-title");

    let new_file = make_icon_button("document-new-symbolic", "New file");
    let new_folder = make_icon_button("folder-new-symbolic", "New folder");
    let collapse_all = make_icon_button("view-restore-symbolic", "Collapse all");
    let toggle_hidden = make_icon_button("view-conceal-symbolic", "Toggle hidden");

    root.append(&title);
    root.append(&new_file);
    root.append(&new_folder);
    root.append(&collapse_all);
    root.append(&toggle_hidden);

    HeaderHandle {
        root,
        title,
        new_file,
        new_folder,
        collapse_all,
        toggle_hidden,
    }
}

#[allow(dead_code)]
fn make_icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let btn = gtk::Button::from_icon_name(icon);
    btn.set_tooltip_text(Some(tooltip));
    btn.add_css_class("flat");
    btn.add_css_class("limux-fp-icon");
    btn
}

#[allow(dead_code)]
pub struct ViewState {
    pub store: gtk4::gio::ListStore,
    pub selection: gtk::MultiSelection,
    pub list_view: gtk::ListView,
}

#[allow(dead_code)]
pub fn placeholder_state() -> Rc<RefCell<Option<ViewState>>> {
    Rc::new(RefCell::new(None))
}
