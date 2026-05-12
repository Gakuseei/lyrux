use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::file_panel::model::GitStatus;

#[derive(Debug)]
#[allow(dead_code)]
pub enum GitError {
    Timeout,
    Spawn(std::io::Error),
    NonZero(i32),
}

#[allow(dead_code)]
pub fn is_git_repo(root: &Path) -> bool {
    let out = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(root)
        .output();
    matches!(out, Ok(o) if o.status.success() && o.stdout.starts_with(b"true"))
}

#[allow(dead_code)]
pub fn run_status(root: &Path) -> Result<HashMap<PathBuf, GitStatus>, GitError> {
    let out = Command::new("git")
        .args(["status", "--porcelain=v2", "-z", "--untracked-files=all"])
        .current_dir(root)
        .output()
        .map_err(GitError::Spawn)?;
    if !out.status.success() {
        return Err(GitError::NonZero(out.status.code().unwrap_or(-1)));
    }
    Ok(parse_porcelain_v2(root, &out.stdout))
}

#[allow(dead_code)]
pub fn parse_porcelain_v2(root: &Path, output: &[u8]) -> HashMap<PathBuf, GitStatus> {
    let mut out = HashMap::new();
    for entry in output.split(|b| *b == 0) {
        if entry.is_empty() {
            continue;
        }
        if entry.first() == Some(&b'1') || entry.first() == Some(&b'2') {
            let parts: Vec<&[u8]> = entry.splitn(9, |b| *b == b' ').collect();
            if parts.len() < 9 {
                continue;
            }
            let xy = parts[1];
            let path_bytes = parts[8];
            let path = root.join(std::str::from_utf8(path_bytes).unwrap_or(""));
            out.insert(path, status_from_xy(xy));
        } else if entry.first() == Some(&b'?') {
            let rest = &entry[2..];
            let path = root.join(std::str::from_utf8(rest).unwrap_or(""));
            out.insert(path, GitStatus::Untracked);
        } else if entry.first() == Some(&b'!') {
            let rest = &entry[2..];
            let path = root.join(std::str::from_utf8(rest).unwrap_or(""));
            out.insert(path, GitStatus::Ignored);
        } else if entry.first() == Some(&b'u') {
            let parts: Vec<&[u8]> = entry.splitn(11, |b| *b == b' ').collect();
            if let Some(last) = parts.last() {
                let path = root.join(std::str::from_utf8(last).unwrap_or(""));
                out.insert(path, GitStatus::Conflict);
            }
        }
    }
    out
}

#[allow(dead_code)]
fn status_from_xy(xy: &[u8]) -> GitStatus {
    if xy.len() != 2 {
        return GitStatus::Modified;
    }
    let x = xy[0];
    let y = xy[1];
    if x == b'A' || y == b'A' {
        GitStatus::Added
    } else if x == b'D' || y == b'D' {
        GitStatus::Deleted
    } else {
        GitStatus::Modified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_untracked() {
        let root = PathBuf::from("/proj");
        let input = b"? src/new.rs\0";
        let m = parse_porcelain_v2(&root, input);
        assert_eq!(
            m.get(&PathBuf::from("/proj/src/new.rs")),
            Some(&GitStatus::Untracked)
        );
    }

    #[test]
    fn parses_modified_changed_entry() {
        let root = PathBuf::from("/proj");
        let input = b"1 .M N... 100644 100644 100644 abc def src/main.rs\0";
        let m = parse_porcelain_v2(&root, input);
        assert_eq!(
            m.get(&PathBuf::from("/proj/src/main.rs")),
            Some(&GitStatus::Modified)
        );
    }

    #[test]
    fn parses_added_entry() {
        let root = PathBuf::from("/proj");
        let input = b"1 A. N... 100644 100644 100644 abc def README.md\0";
        let m = parse_porcelain_v2(&root, input);
        assert_eq!(
            m.get(&PathBuf::from("/proj/README.md")),
            Some(&GitStatus::Added)
        );
    }

    #[test]
    fn parses_ignored() {
        let root = PathBuf::from("/proj");
        let input = b"! target/\0";
        let m = parse_porcelain_v2(&root, input);
        assert_eq!(
            m.get(&PathBuf::from("/proj/target/")),
            Some(&GitStatus::Ignored)
        );
    }

    #[test]
    fn empty_input_returns_empty_map() {
        let root = PathBuf::from("/proj");
        let m = parse_porcelain_v2(&root, b"");
        assert!(m.is_empty());
    }

    use tempfile::TempDir;

    #[test]
    fn is_git_repo_detects_repo() {
        let tmp = TempDir::new().unwrap();
        Command::new("git")
            .arg("init")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(is_git_repo(tmp.path()));
    }

    #[test]
    fn is_git_repo_returns_false_for_plain_dir() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_git_repo(tmp.path()));
    }

    #[test]
    fn run_status_in_repo_returns_map() {
        let tmp = TempDir::new().unwrap();
        Command::new("git")
            .arg("init")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::fs::write(tmp.path().join("a.txt"), "hello").unwrap();
        let m = run_status(tmp.path()).unwrap();
        let a = tmp.path().join("a.txt");
        assert_eq!(m.get(&a), Some(&GitStatus::Untracked));
    }
}
