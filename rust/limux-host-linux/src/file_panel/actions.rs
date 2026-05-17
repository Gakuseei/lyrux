use gtk4 as gtk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;

pub const SORT_MODE_FOLDERS_FIRST: &str = "folders_first";
pub const SORT_MODE_NAME_ASC: &str = "name_asc";
pub const SORT_MODE_NAME_DESC: &str = "name_desc";
pub const SORT_MODE_MODIFIED_DESC: &str = "modified_desc";
pub const SORT_MODE_SIZE_DESC: &str = "size_desc";

pub struct ActionSet {
    #[allow(dead_code)]
    pub names: Vec<&'static str>,
}

pub fn register_all<F>(window: &gtk::ApplicationWindow, dispatch: F) -> ActionSet
where
    F: Fn(&str) + 'static + Clone,
{
    let names = vec![
        "fp-new-file",
        "fp-new-folder",
        "fp-rename",
        "fp-delete",
        "fp-delete-permanent",
        "fp-duplicate",
        "fp-cut",
        "fp-copy",
        "fp-paste",
        "fp-reveal-in-fm",
        "fp-open-in-terminal",
        "fp-copy-path",
        "fp-copy-relative-path",
        "fp-collapse-all",
        "fp-expand-all",
        "fp-toggle-hidden",
        "fp-refresh",
        "fp-open-in-new-pane",
    ];
    for name in &names {
        let action = gio::SimpleAction::new(name, None);
        let dispatch = dispatch.clone();
        let n = *name;
        action.connect_activate(move |_, _| dispatch(n));
        window.add_action(&action);
    }
    // Stateful sort-mode action: the menu items' radio markers track the
    // current state. Default state mirrors `SortMode::default()` so the
    // marker is correct on first paint without an extra round-trip.
    let sort_action = gio::SimpleAction::new_stateful(
        "fp-sort-mode",
        Some(glib::VariantTy::STRING),
        &SORT_MODE_FOLDERS_FIRST.to_variant(),
    );
    let dispatch_sort = dispatch.clone();
    sort_action.connect_activate(move |action, param| {
        let Some(target) = param.and_then(|v| v.str().map(String::from)) else {
            return;
        };
        action.set_state(&target.to_variant());
        let dispatched_name = sort_action_name_for(&target);
        if let Some(name) = dispatched_name {
            dispatch_sort(name);
        }
    });
    window.add_action(&sort_action);
    ActionSet { names }
}

pub fn sort_action_name_for(target: &str) -> Option<&'static str> {
    match target {
        SORT_MODE_FOLDERS_FIRST => Some("fp-sort-folders-first"),
        SORT_MODE_NAME_ASC => Some("fp-sort-name-asc"),
        SORT_MODE_NAME_DESC => Some("fp-sort-name-desc"),
        SORT_MODE_MODIFIED_DESC => Some("fp-sort-modified-desc"),
        SORT_MODE_SIZE_DESC => Some("fp-sort-size-desc"),
        _ => None,
    }
}

pub fn build_sort_menu() -> gio::Menu {
    use crate::file_panel::strings;
    let menu = gio::Menu::new();
    let entries: [(&str, &str); 5] = [
        (strings::SORT_FOLDERS_FIRST, SORT_MODE_FOLDERS_FIRST),
        (strings::SORT_NAME_ASC, SORT_MODE_NAME_ASC),
        (strings::SORT_NAME_DESC, SORT_MODE_NAME_DESC),
        (strings::SORT_MODIFIED_DESC, SORT_MODE_MODIFIED_DESC),
        (strings::SORT_SIZE_DESC, SORT_MODE_SIZE_DESC),
    ];
    for (label, target) in entries {
        let item = gio::MenuItem::new(Some(label), None);
        item.set_action_and_target_value(Some("win.fp-sort-mode"), Some(&target.to_variant()));
        menu.append_item(&item);
    }
    menu
}

pub fn build_context_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    menu.append(Some("New File"), Some("win.fp-new-file"));
    menu.append(Some("New Folder"), Some("win.fp-new-folder"));
    let section1 = gio::Menu::new();
    section1.append(Some("Rename"), Some("win.fp-rename"));
    section1.append(Some("Duplicate"), Some("win.fp-duplicate"));
    section1.append(Some("Delete"), Some("win.fp-delete"));
    menu.append_section(None, &section1);
    let section2 = gio::Menu::new();
    section2.append(Some("Cut"), Some("win.fp-cut"));
    section2.append(Some("Copy"), Some("win.fp-copy"));
    section2.append(Some("Paste"), Some("win.fp-paste"));
    section2.append(Some("Copy Path"), Some("win.fp-copy-path"));
    section2.append(
        Some("Copy Relative Path"),
        Some("win.fp-copy-relative-path"),
    );
    menu.append_section(None, &section2);
    let section3 = gio::Menu::new();
    section3.append(Some("Reveal in File Manager"), Some("win.fp-reveal-in-fm"));
    section3.append(Some("Open in Terminal"), Some("win.fp-open-in-terminal"));
    menu.append_section(None, &section3);
    let section4 = gio::Menu::new();
    section4.append(Some("Collapse All"), Some("win.fp-collapse-all"));
    section4.append(Some("Expand All"), Some("win.fp-expand-all"));
    section4.append(Some("Toggle Hidden Files"), Some("win.fp-toggle-hidden"));
    section4.append(Some("Refresh"), Some("win.fp-refresh"));
    menu.append_section(None, &section4);
    menu
}
