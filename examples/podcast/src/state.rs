//! What every page reads and does, as the web's `state.tsx` keeps it: the listener's shows and
//! episodes, the queue, the settings, the time listened, and the player, one audio element that
//! plays each episode's sound. Here the element is a model of one (`Element`): what it holds, where
//! it is, whether it plays, at what speed and how loud; the app plays the sound (`voice`) and tells
//! the model where it got to (`played_to`) and when it ended (`ended`).
//!
//! The web's backend answers each change a moment later, but shows it at once; the rows here are
//! those it shows. The web's timers are kept as the web sets them, and `advance` moves the clock:
//! the place kept every five seconds while playing, the time listened counted each second and
//! written down every fifteen, downloads moving along every 400 milliseconds, the sleep timer
//! counting down, and Check for new answering after 900 milliseconds. After each change the
//! provider's effects run as React runs them after a render (`render`), in the order the web
//! declares them, each only when what it depends on changed.

use std::sync::Arc;

use serde::Serialize;

use crate::backend::{
    DownloadState, EpisodeState, Listen, PlayerRow, Prefs, PrefsPatch, ShowState, day, default_prefs, seed_episodes,
    seed_listens, seed_player, seed_shows,
};
use crate::catalog::{episode_by_id, episodes as catalog_episodes, show_by_id};
use crate::chapters::{chapter_index, chapter_starts};
use crate::js;
use crate::script::{Script, length_of, script_of};
use crate::sleep_timer::{SleepTimer, chapter_timer_step};

/// How often the place in an episode is kept while it plays, in milliseconds.
const KEEP_EVERY: i64 = 5000;

/// How often the time listened is written down while playing, in seconds.
const LOG_EVERY: u32 = 15;

/// How often a download moves along, in milliseconds.
const DOWNLOAD_EVERY: i64 = 400;

/// How long Check for new takes, as asking a server would.
const CHECK_TAKES: i64 = 900;

/// How long the backend takes to answer, in milliseconds.
const LATENCY: i64 = 150;

/// A note the app shows for a moment.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Toast {
    pub title: String,
    pub description: Option<String>,
}

/// The audio element, as the provider sees it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Element {
    /// The episode whose sound it holds.
    #[serde(skip)]
    pub src: Option<String>,
    pub paused: bool,
    /// Where it is, in seconds.
    pub time: f64,
    pub rate: f64,
    pub volume: f64,
    pub muted: bool,
}

/// Where Queue's add puts an episode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    Next,
    End,
}

/// The time listened, counted while playing until it is written down.
#[derive(Clone, Debug, Default, PartialEq)]
struct Heard {
    show: String,
    seconds: i64,
    content: f64,
    finished: i64,
}

#[derive(Clone, Debug, PartialEq)]
enum Job {
    Keep,
    Count,
    Download,
    Sleep,
    Check,
    ClearStats(Vec<String>),
}

#[derive(Clone, Debug)]
struct Timer {
    at: i64,
    seq: u64,
    every: Option<i64>,
    job: Job,
}

/// What the provider's effects depend on, as a render reads it.
#[derive(Clone, Debug, PartialEq)]
struct Rendered {
    timer: Option<SleepTimer>,
    playing: bool,
    current: Option<String>,
    position: f64,
    volume: f64,
    muted: bool,
    rate: f64,
    downloading: bool,
}

/// The listener's podcasts and the player, on a clock of their own.
pub struct Podcasts {
    /// The clock, in milliseconds.
    pub now: i64,
    pub shows: Vec<ShowState>,
    pub episodes: Vec<EpisodeState>,
    pub player: PlayerRow,
    pub prefs: Prefs,
    /// The time listened, newest written first.
    pub listens: Vec<Listen>,
    pub element: Element,
    /// Whether the element plays, as its events last said.
    pub playing: bool,
    /// Where the element was last reported, in seconds.
    reported: f64,
    heard: Heard,
    /// Ticks counted since playing started.
    ticks: u32,
    timers: Vec<Timer>,
    seq: u64,
    /// Where a loaded episode starts, once its metadata is read.
    metadata: Option<f64>,
    sleep_timer: Option<SleepTimer>,
    sleep_remaining: Option<f64>,
    sleep_left: Option<f64>,
    sleep_since: i64,
    sleep_anchor: i64,
    sleep_last: Option<(f64, i64)>,
    rendered: Option<Rendered>,
    /// The notes shown since they were last taken.
    pub toasts: Vec<Toast>,
    /// How many episodes each Check for new brought, as they answer.
    pub checked: Vec<usize>,
}

