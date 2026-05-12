use std::cell::RefCell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk4::prelude::*;

use crate::file_panel::model::{GitStatus, Kind, TreeModel};
use crate::file_panel::row_object::RowObject;

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

#[allow(dead_code)]
pub fn build_list_view() -> ViewState {
    let store = gtk4::gio::ListStore::new::<RowObject>();
    let selection = gtk::MultiSelection::new(Some(store.clone()));
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        row_box.add_css_class("limux-fp-row");
        let indent = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        indent.set_width_request(0);
        indent.add_css_class("limux-fp-indent");
        let chevron = gtk::Image::from_icon_name("pan-end-symbolic");
        chevron.add_css_class("limux-fp-chevron");
        let icon = gtk::Image::from_icon_name("text-x-generic-symbolic");
        icon.add_css_class("limux-fp-rowicon");
        let label = gtk::Label::new(None);
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.add_css_class("limux-fp-name");
        let marker = gtk::Label::new(None);
        marker.add_css_class("limux-fp-git");
        row_box.append(&indent);
        row_box.append(&chevron);
        row_box.append(&icon);
        row_box.append(&label);
        row_box.append(&marker);
        item.set_child(Some(&row_box));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row_obj = match item.item().and_then(|o| o.downcast::<RowObject>().ok()) {
            Some(o) => o,
            None => return,
        };
        let row_box = match item.child().and_then(|w| w.downcast::<gtk::Box>().ok()) {
            Some(b) => b,
            None => return,
        };
        bind_row(&row_box, &row_obj);
    });
    let list_view = gtk::ListView::new(Some(selection.clone()), Some(factory));
    list_view.add_css_class("limux-fp-listview");
    list_view.set_show_separators(false);
    ViewState {
        store,
        selection,
        list_view,
    }
}

#[allow(dead_code)]
fn bind_row(row_box: &gtk::Box, row_obj: &RowObject) {
    let mut children: Vec<gtk::Widget> = Vec::new();
    let mut child = row_box.first_child();
    while let Some(w) = child {
        child = w.next_sibling();
        children.push(w);
    }
    if children.len() != 5 {
        return;
    }
    let indent = children[0].clone();
    let chevron = children[1].clone().downcast::<gtk::Image>().unwrap();
    let icon = children[2].clone().downcast::<gtk::Image>().unwrap();
    let label = children[3].clone().downcast::<gtk::Label>().unwrap();
    let marker = children[4].clone().downcast::<gtk::Label>().unwrap();

    let depth = row_obj.depth();
    indent.set_width_request((depth as i32) * 16);
    for c in 0..=8 {
        let class = format!("limux-fp-depth-{c}");
        row_box.remove_css_class(&class);
    }
    let class = format!("limux-fp-depth-{}", depth.min(8));
    row_box.add_css_class(&class);

    let is_dir = matches!(row_obj.kind(), Kind::Dir);
    if is_dir {
        chevron.set_visible(true);
        chevron.set_icon_name(Some(if row_obj.expanded() {
            "pan-down-symbolic"
        } else {
            "pan-end-symbolic"
        }));
        icon.set_icon_name(Some("folder-symbolic"));
    } else {
        chevron.set_visible(false);
        icon.set_icon_name(Some("text-x-generic-symbolic"));
    }

    label.set_text(&row_obj.name());

    let (text, css) = git_marker_for(row_obj.git_status());
    marker.set_text(text);
    for c in [
        "limux-fp-git-m",
        "limux-fp-git-a",
        "limux-fp-git-d",
        "limux-fp-git-u",
        "limux-fp-git-c",
    ] {
        marker.remove_css_class(c);
    }
    if let Some(c) = css {
        marker.add_css_class(c);
    }
}

