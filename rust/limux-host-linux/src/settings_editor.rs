use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use crate::app_config::{AppConfig, ColorScheme};
use crate::keybind_editor;
use crate::shortcut_config::{NormalizedShortcut, ResolvedShortcutConfig, ShortcutId};
use crate::update_checker::{self, UpdateStatus};

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

#[derive(Clone, Debug, Default)]
struct UpdatePanelState {
    phase: UpdatePanelPhase,
    last_summary: Option<update_checker::UpdateSummary>,
    prepared_update: Option<update_checker::PreparedUpdate>,
    last_error: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum UpdatePanelPhase {
    #[default]
    Idle,
    Checking,
    Preparing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UpdateButtonAction {
    Check,
    PrepareInstall,
    ApplyPrepared,
    OpenReleases,
}

pub struct SettingsEditorInput {
    pub config: Rc<RefCell<AppConfig>>,
    pub shortcuts: Rc<ResolvedShortcutConfig>,
    pub on_capture: Rc<
        dyn Fn(ShortcutId, Option<NormalizedShortcut>) -> Result<ResolvedShortcutConfig, String>,
    >,
    pub on_config_changed: Rc<OnConfigChanged>,
    #[allow(clippy::type_complexity)]
    pub on_apply_update: Rc<dyn Fn(&Path) -> Result<(), String>>,
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

    let general_page = build_general_page(&input, &toast_overlay);
    let general_stack_page = stack.add_titled(&general_page, Some("general"), "General");
    general_stack_page.set_icon_name(Some("preferences-system-symbolic"));

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

fn build_general_page(
    input: &SettingsEditorInput,
    toast_overlay: &adw::ToastOverlay,
) -> gtk::Widget {
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

    let updates_group = build_updates_group(input, toast_overlay);

    page.add(&group);
    page.add(&updates_group);

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

fn build_updates_group(
    input: &SettingsEditorInput,
    toast_overlay: &adw::ToastOverlay,
) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title("Updates")
        .description("Check, download, and install the latest Limux release from GitHub.")
        .build();

    let current_version_row = adw::ActionRow::builder()
        .title("Current version")
        .subtitle(format!("v{}", crate::VERSION))
        .icon_name("package-x-generic-symbolic")
        .build();
    current_version_row.set_title_lines(1);
    current_version_row.set_subtitle_lines(1);
    group.add(&current_version_row);

    let latest_release_row = adw::ActionRow::builder()
        .title("Latest release")
        .subtitle("Not checked yet")
        .icon_name("folder-download-symbolic")
        .build();
    latest_release_row.set_title_lines(1);
    latest_release_row.set_subtitle_lines(2);
    group.add(&latest_release_row);

    let status_row = adw::ActionRow::builder()
        .title("Update status")
        .subtitle("Check GitHub releases to see whether an update is available.")
        .icon_name("dialog-information-symbolic")
        .build();
    status_row.set_title_lines(1);
    status_row.set_subtitle_lines(3);
    group.add(&status_row);

    let action_row = adw::ActionRow::builder()
        .title("Update action")
        .subtitle("Check for new releases or open the latest release page.")
        .icon_name("software-update-available-symbolic")
        .build();
    action_row.set_title_lines(1);
    action_row.set_subtitle_lines(2);

    let action_button = gtk::Button::with_label("Check for updates");
    action_button.set_valign(gtk::Align::Center);
    action_row.add_suffix(&action_button);
    action_row.set_activatable_widget(Some(&action_button));
    group.add(&action_row);

    let state = Rc::new(RefCell::new(UpdatePanelState::default()));
    render_update_panel(
        &state.borrow(),
        &latest_release_row,
        &status_row,
        &action_row,
        &action_button,
    );

    {
        let state = state.clone();
        let on_apply_update = input.on_apply_update.clone();
        let latest_release_row = latest_release_row.clone();
        let status_row = status_row.clone();
        let action_row = action_row.clone();
        let action_button = action_button.clone();
        let toast_overlay = toast_overlay.clone();
        action_button.clone().connect_clicked(move |_| {
            let action = update_button_action(&state.borrow());
            match action {
                UpdateButtonAction::Check => start_update_check(
                    state.clone(),
                    latest_release_row.clone(),
                    status_row.clone(),
                    action_row.clone(),
                    action_button.clone(),
                    toast_overlay.clone(),
                ),
                UpdateButtonAction::PrepareInstall => {
                    let summary = state.borrow().last_summary.clone();
                    start_prepare_update(
                        state.clone(),
                        summary,
                        latest_release_row.clone(),
                        status_row.clone(),
                        action_row.clone(),
                        action_button.clone(),
                        toast_overlay.clone(),
                    );
                }
                UpdateButtonAction::ApplyPrepared => {
                    let manifest_path = state
                        .borrow()
                        .prepared_update
                        .as_ref()
                        .map(|prepared| prepared.manifest_path.clone());
                    if let Some(manifest_path) = manifest_path {
                        if let Err(err) = on_apply_update(&manifest_path) {
                            push_toast(&toast_overlay, &format!("Update failed: {err}"));
                            let mut panel_state = state.borrow_mut();
                            panel_state.last_error = Some(err);
                            panel_state.prepared_update = None;
                            panel_state.phase = UpdatePanelPhase::Idle;
                            render_update_panel(
                                &panel_state,
                                &latest_release_row,
                                &status_row,
                                &action_row,
                                &action_button,
                            );
                        }
                    }
                }
                UpdateButtonAction::OpenReleases => {
                    let release_url = state
                        .borrow()
                        .last_summary
                        .as_ref()
                        .map(|summary| summary.release_url.clone())
                        .unwrap_or_else(|| update_checker::RELEASES_URL.to_string());
                    if let Err(err) = open_url(&release_url) {
                        push_toast(&toast_overlay, &format!("Could not open browser: {err}"));
                        let mut panel_state = state.borrow_mut();
                        panel_state.last_error = Some(err);
                        render_update_panel(
                            &panel_state,
                            &latest_release_row,
                            &status_row,
                            &action_row,
                            &action_button,
                        );
                    }
                }
            }
        });
    }

    group
}

fn render_update_panel(
    state: &UpdatePanelState,
    latest_release_row: &adw::ActionRow,
    status_row: &adw::ActionRow,
    action_row: &adw::ActionRow,
    action_button: &gtk::Button,
) {
    match state.phase {
        UpdatePanelPhase::Checking => {
            latest_release_row.set_subtitle("Checking GitHub releases...");
            status_row.set_subtitle(
                "Contacting GitHub and selecting the right installer for this Limux installation.",
            );
            action_row.set_subtitle("The button is disabled while the release check is running.");
            action_button.set_label("Checking...");
            action_button.set_sensitive(false);
            return;
        }
        UpdatePanelPhase::Preparing => {
            let action_text = state
                .last_summary
                .as_ref()
                .and_then(|summary| summary.selected_asset.as_ref())
                .map(|asset| format!("Downloading and verifying {}...", asset.name))
                .unwrap_or_else(|| "Downloading and verifying the update...".to_string());
            latest_release_row.set_subtitle("Preparing update");
            status_row.set_subtitle(&action_text);
            action_row.set_subtitle(
                "Limux is downloading the release asset and preparing it for installation.",
            );
            action_button.set_label("Preparing...");
            action_button.set_sensitive(false);
            return;
        }
        UpdatePanelPhase::Idle => {}
    }

    if let Some(prepared) = &state.prepared_update {
        if let Some(summary) = &state.last_summary {
            latest_release_row.set_subtitle(&format!(
                "{} · {}",
                summary.latest_tag,
                update_checker::format_release_date(&summary.published_at)
            ));
            status_row.set_subtitle(&format!(
                "Ready to install {} via {}.",
                prepared.asset_name,
                prepared.install_mode.label()
            ));
            action_row.set_subtitle(
                "Click to restart Limux, apply the prepared update, and relaunch into the new version.",
            );
            action_button.set_label("Restart to install");
            action_button.set_sensitive(true);
            return;
        }
    }

    if let Some(error) = &state.last_error {
        latest_release_row.set_subtitle("Unavailable");
        status_row.set_subtitle(error);
        action_row.set_subtitle(
            "Open the Limux releases page manually if the in-app update flow cannot continue.",
        );
        action_button.set_label("Open releases");
        action_button.set_sensitive(true);
        return;
    }

    if let Some(summary) = &state.last_summary {
        latest_release_row.set_subtitle(&format!(
            "{} · {}",
            summary.latest_tag,
            update_checker::format_release_date(&summary.published_at)
        ));
        status_row.set_subtitle(&summary.status_detail);
        action_button.set_sensitive(true);
        match summary.status {
            UpdateStatus::UpdateAvailable => {
                let next_step = if summary.install_target.mode.needs_privileged_installer() {
                    "Limux will download the installer first, then ask for confirmation and administrator permission during install."
                } else {
                    "Limux will download the update first, then ask to restart to finish the installation."
                };
                action_row.set_subtitle(next_step);
                action_button.set_label("Install update");
            }
            UpdateStatus::UpToDate => {
                action_row.set_subtitle("This build is current. Run the check again any time.");
                action_button.set_label("Check again");
            }
            UpdateStatus::UnsupportedInstallation => {
                action_row.set_subtitle(
                    "This install type cannot be updated fully in-app. Open the release page instead.",
                );
                action_button.set_label("Open releases");
            }
        }
        return;
    }

    latest_release_row.set_subtitle("Not checked yet");
    status_row.set_subtitle("Check GitHub releases to see whether an update is available.");
    action_row.set_subtitle(
        "Checks the latest GitHub release, picks the right installer asset, and compares it with this installed version.",
    );
    action_button.set_label("Check for updates");
    action_button.set_sensitive(true);
}

fn update_button_action(state: &UpdatePanelState) -> UpdateButtonAction {
    if !matches!(state.phase, UpdatePanelPhase::Idle) {
        return UpdateButtonAction::Check;
    }

    if state.prepared_update.is_some() {
        return UpdateButtonAction::ApplyPrepared;
    }

    if state.last_error.is_some() {
        return UpdateButtonAction::OpenReleases;
    }

    match state
        .last_summary
        .as_ref()
        .map(|summary| summary.status.clone())
    {
        Some(UpdateStatus::UpdateAvailable) => UpdateButtonAction::PrepareInstall,
        Some(UpdateStatus::UnsupportedInstallation) => UpdateButtonAction::OpenReleases,
        _ => UpdateButtonAction::Check,
    }
}

fn push_toast(overlay: &adw::ToastOverlay, message: &str) {
    let toast = adw::Toast::new(message);
    toast.set_timeout(4);
    overlay.add_toast(toast);
}

fn start_update_check(
    state: Rc<RefCell<UpdatePanelState>>,
    latest_release_row: adw::ActionRow,
    status_row: adw::ActionRow,
    action_row: adw::ActionRow,
    action_button: gtk::Button,
    toast_overlay: adw::ToastOverlay,
) {
    {
        let mut panel_state = state.borrow_mut();
        if !matches!(panel_state.phase, UpdatePanelPhase::Idle) {
            return;
        }
        panel_state.phase = UpdatePanelPhase::Checking;
        panel_state.last_error = None;
        panel_state.prepared_update = None;
        render_update_panel(
            &panel_state,
            &latest_release_row,
            &status_row,
            &action_row,
            &action_button,
        );
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let spawn_result = std::thread::Builder::new()
        .name("limux-update-check".into())
        .spawn(move || {
            let _ = tx.send(update_checker::fetch_update_summary(crate::VERSION));
        });

    if let Err(err) = spawn_result {
        let message = format!("Failed to start the update check: {err}");
        push_toast(&toast_overlay, &message);
        let mut panel_state = state.borrow_mut();
        panel_state.phase = UpdatePanelPhase::Idle;
        panel_state.last_summary = None;
        panel_state.prepared_update = None;
        panel_state.last_error = Some(message);
        render_update_panel(
            &panel_state,
            &latest_release_row,
            &status_row,
            &action_row,
            &action_button,
        );
        return;
    }

    gtk::glib::timeout_add_local(Duration::from_millis(50), move || match rx.try_recv() {
        Ok(result) => {
            let mut panel_state = state.borrow_mut();
            panel_state.phase = UpdatePanelPhase::Idle;
            match result {
                Ok(summary) => {
                    let toast_text = match summary.status {
                        UpdateStatus::UpToDate => {
                            format!(
                                "You're on the latest version (v{}).",
                                summary.latest_version
                            )
                        }
                        UpdateStatus::UpdateAvailable => {
                            format!("Update available: v{}.", summary.latest_version)
                        }
                        UpdateStatus::UnsupportedInstallation => {
                            "This install type can't be updated in-app. Open the release page."
                                .to_string()
                        }
                    };
                    push_toast(&toast_overlay, &toast_text);
                    panel_state.last_summary = Some(summary);
                    panel_state.last_error = None;
                    panel_state.prepared_update = None;
                }
                Err(err) => {
                    push_toast(&toast_overlay, &err);
                    panel_state.last_summary = None;
                    panel_state.last_error = Some(err);
                    panel_state.prepared_update = None;
                }
            }
            render_update_panel(
                &panel_state,
                &latest_release_row,
                &status_row,
                &action_row,
                &action_button,
            );
            gtk::glib::ControlFlow::Break
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            let message = "Failed to receive the GitHub update check result.".to_string();
            push_toast(&toast_overlay, &message);
            let mut panel_state = state.borrow_mut();
            panel_state.phase = UpdatePanelPhase::Idle;
            panel_state.last_summary = None;
            panel_state.prepared_update = None;
            panel_state.last_error = Some(message);
            render_update_panel(
                &panel_state,
                &latest_release_row,
                &status_row,
                &action_row,
                &action_button,
            );
            gtk::glib::ControlFlow::Break
        }
    });
}

fn start_prepare_update(
    state: Rc<RefCell<UpdatePanelState>>,
    summary: Option<update_checker::UpdateSummary>,
    latest_release_row: adw::ActionRow,
    status_row: adw::ActionRow,
    action_row: adw::ActionRow,
    action_button: gtk::Button,
    toast_overlay: adw::ToastOverlay,
) {
    let Some(summary) = summary else {
        let message = "No update summary is available yet.".to_string();
        push_toast(&toast_overlay, &message);
        let mut panel_state = state.borrow_mut();
        panel_state.last_error = Some(message);
        render_update_panel(
            &panel_state,
            &latest_release_row,
            &status_row,
            &action_row,
            &action_button,
        );
        return;
    };

    {
        let mut panel_state = state.borrow_mut();
        if !matches!(panel_state.phase, UpdatePanelPhase::Idle) {
            return;
        }
        panel_state.phase = UpdatePanelPhase::Preparing;
        panel_state.last_error = None;
        panel_state.prepared_update = None;
        render_update_panel(
            &panel_state,
            &latest_release_row,
            &status_row,
            &action_row,
            &action_button,
        );
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let spawn_result = std::thread::Builder::new()
        .name("limux-update-prepare".into())
        .spawn(move || {
            let _ = tx.send(update_checker::prepare_update(&summary));
        });

    if let Err(err) = spawn_result {
        let message = format!("Failed to start the update download: {err}");
        push_toast(&toast_overlay, &message);
        let mut panel_state = state.borrow_mut();
        panel_state.phase = UpdatePanelPhase::Idle;
        panel_state.last_error = Some(message);
        render_update_panel(
            &panel_state,
            &latest_release_row,
            &status_row,
            &action_row,
            &action_button,
        );
        return;
    }

    gtk::glib::timeout_add_local(Duration::from_millis(50), move || match rx.try_recv() {
        Ok(result) => {
            let mut panel_state = state.borrow_mut();
            panel_state.phase = UpdatePanelPhase::Idle;
            match result {
                Ok(prepared) => {
                    push_toast(
                        &toast_overlay,
                        &format!(
                            "Update downloaded ({}). Click Restart to install.",
                            prepared.asset_name
                        ),
                    );
                    panel_state.prepared_update = Some(prepared);
                    panel_state.last_error = None;
                }
                Err(err) => {
                    push_toast(&toast_overlay, &err);
                    panel_state.prepared_update = None;
                    panel_state.last_error = Some(err);
                }
            }
            render_update_panel(
                &panel_state,
                &latest_release_row,
                &status_row,
                &action_row,
                &action_button,
            );
            gtk::glib::ControlFlow::Break
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            let message = "Failed to receive the prepared update payload.".to_string();
            push_toast(&toast_overlay, &message);
            let mut panel_state = state.borrow_mut();
            panel_state.phase = UpdatePanelPhase::Idle;
            panel_state.prepared_update = None;
            panel_state.last_error = Some(message);
            render_update_panel(
                &panel_state,
                &latest_release_row,
                &status_row,
                &action_row,
                &action_button,
            );
            gtk::glib::ControlFlow::Break
        }
    });
}

fn open_url(url: &str) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("Failed to open the browser for `{url}`: {err}"))
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

    #[test]
    fn update_button_action_prefers_download_for_available_updates() {
        let state = UpdatePanelState {
            last_summary: Some(update_checker::UpdateSummary {
                current_version: "0.1.12".to_string(),
                latest_version: "0.1.13".to_string(),
                latest_tag: "v0.1.13".to_string(),
                published_at: "2026-04-12T06:50:45Z".to_string(),
                commits_behind: 24,
                release_url: update_checker::RELEASES_URL.to_string(),
                selected_asset: Some(update_checker::ReleaseAsset {
                    name: "limux-0.1.13-linux-x86_64.tar.gz".to_string(),
                    download_url: "https://example.invalid/limux-0.1.13-linux-x86_64.tar.gz"
                        .to_string(),
                    digest_sha256: None,
                }),
                install_target: update_checker::InstallTarget {
                    mode: update_checker::InstallMode::Bundle,
                    target_path: std::path::PathBuf::from("/tmp/limux-app"),
                    relaunch_path: std::path::PathBuf::from("/tmp/limux-app/limux"),
                },
                status: UpdateStatus::UpdateAvailable,
                status_detail: "Update available: 24 commits behind.".to_string(),
            }),
            ..UpdatePanelState::default()
        };

        assert_eq!(
            update_button_action(&state),
            UpdateButtonAction::PrepareInstall
        );
    }

    #[test]
    fn update_button_action_uses_release_link_after_failures() {
        let state = UpdatePanelState {
            last_error: Some("network down".to_string()),
            ..UpdatePanelState::default()
        };

        assert_eq!(
            update_button_action(&state),
            UpdateButtonAction::OpenReleases
        );
    }
}
