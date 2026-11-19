//! What the pages, the player, the rail, the palette, the dialogs and the keys ask of the app, and
//! how it does it: each the web's handler of the same name, on the data layer's actions.

use eframe::egui;

use crate::app::VideoApp;
use crate::catalog::{channel_by_id, video_by_id};
use crate::data::{PrefsPatch, Quality, RatingKind, Toast};
use crate::pages;

/// A question asked in a dialog before it is done.
#[derive(Clone, Debug, PartialEq)]
pub enum Asking {
    /// Clear watch history; opened from the palette or the settings at `history/clear`.
    ClearHistory {
        clearing: bool,
    },
    /// Delete the playlist of this id.
    DeletePlaylist(String),
    Reset,
}

/// One thing asked of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// Go to an address, as the web's hash route.
    Go(String),
    Prefs(PrefsPatch),
    /// Follow a channel or stop, said at once.
    Follow(String, bool),
    /// Like or dislike the video playing, or neither.
    Rate(Option<RatingKind>),
    /// Save the video to Watch later, or take it out.
    WatchLater(String),
    /// Open Save to for a video.
    Save(String),
    /// Open Share for a video, starting at a second when given.
    Share(String, Option<f64>),
    /// Tick a playlist in Save to, or untick it.
    SaveToggle(String, String, bool),
    /// Make a playlist from Save to, with the video in it.
    SaveNew(String, String),
    /// Make a playlist from Playlists' field.
    NewPlaylist(String),
    PlayAll(String),
    Shuffle(String),
    RenamePlaylist(String, String),
    DescribePlaylist(String, String),
    /// Move a playlist's video at an index by one place, up (-1) or down (1).
    MoveInPlaylist(String, usize, i64),
    RemoveFromPlaylist(String, String),
    DeletePlaylist(String),
    RemoveFromHistory(String),
    ClearHistory(bool),
    Unlike(String),
    RemoveDownload(String),
    /// The player: play or pause.
    SetPlaying(bool),
    Seek(f64),
    Volume(f32),
    Muted(bool),
    Rate2(f64),
    Quality(Quality),
    Captions(bool),
    Theater,
    Stats,
    Fullscreen(bool),
    PictureInPicture(bool),
    /// Up next's Play now, or its countdown run out.
    PlayNext,
    /// Up next's Stay here.
    Stay,
    Download,
    CancelDownload,
    RemoveThisDownload,
    /// Comments: a new one or a reply, a like or dislike, a reaction, delete and report.
    Comment(String, Option<String>),
    RateComment(String, bool),
    React(String, String, String),
    DeleteComment(String),
    ReportComment(String),
    Unreport(String),
    Ask(Asking),
    ResetDemo,
    /// A note said at once.
    Toast(String),
    /// The settings closed, as a row that goes elsewhere closes them.
    CloseSettings,
}

