use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Dir,
    File,
    Symlink,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub path: PathBuf,
    pub depth: u32,
    pub kind: Kind,
    pub expanded: bool,
    pub git_status: GitStatus,
    pub parent_idx: Option<usize>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct TreeModel {
    pub root: PathBuf,
    pub rows: Vec<Row>,
    pub expanded_paths: HashSet<PathBuf>,
    pub hidden_visible: bool,
    pub git_status_map: HashMap<PathBuf, GitStatus>,
}

#[allow(dead_code)]
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

    pub fn set_hidden_visible(&mut self, v: bool) {
        self.hidden_visible = v;
    }

    pub fn set_git_status_map(&mut self, map: HashMap<PathBuf, GitStatus>) {
        self.git_status_map = map;
    }

    fn rollup_dir_status(&self, dir: &Path) -> GitStatus {
        let prefix = dir;
        let mut best = GitStatus::Clean;
        for (p, s) in &self.git_status_map {
            if p.starts_with(prefix) && s.priority() > best.priority() {
                best = *s;
            }
        }
        best
    }

    pub fn rebuild_visible(&mut self) {
        self.rows.clear();
        let children = self.list_children(&self.root.clone(), 0, None);
        self.rows.extend(children);
    }

    pub fn toggle_expand(&mut self, idx: usize) {
        if idx >= self.rows.len() {
            return;
        }
        if self.rows[idx].kind != Kind::Dir {
            return;
        }
        if self.rows[idx].expanded {
            self.collapse_at(idx);
        } else {
            self.expand_at(idx);
        }
    }

    pub fn find_row(&self, path: &Path) -> Option<usize> {
        self.rows.iter().position(|r| r.path == path)
    }

    pub fn refresh_subtree(&mut self, parent_path: &Path) {
        if parent_path == self.root {
            let was_expanded: HashSet<PathBuf> = self.expanded_paths.clone();
            let saved = std::mem::take(&mut self.expanded_paths);
            self.rebuild_visible();
            self.expanded_paths = saved;
            for path in was_expanded {
                if let Some(idx) = self.find_row(&path) {
                    if !self.rows[idx].expanded {
                        self.toggle_expand(idx);
                    }
                }
            }
            return;
        }
        let parent_idx = match self.find_row(parent_path) {
            Some(idx) => idx,
            None => return,
        };
        if !self.rows[parent_idx].expanded {
            return;
        }
        let depth = self.rows[parent_idx].depth;
        self.rows[parent_idx].expanded = false;
        let mut end = parent_idx + 1;
        while end < self.rows.len() && self.rows[end].depth > depth {
            end += 1;
        }
        self.rows.drain(parent_idx + 1..end);
        self.expand_at(parent_idx);
    }

    fn expand_at(&mut self, idx: usize) {
        let path = self.rows[idx].path.clone();
        let depth = self.rows[idx].depth + 1;
        self.rows[idx].expanded = true;
        self.expanded_paths.insert(path.clone());
        let children = self.list_children(&path, depth, Some(idx));
        let insert_at = idx + 1;
        for (offset, child) in children.into_iter().enumerate() {
            self.rows.insert(insert_at + offset, child);
        }
        self.reindex_parents_after(insert_at);
    }

    fn collapse_at(&mut self, idx: usize) {
        let depth = self.rows[idx].depth;
        self.rows[idx].expanded = false;
        self.expanded_paths.remove(&self.rows[idx].path);
        let mut end = idx + 1;
        while end < self.rows.len() && self.rows[end].depth > depth {
            self.expanded_paths.remove(&self.rows[end].path);
            end += 1;
        }
        self.rows.drain(idx + 1..end);
        self.reindex_parents_after(idx + 1);
    }

    fn reindex_parents_after(&mut self, _from: usize) {}

    fn list_children(&self, dir: &Path, depth: u32, parent_idx: Option<usize>) -> Vec<Row> {
        let mut entries: Vec<(std::path::PathBuf, Kind, String)> = match std::fs::read_dir(dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let path = e.path();
                    let name = e.file_name().to_string_lossy().to_string();
                    if !self.hidden_visible && name.starts_with('.') {
                        return None;
                    }
                    let kind = classify(&path);
                    Some((path, kind, name))
                })
                .collect(),
            Err(err) => {
                eprintln!("limux: read_dir {} failed: {err}", dir.display());
                Vec::new()
            }
        };
        entries.sort_by(|a, b| {
            let a_dir = matches!(a.1, Kind::Dir);
            let b_dir = matches!(b.1, Kind::Dir);
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.2.to_lowercase().cmp(&b.2.to_lowercase()),
            }
        });
        entries
            .into_iter()
            .map(|(path, kind, _)| {
                let expanded = self.expanded_paths.contains(&path);
                let git_status = if matches!(kind, Kind::Dir) {
                    self.rollup_dir_status(&path)
                } else {
                    self.git_status_map
                        .get(&path)
                        .copied()
                        .unwrap_or(GitStatus::Clean)
                };
                Row {
                    path,
                    depth,
                    kind,
                    expanded,
                    git_status,
                    parent_idx,
                }
            })
            .collect()
    }
}

#[allow(dead_code)]
pub fn is_within_root(path: &Path, root: &Path) -> bool {
    let canon = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let parent = match path.parent() {
                Some(p) => p,
                None => return false,
            };
            let canon_parent = match parent.canonicalize() {
                Ok(p) => p,
                Err(_) => return false,
            };
            canon_parent.join(path.file_name().unwrap_or_default())
        }
    };
    canon.starts_with(root)
}

