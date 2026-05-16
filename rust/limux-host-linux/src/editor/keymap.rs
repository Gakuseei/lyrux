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

const FIND_BAR_KEY: &str = "lyrux-find-bar";
const SEARCH_ENTRY_KEY: &str = "lyrux-search-entry";
const REPLACE_ENTRY_KEY: &str = "lyrux-replace-entry";
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
            gtk4::gdk::Key::d => {
                duplicate_line(&state);
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

fn toggle_line_comment(state: &EditorTabState) {
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

fn duplicate_line(state: &EditorTabState) {
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

fn delete_line(state: &EditorTabState) {
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

fn move_line_up(state: &EditorTabState) {
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

fn move_line_down(state: &EditorTabState) {
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

fn ensure_search_context(state: &EditorTabState) -> sourceview5::SearchContext {
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

fn show_find_bar(state: &EditorTabState, with_replace: bool) {
    let ctx = ensure_search_context(state);
    let bar = ensure_find_bar(state, &ctx, with_replace);
    bar.set_search_mode(true);
    let widget: gtk4::Widget = state.root.clone().upcast();
    unsafe {
        if let Some(ptr) = widget.data::<gtk4::SearchEntry>(SEARCH_ENTRY_KEY) {
            let entry = ptr.as_ref().clone();
            entry.grab_focus();
        }
    }
}

fn ensure_find_bar(
    state: &EditorTabState,
    ctx: &sourceview5::SearchContext,
    with_replace: bool,
) -> gtk4::SearchBar {
    let widget: gtk4::Widget = state.root.clone().upcast();
    unsafe {
        if let Some(ptr) = widget.data::<gtk4::SearchBar>(FIND_BAR_KEY) {
            let bar = ptr.as_ref().clone();
            ensure_replace_widgets(state, ctx, &bar, with_replace);
            return bar;
        }
    }

    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let entry = gtk4::SearchEntry::builder()
        .placeholder_text(strings::FIND_PLACEHOLDER)
        .build();
    entry.set_hexpand(true);
    row.append(&entry);

    let bar = gtk4::SearchBar::builder()
        .child(&row)
        .search_mode_enabled(true)
        .show_close_button(true)
        .build();
    bar.connect_entry(&entry);

    state.root.insert_child_after(&bar, Some(&state.banner));

    let settings = ctx.settings();
    let ctx_for_search = ctx.clone();
    entry.connect_search_changed(move |e| {
        settings.set_search_text(Some(&e.text()));
        let buffer = ctx_for_search.buffer();
        let iter = buffer.iter_at_mark(&buffer.get_insert());
        if let Some((s, _e, _wrap)) = ctx_for_search.forward(&iter) {
            buffer.place_cursor(&s);
        }
    });

    let ctx_for_next = ctx.clone();
    entry.connect_activate(move |_| {
        advance_search(&ctx_for_next);
    });

    unsafe {
        widget.set_data::<gtk4::SearchBar>(FIND_BAR_KEY, bar.clone());
        widget.set_data::<gtk4::SearchEntry>(SEARCH_ENTRY_KEY, entry);
    }

    ensure_replace_widgets(state, ctx, &bar, with_replace);
    bar
}

fn ensure_replace_widgets(
    state: &EditorTabState,
    ctx: &sourceview5::SearchContext,
    bar: &gtk4::SearchBar,
    with_replace: bool,
) {
    if !with_replace {
        return;
    }
    let widget: gtk4::Widget = state.root.clone().upcast();
    unsafe {
        if widget.data::<gtk4::Entry>(REPLACE_ENTRY_KEY).is_some() {
            return;
        }
    }
    let row = match bar.child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
        Some(r) => r,
        None => return,
    };
    let replace_entry = gtk4::Entry::builder()
        .placeholder_text(strings::REPLACE_PLACEHOLDER)
        .build();
    let replace_btn = gtk4::Button::with_label(strings::REPLACE_BTN);
    let replace_all_btn = gtk4::Button::with_label(strings::REPLACE_ALL_BTN);
    row.append(&replace_entry);
    row.append(&replace_btn);
    row.append(&replace_all_btn);

    let ctx_one = ctx.clone();
    let entry_one = replace_entry.clone();
    replace_btn.connect_clicked(move |_| {
        replace_one(&ctx_one, &entry_one.text());
    });
    let ctx_all = ctx.clone();
    let entry_all = replace_entry.clone();
    replace_all_btn.connect_clicked(move |_| {
        let _ = ctx_all.replace_all(&entry_all.text());
    });

    unsafe {
        widget.set_data::<gtk4::Entry>(REPLACE_ENTRY_KEY, replace_entry);
    }
}

fn replace_one(ctx: &sourceview5::SearchContext, replacement: &str) {
    let buffer = ctx.buffer();
    let Some((mut s, mut e)) = buffer.selection_bounds() else {
        advance_search(ctx);
        return;
    };
    let _ = ctx.replace(&mut s, &mut e, replacement);
    advance_search(ctx);
}

fn advance_search(ctx: &sourceview5::SearchContext) {
    let buffer = ctx.buffer();
    let iter = buffer.iter_at_mark(&buffer.get_insert());
    if let Some((s, e, _wrap)) = ctx.forward(&iter) {
        buffer.select_range(&s, &e);
    }
}

fn find_next(state: &EditorTabState) {
    let ctx = ensure_search_context(state);
    advance_search(&ctx);
}

fn show_goto_line(state: &EditorTabState) {
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
    use super::{apply_save_transform, strip_trailing_ws, SaveTransform};

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
