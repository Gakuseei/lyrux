use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::glib;
use gtk4::glib::SignalHandlerId;
use gtk4::prelude::*;
use sourceview5::prelude::*;

use crate::editor::keymap;
use crate::editor::strings;
use crate::editor::tab_state::EditorTabState;

const FIND_BAR_STATE_KEY: &str = "lyrux-find-bar-state";
const AUTOFILL_MAX: usize = 100;

#[derive(Clone)]
struct FindBarState {
    bar: gtk4::SearchBar,
    entry: gtk4::SearchEntry,
    replace_row: gtk4::Box,
    replace_entry: gtk4::Entry,
    count_label: gtk4::Label,
    case_toggle: gtk4::ToggleButton,
    word_toggle: gtk4::ToggleButton,
    regex_toggle: gtk4::ToggleButton,
    replace_toggle: gtk4::ToggleButton,
    manual_case: Rc<Cell<bool>>,
    case_handler: Rc<RefCell<Option<SignalHandlerId>>>,
}

pub fn show(state: &EditorTabState, open_replace: bool) {
    let ctx = keymap::ensure_search_context(state);
    let fb = ensure_bar(state, &ctx);
    fb.bar.set_search_mode(true);
    set_replace_visible(&fb, open_replace);
    apply_autofill_from_selection(state, &fb);
    fb.entry.grab_focus();
    fb.entry.select_region(0, -1);
}

pub fn find_next(state: &EditorTabState) {
    let ctx = keymap::ensure_search_context(state);
    let view = state.view.clone();
    advance(&ctx, &view, true);
}

pub fn find_previous(state: &EditorTabState) {
    let ctx = keymap::ensure_search_context(state);
    let view = state.view.clone();
    advance(&ctx, &view, false);
}

fn ensure_bar(state: &EditorTabState, ctx: &sourceview5::SearchContext) -> FindBarState {
    let widget: gtk4::Widget = state.root.clone().upcast();
    unsafe {
        if let Some(ptr) = widget.data::<FindBarState>(FIND_BAR_STATE_KEY) {
            return ptr.as_ref().clone();
        }
    }

    let fb = build_bar(state, ctx);
    state.root.insert_child_after(&fb.bar, Some(&state.banner));
    wire_signals(state, ctx, &fb);
    unsafe {
        widget.set_data::<FindBarState>(FIND_BAR_STATE_KEY, fb.clone());
    }
    fb
}