fn title_of(id: &str) -> String {
    episode_by_id(id).map_or(id.to_owned(), |episode| episode.title.clone())
}

/// A change to an episode's row: the fields given, the rest kept.
#[derive(Clone, Debug, Default)]
struct EpisodePatch {
    out: Option<bool>,
    out_at: Option<i64>,
    position: Option<f64>,
    played: Option<bool>,
    starred: Option<bool>,
    inbox: Option<bool>,
    download: Option<DownloadState>,
    progress: Option<f64>,
    heard: Option<Option<i64>>,
}

impl Podcasts {
    /// The backend as it is seeded at `now`, the player holding nothing yet.
    pub fn new(now: i64) -> Self {
        let player = seed_player();
        let mut podcasts = Self {
            now,
            shows: seed_shows(now),
            episodes: seed_episodes(now),
            element: Element {
                src: None,
                paused: true,
                time: 0.0,
                rate: player.rate,
                volume: player.volume,
                muted: player.muted,
            },
            player,
            prefs: default_prefs(),
            listens: seed_listens(now),
            playing: false,
            reported: 0.0,
            heard: Heard::default(),
            ticks: 0,
            timers: Vec::new(),
            seq: 0,
            metadata: None,
            sleep_timer: None,
            sleep_remaining: None,
            sleep_left: None,
            sleep_since: now,
            sleep_anchor: -1,
            sleep_last: None,
            rendered: None,
            toasts: Vec::new(),
            checked: Vec::new(),
        };
        podcasts.render();
        podcasts
    }

    // --- What the pages read -----------------------------------------------------------------------

    pub fn current(&self) -> Option<&str> {
        self.player.current.as_deref()
    }

    /// The episode the element holds.
    pub fn loaded(&self) -> Option<&str> {
        self.element.src.as_deref()
    }

    /// Whether the element holds what plays, so the bar scrubs it and draws its waveform.
    pub fn media(&self) -> bool {
        self.loaded().is_some() && self.loaded() == self.current()
    }

    /// Where the bar is: where the element is, or before it holds the episode, where it was left.
    pub fn position(&self) -> f64 {
        if self.media() {
            self.reported
        } else {
            self.current()
                .and_then(|id| self.episode(id))
                .map_or(0.0, |row| row.position)
        }
    }

    /// The script of what plays.
    pub fn script(&self) -> Option<Arc<Script>> {
        self.current().and_then(script_of)
    }

    pub fn episode(&self, id: &str) -> Option<&EpisodeState> {
        self.episodes.iter().find(|row| row.id == id)
    }

    pub fn show(&self, id: &str) -> Option<&ShowState> {
        self.shows.iter().find(|row| row.id == id)
    }

    pub fn sleep_timer(&self) -> Option<SleepTimer> {
        self.sleep_timer
    }

    /// The seconds left of a timer in minutes, counted while playing.
    pub fn sleep_remaining(&self) -> Option<f64> {
        self.sleep_remaining
    }

    // --- The clock -----------------------------------------------------------------------------------

    /// Moves the clock to `to`, firing every timer due by then at its time, in the order set.
    pub fn advance(&mut self, to: i64) {
        loop {
            self.settle();
            let next = self
                .timers
                .iter()
                .enumerate()
                .filter(|(_, timer)| timer.at <= to)
                .min_by_key(|(_, timer)| (timer.at, timer.seq))
                .map(|(index, _)| index);
            let Some(index) = next else { break };
            let timer = &mut self.timers[index];
            self.now = timer.at;
            let job = timer.job.clone();
            match timer.every {
                Some(every) => timer.at += every,
                None => {
                    self.timers.remove(index);
                }
            }
            self.fire(job);
        }
        self.now = to.max(self.now);
        self.settle();
    }

