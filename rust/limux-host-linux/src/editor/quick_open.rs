use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;
use ignore::WalkBuilder;

use crate::editor::strings;

pub type OpenFileCallback = Rc<dyn Fn(&Path)>;

const MAX_INDEXED_FILES: usize = 5000;
const MAX_RESULTS: usize = 50;
const POPOVER_WIDTH: i32 = 520;
const POPOVER_HEIGHT: i32 = 360;

#[derive(Clone)]
struct FileEntry {
    path: PathBuf,
    basename: String,
    rel_display: String,
    mtime: SystemTime,
}

pub fn show(parent_widget: &gtk::Widget, workspace_root: Option<&Path>, on_open: OpenFileCallback) {
    let parent: gtk::Widget = parent_widget.clone();
    let files = match workspace_root {
        Some(root) => walk_workspace(root),
        None => Vec::new(),
    };
    let root_buf: Option<PathBuf> = workspace_root.map(|p| p.to_path_buf());

    let popover = gtk::Popover::builder()
        .has_arrow(false)
        .position(gtk::PositionType::Bottom)
        .autohide(true)
        .build();
    popover.add_css_class("lyrux-quick-open");
    popover.set_parent(&parent);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 6);
    vbox.set_margin_top(8);
    vbox.set_margin_bottom(8);
    vbox.set_margin_start(8);
    vbox.set_margin_end(8);
    vbox.set_size_request(POPOVER_WIDTH, POPOVER_HEIGHT);

    let title = gtk::Label::builder()
        .label(strings::QUICK_OPEN_TITLE)
        .xalign(0.0)
        .build();
    title.add_css_class("dim-label");
    vbox.append(&title);

    let entry = gtk::SearchEntry::builder()
        .placeholder_text(strings::QUICK_OPEN_PLACEHOLDER)
        .build();
    vbox.append(&entry);

    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .build();
    let list_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    list_box.add_css_class("lyrux-quick-open-list");
    scroller.set_child(Some(&list_box));
    vbox.append(&scroller);

    let empty_label = gtk::Label::builder()
        .label(strings::QUICK_OPEN_EMPTY)
        .xalign(0.5)
        .build();
    empty_label.add_css_class("dim-label");
    empty_label.set_visible(false);
    vbox.append(&empty_label);

    popover.set_child(Some(&vbox));

    let files_rc: Rc<Vec<FileEntry>> = Rc::new(files);
    let selected: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
    let visible_paths: Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));

    let render = {
        let list_box = list_box.clone();
        let empty_label = empty_label.clone();
        let files_rc = files_rc.clone();
        let selected = selected.clone();
        let visible_paths = visible_paths.clone();
        let popover = popover.clone();
        let on_open = on_open.clone();
        let root_buf = root_buf.clone();
        Rc::new(move |query: &str| {
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }
            visible_paths.borrow_mut().clear();
            *selected.borrow_mut() = 0;

            let ranked = rank(&files_rc, query);
            if ranked.is_empty() {
                empty_label.set_visible(true);
                return;
            }
            empty_label.set_visible(false);

            for (idx, entry) in ranked.iter().enumerate() {
                let row = build_row(entry, idx == 0);
                let path_clone = entry.path.clone();
                let popover_clone = popover.clone();
                let on_open_clone = on_open.clone();
                let root_clone = root_buf.clone();
                row.connect_clicked(move |_| {
                    activate_path(
                        &popover_clone,
                        &on_open_clone,
                        root_clone.as_deref(),
                        &path_clone,
                    );
                });
                list_box.append(&row);
                visible_paths.borrow_mut().push(entry.path.clone());
            }
        })
    };

    render("");

    {
        let render = render.clone();
        entry.connect_search_changed(move |entry| {
            let text = entry.text().to_string();
            render(&text);
        });
    }

    {
        let popover_clone = popover.clone();
        let on_open_inner = on_open.clone();
        let root_clone = root_buf.clone();
        let visible_paths = visible_paths.clone();
        let selected = selected.clone();
        entry.connect_activate(move |_| {
            let idx = *selected.borrow();
            let paths = visible_paths.borrow();
            if let Some(path) = paths.get(idx) {
                activate_path(&popover_clone, &on_open_inner, root_clone.as_deref(), path);
            } else {
                popover_clone.popdown();
            }
        });
    }

    let key_ctrl = gtk::EventControllerKey::new();
    key_ctrl.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let popover_clone = popover.clone();
        let visible_paths = visible_paths.clone();
        let selected = selected.clone();
        let list_box = list_box.clone();
        let on_open_inner = on_open.clone();
        let root_clone = root_buf.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            use gtk::gdk::Key;
            match key {
                Key::Escape => {
                    popover_clone.popdown();
                    glib::Propagation::Stop
                }
                Key::Down => {
                    move_selection(&selected, &visible_paths, &list_box, 1);
                    glib::Propagation::Stop
                }
                Key::Up => {
                    move_selection(&selected, &visible_paths, &list_box, -1);
                    glib::Propagation::Stop
                }
                Key::Return | Key::ISO_Enter | Key::KP_Enter => {
                    let idx = *selected.borrow();
                    let paths = visible_paths.borrow();
                    if let Some(path) = paths.get(idx) {
                        activate_path(&popover_clone, &on_open_inner, root_clone.as_deref(), path);
                    } else {
                        popover_clone.popdown();
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
    }
    entry.add_controller(key_ctrl);

    {
        let popover_clone = popover.clone();
        popover.connect_closed(move |_| {
            popover_clone.unparent();
        });
    }

    popover.popup();
    entry.grab_focus();
}

fn build_row(entry: &FileEntry, is_selected: bool) -> gtk::Button {
    let row_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    row_box.set_margin_top(4);
    row_box.set_margin_bottom(4);
    row_box.set_margin_start(8);
    row_box.set_margin_end(8);

    let name = gtk::Label::builder()
        .label(&entry.basename)
        .xalign(0.0)
        .build();
    name.add_css_class("body");
    row_box.append(&name);

    if !entry.rel_display.is_empty() && entry.rel_display != entry.basename {
        let rel = gtk::Label::builder()
            .label(&entry.rel_display)
            .xalign(0.0)
            .build();
        rel.add_css_class("dim-label");
        rel.add_css_class("caption");
        row_box.append(&rel);
    }

    let btn = gtk::Button::builder()
        .child(&row_box)
        .has_frame(false)
        .build();
    btn.add_css_class("lyrux-quick-open-row");
    btn.set_halign(gtk::Align::Fill);
    if is_selected {
        btn.add_css_class("suggested-action");
    }
    btn
}

fn move_selection(
    selected: &Rc<RefCell<usize>>,
    visible_paths: &Rc<RefCell<Vec<PathBuf>>>,
    list_box: &gtk::Box,
    delta: i32,
) {
    let len = visible_paths.borrow().len();
    if len == 0 {
        return;
    }
    let cur = *selected.borrow() as i32;
    let mut next = cur + delta;
    if next < 0 {
        next = (len as i32) - 1;
    } else if next >= len as i32 {
        next = 0;
    }
    *selected.borrow_mut() = next as usize;
    refresh_selection_styles(list_box, next as usize);
}

fn refresh_selection_styles(list_box: &gtk::Box, selected: usize) {
    let mut idx = 0usize;
    let mut child = list_box.first_child();
    while let Some(widget) = child {
        let next = widget.next_sibling();
        if let Some(btn) = widget.downcast_ref::<gtk::Button>() {
            if idx == selected {
                btn.add_css_class("suggested-action");
                btn.grab_focus();
            } else {
                btn.remove_css_class("suggested-action");
            }
        }
        idx += 1;
        child = next;
    }
}

fn activate_path(
    popover: &gtk::Popover,
    on_open: &OpenFileCallback,
    workspace_root: Option<&Path>,
    path: &Path,
) {
    popover.popdown();
    if !is_path_within_root(workspace_root, path) {
        eprintln!("lyrux: quick-open rejected path outside workspace root");
        return;
    }
    on_open(path);
}

fn is_path_within_root(root: Option<&Path>, path: &Path) -> bool {
    let Some(root) = root else {
        return false;
    };
    let canon_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let canon_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    canon_path.starts_with(&canon_root)
}

fn walk_workspace(root: &Path) -> Vec<FileEntry> {
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .ignore(true)
        .parents(true)
        .build();

    let mut out: Vec<FileEntry> = Vec::with_capacity(1024);
    for dent in walker.flatten() {
        if out.len() >= MAX_INDEXED_FILES {
            break;
        }
        let ft = match dent.file_type() {
            Some(ft) => ft,
            None => continue,
        };
        if !ft.is_file() {
            continue;
        }
        let path = dent.path();
        let basename = match path.file_name().and_then(|s| s.to_str()) {
            Some(b) => b.to_string(),
            None => continue,
        };
        let rel_display = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let mtime = dent
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        out.push(FileEntry {
            path: path.to_path_buf(),
            basename,
            rel_display,
            mtime,
        });
    }
    out
}

fn rank(files: &[FileEntry], query: &str) -> Vec<FileEntry> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        let mut sorted: Vec<FileEntry> = files.to_vec();
        sorted.sort_by(|a, b| b.mtime.cmp(&a.mtime));
        sorted.truncate(MAX_RESULTS);
        return sorted;
    }

    let needle = trimmed.to_ascii_lowercase();
    let mut scored: Vec<(i32, &FileEntry)> = Vec::new();
    for entry in files {
        let base_lower = entry.basename.to_ascii_lowercase();
        let rel_lower = entry.rel_display.to_ascii_lowercase();
        let base_score = fuzzy_score(&needle, &base_lower);
        let rel_score = fuzzy_score(&needle, &rel_lower);
        let combined = match (base_score, rel_score) {
            (Some(b), Some(r)) => Some(b.saturating_mul(2).saturating_add(r)),
            (Some(b), None) => Some(b.saturating_mul(2)),
            (None, Some(r)) => Some(r),
            (None, None) => None,
        };
        if let Some(score) = combined {
            scored.push((score, entry));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.basename.cmp(&b.1.basename)));
    scored
        .into_iter()
        .take(MAX_RESULTS)
        .map(|(_, e)| e.clone())
        .collect()
}

