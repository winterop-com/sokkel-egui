//! What the listener keeps, as the web's `data.ts` keeps it on the fake backend: favorites,
//! playlists, downloads, the stations they made, what they played, what the player was doing and
//! their settings. The web mirrors each collection in a store, changed at once and sent to the
//! backend after; here every change is at once, and the whole `Library` is one serde value the app
//! saves with eframe's storage.
//!
//! Rows are kept in the web's order: a new one first, a changed one in its place. A row the page
//! makes takes the id `<kind>-<now in base 36>-<count>`, the count running over every row made.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sokkel_egui_media::ArtworkCover;

use crate::catalog::{self, Station, StationKind, cover_serde, song_of, songs, songs_of_album, station_from};
use crate::js;
use crate::queue::RepeatMode;
use crate::synth::Style;

/// The moment the seeded rows were made: 2026-11-01 09:00 UTC.
pub const STARTED: i64 = 1_793_523_600_000;

// Favorites

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FavoriteKind {
    Song,
    Album,
    Artist,
}

impl FavoriteKind {
    pub fn key(self) -> &'static str {
        match self {
            FavoriteKind::Song => "song",
            FavoriteKind::Album => "album",
            FavoriteKind::Artist => "artist",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Favorite {
    pub id: String,
    pub kind: FavoriteKind,
    #[serde(rename = "ref")]
    pub reference: String,
    pub at: i64,
}

fn seed_favorites() -> Vec<Favorite> {
    let row = |kind: FavoriteKind, reference: &str, at: i64| Favorite {
        id: format!("{}:{reference}", kind.key()),
        kind,
        reference: reference.to_owned(),
        at,
    };
    let mut rows: Vec<Favorite> = [
        "open-water-1-2",
        "open-water-1-4",
        "northern-coast-1-1",
        "high-desert-1-3",
        "glass-harbor-1-2",
        "sea-glass-1-1",
        "pilot-light-1-1",
    ]
    .iter()
    .enumerate()
    .map(|(index, reference)| row(FavoriteKind::Song, reference, STARTED - index as i64 * 3_600_000))
    .collect();
    rows.extend(
        ["open-water", "archipelago", "estuary"]
            .iter()
            .enumerate()
            .map(|(index, reference)| row(FavoriteKind::Album, reference, STARTED - index as i64 * 7_200_000)),
    );
    rows.extend(
        ["grace", "eelgrass"]
            .iter()
            .enumerate()
            .map(|(index, reference)| row(FavoriteKind::Artist, reference, STARTED - index as i64 * 9_000_000)),
    );
    rows
}

// Playlists

/// One place on a playlist: the same song may stand on it twice, each its own entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlaylistEntry {
    pub key: String,
    pub song: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: String,
    pub entries: Vec<PlaylistEntry>,
    pub updated: i64,
}

/// A change to a playlist: each field given is set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlaylistPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub entries: Option<Vec<PlaylistEntry>>,
}

fn pick_songs(step: usize, count: usize, from: usize) -> Vec<String> {
    let all = songs();
    (0..count)
        .map(|index| all[(from + index * step) % all.len()].id.clone())
        .collect()
}

fn entries_of(songs: &[String]) -> Vec<PlaylistEntry> {
    songs
        .iter()
        .enumerate()
        .map(|(index, song)| PlaylistEntry {
            key: format!("{song}~{index}"),
            song: song.clone(),
        })
        .collect()
}

fn seed_playlists() -> Vec<Playlist> {
    let playlist = |id: &str, name: &str, description: &str, songs: Vec<String>, updated: i64| Playlist {
        id: id.to_owned(),
        name: name.to_owned(),
        description: description.to_owned(),
        entries: entries_of(&songs),
        updated,
    };
    vec![
        playlist(
            "morning-run",
            "Morning run",
            "Fast songs for the coast road.",
            pick_songs(11, 14, 3),
            STARTED,
        ),
        playlist(
            "deep-focus",
            "Deep focus",
            "No words, nothing sudden.",
            pick_songs(17, 12, 40),
            STARTED - 86_400_000,
        ),
        playlist(
            "long-drive",
            "Long drive",
            "",
            pick_songs(7, 20, 9),
            STARTED - 2 * 86_400_000,
        ),
        playlist(
            "sunday-slow",
            "Sunday slow",
            "For the slow end of the week.",
            pick_songs(23, 9, 61),
            STARTED - 5 * 86_400_000,
        ),
    ]
}

// Downloads

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadState {
    Running,
    Kept,
}

