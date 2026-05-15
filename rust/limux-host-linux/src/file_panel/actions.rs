use gtk4 as gtk;
use gtk4::gio;
use gtk4::prelude::*;

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
    ];
    for name in &names {
        let action = gio::SimpleAction::new(name, None);
        let dispatch = dispatch.clone();
        let n = *name;
        action.connect_activate(move |_, _| dispatch(n));
        window.add_action(&action);
    }
    ActionSet { names }
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
