use gtk4 as gtk;
use gtk4::gio;
use gtk4::prelude::*;

#[allow(dead_code)]
pub struct ActionSet {
    pub names: Vec<&'static str>,
}

#[allow(dead_code)]
pub fn register_all<F>(window: &gtk::ApplicationWindow, dispatch: F) -> ActionSet
where
    F: Fn(&str) + 'static + Clone,
{
    let names = vec![
        "toggle-file-panel",
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
