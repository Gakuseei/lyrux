use std::path::{Path, PathBuf};

use crate::file_panel::clipboard::ClipMode;
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

#[allow(dead_code)]
pub fn duplicate(root: &Path, src: &Path) -> Result<PathBuf, OpError> {
    if !is_within_root(src, root) {
        return Err(OpError::OutOfRoot);
    }
    let parent = src.parent().ok_or(OpError::OutOfRoot)?;
    let (stem, ext) = split_name(src);
    let mut n = 1u32;
    loop {
        let candidate_name = match &ext {
            Some(e) => format!("{stem} ({n}).{e}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = parent.join(&candidate_name);
        if !candidate.exists() {
            copy_recursive(src, &candidate)?;
            return Ok(candidate);
        }
        n += 1;
    }
}

fn split_name(p: &Path) -> (String, Option<String>) {
    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if p.is_dir() {
        return (name.to_string(), None);
    }
    match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), Some(name[i + 1..].to_string())),
        _ => (name.to_string(), None),
    }
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<(), OpError> {
    if src.is_dir() {
        std::fs::create_dir(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let child_src = entry.path();
            let child_dst = dst.join(entry.file_name());
            copy_recursive(&child_src, &child_dst)?;
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn paste(
    root: &Path,
    sources: &[PathBuf],
    mode: ClipMode,
    dst_dir: &Path,
) -> Result<Vec<PathBuf>, OpError> {
    if !is_within_root(dst_dir, root) {
        return Err(OpError::OutOfRoot);
    }
    let mut produced = Vec::new();
    for src in sources {
        if !is_within_root(src, root) {
            return Err(OpError::OutOfRoot);
        }
        let target = unique_target(dst_dir, src);
        copy_recursive(src, &target)?;
        if matches!(mode, ClipMode::Cut) {
            if src.is_dir() {
                std::fs::remove_dir_all(src)?;
            } else {
                std::fs::remove_file(src)?;
            }
        }
        produced.push(target);
    }
    Ok(produced)
}

fn unique_target(dst_dir: &Path, src: &Path) -> PathBuf {
    let initial = dst_dir.join(src.file_name().unwrap_or_default());
    if !initial.exists() {
        return initial;
    }
    let (stem, ext) = split_name(src);
    let mut n = 1u32;
    loop {
        let name = match &ext {
            Some(e) => format!("{stem} ({n}).{e}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = dst_dir.join(name);
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
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

    #[test]
    fn duplicate_file_uses_suffix_one() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let src = root.join("foo.rs");
        std::fs::write(&src, b"x").unwrap();
        let dup = duplicate(&root, &src).unwrap();
        assert_eq!(dup.file_name().unwrap(), "foo (1).rs");
        assert!(dup.exists());
    }

    #[test]
    fn duplicate_directory_uses_suffix_one() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let src = root.join("dir");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("inside.txt"), b"x").unwrap();
        let dup = duplicate(&root, &src).unwrap();
        assert_eq!(dup.file_name().unwrap(), "dir (1)");
        assert!(dup.is_dir());
        assert!(dup.join("inside.txt").exists());
    }

    #[test]
    fn duplicate_increments_suffix_when_taken() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        std::fs::write(root.join("a (1).txt"), b"y").unwrap();
        let dup = duplicate(&root, &root.join("a.txt")).unwrap();
        assert_eq!(dup.file_name().unwrap(), "a (2).txt");
    }

    #[test]
    fn paste_copy_keeps_source() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let src = root.join("a.txt");
        std::fs::write(&src, b"hi").unwrap();
        std::fs::create_dir(root.join("dst")).unwrap();
        let out = paste(
            &root,
            std::slice::from_ref(&src),
            ClipMode::Copy,
            &root.join("dst"),
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert!(src.exists());
        assert!(root.join("dst").join("a.txt").exists());
    }

    #[test]
    fn paste_cut_removes_source() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let src = root.join("a.txt");
        std::fs::write(&src, b"hi").unwrap();
        std::fs::create_dir(root.join("dst")).unwrap();
        paste(
            &root,
            std::slice::from_ref(&src),
            ClipMode::Cut,
            &root.join("dst"),
        )
        .unwrap();
        assert!(!src.exists());
        assert!(root.join("dst").join("a.txt").exists());
    }

    #[test]
    fn paste_skips_collisions_with_suffix() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let src = root.join("a.txt");
        std::fs::write(&src, b"hi").unwrap();
        std::fs::create_dir(root.join("dst")).unwrap();
        std::fs::write(root.join("dst").join("a.txt"), b"existing").unwrap();
        let out = paste(&root, &[src], ClipMode::Copy, &root.join("dst")).unwrap();
        assert_eq!(out[0].file_name().unwrap(), "a (1).txt");
    }
}
