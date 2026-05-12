use gtk4 as gtk;
use limux_host_linux::file_panel::FilePanelHandle;

#[test]
fn panel_widget_constructible() {
    if gtk::init().is_err() {
        eprintln!("skipping: gtk init failed (no display)");
        return;
    }
    let h = FilePanelHandle::new();
    let _w = h.widget();
}

#[test]
fn toggle_visible_flips_state() {
    if gtk::init().is_err() {
        eprintln!("skipping: gtk init failed (no display)");
        return;
    }
    let h = FilePanelHandle::new();
    h.set_visible(false);
    assert!(!h.is_visible());
    h.toggle_visible();
    assert!(h.is_visible());
}
