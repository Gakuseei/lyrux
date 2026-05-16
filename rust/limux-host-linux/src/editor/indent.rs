#![allow(dead_code)]

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;
use sourceview5::prelude::*;

pub fn install(view: &sourceview5::View, buffer: &sourceview5::Buffer) {
    let ctrl = gtk::EventControllerKey::new();
    ctrl.set_propagation_phase(gtk::PropagationPhase::Capture);
    let buffer = buffer.clone();
    ctrl.connect_key_pressed(move |_, key, _, mods| {
        if !is_plain_enter(key, mods) {
            return glib::Propagation::Proceed;
        }
        handle_enter(&buffer)
    });
    view.add_controller(ctrl);
}

fn is_plain_enter(key: gtk::gdk::Key, mods: gtk::gdk::ModifierType) -> bool {
    let busy = mods.intersects(
        gtk::gdk::ModifierType::CONTROL_MASK
            | gtk::gdk::ModifierType::ALT_MASK
            | gtk::gdk::ModifierType::SHIFT_MASK
            | gtk::gdk::ModifierType::META_MASK,
    );
    if busy {
        return false;
    }
    matches!(key, gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter)
}

fn handle_enter(buffer: &sourceview5::Buffer) -> glib::Propagation {
    if buffer.selection_bounds().is_some() {
        return glib::Propagation::Proceed;
    }

    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let line = cursor.line();
    let Some(line_start) = buffer.iter_at_line(line) else {
        return glib::Propagation::Proceed;
    };
    let mut line_end = line_start;
    if !line_end.ends_line() {
        line_end.forward_to_line_end();
    }
    let line_text = buffer.text(&line_start, &line_end, false).to_string();

    let lang_id = buffer
        .language()
        .map(|l| l.id().to_string())
        .unwrap_or_default();

    match analyze_line(&line_text, &lang_id) {
        ContinueAction::None => glib::Propagation::Proceed,
        ContinueAction::ContinuePrefix { indent, prefix } => {
            buffer.begin_user_action();
            buffer.insert_at_cursor(&format!("\n{indent}{prefix}"));
            buffer.end_user_action();
            glib::Propagation::Stop
        }
        ContinueAction::ContinueAutoIncrement { indent, n, suffix } => {
            buffer.begin_user_action();
            let next = n.saturating_add(1);
            buffer.insert_at_cursor(&format!("\n{indent}{next}{suffix}"));
            buffer.end_user_action();
            glib::Propagation::Stop
        }
        ContinueAction::ExitList => {
            let mut s = line_start;
            let mut e = line_end;
            buffer.begin_user_action();
            buffer.delete(&mut s, &mut e);
            buffer.insert_at_cursor("\n");
            buffer.end_user_action();
            glib::Propagation::Stop
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContinueAction {
    None,
    ContinuePrefix {
        indent: String,
        prefix: String,
    },
    ContinueAutoIncrement {
        indent: String,
        n: u32,
        suffix: String,
    },
    ExitList,
}

fn split_indent(line: &str) -> (&str, &str) {
    let end = line
        .char_indices()
        .find(|(_, c)| *c != ' ' && *c != '\t')
        .map(|(i, _)| i)
        .unwrap_or(line.len());
    (&line[..end], &line[end..])
}

fn is_line_comment_lang(lang: &str) -> bool {
    matches!(
        lang,
        "rust" | "c" | "cpp" | "chdr" | "typescript" | "javascript" | "go" | "css" | "lua" | "java"
    )
}

fn is_hash_comment_lang(lang: &str) -> bool {
    matches!(
        lang,
        "python" | "python3" | "sh" | "yaml" | "toml" | "ruby" | "conf"
    )
}

fn is_block_comment_lang(lang: &str) -> bool {
    matches!(
        lang,
        "rust" | "c" | "cpp" | "chdr" | "typescript" | "javascript" | "java"
    )
}

pub fn analyze_line(line_text: &str, lang_id: &str) -> ContinueAction {
    let (indent, rest) = split_indent(line_text);

    if lang_id == "markdown" {
        if let Some(action) = markdown_list_action(indent, rest) {
            return action;
        }
        return ContinueAction::None;
    }

    if is_block_comment_lang(lang_id) {
        if let Some(action) = block_comment_action(indent, rest) {
            return action;
        }
    }

    if let Some(action) = line_comment_action(indent, rest, lang_id) {
        return action;
    }

    if lang_id.is_empty() {
        if let Some(action) = line_comment_action(indent, rest, "rust") {
            return action;
        }
        if let Some(action) = line_comment_action(indent, rest, "python") {
            return action;
        }
    }

    ContinueAction::None
}

fn line_comment_action(indent: &str, rest: &str, lang_id: &str) -> Option<ContinueAction> {
    if is_line_comment_lang(lang_id) && rest.starts_with("//") {
        return Some(parse_simple_prefix(indent, rest, "//"));
    }
    if is_hash_comment_lang(lang_id) && rest.starts_with('#') {
        return Some(parse_simple_prefix(indent, rest, "#"));
    }
    None
}

fn parse_simple_prefix(indent: &str, rest: &str, marker: &str) -> ContinueAction {
    let after = &rest[marker.len()..];
    let trailing_space = after.starts_with(' ');
    let body: &str = if trailing_space { &after[1..] } else { after };
    if body.trim().is_empty() {
        return ContinueAction::ExitList;
    }
    let prefix = if trailing_space {
        format!("{marker} ")
    } else {
        marker.to_string()
    };
    ContinueAction::ContinuePrefix {
        indent: indent.to_string(),
        prefix,
    }
}

fn block_comment_action(indent: &str, rest: &str) -> Option<ContinueAction> {
    if let Some(after) = rest.strip_prefix("/**") {
        if after.trim().is_empty() || after.starts_with(' ') {
            return Some(ContinueAction::ContinuePrefix {
                indent: format!("{indent} "),
                prefix: "* ".to_string(),
            });
        }
    }
    if let Some(after) = rest.strip_prefix('*') {
        if after.starts_with('*') {
            return None;
        }
        let trailing_space = after.starts_with(' ');
        let body: &str = if let Some(s) = after.strip_prefix(' ') {
            s
        } else {
            after
        };
        if body.trim().is_empty() {
            return Some(ContinueAction::ExitList);
        }
        let prefix = if trailing_space {
            "* ".to_string()
        } else {
            "*".to_string()
        };
        return Some(ContinueAction::ContinuePrefix {
            indent: indent.to_string(),
            prefix,
        });
    }
    None
}

fn markdown_list_action(indent: &str, rest: &str) -> Option<ContinueAction> {
    if let Some(action) = numbered_list_action(indent, rest) {
        return Some(action);
    }
    if let Some(marker) = rest.chars().next() {
        if marker == '-' || marker == '*' || marker == '+' {
            let after = &rest[marker.len_utf8()..];
            let trailing_space = after.starts_with(' ');
            let body: &str = if trailing_space {
                after.trim_start()
            } else {
                after
            };
            if !trailing_space && !after.is_empty() {
                return None;
            }
            if body.trim().is_empty() {
                return Some(ContinueAction::ExitList);
            }
            return Some(ContinueAction::ContinuePrefix {
                indent: indent.to_string(),
                prefix: format!("{marker} "),
            });
        }
    }
    None
}

fn numbered_list_action(indent: &str, rest: &str) -> Option<ContinueAction> {
    let digit_end = rest
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    if digit_end == 0 {
        return None;
    }
    let digits = &rest[..digit_end];
    let after_digits = &rest[digit_end..];
    let suffix_char = after_digits.chars().next()?;
    if suffix_char != '.' && suffix_char != ')' {
        return None;
    }
    let after_suffix = &after_digits[suffix_char.len_utf8()..];
    if !after_suffix.starts_with(' ') && !after_suffix.is_empty() {
        return None;
    }
    let body = after_suffix.trim_start();
    let n: u32 = digits.parse().ok()?;
    if body.is_empty() {
        return Some(ContinueAction::ExitList);
    }
    Some(ContinueAction::ContinueAutoIncrement {
        indent: indent.to_string(),
        n,
        suffix: format!("{suffix_char} "),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_line_comment_continues() {
        assert_eq!(
            analyze_line("    // hi", "rust"),
            ContinueAction::ContinuePrefix {
                indent: "    ".to_string(),
                prefix: "// ".to_string(),
            }
        );
    }

    #[test]
    fn rust_empty_line_comment_exits() {
        assert_eq!(analyze_line("    // ", "rust"), ContinueAction::ExitList);
        assert_eq!(analyze_line("//", "rust"), ContinueAction::ExitList);
    }

    #[test]
    fn python_hash_continues() {
        assert_eq!(
            analyze_line("# x", "python"),
            ContinueAction::ContinuePrefix {
                indent: String::new(),
                prefix: "# ".to_string(),
            }
        );
    }

    #[test]
    fn python_empty_hash_exits() {
        assert_eq!(analyze_line("# ", "python"), ContinueAction::ExitList);
    }

    #[test]
    fn markdown_dash_continues() {
        assert_eq!(
            analyze_line("- foo", "markdown"),
            ContinueAction::ContinuePrefix {
                indent: String::new(),
                prefix: "- ".to_string(),
            }
        );
    }

    #[test]
    fn markdown_star_continues() {
        assert_eq!(
            analyze_line("* foo", "markdown"),
            ContinueAction::ContinuePrefix {
                indent: String::new(),
                prefix: "* ".to_string(),
            }
        );
    }

    #[test]
    fn markdown_numbered_dot_increments() {
        assert_eq!(
            analyze_line("1. foo", "markdown"),
            ContinueAction::ContinueAutoIncrement {
                indent: String::new(),
                n: 1,
                suffix: ". ".to_string(),
            }
        );
    }

    #[test]
    fn markdown_numbered_paren_increments_with_indent() {
        assert_eq!(
            analyze_line("   2) bar", "markdown"),
            ContinueAction::ContinueAutoIncrement {
                indent: "   ".to_string(),
                n: 2,
                suffix: ") ".to_string(),
            }
        );
    }

    #[test]
    fn markdown_empty_dash_exits() {
        assert_eq!(analyze_line("- ", "markdown"), ContinueAction::ExitList);
        assert_eq!(analyze_line("  - ", "markdown"), ContinueAction::ExitList);
    }

    #[test]
    fn markdown_empty_number_exits() {
        assert_eq!(analyze_line("1. ", "markdown"), ContinueAction::ExitList);
    }

    #[test]
    fn plain_text_no_action() {
        assert_eq!(analyze_line("hello", "rust"), ContinueAction::None);
        assert_eq!(analyze_line("    code()", "rust"), ContinueAction::None);
        assert_eq!(analyze_line("", "rust"), ContinueAction::None);
    }

    #[test]
    fn markdown_no_slash_comment() {
        assert_eq!(
            analyze_line("// not code", "markdown"),
            ContinueAction::None
        );
    }

    #[test]
    fn jsdoc_continuation() {
        assert_eq!(
            analyze_line(" * line", "rust"),
            ContinueAction::ContinuePrefix {
                indent: " ".to_string(),
                prefix: "* ".to_string(),
            }
        );
    }

    #[test]
    fn jsdoc_opening_transitions() {
        assert_eq!(
            analyze_line("/** doc", "rust"),
            ContinueAction::ContinuePrefix {
                indent: " ".to_string(),
                prefix: "* ".to_string(),
            }
        );
    }

    #[test]
    fn unknown_lang_falls_back_for_slash_comment() {
        assert_eq!(
            analyze_line("// hi", ""),
            ContinueAction::ContinuePrefix {
                indent: String::new(),
                prefix: "// ".to_string(),
            }
        );
    }
}
