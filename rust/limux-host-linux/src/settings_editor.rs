use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk4 as gtk;
use gtk4::glib;
use libadwaita as adw;

use crate::app_config::{AppConfig, ColorScheme, EditorSettings};
use crate::editor::settings_panel as editor_settings_panel;
use crate::editor::view as editor_view;
use crate::keybind_editor;
use crate::pane;
use crate::shortcut_config::{NormalizedShortcut, ResolvedShortcutConfig, ShortcutId};

const EDITOR_BROADCAST_DEBOUNCE_MS: u64 = 50;

thread_local! {
    static EDITOR_BROADCAST_DEBOUNCE: RefCell<Option<glib::SourceId>> =
        const { RefCell::new(None) };
}

pub const SETTINGS_CSS: &str = r#"
.limux-settings-window {
    background-color: @window_bg_color;
    color: @window_fg_color;
}

/* Scoped accent override — kills libadwaita default orange inside Settings only */
.limux-settings-window viewswitcher button:checked,
.limux-settings-window viewswitcher togglebutton:checked {
    box-shadow: inset 0 -3px 0 @blue_3;
    color: @window_fg_color;
}
.limux-settings-window viewswitcher button:checked:hover,
.limux-settings-window viewswitcher togglebutton:checked:hover {
    box-shadow: inset 0 -3px 0 @blue_4;
}

.limux-settings-window switch:checked {
    background-color: @blue_3;
}
.limux-settings-window switch:checked:disabled {
    background-color: alpha(@blue_3, 0.5);
}
.limux-settings-window switch:checked > slider {
    background-color: #ffffff;
}

.limux-settings-window dropdown > popover listview > row:selected,
.limux-settings-window dropdown > popover listview > row:selected:hover,
.limux-settings-window popover.menu listview > row:selected {
    background-color: alpha(@blue_3, 0.18);
    color: @window_fg_color;
}

.limux-settings-window button.suggested-action {
    background-color: @blue_3;
    color: #ffffff;
}
.limux-settings-window button.suggested-action:hover {
    background-color: @blue_4;
}
.limux-settings-window button.suggested-action:active {
    background-color: @blue_5;
}

.limux-settings-window entry:focus-within,
.limux-settings-window entry:focus {
    outline-color: @blue_3;
}

.limux-settings-window checkbutton:checked > check,
.limux-settings-window check:checked,
.limux-settings-window radiobutton:checked > radio {
    background-color: @blue_3;
    border-color: @blue_3;
}

.limux-settings-window row:selected {
    background-color: alpha(@blue_3, 0.18);
    color: @window_fg_color;
}

.limux-settings-window progressbar > trough > progress,
.limux-settings-window scale > trough > highlight {
    background-color: @blue_3;
}

.limux-settings-window spinner {
    color: @blue_3;
}

.limux-settings-window :focus-visible,
.limux-settings-window :focus:focus-visible {
    outline-color: @blue_3;
}

/* Kill libadwaita boxed-list bubble backgrounds — flat list look */
.limux-settings-window listbox.boxed-list,
.limux-settings-window listbox.boxed-list-separate,
.limux-settings-window preferencesgroup listbox,
.limux-settings-window preferencesgroup list,
.limux-settings-window preferencesgroup listbox > row,
.limux-settings-window preferencesgroup list > row,
.limux-settings-window preferencesgroup row.activatable,
.limux-settings-window preferencesgroup row {
    background-color: transparent;
    background-image: none;
    background: none;
    border: none;
    box-shadow: none;
    border-radius: 0;
}

/* Hover state — subtle, no red bubble */
.limux-settings-window preferencesgroup row.activatable:hover {
    background-color: alpha(@window_fg_color, 0.04);
    border-radius: 8px;
}

/* Thin separators between rows for visual hierarchy */
.limux-settings-window preferencesgroup listbox > row + row,
.limux-settings-window preferencesgroup list > row + row {
    border-top: 1px solid alpha(@window_fg_color, 0.06);
}

/* Stronger group headers */
.limux-settings-window preferencesgroup > box > label.heading,
.limux-settings-window preferencesgroup label.heading {
    font-weight: 700;
    font-size: 1.05em;
    margin-bottom: 6px;
    margin-top: 4px;
}

/* Breathing room between groups */
.limux-settings-window preferencesgroup {
    margin-bottom: 16px;
}

.limux-settings-window preferencespage > scrolledwindow > viewport > clamp > box {
    margin-top: 12px;
    margin-bottom: 12px;
}
"#;

type OnConfigChanged = dyn Fn(&AppConfig, &AppConfig);

pub struct SettingsEditorInput {
    pub config: Rc<RefCell<AppConfig>>,
    pub shortcuts: Rc<ResolvedShortcutConfig>,
    pub on_capture: Rc<
        dyn Fn(ShortcutId, Option<NormalizedShortcut>) -> Result<ResolvedShortcutConfig, String>,
    >,
    pub on_config_changed: Rc<OnConfigChanged>,
}

