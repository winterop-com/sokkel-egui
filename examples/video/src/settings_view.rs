//! The viewer's settings as a dialog over the page, as the web's `pages/Settings.tsx`: playback and
//! captions, the history and the look, and what this device keeps; each kept as it is changed.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, Icon, Segmented, Select, SettingRow, SettingsDialog,
    SettingsSection, Switch, ThemeWords, theme_settings_of,
};

use crate::act::{Act, Asking};
use crate::app::VideoApp;
use crate::data::{Data, PrefsPatch};
use crate::format::{bytes_of, plural};
use crate::pages::settings::{backgrounds, caption_controls, caption_sizes, colors, edges, notes, qualities, speeds};
use crate::themes::CARRIED;

/// A small outline button.
fn small(label: &str, on: bool) -> Button {
    Button::new(label)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .enabled(on)
}

/// A switch for a setting: the new value when it was turned.
fn switch(ui: &mut Ui, label: &str, on: bool) -> Option<bool> {
    let mut value = on;
    ui.add(Switch::new(&mut value, label));
    (value != on).then_some(value)
}

/// One of a few choices as a segmented control: the index chosen when it changed.
fn segmented<'a>(ui: &mut Ui, chosen: usize, options: impl IntoIterator<Item = &'a str>) -> Option<usize> {
    let mut at = chosen;
    ui.add(Segmented::new(&mut at, options)).changed().then_some(at)
}

fn patch(asked: &mut Vec<Act>, patch: PrefsPatch) {
    asked.push(Act::Prefs(patch));
}

