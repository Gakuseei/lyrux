use std::cell::RefCell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;

use crate::editor::strings;

const POPOVER_WIDTH: i32 = 520;
const POPOVER_HEIGHT: i32 = 380;
const MAX_RESULTS: usize = 60;

#[derive(Clone, Copy)]
struct CommandEntry {
    label: &'static str,
    action_name: &'static str,
    accel: Option<&'static str>,
}

const COMMANDS: &[CommandEntry] = &[
    CommandEntry {
        label: strings::CMD_NEW_WORKSPACE,
        action_name: "win.new-workspace",
        accel: Some("Ctrl+Shift+N"),
    },
    CommandEntry {
        label: strings::CMD_CLOSE_WORKSPACE,
        action_name: "win.close-workspace",
        accel: Some("Ctrl+Shift+W"),
    },
    CommandEntry {
        label: strings::CMD_NEXT_WORKSPACE,
        action_name: "win.next-workspace",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_PREV_WORKSPACE,
        action_name: "win.prev-workspace",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_SPLIT_RIGHT,
        action_name: "win.split-right",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_SPLIT_DOWN,
        action_name: "win.split-down",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_CLOSE_FOCUSED_PANE,
        action_name: "win.close-focused-pane",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_FOCUS_LEFT,
        action_name: "win.focus-left",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_FOCUS_RIGHT,
        action_name: "win.focus-right",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_FOCUS_UP,
        action_name: "win.focus-up",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_FOCUS_DOWN,
        action_name: "win.focus-down",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_CYCLE_TAB_NEXT,
        action_name: "win.cycle-tab-next",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_CYCLE_TAB_PREV,
        action_name: "win.cycle-tab-prev",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_TOGGLE_SIDEBAR,
        action_name: "win.toggle-sidebar",
        accel: Some("Ctrl+M"),
    },
    CommandEntry {
        label: strings::CMD_TOGGLE_FILE_PANEL,
        action_name: "win.toggle-file-panel",
        accel: Some("Ctrl+B"),
    },
    CommandEntry {
        label: strings::CMD_TOGGLE_TOP_BAR,
        action_name: "win.toggle-top-bar",
        accel: Some("Ctrl+Shift+M"),
    },
    CommandEntry {
        label: strings::CMD_TOGGLE_FULLSCREEN,
        action_name: "win.toggle-fullscreen",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_EDITOR_TOGGLE_CURRENT_PANE,
        action_name: "win.editor-toggle-current-pane",
        accel: Some("Ctrl+Shift+E"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_QUICK_OPEN,
        action_name: "win.editor-quick-open",
        accel: Some("Ctrl+P"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_SAVE,
        action_name: "win.editor-save-active",
        accel: Some("Ctrl+S"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_FIND,
        action_name: "win.editor-find",
        accel: Some("Ctrl+F"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_REPLACE,
        action_name: "win.editor-replace",
        accel: Some("Ctrl+H"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_FIND_NEXT,
        action_name: "win.editor-find-next",
        accel: Some("Ctrl+G"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_GOTO_LINE,
        action_name: "win.editor-goto-line",
        accel: Some("Ctrl+L"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_TOGGLE_COMMENT,
        action_name: "win.editor-toggle-comment",
        accel: Some("Ctrl+/"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_DUPLICATE_LINE,
        action_name: "win.editor-duplicate-line",
        accel: Some("Ctrl+Shift+D"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_DELETE_LINE,
        action_name: "win.editor-delete-line",
        accel: Some("Ctrl+Shift+K"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_MOVE_LINE_UP,
        action_name: "win.editor-move-line-up",
        accel: Some("Alt+Up"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_MOVE_LINE_DOWN,
        action_name: "win.editor-move-line-down",
        accel: Some("Alt+Down"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_SELECT_NEXT_OCCURRENCE,
        action_name: "win.editor-select-next-occurrence",
        accel: Some("Ctrl+D"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_TOGGLE_WRAP,
        action_name: "win.editor-toggle-wrap",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_PANE_TOGGLE_PIN,
        action_name: "win.pane-toggle-pin-tab",
        accel: Some("Ctrl+Alt+P"),
    },
    CommandEntry {
        label: strings::CMD_EDITOR_REOPEN_CLOSED,
        action_name: "win.editor-reopen-closed-tab",
        accel: Some("Ctrl+Alt+T"),
    },
    CommandEntry {
        label: strings::CMD_NEW_TERMINAL,
        action_name: "win.new-terminal",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_NEW_TERMINAL_IN_PANE,
        action_name: "win.new-terminal-in-focused-pane",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_OPEN_BROWSER_IN_SPLIT,
        action_name: "win.open-browser-in-split",
        accel: None,
    },
    CommandEntry {
        label: strings::CMD_QUIT_APP,
        action_name: "app.quit",
        accel: Some("Ctrl+Q"),
    },
];

pub fn show(parent_widget: &gtk::Widget) {
    let parent: gtk::Widget = parent_widget.clone();

    let popover = gtk::Popover::builder()
        .has_arrow(false)
        .position(gtk::PositionType::Bottom)
        .autohide(true)
        .build();
    popover.add_css_class("lyrux-command-palette");
    popover.set_parent(&parent);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 6);
    vbox.set_margin_top(8);
    vbox.set_margin_bottom(8);
    vbox.set_margin_start(8);
    vbox.set_margin_end(8);
    vbox.set_size_request(POPOVER_WIDTH, POPOVER_HEIGHT);

    let title = gtk::Label::builder()
        .label(strings::CMD_PALETTE_TITLE)
        .xalign(0.0)
        .build();
    title.add_css_class("dim-label");
    vbox.append(&title);

    let entry = gtk::SearchEntry::builder()
        .placeholder_text(strings::CMD_PALETTE_PLACEHOLDER)
        .build();
    vbox.append(&entry);

    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .build();
    let list_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    list_box.add_css_class("lyrux-command-palette-list");
    scroller.set_child(Some(&list_box));
    vbox.append(&scroller);

    let empty_label = gtk::Label::builder()
        .label(strings::CMD_PALETTE_EMPTY)
        .xalign(0.5)
        .build();
    empty_label.add_css_class("dim-label");
    empty_label.set_visible(false);
    vbox.append(&empty_label);

    popover.set_child(Some(&vbox));

    let selected: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
    let visible_actions: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));

    let render = {
        let list_box = list_box.clone();
        let empty_label = empty_label.clone();
        let selected = selected.clone();
        let visible_actions = visible_actions.clone();
        let popover = popover.clone();
        let parent = parent.clone();
        Rc::new(move |query: &str| {
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }
            visible_actions.borrow_mut().clear();
            *selected.borrow_mut() = 0;

            let filtered = filter_commands(query);
            if filtered.is_empty() {
                empty_label.set_visible(true);
                return;
            }
            empty_label.set_visible(false);

            for (idx, cmd) in filtered.iter().enumerate() {
                let row = build_row(cmd, idx == 0);
                let action_name = cmd.action_name;
                let popover_clone = popover.clone();
                let parent_clone = parent.clone();
                row.connect_clicked(move |_| {
                    activate(&popover_clone, &parent_clone, action_name);
                });
                list_box.append(&row);
                visible_actions.borrow_mut().push(cmd.action_name);
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
        let parent_clone = parent.clone();
        let visible_actions = visible_actions.clone();
        let selected = selected.clone();
        entry.connect_activate(move |_| {
            let idx = *selected.borrow();
            let actions = visible_actions.borrow();
            if let Some(name) = actions.get(idx) {
                activate(&popover_clone, &parent_clone, name);
            } else {
                popover_clone.popdown();
            }
        });
    }

    let key_ctrl = gtk::EventControllerKey::new();
    key_ctrl.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let popover_clone = popover.clone();
        let parent_clone = parent.clone();
        let visible_actions = visible_actions.clone();
        let selected = selected.clone();
        let list_box = list_box.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            use gtk::gdk::Key;
            match key {
                Key::Escape => {
                    popover_clone.popdown();
                    glib::Propagation::Stop
                }
                Key::Down => {
                    move_selection(&selected, &visible_actions, &list_box, 1);
                    glib::Propagation::Stop
                }
                Key::Up => {
                    move_selection(&selected, &visible_actions, &list_box, -1);
                    glib::Propagation::Stop
                }
                Key::Return | Key::ISO_Enter | Key::KP_Enter => {
                    let idx = *selected.borrow();
                    let actions = visible_actions.borrow();
                    if let Some(name) = actions.get(idx) {
                        activate(&popover_clone, &parent_clone, name);
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

fn build_row(cmd: &CommandEntry, is_selected: bool) -> gtk::Button {
    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row_box.set_margin_top(4);
    row_box.set_margin_bottom(4);
    row_box.set_margin_start(8);
    row_box.set_margin_end(8);

    let label = gtk::Label::builder()
        .label(cmd.label)
        .xalign(0.0)
        .hexpand(true)
        .build();
    label.add_css_class("body");
    row_box.append(&label);

    if let Some(accel) = cmd.accel {
        let accel_label = gtk::Label::builder().label(accel).xalign(1.0).build();
        accel_label.add_css_class("dim-label");
        accel_label.add_css_class("caption");
        row_box.append(&accel_label);
    }

    let btn = gtk::Button::builder()
        .child(&row_box)
        .has_frame(false)
        .build();
    btn.add_css_class("lyrux-command-palette-row");
    btn.set_halign(gtk::Align::Fill);
    if is_selected {
        btn.add_css_class("suggested-action");
    }
    btn
}

fn move_selection(
    selected: &Rc<RefCell<usize>>,
    visible_actions: &Rc<RefCell<Vec<&'static str>>>,
    list_box: &gtk::Box,
    delta: i32,
) {
    let len = visible_actions.borrow().len();
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

fn activate(popover: &gtk::Popover, parent: &gtk::Widget, action_name: &str) {
    popover.popdown();
    if let Err(err) = parent.activate_action(action_name, None) {
        eprintln!("lyrux: command palette failed to activate `{action_name}`: {err}");
    }
}

fn filter_commands(query: &str) -> Vec<CommandEntry> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return COMMANDS.iter().copied().take(MAX_RESULTS).collect();
    }
    let needle = trimmed.to_ascii_lowercase();
    let mut matches: Vec<CommandEntry> = Vec::new();
    for cmd in COMMANDS {
        if cmd.label.to_ascii_lowercase().contains(&needle) {
            matches.push(*cmd);
        }
    }
    matches.truncate(MAX_RESULTS);
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_returns_all_commands() {
        let result = filter_commands("");
        assert_eq!(result.len(), COMMANDS.len());
    }

    #[test]
    fn substring_filter_matches_case_insensitive() {
        let result = filter_commands("WORKSPACE");
        assert!(result.iter().any(|c| c.label.contains("Workspace")));
        assert!(!result.is_empty());
    }

    #[test]
    fn unmatched_query_returns_empty() {
        let result = filter_commands("zzzzzzzzz-no-such-command");
        assert!(result.is_empty());
    }

    #[test]
    fn every_command_label_non_empty() {
        for cmd in COMMANDS {
            assert!(!cmd.label.is_empty(), "label must be non-empty");
            assert!(!cmd.action_name.is_empty(), "action_name must be non-empty");
            assert!(
                cmd.action_name.starts_with("win.") || cmd.action_name.starts_with("app."),
                "action_name `{}` must be win.* or app.*",
                cmd.action_name
            );
        }
    }
}