pub fn present_settings_dialog(parent: &impl IsA<gtk::Widget>, input: SettingsEditorInput) {
    let window = adw::Window::new();
    window.set_title(Some("Settings"));
    window.set_default_size(760, 680);
    window.set_modal(true);

    if let Some(parent_window) = parent
        .root()
        .and_then(|root| root.downcast::<gtk::Window>().ok())
    {
        window.set_transient_for(Some(&parent_window));
        if let Some(app) = parent_window.application() {
            window.set_application(Some(&app));
        }
    }

    let content = build_settings_window_content(&window, input);
    window.set_content(Some(&content));
    window.present();
}

fn apply_config_change<F, G>(config: &Rc<RefCell<AppConfig>>, on_changed: &F, update: G)
where
    F: Fn(&AppConfig, &AppConfig) + ?Sized,
    G: FnOnce(&mut AppConfig),
{
    let (previous, updated) = {
        let mut config_ref = config.borrow_mut();
        let previous = config_ref.clone();
        update(&mut config_ref);
        let updated = config_ref.clone();
        (previous, updated)
    };
    on_changed(&previous, &updated);
}

fn build_settings_window_content(window: &adw::Window, input: SettingsEditorInput) -> gtk::Widget {
    let toast_overlay = adw::ToastOverlay::new();

    let stack = adw::ViewStack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    let general_page = build_general_page(&input);
    let general_stack_page = stack.add_titled(&general_page, Some("general"), "General");
    general_stack_page.set_icon_name(Some("preferences-system-symbolic"));

    let editor_page = build_editor_page(&input);
    let editor_stack_page = stack.add_titled(&editor_page, Some("editor"), "Editor");
    editor_stack_page.set_icon_name(Some("accessories-text-editor-symbolic"));

    let keybinds_page = keybind_editor::build_keybind_editor(&input.shortcuts, input.on_capture);
    let keybinds_stack_page = stack.add_titled(&keybinds_page, Some("keybindings"), "Keybindings");
    keybinds_stack_page.set_icon_name(Some("input-keyboard-symbolic"));

    let switcher = adw::ViewSwitcher::builder()
        .stack(&stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .build();

    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text("Close settings")
        .valign(gtk::Align::Center)
        .build();
    close_button.add_css_class("flat");

    {
        let window = window.clone();
        close_button.connect_clicked(move |_| {
            window.close();
        });
    }

    let header_bar = adw::HeaderBar::new();
    header_bar.set_show_start_title_buttons(false);
    header_bar.set_show_end_title_buttons(false);
    header_bar.set_title_widget(Some(&switcher));
    header_bar.pack_end(&close_button);

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.add_css_class("limux-settings-window");
    outer.append(&header_bar);
    outer.append(&stack);

    toast_overlay.set_child(Some(&outer));
    toast_overlay.upcast()
}

fn build_general_page(input: &SettingsEditorInput) -> gtk::Widget {
    let page = adw::PreferencesPage::new();
    page.set_title("General");
    page.set_name(Some("general"));
    page.set_icon_name(Some("preferences-system-symbolic"));
    page.set_hexpand(true);
    page.set_vexpand(true);

    let group = adw::PreferencesGroup::new();

    let color_row = adw::ActionRow::builder()
        .title("GTK color scheme")
        .subtitle("Choose whether the GTK interface follows system, dark, or light")
        .icon_name("applications-graphics-symbolic")
        .build();
    color_row.set_title_lines(1);
    color_row.set_subtitle_lines(2);
    let color_dropdown = gtk::DropDown::from_strings(&["System", "Dark", "Light"]);
    let initial_scheme = input.config.borrow().appearance.color_scheme;
    color_dropdown.set_selected(match initial_scheme {
        ColorScheme::System => 0,
        ColorScheme::Dark => 1,
        ColorScheme::Light => 2,
    });
    color_dropdown.set_valign(gtk::Align::Center);
    color_row.add_suffix(&color_dropdown);
    color_row.set_activatable_widget(Some(&color_dropdown));
    group.add(&color_row);

    let ghostty_row = adw::ActionRow::builder()
        .title("Ghostty color scheme")
        .subtitle("Choose whether terminal surfaces follow system, dark, or light")
        .icon_name("utilities-terminal-symbolic")
        .build();
    ghostty_row.set_title_lines(1);
    ghostty_row.set_subtitle_lines(2);
    let ghostty_dropdown = gtk::DropDown::from_strings(&["System", "Dark", "Light"]);
    let initial_ghostty_scheme = input.config.borrow().appearance.ghostty_color_scheme;
    ghostty_dropdown.set_selected(match initial_ghostty_scheme {
        ColorScheme::System => 0,
        ColorScheme::Dark => 1,
        ColorScheme::Light => 2,
    });
    ghostty_dropdown.set_valign(gtk::Align::Center);
    ghostty_row.add_suffix(&ghostty_dropdown);
    ghostty_row.set_activatable_widget(Some(&ghostty_dropdown));
    group.add(&ghostty_row);

    let hover_row = adw::ActionRow::builder()
        .title("Hover terminal focus")
        .subtitle("Focus terminal panes when the mouse pointer enters them")
        .icon_name("input-mouse-symbolic")
        .build();
    hover_row.set_title_lines(1);
    hover_row.set_subtitle_lines(2);
    let hover_switch = gtk::Switch::new();
    hover_switch.set_active(input.config.borrow().focus.hover_terminal_focus);
    hover_switch.set_valign(gtk::Align::Center);
    hover_row.add_suffix(&hover_switch);
    hover_row.set_activatable_widget(Some(&hover_switch));
    group.add(&hover_row);

    page.add(&group);

    {
        let config = input.config.clone();
        let on_changed = input.on_config_changed.clone();
        color_dropdown.connect_selected_notify(move |dropdown| {
            let scheme = match dropdown.selected() {
                1 => ColorScheme::Dark,
                2 => ColorScheme::Light,
                _ => ColorScheme::System,
            };
            apply_config_change(&config, &*on_changed, move |c| {
                c.appearance.color_scheme = scheme;
            });
        });
    }
    {
        let config = input.config.clone();
        let on_changed = input.on_config_changed.clone();
        ghostty_dropdown.connect_selected_notify(move |dropdown| {
            let scheme = match dropdown.selected() {
                1 => ColorScheme::Dark,
                2 => ColorScheme::Light,
                _ => ColorScheme::System,
            };
            apply_config_change(&config, &*on_changed, move |c| {
                c.appearance.ghostty_color_scheme = scheme;
            });
        });
    }
    {
        let config = input.config.clone();
        let on_changed = input.on_config_changed.clone();
        hover_switch.connect_active_notify(move |switch| {
            let hover_terminal_focus = switch.is_active();
            apply_config_change(&config, &*on_changed, move |c| {
                c.focus.hover_terminal_focus = hover_terminal_focus;
            });
        });
    }

    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&page)
        .build();
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);

    scroller.upcast()
}

