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

#[allow(dead_code)]
pub fn rename(root: &Path, old: &Path, new_name: &str) -> Result<PathBuf, OpError> {
    let parent = old.parent().ok_or(OpError::OutOfRoot)?;
    let target = parent.join(new_name);
    if !is_within_root(&target, root) {
        return Err(OpError::OutOfRoot);
    }
    if target.exists() {
        return Err(OpError::AlreadyExists);
    }
    std::fs::rename(old, &target)?;
    Ok(target)
}

#[allow(dead_code)]
pub fn delete(root: &Path, paths: &[PathBuf]) -> Result<(), OpError> {
    for p in paths {
        if !is_within_root(p, root) {
            return Err(OpError::OutOfRoot);
        }
    }
    for p in paths {
        trash::delete(p)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn delete_permanent(root: &Path, paths: &[PathBuf]) -> Result<(), OpError> {
    for p in paths {
        if !is_within_root(p, root) {
            return Err(OpError::OutOfRoot);
        }
    }
    for p in paths {
        if p.is_dir() {
            std::fs::remove_dir_all(p)?;
        } else {
            std::fs::remove_file(p)?;
        }
    }
    Ok(())
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

    #[test]
    fn rename_within_root_ok() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        std::fs::File::create(root.join("a")).unwrap();
        let new = rename(&root, &root.join("a"), "b").unwrap();
        assert_eq!(new, root.join("b"));
        assert!(new.exists());
        assert!(!root.join("a").exists());
    }

    #[test]
    fn rename_collision_errors() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        std::fs::File::create(root.join("a")).unwrap();
        std::fs::File::create(root.join("b")).unwrap();
        let err = rename(&root, &root.join("a"), "b").unwrap_err();
        assert!(matches!(err, OpError::AlreadyExists));
    }

    #[test]
    fn delete_permanent_removes_file() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let p = root.join("a");
        std::fs::File::create(&p).unwrap();
        delete_permanent(&root, std::slice::from_ref(&p)).unwrap();
        assert!(!p.exists());
    }

    #[test]
    fn delete_permanent_blocks_out_of_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let outside = root.join("..").join("escape.txt");
        let err = delete_permanent(&root, &[outside]).unwrap_err();
        assert!(matches!(err, OpError::OutOfRoot));
    }

    #[test]
    fn delete_to_trash_removes_from_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let p = root.join("a");
        std::fs::File::create(&p).unwrap();
        let _ = delete(&root, std::slice::from_ref(&p));
        assert!(
            !p.exists() || std::fs::metadata(&p).is_err(),
            "file moved to trash or removed"
        );
    }
}