impl VideoApp {
    /// Does what was asked.
    pub fn apply(&mut self, _ctx: &egui::Context, act: Act) {
        match act {
            Act::Go(path) => self.go(&path),
            Act::Prefs(patch) => self.data.set_prefs(&patch),
            Act::Follow(id, on) => {
                if let Some(channel) = channel_by_id(&id) {
                    pages::search::follow(&mut self.data, channel, on);
                }
            }
            Act::Rate(rating) => {
                if let Some(watching) = &self.watching {
                    watching.rate(&mut self.data, rating);
                }
            }
            Act::WatchLater(id) => {
                if let Some(video) = video_by_id(&id) {
                    crate::parts::toggle_watch_later(&mut self.data, video);
                }
            }
            Act::Save(id) => {
                self.saving = Some(id);
                self.views.save_name.clear();
            }
            Act::Share(id, at) => self.sharing = Some((id, at, false)),
            Act::SaveToggle(list, id, on) => {
                if let Some(video) = video_by_id(&id) {
                    crate::dialogs::save_toggle(&mut self.data, &list, video, on);
                }
            }
            Act::SaveNew(name, id) => {
                if let Some(video) = video_by_id(&id) {
                    crate::dialogs::save_new(&mut self.data, &name, video);
                }
            }
            Act::NewPlaylist(name) => {
                let id = pages::playlists::add(&mut self.data, &name);
                self.toast_acts.push_back(Act::Go(format!("playlist/{id}")));
            }
            Act::PlayAll(id) => pages::playlist::play_all(&mut self.data, &id),
            Act::Shuffle(id) => pages::playlist::shuffle(&mut self.data, &id),
            Act::RenamePlaylist(id, name) => pages::playlist::rename(&mut self.data, &id, &name),
            Act::DescribePlaylist(id, words) => pages::playlist::describe(&mut self.data, &id, &words),
            Act::MoveInPlaylist(id, index, by) => pages::playlist::move_by(&mut self.data, &id, index, by),
            Act::RemoveFromPlaylist(id, video) => pages::playlist::remove(&mut self.data, &id, &video),
            Act::DeletePlaylist(id) => {
                if self.path.first().is_some_and(|first| first == "playlist") {
                    pages::playlist::delete(&mut self.data, &id);
                } else if let Some(list) = self.data.playlists.seen_get(&id).cloned() {
                    pages::playlists::delete(&mut self.data, &list);
                }
            }
            Act::RemoveFromHistory(id) => pages::history::remove(&mut self.data, &id),
            Act::ClearHistory(clearing) => pages::history::clear(&mut self.data, clearing),
            Act::Unlike(id) => pages::liked::remove(&mut self.data, &id),
            Act::RemoveDownload(id) => pages::downloads::remove(&mut self.data, &id),
            Act::SetPlaying(on) => self.set_playing(on),
            Act::Seek(at) => self.seek(at),
            Act::Volume(level) => {
                self.player.volume = level;
                if level > 0.0 {
                    self.player.muted = false;
                }
            }
            Act::Muted(muted) => self.player.muted = muted,
            Act::Rate2(rate) => {
                self.player.rate = rate;
                if let Some(watching) = &self.watching {
                    watching.rate_changed(&mut self.data, rate);
                }
            }
            Act::Quality(quality) => {
                let player = self.player_state();
                if let Some(watching) = &mut self.watching {
                    watching.choose_quality(&mut self.data, quality, player);
                    self.player.attached = false;
                }
            }
            Act::Captions(on) => pages::settings::show_captions(&mut self.data, on),
            Act::Theater => {
                let on = !self.data.prefs.theater;
                self.data.set_prefs(&PrefsPatch {
                    theater: Some(on),
                    ..PrefsPatch::default()
                });
            }
            Act::Stats => {
                let on = !self.data.prefs.stats;
                self.data.set_prefs(&PrefsPatch {
                    stats: Some(on),
                    ..PrefsPatch::default()
                });
            }
            Act::Fullscreen(on) => self.fullscreen = on,
            Act::PictureInPicture(on) => self.picture_in_picture = on,
            Act::PlayNext => {
                if let Some(watching) = &self.watching {
                    watching.play_next(&mut self.data);
                }
            }
            Act::Stay => {
                if let Some(watching) = &mut self.watching {
                    watching.stay();
                }
                self.player.ended_at = None;
            }
            Act::Download => {
                if let Some(watching) = &self.watching {
                    watching.download(&mut self.data);
                }
            }
            Act::CancelDownload => {
                if let Some(watching) = &self.watching {
                    watching.cancel_download(&mut self.data);
                    self.data.toast(Toast::plain("Download cancelled"));
                }
            }
            Act::RemoveThisDownload => {
                if let Some(watching) = &self.watching {
                    watching.remove_download(&mut self.data);
                }
            }
            Act::Comment(text, parent) => {
                if let Some(watching) = &self.watching {
                    pages::watch::comments::add(&mut self.data, watching.video, &text, parent.as_deref());
                }
            }
            Act::RateComment(id, liked) => {
                if let Some(watching) = &self.watching {
                    pages::watch::comments::rate(&mut self.data, watching.video, &id, liked);
                }
            }
            Act::React(id, emoji, name) => {
                if let Some(watching) = &self.watching {
                    pages::watch::comments::react(&mut self.data, watching.video, &id, &emoji, &name);
                }
            }
            Act::DeleteComment(id) => {
                if let Some(watching) = &self.watching {
                    pages::watch::comments::remove(&mut self.data, watching.video, &id);
                }
            }
            Act::ReportComment(id) => {
                pages::watch::comments::report(&mut self.data, &id);
                self.toast_acts.push_back(Act::Unreport(id));
            }
            Act::Unreport(id) => pages::watch::comments::unreport(&mut self.data, &id),
            Act::Ask(asking) => self.asking = Some(asking),
            Act::ResetDemo => self.reset_demo(),
            Act::Toast(title) => self.data.toast(Toast::plain(title)),
            Act::CloseSettings => self.settings = None,
        }
    }
}