/// The rows of a section, asking for the changes made.
fn rows(section: &str, ui: &mut Ui, data: &Data, appearance: &mut Appearance, asked: &mut Vec<Act>) {
    let prefs = &data.prefs;
    match section {
        "playback" => {
            SettingRow::new(
                "Autoplay",
                "When a video ends, Up next counts down and plays the next one. Off, it waits for Play now.",
            )
            .keywords(&["next", "up next", "countdown"])
            .show(ui, |ui| {
                if let Some(on) = switch(ui, "Autoplay", prefs.autoplay) {
                    patch(
                        asked,
                        PrefsPatch {
                            autoplay: Some(on),
                            ..Default::default()
                        },
                    );
                }
            });
            SettingRow::new(
                "Theater mode",
                "The player as wide as the page, with what plays next under it.",
            )
            .keywords(&["wide", "cinema"])
            .show(ui, |ui| {
                if let Some(on) = switch(ui, "Theater mode", prefs.theater) {
                    patch(
                        asked,
                        PrefsPatch {
                            theater: Some(on),
                            ..Default::default()
                        },
                    );
                }
            });
            SettingRow::new(
                "Speed",
                "The speed every video starts at. Picking one in the player keeps it here too.",
            )
            .keywords(&["rate", "faster", "slower"])
            .show(ui, |ui| {
                let offered = speeds();
                let mut at = offered.iter().position(|(rate, _)| *rate == prefs.rate).unwrap_or(2);
                let before = at;
                ui.add(Select::new(&mut at, offered.iter().map(|(_, word)| word.clone()), "Speed").width(96.0));
                if at != before {
                    patch(
                        asked,
                        PrefsPatch {
                            rate: Some(offered[at].0),
                            ..Default::default()
                        },
                    );
                }
            });
            SettingRow::new(
                "Quality",
                "Auto plays 480p where the player is wide and 360p on a phone; a higher quality takes longer to make the first time.",
            )
            .keywords(&["resolution", "rendition", "720p", "480p"])
            .below()
            .show(ui, |ui| {
                let offered = qualities();
                let at = offered.iter().position(|(one, _)| *one == prefs.quality).unwrap_or(0);
                if let Some(at) = segmented(ui, at, offered.iter().map(|(_, label)| *label)) {
                    patch(asked, PrefsPatch { quality: Some(offered[at].0), ..Default::default() });
                }
            });
            SettingRow::new(
                "Stats over the picture",
                "The picture size, bitrate, link speed, buffer and dropped frames while a video plays. The I key turns them on and off.",
            )
            .keywords(&["nerd", "bitrate", "debug"])
            .show(ui, |ui| {
                if let Some(on) = switch(ui, "Stats over the picture", prefs.stats) {
                    patch(asked, PrefsPatch { stats: Some(on), ..Default::default() });
                }
            });
        }
        "captions" => {
            let controls = caption_controls(data);
            let mut change = |change: Box<dyn FnOnce(&mut crate::data::CaptionChoice)>| {
                let mut captions = prefs.captions.clone();
                change(&mut captions);
                patch(
                    asked,
                    PrefsPatch {
                        captions: Some(captions),
                        ..Default::default()
                    },
                );
            };
            SettingRow::new(
                "Show captions",
                "Captions on every video that has them. The C key and the captions menu in the player set the same.",
            )
            .keywords(&["subtitles"])
            .show(ui, |ui| {
                if let Some(on) = switch(ui, "Show captions", controls.on) {
                    change(Box::new(move |c| {
                        c.track = if on {
                            crate::data::CaptionTrack::On("en".to_owned())
                        } else {
                            crate::data::CaptionTrack::Off
                        };
                    }));
                }
            });
            SettingRow::new("Size", "").below().show(ui, |ui| {
                let sizes = caption_sizes();
                let at = sizes.iter().position(|(value, _)| *value == controls.size).unwrap_or(2);
                if let Some(at) = segmented(ui, at, sizes.iter().map(|(_, label)| label.as_str())) {
                    let size: f64 = sizes[at].0.parse().unwrap_or(1.0);
                    change(Box::new(move |c| c.size = Some(size)));
                }
            });
            SettingRow::new("Color", "").show(ui, |ui| {
                let offered = colors();
                let mut at = offered.iter().position(|(one, _)| *one == controls.color).unwrap_or(0);
                let before = at;
                ui.add(Select::new(&mut at, offered.iter().map(|(_, name)| *name), "Caption color").width(120.0));
                if at != before {
                    let color = offered[at].0.to_owned();
                    change(Box::new(move |c| c.color = Some(color)));
                }
            });
            SettingRow::new("Background", "").below().show(ui, |ui| {
                let offered = backgrounds();
                let at = offered
                    .iter()
                    .position(|(one, _)| *one == controls.background)
                    .unwrap_or(0);
                if let Some(at) = segmented(ui, at, offered.iter().map(|(_, name)| *name)) {
                    let background = offered[at].0.to_owned();
                    change(Box::new(move |c| c.background = Some(background)));
                }
            });
            SettingRow::new("Edge", "").below().show(ui, |ui| {
                let offered = edges();
                let at = offered.iter().position(|(one, _)| *one == controls.edge).unwrap_or(0);
                if let Some(at) = segmented(ui, at, offered.iter().map(|(_, name)| *name)) {
                    let edge = offered[at].0.to_owned();
                    change(Box::new(move |c| c.edge = Some(edge)));
                }
            });
        }
        "history" => {
            let said = notes(data);
            SettingRow::new(
                "Keep watch history",
                "Remembers what you watch and where you stopped, so a video picks up there.",
            )
            .keywords(&["resume", "pause history"])
            .show(ui, |ui| {
                if let Some(on) = switch(ui, "Keep watch history", prefs.history) {
                    patch(
                        asked,
                        PrefsPatch {
                            history: Some(on),
                            ..Default::default()
                        },
                    );
                }
            });
            SettingRow::new("Clear watch history", &said.history).show(ui, |ui| {
                if ui.add(small("Clear", !said.clear_disabled)).clicked() {
                    asked.push(Act::Ask(Asking::ClearHistory { clearing: false }));
                }
            });
        }
        "storage" => {
            let said = notes(data);
            SettingRow::new(
                "Made videos",
                "Every video is made in this browser the first time it plays, then kept so it plays at once the next time. Clearing makes each again when next watched; downloads stay.",
            )
            .keywords(&["cache", "space", "clips"])
            .note(format!("{} made on this visit, {}", plural(0.0, "video", None), bytes_of(0.0)))
            .show(ui, |ui| {
                if ui.add(small("Clear", true)).clicked() {
                    asked.push(Act::Toast("Made videos cleared".into()));
                }
            });
            SettingRow::new("Downloads", &said.downloads).show(ui, |ui| {
                if ui.add(small("Open", true)).clicked() {
                    asked.push(Act::CloseSettings);
                    asked.push(Act::Go("downloads".into()));
                }
            });
        }
        "demo" => {
            SettingRow::new(
                "Reset demo data",
                "Puts back the channels followed, the history, playlists, likes, comments and these settings as the demo started, and lets the downloads go.",
            )
            .show(ui, |ui| {
                if ui.add(small("Reset", true)).clicked() {
                    asked.push(Act::Ask(Asking::Reset));
                }
            });
        }
        _ => theme_settings_of(ui, appearance, &CARRIED, true, &ThemeWords::default()),
    }
}

impl VideoApp {
    /// The settings over the page while they are open.
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut active) = self.settings.take() else { return };
        let sections = [
            SettingsSection::new("playback", "Playback", Icon::Play).group("Player"),
            SettingsSection::new("captions", "Captions", Icon::Captions).group("Player"),
            SettingsSection::new("history", "History", Icon::History).group("You"),
            SettingsSection::new("appearance", "Appearance", Icon::Palette).group("You"),
            SettingsSection::new("storage", "Storage", Icon::HardDrive)
                .group("This device")
                .reset(false),
            SettingsSection::new("demo", "Demo data", Icon::Database)
                .group("This device")
                .reset(false),
        ];
        let mut asked = Vec::new();
        let data = &self.data;
        let appearance = &mut self.appearance;
        let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| {
            rows(section, ui, data, appearance, &mut asked);
        });
        if outcome.reset {
            if active == "appearance" {
                self.appearance = Appearance::default();
            } else {
                crate::pages::settings::reset(&mut self.data, &active);
            }
        }
        let mut open = !outcome.close;
        for act in asked {
            if act == Act::CloseSettings {
                open = false;
            } else {
                self.apply(ctx, act);
            }
        }
        if open {
            self.settings = Some(active);
        }
    }
}
