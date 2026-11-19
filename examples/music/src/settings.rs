//! The player's settings as a dialog over the page, as the web's `pages/Settings.tsx`: playback,
//! the bar, the visualizer, the library kept here, and the look.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, ConfirmAnswer, ConfirmDialog, Icon, MEDIA_RATES, Segmented,
    SettingRow, SettingsDialog, SettingsSection, SliderField, Switch, ThemeWords, theme_settings_of,
};

use crate::app::MusicApp;
use crate::catalog::format_bytes;
use crate::data::{Effect, Layout, Library, Look, Ramp, Settings};
use crate::js::number_to_string;
use crate::look::{EFFECTS, LOOKS, RAMPS};
use crate::themes::CARRIED;

/// A choice among a few values as a segmented control: true when it changed.
fn choose<T: Copy + PartialEq>(ui: &mut Ui, value: &mut T, options: &[(T, String)]) -> bool {
    let mut at = options.iter().position(|(one, _)| one == value).unwrap_or(0);
    let changed = ui
        .add(Segmented::new(&mut at, options.iter().map(|(_, label)| label.clone())))
        .changed();
    if changed {
        *value = options[at].0;
    }
    changed
}

/// The rows of a section, changing `settings` and asking for what the buttons do.
fn rows(
    section: &str,
    ui: &mut Ui,
    settings: &mut Settings,
    library: &Library,
    appearance: &mut Appearance,
    asked: &mut Asked,
) {
    match section {
        "playback" => {
            SettingRow::new(
                "Crossfade",
                "Fade each song into the next over a few seconds, rather than playing them back to back.",
            )
            .keywords(&["fade", "gapless", "transition"])
            .show(ui, |ui| {
                let options: Vec<(u32, String)> = [0, 2, 5, 8]
                    .into_iter()
                    .map(|s| (s, if s == 0 { "Off".to_owned() } else { format!("{s} s") }))
                    .collect();
                let mut value = settings.crossfade as u32;
                if choose(ui, &mut value, &options) {
                    settings.crossfade = f64::from(value);
                }
            });
            SettingRow::new(
                "Speed",
                "How fast songs play, the pitch kept. The player bar has the same menu.",
            )
            .keywords(&["rate", "tempo"])
            .below()
            .show(ui, |ui| {
                let options: Vec<(u32, String)> = MEDIA_RATES
                    .iter()
                    .map(|rate| ((rate * 100.0) as u32, format!("{}x", number_to_string(*rate))))
                    .collect();
                let mut value = (settings.rate * 100.0).round() as u32;
                if choose(ui, &mut value, &options) {
                    settings.rate = f64::from(value) / 100.0;
                }
            });
            SettingRow::new("Volume", "").show(ui, |ui| {
                let mut volume = (settings.volume * 100.0).round();
                if ui
                    .add(SliderField::new("Volume", &mut volume, 100.0).unit("%").inline(true))
                    .changed()
                {
                    settings.volume = volume / 100.0;
                    settings.muted = false;
                }
            });
            SettingRow::new("Mute", "Silences the player without losing the volume.").show(ui, |ui| {
                ui.add(Switch::new(&mut settings.muted, "Mute"));
            });
            SettingRow::new(
                "Pick up where I left off",
                "Open the player at the song and the place it was at when the page was left.",
            )
            .keywords(&["restore", "remember"])
            .show(ui, |ui| {
                ui.add(Switch::new(&mut settings.resume, "Pick up where I left off"));
            });
        }
        "bar" => {
            SettingRow::new(
                "Grown bar",
                "How the bar arranges itself once dragged taller from its top edge, and how the full screen follows it.",
            )
            .keywords(&["layout", "rows", "panel", "stage"])
            .show(ui, |ui| {
                choose(
                    ui,
                    &mut settings.layout,
                    &[
                        (Layout::Rows, "Rows".into()),
                        (Layout::Panel, "Panel".into()),
                        (Layout::Stage, "Stage".into()),
                    ],
                );
            });
            // the bar draws one line up to about 96 pixels
            let one_line = settings.bar_size.is_none_or(|size| size <= 96.0);
            let description = if one_line {
                "One line. Drag its top edge to grow it.".to_owned()
            } else {
                format!("{} pixels tall.", number_to_string(settings.bar_size.unwrap_or(0.0)))
            };
            SettingRow::new("Height", &description).show(ui, |ui| {
                let mut line = one_line;
                if choose(ui, &mut line, &[(true, "One line".into()), (false, "Grown".into())]) {
                    settings.bar_size = if line { None } else { Some(300.0) };
                }
            });
            SettingRow::new(
                "Visualizer behind the bar",
                "A faint picture of the sound behind the words, once the bar is grown.",
            )
            .show(ui, |ui| {
                ui.add(Switch::new(&mut settings.bar_visualizer, "Visualizer behind the bar"));
            });
        }
        "visualizer" => {
            SettingRow::new(
                "Look",
                "The picture of the sound in the full screen, also picked there.",
            )
            .keywords(&["spectrum", "bars", "ring", "wave"])
            .below()
            .show(ui, |ui| {
                let options: Vec<(Look, String)> =
                    LOOKS.iter().map(|(look, label)| (*look, (*label).to_owned())).collect();
                choose(ui, &mut settings.look, &options);
            });
            SettingRow::new(
                "Colors",
                "The colors it runs through from quiet to loud, each taken from the theme's own.",
            )
            .below()
            .show(ui, |ui| {
                let options: Vec<(Ramp, String)> =
                    RAMPS.iter().map(|(ramp, label)| (*ramp, (*label).to_owned())).collect();
                choose(ui, &mut settings.ramp, &options);
            });
            SettingRow::new(
                "Effect",
                "A glow around what is lit, or trails behind it. Off while motion is reduced.",
            )
            .show(ui, |ui| {
                let options: Vec<(Effect, String)> = EFFECTS
                    .iter()
                    .map(|(effect, label)| (*effect, (*label).to_owned()))
                    .collect();
                choose(ui, &mut settings.effect, &options);
            });
            SettingRow::new(
                "Delay",
                "Holds the picture back for a speaker that plays late, such as a wireless one.",
            )
            .keywords(&["bluetooth", "latency", "sync"])
            .show(ui, |ui| {
                ui.add(
                    SliderField::new("Delay", &mut settings.delay, 500.0)
                        .step(10.0)
                        .unit("ms")
                        .inline(true),
                );
            });
            SettingRow::new(
                "Words in the full screen",
                "Open the full screen on the words of what plays, where it has them, rather than the picture.",
            )
            .keywords(&["lyrics"])
            .show(ui, |ui| {
                ui.add(Switch::new(&mut settings.lyrics, "Words in the full screen"));
            });
        }
        "library" => {
            let kept = &library.downloads;
            let bytes: f64 = kept.iter().map(|row| row.bytes).sum();
            let downloads = if kept.is_empty() {
                "Nothing is downloaded.".to_owned()
            } else {
                format!("{} albums, {}.", kept.len(), format_bytes(bytes))
            };
            let button = |label: &str, on: bool| {
                Button::new(label)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm)
                    .enabled(on)
            };
            SettingRow::new("Downloads", &downloads).show(ui, |ui| {
                asked.remove_downloads = ui.add(button("Remove all", !kept.is_empty())).clicked();
            });
            let played = format!("{} songs, listed beside the queue.", library.history.len());
            SettingRow::new("Recently played", &played).show(ui, |ui| {
                asked.clear_history = ui.add(button("Clear", !library.history.is_empty())).clicked();
            });
            let made = &library.stations;
            let stations = if made.is_empty() {
                "None yet.".to_owned()
            } else {
                format!("{} started from artists, albums and songs.", made.len())
            };
            SettingRow::new("Your stations", &stations).show(ui, |ui| {
                asked.remove_stations = ui.add(button("Remove all", !made.is_empty())).clicked();
            });
            SettingRow::new(
                "Reset demo data",
                "Puts the favorites, playlists, downloads, stations, history, queue and settings back as they were.",
            )
            .show(ui, |ui| {
                asked.reset = ui.add(button("Reset", true)).clicked();
            });
        }
        _ => theme_settings_of(
            ui,
            appearance,
            &CARRIED,
            true,
            &ThemeWords {
                theme_hint:
                    "The colors of surfaces, lines, text and the accent. The full screen stays dark in every one."
                        .into(),
                ..ThemeWords::default()
            },
        ),
    }
}