    fn add_timer(&mut self, job: Job, after: i64, every: bool) {
        self.seq += 1;
        self.timers.push(Timer {
            at: self.now + after,
            seq: self.seq,
            every: every.then_some(after),
            job,
        });
    }

    fn clear_timer(&mut self, job: &Job) {
        self.timers.retain(|timer| timer.job != *job);
    }

    fn fire(&mut self, job: Job) {
        match job {
            Job::Keep => self.keep_place(),
            Job::Count => {
                self.heard.seconds += 1;
                self.heard.content += self.element.rate;
                self.ticks += 1;
                if self.ticks.is_multiple_of(LOG_EVERY) {
                    self.write_listens();
                }
            }
            Job::Download => {
                let running: Vec<(String, f64)> = self
                    .episodes
                    .iter()
                    .filter(|row| row.download == DownloadState::Running)
                    .map(|row| (row.id.clone(), row.progress))
                    .collect();
                for (id, progress) in running {
                    let step = 0.05 + ((id.encode_utf16().count() * 7) % 5) as f64 * 0.015;
                    let next = 1.0_f64.min(progress + step);
                    self.patch_episode(
                        &id,
                        if next >= 1.0 {
                            EpisodePatch {
                                download: Some(DownloadState::Kept),
                                progress: Some(1.0),
                                ..Default::default()
                            }
                        } else {
                            EpisodePatch {
                                progress: Some(next),
                                ..Default::default()
                            }
                        },
                    );
                }
            }
            Job::Sleep => {
                let next = self.sleep_left.unwrap_or(0.0) - (self.now - self.sleep_since) as f64 / 1000.0;
                self.sleep_since = self.now;
                if next <= 0.0 {
                    self.run_out();
                } else {
                    self.sleep_left = Some(next);
                    self.sleep_remaining = Some(next.ceil());
                }
            }
            Job::Check => {
                let followed: Vec<String> = self
                    .shows
                    .iter()
                    .filter(|show| show.subscribed)
                    .map(|show| show.id.clone())
                    .collect();
                let mut count = 0;
                for show in followed {
                    let next = catalog_episodes()
                        .iter()
                        .filter(|one| one.show == show && self.episode(&one.id).is_some_and(|row| !row.out))
                        .min_by_key(|one| one.number);
                    let Some(next) = next else { continue };
                    count += 1;
                    let auto = self.prefs.auto_download;
                    self.patch_episode(
                        &next.id,
                        EpisodePatch {
                            out: Some(true),
                            out_at: Some(self.now),
                            inbox: Some(self.prefs.inbox),
                            download: auto.then_some(DownloadState::Running),
                            progress: auto.then_some(0.0),
                            ..Default::default()
                        },
                    );
                }
                self.checked.push(count);
            }
            Job::ClearStats(ids) => self.listens.retain(|row| !ids.contains(&row.id)),
        }
    }

    /// What follows a change: the element's metadata read, then the effects of a render.
    fn settle(&mut self) {
        if let Some(at) = self.metadata.take() {
            self.set_time(at);
        }
        self.render();
    }

    fn snapshot(&self) -> Rendered {
        Rendered {
            timer: self.sleep_timer,
            playing: self.playing,
            current: self.player.current.clone(),
            position: self.position(),
            volume: self.player.volume,
            muted: self.player.muted,
            rate: self.player.rate,
            downloading: self.episodes.iter().any(|row| row.download == DownloadState::Running),
        }
    }

