//! The queue beside the page, as the web's `parts/QueuePanel.tsx`: Queue and History as tabs; what
//! plays now, what comes after it in the order it plays, dragged into another order or taken out,
//! where it came from, and what was played lately.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Icon, IconButton, MetaLine, SidePanel, TextLink, TextRole, format_time,
    panel_section, panel_section_actions, tokens,
};
use sokkel_egui_media::{Artwork, ReorderableTrackList, Track, TrackList, TrackListVariant};

use crate::act::{Act, songs_word};
use crate::catalog::{Song, album_of, artist_name, length_of_songs, minutes, song_of};
use crate::data::{Source, SourceKind};
use crate::pages::Here;
use crate::songs::{as_track, nothing};

/// What plays now: its cover, title, artist and album, and how far along it is.
fn now_playing(ui: &mut Ui, here: &Here) {
    let Some(song) = here.player.current_song(here.library) else {
        return;
    };
    let t = tokens(ui.ctx());
    let album = album_of(&song, &here.library.stations);
    panel_section(ui, "Now playing", |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            ui.add(Artwork::of(album.cover).size(egui::vec2(56.0, 56.0)).framed(true));
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add(
                    egui::Label::new(
                        TextRole::Body
                            .rich(&t, &song.title)
                            .variation("wght", 500.0)
                            .color(t.color("foreground")),
                    )
                    .truncate(),
                );
                MetaLine::new([artist_name(&song.artist), album.title.clone()])
                    .truncate()
                    .show(ui);
                let at = here.player.position;
                let time = if here.player.radio.is_none() {
                    format!("{} of {}", format_time(at), format_time(song.duration))
                } else {
                    format_time(at)
                };
                let spec = t.text(TextRole::Help.key());
                let font = sokkel_egui::mono_font(ui.ctx(), sokkel_egui::mono_size(&t, spec.size), spec.weight);
                ui.label(egui::RichText::new(time).font(font).color(t.color("faint")));
            });
        });
    });
}

/// The queue: what plays now, what comes after it, and where it came from.
fn queue_tab(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let player = here.player;
    now_playing(ui, here);
    let next = player.up_next();
    let songs: Vec<Song> = next
        .iter()
        .filter_map(|(entry, _)| song_of(&entry.song, &here.library.stations))
        .collect();
    let first = next.first().map_or(0, |(_, at)| *at);
    let title = if next.is_empty() {
        "Up next".to_owned()
    } else {
        format!(
            "Up next, {}, {}",
            songs_word(next.len()),
            minutes(length_of_songs(&songs))
        )
    };
    let clearable = !next.is_empty() && player.radio.is_none();
    let mut clear = false;
    panel_section_actions(
        ui,
        &title,
        |ui| {
            if clearable
                && ui
                    .add(Button::new("Clear").variant(ButtonVariant::Ghost).size(ButtonSize::Xs))
                    .clicked()
            {
                clear = true;
            }
        },
        |ui| {
            if next.is_empty() {
                let t = tokens(ui.ctx());
                ui.label(
                    TextRole::Body
                        .rich(
                            &t,
                            "Nothing after this song. Add songs from their menus, or play an album.",
                        )
                        .color(t.color("muted-foreground")),
                );
                return;
            }
            let tracks: Vec<Track> = next
                .iter()
                .map(|(entry, _)| match song_of(&entry.song, &here.library.stations) {
                    Some(song) => Track {
                        id: entry.key.clone(),
                        ..as_track(here, &song, true, false)
                    },
                    None => Track::new(&entry.key, &entry.song, 0.0),
                })
                .collect();
            let mut removed = None;
            let out = ReorderableTrackList::new(
                TrackList::new("queue", &tracks)
                    .variant(TrackListVariant::Plain)
                    .follow(false)
                    .actions(|ui, _, index| {
                        if ui.add(IconButton::new(Icon::X, "Remove from the queue")).clicked() {
                            removed = Some(index);
                        }
                    }),
            )
            .show(ui);
            if let Some(index) = removed {
                acts.push(Act::Dequeue(next[index].1));
            }
            if let Some(index) = out.played {
                acts.push(Act::PlayPosition(next[index].1));
            }
            if let Some((from, to)) = out.moved {
                acts.push(Act::MoveInQueue(first + from, first + to));
            }
        },
    );
    if clear {
        acts.push(Act::ClearUpNext);
    }
    if let Some(source) = &player.source {
        panel_section(ui, "Playing from", |ui| match &source.href {
            None => {
                let t = tokens(ui.ctx());
                ui.label(TextRole::Body.rich(&t, &source.label).color(t.color("foreground")));
            }
            Some(href) => {
                if ui.add(TextLink::new(&source.label)).clicked() {
                    acts.push(Act::Go(href.clone()));
                }
            }
        });
    }
}

/// What was played lately, newest first, each played again from a press.
fn history_tab(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let rows: Vec<(String, Song)> = here
        .library
        .history
        .iter()
        .filter_map(|row| song_of(&row.song, &here.library.stations).map(|song| (row.id.clone(), song)))
        .collect();
    if rows.is_empty() {
        panel_section(ui, "Played lately", |ui| {
            nothing(
                ui,
                Icon::History,
                "Nothing played yet",
                "What you play is listed here.",
                None,
            );
        });
        return;
    }
    let mut clear = false;
    panel_section_actions(
        ui,
        "Played lately",
        |ui| {
            if ui
                .add(Button::new("Clear").variant(ButtonVariant::Ghost).size(ButtonSize::Xs))
                .clicked()
            {
                clear = true;
            }
        },
        |ui| {
            let tracks: Vec<Track> = rows
                .iter()
                .map(|(id, song)| Track {
                    id: id.clone(),
                    ..as_track(here, song, true, false)
                })
                .collect();
            let out = TrackList::new("history", &tracks)
                .variant(TrackListVariant::Plain)
                .follow(false)
                .show(ui);
            if let Some(index) = out.played {
                acts.push(Act::Play {
                    songs: rows.iter().map(|(_, song)| song.id.clone()).collect(),
                    start: index,
                    source: Source::new(SourceKind::History, "Played lately", None),
                    shuffle: None,
                });
            }
        },
    );
    if clear {
        acts.push(Act::ClearHistory);
    }
}

/// The queue beside the page: Queue and History as tabs.
pub fn queue_panel(ui: &mut Ui, here: &Here, tab: &mut usize, acts: &mut Vec<Act>) {
    SidePanel::new("queue-panel", &["Queue", "History"], tab).show(ui, |active, ui| {
        if active == 0 {
            queue_tab(ui, here, acts);
        } else {
            history_tab(ui, here, acts);
        }
    });
}
