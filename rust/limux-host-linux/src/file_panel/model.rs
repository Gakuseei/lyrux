use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

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
    pub ignored: bool,
}

#[derive(Clone, Debug)]
pub struct TreeModel {
    pub root: PathBuf,
    pub rows: Vec<Row>,
    pub expanded_paths: HashSet<PathBuf>,
    pub hidden_visible: bool,
    pub git_status_map: HashMap<PathBuf, GitStatus>,
    pub git_status_prefixes: Vec<(PathBuf, GitStatus)>,
    pub gitignore: Option<Rc<ignore::gitignore::Gitignore>>,
    pub ignored_cache: HashSet<PathBuf>,
}

#[derive(Clone, Debug)]
pub enum ListChange {
    Replace {
        at: u32,
        removed: u32,
        rows: Vec<Row>,
    },
}

impl TreeModel {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            rows: Vec::new(),
            expanded_paths: HashSet::new(),
            hidden_visible: false,
            git_status_map: HashMap::new(),
            git_status_prefixes: Vec::new(),
            gitignore: None,
            ignored_cache: HashSet::new(),
        }
    }

    pub fn set_hidden_visible(&mut self, v: bool) {
        self.hidden_visible = v;
    }

    pub fn set_gitignore(&mut self, gi: Rc<ignore::gitignore::Gitignore>) {
        self.gitignore = Some(gi);
        self.ignored_cache.clear();
    }

    fn is_ignored(&self, path: &Path, _is_dir: bool) -> bool {
        self.ignored_cache.contains(path)
    }

    fn populate_ignored_cache(&mut self, gi: &ignore::gitignore::Gitignore) {
        let mut stack: Vec<PathBuf> = vec![self.root.clone()];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let name = match path.file_name() {
                    Some(n) => n.to_os_string(),
                    None => continue,
                };
                if !self.hidden_visible && name.to_string_lossy().starts_with('.') {
                    continue;
                }
                if gi.matched(&path, is_dir).is_ignore() {
                    self.ignored_cache.insert(path);
                    continue;
                }
                if is_dir {
                    stack.push(path);
                }
            }
        }
    }

    pub fn set_git_status_map(&mut self, map: HashMap<PathBuf, GitStatus>) {
        let mut prefixes: Vec<(PathBuf, GitStatus)> =
            map.iter().map(|(p, s)| (p.clone(), *s)).collect();
        prefixes.sort_by(|a, b| a.0.cmp(&b.0));
        self.git_status_prefixes = prefixes;
        self.git_status_map = map;
    }

    fn rollup_dir_status(&self, dir: &Path) -> GitStatus {
        let lo = self
            .git_status_prefixes
            .partition_point(|(p, _)| p.as_path() < dir);
        let mut best = GitStatus::Clean;
        for (p, s) in &self.git_status_prefixes[lo..] {
            if !p.starts_with(dir) {
                break;
            }
            if s.priority() > best.priority() {
                best = *s;
            }
        }
        best
    }

    pub fn rebuild_visible(&mut self) {
        self.rows.clear();
        self.ignored_cache.clear();
        if let Some(gi) = self.gitignore.clone() {
            self.populate_ignored_cache(&gi);
        }
        let children = self.list_children(&self.root.clone(), 0, None);
        self.rows.extend(children);
    }

    pub fn toggle_expand(&mut self, idx: usize) -> Option<ListChange> {
        if idx >= self.rows.len() {
            return None;
        }
        if self.rows[idx].kind != Kind::Dir {
            return None;
        }
        if self.rows[idx].expanded {
            Some(self.collapse_at(idx))
        } else {
            Some(self.expand_at(idx))
        }
    }

    pub fn find_row(&self, path: &Path) -> Option<usize> {
        self.rows.iter().position(|r| r.path == path)
    }

    pub fn refresh_subtree(&mut self, parent_path: &Path) -> bool {
        crate::file_panel::perf_log!(
            "limux-perf: refresh_subtree ENTER path={:?} expanded_paths_len={}",
            parent_path,
            self.expanded_paths.len()
        );
        if parent_path == self.root {
            if self.tree_matches_fs() {
                crate::file_panel::perf_log!(
                    "limux-perf: refresh_subtree EXIT(root,no-fs-change) expanded_paths_len={} rows_len={}",
                    self.expanded_paths.len(),
                    self.rows.len()
                );
                return false;
            }
            let mut was_expanded: Vec<PathBuf> = self.expanded_paths.iter().cloned().collect();
            was_expanded.sort_by_key(|p| p.components().count());
            let saved = std::mem::take(&mut self.expanded_paths);
            self.rebuild_visible();
            self.expanded_paths = saved;
            for path in was_expanded {
                self.force_expand_at_path(&path);
            }
            crate::file_panel::perf_log!(
                "limux-perf: refresh_subtree EXIT(root) expanded_paths_len={} rows_len={}",
                self.expanded_paths.len(),
                self.rows.len()
            );
            return true;
        }
        let parent_idx = match self.find_row(parent_path) {
            Some(idx) => idx,
            None => return false,
        };
        if !self.rows[parent_idx].expanded {
            return false;
        }
        let depth = self.rows[parent_idx].depth;
        let mut end = parent_idx + 1;
        while end < self.rows.len() && self.rows[end].depth > depth {
            end += 1;
        }
        let mut deep_expanded: Vec<PathBuf> = self.rows[parent_idx + 1..end]
            .iter()
            .filter(|r| self.expanded_paths.contains(&r.path))
            .map(|r| r.path.clone())
            .collect();
        deep_expanded.sort_by_key(|p| p.components().count());
        self.rows[parent_idx].expanded = false;
        self.rows.drain(parent_idx + 1..end);
        self.ignored_cache.clear();
        if let Some(gi) = self.gitignore.clone() {
            self.populate_ignored_cache(&gi);
        }
        self.expand_at(parent_idx);
        for path in deep_expanded {
            self.force_expand_at_path(&path);
        }
        crate::file_panel::perf_log!(
            "limux-perf: refresh_subtree EXIT(parent) path={:?} expanded_paths_len={} rows_len={}",
            parent_path,
            self.expanded_paths.len(),
            self.rows.len()
        );
        true
    }

    /// Locate `path` in `rows` and force a fresh expansion (loads children
    /// into `rows`). Tolerates a stale `row.expanded == true` left over from
    /// `list_children` consulting `expanded_paths` — without this reset,
    /// `toggle_expand` would `collapse_at` instead of `expand_at`.
    fn force_expand_at_path(&mut self, path: &Path) {
        match self.find_row(path) {
            Some(idx) => {
                if self.rows[idx].kind == Kind::Dir {
                    if self.rows[idx].expanded && self.row_kids_match_fs(idx) {
                        crate::file_panel::perf_log!(
                            "limux-perf: force_expand_at_path SKIP(already-expanded-with-kids) {:?}",
                            path
                        );
                        return;
                    }
                    crate::file_panel::perf_log!(
                        "limux-perf: force_expand_at_path RE-EXPAND {:?}",
                        path
                    );
                    self.rows[idx].expanded = false;
                    // Use the no-ListChange variant: refresh_subtree discards
                    // the return value, so building the Vec<Row> just to drop
                    // it allocates O(children) Rows per re-expand × O(50)
                    // expansions = wasted alloc burst on every root refresh.
                    self.expand_at_no_change(idx);
                } else {
                    crate::file_panel::perf_log!(
                        "limux-perf: force_expand_at_path SKIP(not-dir) {:?}",
                        path
                    );
                }
            }
            None => {
                crate::file_panel::perf_log!(
                    "limux-perf: force_expand_at_path SKIP(no-row) {:?}",
                    path
                );
            }
        }
    }

    fn fs_child_names(&self, dir: &Path) -> Option<Vec<std::ffi::OsString>> {
        let rd = std::fs::read_dir(dir).ok()?;
        let mut names: Vec<std::ffi::OsString> = rd
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name();
                if !self.hidden_visible {
                    if let Some(s) = name.to_str() {
                        if s.starts_with('.') {
                            return None;
                        }
                    }
                }
                Some(name)
            })
            .collect();
        names.sort();
        Some(names)
    }

    fn row_kids_match_fs(&self, parent_idx: usize) -> bool {
        let parent = &self.rows[parent_idx];
        let parent_depth = parent.depth;
        let parent_path = &parent.path;
        let fs_names = match self.fs_child_names(parent_path) {
            Some(n) => n,
            None => return false,
        };
        let mut current: Vec<std::ffi::OsString> = Vec::with_capacity(fs_names.len());
        let mut i = parent_idx + 1;
        while i < self.rows.len() && self.rows[i].depth > parent_depth {
            if self.rows[i].depth == parent_depth + 1 {
                if let Some(name) = self.rows[i].path.file_name() {
                    current.push(name.to_os_string());
                }
            }
            i += 1;
        }
        current.sort();
        current == fs_names
    }

    fn tree_matches_fs(&self) -> bool {
        let root_fs = match self.fs_child_names(&self.root) {
            Some(n) => n,
            None => return false,
        };
        let mut root_rows: Vec<std::ffi::OsString> = self
            .rows
            .iter()
            .filter(|r| r.depth == 0)
            .filter_map(|r| r.path.file_name().map(|n| n.to_os_string()))
            .collect();
        root_rows.sort();
        if root_rows != root_fs {
            return false;
        }
        for path in &self.expanded_paths {
            let idx = match self.find_row(path) {
                Some(i) => i,
                None => return false,
            };
            if self.rows[idx].kind != Kind::Dir {
                return false;
            }
            if !self.row_kids_match_fs(idx) {
                return false;
            }
        }
        true
    }

    fn expand_at(&mut self, idx: usize) -> ListChange {
        let inserted = self.expand_at_no_change(idx);
        let mut rows = Vec::with_capacity(1 + inserted);
        rows.push(self.rows[idx].clone());
        let start = idx + 1;
        rows.extend(self.rows[start..start + inserted].iter().cloned());
        ListChange::Replace {
            at: idx as u32,
            removed: 1,
            rows,
        }
    }

    /// Same as `expand_at` but does NOT build the `ListChange`. Callers that
    /// discard the return value (refresh paths) skip the O(children) Row
    /// clone burst that the public variant produces just to be dropped.
    /// Returns the number of children inserted after `idx`.
    fn expand_at_no_change(&mut self, idx: usize) -> usize {
        let path = self.rows[idx].path.clone();
        let depth = self.rows[idx].depth + 1;
        self.rows[idx].expanded = true;
        self.expanded_paths.insert(path.clone());
        let children = self.list_children(&path, depth, Some(idx));
        let count = children.len();
        let insert_at = idx + 1;
        self.rows.splice(insert_at..insert_at, children);
        self.reindex_parents_after(insert_at);
        count
    }

    fn collapse_at(&mut self, idx: usize) -> ListChange {
        let depth = self.rows[idx].depth;
        let top_path = self.rows[idx].path.clone();
        self.rows[idx].expanded = false;
        self.expanded_paths.remove(&top_path);
        let mut end = idx + 1;
        let mut descendant_removed = 0u32;
        while end < self.rows.len() && self.rows[end].depth > depth {
            self.expanded_paths.remove(&self.rows[end].path);
            descendant_removed += 1;
            end += 1;
        }
        crate::file_panel::perf_log!(
            "limux-perf: collapse_at path={:?} descendants_removed={} expanded_paths_len={}",
            top_path,
            descendant_removed,
            self.expanded_paths.len()
        );
        let removed_count = end - idx;
        self.rows.drain(idx + 1..end);
        self.reindex_parents_after(idx + 1);
        ListChange::Replace {
            at: idx as u32,
            removed: removed_count as u32,
            rows: vec![self.rows[idx].clone()],
        }
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
            Err(_) => Vec::new(),
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
                let is_dir = matches!(kind, Kind::Dir);
                let git_status = if is_dir {
                    self.rollup_dir_status(&path)
                } else {
                    self.git_status_map
                        .get(&path)
                        .copied()
                        .unwrap_or(GitStatus::Clean)
                };
                let ignored = self.is_ignored(&path, is_dir);
                Row {
                    path,
                    depth,
                    kind,
                    expanded,
                    git_status,
                    parent_idx,
                    ignored,
                }
            })
            .collect()
    }
}

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
    fn refresh_subtree_root_preserves_depth2_expansion() {
        // Regression: opening `tests/sub/` (depth-2 expansion) and then
        // having `apply_git_result` fire `refresh_subtree(&root)` used to
        // drop the depth-2 children. The depth-sort re-expand loop relied
        // on `toggle_expand`, which collapsed instead of expanded because
        // `list_children` had pre-set `row.expanded = true` from the
        // restored `expanded_paths` set.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "tests");
        let tests = root.join("tests");
        mkdir(&tests, "sub");
        let sub = tests.join("sub");
        touch(&sub, "a.txt");
        touch(&sub, "b.txt");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let tests_idx = m.find_row(&tests).unwrap();
        m.toggle_expand(tests_idx);
        let sub_idx = m.find_row(&sub).unwrap();
        m.toggle_expand(sub_idx);
        assert_eq!(m.rows.len(), 4, "expect root/tests + tests/sub + 2 files");
        // Force a full-tree refresh — mimics apply_git_result.
        m.refresh_subtree(root);
        assert_eq!(
            m.rows.len(),
            4,
            "refresh_subtree(root) must preserve depth-2 expansion"
        );
        assert!(m.find_row(&sub.join("a.txt")).is_some());
        assert!(m.find_row(&sub.join("b.txt")).is_some());
    }

    #[test]
    fn refresh_subtree_parent_preserves_descendant_expansion() {
        // Regression: refresh_subtree on a parent path used to drop deeper
        // expansions because `expand_at` only loads direct children.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "src");
        let src = root.join("src");
        mkdir(&src, "inner");
        let inner = src.join("inner");
        touch(&inner, "x.rs");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let src_idx = m.find_row(&src).unwrap();
        m.toggle_expand(src_idx);
        let inner_idx = m.find_row(&inner).unwrap();
        m.toggle_expand(inner_idx);
        assert_eq!(m.rows.len(), 3);
        // Refresh subtree at the parent of the deeper expansion.
        m.refresh_subtree(&src);
        assert_eq!(
            m.rows.len(),
            3,
            "refresh_subtree(parent) must preserve grandchildren"
        );
        assert!(m.find_row(&inner.join("x.rs")).is_some());
    }

    #[test]
    fn refresh_subtree_root_does_not_reexpand_collapsed_path() {
        // Bug A regression: user expands tests/sub, then collapses it.
        // Some time later apply_git_result fires refresh_subtree(&root).
        // The collapsed path must NOT be re-expanded.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "tests");
        let tests = root.join("tests");
        mkdir(&tests, "sub");
        let sub = tests.join("sub");
        touch(&sub, "a.txt");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let tests_idx = m.find_row(&tests).unwrap();
        m.toggle_expand(tests_idx);
        let sub_idx = m.find_row(&sub).unwrap();
        m.toggle_expand(sub_idx);
        assert_eq!(m.rows.len(), 3);
        let sub_idx = m.find_row(&sub).unwrap();
        m.toggle_expand(sub_idx);
        assert_eq!(m.rows.len(), 2);
        assert!(!m.expanded_paths.contains(&sub));
        m.refresh_subtree(root);
        assert_eq!(
            m.rows.len(),
            2,
            "collapsed sub must stay collapsed after refresh_subtree(root)"
        );
        assert!(!m.expanded_paths.contains(&sub));
        let sub_row = m.find_row(&sub).unwrap();
        assert!(
            !m.rows[sub_row].expanded,
            "collapsed sub row must not be re-expanded"
        );
    }

    #[test]
    fn refresh_subtree_parent_does_not_reexpand_collapsed_child() {
        // Bug A regression: collapse a depth-2 dir, then watcher fires
        // refresh_subtree on the depth-1 parent. The collapsed child
        // must not be re-expanded.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "src");
        let src = root.join("src");
        mkdir(&src, "inner");
        let inner = src.join("inner");
        touch(&inner, "x.rs");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let src_idx = m.find_row(&src).unwrap();
        m.toggle_expand(src_idx);
        let inner_idx = m.find_row(&inner).unwrap();
        m.toggle_expand(inner_idx);
        assert_eq!(m.rows.len(), 3);
        let inner_idx = m.find_row(&inner).unwrap();
        m.toggle_expand(inner_idx);
        assert_eq!(m.rows.len(), 2);
        assert!(!m.expanded_paths.contains(&inner));
        m.refresh_subtree(&src);
        assert_eq!(
            m.rows.len(),
            2,
            "collapsed inner must stay collapsed after refresh_subtree(parent)"
        );
        assert!(!m.expanded_paths.contains(&inner));
    }

    #[test]
    fn refresh_subtree_root_is_noop_when_fs_unchanged() {
        // Patch B regression: when nothing on disk has changed, the
        // root-refresh path used to do a destructive rebuild_visible +
        // re-expand-all anyway, allocating churn for every drain tick.
        // Now: detect fingerprint match and return false without touching
        // `rows`.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        mkdir(root, "src");
        let src = root.join("src");
        touch(&src, "main.rs");
        let mut m = TreeModel::new(root.to_path_buf());
        m.rebuild_visible();
        let src_idx = m.find_row(&src).unwrap();
        m.toggle_expand(src_idx);
        let rows_before = m.rows.clone();
        let changed = m.refresh_subtree(root);
        assert!(
            !changed,
            "refresh_subtree(root) must report no-change when fs unchanged"
        );
        assert_eq!(m.rows, rows_before, "rows must be untouched");
    }

    #[test]
    fn row_in_gitignored_dir_has_ignored_flag() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join(".gitignore"), "*.log\n").unwrap();
        touch(root, "foo.log");
        touch(root, "bar.txt");
        let gi = {
            let mut b = ignore::gitignore::GitignoreBuilder::new(root);
            let _ = b.add(root.join(".gitignore"));
            b.build().unwrap()
        };
        let mut m = TreeModel::new(root.to_path_buf());
        m.set_gitignore(Rc::new(gi));
        m.rebuild_visible();
        let foo = m
            .rows
            .iter()
            .find(|r| r.path == root.join("foo.log"))
            .unwrap();
        let bar = m
            .rows
            .iter()
            .find(|r| r.path == root.join("bar.txt"))
            .unwrap();
        assert!(foo.ignored, "foo.log must be marked ignored");
        assert!(!bar.ignored, "bar.txt must not be marked ignored");
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
