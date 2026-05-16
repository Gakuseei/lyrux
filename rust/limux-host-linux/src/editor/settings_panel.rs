use std::rc::Rc;

use gtk4::prelude::*;

use crate::editor::settings::EditorSettings;
use crate::editor::{strings, themes};

pub struct SettingsCallbacks {
    pub on_change: Rc<dyn Fn(&EditorSettings)>,
}

pub fn build(current: &EditorSettings, cb: SettingsCallbacks) -> gtk4::Widget {
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);

    let title = gtk4::Label::new(Some(strings::SECTION_EDITOR));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    root.append(&title);

    root.append(&section_header(strings::SETTINGS_SECTION_DISPLAY, false));
    root.append(&theme_row(current, cb.on_change.clone()));
    root.append(&font_row(current, cb.on_change.clone()));
    root.append(&bool_row(
        strings::SETTING_LINE_NUMBERS,
        current.show_line_numbers,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.show_line_numbers = v;
                cb(&next);
            }
        },
    ));
    root.append(&bool_row(
        strings::SETTING_SHOW_WHITESPACE,
        current.show_whitespace,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.show_whitespace = v;
                cb(&next);
            }
        },
    ));
    root.append(&bool_row(
        strings::SETTING_WRAP_LINES,
        current.wrap_lines,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.wrap_lines = v;
                cb(&next);
            }
        },
    ));
    root.append(&bool_row(
        strings::SETTING_HIGHLIGHT_LINE,
        current.highlight_current_line,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.highlight_current_line = v;
                cb(&next);
            }
        },
    ));
    root.append(&bool_row(
        strings::SETTING_HIGHLIGHT_BRACKETS,
        current.highlight_matching_brackets,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.highlight_matching_brackets = v;
                cb(&next);
            }
        },
    ));
    root.append(&bool_row(
        strings::SETTING_INDENT_GUIDES,
        current.show_indent_guides,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.show_indent_guides = v;
                cb(&next);
            }
        },
    ));
    root.append(&bool_row(
        strings::SETTING_STICKY_SCROLL,
        current.show_sticky_scroll,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.show_sticky_scroll = v;
                cb(&next);
            }
        },
    ));
    root.append(&bool_row(
        strings::SETTING_HIGHLIGHT_WORD,
        current.highlight_word_at_cursor,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.highlight_word_at_cursor = v;
                cb(&next);
            }
        },
    ));

    root.append(&section_header(strings::SETTINGS_SECTION_EDITING, true));
    root.append(&tab_width_row(current, cb.on_change.clone()));
    root.append(&bool_row(
        strings::SETTING_INSERT_SPACES,
        current.insert_spaces,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.insert_spaces = v;
                cb(&next);
            }
        },
    ));
    root.append(&bool_row(
        strings::SETTING_AUTO_INDENT,
        current.auto_indent,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.auto_indent = v;
                cb(&next);
            }
        },
    ));

    root.append(&section_header(strings::SETTINGS_SECTION_ON_SAVE, true));
    root.append(&bool_row(
        strings::SETTING_STRIP_WS,
        current.strip_trailing_whitespace,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.strip_trailing_whitespace = v;
                cb(&next);
            }
        },
    ));
    root.append(&bool_row(
        strings::SETTING_FINAL_NEWLINE,
        current.ensure_final_newline,
        {
            let cb = cb.on_change.clone();
            let snapshot = current.clone();
            move |v| {
                let mut next = snapshot.clone();
                next.ensure_final_newline = v;
                cb(&next);
            }
        },
    ));

    root.append(&vim_row());

    let scroller = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .child(&root)
        .hexpand(true)
        .vexpand(true)
        .build();
    scroller.upcast()
}

fn section_header(label: &str, top_margin: bool) -> gtk4::Label {
    let header = gtk4::Label::new(Some(label));
    header.set_xalign(0.0);
    header.add_css_class("title-3");
    if top_margin {
        header.set_margin_top(8);
    }
    header
}