/// An album kept on the device, or coming in; its id is the album's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Download {
    pub id: String,
    pub state: DownloadState,
    /// How many of the album's songs are in.
    pub done: u32,
    pub total: u32,
    pub bytes: f64,
    pub at: i64,
}

fn seed_downloads() -> Vec<Download> {
    ["northern-coast", "sea-glass", "pilot-light"]
        .iter()
        .enumerate()
        .map(|(index, album)| {
            let songs = songs_of_album(album);
            Download {
                id: (*album).to_owned(),
                state: DownloadState::Kept,
                done: songs.len() as u32,
                total: songs.len() as u32,
                bytes: songs
                    .iter()
                    .fold(0.0, |sum, song| sum + js::round(song.duration * 44_100.0 + 44.0)),
                at: STARTED - index as i64 * 86_400_000,
            }
        })
        .collect()
}

// Stations

/// What a made station was started from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationFrom {
    pub kind: StationKind,
    #[serde(rename = "ref")]
    pub reference: String,
}

/// A station the listener started from an artist, an album or a song.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MadeStation {
    pub id: String,
    pub name: String,
    pub style: Style,
    pub about: String,
    #[serde(with = "cover_serde")]
    pub cover: ArtworkCover,
    pub artists: Vec<String>,
    pub from: StationFrom,
    pub at: i64,
}

impl MadeStation {
    /// The station it is, for its songs to be made from.
    pub fn station(&self) -> Station {
        Station {
            id: self.id.clone(),
            name: self.name.clone(),
            style: self.style,
            about: self.about.clone(),
            cover: self.cover,
            artists: self.artists.clone(),
        }
    }
}

// History

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Played {
    pub id: String,
    pub song: String,
    pub at: i64,
}

fn seed_history() -> Vec<Played> {
    ["glass-harbor-1-1", "lowlands-1-3", "shoals-1-2", "open-water-1-1"]
        .iter()
        .enumerate()
        .map(|(index, song)| Played {
            id: format!("played-seed-{index}"),
            song: (*song).to_owned(),
            at: STARTED - (index as i64 + 1) * 600_000,
        })
        .collect()
}

/// How many plays the history keeps.
pub const HISTORY: usize = 60;

// The player, as it was left

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Album,
    Artist,
    Playlist,
    Favorites,
    Tracks,
    Radio,
    Search,
    History,
}

/// Where what plays came from, for the queue's "Playing from" and a way back to it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub kind: SourceKind,
    pub label: String,
    /// The route back to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

impl Source {
    pub fn new(kind: SourceKind, label: impl Into<String>, href: Option<String>) -> Self {
        Self {
            kind,
            label: label.into(),
            href,
        }
    }
}

/// One place in the player's queue: a song, keyed so the same song may stand twice.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueueEntry {
    pub key: String,
    pub song: String,
}

/// The station playing, and how many songs it has made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Radio {
    pub station: String,
    pub made: u64,
}

/// What the player was doing, kept so a new session picks up where it was.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerRow {
    pub id: String,
    pub entries: Vec<QueueEntry>,
    pub order: Vec<i64>,
    pub at: i64,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub position: f64,
    pub source: Option<Source>,
    pub radio: Option<Radio>,
}

impl Default for PlayerRow {
    /// The first album, ready to play.
    fn default() -> Self {
        let first_album = &catalog::albums()[0];
        let first = songs_of_album(&first_album.id);
        Self {
            id: "now".to_owned(),
            entries: first
                .iter()
                .map(|song| QueueEntry {
                    key: song.id.clone(),
                    song: song.id.clone(),
                })
                .collect(),
            order: (0..first.len() as i64).collect(),
            at: 0,
            shuffle: false,
            repeat: RepeatMode::Off,
            position: 0.0,
            source: Some(Source::new(
                SourceKind::Album,
                first_album.title.clone(),
                Some(format!("/albums/{}", first_album.id)),
            )),
            radio: None,
        }
    }
}

// Settings

/// The visualizer's look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Look {
    #[default]
    Spectrum,
    Mirror,
    Area,
    Wave,
    Ring,
}

/// The visualizer's colors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ramp {
    #[default]
    Theme,
    Heat,
    Cool,
    Lagoon,
    Ink,
}

/// What the visualizer draws over itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    #[default]
    None,
    Glow,
    Trails,
}

/// How the player bar is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    #[default]
    Rows,
    Panel,
    Stage,
}

