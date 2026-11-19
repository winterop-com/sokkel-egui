//! One video playing, as the web's `pages/Watch.tsx` keeps it: where the player starts, what is
//! kept of where the viewer is as it plays, the quality, the download, the rating and Follow, the
//! stats over the picture, and what plays next when it ends. The player itself is the app's; it
//! tells `Watching` what happens (`playing_changed`, `time`, `ended`).

pub mod aside;
pub mod comments;

use serde::Serialize;

use crate::catalog::{TOPICS, Video, channel_of, video_by_id};
use crate::clips::{RENDITIONS, Rendition, clip_of, rendition_of};
use crate::data::{CaptionTrack, Data, Playlist, PrefsPatch, Quality, RatingKind, Toast, WatchedPatch};
use crate::format::{bitrate_of, bytes_of, days_ago, views_of};
use crate::js::{round, to_fixed};
use crate::parts::{path_of, view_count, watch_href};

/// How long Up next counts before it plays the next video by itself, in seconds.
pub const COUNTDOWN: f64 = 8.0;

/// How often the viewer's place is kept while playing, in seconds of the video.
pub const KEEP_EVERY: f64 = 4.0;

/// The rendition Auto plays: 480p where the player is wide enough, 360p on a phone.
pub fn pick_auto(width: f32) -> &'static Rendition {
    rendition_of(if width >= 700.0 { "480" } else { "360" })
}

/// One video's page, while it is open: what it keeps of its own between the player's events.
#[derive(Clone, Debug, PartialEq)]
pub struct Watching {
    pub video: &'static Video,
    /// The playlist it plays inside, by id.
    pub list: Option<String>,
    /// The second the address asks to start at.
    pub at: Option<f64>,
    /// The rendition Auto plays here, chosen as the page opened.
    pub auto: &'static Rendition,
    /// Where a change of quality left off.
    pub switched_at: Option<f64>,
    /// Whether the player plays as the clip attaches.
    pub play_on_load: bool,
    pub playing: bool,
    /// Whether the video played to its end, until it plays again or Up next is cancelled.
    pub ended: bool,
    /// Whether this visit's view was counted.
    counted: bool,
    /// The second last kept while playing.
    kept_at: f64,
}

/// What the player hands back about itself when asked.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerState {
    pub time: f64,
    pub playing: bool,
}

impl Watching {
    /// The page for `id` opened, `width` wide; none for a video there is not.
    pub fn open(id: &str, list: Option<&str>, at: Option<f64>, width: f32) -> Option<Self> {
        Some(Watching {
            video: video_by_id(id)?,
            list: list.map(str::to_owned),
            at,
            auto: pick_auto(width),
            switched_at: None,
            play_on_load: true,
            playing: false,
            ended: false,
            counted: false,
            kept_at: -KEEP_EVERY,
        })
    }

