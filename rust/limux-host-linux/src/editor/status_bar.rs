#![allow(dead_code)]

use std::rc::Rc;

use gtk4::prelude::*;
use sourceview5::prelude::*;

use crate::editor::strings;
use crate::editor::view::ViewConfig;

pub struct StatusBar {
    pub root: gtk4::Box,
    pub wrap_button: gtk4::Button,
}

pub fn build(buffer: &sourceview5::Buffer, cfg: &ViewConfig) -> StatusBar {
    let bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    bar.add_css_class("lyrux-editor-statusbar");
    bar.set_margin_top(2);
    bar.set_margin_bottom(2);
    bar.set_margin_start(8);
    bar.set_margin_end(8);

    let line_col_label = gtk4::Label::new(Some(""));
    line_col_label.set_xalign(0.0);
    bar.append(&line_col_label);

    let lang_label = gtk4::Label::new(Some(""));
    lang_label.set_xalign(0.0);
    bar.append(&lang_label);

    let indent_label = gtk4::Label::new(Some(""));
    indent_label.set_xalign(0.0);
    bar.append(&indent_label);

    let eol_label = gtk4::Label::new(Some(strings::STATUS_EOL_LF));
    eol_label.set_xalign(1.0);
    eol_label.set_hexpand(true);
    bar.append(&eol_label);

    let wrap_button = gtk4::Button::builder()
        .label(wrap_label_text(cfg.wrap_lines))
        .has_frame(false)
        .tooltip_text(strings::STATUS_WRAP_TOOLTIP)
        .action_name("win.editor-toggle-wrap")
        .build();
    wrap_button.add_css_class("lyrux-editor-statusbar-wrap");
    bar.append(&wrap_button);

    let encoding_label = gtk4::Label::new(Some(strings::STATUS_ENCODING_UTF8));
    encoding_label.set_xalign(1.0);
    bar.append(&encoding_label);

    let indent_text = if cfg.insert_spaces {
        format!("{} {}", strings::STATUS_SPACES, cfg.tab_width)
    } else {
        format!("{} {}", strings::STATUS_TAB_WIDTH, cfg.tab_width)
    };
    indent_label.set_text(&indent_text);

    let buffer_weak = buffer.downgrade();
    let line_col_l = line_col_label.clone();
    let lang_l = lang_label.clone();
    let update: Rc<dyn Fn()> = Rc::new(move || {
        let Some(buf) = buffer_weak.upgrade() else {
            return;
        };
        let cursor = buf.iter_at_mark(&buf.get_insert());
        let ln = cursor.line() + 1;
        let col = cursor.line_offset() + 1;
        line_col_l.set_text(&format!(
            "{}{}, {}{}",
            strings::STATUS_LINE_PREFIX,
            ln,
            strings::STATUS_COL_PREFIX,
            col
        ));

        let lang_name = buf
            .language()
            .map(|l| l.name().to_string())
            .unwrap_or_else(|| strings::STATUS_LANG_PLAIN_TEXT.to_string());
        lang_l.set_text(&lang_name);
    });
    update();

    {
        let update = update.clone();
        buffer.connect_cursor_position_notify(move |_| update());
    }
    {
        let update = update.clone();
        buffer.connect_language_notify(move |_| update());
    }

    StatusBar {
        root: bar,
        wrap_button,
    }
}

pub fn wrap_label_text(wrap_lines: bool) -> &'static str {
    if wrap_lines {
        strings::STATUS_WRAP_ON
    } else {
        strings::STATUS_WRAP_OFF
    }
}
