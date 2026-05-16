use std::cell::RefCell;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;

use crate::editor::strings;
use crate::editor::tab_state::EditorTabState;

pub type OpenFileAtCallback = Rc<dyn Fn(&Path, usize, usize)>;

const MIN_QUERY_LEN: usize = 2;
const MAX_RESULTS: usize = 500;
const DEBOUNCE_MS: u32 = 250;
const POPOVER_WIDTH: i32 = 720;
const POPOVER_HEIGHT: i32 = 480;
const RG_MAX_FILESIZE: &str = "10M";
const RG_MAX_COLUMNS: &str = "500";

#[derive(Clone)]
struct Hit {
    path: PathBuf,
    line: usize,
    col: usize,
    snippet: String,
}

enum WorkerMessage {
    Done(Vec<Hit>),
    Failed(String),
}

pub fn show(
    parent_widget: &gtk::Widget,
    workspace_root: Option<&Path>,
    on_open: OpenFileAtCallback,
) {
    let parent: gtk::Widget = parent_widget.clone();
    let root_buf: Option<PathBuf> = workspace_root.map(|p| p.to_path_buf());

    let popover = gtk::Popover::builder()
        .has_arrow(false)
        .position(gtk::PositionType::Bottom)
        .autohide(true)
        .build();
    popover.add_css_class("lyrux-find-in-files");
    popover.set_parent(&parent);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 6);
    vbox.set_margin_top(8);
    vbox.set_margin_bottom(8);
    vbox.set_margin_start(8);
    vbox.set_margin_end(8);
    vbox.set_size_request(POPOVER_WIDTH, POPOVER_HEIGHT);

    let title = gtk::Label::builder()
        .label(strings::FIF_TITLE)
        .xalign(0.0)
        .build();
    title.add_css_class("dim-label");
    vbox.append(&title);

    let entry = gtk::SearchEntry::builder()
        .placeholder_text(strings::FIF_PLACEHOLDER)
        .build();
    vbox.append(&entry);

    let summary = gtk::Label::builder().label("").xalign(0.0).build();
    summary.add_css_class("dim-label");
    summary.add_css_class("caption");
    vbox.append(&summary);

    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .build();
    let list_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    list_box.add_css_class("lyrux-find-in-files-list");
    scroller.set_child(Some(&list_box));
    vbox.append(&scroller);

    let status_label = gtk::Label::builder().label("").xalign(0.5).build();
    status_label.add_css_class("dim-label");
    status_label.set_visible(false);
    vbox.append(&status_label);

    popover.set_child(Some(&vbox));

    let generation: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let active_child: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let debounce_source: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    if !rg_available() {
        status_label.set_label(strings::FIF_NO_RG);
        status_label.set_visible(true);
        entry.set_sensitive(false);
    }

    let trigger_search = {
        let list_box = list_box.clone();
        let summary = summary.clone();
        let status_label = status_label.clone();
        let popover = popover.clone();
        let on_open = on_open.clone();
        let root_buf = root_buf.clone();
        let generation = generation.clone();
        let active_child = active_child.clone();
        Rc::new(move |query: String| {
            let gen_id = generation.fetch_add(1, Ordering::AcqRel) + 1;
            kill_active_child(&active_child);
            clear_results(&list_box);
            summary.set_label("");
            status_label.set_visible(false);

            let trimmed = query.trim().to_string();
            if trimmed.len() < MIN_QUERY_LEN {
                return;
            }
            let Some(root) = root_buf.clone() else {
                status_label.set_label(strings::FIF_NO_ROOT);
                status_label.set_visible(true);
                return;
            };

            let (tx, rx) = mpsc::channel::<WorkerMessage>();
            let generation_for_worker = generation.clone();
            let active_child_for_worker = active_child.clone();
            let root_for_worker = root.clone();
            let trimmed_for_worker = trimmed.clone();
            std::thread::Builder::new()
                .name("lyrux-find-in-files".into())
                .spawn(move || {
                    let result = run_ripgrep(
                        &trimmed_for_worker,
                        &root_for_worker,
                        &active_child_for_worker,
                    );
                    if generation_for_worker.load(Ordering::Acquire) != gen_id {
                        return;
                    }
                    let msg = match result {
                        Ok(hits) => WorkerMessage::Done(hits),
                        Err(err) => WorkerMessage::Failed(err),
                    };
                    let _ = tx.send(msg);
                })
                .ok();

            let list_box_poll = list_box.clone();
            let summary_poll = summary.clone();
            let status_label_poll = status_label.clone();
            let popover_poll = popover.clone();
            let on_open_poll = on_open.clone();
            let root_poll = root.clone();
            let generation_poll = generation.clone();
            glib::timeout_add_local(Duration::from_millis(40), move || match rx.try_recv() {
                Ok(WorkerMessage::Done(hits)) => {
                    if generation_poll.load(Ordering::Acquire) == gen_id {
                        render_hits(
                            &list_box_poll,
                            &summary_poll,
                            &status_label_poll,
                            &popover_poll,
                            &on_open_poll,
                            &root_poll,
                            hits,
                        );
                    }
                    glib::ControlFlow::Break
                }
                Ok(WorkerMessage::Failed(err)) => {
                    if generation_poll.load(Ordering::Acquire) == gen_id {
                        status_label_poll
                            .set_label(&format!("{}{err}", strings::FIF_RG_FAILED_PREFIX));
                        status_label_poll.set_visible(true);
                    }
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => {
                    if generation_poll.load(Ordering::Acquire) != gen_id {
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            });
        })
    };

    {
        let trigger_search = trigger_search.clone();
        let debounce_source = debounce_source.clone();
        entry.connect_search_changed(move |entry| {
            let text = entry.text().to_string();
            if let Some(source_id) = debounce_source.borrow_mut().take() {
                source_id.remove();
            }
            let trigger_search = trigger_search.clone();
            let debounce_source_inner = debounce_source.clone();
            let id = glib::timeout_add_local_once(
                Duration::from_millis(DEBOUNCE_MS.into()),
                move || {
                    debounce_source_inner.borrow_mut().take();
                    trigger_search(text);
                },
            );
            *debounce_source.borrow_mut() = Some(id);
        });
    }

    {
        let trigger_search = trigger_search.clone();
        let debounce_source = debounce_source.clone();
        entry.connect_activate(move |entry| {
            if let Some(source_id) = debounce_source.borrow_mut().take() {
                source_id.remove();
            }
            trigger_search(entry.text().to_string());
        });
    }

    let key_ctrl = gtk::EventControllerKey::new();
    key_ctrl.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let popover_clone = popover.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            use gtk::gdk::Key;
            if matches!(key, Key::Escape) {
                popover_clone.popdown();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    entry.add_controller(key_ctrl);

    {
        let popover_clone = popover.clone();
        let generation = generation.clone();
        let active_child = active_child.clone();
        let debounce_source = debounce_source.clone();
        popover.connect_closed(move |_| {
            generation.fetch_add(1, Ordering::AcqRel);
            kill_active_child(&active_child);
            if let Some(source_id) = debounce_source.borrow_mut().take() {
                source_id.remove();
            }
            popover_clone.unparent();
        });
    }

    popover.popup();
    entry.grab_focus();
}

fn render_hits(
    list_box: &gtk::Box,
    summary: &gtk::Label,
    status_label: &gtk::Label,
    popover: &gtk::Popover,
    on_open: &OpenFileAtCallback,
    root: &Path,
    hits: Vec<Hit>,
) {
    clear_results(list_box);
    if hits.is_empty() {
        summary.set_label("");
        status_label.set_label(strings::FIF_NO_MATCHES);
        status_label.set_visible(true);
        return;
    }
    status_label.set_visible(false);

    let mut seen_files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for hit in &hits {
        seen_files.insert(hit.path.clone());
    }
    let summary_text = strings::fif_results_label(hits.len(), seen_files.len());
    summary.set_label(&summary_text);

    let root_buf = root.to_path_buf();
    for hit in hits.into_iter().take(MAX_RESULTS) {
        let row = build_row(&hit, &root_buf);
        let path_clone = hit.path.clone();
        let line = hit.line;
        let col = hit.col;
        let popover_clone = popover.clone();
        let on_open_clone = on_open.clone();
        let root_clone = root_buf.clone();
        row.connect_clicked(move |_| {
            popover_clone.popdown();
            if !is_path_within_root(&root_clone, &path_clone) {
                eprintln!("lyrux: find-in-files rejected path outside workspace root");
                return;
            }
            on_open_clone(&path_clone, line, col);
        });
        list_box.append(&row);
    }
}

fn build_row(hit: &Hit, root: &Path) -> gtk::Button {
    let row_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    row_box.set_margin_top(4);
    row_box.set_margin_bottom(4);
    row_box.set_margin_start(8);
    row_box.set_margin_end(8);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let basename = hit
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let name = gtk::Label::builder().label(&basename).xalign(0.0).build();
    name.add_css_class("body");
    header.append(&name);

    let rel = hit
        .path
        .strip_prefix(root)
        .unwrap_or(&hit.path)
        .to_string_lossy()
        .to_string();
    let loc_text = format!("{rel}:{}:{}", hit.line, hit.col);
    let loc = gtk::Label::builder().label(&loc_text).xalign(0.0).build();
    loc.add_css_class("dim-label");
    loc.add_css_class("caption");
    loc.set_hexpand(true);
    loc.set_halign(gtk::Align::Start);
    header.append(&loc);
    row_box.append(&header);

    let snippet = gtk::Label::builder()
        .label(hit.snippet.trim_end())
        .xalign(0.0)
        .build();
    snippet.add_css_class("monospace");
    snippet.add_css_class("caption");
    snippet.set_ellipsize(gtk::pango::EllipsizeMode::End);
    row_box.append(&snippet);

    let btn = gtk::Button::builder()
        .child(&row_box)
        .has_frame(false)
        .build();
    btn.add_css_class("lyrux-find-in-files-row");
    btn.set_halign(gtk::Align::Fill);
    btn
}

fn clear_results(list_box: &gtk::Box) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
}

fn kill_active_child(slot: &Arc<Mutex<Option<Child>>>) {
    if let Ok(mut guard) = slot.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn rg_available() -> bool {
    Command::new("rg")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok()
}

fn run_ripgrep(
    query: &str,
    root: &Path,
    active_child: &Arc<Mutex<Option<Child>>>,
) -> Result<Vec<Hit>, String> {
    let mut cmd = Command::new("rg");
    cmd.arg("--vimgrep")
        .arg("--color")
        .arg("never")
        .arg("--no-heading")
        .arg("--no-messages")
        .arg("--smart-case")
        .arg("--hidden")
        .arg("--max-columns")
        .arg(RG_MAX_COLUMNS)
        .arg("--max-filesize")
        .arg(RG_MAX_FILESIZE)
        .arg("-e")
        .arg(query)
        .arg(root);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "no stdout".to_string())?;
    {
        let mut guard = active_child.lock().map_err(|e| e.to_string())?;
        *guard = Some(child);
    }

    let reader = BufReader::new(stdout);
    let mut hits: Vec<Hit> = Vec::new();
    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        if let Some(hit) = parse_vimgrep_line(&line) {
            hits.push(hit);
            if hits.len() >= MAX_RESULTS {
                break;
            }
        }
    }

    if let Ok(mut guard) = active_child.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    Ok(hits)
}

fn parse_vimgrep_line(line: &str) -> Option<Hit> {
    let mut parts = line.splitn(4, ':');
    let path_str = parts.next()?;
    let line_str = parts.next()?;
    let col_str = parts.next()?;
    let content = parts.next()?;
    if path_str.is_empty() {
        return None;
    }
    let line_no: usize = line_str.parse().ok()?;
    let col_no: usize = col_str.parse().ok()?;
    Some(Hit {
        path: PathBuf::from(path_str),
        line: line_no,
        col: col_no,
        snippet: content.to_string(),
    })
}

fn is_path_within_root(root: &Path, path: &Path) -> bool {
    let canon_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let canon_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    canon_path.starts_with(&canon_root)
}

pub fn jump_to_position(state: &EditorTabState, line: usize, col: usize) {
    let line_idx = line.saturating_sub(1) as i32;
    let col_idx = col.saturating_sub(1) as i32;
    let maybe_iter = state
        .buffer
        .iter_at_line_offset(line_idx, col_idx)
        .or_else(|| state.buffer.iter_at_line(line_idx));
    if let Some(mut iter) = maybe_iter {
        state.buffer.place_cursor(&iter);
        state.view.scroll_to_iter(&mut iter, 0.1, false, 0.0, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_vimgrep_line() {
        let hit = parse_vimgrep_line("/tmp/foo.rs:12:5:    let x = 1;").unwrap();
        assert_eq!(hit.path, PathBuf::from("/tmp/foo.rs"));
        assert_eq!(hit.line, 12);
        assert_eq!(hit.col, 5);
        assert_eq!(hit.snippet, "    let x = 1;");
    }

    #[test]
    fn parses_line_with_colons_in_content() {
        let hit = parse_vimgrep_line("/tmp/foo.rs:3:9:    let x: u32 = 1;").unwrap();
        assert_eq!(hit.line, 3);
        assert_eq!(hit.col, 9);
        assert_eq!(hit.snippet, "    let x: u32 = 1;");
    }

    #[test]
    fn rejects_malformed_line() {
        assert!(parse_vimgrep_line("not a real line").is_none());
        assert!(parse_vimgrep_line("/tmp/f.rs:nope:5:x").is_none());
        assert!(parse_vimgrep_line(":1:1:body").is_none());
    }

    #[test]
    fn rejects_missing_fields() {
        assert!(parse_vimgrep_line("/tmp/f.rs:1:2").is_none());
        assert!(parse_vimgrep_line("/tmp/f.rs:1").is_none());
    }
}