    /// The playlist, as the page shows it.
    pub fn playlist<'a>(&self, data: &'a Data) -> Option<&'a Playlist> {
        self.list.as_deref().and_then(|id| data.playlists.seen_get(id))
    }

    /// The rendition that plays: the one chosen, or Auto's.
    pub fn rendition(&self, data: &Data) -> &'static Rendition {
        match data.prefs.quality {
            Quality::Auto => self.auto,
            Quality::Rendition(id) => rendition_of(id.id()),
        }
    }

    /// Whether the player is handed the clip yet: once the history has come, so it starts where
    /// the viewer stopped.
    pub fn attached(&self, data: &Data) -> bool {
        data.history.ready()
    }

    /// Where the viewer stopped, when it is worth starting there.
    pub fn resume(&self, data: &Data) -> Option<f64> {
        let seen = data.history.seen_get(self.video.id)?;
        (!seen.finished && seen.position > 3.0 && seen.position < self.video.duration - 3.0).then_some(seen.position)
    }

    /// Where the player starts: where a change of quality left off, else the address's second,
    /// else where the viewer stopped.
    pub fn start_at(&self, data: &Data) -> Option<f64> {
        self.switched_at.or(self.at).or_else(|| self.resume(data))
    }

    /// What plays after this one.
    pub fn next(&self, data: &Data) -> Option<&'static Video> {
        aside::next_of(self.video, self.playlist(data), &aside::up_next_for(self.video))
    }

    /// The player's time, every so often while it plays: kept every `KEEP_EVERY` seconds.
    pub fn time(&mut self, data: &mut Data, seconds: f64) {
        if !data.prefs.history || (seconds - self.kept_at).abs() < KEEP_EVERY {
            return;
        }
        self.kept_at = seconds;
        let at = data.now();
        data.keep_watched(
            self.video.id,
            WatchedPatch {
                position: Some(seconds),
                at: Some(at),
                finished: Some(false),
                ..WatchedPatch::default()
            },
        );
    }

    /// The player started or stopped; `player` is where it is now.
    pub fn playing_changed(&mut self, data: &mut Data, on: bool, player: PlayerState) {
        self.playing = on;
        if on {
            self.ended = false;
        }
        if !data.prefs.history {
            return;
        }
        let at = data.now();
        if on && !self.counted {
            self.counted = true;
            let times = data.history.seen_get(self.video.id).map_or(0, |one| one.times) + 1;
            data.keep_watched(
                self.video.id,
                WatchedPatch {
                    at: Some(at),
                    times: Some(times),
                    finished: Some(false),
                    ..WatchedPatch::default()
                },
            );
        } else if !on && player.time > 0.0 && player.time < self.video.duration - 0.5 {
            data.keep_watched(
                self.video.id,
                WatchedPatch {
                    position: Some(player.time),
                    at: Some(at),
                    ..WatchedPatch::default()
                },
            );
        }
    }

    /// The video played to its end.
    pub fn ended(&mut self, data: &mut Data) {
        self.ended = true;
        if data.prefs.history {
            let at = data.now();
            data.keep_watched(
                self.video.id,
                WatchedPatch {
                    position: Some(0.0),
                    finished: Some(true),
                    at: Some(at),
                    ..WatchedPatch::default()
                },
            );
        }
    }

    /// Up next's Play now, or its countdown run out: the next video, inside the playlist.
    pub fn play_next(&self, data: &mut Data) {
        if let Some(next) = self.next(data) {
            let list = self.list.clone().filter(|id| data.playlists.seen_get(id).is_some());
            data.go(path_of(&watch_href(next, list.as_deref())));
        }
    }

    /// Up next's Stay here.
    pub fn stay(&mut self) {
        self.ended = false;
    }

    /// A quality picked in the player: it goes on from where it was, playing only if it was.
    pub fn choose_quality(&mut self, data: &mut Data, quality: Quality, player: Option<PlayerState>) {
        self.switched_at = player.map(|one| one.time);
        self.play_on_load = player.is_some_and(|one| one.playing);
        data.set_prefs(&PrefsPatch {
            quality: Some(quality),
            ..PrefsPatch::default()
        });
    }

    /// The player's quality menu: Auto with the rendition it plays, then each with its bitrate.
    pub fn qualities(&self) -> Vec<(Quality, &'static str, String)> {
        std::iter::once((Quality::Auto, "Auto", self.auto.label.to_owned()))
            .chain(
                RENDITIONS
                    .iter()
                    .map(|one| (Quality::Rendition(one.id), one.label, bitrate_of(one.bitrate))),
            )
            .collect()
    }

    /// The player's speed picked: kept as the speed every video starts at.
    pub fn rate_changed(&self, data: &mut Data, rate: f64) {
        data.set_prefs(&PrefsPatch {
            rate: Some(rate),
            ..PrefsPatch::default()
        });
    }

    /// The C key: captions on or off.
    pub fn toggle_captions(&self, data: &mut Data) {
        let mut captions = data.prefs.captions.clone();
        captions.track = if captions.track == CaptionTrack::Off {
            CaptionTrack::On("en".to_owned())
        } else {
            CaptionTrack::Off
        };
        data.set_prefs(&PrefsPatch {
            captions: Some(captions),
            ..PrefsPatch::default()
        });
    }

    /// The T key and the theater button.
    pub fn toggle_theater(&self, data: &mut Data) {
        let on = !data.prefs.theater;
        data.set_prefs(&PrefsPatch {
            theater: Some(on),
            ..PrefsPatch::default()
        });
    }

    /// The I key and the Stats entry.
    pub fn toggle_stats(&self, data: &mut Data) {
        let on = !data.prefs.stats;
        data.set_prefs(&PrefsPatch {
            stats: Some(on),
            ..PrefsPatch::default()
        });
    }

    /// Follow beside the channel, said at once.
    pub fn follow(&self, data: &mut Data, on: bool) {
        crate::pages::search::follow(data, channel_of(self.video), on);
    }

    /// Like or Dislike, or neither.
    pub fn rate(&self, data: &mut Data, rating: Option<RatingKind>) {
        data.rate_video(self.video.id, rating);
    }

    /// Download: the clip at the rendition playing, kept on this device.
    pub fn download(&self, data: &mut Data) {
        let rendition = self.rendition(data).id;
        data.start_download(self.video.id, rendition);
    }

    /// The download's Cancel.
    pub fn cancel_download(&self, data: &mut Data) {
        data.cancel_download(self.video.id);
    }

    /// The download's Remove.
    pub fn remove_download(&self, data: &mut Data) {
        data.remove_download(self.video.id, Some(Toast::plain("Download removed")), true);
    }

    /// The Download action pressed as an entry of the actions' menu: the downloads when kept.
    pub fn download_entry(&self, data: &mut Data) {
        if data.downloads.seen_get(self.video.id).is_some() {
            data.go("/downloads");
        } else {
            self.download(data);
        }
    }

    /// What the page shows.
    pub fn view(&self, data: &Data) -> WatchView {
        let video = self.video;
        let channel = channel_of(video);
        let seen = data.history.seen_get(video.id);
        let following = data.follows.seen_get(channel.id).is_some();
        let rating = data.ratings.seen_get(video.id).map(|one| one.rating);
        let saved = data
            .playlists
            .seen()
            .iter()
            .any(|one| one.videos.iter().any(|id| id == video.id));
        let download = data.downloads.seen_get(video.id);
        let fetching = data.fetching.get(video.id).map(|one| one.share);
        let rendition = self.rendition(data);
        let clip = clip_of(video, rendition);
        let next = self.next(data);
        let prefs = &data.prefs;
        WatchView {
            title: video.title,
            channel: channel.name,
            followers: channel.followers + i64::from(following),
            following,
            rating: rating.map(|one| match one {
                RatingKind::Like => "like",
                RatingKind::Dislike => "dislike",
            }),
            likes: video.likes + i64::from(rating == Some(RatingKind::Like)),
            saved_label: if saved { "Saved" } else { "Save" },
            download_label: if download.is_some() {
                "Downloaded".to_owned()
            } else if let Some(share) = fetching {
                format!("Downloading {}%", round(share * 100.0))
            } else {
                "Download".to_owned()
            },
            download_state: if download.is_some() {
                "kept"
            } else if fetching.is_some() {
                "running"
            } else {
                "idle"
            },
            download_value: fetching,
            kept_detail: download.map(|one| {
                format!(
                    "On this device, {}, {}",
                    bytes_of(one.bytes),
                    rendition_of(one.rendition.id()).label
                )
            }),
            meta: vec![
                views_of(view_count(video, seen)),
                days_ago(video.days),
                TOPICS
                    .iter()
                    .find(|(topic, _)| *topic == video.topic)
                    .map_or("", |(_, label)| label)
                    .to_owned(),
            ],
            attached: self.attached(data),
            start_at: self.start_at(data),
            play_on_load: self.play_on_load,
            rate: prefs.rate,
            quality: prefs.quality.id(),
            theater: prefs.theater,
            qualities: self
                .qualities()
                .into_iter()
                .map(|(quality, label, detail)| (quality.id(), label, detail))
                .collect(),
            stats: prefs.stats.then(|| {
                vec![
                    (
                        "Quality",
                        if prefs.quality == Quality::Auto {
                            format!("{}, Auto", clip.rendition.label)
                        } else {
                            clip.rendition.label.to_owned()
                        },
                    ),
                    (
                        "Codecs",
                        if clip.sound { "VP8, Opus" } else { "VP8, no sound" }.to_owned(),
                    ),
                    ("Clip size", bytes_of(clip.bytes)),
                    (
                        "Made in",
                        if clip.kept {
                            "an earlier visit".to_owned()
                        } else {
                            format!("{} s", to_fixed(clip.made / 1000.0, 1))
                        },
                    ),
                ]
            }),
            up_next: (self.ended).then_some(next).flatten().map(|next| UpNextView {
                id: next.id,
                title: next.title,
                subtitle: channel_of(next).name,
                countdown: prefs.autoplay.then_some(COUNTDOWN),
            }),
            next: next.map(|one| one.id),
        }
    }
}

/// Up next over the picture once the video ended.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UpNextView {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    /// The seconds it counts down from, while autoplay is on.
    pub countdown: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WatchView {
    pub title: &'static str,
    pub channel: &'static str,
    pub followers: i64,
    pub following: bool,
    pub rating: Option<&'static str>,
    pub likes: i64,
    pub saved_label: &'static str,
    pub download_label: String,
    pub download_state: &'static str,
    pub download_value: Option<f64>,
    pub kept_detail: Option<String>,
    pub meta: Vec<String>,
    /// Whether the player has the clip.
    pub attached: bool,
    pub start_at: Option<f64>,
    pub play_on_load: bool,
    pub rate: f64,
    pub quality: &'static str,
    pub theater: bool,
    pub qualities: Vec<(&'static str, &'static str, String)>,
    /// The stats' rows over the picture, while shown.
    pub stats: Option<Vec<(&'static str, String)>>,
    pub up_next: Option<UpNextView>,
    pub next: Option<&'static str>,
}
