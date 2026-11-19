//! What the pages, the player bar, the rail, the palette and the keys ask of the app, and how it
//! does it: each the provider's action of the same name in the web's `state.tsx`, the toasts it
//! says, and the questions asked before what cannot be undone.

use eframe::egui;
use sokkel_egui::Toast;

use crate::app::PodcastApp;
use crate::backend::PrefsPatch;
use crate::sleep_timer::SleepTimer;
use crate::state::Where;

/// A question asked in a dialog before it is done.
#[derive(Clone, Debug, PartialEq)]
pub enum Asking {
    ClearQueue,
    RemoveDownloads,
    /// Unfollow the show of this id.
    Unfollow(String),
    ClearStats,
    Reset,
}

/// One thing asked of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// Go to an address.
    Go(String),
    /// Play an episode, from a second or where it picks up, or play and pause the one playing.
    Play(String, Option<f64>),
    SetPlaying(bool),
    TogglePlay,
    Seek(f64),
    Skip(f64),
    Rate(f64),
    Volume(f64),
    Muted(bool),
    PlayNext,
    QueueAdd(String, Where),
    QueueRemove(String),
    QueueMove(usize, usize),
    QueueClear,
    MarkPlayed(String, bool),
    /// Marks every unplayed episode of a show played.
    MarkAllPlayed(Vec<String>),
    Star(String, bool),
    Archive(String),
    ArchiveAll,
    Download(String),
    RemoveDownload(String),
    RemoveAllDownloads,
    CheckForNew,
    Subscribe(String),
    Unsubscribe(String),
    Prefs(PrefsPatch),
    Sleep(Option<SleepTimer>),
    Fullscreen(bool),
    /// Reads an episode's summary aloud, or stops it.
    Speak(String),
    ClearStats,
    Ask(Asking),
}

impl PodcastApp {
    /// Does what was asked.
    pub fn apply(&mut self, ctx: &egui::Context, act: Act) {
        let state = &mut self.state;
        match act {
            Act::Go(path) => self.go(&path),
            Act::Play(id, at) => state.play(&id, at),
            Act::SetPlaying(on) => state.set_playing(on),
            Act::TogglePlay => {
                let on = !state.playing;
                state.set_playing(on);
            }
            Act::Seek(seconds) => state.seek(seconds),
            Act::Skip(by) => state.skip(by),
            Act::Rate(rate) => state.set_rate(rate),
            Act::Volume(volume) => state.set_volume(volume),
            Act::Muted(muted) => state.set_muted(muted),
            Act::PlayNext => {
                state.play_next();
            }
            Act::QueueAdd(id, place) => state.queue_add(&id, place),
            Act::QueueRemove(id) => state.queue_remove(&id),
            Act::QueueMove(from, to) => state.queue_move(from as i64, to as i64),
            Act::QueueClear => state.queue_clear(),
            Act::MarkPlayed(id, played) => state.mark_played(&id, played),
            Act::MarkAllPlayed(ids) => {
                for id in ids {
                    state.mark_played(&id, true);
                }
            }
            Act::Star(id, on) => state.star(&id, on),
            Act::Archive(id) => state.archive(&id),
            Act::ArchiveAll => state.archive_all(),
            Act::Download(id) => state.download(&id),
            Act::RemoveDownload(id) => state.remove_download(&id),
            Act::RemoveAllDownloads => state.remove_all_downloads(),
            Act::CheckForNew => {
                if self.checking.is_none() {
                    self.checking = Some(state.checked.len());
                    state.check_for_new();
                }
            }
            Act::Subscribe(id) => state.subscribe(&id),
            Act::Unsubscribe(id) => state.unsubscribe(&id),
            Act::Prefs(patch) => state.set_prefs(&patch),
            Act::Sleep(timer) => state.set_sleep(timer),
            Act::Fullscreen(open) => self.fullscreen = open,
            Act::Speak(id) => self.speak(&id),
            Act::ClearStats => {
                state.clear_stats();
                Toast::new("Stats cleared").timeout(Some(2.5)).show(ctx);
            }
            Act::Ask(asking) => self.asking = Some(asking),
        }
    }
}
