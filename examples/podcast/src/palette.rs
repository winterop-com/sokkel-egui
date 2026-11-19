//! The palette and the player's keys, as the web's `Podcasts.tsx` builds them: the player while an
//! episode is there, Check for new, the rail's places, every show and every released episode, and
//! the look.

use eframe::egui;
use sokkel_egui::{
    AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, RailGroup, appearance_commands,
    place_commands, place_of, run_appearance_command,
};

use crate::RATES;
use crate::act::Act;
use crate::app::PodcastApp;
use crate::catalog::{category_label, episode_by_id, episodes, show_by_id, shows};
use crate::chapters::{chapter_starts, next_chapter_start, previous_chapter_start};
use crate::js::number_to_string;
use crate::sleep_timer::SleepTimer;
use crate::themes::CARRIED;

impl PodcastApp {
    /// What the player's commands and keys ask, by the command's id.
    fn playback(&self, command: &str) -> Vec<Act> {
        let state = &self.state;
        let Some(current) = state.current().map(str::to_owned) else {
            return match command {
                "fullscreen" => vec![Act::Fullscreen(!self.fullscreen)],
                "next" => vec![Act::PlayNext],
                _ => Vec::new(),
            };
        };
        let skip = state.prefs.skip;
        let starts = state
            .script()
            .map(|script| chapter_starts(&script.chapters))
            .unwrap_or_default();
        let position = state.position();
        match command {
            "toggle" => vec![Act::TogglePlay],
            "back" => vec![Act::Skip(-skip)],
            "forward" => vec![Act::Skip(skip)],
            "previous-chapter" => vec![Act::Seek(previous_chapter_start(&starts, position))],
            "next-chapter" => next_chapter_start(&starts, position)
                .map(Act::Seek)
                .into_iter()
                .collect(),
            "next" => vec![Act::PlayNext],
            "now-playing" => vec![Act::Go(format!("episode/{current}"))],
            "star" => {
                let starred = state.episode(&current).is_some_and(|row| row.starred);
                vec![Act::Star(current, !starred)]
            }
            "fullscreen" => vec![Act::Fullscreen(!self.fullscreen)],
            "sleep-off" => vec![Act::Sleep(None)],
            other => {
                if let Some(rate) = other.strip_prefix("rate-").and_then(|r| r.parse::<f64>().ok()) {
                    vec![Act::Rate(rate)]
                } else if let Some(minutes) = other.strip_prefix("sleep-").and_then(|m| m.parse::<f64>().ok()) {
                    vec![Act::Sleep(Some(SleepTimer::Minutes(minutes)))]
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// The keys the player answers, as the web's shortcuts, while no field has the keyboard.
    pub fn keys(&mut self, ctx: &egui::Context) -> Vec<Act> {
        if ctx.egui_wants_keyboard_input() {
            return Vec::new();
        }
        let pressed: Vec<&str> = ctx.input(|i| {
            let mut keys = Vec::new();
            for event in &i.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    repeat,
                    ..
                } = event
                    && !modifiers.command
                    && !modifiers.ctrl
                    && !modifiers.alt
                {
                    let command = match key {
                        egui::Key::Space if !repeat => "toggle",
                        egui::Key::ArrowLeft => "back",
                        egui::Key::ArrowRight => "forward",
                        egui::Key::N if !repeat => "next",
                        egui::Key::S if !repeat => "star",
                        egui::Key::F if !repeat => "fullscreen",
                        _ => continue,
                    };
                    keys.push(command);
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
        let state = &self.state;
        let mut groups: Vec<CommandGroup> = Vec::new();
        if let Some(current) = state.current().and_then(episode_by_id) {
            let skip = number_to_string(state.prefs.skip);
            let starts = state
                .script()
                .map(|script| chapter_starts(&script.chapters))
                .unwrap_or_default();
            let mut items = vec![
                MenuItem::action(if state.playing { "Pause" } else { "Play" })
                    .icon(if state.playing { Icon::Pause } else { Icon::Play })
                    .detail(&current.title)
                    .shortcut("Space")
                    .id("toggle"),
                MenuItem::action(format!("Back {skip} seconds"))
                    .icon(Icon::RotateCcw)
                    .shortcut("←")
                    .id("back"),
                MenuItem::action(format!("Forward {skip} seconds"))
                    .icon(Icon::RotateCw)
                    .shortcut("→")
                    .id("forward"),
                MenuItem::action("Previous chapter")
                    .icon(Icon::SkipBack)
                    .id("previous-chapter"),
            ];
            if next_chapter_start(&starts, state.position()).is_some() {
                items.push(
                    MenuItem::action("Next chapter")
                        .icon(Icon::SkipForward)
                        .id("next-chapter"),
                );
            }
            if let Some(next) = state.player.queue.first().and_then(|id| episode_by_id(id)) {
                items.push(
                    MenuItem::action("Play the next in the queue")
                        .icon(Icon::SkipForward)
                        .detail(&next.title)
                        .shortcut("N")
                        .id("next"),
                );
            }
            items.push(
                MenuItem::action("Open what plays")
                    .icon(Icon::FileText)
                    .detail(&current.title)
                    .keywords(["transcript", "chapters", "notes"])
                    .id("now-playing"),
            );
            for rate in RATES {
                items.push(
                    MenuItem::action(format!("Speed {}x", number_to_string(rate)))
                        .keywords(["speed", "rate"])
                        .id(format!("rate-{rate}")),
                );
            }
            for minutes in [15, 30, 60] {
                items.push(
                    MenuItem::action(format!("Sleep in {minutes} minutes"))
                        .icon(Icon::Moon)
                        .keywords(["sleep", "timer"])
                        .id(format!("sleep-{minutes}")),
                );
            }
            if state.sleep_timer().is_some() {
                items.push(
                    MenuItem::action("Turn the sleep timer off")
                        .icon(Icon::Moon)
                        .id("sleep-off"),
                );
            }
            groups.push(CommandGroup::new("Player", items));
        }
        groups.push(CommandGroup::new(
            "Podcasts",
            [MenuItem::action("Check for new episodes")
                .icon(Icon::RefreshCw)
                .id("check")],
        ));
        groups.extend(place_commands(rail, "Go to"));
        groups.push(CommandGroup::new(
            "Shows",
            shows().iter().map(|show| {
                let hosts: Vec<&str> = show.hosts.iter().map(|host| host.name()).collect();
                MenuItem::action(&show.title)
                    .icon(Icon::Podcast)
                    .detail(format!("{} \u{b7} {}", hosts.join(", "), category_label(show.category)))
                    .keywords(std::iter::once(show.blurb.clone()).chain(hosts.iter().map(|h| (*h).to_owned())))
                    .id(format!("show:{}", show.id))
            }),
        ));
        groups.push(CommandGroup::new(
            "Episodes",
            episodes()
                .iter()
                .filter(|episode| state.episode(&episode.id).is_some_and(|row| row.out))
                .map(|episode| {
                    let mut item = MenuItem::action(&episode.title)
                        .icon(Icon::AudioLines)
                        .keywords([episode.summary.clone(), episode.place.clone()])
                        .id(format!("episode:{}", episode.id));
                    if let Some(show) = show_by_id(&episode.show) {
                        item = item.detail(&show.title);
                    }
                    item
                }),
        ));
        let themes: Vec<_> = sokkel_egui::themes()
            .filter(|theme| CARRIED.contains(&theme.name.as_str()))
            .collect();
        groups.extend(appearance_commands(
            &self.appearance,
            &themes,
            &AppearanceCommandWords::default(),
        ));
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight).show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        if let Some(command) = command.as_deref() {
            let acts = if command == "check" {
                vec![Act::Go("inbox".into()), Act::CheckForNew]
            } else if let Some(id) = command.strip_prefix("show:") {
                vec![Act::Go(format!("show/{id}"))]
            } else if let Some(id) = command.strip_prefix("episode:") {
                vec![Act::Go(format!("episode/{id}"))]
            } else if let Some(place) = place_of(command) {
                vec![Act::Go(place.to_owned())]
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
