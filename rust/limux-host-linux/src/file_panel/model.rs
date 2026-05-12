#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Dir,
    File,
    Symlink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GitStatus {
    #[default]
    Clean,
    Modified,
    Added,
    Deleted,
    Untracked,
    Conflict,
    Ignored,
}

impl GitStatus {
    pub fn priority(self) -> u8 {
        match self {
            GitStatus::Conflict => 6,
            GitStatus::Modified => 5,
            GitStatus::Added => 4,
            GitStatus::Deleted => 3,
            GitStatus::Untracked => 2,
            GitStatus::Ignored => 1,
            GitStatus::Clean => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub path: PathBuf,
    pub depth: u32,
    pub kind: Kind,
    pub expanded: bool,
    pub git_status: GitStatus,
    pub parent_idx: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct TreeModel {
    pub root: PathBuf,
    pub rows: Vec<Row>,
    pub expanded_paths: HashSet<PathBuf>,
    pub hidden_visible: bool,
    pub git_status_map: HashMap<PathBuf, GitStatus>,
}

impl TreeModel {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            rows: Vec::new(),
            expanded_paths: HashSet::new(),
            hidden_visible: false,
            git_status_map: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_model_is_empty() {
        let m = TreeModel::new(PathBuf::from("/tmp"));
        assert_eq!(m.root, PathBuf::from("/tmp"));
        assert!(m.rows.is_empty());
        assert!(m.expanded_paths.is_empty());
        assert!(!m.hidden_visible);
        assert!(m.git_status_map.is_empty());
    }

    #[test]
    fn git_status_priority_ordering() {
        assert!(GitStatus::Modified.priority() > GitStatus::Untracked.priority());
        assert!(GitStatus::Conflict.priority() > GitStatus::Modified.priority());
        assert_eq!(GitStatus::Clean.priority(), 0);
    }

    #[test]
    fn kind_default_not_required() {
        assert_eq!(Kind::Dir, Kind::Dir);
    }
}