/// How the albums page orders them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlbumOrder {
    #[default]
    Recent,
    Title,
    Artist,
    Year,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub id: String,
    pub volume: f64,
    pub muted: bool,
    pub rate: f64,
    /// Seconds one song fades into the next; 0 plays them back to back.
    pub crossfade: f64,
    pub layout: Layout,
    /// The bar's height in pixels, as it was left; none for the one line.
    pub bar_size: Option<f64>,
    /// Whether the visualizer rises behind the grown bar.
    pub bar_visualizer: bool,
    pub look: Look,
    pub ramp: Ramp,
    pub effect: Effect,
    /// Milliseconds the picture waits for the output, for a slow speaker.
    pub delay: f64,
    /// Whether the fullscreen view shows the words in place of the picture.
    pub lyrics: bool,
    pub queue_open: bool,
    /// Whether a new session picks up where the last one stopped.
    pub resume: bool,
    pub album_order: AlbumOrder,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: "settings".to_owned(),
            volume: 0.8,
            muted: false,
            rate: 1.0,
            crossfade: 0.0,
            layout: Layout::Rows,
            bar_size: None,
            bar_visualizer: true,
            look: Look::Spectrum,
            ramp: Ramp::Theme,
            effect: Effect::None,
            delay: 0.0,
            lyrics: false,
            queue_open: false,
            resume: true,
            album_order: AlbumOrder::Recent,
        }
    }
}

// The library

/// Everything the listener keeps, as the app saves it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub favorites: Vec<Favorite>,
    pub playlists: Vec<Playlist>,
    pub downloads: Vec<Download>,
    pub stations: Vec<MadeStation>,
    pub history: Vec<Played>,
    pub player: PlayerRow,
    pub settings: Settings,
    /// How many rows the page has made, for the next one's id.
    pub made: u64,
    /// When the downloads next move on, while one is running (the web's interval).
    #[serde(skip)]
    pub(crate) download_tick: Option<i64>,
}

impl Default for Library {
    /// The library as a new listener starts with it.
    fn default() -> Self {
        Self {
            favorites: seed_favorites(),
            playlists: seed_playlists(),
            downloads: seed_downloads(),
            stations: Vec::new(),
            history: seed_history(),
            player: PlayerRow::default(),
            settings: Settings::default(),
            made: 0,
            download_tick: None,
        }
    }
}

impl Library {
    /// A fresh id for a row the page makes.
    pub fn new_id(&mut self, kind: &str, now: i64) -> String {
        self.made += 1;
        format!("{kind}-{}-{}", js::base36(now), self.made)
    }

    /// Stars or unstars a song, an album or an artist.
    pub fn set_favorite(&mut self, kind: FavoriteKind, reference: &str, on: bool, now: i64) {
        let id = format!("{}:{reference}", kind.key());
        self.favorites.retain(|one| one.id != id);
        if on {
            self.favorites.insert(
                0,
                Favorite {
                    id,
                    kind,
                    reference: reference.to_owned(),
                    at: now,
                },
            );
        }
    }

    /// The starred ones of a kind, by their ids.
    pub fn starred(&self, kind: FavoriteKind) -> HashSet<&str> {
        self.favorites
            .iter()
            .filter(|row| row.kind == kind)
            .map(|row| row.reference.as_str())
            .collect()
    }

    /// Whether one is starred.
    pub fn is_starred(&self, kind: FavoriteKind, reference: &str) -> bool {
        self.favorites
            .iter()
            .any(|row| row.kind == kind && row.reference == reference)
    }

    pub fn playlist(&self, id: &str) -> Option<&Playlist> {
        self.playlists.iter().find(|row| row.id == id)
    }

    /// Changes a playlist, and marks it changed now.
    pub fn change_playlist(&mut self, id: &str, patch: PlaylistPatch, now: i64) {
        if let Some(row) = self.playlists.iter_mut().find(|row| row.id == id) {
            if let Some(name) = patch.name {
                row.name = name;
            }
            if let Some(description) = patch.description {
                row.description = description;
            }
            if let Some(entries) = patch.entries {
                row.entries = entries;
            }
            row.updated = now;
        }
    }

    /// Makes a playlist holding `songs`, and answers its id.
    pub fn create_playlist(&mut self, name: &str, songs: &[String], now: i64) -> String {
        let id = self.new_id("playlist", now);
        let entries = entries_of(songs)
            .into_iter()
            .map(|entry| PlaylistEntry {
                key: format!("{}~{id}", entry.key),
                song: entry.song,
            })
            .collect();
        self.playlists.insert(
            0,
            Playlist {
                id: id.clone(),
                name: name.to_owned(),
                description: String::new(),
                entries,
                updated: now,
            },
        );
        id
    }

