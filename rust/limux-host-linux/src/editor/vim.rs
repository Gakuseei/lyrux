#![allow(dead_code)]

use std::rc::Rc;

use gtk4::prelude::*;

use crate::editor::strings;
use crate::editor::tab_state::EditorTabState;

pub fn apply_to_tab(state: &EditorTabState, enabled: bool) {
    if enabled {
        if state.vim_im_context.borrow().is_some() {
            return;
        }
        attach(state);
    } else {
        detach(state);
    }
}

fn attach(state: &EditorTabState) {
    let im = sourceview5::VimIMContext::new();
    let key = gtk4::EventControllerKey::new();
    key.set_im_context(Some(&im));
    key.set_propagation_phase(gtk4::PropagationPhase::Capture);
    state.view.add_controller(key.clone());
    im.set_client_widget(Some(&state.view));

    let label = state.vim_label.clone();
    label.set_text(strings::STATUS_VIM_NORMAL);
    label.set_visible(true);

    let label_for_notify = label.clone();
    im.connect_command_bar_text_notify(move |ctx| {
        let txt = ctx.command_bar_text();
        let s = txt.as_str();
        if s.is_empty() {
            label_for_notify.set_text(strings::STATUS_VIM_NORMAL);
        } else {
            label_for_notify.set_text(s);
        }
    });

    let save_action = state.save_action.clone();
    let close_action = state.close_action.clone();
    im.connect_execute_command(move |_, command| {
        handle_command(command, &save_action, &close_action)
    });

    *state.vim_im_context.borrow_mut() = Some(im);
    *state.vim_key_controller.borrow_mut() = Some(key);
}

fn detach(state: &EditorTabState) {
    if let Some(key) = state.vim_key_controller.borrow_mut().take() {
        state.view.remove_controller(&key);
    }
    if let Some(im) = state.vim_im_context.borrow_mut().take() {
        im.set_client_widget(None::<&gtk4::Widget>);
    }
    state.vim_label.set_text("");
    state.vim_label.set_visible(false);
}

type ActionCb = Rc<std::cell::RefCell<Option<Rc<dyn Fn()>>>>;

fn handle_command(command: &str, save: &ActionCb, close: &ActionCb) -> bool {
    let cmd = command.trim().trim_start_matches(':');
    match cmd {
        "w" | "write" => {
            invoke(save);
            true
        }
        "q" | "quit" => {
            invoke(close);
            true
        }
        "wq" | "x" | "wq!" => {
            invoke(save);
            invoke(close);
            true
        }
        _ => false,
    }
}

fn invoke(slot: &ActionCb) {
    let action = slot.borrow().clone();
    if let Some(cb) = action {
        cb();
    }
}