/// What the library section's buttons asked.
#[derive(Default)]
struct Asked {
    remove_downloads: bool,
    clear_history: bool,
    remove_stations: bool,
    reset: bool,
}

impl MusicApp {
    /// The settings over the page while they are open, and the question before the demo's data
    /// goes back.
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        if let Some(mut active) = self.settings.take() {
            let sections = [
                SettingsSection::new("playback", "Playback", Icon::Play).group("Listening"),
                SettingsSection::new("bar", "Player bar", Icon::PanelBottom).group("Listening"),
                SettingsSection::new("visualizer", "Visualizer", Icon::AudioLines).group("Listening"),
                SettingsSection::new("library", "On this device", Icon::Library)
                    .group("Library")
                    .reset(false),
                SettingsSection::new("appearance", "Appearance", Icon::Palette).group("You"),
            ];
            let mut settings = self.library.settings.clone();
            let mut asked = Asked::default();
            let library = &self.library;
            let appearance = &mut self.appearance;
            let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| {
                rows(section, ui, &mut settings, library, appearance, &mut asked);
            });
            if outcome.reset {
                let defaults = Settings::default();
                match active.as_str() {
                    "playback" => {
                        settings.crossfade = defaults.crossfade;
                        settings.rate = defaults.rate;
                        settings.volume = defaults.volume;
                        settings.muted = defaults.muted;
                        settings.resume = defaults.resume;
                    }
                    "bar" => {
                        settings.layout = defaults.layout;
                        settings.bar_size = defaults.bar_size;
                        settings.bar_visualizer = defaults.bar_visualizer;
                    }
                    "visualizer" => {
                        settings.look = defaults.look;
                        settings.ramp = defaults.ramp;
                        settings.effect = defaults.effect;
                        settings.delay = defaults.delay;
                        settings.lyrics = defaults.lyrics;
                    }
                    "appearance" => self.appearance = Appearance::default(),
                    _ => {}
                }
            }
            self.library.settings = settings;
            if asked.remove_downloads {
                self.library.remove_all_downloads();
            }
            if asked.clear_history {
                self.library.clear_history();
            }
            if asked.remove_stations {
                self.library.stations.clear();
            }
            if asked.reset {
                self.resetting = true;
            }
            if !outcome.close {
                self.settings = Some(active);
            }
        }
        if self.resetting {
            let question = ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                .detail(
                    "Every favorite, playlist, download, station, play, the queue and the settings go back to how the demo started.",
                )
                .destructive(true);
            match question.show(ctx) {
                Some(ConfirmAnswer::Confirm) => self.reset_demo(),
                Some(_) => self.resetting = false,
                None => {}
            }
        }
    }

    /// Everything the listener kept as the demo started, the page loaded again where it stood.
    pub fn reset_demo(&mut self) {
        let now = self.now();
        let mut library = Library::default();
        let mut player = crate::player::Player::default();
        let row = library.player.clone();
        player.restore(&mut library, &row, true, now);
        self.library = library;
        self.player = player;
        self.fullscreen = false;
        self.resetting = false;
        self.settings = None;
        self.asking = None;
    }
}
