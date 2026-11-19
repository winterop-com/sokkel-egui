//! The palette and the player's keys, as the web's `Music.tsx` builds them: playback while there
//! is a song, the rail's places, a new playlist, the look, and the library's search once two
//! letters are typed.

use eframe::egui;
use sokkel_egui::{
    AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, RailGroup, appearance_commands,
    place_commands, place_of, run_appearance_command,
};

use crate::act::Act;
use crate::app::{MusicApp, open_search, search_results};
use crate::catalog::StationKind;
use crate::data::FavoriteKind;
use crate::dialogs::Asking;
use crate::queue::RepeatMode;
use crate::themes::CARRIED;

impl MusicApp {
    /// What the palette's playback commands and the keys ask, by the command's id.
    fn playback(&self, command: &str) -> Vec<Act> {
        let settings = &self.library.settings;
        let Some(song) = self.player.current_song(&self.library) else {
            return Vec::new();
        };
        let radio = self.player.radio.is_some();
        let mut next = settings.clone();
        match command {
            "play" => vec![Act::TogglePlay],
            "next" => vec![Act::Next],
            "previous" if !radio => vec![Act::Previous],
            "shuffle" if !radio => vec![Act::Shuffle(!self.player.queue.shuffle)],
            "repeat" if !radio => vec![Act::Repeat(self.player.queue.repeat.next())],
            "back" if !radio => vec![Act::SeekBy(-5.0)],
            "ahead" if !radio => vec![Act::SeekBy(5.0)],
            "star" => vec![Act::Star(FavoriteKind::Song, song.id, true)],
            "fullscreen" => vec![Act::Fullscreen(!self.fullscreen)],
            "fullscreen-open" => vec![Act::Fullscreen(true)],
            "radio" => vec![Act::Radio(StationKind::Song, song.id)],
            _ => {
                match command {
                    "louder" => {
                        next.volume = (settings.volume + 0.05).min(1.0);
                        next.muted = false;
                    }
                    "quieter" => next.volume = (settings.volume - 0.05).max(0.0),
                    "mute" => next.muted = !settings.muted,
                    "queue" => next.queue_open = !settings.queue_open,
                    "visualizer" => next.bar_visualizer = !settings.bar_visualizer,
                    "lyrics" => {
                        next.lyrics = !(self.fullscreen && settings.lyrics);
                        return vec![Act::Settings(next), Act::Fullscreen(true)];
                    }
                    "lyrics-open" => {
                        next.lyrics = true;
                        return vec![Act::Settings(next), Act::Fullscreen(true)];
                    }
                    _ => return Vec::new(),
                }
                vec![Act::Settings(next)]
            }
        }
    }