    /// The provider's effects after a render, in the order the web declares them, each run when
    /// what it read changed, until nothing more changes.
    fn render(&mut self) {
        for _ in 0..100 {
            let now = self.snapshot();
            let before = self.rendered.clone();
            if before.as_ref() == Some(&now) {
                return;
            }
            let changed = |read: &dyn Fn(&Rendered) -> bool| before.as_ref().is_none_or(|before| !read(before));
            // The sleep timer's minutes count down only while playing.
            if changed(&|was| was.timer == now.timer && was.playing == now.playing) {
                self.clear_timer(&Job::Sleep);
                if matches!(now.timer, Some(SleepTimer::Minutes(_))) && now.playing {
                    self.sleep_since = self.now;
                    self.add_timer(Job::Sleep, 1000, true);
                }
            }
            // A timer to the end of the chapter follows each position reported.
            if changed(&|was| {
                was.timer == now.timer
                    && was.current == now.current
                    && was.position == now.position
                    && was.playing == now.playing
            }) {
                let last = self.sleep_last.replace((now.position, self.now));
                let starts = now
                    .current
                    .as_deref()
                    .and_then(script_of)
                    .map(|script| chapter_starts(&script.chapters));
                if let (Some(SleepTimer::Chapter), Some(starts)) = (now.timer, starts) {
                    let (ended, anchor) = chapter_timer_step(
                        &starts,
                        self.sleep_anchor,
                        last.map(|(position, _)| position),
                        now.position,
                        last.map_or(0.0, |(_, at)| (self.now - at) as f64 / 1000.0),
                        now.playing,
                    );
                    self.sleep_anchor = anchor;
                    if ended {
                        self.run_out();
                    }
                }
            }
            // While playing: the place kept every few seconds, and the time listened counted each second.
            if changed(&|was| was.playing == now.playing) {
                self.clear_timer(&Job::Keep);
                self.clear_timer(&Job::Count);
                if now.playing {
                    self.add_timer(Job::Keep, KEEP_EVERY, true);
                    self.ticks = 0;
                    self.add_timer(Job::Count, 1000, true);
                }
            }
            // The element follows the player's speed and loudness.
            if changed(&|was| was.volume == now.volume && was.muted == now.muted) {
                self.element.volume = now.volume;
                self.element.muted = now.muted;
            }
            if changed(&|was| was.rate == now.rate) {
                self.element.rate = now.rate;
            }
            // Downloads move along a little at a time until they are kept.
            if changed(&|was| was.downloading == now.downloading) {
                self.clear_timer(&Job::Download);
                if now.downloading {
                    self.add_timer(Job::Download, DOWNLOAD_EVERY, true);
                }
            }
            self.rendered = Some(now);
        }
    }

    // --- The element and its events -----------------------------------------------------------------

    fn set_time(&mut self, time: f64) {
        self.element.time = time;
        self.reported = time;
    }

    fn element_play(&mut self) {
        if self.element.paused {
            self.element.paused = false;
            self.playing = true;
        }
    }

    fn element_pause(&mut self) {
        if !self.element.paused {
            self.element.paused = true;
            self.paused();
        }
    }

    fn paused(&mut self) {
        self.playing = false;
        self.keep_place();
        self.write_listens();
    }

    fn on_ended(&mut self) {
        self.playing = false;
        let Some(id) = self.element.src.clone() else { return };
        self.heard.finished += 1;
        self.write_listens();
        let archive = self.prefs.archive_played;
        self.patch_episode(
            &id,
            EpisodePatch {
                played: Some(true),
                position: Some(0.0),
                heard: Some(Some(self.now)),
                inbox: archive.then_some(false),
                ..Default::default()
            },
        );
        if self.sleep_ended() {
            return;
        }
        if self.prefs.continue_queue
            && let Some(next) = self.player.queue.first().cloned()
        {
            self.toast("Up next", Some(title_of(&next)));
            self.play_now(&next, None);
        }
    }

    /// Tells the sleep timer what plays has ended: a timer to the end, or to the end of the
    /// chapter, runs out there and answers true.
    fn sleep_ended(&mut self) -> bool {
        let timer = self.rendered.as_ref().and_then(|rendered| rendered.timer);
        if !matches!(timer, Some(SleepTimer::End | SleepTimer::Chapter)) {
            return false;
        }
        self.run_out();
        true
    }

    fn run_out(&mut self) {
        self.sleep_left = None;
        self.sleep_remaining = None;
        self.sleep_timer = None;
        self.element_pause();
    }

    // --- The rows ----------------------------------------------------------------------------------

    fn toast(&mut self, title: &str, description: Option<String>) {
        self.toasts.push(Toast {
            title: title.to_owned(),
            description,
        });
    }