fn build_bar(state: &EditorTabState, ctx: &sourceview5::SearchContext) -> FindBarState {
    let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    outer.add_css_class("lyrux-find-bar");

    let find_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);

    let prev_btn = gtk4::Button::with_label(strings::FIND_PREV_LABEL);
    prev_btn.set_tooltip_text(Some(strings::FIND_PREV_TOOLTIP));
    prev_btn.add_css_class("lyrux-find-nav");
    prev_btn.set_focus_on_click(false);
    let next_btn = gtk4::Button::with_label(strings::FIND_NEXT_LABEL);
    next_btn.set_tooltip_text(Some(strings::FIND_NEXT_TOOLTIP));
    next_btn.add_css_class("lyrux-find-nav");
    next_btn.set_focus_on_click(false);

    let entry = gtk4::SearchEntry::builder()
        .placeholder_text(strings::FIND_PLACEHOLDER)
        .build();
    entry.set_hexpand(true);

    let count_label = gtk4::Label::new(Some(""));
    count_label.add_css_class("lyrux-find-count");
    count_label.set_xalign(0.0);

    let case_toggle = gtk4::ToggleButton::with_label(strings::FIND_CASE_LABEL);
    case_toggle.set_tooltip_text(Some(strings::FIND_CASE_TOOLTIP));
    case_toggle.add_css_class("lyrux-find-toggle");
    case_toggle.set_focus_on_click(false);

    let word_toggle = gtk4::ToggleButton::with_label(strings::FIND_WORD_LABEL);
    word_toggle.set_tooltip_text(Some(strings::FIND_WORD_TOOLTIP));
    word_toggle.add_css_class("lyrux-find-toggle");
    word_toggle.set_focus_on_click(false);

    let regex_toggle = gtk4::ToggleButton::with_label(strings::FIND_REGEX_LABEL);
    regex_toggle.set_tooltip_text(Some(strings::FIND_REGEX_TOOLTIP));
    regex_toggle.add_css_class("lyrux-find-toggle");
    regex_toggle.set_focus_on_click(false);

    let replace_toggle = gtk4::ToggleButton::with_label(strings::FIND_TOGGLE_REPLACE_LABEL);
    replace_toggle.set_tooltip_text(Some(strings::FIND_TOGGLE_REPLACE_TOOLTIP));
    replace_toggle.add_css_class("lyrux-find-toggle");
    replace_toggle.set_focus_on_click(false);

    let close_btn = gtk4::Button::with_label(strings::FIND_CLOSE_LABEL);
    close_btn.set_tooltip_text(Some(strings::FIND_CLOSE_TOOLTIP));
    close_btn.add_css_class("lyrux-find-nav");
    close_btn.set_focus_on_click(false);

    find_row.append(&prev_btn);
    find_row.append(&next_btn);
    find_row.append(&entry);
    find_row.append(&count_label);
    find_row.append(&case_toggle);
    find_row.append(&word_toggle);
    find_row.append(&regex_toggle);
    find_row.append(&replace_toggle);
    find_row.append(&close_btn);

    let replace_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    let replace_entry = gtk4::Entry::builder()
        .placeholder_text(strings::REPLACE_ENTRY_PLACEHOLDER)
        .build();
    replace_entry.set_hexpand(true);
    let replace_btn = gtk4::Button::with_label(strings::REPLACE_BTN);
    let replace_all_btn = gtk4::Button::with_label(strings::REPLACE_ALL_BTN);
    replace_row.append(&replace_entry);
    replace_row.append(&replace_btn);
    replace_row.append(&replace_all_btn);
    replace_row.set_visible(false);

    outer.append(&find_row);
    outer.append(&replace_row);

    let bar = gtk4::SearchBar::builder()
        .child(&outer)
        .search_mode_enabled(true)
        .show_close_button(false)
        .build();
    bar.connect_entry(&entry);

    let ctx_prev = ctx.clone();
    let view_prev = state.view.clone();
    prev_btn.connect_clicked(move |_| {
        advance(&ctx_prev, &view_prev, false);
    });

    let ctx_next = ctx.clone();
    let view_next = state.view.clone();
    next_btn.connect_clicked(move |_| {
        advance(&ctx_next, &view_next, true);
    });

    let ctx_one = ctx.clone();
    let view_one = state.view.clone();
    let replace_entry_one = replace_entry.clone();
    replace_btn.connect_clicked(move |_| {
        replace_one(&ctx_one, &view_one, &replace_entry_one.text());
    });

    let ctx_all = ctx.clone();
    let replace_entry_all = replace_entry.clone();
    let count_label_all = count_label.clone();
    replace_all_btn.connect_clicked(move |_| {
        do_replace_all(&ctx_all, &replace_entry_all.text(), &count_label_all);
    });

    let bar_close = bar.clone();
    close_btn.connect_clicked(move |_| {
        bar_close.set_search_mode(false);
    });

    let replace_row_toggle = replace_row.clone();
    replace_toggle.connect_toggled(move |tb| {
        replace_row_toggle.set_visible(tb.is_active());
    });

    FindBarState {
        bar,
        entry,
        replace_row,
        replace_entry,
        count_label,
        case_toggle,
        word_toggle,
        regex_toggle,
        replace_toggle,
        manual_case: Rc::new(Cell::new(false)),
        case_handler: Rc::new(RefCell::new(None)),
    }
}