#[allow(dead_code)]
fn git_marker_for(s: GitStatus) -> (&'static str, Option<&'static str>) {
    match s {
        GitStatus::Modified => ("M", Some("limux-fp-git-m")),
        GitStatus::Added => ("A", Some("limux-fp-git-a")),
        GitStatus::Deleted => ("D", Some("limux-fp-git-d")),
        GitStatus::Untracked => ("?", Some("limux-fp-git-u")),
        GitStatus::Conflict => ("!", Some("limux-fp-git-c")),
        GitStatus::Ignored | GitStatus::Clean => ("", None),
    }
}

#[allow(dead_code)]
pub fn apply_model_to_store(model: &TreeModel, store: &gtk4::gio::ListStore) {
    store.remove_all();
    for row in &model.rows {
        let obj = RowObject::from_row(row);
        store.append(&obj);
    }
}

#[allow(dead_code)]
pub fn file_panel_css() -> &'static str {
    r#"
.limux-fp-header { background: transparent; color: #5a5a5a; }
.limux-fp-title { font-size: 10px; letter-spacing: 0.18em; text-transform: uppercase; color: #c0a060; }
.limux-fp-icon { padding: 2px; min-height: 0; min-width: 0; }
.limux-fp-listview row { padding: 0; }
.limux-fp-row { padding: 2px 8px 2px 6px; color: #8a8a8a; }
.limux-fp-row:selected { background: rgba(192, 160, 96, 0.12); color: #d0d0c8; }
.limux-fp-indent { background-image: linear-gradient(to right, transparent 7px, rgba(255,255,255,0.06) 7px, rgba(255,255,255,0.06) 8px, transparent 8px); }
.limux-fp-chevron { color: #555; }
.limux-fp-rowicon { color: #888; }
.limux-fp-name { color: #b8b8b0; }
.limux-fp-git { font-size: 9px; padding-left: 8px; }
.limux-fp-git-m { color: #c0a060; }
.limux-fp-git-a { color: #7aa67a; }
.limux-fp-git-d { color: #c08080; }
.limux-fp-git-u { color: #7a7a7a; }
.limux-fp-git-c { color: #c06060; }
"#
}

#[allow(dead_code)]
pub fn build_sticky_overlay(
    list_view: &gtk::ListView,
    scrolled: &gtk::ScrolledWindow,
    store: &gtk4::gio::ListStore,
) -> gtk::Box {
    let overlay = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    overlay.add_css_class("limux-fp-sticky");
    overlay.set_visible(false);

    let label = gtk::Label::new(None);
    label.set_xalign(0.0);
    overlay.append(&label);

    let list_view = list_view.clone();
    let store = store.clone();
    let label_clone = label.clone();
    let overlay_clone = overlay.clone();
    scrolled.vadjustment().connect_value_changed(move |adj| {
        update_sticky(
            &list_view,
            &store,
            adj.value(),
            &overlay_clone,
            &label_clone,
        );
    });

    overlay
}

#[allow(dead_code)]
fn update_sticky(
    _list_view: &gtk::ListView,
    store: &gtk4::gio::ListStore,
    scroll_y: f64,
    overlay: &gtk::Box,
    label: &gtk::Label,
) {
    if scroll_y <= 1.0 {
        overlay.set_visible(false);
        return;
    }
    let topmost = approximate_top_visible_index(scroll_y);
    let mut best: Option<RowObject> = None;
    let n = store.n_items();
    for i in (0..=topmost.min(n.saturating_sub(1))).rev() {
        if let Some(obj) = store.item(i).and_then(|o| o.downcast::<RowObject>().ok()) {
            if matches!(obj.kind(), Kind::Dir) {
                best = Some(obj);
                break;
            }
        }
    }
    match best {
        Some(obj) => {
            label.set_text(&obj.name());
            overlay.set_visible(true);
        }
        None => overlay.set_visible(false),
    }
}

#[allow(dead_code)]
fn approximate_top_visible_index(scroll_y: f64) -> u32 {
    let row_h = 22.0;
    (scroll_y / row_h).floor() as u32
}
