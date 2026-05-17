use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use sourceview5::prelude::*;

use crate::editor::buffer;
use crate::editor::strings;
use crate::editor::tab_state::EditorTabState;
use crate::file_panel::model::is_within_root;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveTransform {
    pub strip_trailing_whitespace: bool,
    pub ensure_final_newline: bool,
}

pub type SaveTransformFn = Rc<dyn Fn() -> SaveTransform>;

const SEARCH_CONTEXT_KEY: &str = "lyrux-search-ctx";

pub fn install(
    view: &sourceview5::View,
    state: &EditorTabState,
    workspace_root: Option<PathBuf>,
    on_clean: Rc<dyn Fn()>,
    on_close_request: Rc<dyn Fn()>,
    save_transform: SaveTransformFn,
) {
    let ctrl = gtk4::EventControllerKey::new();
    let state = state.clone();
    ctrl.connect_key_pressed(move |_, key, _, mods| {
        let ctrl_held = mods.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
        let alt_held = mods.contains(gtk4::gdk::ModifierType::ALT_MASK);
        let shift_held = mods.contains(gtk4::gdk::ModifierType::SHIFT_MASK);

        if alt_held && !ctrl_held {
            return match key {
                gtk4::gdk::Key::Up => {
                    move_line_up(&state);
                    glib::Propagation::Stop
                }
                gtk4::gdk::Key::Down => {
                    move_line_down(&state);
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            };
        }

        if !ctrl_held {
            return glib::Propagation::Proceed;
        }

        match key {
            gtk4::gdk::Key::s => {
                save_tab(
                    &state,
                    workspace_root.as_deref(),
                    &on_clean,
                    save_transform(),
                );
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::f => {
                show_find_bar(&state, false);
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::h => {
                show_find_bar(&state, true);
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::g | gtk4::gdk::Key::F3 => {
                find_next(&state);
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::l => {
                show_goto_line(&state);
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::slash => {
                toggle_line_comment(&state);
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::D if shift_held => {
                duplicate_line(&state);
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::d => {
                select_next_occurrence(&state);
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::K if shift_held => {
                delete_line(&state);
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::w => {
                on_close_request();
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
    view.add_controller(ctrl);
}

pub fn save_tab(
    state: &EditorTabState,
    workspace_root: Option<&Path>,
    on_clean: &Rc<dyn Fn()>,
    transform: SaveTransform,
) {
    let current = state.path.borrow().clone();
    if current.as_os_str().is_empty() {
        show_save_as_dialog(state, workspace_root, on_clean, transform);
        return;
    }
    if let Some(root) = workspace_root {
        if !is_within_root(&current, root) {
            eprintln!("lyrux: {}", strings::ERROR_OUTSIDE_WORKSPACE);
            return;
        }
    }
    let text = apply_save_transform(state.snapshot_text(), transform);
    match buffer::save_atomic(&current, &text) {
        Ok(etag) => {
            state.mark_clean(etag);
            if let Some(sp) = state.swap_path.borrow_mut().take() {
                let _ = crate::editor::swap::discard(&sp);
            }
            on_clean();
        }
        Err(e) => {
            let msg = format!("{}{e}", strings::ERROR_WRITE_FAILED_PREFIX);
            eprintln!("lyrux: {msg}");
            crate::editor::watcher::show_error_banner(state, &msg);
        }
    }
}

fn show_save_as_dialog(
    state: &EditorTabState,
    workspace_root: Option<&Path>,
    on_clean: &Rc<dyn Fn()>,
    transform: SaveTransform,
) {
    let dialog = gtk4::FileDialog::builder()
        .title(strings::SAVE_AS_DIALOG_TITLE)
        .modal(true)
        .build();
    if let Some(root) = workspace_root {
        let initial = gtk4::gio::File::for_path(root);
        dialog.set_initial_folder(Some(&initial));
    }
    let parent = state
        .root
        .root()
        .and_then(|r| r.downcast::<gtk4::Window>().ok());
    let state_for_cb = state.clone();
    let workspace_root_owned = workspace_root.map(|p| p.to_path_buf());
    let on_clean_for_cb = on_clean.clone();
    dialog.save(
        parent.as_ref(),
        None::<&gtk4::gio::Cancellable>,
        move |result| {
            let picked = match result {
                Ok(f) => match f.path() {
                    Some(p) => p,
                    None => return,
                },
                Err(_) => return,
            };
            if let Some(root) = workspace_root_owned.as_deref() {
                if !is_within_root(&picked, root) {
                    eprintln!("lyrux: {}", strings::ERROR_OUTSIDE_WORKSPACE);
                    return;
                }
            }
            let state_for_save = state_for_cb.clone();
            let on_clean_for_save = on_clean_for_cb.clone();
            let do_save: Rc<dyn Fn(PathBuf)> = Rc::new(move |picked: PathBuf| {
                save_as_finalize(&state_for_save, &on_clean_for_save, transform, picked);
            });

            if matches!(
                crate::editor::classify_file(&picked),
                crate::editor::FileKind::Image
            ) {
                let parent_win = state_for_cb
                    .root
                    .root()
                    .and_then(|r| r.downcast::<gtk4::Window>().ok());
                let warn = gtk4::AlertDialog::builder()
                    .modal(true)
                    .message(strings::SAVE_AS_BINARY_WARN_BODY)
                    .buttons([
                        strings::DIALOG_BTN_CANCEL,
                        strings::SAVE_AS_BINARY_WARN_PROCEED,
                    ])
                    .cancel_button(0)
                    .default_button(0)
                    .build();
                let do_save_for_warn = do_save.clone();
                warn.choose(
                    parent_win.as_ref(),
                    None::<&gtk4::gio::Cancellable>,
                    move |result| {
                        if result.unwrap_or(0) == 1 {
                            do_save_for_warn(picked);
                        }
                    },
                );
            } else {
                do_save(picked);
            }
        },
    );
}

fn save_as_finalize(
    state: &EditorTabState,
    on_clean: &Rc<dyn Fn()>,
    transform: SaveTransform,
    picked: PathBuf,
) {
    let text = apply_save_transform(state.snapshot_text(), transform);
    match buffer::save_atomic(&picked, &text) {
        Ok(etag) => {
            *state.path.borrow_mut() = picked.clone();
            state.mark_clean(etag);
            if let Some(sp) = state.swap_path.borrow_mut().take() {
                let _ = crate::editor::swap::discard(&sp);
            }
            if let Some(cb) = state.title_cb.borrow().as_ref() {
                let title = picked
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(strings::TAB_TITLE_UNTITLED)
                    .to_string();
                cb(&title);
            }
            state.set_monitor(crate::editor::watcher::install(state));
            on_clean();
        }
        Err(e) => {
            let msg = format!("{}{e}", strings::ERROR_WRITE_FAILED_PREFIX);
            eprintln!("lyrux: {msg}");
            crate::editor::watcher::show_error_banner(state, &msg);
        }
    }
}

fn comment_prefix_for(lang_id: &str) -> Option<&'static str> {
    match lang_id {
        "rust" | "typescript" | "javascript" | "c" | "cpp" | "chdr" | "go" | "css" | "lua" => {
            Some("// ")
        }
        "python" | "python3" | "sh" | "yaml" | "toml" | "ruby" => Some("# "),
        _ => None,
    }
}

pub(crate) fn toggle_line_comment(state: &EditorTabState) {
    let lang = match state.buffer.language() {
        Some(l) => l,
        None => return,
    };
    let prefix = match comment_prefix_for(lang.id().as_str()) {
        Some(p) => p,
        None => return,
    };
    let trimmed = prefix.trim_end();
    let buffer = &state.buffer;
    let (sel_start, sel_end) = match buffer.selection_bounds() {
        Some(b) => b,
        None => {
            let mark = buffer.get_insert();
            let it = buffer.iter_at_mark(&mark);
            (it, it)
        }
    };
    let start_line = sel_start.line();
    let end_line = if sel_end.line() > start_line && sel_end.line_offset() == 0 {
        sel_end.line() - 1
    } else {
        sel_end.line()
    };
    buffer.begin_user_action();
    let all_commented = (start_line..=end_line).all(|ln| line_starts_with(buffer, ln, trimmed));
    for ln in (start_line..=end_line).rev() {
        if all_commented {
            uncomment_line(buffer, ln, prefix, trimmed);
        } else {
            comment_line(buffer, ln, prefix);
        }
    }
    buffer.end_user_action();
}

fn line_starts_with(buffer: &sourceview5::Buffer, line: i32, needle: &str) -> bool {
    let Some(start) = buffer.iter_at_line(line) else {
        return false;
    };
    let mut iter = start;
    while !iter.ends_line() && iter.char().is_whitespace() {
        if !iter.forward_char() {
            break;
        }
    }
    let mut tail = iter;
    for _ in 0..needle.chars().count() {
        if !tail.forward_char() {
            return false;
        }
    }
    buffer.text(&iter, &tail, false).as_str() == needle
}

fn comment_line(buffer: &sourceview5::Buffer, line: i32, prefix: &str) {
    let Some(start) = buffer.iter_at_line(line) else {
        return;
    };
    let mut iter = start;
    while !iter.ends_line() && iter.char().is_whitespace() {
        if !iter.forward_char() {
            break;
        }
    }
    let mut insert_at = iter;
    buffer.insert(&mut insert_at, prefix);
}

fn uncomment_line(buffer: &sourceview5::Buffer, line: i32, prefix: &str, trimmed: &str) {
    let Some(start) = buffer.iter_at_line(line) else {
        return;
    };
    let mut iter = start;
    while !iter.ends_line() && iter.char().is_whitespace() {
        if !iter.forward_char() {
            break;
        }
    }
    let mut tail = iter;
    for _ in 0..trimmed.chars().count() {
        if !tail.forward_char() {
            return;
        }
    }
    if buffer.text(&iter, &tail, false).as_str() != trimmed {
        return;
    }
    let mut remove_end = tail;
    if remove_end.char() == ' ' && prefix.ends_with(' ') {
        remove_end.forward_char();
    }
    let mut s = iter;
    let mut e = remove_end;
    buffer.delete(&mut s, &mut e);
}

pub(crate) fn duplicate_line(state: &EditorTabState) {
    let buffer = &state.buffer;
    let mark = buffer.get_insert();
    let cursor = buffer.iter_at_mark(&mark);
    let line = cursor.line();
    let Some(line_start) = buffer.iter_at_line(line) else {
        return;
    };
    let mut line_end = line_start;
    if !line_end.ends_line() {
        line_end.forward_to_line_end();
    }
    let text = buffer.text(&line_start, &line_end, false).to_string();
    buffer.begin_user_action();
    let mut insert_at = line_end;
    buffer.insert(&mut insert_at, "\n");
    buffer.insert(&mut insert_at, &text);
    buffer.end_user_action();
}

pub(crate) fn delete_line(state: &EditorTabState) {
    let buffer = &state.buffer;
    let mark = buffer.get_insert();
    let cursor = buffer.iter_at_mark(&mark);
    let line = cursor.line();
    let Some(mut start) = buffer.iter_at_line(line) else {
        return;
    };
    let mut end = buffer
        .iter_at_line(line + 1)
        .unwrap_or_else(|| buffer.end_iter());
    if start == end && !start.backward_char() {
        return;
    }
    buffer.begin_user_action();
    buffer.delete(&mut start, &mut end);
    buffer.end_user_action();
}

pub(crate) fn select_next_occurrence(state: &EditorTabState) {
    let buffer = &state.buffer;
    let needle = match buffer.selection_bounds() {
        Some((s, e)) => buffer.text(&s, &e, false).to_string(),
        None => {
            let cursor = buffer.iter_at_mark(&buffer.get_insert());
            let (ws, we) = word_bounds_at_iter(buffer, cursor);
            if ws.offset() == we.offset() {
                return;
            }
            buffer.select_range(&ws, &we);
            scroll_to_insert(state);
            return;
        }
    };
    if needle.is_empty() {
        return;
    }
    let ctx = ensure_search_context(state);
    let settings = ctx.settings();
    settings.set_search_text(Some(&needle));
    settings.set_wrap_around(true);
    settings.set_case_sensitive(true);
    settings.set_at_word_boundaries(false);
    let from = match buffer.selection_bounds() {
        Some((_, e)) => e,
        None => buffer.iter_at_mark(&buffer.get_insert()),
    };
    if let Some((s, e, _wrap)) = ctx.forward(&from) {
        buffer.select_range(&s, &e);
        scroll_to_insert(state);
    }
}

fn scroll_to_insert(state: &EditorTabState) {
    let mut iter = state.buffer.iter_at_mark(&state.buffer.get_insert());
    state.view.scroll_to_iter(&mut iter, 0.1, false, 0.0, 0.5);
}

fn word_bounds_at_iter(
    buffer: &sourceview5::Buffer,
    iter: gtk4::TextIter,
) -> (gtk4::TextIter, gtk4::TextIter) {
    let text = buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string();
    let cursor_byte = byte_offset_from_char_offset(&text, iter.offset() as usize);
    let (s_byte, e_byte) = word_bounds_at_offset(&text, cursor_byte);
    let s_char = char_offset_from_byte_offset(&text, s_byte);
    let e_char = char_offset_from_byte_offset(&text, e_byte);
    let s_iter = buffer.iter_at_offset(s_char as i32);
    let e_iter = buffer.iter_at_offset(e_char as i32);
    (s_iter, e_iter)
}

fn byte_offset_from_char_offset(text: &str, char_offset: usize) -> usize {
    text.char_indices()
        .nth(char_offset)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}

fn char_offset_from_byte_offset(text: &str, byte_offset: usize) -> usize {
    text.char_indices()
        .position(|(b, _)| b >= byte_offset)
        .unwrap_or_else(|| text.chars().count())
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub(crate) fn word_bounds_at_offset(text: &str, offset: usize) -> (usize, usize) {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let total = text.len();
    let safe_off = offset.min(total);
    let here_idx = chars.iter().position(|(i, _)| *i == safe_off);
    let here_word = here_idx.map(|i| is_word_char(chars[i].1)).unwrap_or(false);
    let prev_word = here_idx
        .and_then(|i| i.checked_sub(1))
        .or_else(|| {
            if here_idx.is_none() && !chars.is_empty() {
                Some(chars.len() - 1)
            } else {
                None
            }
        })
        .map(|i| is_word_char(chars[i].1))
        .unwrap_or(false);
    if !here_word && !prev_word {
        return (safe_off, safe_off);
    }
    let anchor_idx = if here_word {
        here_idx.unwrap()
    } else if let Some(i) = here_idx {
        i - 1
    } else {
        chars.len() - 1
    };
    let mut s = anchor_idx;
    while s > 0 && is_word_char(chars[s - 1].1) {
        s -= 1;
    }
    let mut e = anchor_idx;
    while e < chars.len() && is_word_char(chars[e].1) {
        e += 1;
    }
    let start_byte = chars[s].0;
    let end_byte = if e < chars.len() { chars[e].0 } else { total };
    (start_byte, end_byte)
}

pub(crate) fn move_line_up(state: &EditorTabState) {
    let buffer = &state.buffer;
    let mark = buffer.get_insert();
    let cursor = buffer.iter_at_mark(&mark);
    let line = cursor.line();
    if line == 0 {
        return;
    }
    swap_lines(buffer, line - 1, line);
    place_cursor_on_line(buffer, line - 1);
}

pub(crate) fn move_line_down(state: &EditorTabState) {
    let buffer = &state.buffer;
    let mark = buffer.get_insert();
    let cursor = buffer.iter_at_mark(&mark);
    let line = cursor.line();
    let last = buffer.end_iter().line();
    if line >= last {
        return;
    }
    swap_lines(buffer, line, line + 1);
    place_cursor_on_line(buffer, line + 1);
}

fn line_text(buffer: &sourceview5::Buffer, line: i32) -> String {
    let Some(start) = buffer.iter_at_line(line) else {
        return String::new();
    };
    let mut end = start;
    if !end.ends_line() {
        end.forward_to_line_end();
    }
    buffer.text(&start, &end, false).to_string()
}

fn swap_lines(buffer: &sourceview5::Buffer, upper: i32, lower: i32) {
    let upper_text = line_text(buffer, upper);
    let lower_text = line_text(buffer, lower);
    let Some(mut up_s) = buffer.iter_at_line(upper) else {
        return;
    };
    let mut up_e = up_s;
    if !up_e.ends_line() {
        up_e.forward_to_line_end();
    }
    buffer.begin_user_action();
    buffer.delete(&mut up_s, &mut up_e);
    let mut up_insert = buffer
        .iter_at_line(upper)
        .unwrap_or_else(|| buffer.start_iter());
    buffer.insert(&mut up_insert, &lower_text);
    let Some(mut lo_s) = buffer.iter_at_line(lower) else {
        buffer.end_user_action();
        return;
    };
    let mut lo_e = lo_s;
    if !lo_e.ends_line() {
        lo_e.forward_to_line_end();
    }
    buffer.delete(&mut lo_s, &mut lo_e);
    let mut lo_insert = buffer
        .iter_at_line(lower)
        .unwrap_or_else(|| buffer.end_iter());
    buffer.insert(&mut lo_insert, &upper_text);
    buffer.end_user_action();
}

fn place_cursor_on_line(buffer: &sourceview5::Buffer, line: i32) {
    let iter = buffer
        .iter_at_line(line)
        .unwrap_or_else(|| buffer.start_iter());
    buffer.place_cursor(&iter);
}

pub(crate) fn ensure_search_context(state: &EditorTabState) -> sourceview5::SearchContext {
    let widget: gtk4::Widget = state.root.clone().upcast();
    unsafe {
        if let Some(ptr) = widget.data::<sourceview5::SearchContext>(SEARCH_CONTEXT_KEY) {
            return ptr.as_ref().clone();
        }
    }
    let settings = sourceview5::SearchSettings::new();
    settings.set_wrap_around(true);
    let ctx = sourceview5::SearchContext::new(&state.buffer, Some(&settings));
    ctx.set_highlight(true);
    unsafe {
        widget.set_data::<sourceview5::SearchContext>(SEARCH_CONTEXT_KEY, ctx.clone());
    }
    ctx
}

pub(crate) fn show_find_bar(state: &EditorTabState, with_replace: bool) {
    crate::editor::find_bar::show(state, with_replace);
}

pub(crate) fn find_next(state: &EditorTabState) {
    crate::editor::find_bar::find_next(state);
}

pub(crate) fn show_goto_line(state: &EditorTabState) {
    let buffer = state.buffer.clone();
    let last_line = buffer.end_iter().line();
    let max_line = (last_line + 1).max(1) as f64;

    let dialog = gtk4::Window::builder()
        .title(strings::GOTO_LINE_TITLE)
        .modal(true)
        .resizable(false)
        .default_width(280)
        .build();
    if let Some(parent) = state
        .root
        .root()
        .and_then(|r| r.downcast::<gtk4::Window>().ok())
    {
        dialog.set_transient_for(Some(&parent));
    }

    let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    outer.set_margin_top(12);
    outer.set_margin_bottom(12);
    outer.set_margin_start(12);
    outer.set_margin_end(12);

    let label = gtk4::Label::new(Some(strings::GOTO_LINE_PROMPT));
    label.set_halign(gtk4::Align::Start);
    outer.append(&label);

    let adjustment = gtk4::Adjustment::new(1.0, 1.0, max_line, 1.0, 10.0, 0.0);
    let spin = gtk4::SpinButton::new(Some(&adjustment), 1.0, 0);
    spin.set_value(1.0);
    outer.append(&spin);

    let btn_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    btn_row.set_halign(gtk4::Align::End);
    let cancel = gtk4::Button::with_label(strings::DIALOG_BTN_CANCEL);
    let go = gtk4::Button::with_label(strings::GOTO_BTN);
    go.add_css_class("suggested-action");
    btn_row.append(&cancel);
    btn_row.append(&go);
    outer.append(&btn_row);

    dialog.set_child(Some(&outer));

    let dialog_for_cancel = dialog.clone();
    cancel.connect_clicked(move |_| dialog_for_cancel.close());

    let dialog_for_go = dialog.clone();
    let view = state.view.clone();
    let buffer_for_go = buffer.clone();
    let spin_for_go = spin.clone();
    go.connect_clicked(move |_| {
        let target = (spin_for_go.value_as_int() - 1).max(0);
        goto_line_apply(&buffer_for_go, &view, target);
        dialog_for_go.close();
    });

    let key_ctrl = gtk4::EventControllerKey::new();
    let dialog_for_key = dialog.clone();
    let view_key = state.view.clone();
    let buffer_key = buffer.clone();
    let spin_key = spin.clone();
    key_ctrl.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
            let target = (spin_key.value_as_int() - 1).max(0);
            goto_line_apply(&buffer_key, &view_key, target);
            dialog_for_key.close();
            return glib::Propagation::Stop;
        }
        if key == gtk4::gdk::Key::Escape {
            dialog_for_key.close();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    spin.add_controller(key_ctrl);

    dialog.present();
    spin.grab_focus();
}

fn goto_line_apply(buffer: &sourceview5::Buffer, view: &sourceview5::View, line: i32) {
    let iter = buffer
        .iter_at_line(line)
        .unwrap_or_else(|| buffer.end_iter());
    buffer.place_cursor(&iter);
    let mut scroll_iter = iter;
    view.scroll_to_iter(&mut scroll_iter, 0.1, true, 0.0, 0.5);
    view.grab_focus();
}

fn apply_save_transform(mut text: String, transform: SaveTransform) -> String {
    if transform.strip_trailing_whitespace {
        text = strip_trailing_ws(&text);
    }
    if transform.ensure_final_newline && !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn strip_trailing_ws(text: &str) -> String {
    let had_trailing_nl = text.ends_with('\n');
    let mut out = text
        .split('\n')
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    if had_trailing_nl && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{apply_save_transform, strip_trailing_ws, word_bounds_at_offset, SaveTransform};

    #[test]
    fn word_bounds_inside_word() {
        assert_eq!(word_bounds_at_offset("foo bar baz", 5), (4, 7));
    }

    #[test]
    fn word_bounds_at_word_start() {
        assert_eq!(word_bounds_at_offset("foo bar", 4), (4, 7));
    }

    #[test]
    fn word_bounds_at_word_end_returns_word() {
        assert_eq!(word_bounds_at_offset("foo bar", 3), (0, 3));
    }

    #[test]
    fn word_bounds_in_whitespace() {
        assert_eq!(word_bounds_at_offset("a  b", 2), (2, 2));
    }

    #[test]
    fn word_bounds_with_underscore() {
        assert_eq!(word_bounds_at_offset("snake_case x", 3), (0, 10));
    }

    #[test]
    fn word_bounds_alphanumeric() {
        assert_eq!(word_bounds_at_offset("abc123 x", 4), (0, 6));
    }

    #[test]
    fn word_bounds_empty_string() {
        assert_eq!(word_bounds_at_offset("", 0), (0, 0));
    }

    #[test]
    fn word_bounds_offset_past_end_after_word() {
        assert_eq!(word_bounds_at_offset("foo", 3), (0, 3));
    }

    #[test]
    fn word_bounds_offset_past_end_after_space() {
        assert_eq!(word_bounds_at_offset("foo ", 4), (4, 4));
    }

    #[test]
    fn strip_trailing_ws_basic() {
        assert_eq!(strip_trailing_ws("hello \nworld  \n"), "hello\nworld\n");
    }

    #[test]
    fn strip_trailing_ws_single_word() {
        assert_eq!(strip_trailing_ws("x"), "x");
    }

    #[test]
    fn strip_trailing_ws_empty() {
        assert_eq!(strip_trailing_ws(""), "");
    }

    #[test]
    fn strip_trailing_ws_only_newlines() {
        assert_eq!(strip_trailing_ws("\n\n"), "\n\n");
    }

    #[test]
    fn strip_trailing_ws_no_newline() {
        assert_eq!(strip_trailing_ws("hello"), "hello");
    }

    #[test]
    fn apply_save_transform_adds_final_newline() {
        let t = SaveTransform {
            strip_trailing_whitespace: false,
            ensure_final_newline: true,
        };
        assert_eq!(apply_save_transform("hello".to_string(), t), "hello\n");
    }

    #[test]
    fn apply_save_transform_skips_final_newline_on_empty() {
        let t = SaveTransform {
            strip_trailing_whitespace: false,
            ensure_final_newline: true,
        };
        assert_eq!(apply_save_transform(String::new(), t), "");
    }

    #[test]
    fn apply_save_transform_both() {
        let t = SaveTransform {
            strip_trailing_whitespace: true,
            ensure_final_newline: true,
        };
        assert_eq!(
            apply_save_transform("foo  \nbar".to_string(), t),
            "foo\nbar\n"
        );
    }

    #[test]
    fn apply_save_transform_disabled() {
        let t = SaveTransform::default();
        assert_eq!(
            apply_save_transform("foo  \nbar".to_string(), t),
            "foo  \nbar"
        );
    }
}