    fn patch_episode(&mut self, id: &str, patch: EpisodePatch) {
        let Some(row) = self.episodes.iter_mut().find(|row| row.id == id) else {
            return;
        };
        if let Some(value) = patch.out {
            row.out = value;
        }
        if let Some(value) = patch.out_at {
            row.out_at = value;
        }
        if let Some(value) = patch.position {
            row.position = value;
        }
        if let Some(value) = patch.played {
            row.played = value;
        }
        if let Some(value) = patch.starred {
            row.starred = value;
        }
        if let Some(value) = patch.inbox {
            row.inbox = value;
        }
        if let Some(value) = patch.download {
            row.download = value;
        }
        if let Some(value) = patch.progress {
            row.progress = value;
        }
        if let Some(value) = patch.heard {
            row.heard = value;
        }
    }

    /// Keeps where the loaded episode is.
    fn keep_place(&mut self) {
        if let Some(id) = self.element.src.clone() {
            let (position, now) = (self.element.time, self.now);
            self.patch_episode(
                &id,
                EpisodePatch {
                    position: Some(position),
                    heard: Some(Some(now)),
                    ..Default::default()
                },
            );
        }
    }

    /// Writes down the time listened since it was last written.
    fn write_listens(&mut self) {
        let heard = std::mem::take(&mut self.heard);
        self.heard.show = heard.show.clone();
        if heard.show.is_empty() || (heard.seconds == 0 && heard.finished == 0) {
            return;
        }
        let date = day(0, self.now);
        let id = format!("{date}:{}", heard.show);
        let content = js::round(heard.content) as i64;
        match self.listens.iter_mut().find(|row| row.id == id) {
            Some(found) => {
                found.seconds += heard.seconds;
                found.content += content;
                found.finished += heard.finished;
            }
            None => self.listens.insert(
                0,
                Listen {
                    id,
                    day: date,
                    show: heard.show,
                    seconds: heard.seconds,
                    content,
                    finished: heard.finished,
                },
            ),
        }
    }

    fn load(&mut self, id: &str, at: f64) {
        // A new source stops what played without a word; its metadata is read a moment later.
        self.element.src = Some(id.to_owned());
        self.element.paused = true;
        self.element.time = 0.0;
        self.metadata = Some(at);
        self.element.rate = self.player.rate;
        self.reported = at;
        let show = episode_by_id(id).map_or(String::new(), |episode| episode.show.clone());
        if self.heard.show != show {
            self.write_listens();
            self.heard.show = show;
        }
    }

    /// Where an episode picks up: where it was left, or the start once played or all but finished.
    fn resume_at(&self, id: &str) -> f64 {
        let length = length_of(id);
        match self.episode(id) {
            Some(row) if !row.played && row.position < length - 2.0 => row.position,
            _ => 0.0,
        }
    }

    fn play_now(&mut self, id: &str, at: Option<f64>) {
        let playing_now = self.player.current.clone();
        if playing_now.as_deref() == Some(id) && self.loaded() == Some(id) && at.is_none() {
            if self.element.paused {
                self.element_play();
            } else {
                self.element_pause();
            }
            return;
        }
        self.keep_place();
        let from = at.unwrap_or_else(|| self.resume_at(id));
        let other = playing_now.as_deref() != Some(id);
        if other {
            self.element.rate = self.prefs.speed;
            self.player.rate = self.prefs.speed;
        }
        self.player.current = Some(id.to_owned());
        self.player.queue.retain(|one| one != id);
        if self.episode(id).is_some_and(|row| row.played) {
            self.patch_episode(
                id,
                EpisodePatch {
                    played: Some(false),
                    ..Default::default()
                },
            );
        }
        if self.loaded() != Some(id) {
            self.load(id, from);
        } else {
            self.set_time(from);
        }
        self.element_play();
    }

    fn seek_to(&mut self, seconds: f64) {
        let Some(id) = self.player.current.clone() else { return };
        let to = seconds.min(length_of(&id)).max(0.0);
        if self.loaded() == Some(id.as_str()) {
            self.set_time(to);
        } else {
            self.patch_episode(
                &id,
                EpisodePatch {
                    position: Some(to),
                    ..Default::default()
                },
            );
        }
        self.reported = to;
    }