    /// The keys the player answers, as the web's `PlayerKeys`, while no field has the keyboard.
    pub fn keys(&mut self, ctx: &egui::Context) -> Vec<Act> {
        if ctx.egui_wants_keyboard_input() {
            return Vec::new();
        }
        let pressed: Vec<&str> = ctx.input(|i| {
            let mut keys = Vec::new();
            for event in &i.events {
                match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        repeat,
                        ..
                    } if !modifiers.command && !modifiers.ctrl && !modifiers.alt => {
                        let command = match key {
                            egui::Key::Space if !repeat => "play",
                            egui::Key::N if !repeat => "next",
                            egui::Key::P if !repeat => "previous",
                            egui::Key::S if !repeat => "shuffle",
                            egui::Key::R if !repeat => "repeat",
                            egui::Key::ArrowLeft => "back",
                            egui::Key::ArrowRight => "ahead",
                            egui::Key::ArrowUp => "louder",
                            egui::Key::ArrowDown => "quieter",
                            egui::Key::M if !repeat => "mute",
                            egui::Key::F if !repeat => "fullscreen",
                            egui::Key::L if !repeat => "lyrics",
                            egui::Key::Q if !repeat => "queue",
                            egui::Key::V if !repeat => "visualizer",
                            _ => continue,
                        };
                        keys.push(command);
                    }
                    egui::Event::Text(text) if text == "*" => keys.push("star"),
                    _ => {}
                }
            }
            keys
        });
        pressed.into_iter().flat_map(|command| self.playback(command)).collect()
    }

    /// The palette while it is open, and what its command does.
    pub fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let settings = &self.library.settings;
        let mut groups: Vec<CommandGroup> = Vec::new();
        if let Some(song) = self.player.current_song(&self.library) {
            let playing = self.player.playing;
            let repeat = match self.player.queue.repeat {
                RepeatMode::Off => "Repeat the queue",
                RepeatMode::All => "Repeat this song",
                RepeatMode::One => "Repeat off",
            };
            let mut items = vec![
                MenuItem::action(if playing {
                    format!("Pause {}", song.title)
                } else {
                    format!("Play {}", song.title)
                })
                .icon(if playing { Icon::Pause } else { Icon::Play })
                .shortcut("Space")
                .id("play"),
                MenuItem::action("Next song")
                    .icon(Icon::SkipForward)
                    .shortcut("N")
                    .id("next"),
                MenuItem::action("Previous song")
                    .icon(Icon::SkipBack)
                    .shortcut("P")
                    .id("previous"),
                MenuItem::action(if self.player.queue.shuffle {
                    "Play in order"
                } else {
                    "Shuffle the queue"
                })
                .icon(Icon::Shuffle)
                .shortcut("S")
                .id("shuffle"),
                MenuItem::action(repeat).icon(Icon::Repeat).shortcut("R").id("repeat"),
                MenuItem::action(format!("Star {}", song.title))
                    .icon(Icon::Star)
                    .shortcut("*")
                    .id("star"),
                MenuItem::action("Full screen")
                    .icon(Icon::Expand)
                    .shortcut("F")
                    .id("fullscreen-open"),
                MenuItem::action("Show the words of what plays")
                    .icon(Icon::AudioLines)
                    .shortcut("L")
                    .id("lyrics-open"),
                MenuItem::action(if settings.queue_open {
                    "Hide the queue"
                } else {
                    "Show the queue"
                })
                .icon(Icon::ListMusic)
                .shortcut("Q")
                .id("queue"),
                MenuItem::action(if settings.muted { "Unmute" } else { "Mute" })
                    .icon(if settings.muted { Icon::Volume2 } else { Icon::VolumeX })
                    .shortcut("M")
                    .id("mute"),
            ];
            if !song.id.starts_with("radio:") {
                items.push(
                    MenuItem::action(format!("Start radio from {}", song.title))
                        .icon(Icon::Radio)
                        .id("radio"),
                );
            }
            groups.push(CommandGroup::new("Playback", items));
        }
        groups.extend(place_commands(rail, "Go to"));
        groups.push(CommandGroup::new(
            "Playlists",
            [MenuItem::action("New playlist").icon(Icon::ListPlus).id("new-playlist")],
        ));
        let themes: Vec<_> = sokkel_egui::themes()
            .filter(|theme| CARRIED.contains(&theme.name.as_str()))
            .collect();
        groups.extend(appearance_commands(
            &self.appearance,
            &themes,
            &AppearanceCommandWords::default(),
        ));
        // the library's answer once two letters are typed
        let found = search_results(&query);
        for group in &found {
            groups.push(CommandGroup::new(
                &group.title,
                group.results.iter().map(|result| {
                    let mut item = MenuItem::action(&result.title).id(format!("found:{}:{}", group.kind, result.id));
                    if let Some(icon) = result.icon {
                        item = item.icon(icon);
                    }
                    if let Some(detail) = &result.detail {
                        item = item.detail(detail);
                    }
                    item.keywords([query.clone()])
                }),
            ));
        }
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight).show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        if let Some(command) = command.as_deref() {
            let acts = if let Some(rest) = command.strip_prefix("found:") {
                let (kind, id) = rest.split_once(':').unwrap_or((rest, ""));
                open_search(kind, id)
            } else if command == "new-playlist" {
                vec![Act::Ask(Asking::New(Vec::new(), None))]
            } else if let Some(place) = place_of(command) {
                match place.strip_prefix("playlist:") {
                    Some(id) => vec![Act::Go(format!("playlists/{id}"))],
                    None => vec![Act::Go(place.to_owned())],
                }
            } else if run_appearance_command(&mut self.appearance, command) {
                Vec::new()
            } else {
                self.playback(command)
            };
            for act in acts {
                self.apply(ctx, act);
            }
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }
}