    pub fn delete_playlist(&mut self, id: &str) {
        self.playlists.retain(|row| row.id != id);
    }

    /// Adds songs at the end of a playlist.
    pub fn add_to_playlist(&mut self, id: &str, songs: &[String], now: i64) {
        let Some(playlist) = self.playlist(id) else { return };
        let stamp = js::base36(now);
        let mut entries = playlist.entries.clone();
        entries.extend(songs.iter().enumerate().map(|(index, song)| PlaylistEntry {
            key: format!("{song}~{stamp}~{index}"),
            song: song.clone(),
        }));
        self.change_playlist(
            id,
            PlaylistPatch {
                entries: Some(entries),
                ..PlaylistPatch::default()
            },
            now,
        );
    }

    /// Renames a playlist, as its page's title is edited: the name trimmed.
    pub fn rename_playlist(&mut self, id: &str, name: &str, now: i64) {
        let name = Some(js::trim(name).to_owned());
        self.change_playlist(
            id,
            PlaylistPatch {
                name,
                ..PlaylistPatch::default()
            },
            now,
        );
    }

    /// Says what a playlist is for, as its page's line under the title is edited: trimmed.
    pub fn describe_playlist(&mut self, id: &str, description: &str, now: i64) {
        let description = Some(js::trim(description).to_owned());
        self.change_playlist(
            id,
            PlaylistPatch {
                description,
                ..PlaylistPatch::default()
            },
            now,
        );
    }

    /// The playlist's entries whose songs there are, as its page lists them.
    pub fn playlist_entries(&self, id: &str) -> Vec<PlaylistEntry> {
        self.playlist(id).map_or_else(Vec::new, |playlist| {
            playlist
                .entries
                .iter()
                .filter(|entry| song_of(&entry.song, &self.stations).is_some())
                .cloned()
                .collect()
        })
    }

    /// Moves a song on a playlist's page from `from` to `to`, as its grip is dragged.
    pub fn move_in_playlist(&mut self, id: &str, from: usize, to: usize, now: i64) {
        let entries = self.playlist_entries(id);
        if from >= entries.len() || self.playlist(id).is_none() {
            return;
        }
        let next = crate::queue::moved(&entries, from, to);
        self.change_playlist(
            id,
            PlaylistPatch {
                entries: Some(next),
                ..PlaylistPatch::default()
            },
            now,
        );
    }

    /// Takes the entry under `key` off a playlist.
    pub fn remove_from_playlist(&mut self, id: &str, key: &str, now: i64) {
        let Some(playlist) = self.playlist(id) else { return };
        let entries = playlist
            .entries
            .iter()
            .filter(|entry| entry.key != key)
            .cloned()
            .collect();
        self.change_playlist(
            id,
            PlaylistPatch {
                entries: Some(entries),
                ..PlaylistPatch::default()
            },
            now,
        );
    }

    /// Tells the history a song started.
    pub fn played(&mut self, song: &str, now: i64) {
        if self.history.first().is_some_and(|row| row.song == song) {
            return;
        }
        let id = self.new_id("played", now);
        self.history.insert(
            0,
            Played {
                id,
                song: song.to_owned(),
                at: now,
            },
        );
        self.history.truncate(HISTORY);
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Starts a station from an artist, an album or a song and keeps it among the listener's
    /// stations, or finds the one made from it before: its id and its name, for the player to play.
    pub fn start_radio(&mut self, kind: StationKind, reference: &str, now: i64) -> Option<(String, String)> {
        let made = station_from(kind, reference)?;
        if let Some(existing) = self
            .stations
            .iter()
            .find(|row| row.from.kind == kind && row.from.reference == reference)
        {
            return Some((existing.id.clone(), made.name));
        }
        let id = self.new_id("station", now);
        self.stations.insert(
            0,
            MadeStation {
                id: id.clone(),
                name: made.name.clone(),
                style: made.style,
                about: made.about,
                cover: made.cover,
                artists: made.artists,
                from: StationFrom {
                    kind,
                    reference: reference.to_owned(),
                },
                at: now,
            },
        );
        Some((id, made.name))
    }

    /// Whether an album is kept or coming in.
    pub fn download_of(&self, album: &str) -> Option<&Download> {
        self.downloads.iter().find(|row| row.id == album)
    }
}