fn build_editor_page(input: &SettingsEditorInput) -> gtk::Widget {
    let current = input.config.borrow().editor.clone();
    let config = input.config.clone();
    let on_changed = input.on_config_changed.clone();
    let cb: Rc<dyn Fn(&EditorSettings)> = Rc::new(move |next: &EditorSettings| {
        let next_clone = next.clone();
        apply_config_change(&config, &*on_changed, move |c| {
            c.editor = next_clone;
        });
        broadcast_editor_settings(next);
    });
    editor_settings_panel::build(
        &current,
        editor_settings_panel::SettingsCallbacks { on_change: cb },
    )
}

pub fn broadcast_editor_settings(settings: &EditorSettings) {
    let pending = settings.clone();
    EDITOR_BROADCAST_DEBOUNCE.with(|slot| {
        if let Some(id) = slot.borrow_mut().take() {
            id.remove();
        }
        let source_id = glib::timeout_add_local_once(
            Duration::from_millis(EDITOR_BROADCAST_DEBOUNCE_MS),
            move || {
                EDITOR_BROADCAST_DEBOUNCE.with(|slot| slot.borrow_mut().take());
                let system_prefers_dark = crate::window::current_system_prefers_dark();
                apply_editor_settings_now(&pending, system_prefers_dark);
            },
        );
        *slot.borrow_mut() = Some(source_id);
    });
}

pub fn reapply_editor_settings_for_system_pref(
    settings: &EditorSettings,
    system_prefers_dark: Option<bool>,
) {
    apply_editor_settings_now(settings, system_prefers_dark);
}

fn apply_editor_settings_now(settings: &EditorSettings, system_prefers_dark: Option<bool>) {
    let view_cfg = settings.to_view_config_with_system_pref(system_prefers_dark);
    pane::for_each_editor_tab(|state| {
        editor_view::apply_to_view(&state.view, &view_cfg);
        editor_view::apply_to_buffer(&state.buffer, &view_cfg);
        editor_view::apply_css(&state.view, &view_cfg, &state.css_provider);
        state
            .highlight
            .set_enabled(view_cfg.highlight_word_at_cursor);
        state.sticky.set_enabled(view_cfg.show_sticky_scroll);
        state.minimap.set_visible(view_cfg.show_minimap);
        state
            .wrap_button
            .set_label(crate::editor::status_bar::wrap_label_text(
                view_cfg.wrap_lines,
            ));
    });
    crate::window::apply_file_panel_columns(settings.fp_show_size, settings.fp_show_mtime);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_config_change_allows_reentrant_config_sync() {
        let config = Rc::new(RefCell::new(AppConfig::default()));

        apply_config_change(
            &config,
            &|_previous, updated| {
                config.borrow_mut().clone_from(updated);
            },
            |current| {
                current.focus.hover_terminal_focus = true;
            },
        );

        assert!(config.borrow().focus.hover_terminal_focus);
    }
}
