#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gtk4::glib;
use gtk4::prelude::*;
use sourceview5::prelude::*;

const DEBOUNCE_MS: u64 = 80;
const MIN_WORD_LEN: usize = 2;

#[derive(Clone)]
pub struct HighlightController {
    enabled: Rc<Cell<bool>>,
    settings: sourceview5::SearchSettings,
    context: sourceview5::SearchContext,
}

impl HighlightController {
    pub fn set_enabled(&self, on: bool) {
        if self.enabled.get() == on {
            return;
        }
        self.enabled.set(on);
        if on {
            self.context.set_highlight(true);
        } else {
            self.context.set_highlight(false);
            self.settings.set_search_text(None);
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.get()
    }
}

pub fn install(buffer: &sourceview5::Buffer, enabled: bool) -> HighlightController {
    let settings = sourceview5::SearchSettings::builder()
        .case_sensitive(true)
        .at_word_boundaries(true)
        .regex_enabled(false)
        .wrap_around(true)
        .build();
    let context = sourceview5::SearchContext::new(buffer, Some(&settings));
    context.set_highlight(enabled);

    let controller = HighlightController {
        enabled: Rc::new(Cell::new(enabled)),
        settings: settings.clone(),
        context,
    };

    let debounce: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let enabled_flag = controller.enabled.clone();
    let buffer_weak = buffer.downgrade();
    let settings_weak = settings.downgrade();

    buffer.connect_cursor_position_notify(move |_| {
        if !enabled_flag.get() {
            return;
        }
        if let Some(id) = debounce.borrow_mut().take() {
            id.remove();
        }
        let buffer_weak_inner = buffer_weak.clone();
        let settings_weak_inner = settings_weak.clone();
        let debounce_inner = debounce.clone();
        let enabled_inner = enabled_flag.clone();
        let id = glib::timeout_add_local_once(Duration::from_millis(DEBOUNCE_MS), move || {
            *debounce_inner.borrow_mut() = None;
            if !enabled_inner.get() {
                return;
            }
            let (Some(buf), Some(set)) =
                (buffer_weak_inner.upgrade(), settings_weak_inner.upgrade())
            else {
                return;
            };
            match current_word_at_cursor(&buf) {
                Some(word) => set.set_search_text(Some(&word)),
                None => set.set_search_text(None),
            }
        });
        *debounce.borrow_mut() = Some(id);
    });

    controller
}

fn current_word_at_cursor(buffer: &sourceview5::Buffer) -> Option<String> {
    if buffer.has_selection() {
        return None;
    }
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let mut start = cursor;
    while start.offset() > 0 {
        let mut probe = start;
        if !probe.backward_char() {
            break;
        }
        if !is_word_char(probe.char()) {
            break;
        }
        start = probe;
    }
    let mut end = cursor;
    while !end.is_end() && is_word_char(end.char()) && end.forward_char() {}
    if start.offset() == end.offset() {
        return None;
    }
    let word = buffer.text(&start, &end, false).to_string();
    if word.chars().count() < MIN_WORD_LEN {
        return None;
    }
    if word.chars().any(|c| !is_word_char(c)) {
        return None;
    }
    Some(word)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_char_classification() {
        assert!(is_word_char('a'));
        assert!(is_word_char('Z'));
        assert!(is_word_char('0'));
        assert!(is_word_char('_'));
        assert!(is_word_char('ä'));
        assert!(!is_word_char(' '));
        assert!(!is_word_char('-'));
        assert!(!is_word_char('.'));
        assert!(!is_word_char('('));
        assert!(!is_word_char('\n'));
    }
}
