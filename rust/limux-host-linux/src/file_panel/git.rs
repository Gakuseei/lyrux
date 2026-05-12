use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::file_panel::model::GitStatus;

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
}