fn theme_row(current: &EditorSettings, on_change: Rc<dyn Fn(&EditorSettings)>) -> gtk4::Box {
    let row = labeled_row(strings::SETTING_THEME);
    let labels: Vec<&str> = themes::available()
        .iter()
        .map(|(_, label, _)| *label)
        .collect();
    let dropdown = gtk4::DropDown::from_strings(&labels);
    let pos = themes::available()
        .iter()
        .position(|(id, _, _)| *id == current.theme_id)
        .unwrap_or(0) as u32;
    dropdown.set_selected(pos);
    dropdown.set_valign(gtk4::Align::Center);
    let snapshot = current.clone();
    dropdown.connect_selected_notify(move |dd| {
        let idx = dd.selected() as usize;
        if let Some((id, _, _)) = themes::available().get(idx) {
            let mut next = snapshot.clone();
            next.theme_id = (*id).to_string();
            on_change(&next);
        }
    });
    row.append(&dropdown);
    row
}

fn font_row(current: &EditorSettings, on_change: Rc<dyn Fn(&EditorSettings)>) -> gtk4::Box {
    let row = labeled_row(strings::SETTING_FONT);
    let monospace_filter = gtk4::CustomFilter::new(|obj| {
        obj.downcast_ref::<gtk4::pango::FontFamily>()
            .map(|family| family.is_monospace())
            .unwrap_or(false)
    });
    let dialog = gtk4::FontDialog::builder()
        .modal(true)
        .title(strings::SETTING_FONT)
        .filter(&monospace_filter)
        .build();
    let initial_desc = gtk4::pango::FontDescription::from_string(&format!(
        "{} {}",
        current.font_family, current.font_size
    ));
    let btn = gtk4::FontDialogButton::builder()
        .dialog(&dialog)
        .font_desc(&initial_desc)
        .build();
    btn.set_valign(gtk4::Align::Center);
    let snapshot = current.clone();
    btn.connect_font_desc_notify(move |b| {
        if let Some(desc) = b.font_desc() {
            let family = desc.family().map(|s| s.to_string()).unwrap_or_default();
            let size_pts = (desc.size() / gtk4::pango::SCALE).max(8);
            let mut next = snapshot.clone();
            if !family.is_empty() {
                next.font_family = family;
            }
            next.font_size = size_pts;
            on_change(&next);
        }
    });
    row.append(&btn);
    row
}

fn tab_width_row(current: &EditorSettings, on_change: Rc<dyn Fn(&EditorSettings)>) -> gtk4::Box {
    let row = labeled_row(strings::SETTING_TAB_WIDTH);
    let adj = gtk4::Adjustment::new(current.tab_width as f64, 1.0, 8.0, 1.0, 1.0, 0.0);
    let spin = gtk4::SpinButton::builder()
        .adjustment(&adj)
        .numeric(true)
        .build();
    spin.set_valign(gtk4::Align::Center);
    let snapshot = current.clone();
    spin.connect_value_changed(move |s| {
        let mut next = snapshot.clone();
        next.tab_width = s.value() as u32;
        on_change(&next);
    });
    row.append(&spin);
    row
}

fn bool_row(label: &str, initial: bool, on_change: impl Fn(bool) + 'static) -> gtk4::Box {
    let row = labeled_row(label);
    let switch = gtk4::Switch::builder().active(initial).build();
    switch.set_valign(gtk4::Align::Center);
    switch.connect_active_notify(move |s| on_change(s.is_active()));
    row.append(&switch);
    row
}

fn vim_row() -> gtk4::Box {
    let row = labeled_row(strings::SETTING_VIM);
    let switch = gtk4::Switch::builder()
        .active(false)
        .sensitive(false)
        .build();
    switch.set_valign(gtk4::Align::Center);
    row.append(&switch);
    row
}

fn labeled_row(label: &str) -> gtk4::Box {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    row.set_margin_top(2);
    let lbl = gtk4::Label::new(Some(label));
    lbl.set_xalign(0.0);
    lbl.set_hexpand(true);
    row.append(&lbl);
    row
}