    // --- What the pages do ----------------------------------------------------------------------------

    /// Plays an episode, from `at` or where it picks up, or plays and pauses the one playing.
    pub fn play(&mut self, id: &str, at: Option<f64>) {
        self.play_now(id, at);
        self.settle();
    }

    pub fn set_playing(&mut self, on: bool) {
        if let Some(id) = self.player.current.clone() {
            if !on {
                self.element_pause();
            } else if self.loaded() != Some(id.as_str()) {
                self.play_now(&id, None);
            } else {
                self.element_play();
            }
        }
        self.settle();
    }

    pub fn seek(&mut self, seconds: f64) {
        self.seek_to(seconds);
        self.settle();
    }

    pub fn skip(&mut self, by: f64) {
        if let Some(id) = self.player.current.clone() {
            let from = if self.loaded() == Some(id.as_str()) {
                self.element.time
            } else {
                self.episode(&id).map_or(0.0, |row| row.position)
            };
            self.seek_to(from + by);
        }
        self.settle();
    }

    pub fn set_rate(&mut self, rate: f64) {
        self.element.rate = rate;
        self.player.rate = rate;
        self.settle();
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.player.volume = volume;
        self.player.muted = false;
        self.settle();
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.player.muted = muted;
        self.settle();
    }

    /// Plays the first in the queue; false when the queue is empty.
    pub fn play_next(&mut self) -> bool {
        let Some(next) = self.player.queue.first().cloned() else {
            return false;
        };
        self.play_now(&next, None);
        self.settle();
        true
    }

    /// The element played on to `seconds`, as its time updates say.
    pub fn played_to(&mut self, seconds: f64) {
        self.set_time(seconds);
        self.settle();
    }

    /// The element played to the end of what it holds.
    pub fn ended(&mut self) {
        let length = self.player.current.as_deref().map_or(0.0, length_of);
        self.set_time(length);
        if !self.element.paused {
            self.element.paused = true;
            self.paused();
        }
        self.on_ended();
        self.settle();
    }

    /// The page is left: the place and the time are kept.
    pub fn away(&mut self) {
        self.keep_place();
        self.write_listens();
        self.settle();
    }

    pub fn queue_add(&mut self, id: &str, place: Where) {
        if self.player.current.as_deref() == Some(id) {
            self.toast("Already playing", Some(title_of(id)));
            self.settle();
            return;
        }
        self.player.queue.retain(|one| one != id);
        match place {
            Where::Next => self.player.queue.insert(0, id.to_owned()),
            Where::End => self.player.queue.push(id.to_owned()),
        }
        let title = if place == Where::Next {
            "Plays next"
        } else {
            "Added to the queue"
        };
        self.toast(title, Some(title_of(id)));
        self.settle();
    }

    pub fn queue_remove(&mut self, id: &str) {
        self.player.queue.retain(|one| one != id);
        self.settle();
    }

    /// Moves the queue's `from`th to `to`, as the web's `splice` does: a `from` below zero counts
    /// from the end, and a `to` outside the queue moves nothing.
    pub fn queue_move(&mut self, from: i64, to: i64) {
        let length = self.player.queue.len() as i64;
        let from = if from < 0 { (length + from).max(0) } else { from };
        if to >= 0 && to < length && from < length {
            let moved = self.player.queue.remove(from as usize);
            self.player.queue.insert(to as usize, moved);
        }
        self.settle();
    }

    pub fn queue_clear(&mut self) {
        self.player.queue.clear();
        self.settle();
    }

    pub fn mark_played(&mut self, id: &str, played: bool) {
        let archive = played && self.prefs.archive_played;
        self.patch_episode(
            id,
            EpisodePatch {
                played: Some(played),
                position: Some(0.0),
                inbox: archive.then_some(false),
                ..Default::default()
            },
        );
        if self.loaded() == Some(id) && self.current() == Some(id) {
            self.element_pause();
            self.set_time(0.0);
        }
        self.settle();
    }