fn wire_signals(state: &EditorTabState, ctx: &sourceview5::SearchContext, fb: &FindBarState) {
    let settings = ctx.settings();

    let settings_changed = settings.clone();
    let ctx_changed = ctx.clone();
    let view_changed = state.view.clone();
    let manual_case_changed = fb.manual_case.clone();
    let case_toggle_changed = fb.case_toggle.clone();
    let case_handler_changed = fb.case_handler.clone();
    let settings_smart = settings.clone();
    fb.entry.connect_search_changed(move |e| {
        let text = e.text().to_string();
        if !manual_case_changed.get() {
            let has_upper = text.chars().any(|c| c.is_uppercase());
            let handler = case_handler_changed.borrow();
            if let Some(id) = handler.as_ref() {
                case_toggle_changed.block_signal(id);
                case_toggle_changed.set_active(has_upper);
                case_toggle_changed.unblock_signal(id);
            } else {
                case_toggle_changed.set_active(has_upper);
            }
            settings_smart.set_case_sensitive(has_upper);
        }
        settings_changed.set_search_text(Some(&text));
        let buffer = ctx_changed.buffer();
        let iter = buffer.iter_at_mark(&buffer.get_insert());
        if let Some((s, e_iter, _wrap)) = ctx_changed.forward(&iter) {
            buffer.select_range(&s, &e_iter);
            scroll_to(&view_changed, &buffer, &s);
        }
    });

    let ctx_enter = ctx.clone();
    let view_enter = state.view.clone();
    fb.entry.connect_activate(move |_| {
        advance(&ctx_enter, &view_enter, true);
    });

    let key_ctrl = gtk4::EventControllerKey::new();
    let ctx_key = ctx.clone();
    let view_key = state.view.clone();
    let replace_entry_key = fb.replace_entry.clone();
    let count_label_key = fb.count_label.clone();
    let bar_key = fb.bar.clone();
    key_ctrl.connect_key_pressed(move |_, key, _, mods| {
        let shift = mods.contains(gtk4::gdk::ModifierType::SHIFT_MASK);
        let ctrl = mods.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
        let alt = mods.contains(gtk4::gdk::ModifierType::ALT_MASK);
        match key {
            gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter => {
                if ctrl && alt {
                    do_replace_all(&ctx_key, &replace_entry_key.text(), &count_label_key);
                    return glib::Propagation::Stop;
                }
                advance(&ctx_key, &view_key, !shift);
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::Escape => {
                bar_key.set_search_mode(false);
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
    fb.entry.add_controller(key_ctrl);

    let key_ctrl_replace = gtk4::EventControllerKey::new();
    let ctx_rkey = ctx.clone();
    let view_rkey = state.view.clone();
    let replace_entry_rkey = fb.replace_entry.clone();
    let count_label_rkey = fb.count_label.clone();
    let bar_rkey = fb.bar.clone();
    key_ctrl_replace.connect_key_pressed(move |_, key, _, mods| {
        let ctrl = mods.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
        let alt = mods.contains(gtk4::gdk::ModifierType::ALT_MASK);
        match key {
            gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter => {
                if ctrl && alt {
                    do_replace_all(&ctx_rkey, &replace_entry_rkey.text(), &count_label_rkey);
                } else {
                    replace_one(&ctx_rkey, &view_rkey, &replace_entry_rkey.text());
                }
                glib::Propagation::Stop
            }
            gtk4::gdk::Key::Escape => {
                bar_rkey.set_search_mode(false);
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
    fb.replace_entry.add_controller(key_ctrl_replace);

    let settings_case = settings.clone();
    let manual_case_marker = fb.manual_case.clone();
    let ctx_case = ctx.clone();
    let case_handler_id = fb.case_toggle.connect_toggled(move |tb| {
        manual_case_marker.set(true);
        settings_case.set_case_sensitive(tb.is_active());
        retrigger(&ctx_case);
    });
    *fb.case_handler.borrow_mut() = Some(case_handler_id);

    let settings_word = settings.clone();
    let ctx_word = ctx.clone();
    fb.word_toggle.connect_toggled(move |tb| {
        settings_word.set_at_word_boundaries(tb.is_active());
        retrigger(&ctx_word);
    });

    let settings_regex = settings;
    let ctx_regex = ctx.clone();
    fb.regex_toggle.connect_toggled(move |tb| {
        settings_regex.set_regex_enabled(tb.is_active());
        retrigger(&ctx_regex);
    });

    let count_label_occ = fb.count_label.clone();
    let ctx_occ = ctx.clone();
    ctx.connect_occurrences_count_notify(move |c| {
        update_count(&ctx_occ, &count_label_occ, c.occurrences_count());
    });

    let count_label_cur = fb.count_label.clone();
    let ctx_cur = ctx.clone();
    state.buffer.connect_cursor_position_notify(move |_| {
        update_count(&ctx_cur, &count_label_cur, ctx_cur.occurrences_count());
    });

    let entry_err = fb.entry.clone();
    ctx.connect_regex_error_notify(move |c| {
        if c.regex_error().is_some() {
            entry_err.add_css_class("lyrux-find-invalid");
            entry_err.set_tooltip_text(Some(strings::FIND_REGEX_INVALID));
        } else {
            entry_err.remove_css_class("lyrux-find-invalid");
            entry_err.set_tooltip_text(None);
        }
    });
}

fn set_replace_visible(fb: &FindBarState, visible: bool) {
    if visible {
        if !fb.replace_toggle.is_active() {
            fb.replace_toggle.set_active(true);
        } else {
            fb.replace_row.set_visible(true);
        }
    }
}

fn apply_autofill_from_selection(state: &EditorTabState, fb: &FindBarState) {
    let Some((s, e)) = state.buffer.selection_bounds() else {
        return;
    };
    if s.offset() == e.offset() {
        return;
    }
    let text = state.buffer.text(&s, &e, false).to_string();
    if text.contains('\n') {
        return;
    }
    let truncated: String = text.chars().take(AUTOFILL_MAX).collect();
    if truncated.is_empty() {
        return;
    }
    fb.entry.set_text(&truncated);
    fb.entry.set_position(-1);
}

fn advance(ctx: &sourceview5::SearchContext, view: &sourceview5::View, forward: bool) {
    let buffer = ctx.buffer();
    let search_from = if let Some((sel_s, sel_e)) = buffer.selection_bounds() {
        if forward {
            sel_e
        } else {
            sel_s
        }
    } else {
        buffer.iter_at_mark(&buffer.get_insert())
    };
    let hit = if forward {
        ctx.forward(&search_from)
    } else {
        ctx.backward(&search_from)
    };
    if let Some((s, e, _wrap)) = hit {
        buffer.select_range(&s, &e);
        scroll_to(view, &buffer, &s);
    }
}

fn replace_one(ctx: &sourceview5::SearchContext, view: &sourceview5::View, replacement: &str) {
    let buffer = ctx.buffer();
    let Some((mut s, mut e)) = buffer.selection_bounds() else {
        advance(ctx, view, true);
        return;
    };
    let _ = ctx.replace(&mut s, &mut e, replacement);
    advance(ctx, view, true);
}

fn do_replace_all(ctx: &sourceview5::SearchContext, replacement: &str, count_label: &gtk4::Label) {
    let before = ctx.occurrences_count().max(0) as u32;
    if ctx.replace_all(replacement).is_err() {
        return;
    }
    count_label.set_text(&strings::find_replaced(before));
    let label = count_label.clone();
    let ctx_after = ctx.clone();
    glib::timeout_add_local_once(std::time::Duration::from_millis(1600), move || {
        update_count(&ctx_after, &label, ctx_after.occurrences_count());
    });
}

fn retrigger(ctx: &sourceview5::SearchContext) {
    let buffer = ctx.buffer();
    let iter = buffer.iter_at_mark(&buffer.get_insert());
    let _ = ctx.forward(&iter);
}

fn update_count(ctx: &sourceview5::SearchContext, label: &gtk4::Label, total: i32) {
    if total <= 0 {
        let q = ctx
            .settings()
            .search_text()
            .map(|g| g.to_string())
            .unwrap_or_default();
        if q.is_empty() {
            label.set_text("");
        } else {
            label.set_text(strings::FIND_COUNT_NONE);
        }
        return;
    }
    let buffer = ctx.buffer();
    let bounds = buffer.selection_bounds();
    let pos = match bounds {
        Some((s, e)) => ctx.occurrence_position(&s, &e),
        None => -1,
    };
    let m = total as u32;
    if pos > 0 {
        label.set_text(&strings::find_count(pos as u32, m));
    } else {
        label.set_text(&strings::find_count_pending(m));
    }
}

fn scroll_to(view: &sourceview5::View, _buffer: &sourceview5::Buffer, iter: &gtk4::TextIter) {
    let mut it = *iter;
    view.scroll_to_iter(&mut it, 0.1, false, 0.5, 0.5);
}
