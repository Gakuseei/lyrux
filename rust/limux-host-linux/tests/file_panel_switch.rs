use std::fs;
use tempfile::TempDir;

use gtk4 as gtk;
use limux_host_linux::file_panel::FilePanelHandle;

#[test]
fn switch_workspace_swaps_root() {
    if gtk::init().is_err() {
        eprintln!("skipping: gtk init failed (no display)");
        return;
    }
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    fs::write(a.path().join("ina.txt"), b"a").unwrap();
    fs::write(b.path().join("inb.txt"), b"b").unwrap();
    let h = FilePanelHandle::new();
    h.show_workspace("A".into(), a.path().to_path_buf(), Vec::new());
    h.show_workspace("B".into(), b.path().to_path_buf(), Vec::new());
}
