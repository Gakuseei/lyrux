use std::path::{Path, PathBuf};

use crate::file_panel::model::is_within_root;

#[allow(dead_code)]
#[derive(Debug)]
pub enum OpError {
    OutOfRoot,
    AlreadyExists,
    Io(std::io::Error),
    Trash(trash::Error),
}

impl From<std::io::Error> for OpError {
    fn from(e: std::io::Error) -> Self {
        OpError::Io(e)
    }
}

impl From<trash::Error> for OpError {
    fn from(e: trash::Error) -> Self {
        OpError::Trash(e)
    }
}

#[allow(dead_code)]
pub fn new_file(root: &Path, parent: &Path, name: &str) -> Result<PathBuf, OpError> {
    let target = parent.join(name);
    if !is_within_root(&target, root) {
        return Err(OpError::OutOfRoot);
    }
    if target.exists() {
        return Err(OpError::AlreadyExists);
    }
    std::fs::File::create(&target)?;
    Ok(target)
}

#[allow(dead_code)]
pub fn new_folder(root: &Path, parent: &Path, name: &str) -> Result<PathBuf, OpError> {
    let target = parent.join(name);
    if !is_within_root(&target, root) {
        return Err(OpError::OutOfRoot);
    }
    if target.exists() {
        return Err(OpError::AlreadyExists);
    }
    std::fs::create_dir(&target)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn new_file_creates() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let p = new_file(&root, &root, "hello.txt").unwrap();
        assert!(p.exists());
        assert!(p.is_file());
    }

    #[test]
    fn new_file_blocks_out_of_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let err = new_file(&root, &root.join(".."), "hello.txt").unwrap_err();
        assert!(matches!(err, OpError::OutOfRoot));
    }

    #[test]
    fn new_file_rejects_existing() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        std::fs::File::create(root.join("x")).unwrap();
        let err = new_file(&root, &root, "x").unwrap_err();
        assert!(matches!(err, OpError::AlreadyExists));
    }

    #[test]
    fn new_folder_creates() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let p = new_folder(&root, &root, "subdir").unwrap();
        assert!(p.is_dir());
    }
}
