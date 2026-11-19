//! What the pages, the player bar, the queue, the palette and the keys ask of the app, and how it
//! does it, as the web's handlers do: `actions.tsx` with its toasts, the player's calls, and the
//! settings' `setSettings`.

use eframe::egui;
use sokkel_egui::Toast;

use crate::app::MusicApp;
use crate::catalog::StationKind;
use crate::data::{FavoriteKind, Settings, Source};
use crate::dialogs::Asking;
use crate::player::{Place, PlayFrom};
use crate::queue::RepeatMode;

/// One thing asked of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// Go to an address.
    Go(String),
    /// Back to the address before, as a page's back button.
    Back,
    /// Play `songs` from the one at `start` as the queue from `source`; `shuffle` set on or off, or
    /// kept as it is.
    Play {
        songs: Vec<String>,
        start: usize,
        source: Source,
        shuffle: Option<bool>,
    },
    SetPlaying(bool),
    TogglePlay,
    Next,
    Previous,
    Seek(f64),
    SeekBy(f64),
    Shuffle(bool),
    Repeat(RepeatMode),
    Fullscreen(bool),
    /// Plays a station, by its id and name.
    Station(String, String),
    Star(FavoriteKind, String, bool),
    Download(String),
    RemoveDownload(String),
    /// Asks before every download goes.
    AskRemoveAll,
    /// Puts songs in the queue and says so, with what they are.
    Enqueue(Vec<String>, Place, String),
    /// Starts a station from an artist, an album or a song.
    Radio(StationKind, String),
    /// Adds songs at the end of a playlist and says so.
    AddTo(String, Vec<String>, Option<String>),
    Ask(Asking),
    Rename(String, String),
    Describe(String, String),
    MoveInPlaylist(String, usize, usize),
    RemoveFromPlaylist(String, String),
    RemoveStation(String),
    RemoveAllStations,
    /// Plays the queue's entry at a place in its play order.
    PlayPosition(usize),
    Dequeue(usize),
    MoveInQueue(usize, usize),
    ClearUpNext,
    ClearHistory,
    Settings(Settings),
}

/// `1 song`, `3 songs`.
pub fn songs_word(count: usize) -> String {
    format!("{count} {}", if count == 1 { "song" } else { "songs" })
}

impl MusicApp {
    /// Does what was asked.
    pub fn apply(&mut self, ctx: &egui::Context, act: Act) {
        let now = self.now();
        let library = &mut self.library;
        let player = &mut self.player;
        match act {
            Act::Go(path) => self.go(&path),
            Act::Back => self.back(),
            Act::Play {
                songs,
                start,
                source,
                shuffle,
            } => {
                let random = &mut self.random;
                player.play_songs(library, &songs, source, PlayFrom { start, shuffle }, now, &mut || {
                    random.next_f64()
                });
            }
            Act::SetPlaying(on) => player.set_playing(library, on, now),
            Act::TogglePlay => player.toggle_play(library, now),
            Act::Next => player.next(library, now),
            Act::Previous => player.previous(library, now),
            Act::Seek(seconds) => player.seek(library, seconds, now),
            Act::SeekBy(seconds) => player.seek_by(library, seconds, now),
            Act::Shuffle(on) => {
                let random = &mut self.random;
                player.set_shuffle(library, on, now, &mut || random.next_f64());
            }
            Act::Repeat(mode) => player.set_repeat(library, mode, now),
            Act::Fullscreen(open) => self.fullscreen = open,
            Act::Station(id, name) => player.play_station(library, &id, &name, now),
            Act::Star(kind, reference, on) => library.set_favorite(kind, &reference, on, now),
            Act::Download(album) => library.download(&album, now),
            Act::RemoveDownload(album) => library.remove_download(&album),
            Act::AskRemoveAll => self.clearing = true,
            Act::Enqueue(songs, place, what) => {
                player.enqueue(library, &songs, place, now);
                Toast::new(if place == Place::Next {
                    "Playing next"
                } else {
                    "Added to the queue"
                })
                .description(what)
                .timeout(Some(2.5))
                .show(ctx);
            }
            Act::Radio(kind, reference) => {
                if let Some((id, name)) = library.start_radio(kind, &reference, now) {
                    player.play_station(library, &id, &name, now);
                }
            }
            Act::AddTo(id, songs, what) => {
                let Some(name) = library.playlist(&id).map(|p| p.name.clone()) else {
                    return;
                };
                library.add_to_playlist(&id, &songs, now);
                Toast::success(format!("Added to {name}"))
                    .description(what.unwrap_or_else(|| songs_word(songs.len())))
                    .timeout(Some(2.5))
                    .show(ctx);
            }
            Act::Ask(asking) => self.ask(asking),
            Act::Rename(id, name) => library.rename_playlist(&id, &name, now),
            Act::Describe(id, description) => library.describe_playlist(&id, &description, now),
            Act::MoveInPlaylist(id, from, to) => library.move_in_playlist(&id, from, to, now),
            Act::RemoveFromPlaylist(id, key) => library.remove_from_playlist(&id, &key, now),
            Act::RemoveStation(id) => library.stations.retain(|row| row.id != id),
            Act::RemoveAllStations => library.stations.clear(),
            Act::PlayPosition(at) => player.play_position(library, at, now),
            Act::Dequeue(at) => player.dequeue(library, &[at], now),
            Act::MoveInQueue(from, to) => player.move_in_queue(library, from, to, now),
            Act::ClearUpNext => player.clear_up_next(library, now),
            Act::ClearHistory => library.clear_history(),
            Act::Settings(settings) => library.settings = settings,
        }
    }
}
