//! The palette and the player's keys, as the web's `Site.tsx` and `pages/Watch.tsx` build them: the
//! rail's places, the playlists, the player's settings, this video's commands while one plays, the
//! videos and channels its search finds, and the look; and the keys a video answers.

use eframe::egui;
use sokkel_egui::{
    AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, RailGroup, appearance_commands,
    place_commands, place_of, run_appearance_command,
};

use crate::act::Act;
use crate::app::VideoApp;
use crate::catalog::{CHANNELS, VIDEOS, channel_of};
use crate::format::short;
use crate::site::{CommandAction, player_commands, playlist_commands};
use crate::themes::CARRIED;

impl VideoApp {
    /// What a key or a command of the video's asks, by the command's id.
    fn video_command(&self, command: &str) -> Vec<Act> {
        let Some(watching) = &self.watching else {
            return Vec::new();
        };
        let duration = watching.video.duration;
        let time = self.player.time;
        match command {
            "toggle" => vec![Act::SetPlaying(!self.player.playing)],
            "back-10" => vec![Act::Seek(time - 10.0)],
            "forward-10" => vec![Act::Seek(time + 10.0)],
            "back-5" => vec![Act::Seek(time - 5.0)],
            "forward-5" => vec![Act::Seek(time + 5.0)],
            "mute" => vec![Act::Muted(!self.player.muted)],
            "full" => vec![Act::Fullscreen(!self.fullscreen)],
            "captions" => {
                let on = matches!(self.data.prefs.captions.track, crate::data::CaptionTrack::On(_));
                vec![Act::Captions(!on)]
            }
            "theater" => vec![Act::Theater],
            "stats" => vec![Act::Stats],
            "next" if watching.next(&self.data).is_some() => vec![Act::PlayNext],
            "share" => vec![Act::Share(watching.video.id.to_owned(), Some(time))],
            "save" => vec![Act::Save(watching.video.id.to_owned())],
            other => other
                .strip_prefix("at-")
                .and_then(|digit| digit.parse::<f64>().ok())
                .map(|digit| vec![Act::Seek(duration * digit / 10.0)])
                .unwrap_or_default(),
        }
    }

    /// The keys a video answers while its page is open and no field has the keyboard.
    pub fn keys(&mut self, ctx: &egui::Context) -> Vec<Act> {
        if self.watching.is_none() || ctx.egui_wants_keyboard_input() {
            return Vec::new();
        }
        let pressed: Vec<String> = ctx.input(|i| {
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
                        egui::Key::K | egui::Key::Space if !repeat => "toggle".to_owned(),
                        egui::Key::J => "back-10".to_owned(),
                        egui::Key::L => "forward-10".to_owned(),
                        egui::Key::ArrowLeft => "back-5".to_owned(),
                        egui::Key::ArrowRight => "forward-5".to_owned(),
                        egui::Key::M if !repeat => "mute".to_owned(),
                        egui::Key::F if !repeat => "full".to_owned(),
                        egui::Key::C if !repeat => "captions".to_owned(),
                        egui::Key::T if !repeat => "theater".to_owned(),
                        egui::Key::I if !repeat => "stats".to_owned(),
                        egui::Key::N if modifiers.shift && !repeat => "next".to_owned(),
                        other => match other.name().parse::<u8>() {
                            Ok(digit) if !repeat => format!("at-{digit}"),
                            _ => continue,
                        },
                    };
                    keys.push(command);
                }
            }
            keys
        });
        pressed.iter().flat_map(|command| self.video_command(command)).collect()
    }

    /// The palette while it is open, and what its command does.
    pub fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let mut groups: Vec<CommandGroup> = Vec::new();
        if self.watching.is_some() {
            let prefs = &self.data.prefs;
            groups.push(CommandGroup::new(
                "This video",
                [
                    MenuItem::action(if prefs.theater {
                        "Leave theater mode"
                    } else {
                        "Theater mode"
                    })
                    .icon(Icon::RectangleHorizontal)
                    .shortcut("T")
                    .id("video:theater"),
                    MenuItem::action(if prefs.stats {
                        "Hide the stats"
                    } else {
                        "Show the stats"
                    })
                    .icon(Icon::Activity)
                    .shortcut("I")
                    .id("video:stats"),
                    MenuItem::action("Captions on or off")
                        .icon(Icon::Captions)
                        .shortcut("C")
                        .id("video:captions"),
                    MenuItem::action("Share this video")
                        .icon(Icon::Share2)
                        .id("video:share"),
                    MenuItem::action("Save this video")
                        .icon(Icon::Bookmark)
                        .id("video:save"),
                ],
            ));
        }
        groups.extend(place_commands(rail, "Go to"));
        let lists = playlist_commands(&self.data);
        groups.push(CommandGroup::new(
            "Playlists",
            lists.iter().map(|(command, _)| {
                let mut item = MenuItem::action(&command.label)
                    .icon(Icon::ListVideo)
                    .keywords(command.keywords.iter().copied())
                    .id(format!("site:{}", command.id));
                if let Some(hint) = &command.hint {
                    item = item.hint(hint);
                }
                item
            }),
        ));
        let player = player_commands(&self.data);
        let icons = [
            Icon::Play,
            Icon::RectangleHorizontal,
            Icon::Activity,
            Icon::Settings,
            Icon::Trash2,
        ];
        groups.push(CommandGroup::new(
            "Player",
            player.iter().zip(icons).map(|((command, _), icon)| {
                MenuItem::action(&command.label)
                    .icon(icon)
                    .keywords(command.keywords.iter().copied())
                    .id(format!("site:{}", command.id))
            }),
        ));
        groups.push(CommandGroup::new(
            "Videos",
            VIDEOS.iter().map(|video| {
                MenuItem::action(video.title)
                    .icon(Icon::Video)
                    .detail(channel_of(video).name)
                    .hint(format!("{} views", short(video.views as f64)))
                    .keywords([video.description, video.topic.id()])
                    .id(format!("watch:{}", video.id))
            }),
        ));
        groups.push(CommandGroup::new(
            "Channels",
            CHANNELS.iter().map(|channel| {
                MenuItem::action(channel.name)
                    .icon(Icon::Users)
                    .detail(channel.handle)
                    .hint(format!("{} followers", short(channel.followers as f64)))
                    .keywords([channel.about])
                    .id(format!("channel:{}", channel.id))
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
            let acts = if let Some(id) = command.strip_prefix("video:") {
                self.video_command(id)
            } else if let Some(id) = command.strip_prefix("site:") {
                let all = playlist_commands(&self.data)
                    .into_iter()
                    .chain(player_commands(&self.data));
                match all.into_iter().find(|(one, _)| one.id == id).map(|(_, action)| action) {
                    Some(CommandAction::Go(path)) => vec![Act::Go(path)],
                    Some(CommandAction::Prefs(patch)) => vec![Act::Prefs(patch)],
                    None => Vec::new(),
                }
            } else if let Some(id) = command.strip_prefix("watch:") {
                vec![Act::Go(format!("watch/{id}"))]
            } else if let Some(id) = command.strip_prefix("channel:") {
                vec![Act::Go(format!("channel/{id}"))]
            } else if let Some(place) = place_of(command) {
                vec![Act::Go(match place.strip_prefix("channel-") {
                    Some(channel) => format!("channel/{channel}"),
                    None => place.to_owned(),
                })]
            } else {
                run_appearance_command(&mut self.appearance, command);
                Vec::new()
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