    pub fn star(&mut self, id: &str, starred: bool) {
        self.patch_episode(
            id,
            EpisodePatch {
                starred: Some(starred),
                ..Default::default()
            },
        );
        self.settle();
    }

    pub fn archive(&mut self, id: &str) {
        self.patch_episode(
            id,
            EpisodePatch {
                inbox: Some(false),
                ..Default::default()
            },
        );
        self.settle();
    }

    pub fn archive_all(&mut self) {
        for row in &mut self.episodes {
            row.inbox = false;
        }
        self.settle();
    }

    pub fn download(&mut self, id: &str) {
        self.patch_episode(
            id,
            EpisodePatch {
                download: Some(DownloadState::Running),
                progress: Some(0.0),
                ..Default::default()
            },
        );
        self.settle();
    }

    pub fn remove_download(&mut self, id: &str) {
        self.patch_episode(
            id,
            EpisodePatch {
                download: Some(DownloadState::Idle),
                progress: Some(0.0),
                ..Default::default()
            },
        );
        self.settle();
    }

    pub fn remove_all_downloads(&mut self) {
        for row in &mut self.episodes {
            if row.download != DownloadState::Idle {
                row.download = DownloadState::Idle;
                row.progress = 0.0;
            }
        }
        self.settle();
    }

    /// Brings the next episode of each followed show that has one still to come, as a server
    /// answers; how many came is pushed to `checked`.
    pub fn check_for_new(&mut self) {
        self.add_timer(Job::Check, CHECK_TAKES, false);
        self.settle();
    }

    pub fn subscribe(&mut self, id: &str) {
        let now = self.now;
        if let Some(show) = self.shows.iter_mut().find(|show| show.id == id) {
            show.subscribed = true;
            show.since = Some(now);
        }
        let latest = catalog_episodes()
            .iter()
            .filter(|one| one.show == id && self.episode(&one.id).is_some_and(|row| row.out))
            .max_by_key(|one| one.number);
        if let Some(latest) = latest
            && let Some(row) = self.episode(&latest.id).cloned()
            && !row.played
        {
            let download = self.prefs.auto_download && row.download == DownloadState::Idle;
            self.patch_episode(
                &latest.id,
                EpisodePatch {
                    inbox: Some(self.prefs.inbox || row.inbox),
                    download: download.then_some(DownloadState::Running),
                    progress: download.then_some(0.0),
                    ..Default::default()
                },
            );
        }
        self.toast("Following", show_by_id(id).map(|show| show.title.clone()));
        self.settle();
    }

    pub fn unsubscribe(&mut self, id: &str) {
        if let Some(show) = self.shows.iter_mut().find(|show| show.id == id) {
            show.subscribed = false;
            show.since = None;
        }
        for row in &mut self.episodes {
            if row.inbox && episode_by_id(&row.id).is_some_and(|episode| episode.show == id) {
                row.inbox = false;
            }
        }
        self.toast("No longer following", show_by_id(id).map(|show| show.title.clone()));
        self.settle();
    }

    pub fn set_prefs(&mut self, patch: &PrefsPatch) {
        self.prefs.apply(patch);
        self.settle();
    }

    /// Forgets the time listened, as the backend answers: the rows there now go a moment later.
    pub fn clear_stats(&mut self) {
        let ids = self.listens.iter().map(|row| row.id.clone()).collect();
        self.add_timer(Job::ClearStats(ids), LATENCY, false);
        self.settle();
    }

    /// Sets the sleep timer, a chapter timer waiting on the chapter playing now.
    pub fn set_sleep(&mut self, timer: Option<SleepTimer>) {
        let starts = self.script().map(|script| chapter_starts(&script.chapters));
        self.sleep_anchor = starts.map_or(-1, |starts| chapter_index(&starts, self.position()));
        self.sleep_left = match timer {
            Some(SleepTimer::Minutes(minutes)) => Some(minutes * 60.0),
            _ => None,
        };
        self.sleep_remaining = self.sleep_left;
        self.sleep_timer = timer;
        self.settle();
    }
}
