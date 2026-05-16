#![allow(dead_code)]

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;
use sourceview5::prelude::*;

pub fn pair_for(open: char, lang_id: &str) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '`' => Some('`'),
        '<' if is_markup_lang(lang_id) => Some('>'),
        _ => None,
    }
}

pub fn close_for(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '`' => Some('`'),
        '<' => Some('>'),
        _ => None,
    }
}

pub fn is_close_char(c: char) -> bool {
    matches!(c, ')' | ']' | '}' | '"' | '\'' | '`' | '>')
}

pub fn is_open_char(c: char) -> bool {
    matches!(c, '(' | '[' | '{' | '"' | '\'' | '`' | '<')
}

pub fn is_markup_lang(lang_id: &str) -> bool {
    matches!(lang_id, "html" | "xml" | "markdown" | "css")
}

pub fn should_autoclose_quote_at(prev: Option<char>) -> bool {
    !matches!(prev, Some(c) if c.is_alphanumeric() || c == '_')
}

pub fn is_triple_quote_trigger(prev_two: (Option<char>, Option<char>), open: char) -> bool {
    open == '"' && prev_two.0 == Some('"') && prev_two.1 == Some('"')
}

pub fn install(view: &sourceview5::View, buffer: &sourceview5::Buffer) {
    let ctrl = gtk::EventControllerKey::new();
    ctrl.set_propagation_phase(gtk::PropagationPhase::Capture);
    let buffer = buffer.clone();
    ctrl.connect_key_pressed(move |_, key, _, mods| {
        if mods.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            || mods.contains(gtk::gdk::ModifierType::ALT_MASK)
            || mods.contains(gtk::gdk::ModifierType::META_MASK)
        {
            return glib::Propagation::Proceed;
        }

        if key == gtk::gdk::Key::BackSpace {
            return handle_backspace(&buffer);
        }

        let ch = match key.to_unicode() {
            Some(c) => c,
            None => return glib::Propagation::Proceed,
        };

        let lang_id = buffer
            .language()
            .map(|l| l.id().to_string())
            .unwrap_or_default();

        if is_close_char(ch) && handle_close_skip(&buffer, ch) {
            return glib::Propagation::Stop;
        }

        if is_open_char(ch) {
            return handle_open(&buffer, ch, &lang_id);
        }

        glib::Propagation::Proceed
    });
    view.add_controller(ctrl);
}

fn handle_close_skip(buffer: &sourceview5::Buffer, ch: char) -> bool {
    if buffer.selection_bounds().is_some() {
        return false;
    }
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let next = cursor.char();
    if next == ch {
        let mut advance = cursor;
        if advance.forward_char() {
            buffer.place_cursor(&advance);
            return true;
        }
    }
    false
}

fn handle_open(buffer: &sourceview5::Buffer, ch: char, lang_id: &str) -> glib::Propagation {
    let close = match pair_for(ch, lang_id) {
        Some(c) => c,
        None => return glib::Propagation::Proceed,
    };

    if let Some((s, e)) = buffer.selection_bounds() {
        let s_off = s.offset();
        let e_off = e.offset();
        buffer.begin_user_action();
        let mut end_iter = buffer.iter_at_offset(e_off);
        buffer.insert(&mut end_iter, &close.to_string());
        let mut start_iter = buffer.iter_at_offset(s_off);
        buffer.insert(&mut start_iter, &ch.to_string());
        buffer.end_user_action();
        let select_start = buffer.iter_at_offset(s_off + 1);
        let select_end = buffer.iter_at_offset(e_off + 1);
        buffer.select_range(&select_start, &select_end);
        return glib::Propagation::Stop;
    }

    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let prev = prev_char(&cursor);

    if ch == '\'' && !should_autoclose_quote_at(prev) {
        return glib::Propagation::Proceed;
    }

    if ch == '"' {
        let prev2 = prev_two_chars(&cursor);
        if is_triple_quote_trigger(prev2, ch) {
            return glib::Propagation::Proceed;
        }
    }

    if (ch == '"' || ch == '\'' || ch == '`') && cursor.char() == ch {
        let mut advance = cursor;
        if advance.forward_char() {
            buffer.place_cursor(&advance);
            return glib::Propagation::Stop;
        }
    }

    buffer.begin_user_action();
    let pair_str = format!("{ch}{close}");
    buffer.insert_at_cursor(&pair_str);
    let mut after = buffer.iter_at_mark(&buffer.get_insert());
    if after.backward_char() {
        buffer.place_cursor(&after);
    }
    buffer.end_user_action();
    glib::Propagation::Stop
}

