use std::fs;
use tempfile::TempDir;

use limux_host_linux::file_panel::ops;

#[test]
fn new_file_action_creates_file() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let p = ops::new_file(&root, &root, "via_action.txt").unwrap();
    assert!(p.is_file());
    assert!(fs::read_to_string(&p).unwrap().is_empty());
}