#[allow(dead_code)]
fn classify(path: &Path) -> Kind {
    let md = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return Kind::File,
    };
    if md.file_type().is_symlink() {
        Kind::Symlink
    } else if md.is_dir() {
        Kind::Dir
    } else {
        Kind::File
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

    use std::fs;
    use tempfile::TempDir;

    fn touch(dir: &Path, name: &str) {
        fs::File::create(dir.join(name)).unwrap();
    }

    fn mkdir(dir: &Path, name: &str) {
        fs::create_dir(dir.join(name)).unwrap();
    }

    #[test]
    fn rebuild_visible_lists_root_children_dirs_first() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "src");
        mkdir(root, "docs");
        touch(root, "Cargo.toml");
        touch(root, "README.md");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let names: Vec<&str> = m
            .rows
            .iter()
            .map(|r| r.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, vec!["docs", "src", "Cargo.toml", "README.md"]);
    }

    #[test]
    fn rebuild_visible_uses_case_insensitive_sort() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(root, "Apple.md");
        touch(root, "banana.md");
        touch(root, "Cherry.md");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let names: Vec<&str> = m
            .rows
            .iter()
            .map(|r| r.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, vec!["Apple.md", "banana.md", "Cherry.md"]);
    }

    #[test]
    fn rebuild_visible_marks_dirs_kind() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "src");
        touch(root, "main.rs");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        assert_eq!(m.rows[0].kind, Kind::Dir);
        assert_eq!(m.rows[1].kind, Kind::File);
    }

    #[test]
    fn toggle_expand_inserts_children_after_parent() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "src");
        let src = root.join("src");
        touch(&src, "main.rs");
        touch(&src, "lib.rs");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let src_idx = m.rows.iter().position(|r| r.path == src).unwrap();
        m.toggle_expand(src_idx);
        assert!(m.rows[src_idx].expanded);
        assert_eq!(m.rows.len(), 3);
        assert_eq!(m.rows[src_idx + 1].depth, 1);
        assert_eq!(m.rows[src_idx + 1].parent_idx, Some(src_idx));
    }

    #[test]
    fn toggle_expand_twice_collapses_subtree() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "a");
        let a = root.join("a");
        mkdir(&a, "b");
        let b = a.join("b");
        touch(&b, "c.txt");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let a_idx = m.rows.iter().position(|r| r.path == a).unwrap();
        m.toggle_expand(a_idx);
        let b_idx = m.rows.iter().position(|r| r.path == b).unwrap();
        m.toggle_expand(b_idx);
        assert_eq!(m.rows.len(), 3);
        m.toggle_expand(a_idx);
        assert_eq!(m.rows.len(), 1);
        assert!(!m.rows[0].expanded);
    }

    #[test]
    fn toggle_expand_on_file_is_noop() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(root, "x.txt");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        m.toggle_expand(0);
        assert_eq!(m.rows.len(), 1);
        assert!(!m.rows[0].expanded);
    }

    #[test]
    fn hidden_files_filtered_by_default() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(root, ".env");
        touch(root, "visible.txt");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        assert_eq!(m.rows.len(), 1);
        assert_eq!(m.rows[0].path.file_name().unwrap(), "visible.txt");
    }

    #[test]
    fn hidden_files_shown_when_toggled() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(root, ".env");
        touch(root, "visible.txt");
        let mut m = TreeModel::new(root.to_path_buf());
        m.set_hidden_visible(true);
        m.rebuild_visible();
        assert_eq!(m.rows.len(), 2);
    }

    #[test]
    fn toggling_hidden_after_rebuild_requires_rebuild() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(root, ".env");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        assert_eq!(m.rows.len(), 0);
        m.set_hidden_visible(true);
        m.rebuild_visible();
        assert_eq!(m.rows.len(), 1);
    }

    #[test]
    fn git_status_applied_to_files_on_rebuild() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(root, "main.rs");
        let mut m = TreeModel::new(root.to_path_buf());
        let mut map = HashMap::new();
        map.insert(root.join("main.rs"), GitStatus::Modified);
        m.set_git_status_map(map);
        m.rebuild_visible();
        assert_eq!(m.rows[0].git_status, GitStatus::Modified);
    }

    #[test]
    fn git_status_rolls_up_to_parent_dir_with_highest_priority() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "src");
        let src = root.join("src");
        touch(&src, "a.rs");
        touch(&src, "b.rs");
        let mut m = TreeModel::new(root.to_path_buf());
        let mut map = HashMap::new();
        map.insert(src.join("a.rs"), GitStatus::Untracked);
        map.insert(src.join("b.rs"), GitStatus::Modified);
        m.set_git_status_map(map);
        m.rebuild_visible();
        let src_row = m.rows.iter().find(|r| r.path == src).unwrap();
        assert_eq!(src_row.git_status, GitStatus::Modified);
    }

    #[test]
    fn find_row_returns_index_for_existing_path() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(root, "main.rs");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let idx = m.find_row(&root.join("main.rs"));
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn find_row_returns_none_for_missing_path() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        assert_eq!(m.find_row(&root.join("nope.rs")), None);
    }

    #[test]
    fn refresh_subtree_picks_up_new_file_in_expanded_dir() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "src");
        let src = root.join("src");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let src_idx = m.find_row(&src).unwrap();
        m.toggle_expand(src_idx);
        assert_eq!(m.rows.len(), 1);
        touch(&src, "new.rs");
        m.refresh_subtree(&src);
        assert_eq!(m.rows.len(), 2);
    }

    #[test]
    fn boundary_check_blocks_escape_via_parent_dir() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let bad = root.join("..").join("escape.txt");
        assert!(!is_within_root(&bad, &root));
    }

    #[test]
    fn boundary_check_allows_paths_inside_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let good = root.join("inside.txt");
        assert!(is_within_root(&good, &root));
    }
}
