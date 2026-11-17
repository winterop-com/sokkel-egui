//! The app's preferences as a dialog over the page, as the web's `Settings.tsx`: the dashboards'
//! range, refresh and time zone, the look, and the demo's data.

use eframe::egui;
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, ConfirmAnswer, ConfirmDialog, Icon, Segmented, Select,
    SettingsDialog, SettingsSection, ThemeWords, setting_row, setting_row_found_by, theme_settings,
};

use crate::MetricsApp;
use crate::app::{Preferences, Zone};
use crate::model::seed_dashboards;
use crate::query::{INTERVALS, PRESETS, Range};

impl MetricsApp {
    /// The settings over the page while they are open, and the question before the demo's data
    /// goes back.
    pub(crate) fn settings_dialog(&mut self, ctx: &egui::Context) {
        if let Some(mut active) = self.settings.take() {
            let sections = [
                SettingsSection::new("dashboards", "Dashboards", Icon::LayoutDashboard).group("Preferences"),
                SettingsSection::new("appearance", "Appearance", Icon::Palette).group("Preferences"),
                SettingsSection::new("data", "Data", Icon::Database)
                    .group("This demo")
                    .reset(false),
            ];
            let before = self.prefs.clone();
            let (prefs, appearance) = (&mut self.prefs, &mut self.appearance);
            let mut reset_data = false;
            let outcome =
                SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| match section {
                    "dashboards" => {
                        let labels: Vec<&str> = PRESETS.iter().map(|p| p.label).collect();
                        setting_row_found_by(
                            ui,
                            "Range",
                            "What a dashboard covers when its link names no range.",
                            &["time", "default"],
                            |ui| {
                                let mut at = PRESETS.iter().position(|p| p.id == prefs.range).unwrap_or(0);
                                if ui
                                    .add(Select::new(&mut at, labels.iter().copied(), "Range").width(150.0))
                                    .changed()
                                {
                                    prefs.range = PRESETS[at].id;
                                }
                            },
                        );
                        setting_row_found_by(
                            ui,
                            "Auto refresh",
                            "How often a dashboard reads the source again when its link says nothing.",
                            &["live", "interval"],
                            |ui| {
                                let labels: Vec<String> = INTERVALS
                                    .iter()
                                    .map(|s| if *s == 0 { "Off".to_owned() } else { format!("{s}s") })
                                    .collect();
                                let mut at = INTERVALS.iter().position(|s| *s == prefs.refresh).unwrap_or(1);
                                if ui.add(Segmented::new(&mut at, labels)).changed() {
                                    prefs.refresh = INTERVALS[at];
                                }
                            },
                        );
                        setting_row_found_by(
                            ui,
                            "Time zone",
                            "Where the times on the axes, the logs and the alerts are read.",
                            &["utc", "clock"],
                            |ui| {
                                let mut at = usize::from(prefs.zone == Zone::Utc);
                                if ui.add(Segmented::new(&mut at, ["This computer", "UTC"])).changed() {
                                    prefs.zone = if at == 0 { Zone::Local } else { Zone::Utc };
                                }
                            },
                        );
                    }
                    "appearance" => theme_settings(ui, appearance, true, &ThemeWords::default()),
                    _ => {
                        setting_row(
                            ui,
                            "Reset demo data",
                            "Puts the dashboards and these preferences back as they first were.",
                            |ui| {
                                let reset = Button::new("Reset")
                                    .size(ButtonSize::Sm)
                                    .variant(ButtonVariant::Outline);
                                if ui.add(reset).clicked() {
                                    reset_data = true;
                                }
                            },
                        );
                    }
                });
            if outcome.reset {
                match active.as_str() {
                    "dashboards" => {
                        let defaults = Preferences::default();
                        self.prefs.range = defaults.range;
                        self.prefs.refresh = defaults.refresh;
                        self.prefs.zone = defaults.zone;
                    }
                    "appearance" => self.appearance = Appearance::default(),
                    _ => {}
                }
            }
            self.follow_preferences(&before);
            if reset_data {
                self.resetting = true;
            }
            if !outcome.close {
                self.settings = Some(active);
            }
        }
        if self.resetting {
            let question = ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                .detail("Every dashboard goes back to how the demo started. This cannot be undone.")
                .destructive(true);
            match question.show(ctx) {
                Some(ConfirmAnswer::Confirm) => {
                    self.resetting = false;
                    self.reset_demo();
                }
                Some(_) => self.resetting = false,
                None => {}
            }
        }
    }

    /// A changed default reaches the board where the reader has not picked their own, as a link
    /// naming no range or refresh follows the preferences.
    fn follow_preferences(&mut self, before: &Preferences) {
        if self.prefs.range != before.range
            && !self.range_chosen
            && let Some(preset) = PRESETS.iter().find(|p| p.id == self.prefs.range)
        {
            self.range = Range::Preset(preset.id);
            self.view_at = self.at;
        }
        if self.prefs.refresh != before.refresh && !self.refresh_chosen {
            self.refresh = self.prefs.refresh;
        }
    }

    /// The dashboards and the preferences as the demo started, on its first dashboard.
    fn reset_demo(&mut self) {
        self.dashboards = seed_dashboards();
        self.prefs = Preferences::default();
        self.edit.cancel();
        self.editor = None;
        self.range = Range::default();
        self.refresh = self.prefs.refresh;
        self.range_chosen = false;
        self.refresh_chosen = false;
        self.place = self.dashboards.first().map_or_else(String::new, |b| b.id.clone());
    }
}
