//! The person's settings as a dialog over the page, as the web's `Settings.tsx`: playback, editing,
//! the export, the look, and the demo's data.

use eframe::egui;
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, ConfirmAnswer, ConfirmDialog, Icon, NumberField, Segmented,
    SettingsDialog, SettingsSection, Switch, ThemeWords, setting_row, setting_row_found_by, theme_settings_of,
};

use crate::app::CutApp;
use crate::backend::Backend;
use crate::model::{ExportSize, Prefs};

/// The themes the app carries, as the web's `themes.ts`: the only ones it offers.
pub const CARRIED: [&str; 4] = ["tidevann", "paper", "contrast", "ocean"];

impl CutApp {
    /// The settings over the page while they are open, and the question before the demo's data
    /// goes back.
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        if let Some(mut active) = self.settings.take() {
            let sections = [
                SettingsSection::new("playback", "Playback", Icon::Play).group("Preferences"),
                SettingsSection::new("editing", "Editing", Icon::Scissors).group("Preferences"),
                SettingsSection::new("export", "Export", Icon::Film).group("Preferences"),
                SettingsSection::new("appearance", "Appearance", Icon::Palette).group("Preferences"),
                SettingsSection::new("data", "Data", Icon::Database)
                    .group("This demo")
                    .reset(false),
            ];
            let mut prefs = self.backend.prefs.clone();
            let appearance = &mut self.appearance;
            let mut reset = false;
            let outcome =
                SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| match section {
                    "playback" => {
                        setting_row_found_by(
                            ui,
                            "Loop",
                            "Play from the start again on reaching the end of the edit.",
                            &["repeat", "again"],
                            |ui| {
                                ui.add(Switch::new(&mut prefs.looping, "Loop"));
                            },
                        );
                        setting_row_found_by(
                            ui,
                            "Light preview",
                            "Draw the preview at half its size, for a slower machine. The export is not changed.",
                            &["quality", "resolution", "fast"],
                            |ui| {
                                ui.add(Switch::new(&mut prefs.light_preview, "Light preview"));
                            },
                        );
                    }
                    "editing" => {
                        setting_row_found_by(
                            ui,
                            "Snap",
                            "Clips snap to the edges of other clips and to the playhead while they are dragged.",
                            &["magnet", "align"],
                            |ui| {
                                ui.add(Switch::new(&mut prefs.snap, "Snap"));
                            },
                        );
                        setting_row_found_by(
                            ui,
                            "Delete closes the gap",
                            "Delete moves what follows on the track left into the gap, as Ripple delete does.",
                            &["ripple", "remove"],
                            |ui| {
                                ui.add(Switch::new(&mut prefs.ripple_by_default, "Delete closes the gap"));
                            },
                        );
                        setting_row_found_by(
                            ui,
                            "New title length",
                            "How long a title added from the bin runs.",
                            &["caption", "text", "duration"],
                            |ui| {
                                let mut length = prefs.title_length;
                                let field = NumberField::new("title-length", "New title length", &mut length)
                                    .bare(true)
                                    .step(0.5)
                                    .suffix("s")
                                    .width(96.0);
                                if ui.add(field).changed() && (1.0..=30.0).contains(&length) {
                                    prefs.title_length = length;
                                }
                            },
                        );
                    }
                    "export" => {
                        setting_row_found_by(
                            ui,
                            "Resolution",
                            "The size an export is drawn and encoded at. A larger one takes longer.",
                            &["quality", "720p", "480p", "360p"],
                            |ui| {
                                let mut at = ExportSize::ALL
                                    .iter()
                                    .position(|s| *s == prefs.export_size)
                                    .unwrap_or(0);
                                let labels = ExportSize::ALL.map(|s| s.spec().label);
                                if ui.add(Segmented::new(&mut at, labels)).changed() {
                                    prefs.export_size = ExportSize::ALL[at];
                                }
                            },
                        );
                    }
                    "appearance" => theme_settings_of(ui, appearance, &CARRIED, true, &ThemeWords::default()),
                    _ => {
                        setting_row(
                            ui,
                            "Reset demo data",
                            "Puts the projects, their edits and these preferences back as they first were.",
                            |ui| {
                                let button = Button::new("Reset")
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm);
                                reset = ui.add(button).clicked();
                            },
                        );
                    }
                });
            if outcome.reset {
                let defaults = Prefs::default();
                match active.as_str() {
                    "playback" => {
                        prefs.looping = defaults.looping;
                        prefs.light_preview = defaults.light_preview;
                    }
                    "editing" => {
                        prefs.snap = defaults.snap;
                        prefs.ripple_by_default = defaults.ripple_by_default;
                        prefs.title_length = defaults.title_length;
                    }
                    "export" => prefs.export_size = defaults.export_size,
                    "appearance" => self.appearance = Appearance::default(),
                    _ => {}
                }
            }
            if prefs != self.backend.prefs {
                self.backend.update_prefs(|p| *p = prefs);
            }
            if reset {
                self.resetting = true;
            }
            if !outcome.close {
                self.settings = Some(active);
            }
        }
        if self.resetting {
            let question = ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                .detail("Everything changed since the demo started is put back, and the page loads again.")
                .destructive(true);
            match question.show(ctx) {
                Some(ConfirmAnswer::Confirm) => self.reset_demo(),
                Some(_) => self.resetting = false,
                None => {}
            }
        }
    }

    /// The projects and settings as they started, the page loaded again where it stood.
    pub fn reset_demo(&mut self) {
        let now = self.now();
        self.backend = Backend::seeded(now);
        self.editor = None;
        self.resetting = false;
        self.settings = None;
        self.asking = None;
    }
}
