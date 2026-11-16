use eframe::egui::Ui;
use sokkel_egui::{
    Appearance, Button, ButtonVariant, Icon, SettingsDialog, SettingsSection, Switch, ThemeWords, setting_row,
    theme_settings,
};

use super::{State, section};

pub fn sections() -> Vec<SettingsSection> {
    vec![
        SettingsSection::new("appearance", "Appearance", Icon::Palette),
        SettingsSection::new("notifications", "Notifications", Icon::Bell),
        SettingsSection::new("runs", "Runs", Icon::Play),
    ]
}

/// The rows of one section of Tidevann's settings.
pub fn rows(state: &mut State, appearance: &mut Appearance, section: &str, ui: &mut Ui) {
    match section {
        "appearance" => theme_settings(ui, appearance, true, &ThemeWords::default()),
        "notifications" => {
            setting_row(
                ui,
                "Failed runs",
                "Tell ada when a run fails, with its first problem.",
                |ui| {
                    ui.add(Switch::new(&mut state.notify, "Failed runs"));
                },
            );
            setting_row(ui, "Retries", "Tell ada when a run needed a retry to pass.", |ui| {
                ui.add(Switch::new(&mut state.retry, "Retries"));
            });
        }
        _ => {
            setting_row(
                ui,
                "Pause every schedule",
                "No job starts on its own until this is off again.",
                |ui| {
                    ui.add(Switch::new(&mut state.paused, "Pause every schedule"));
                },
            );
        }
    }
}

pub fn settings_dialog(state: &mut State, ui: &mut Ui) {
    section(ui, "Tidevann's settings");
    if ui
        .add(
            Button::new("Settings")
                .icon(Icon::Settings)
                .variant(ButtonVariant::Outline),
        )
        .clicked()
    {
        state.settings_open = true;
    }
    if state.settings_open {
        let sections = sections();
        let mut active = std::mem::take(&mut state.settings_section);
        let mut appearance = std::mem::take(&mut state.settings_appearance);
        let outcome = SettingsDialog::new("story-settings", &sections, &mut active)
            .show(ui.ctx(), |section, ui| rows(state, &mut appearance, section, ui));
        if outcome.reset && active == "appearance" {
            appearance = Appearance::default();
        }
        state.settings_section = active;
        state.settings_appearance = appearance;
        if outcome.close {
            state.settings_open = false;
        }
    }
}