fn handle_backspace(buffer: &sourceview5::Buffer) -> glib::Propagation {
    if buffer.selection_bounds().is_some() {
        return glib::Propagation::Proceed;
    }
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let prev = match prev_char(&cursor) {
        Some(c) => c,
        None => return glib::Propagation::Proceed,
    };
    let close = match close_for(prev) {
        Some(c) => c,
        None => return glib::Propagation::Proceed,
    };
    if cursor.char() != close {
        return glib::Propagation::Proceed;
    }
    let mut start = cursor;
    if !start.backward_char() {
        return glib::Propagation::Proceed;
    }
    let mut end = cursor;
    if !end.forward_char() {
        return glib::Propagation::Proceed;
    }
    buffer.begin_user_action();
    buffer.delete(&mut start, &mut end);
    buffer.end_user_action();
    glib::Propagation::Stop
}

fn prev_char(iter: &gtk::TextIter) -> Option<char> {
    let mut back = *iter;
    if !back.backward_char() {
        return None;
    }
    Some(back.char())
}

fn prev_two_chars(iter: &gtk::TextIter) -> (Option<char>, Option<char>) {
    let mut a = *iter;
    if !a.backward_char() {
        return (None, None);
    }
    let ca = Some(a.char());
    let mut b = a;
    if !b.backward_char() {
        return (ca, None);
    }
    (ca, Some(b.char()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_brackets() {
        assert_eq!(pair_for('(', "rust"), Some(')'));
        assert_eq!(pair_for('[', "rust"), Some(']'));
        assert_eq!(pair_for('{', "rust"), Some('}'));
    }

    #[test]
    fn pair_quotes() {
        assert_eq!(pair_for('"', "rust"), Some('"'));
        assert_eq!(pair_for('\'', "rust"), Some('\''));
        assert_eq!(pair_for('`', "rust"), Some('`'));
    }

    #[test]
    fn angle_only_in_markup() {
        assert_eq!(pair_for('<', "html"), Some('>'));
        assert_eq!(pair_for('<', "xml"), Some('>'));
        assert_eq!(pair_for('<', "markdown"), Some('>'));
        assert_eq!(pair_for('<', "css"), Some('>'));
        assert_eq!(pair_for('<', "rust"), None);
        assert_eq!(pair_for('<', "typescript"), None);
        assert_eq!(pair_for('<', "javascript"), None);
        assert_eq!(pair_for('<', ""), None);
    }

    #[test]
    fn unknown_open() {
        assert_eq!(pair_for('a', "rust"), None);
        assert_eq!(pair_for('!', "rust"), None);
    }

    #[test]
    fn close_for_each_open() {
        assert_eq!(close_for('('), Some(')'));
        assert_eq!(close_for('['), Some(']'));
        assert_eq!(close_for('{'), Some('}'));
        assert_eq!(close_for('"'), Some('"'));
        assert_eq!(close_for('\''), Some('\''));
        assert_eq!(close_for('`'), Some('`'));
        assert_eq!(close_for('<'), Some('>'));
        assert_eq!(close_for('a'), None);
    }

    #[test]
    fn close_chars_classified() {
        assert!(is_close_char(')'));
        assert!(is_close_char(']'));
        assert!(is_close_char('}'));
        assert!(is_close_char('>'));
        assert!(is_close_char('"'));
        assert!(is_close_char('\''));
        assert!(is_close_char('`'));
        assert!(!is_close_char('('));
        assert!(!is_close_char('a'));
    }

    #[test]
    fn open_chars_classified() {
        assert!(is_open_char('('));
        assert!(is_open_char('['));
        assert!(is_open_char('{'));
        assert!(is_open_char('<'));
        assert!(is_open_char('"'));
        assert!(is_open_char('\''));
        assert!(is_open_char('`'));
        assert!(!is_open_char(')'));
        assert!(!is_open_char('a'));
    }

    #[test]
    fn markup_lang_set() {
        assert!(is_markup_lang("html"));
        assert!(is_markup_lang("xml"));
        assert!(is_markup_lang("markdown"));
        assert!(is_markup_lang("css"));
        assert!(!is_markup_lang("rust"));
        assert!(!is_markup_lang("typescript"));
        assert!(!is_markup_lang(""));
    }

    #[test]
    fn quote_blocked_after_word_char() {
        assert!(!should_autoclose_quote_at(Some('a')));
        assert!(!should_autoclose_quote_at(Some('Z')));
        assert!(!should_autoclose_quote_at(Some('0')));
        assert!(!should_autoclose_quote_at(Some('_')));
    }

    #[test]
    fn quote_allowed_at_boundary() {
        assert!(should_autoclose_quote_at(None));
        assert!(should_autoclose_quote_at(Some(' ')));
        assert!(should_autoclose_quote_at(Some('(')));
        assert!(should_autoclose_quote_at(Some('=')));
        assert!(should_autoclose_quote_at(Some('\n')));
    }

    #[test]
    fn triple_quote_detection() {
        assert!(is_triple_quote_trigger((Some('"'), Some('"')), '"'));
        assert!(!is_triple_quote_trigger((Some('"'), None), '"'));
        assert!(!is_triple_quote_trigger((Some('a'), Some('"')), '"'));
        assert!(!is_triple_quote_trigger((Some('"'), Some('"')), '\''));
    }
}