fn fuzzy_score(needle: &str, haystack: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(0);
    }
    let needle_chars: Vec<char> = needle.chars().collect();
    let haystack_chars: Vec<char> = haystack.chars().collect();
    if needle_chars.len() > haystack_chars.len() {
        return None;
    }

    let mut score: i32 = 0;
    let mut ni: usize = 0;
    let mut prev_match: Option<usize> = None;
    let mut prev_char_was_boundary = true;

    for (hi, hc) in haystack_chars.iter().enumerate() {
        if ni >= needle_chars.len() {
            break;
        }
        let nc = needle_chars[ni];
        let boundary_here = prev_char_was_boundary;
        let is_boundary_char = matches!(*hc, '/' | '_' | '-' | '.' | ' ');
        if *hc == nc {
            let mut gain: i32 = 1;
            if let Some(prev) = prev_match {
                if hi == prev + 1 {
                    gain += 5;
                }
            }
            if boundary_here {
                gain += 8;
            }
            if hi == 0 {
                gain += 4;
            }
            score += gain;
            prev_match = Some(hi);
            ni += 1;
        }
        prev_char_was_boundary = is_boundary_char;
    }

    if ni == needle_chars.len() {
        Some(score)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_subsequence_match() {
        assert!(fuzzy_score("abc", "axbxc").is_some());
        assert!(fuzzy_score("abc", "cba").is_none());
        assert!(fuzzy_score("foo", "foobar").is_some());
    }

    #[test]
    fn fuzzy_consecutive_beats_scattered() {
        let dense = fuzzy_score("ab", "ab").unwrap();
        let sparse = fuzzy_score("ab", "axxxxb").unwrap();
        assert!(dense > sparse);
    }

    #[test]
    fn fuzzy_boundary_bonus() {
        let boundary = fuzzy_score("m", "main.rs").unwrap();
        let mid = fuzzy_score("m", "name.rs").unwrap();
        assert!(boundary > mid);
    }

    #[test]
    fn rank_empty_query_returns_recency_order() {
        let now = SystemTime::now();
        let older = now - std::time::Duration::from_secs(60);
        let files = vec![
            FileEntry {
                path: PathBuf::from("/a/old.rs"),
                basename: "old.rs".to_string(),
                rel_display: "old.rs".to_string(),
                mtime: older,
            },
            FileEntry {
                path: PathBuf::from("/a/new.rs"),
                basename: "new.rs".to_string(),
                rel_display: "new.rs".to_string(),
                mtime: now,
            },
        ];
        let ranked = rank(&files, "");
        assert_eq!(ranked[0].basename, "new.rs");
        assert_eq!(ranked[1].basename, "old.rs");
    }

    #[test]
    fn rank_basename_beats_path_match() {
        let now = SystemTime::now();
        let files = vec![
            FileEntry {
                path: PathBuf::from("/a/main.rs"),
                basename: "main.rs".to_string(),
                rel_display: "src/main.rs".to_string(),
                mtime: now,
            },
            FileEntry {
                path: PathBuf::from("/a/main/lib.rs"),
                basename: "lib.rs".to_string(),
                rel_display: "main/lib.rs".to_string(),
                mtime: now,
            },
        ];
        let ranked = rank(&files, "main");
        assert_eq!(ranked[0].basename, "main.rs");
    }

    #[test]
    fn rank_respects_max_results() {
        let now = SystemTime::now();
        let files: Vec<FileEntry> = (0..120)
            .map(|i| FileEntry {
                path: PathBuf::from(format!("/a/file{i}.rs")),
                basename: format!("file{i}.rs"),
                rel_display: format!("file{i}.rs"),
                mtime: now,
            })
            .collect();
        let ranked = rank(&files, "file");
        assert_eq!(ranked.len(), MAX_RESULTS);
    }
}
