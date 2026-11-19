//! The listener's settings as a dialog over the page, as the web's `pages/Settings.tsx`: playback,
//! new episodes, the look with the themes carried, and the data kept.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, Icon, Segmented, SettingRow, SettingsDialog, SettingsSection,
    Switch, ThemeWords, theme_settings_of,
};

use crate::RATES;
use crate::act::{Act, Asking};
use crate::app::PodcastApp;
use crate::backend::{Prefs, PrefsPatch, default_prefs};
use crate::js::number_to_string;
use crate::themes::CARRIED;

/// How far the skip buttons move, as the web's `SKIPS`.
const SKIPS: [f64; 3] = [10.0, 15.0, 30.0];

/// A choice among a few numbers as a segmented control: the one chosen when it changed.
fn choose(ui: &mut Ui, value: f64, options: &[f64], word: impl Fn(f64) -> String) -> Option<f64> {
    let mut at = options.iter().position(|one| *one == value).unwrap_or(0);
    let changed = ui
        .add(Segmented::new(&mut at, options.iter().map(|one| word(*one))))
        .changed();
    changed.then(|| options[at])
}

/// The rows of a section, asking for the changes made.
fn rows(section: &str, ui: &mut Ui, prefs: &Prefs, stats: bool, appearance: &mut Appearance, asked: &mut Vec<Act>) {
    let switch = |ui: &mut Ui, label: &str, on: bool| -> Option<bool> {
        let mut value = on;
        ui.add(Switch::new(&mut value, label));
        (value != on).then_some(value)
    };
    match section {
        "playback" => {
            SettingRow::new(
                "Default speed",
                "The speed a new episode starts at. The player changes it for the episode playing.",
            )
            .keywords(&["rate", "faster", "slower"])
            .show(ui, |ui| {
                if let Some(speed) = choose(ui, prefs.speed, &RATES, |rate| format!("{}x", number_to_string(rate))) {
                    asked.push(Act::Prefs(PrefsPatch {
                        speed: Some(speed),
                        ..PrefsPatch::default()
                    }));
                }
            });
            SettingRow::new(
                "Skip back and forward",
                "How far the skip buttons, the arrow keys and the lock screen move.",
            )
            .keywords(&["seconds", "jump"])
            .show(ui, |ui| {
                if let Some(skip) = choose(ui, prefs.skip, &SKIPS, |seconds| {
                    format!("{} s", number_to_string(seconds))
                }) {
                    asked.push(Act::Prefs(PrefsPatch {
                        skip: Some(skip),
                        ..PrefsPatch::default()
                    }));
                }
            });
            SettingRow::new(
                "Continue with the queue",
                "When an episode ends, the next in the queue plays.",
            )
            .keywords(&["autoplay", "next"])
            .show(ui, |ui| {
                if let Some(on) = switch(ui, "Continue with the queue", prefs.continue_queue) {
                    asked.push(Act::Prefs(PrefsPatch {
                        continue_queue: Some(on),
                        ..PrefsPatch::default()
                    }));
                }
            });
        }
        "episodes" => {
            SettingRow::new(
                "Put new episodes in New",
                "Episodes of the shows you follow wait in New until you play or archive them.",
            )
            .keywords(&["inbox"])
            .show(ui, |ui| {
                if let Some(on) = switch(ui, "Put new episodes in New", prefs.inbox) {
                    asked.push(Act::Prefs(PrefsPatch {
                        inbox: Some(on),
                        ..PrefsPatch::default()
                    }));
                }
            });
            SettingRow::new(
                "Download new episodes",
                "New episodes of the shows you follow download as they come out.",
            )
            .keywords(&["offline", "automatic"])
            .show(ui, |ui| {
                if let Some(on) = switch(ui, "Download new episodes", prefs.auto_download) {
                    asked.push(Act::Prefs(PrefsPatch {
                        auto_download: Some(on),
                        ..PrefsPatch::default()
                    }));
                }
            });
            SettingRow::new(
                "Archive once played",
                "An episode leaves New when it has played to the end or is marked played.",
            )
            .keywords(&["inbox", "played"])
            .show(ui, |ui| {
                if let Some(on) = switch(ui, "Archive once played", prefs.archive_played) {
                    asked.push(Act::Prefs(PrefsPatch {
                        archive_played: Some(on),
                        ..PrefsPatch::default()
                    }));
                }
            });
        }
        "data" => {
            let button = |label: &str, on: bool| {
                Button::new(label)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm)
                    .enabled(on)
            };
            SettingRow::new(
                "Clear the stats",
                "Forgets the time listened and the episodes finished.",
            )
            .keywords(&["history", "listened"])
            .show(ui, |ui| {
                if ui.add(button("Clear", stats)).clicked() {
                    asked.push(Act::Ask(Asking::ClearStats));
                }
            });
            SettingRow::new(
                "Reset the demo data",
                "Puts back the shows, episodes, queue and settings as they started, and loads the page again.",
            )
            .keywords(&["seed", "start over"])
            .show(ui, |ui| {
                if ui.add(button("Reset", true)).clicked() {
                    asked.push(Act::Ask(Asking::Reset));
                }
            });
        }
        _ => theme_settings_of(ui, appearance, &CARRIED, true, &ThemeWords::default()),
    }
}

impl PodcastApp {
    /// The settings over the page while they are open.
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut active) = self.settings.take() else { return };
        let sections = [
            SettingsSection::new("playback", "Playback", Icon::CirclePlay).group("Listening"),
            SettingsSection::new("episodes", "New episodes", Icon::Download).group("Listening"),
            SettingsSection::new("appearance", "Appearance", Icon::Palette).group("App"),
            SettingsSection::new("data", "Data", Icon::Database)
                .group("App")
                .reset(false),
        ];
        let prefs = self.state.prefs.clone();
        let stats = !self.state.listens.is_empty();
        let mut asked = Vec::new();
        let appearance = &mut self.appearance;
        let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| {
            rows(section, ui, &prefs, stats, appearance, &mut asked);
        });
        if outcome.reset {
            let defaults = default_prefs();
            match active.as_str() {
                "appearance" => self.appearance = Appearance::default(),
                "playback" => asked.push(Act::Prefs(PrefsPatch {
                    speed: Some(defaults.speed),
                    skip: Some(defaults.skip),
                    continue_queue: Some(defaults.continue_queue),
                    ..PrefsPatch::default()
                })),
                "episodes" => asked.push(Act::Prefs(PrefsPatch {
                    inbox: Some(defaults.inbox),
                    auto_download: Some(defaults.auto_download),
                    archive_played: Some(defaults.archive_played),
                    ..PrefsPatch::default()
                })),
                _ => {}
            }
        }
        if !outcome.close {
            self.settings = Some(active);
        }
        for act in asked {
            self.apply(ctx, act);
        }
    }
}
