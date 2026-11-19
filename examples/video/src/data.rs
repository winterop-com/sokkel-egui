//! What the viewer keeps on the site, as the web's `data.ts` keeps it on the shared fake backend:
//! the channels they follow, what they liked, what they watched and where they stopped, their
//! playlists, comments, downloads and player settings.
//!
//! Each collection is the web's `collection` from `tidevann-shared/backend`: a change is made and
//! kept at once, its answer comes `LATENCY` milliseconds later, and every change is told to those
//! listening that much later with the rows as they were when it was made. A page reads a
//! collection's rows as they were last told or listed (`Collection::seen`), none until the first
//! answer, so what a page shows lags a change by the same moment the web's does. The web's timers
//! are kept as the web sets them, and `Data::advance` moves the clock. Days are read in UTC, as
//! the web's tests run it.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize, Serializer};

use crate::catalog::video_by_id;
use crate::catalog::{Person, VIDEOS, VIEWER, channel_of};
use crate::clips::{RenditionId, clip_of, rendition_of};

pub const MINUTE: i64 = 60_000;
pub const HOUR: i64 = 60 * MINUTE;
pub const DAY: i64 = 24 * HOUR;

/// How long each call to the backend takes, in milliseconds.
pub const LATENCY: i64 = 150;

/// A row of a collection, kept under its id.
pub trait Row: Clone {
    fn id(&self) -> &str;
}

macro_rules! row {
    ($($kind:ty),*) => {
        $(impl Row for $kind {
            fn id(&self) -> &str {
                &self.id
            }
        })*
    };
}

/// A channel the viewer follows, under the channel's id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Follow {
    pub id: String,
    pub since: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RatingKind {
    Like,
    Dislike,
}

/// What the viewer thinks of a video, under the video's id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rating {
    pub id: String,
    pub rating: RatingKind,
}

/// A video the viewer watched, under its id: where they stopped, when, and whether they saw it to
/// the end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Watched {
    pub id: String,
    pub position: f64,
    pub at: i64,
    pub finished: bool,
    pub times: i64,
}

/// A change to a history row: the fields given, the rest kept.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WatchedPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub times: Option<i64>,
}

impl Watched {
    pub fn apply(&mut self, patch: &WatchedPatch) {
        if let Some(position) = patch.position {
            self.position = position;
        }
        if let Some(at) = patch.at {
            self.at = at;
        }
        if let Some(finished) = patch.finished {
            self.finished = finished;
        }
        if let Some(times) = patch.times {
            self.times = times;
        }
    }
}

/// A list of videos the viewer keeps; `later` is Watch later, the rest are their own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: String,
    pub videos: Vec<String>,
    pub created: i64,
    pub updated: i64,
}

/// A reaction to a comment, under the emoji as its id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommentReaction {
    pub id: String,
    pub label: String,
    pub count: i64,
    pub mine: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub video: String,
    /// The comment it answers, or none for one on the video itself.
    pub parent: Option<String>,
    pub author: Person,
    pub text: String,
    pub at: i64,
    pub likes: i64,
    pub liked: bool,
    pub disliked: bool,
    pub pinned: bool,
    /// Reported by the viewer, and so hidden from them.
    pub reported: bool,
    /// The emoji people reacted with, each with its name, how many gave it and whether the viewer
    /// did; none until the first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reactions: Option<Vec<CommentReaction>>,
}

/// A video kept on this device to watch without the site, under the video's id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Download {
    pub id: String,
    pub rendition: RenditionId,
    pub bytes: f64,
    pub at: i64,
}

row!(Follow, Rating, Watched, Playlist, Comment, Download, Prefs);

/// The quality a viewer picks: Auto, or one rendition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Auto,
    Rendition(RenditionId),
}

impl Quality {
    pub fn id(self) -> &'static str {
        match self {
            Quality::Auto => "auto",
            Quality::Rendition(one) => one.id(),
        }
    }

    pub fn from_id(id: &str) -> Option<Quality> {
        if id == "auto" {
            Some(Quality::Auto)
        } else {
            RenditionId::from_id(id).map(Quality::Rendition)
        }
    }
}

impl Serialize for Quality {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for Quality {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        Quality::from_id(&id).ok_or_else(|| serde::de::Error::custom(format!("no quality {id}")))
    }
}

/// Which caption track is on, as the player keeps it: none chosen yet, off, or the track's key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CaptionTrack {
    #[default]
    Unchosen,
    Off,
    On(String),
}

impl CaptionTrack {
    pub fn is_unchosen(&self) -> bool {
        *self == CaptionTrack::Unchosen
    }
}

impl Serialize for CaptionTrack {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            CaptionTrack::On(key) => serializer.serialize_str(key),
            _ => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for CaptionTrack {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Option::<String>::deserialize(deserializer)?.map_or(CaptionTrack::Off, CaptionTrack::On))
    }
}

/// What the viewer chose for captions, kept between videos, each field only once chosen, as the
/// web's `Partial<CaptionSettings>`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CaptionChoice {
    #[serde(default, skip_serializing_if = "CaptionTrack::is_unchosen")]
    pub track: CaptionTrack,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge: Option<String>,
}

/// The viewer's player and site settings, one row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prefs {
    pub id: String,
    /// Plays what is next when a video ends, after a countdown.
    pub autoplay: bool,
    pub theater: bool,
    /// The speed every video starts at.
    pub rate: f64,
    pub quality: Quality,
    /// The captions' look and whether they are on.
    pub captions: CaptionChoice,
    /// Keeps what is watched in the history.
    pub history: bool,
    /// Shows the stats over the picture.
    pub stats: bool,
}

/// A change to the settings: the fields given, the rest kept.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PrefsPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoplay: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theater: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<Quality>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captions: Option<CaptionChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<bool>,
}

impl Prefs {
    pub fn apply(&mut self, patch: &PrefsPatch) {
        if let Some(on) = patch.autoplay {
            self.autoplay = on;
        }
        if let Some(on) = patch.theater {
            self.theater = on;
        }
        if let Some(rate) = patch.rate {
            self.rate = rate;
        }
        if let Some(quality) = patch.quality {
            self.quality = quality;
        }
        if let Some(captions) = &patch.captions {
            self.captions = captions.clone();
        }
        if let Some(on) = patch.history {
            self.history = on;
        }
        if let Some(on) = patch.stats {
            self.stats = on;
        }
    }
}

/// The id of Watch later among the playlists.
pub const WATCH_LATER: &str = "later";

/// The settings a viewer starts with.
pub fn default_prefs() -> Prefs {
    Prefs {
        id: "me".to_owned(),
        autoplay: true,
        theater: false,
        rate: 1.0,
        quality: Quality::Auto,
        captions: CaptionChoice {
            track: CaptionTrack::On("en".to_owned()),
            ..CaptionChoice::default()
        },
        history: true,
        stats: false,
    }
}

pub fn seed_follows(now: i64) -> Vec<Follow> {
    [("harbor", 200), ("night", 40), ("tides", 10)]
        .into_iter()
        .map(|(id, days)| Follow {
            id: id.to_owned(),
            since: now - days * DAY,
        })
        .collect()
}

pub fn seed_ratings() -> Vec<Rating> {
    ["aurora-fjord", "first-light", "island-ferry"]
        .into_iter()
        .map(|id| Rating {
            id: id.to_owned(),
            rating: RatingKind::Like,
        })
        .collect()
}

pub fn seed_history(now: i64) -> Vec<Watched> {
    [
        ("first-light", 17.0, 2 * HOUR, false, 1),
        ("tide-table", 0.0, 5 * HOUR, true, 2),
        ("island-ferry", 0.0, DAY + 3 * HOUR, true, 1),
        ("snow-pass", 24.0, 3 * DAY, false, 1),
        ("valley-line", 9.0, 6 * DAY, false, 1),
    ]
    .into_iter()
    .map(|(id, position, ago, finished, times)| Watched {
        id: id.to_owned(),
        position,
        at: now - ago,
        finished,
        times,
    })
    .collect()
}

pub fn seed_playlists(now: i64) -> Vec<Playlist> {
    let list = |id: &str, name: &str, description: &str, videos: [&str; 3], created: i64, updated: i64| Playlist {
        id: id.to_owned(),
        name: name.to_owned(),
        description: description.to_owned(),
        videos: videos.iter().map(|one| (*one).to_owned()).collect(),
        created: now - created * DAY,
        updated: now - updated * DAY,
    };
    vec![
        list(
            WATCH_LATER,
            "Watch later",
            "",
            ["night-train", "spring-tide", "aurora-night"],
            90,
            1,
        ),
        list(
            "coast-mornings",
            "Coast mornings",
            "Slow starts by the sea, for the first coffee.",
            ["first-light", "harbor-wakes", "boat-afloat"],
            30,
            4,
        ),
        list(
            "window-seat",
            "Window seat",
            "Long train rides, filmed from the window.",
            ["valley-line", "salt-flats", "night-train"],
            12,
            2,
        ),
    ]
}

/// What people say under any video of the site, a few picked for each.
const SAID: [&str; 8] = [
    "Watched this twice already. The sound alone is worth it.",
    "Where exactly is this filmed? It looks a lot like the coast where I grew up.",
    "Perfect to have on while working. More of these, please.",
    "The pacing is exactly right: nothing rushed, nothing added.",
    "I would love a longer version of this one.",
    "Saved it for the next rainy evening.",
    "How long did this take to film? It must have been a lot of early mornings.",
    "This is the calmest thing I have seen all week.",
];

const REPLIES: [&str; 4] = [
    "Same here.",
    "Agreed, the ending especially.",
    "It is about an hour north of the city, past the old ferry quay.",
    "Thanks, that means a lot.",
];

/// A comment or two from each channel's owner, pinned, and a few from the rest, with some replies.
pub fn seed_comments(now: i64) -> Vec<Comment> {
    let mut out = Vec::new();
    for (index, video) in VIDEOS.iter().enumerate() {
        let index = index as i64;
        let owner = channel_of(video).owner;
        let days = video.days as i64;
        let at = |ago: i64, hours: i64| now - ago.min(days) * DAY - hours * HOUR - (index % 5) * MINUTE * 7;
        let comment = |id: &str, author: Person, text: String, when: i64, likes: i64, parent: Option<&str>| Comment {
            id: format!("{}-{id}", video.id),
            video: video.id.to_owned(),
            parent: parent.map(|one| format!("{}-{one}", video.id)),
            author,
            text,
            at: when,
            likes,
            liked: false,
            disliked: false,
            pinned: false,
            reported: false,
            reactions: None,
        };
        if index % 3 != 2 {
            let mut pinned = comment(
                "owner",
                owner,
                format!(
                    "Thank you for watching. {} The next one is already in the works.",
                    video.captions.first().copied().unwrap_or_default()
                ),
                at(days, -1),
                120 + index * 37,
                None,
            );
            pinned.pinned = true;
            pinned.reactions = Some(vec![
                CommentReaction {
                    id: "\u{2764}\u{FE0F}".to_owned(),
                    label: "red heart".to_owned(),
                    count: 8 + index % 5,
                    mine: false,
                },
                CommentReaction {
                    id: "\u{1F389}".to_owned(),
                    label: "party popper".to_owned(),
                    count: 3,
                    mine: false,
                },
            ]);
            out.push(pinned);
        }
        let others: Vec<Person> = Person::ALL
            .into_iter()
            .filter(|person| *person != owner && *person != VIEWER)
            .collect();
        for n in 0..2 + index % 3 {
            let author = others[((index + n) as usize) % others.len()];
            let text = SAID[((index * 3 + n) as usize) % SAID.len()];
            let id = format!("c{n}");
            out.push(comment(
                &id,
                author,
                text.to_owned(),
                at((days - n - 1).max(0), n * 3),
                (90 - n * 31 - (index % 7) * 9).max(0),
                None,
            ));
            if n == 0 {
                out.push(comment(
                    &format!("c{n}-r"),
                    if owner == author { Person::Linus } else { owner },
                    REPLIES[(index as usize) % REPLIES.len()].to_owned(),
                    at((days - n - 1).max(0), n * 3 - 2),
                    4 + index % 9,
                    Some(&id),
                ));
            }
        }
    }
    out
}

/// The collections, by the name the backend keeps each under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Name {
    Follows,
    Ratings,
    History,
    Playlists,
    Comments,
    Downloads,
    Prefs,
}

impl Name {
    pub fn key(self) -> &'static str {
        match self {
            Name::Follows => "follows",
            Name::Ratings => "ratings",
            Name::History => "history",
            Name::Playlists => "playlists",
            Name::Comments => "comments",
            Name::Downloads => "downloads",
            Name::Prefs => "prefs",
        }
    }
}

/// A note the site shows for a moment.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Toast {
    /// `error` for a failure, none for a plain note.
    #[serde(rename = "type")]
    pub kind: Option<&'static str>,
    pub title: String,
    pub description: Option<String>,
    /// The words of its one button, such as Undo, when it has one.
    pub action: Option<String>,
}

impl Toast {
    pub fn plain(title: impl Into<String>) -> Self {
        Toast {
            kind: None,
            title: title.into(),
            description: None,
            action: None,
        }
    }

    pub fn with(title: impl Into<String>, description: impl Into<String>) -> Self {
        Toast {
            description: Some(description.into()),
            ..Toast::plain(title)
        }
    }
}

/// A download under way, as the watch page keeps it: the ticket it was started with and how far
/// it is said to have come.
#[derive(Clone, Debug, PartialEq)]
pub struct Fetch {
    pub ticket: u64,
    pub began: i64,
    pub rendition: RenditionId,
    /// From 0 to 0.95, moved every `TICK` milliseconds.
    pub share: f64,
}

/// How often a download's share moves, in milliseconds.
pub const TICK: i64 = 120;

/// How long a download takes at the least, as a fetch would.
pub const FETCH_TAKES: i64 = 2000;

#[derive(Clone, Debug, PartialEq)]
enum Job {
    /// A collection's change told to those listening, with the rows kept under the timer's seq.
    Tell(Name),
    /// A collection's list answered, with the rows of the snapshot under the timer's seq.
    Listed(Name, Listener),
    /// The history row made for `id` is answered: the patches asked for meanwhile follow.
    HistoryMade(String),
    /// What follows a change once it is answered.
    Then(Then),
    DownloadTick {
        video: String,
        ticket: u64,
    },
    DownloadReady {
        video: String,
        ticket: u64,
    },
    DownloadKept {
        video: String,
        ticket: u64,
    },
}

/// Who asked for a list: the module itself, as `data.ts` does as it loads, or a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Listener {
    Module,
    Page,
}

#[derive(Clone, Debug)]
struct Timer {
    at: i64,
    seq: u64,
    every: Option<i64>,
    job: Job,
}

/// The web's clock and its timers.
#[derive(Clone, Debug, Default)]
pub struct Clock {
    pub now: i64,
    /// How many rows the backend added, whose count ends the ids it makes.
    made: u64,
    seq: u64,
    timers: Vec<Timer>,
}

impl Clock {
    fn after(&mut self, ms: i64, job: Job) -> u64 {
        self.seq += 1;
        self.timers.push(Timer {
            at: self.now + ms,
            seq: self.seq,
            every: None,
            job,
        });
        self.seq
    }

    fn every(&mut self, ms: i64, job: Job) -> u64 {
        self.seq += 1;
        self.timers.push(Timer {
            at: self.now + ms,
            seq: self.seq,
            every: Some(ms),
            job,
        });
        self.seq
    }

    fn clear(&mut self, seq: u64) {
        self.timers.retain(|timer| timer.seq != seq);
    }
}

/// One collection: its rows as the backend keeps them, and as the pages last heard them.
#[derive(Clone, Debug)]
pub struct Collection<T: Row> {
    pub name: Name,
    /// The rows as kept, every change made at once.
    pub rows: Vec<T>,
    /// The rows as last told or listed, none until the first answer.
    pub seen: Option<Vec<T>>,
    /// The rows each change or list will tell, under its timer's seq.
    pending: Vec<(u64, Vec<T>)>,
}

impl<T: Row> Collection<T> {
    fn new(name: Name, rows: Vec<T>) -> Self {
        Collection {
            name,
            rows,
            seen: None,
            pending: Vec::new(),
        }
    }

    /// The rows a page shows: none until the first answer.
    pub fn seen(&self) -> &[T] {
        self.seen.as_deref().unwrap_or_default()
    }

    /// Whether the first answer has come.
    pub fn ready(&self) -> bool {
        self.seen.is_some()
    }

    /// The row under `id` a page shows.
    pub fn seen_get(&self, id: &str) -> Option<&T> {
        self.seen().iter().find(|row| row.id() == id)
    }

    /// The row under `id` as kept.
    pub fn get(&self, id: &str) -> Option<&T> {
        self.rows.iter().find(|row| row.id() == id)
    }

    fn save(&mut self, clock: &mut Clock, next: Vec<T>) {
        self.rows = next;
        let seq = clock.after(LATENCY, Job::Tell(self.name));
        self.pending.push((seq, self.rows.clone()));
    }

    fn list(&mut self, clock: &mut Clock, listener: Listener) {
        let seq = clock.after(LATENCY, Job::Listed(self.name, listener));
        self.pending.push((seq, self.rows.clone()));
    }

    fn take(&mut self, seq: u64) -> Option<Vec<T>> {
        let at = self.pending.iter().position(|(one, _)| *one == seq)?;
        Some(self.pending.remove(at).1)
    }

    /// Adds a row at the front, as the backend's `create` and `upsert` add one.
    fn create(&mut self, clock: &mut Clock, row: T) {
        clock.made += 1;
        let mut next = vec![row];
        next.extend(self.rows.iter().cloned());
        self.save(clock, next);
    }

    /// Changes the row under `id`; there being none is refused, and nothing is kept.
    fn update(&mut self, clock: &mut Clock, id: &str, change: impl FnOnce(&mut T)) -> Result<(), String> {
        let Some(found) = self.get(id) else {
            return Err(format!("No {} with the id {id}.", self.name.key()));
        };
        let mut row = found.clone();
        change(&mut row);
        let next = self
            .rows
            .iter()
            .map(|one| if one.id() == id { row.clone() } else { one.clone() })
            .collect();
        self.save(clock, next);
        Ok(())
    }

    fn remove(&mut self, clock: &mut Clock, id: &str) {
        let next = self.rows.iter().filter(|row| row.id() != id).cloned().collect();
        self.save(clock, next);
    }
}

/// Everything the viewer keeps, on the web's clock, and what the site said and where it went.
#[derive(Clone, Debug)]
pub struct Data {
    pub clock: Clock,
    pub follows: Collection<Follow>,
    pub ratings: Collection<Rating>,
    pub history: Collection<Watched>,
    pub playlists: Collection<Playlist>,
    pub comments: Collection<Comment>,
    pub downloads: Collection<Download>,
    pub prefs_rows: Collection<Prefs>,
    /// The settings as the pages hold them: every change at once, the backend's word as it comes.
    pub prefs: Prefs,
    /// Whether the backend has answered with the settings yet.
    pub prefs_ready: bool,
    /// The channels followed as the backend last said, and as presses since have changed them.
    followed_now: BTreeSet<String>,
    /// The ids the history holds as the backend last said.
    in_history: BTreeSet<String>,
    /// The history rows being made, with the patches asked for meanwhile.
    making_history: BTreeMap<String, Vec<WatchedPatch>>,
    /// The downloads under way, by video.
    pub fetching: BTreeMap<String, Fetch>,
    /// The ticket of the latest download started or cancelled, by video.
    tickets: BTreeMap<String, u64>,
    /// The timers of the downloads under way, by video.
    ticking: BTreeMap<String, u64>,
    /// The notes shown since they were last taken.
    pub toasts: Vec<Toast>,
    /// The places gone to since they were last taken, as `go` writes them.
    pub went: Vec<String>,
}

impl Data {
    /// The site as it opens at `now`: every collection from its seed, listed once as `data.ts`
    /// and the pages list them.
    pub fn new(now: i64) -> Self {
        let mut clock = Clock {
            now,
            ..Clock::default()
        };
        let mut follows = Collection::new(Name::Follows, seed_follows(now));
        let mut history = Collection::new(Name::History, seed_history(now));
        let mut prefs_rows = Collection::new(Name::Prefs, vec![default_prefs()]);
        follows.list(&mut clock, Listener::Module);
        history.list(&mut clock, Listener::Module);
        prefs_rows.list(&mut clock, Listener::Module);
        let mut data = Data {
            clock,
            follows,
            ratings: Collection::new(Name::Ratings, seed_ratings()),
            history,
            playlists: Collection::new(Name::Playlists, seed_playlists(now)),
            comments: Collection::new(Name::Comments, seed_comments(now)),
            downloads: Collection::new(Name::Downloads, Vec::new()),
            prefs_rows,
            prefs: default_prefs(),
            prefs_ready: false,
            followed_now: BTreeSet::new(),
            in_history: BTreeSet::new(),
            making_history: BTreeMap::new(),
            fetching: BTreeMap::new(),
            tickets: BTreeMap::new(),
            ticking: BTreeMap::new(),
            toasts: Vec::new(),
            went: Vec::new(),
        };
        // The pages' own lists, as the site's frame mounts them.
        let clock = &mut data.clock;
        data.follows.list(clock, Listener::Page);
        data.playlists.list(clock, Listener::Page);
        data.ratings.list(clock, Listener::Page);
        data.downloads.list(clock, Listener::Page);
        data.history.list(clock, Listener::Page);
        data.comments.list(clock, Listener::Page);
        data
    }

    pub fn now(&self) -> i64 {
        self.clock.now
    }

    /// Fires every timer due by `to`, each at its time, in the order they were set.
    pub fn advance(&mut self, to: i64) {
        loop {
            let next = self
                .clock
                .timers
                .iter()
                .enumerate()
                .filter(|(_, timer)| timer.at <= to)
                .min_by_key(|(_, timer)| (timer.at, timer.seq))
                .map(|(index, _)| index);
            let Some(index) = next else { break };
            let timer = &mut self.clock.timers[index];
            self.clock.now = timer.at;
            let (seq, job) = (timer.seq, timer.job.clone());
            match timer.every {
                Some(every) => timer.at += every,
                None => {
                    self.clock.timers.remove(index);
                }
            }
            self.fire(seq, job);
        }
        self.clock.now = self.clock.now.max(to);
    }

    /// The time of the next timer due, if any.
    pub fn next_due(&self) -> Option<i64> {
        self.clock.timers.iter().map(|timer| timer.at).min()
    }

    fn fire(&mut self, seq: u64, job: Job) {
        match job {
            Job::Tell(name) => self.heard(name, seq, Listener::Page, true),
            Job::Listed(name, listener) => self.heard(name, seq, listener, false),
            Job::HistoryMade(id) => {
                for patch in self.making_history.remove(&id).unwrap_or_default() {
                    let _ = self.history.update(&mut self.clock, &id, |row| row.apply(&patch));
                }
            }
            Job::Then(Then::Toast(toast)) => self.toasts.push(toast),
            Job::Then(Then::Go(path)) => self.went.push(path),
            Job::DownloadTick { video, ticket } => {
                if self.tickets.get(&video) == Some(&ticket) {
                    let now = self.clock.now;
                    if let Some(fetch) = self.fetching.get_mut(&video) {
                        fetch.share = (((now - fetch.began) as f64) / FETCH_TAKES as f64).min(0.95);
                    }
                }
            }
            Job::DownloadReady { video, ticket } => self.download_ready(&video, ticket),
            Job::DownloadKept { video, ticket } => {
                if let Some(title) = video_by_id(&video).map(|one| one.title) {
                    self.toasts
                        .push(Toast::with("Downloaded", format!("{title}, kept on this device")));
                }
                self.download_settled(&video, ticket);
            }
        }
    }

    /// A collection's rows heard: by the pages, and by `data.ts`'s own listeners.
    fn heard(&mut self, name: Name, seq: u64, listener: Listener, told: bool) {
        macro_rules! take {
            ($collection:expr) => {{
                let Some(rows) = $collection.take(seq) else { return };
                if listener == Listener::Page {
                    $collection.seen = Some(rows.clone());
                }
                rows
            }};
        }
        match name {
            Name::Follows => {
                let rows = take!(self.follows);
                if told {
                    self.followed_now.clear();
                }
                if told || listener == Listener::Module {
                    self.followed_now.extend(rows.iter().map(|row| row.id.clone()));
                }
            }
            Name::History => {
                let rows = take!(self.history);
                if told {
                    self.in_history.clear();
                }
                if told || listener == Listener::Module {
                    self.in_history.extend(rows.iter().map(|row| row.id.clone()));
                }
            }
            Name::Prefs => {
                let rows = take!(self.prefs_rows);
                if listener == Listener::Module {
                    self.prefs_ready = true;
                }
                if let Some(row) = rows.iter().find(|one| one.id == "me") {
                    self.prefs = row.clone();
                }
            }
            Name::Ratings => {
                take!(self.ratings);
            }
            Name::Playlists => {
                take!(self.playlists);
            }
            Name::Comments => {
                take!(self.comments);
            }
            Name::Downloads => {
                take!(self.downloads);
            }
        }
    }

    /// Schedules what follows a change made now once it is answered.
    pub fn then(&mut self, then: Then) {
        self.clock.after(LATENCY, Job::Then(then));
    }

    fn then_toast(&mut self, toast: Toast) {
        self.then(Then::Toast(toast));
    }

    /// Goes to a place, as the web's `go` sets the address.
    pub fn go(&mut self, path: impl Into<String>) {
        self.went.push(path.into());
    }

    pub fn toast(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    // --- data.ts --------------------------------------------------------------------------------

    /// Follows a channel or stops, kept by the backend; a press that asks for what already holds
    /// does nothing.
    pub fn set_following(&mut self, channel: &str, on: bool) {
        if on == self.followed_now.contains(channel) {
            return;
        }
        if on {
            self.followed_now.insert(channel.to_owned());
            let since = self.clock.now;
            self.follows.create(
                &mut self.clock,
                Follow {
                    id: channel.to_owned(),
                    since,
                },
            );
        } else {
            self.followed_now.remove(channel);
            self.follows.remove(&mut self.clock, channel);
        }
    }

    /// Keeps where the viewer is in a video, making its history row the first time.
    pub fn keep_watched(&mut self, id: &str, patch: WatchedPatch) {
        if let Some(waiting) = self.making_history.get_mut(id) {
            waiting.push(patch);
            return;
        }
        if self.in_history.contains(id) {
            let _ = self.history.update(&mut self.clock, id, |row| row.apply(&patch));
            return;
        }
        let mut row = Watched {
            id: id.to_owned(),
            position: 0.0,
            at: self.clock.now,
            finished: false,
            times: 0,
        };
        row.apply(&patch);
        self.history.create(&mut self.clock, row);
        self.making_history.insert(id.to_owned(), Vec::new());
        self.clock.after(LATENCY, Job::HistoryMade(id.to_owned()));
    }

    /// Changes settings at once on the page and tells the backend.
    pub fn set_prefs(&mut self, patch: &PrefsPatch) {
        self.prefs.apply(patch);
        let _ = self.prefs_rows.update(&mut self.clock, "me", |row| row.apply(patch));
    }

    // --- The collections, as the pages change them ----------------------------------------------

    /// The id the backend makes for a row added without one.
    fn made_id(&self, name: Name) -> String {
        format!("{}-{}-{}", name.key(), base36(self.clock.now), self.clock.made + 1)
    }

    /// Rates a video, or takes the rating back: a rating the page knows is changed, else made.
    pub fn rate_video(&mut self, id: &str, rating: Option<RatingKind>) {
        let known = self.ratings.seen_get(id).is_some();
        match rating {
            None => self.ratings.remove(&mut self.clock, id),
            Some(rating) if known => {
                let _ = self.ratings.update(&mut self.clock, id, |row| row.rating = rating);
            }
            Some(rating) => self.ratings.create(
                &mut self.clock,
                Rating {
                    id: id.to_owned(),
                    rating,
                },
            ),
        }
    }

    /// Takes a rating away, with a note once it is gone.
    pub fn remove_rating(&mut self, id: &str, toast: Option<Toast>) {
        self.ratings.remove(&mut self.clock, id);
        if let Some(toast) = toast {
            self.then_toast(toast);
        }
    }

    /// Adds a video to a playlist as the page shows it, or takes it out, with a note once kept.
    pub fn toggle_in(&mut self, list: &str, video: &str, on: bool, toast: Option<Toast>) {
        let Some(shown) = self.playlists.seen_get(list) else {
            return;
        };
        let mut videos: Vec<String> = shown.videos.iter().filter(|one| *one != video).cloned().collect();
        if on {
            videos.push(video.to_owned());
        }
        let patch = PlaylistPatch {
            videos: Some(videos),
            ..PlaylistPatch::default()
        };
        self.save_playlist(list, patch, toast.map(Then::Toast).into_iter().collect());
    }

    /// Changes a playlist, its time of change now, and follows the answer with `then` when kept.
    pub fn save_playlist(&mut self, list: &str, patch: PlaylistPatch, then: Vec<Then>) {
        let now = self.clock.now;
        let kept = self.playlists.update(&mut self.clock, list, |row| {
            if let Some(name) = &patch.name {
                row.name = name.clone();
            }
            if let Some(description) = &patch.description {
                row.description = description.clone();
            }
            if let Some(videos) = &patch.videos {
                row.videos = videos.clone();
            }
            row.updated = now;
        });
        if kept.is_ok() {
            for one in then {
                self.then(one);
            }
        }
    }

    /// Makes a playlist, with a video in it when given, and says so once it is made; its id.
    pub fn new_playlist(
        &mut self,
        name: &str,
        video: Option<&str>,
        then: impl FnOnce(&Playlist) -> Vec<Then>,
    ) -> String {
        let id = self.made_id(Name::Playlists);
        let now = self.clock.now;
        let row = Playlist {
            id: id.clone(),
            name: crate::js::trim(name).to_owned(),
            description: String::new(),
            videos: video.map(|one| vec![one.to_owned()]).unwrap_or_default(),
            created: now,
            updated: now,
        };
        let jobs = then(&row);
        self.playlists.create(&mut self.clock, row);
        for job in jobs {
            self.then(job);
        }
        id
    }

    /// Deletes a playlist, with a note once it is gone.
    pub fn delete_playlist(&mut self, list: &str, toast: Option<Toast>) {
        self.playlists.remove(&mut self.clock, list);
        if let Some(toast) = toast {
            self.then_toast(toast);
        }
    }

    /// Takes a video out of the history, with a note once it is gone.
    pub fn remove_watched(&mut self, id: &str, toast: Option<Toast>) {
        self.history.remove(&mut self.clock, id);
        if let Some(toast) = toast {
            self.then_toast(toast);
        }
    }

    /// Adds a comment, or a reply to one, by the viewer.
    pub fn add_comment(&mut self, video: &str, text: &str, parent: Option<&str>) {
        let id = self.made_id(Name::Comments);
        let row = Comment {
            id,
            video: video.to_owned(),
            parent: parent.map(str::to_owned),
            author: VIEWER,
            text: crate::js::trim(text).to_owned(),
            at: self.clock.now,
            likes: 0,
            liked: false,
            disliked: false,
            pinned: false,
            reported: false,
            reactions: None,
        };
        self.comments.create(&mut self.clock, row);
    }

    /// Changes a comment as kept.
    pub fn update_comment(&mut self, id: &str, change: impl FnOnce(&mut Comment)) {
        let _ = self.comments.update(&mut self.clock, id, change);
    }

    pub fn remove_comment(&mut self, id: &str) {
        self.comments.remove(&mut self.clock, id);
    }

    // --- Downloads, as the watch page makes them ------------------------------------------------

    /// Starts keeping a video on this device: it takes `FETCH_TAKES` at the least, as a fetch
    /// would, its share moving every `TICK`.
    pub fn start_download(&mut self, video: &str, rendition: RenditionId) {
        if self.fetching.contains_key(video) || self.downloads.seen_get(video).is_some() {
            return;
        }
        let ticket = self.tickets.get(video).copied().unwrap_or(0) + 1;
        self.tickets.insert(video.to_owned(), ticket);
        let began = self.clock.now;
        self.fetching.insert(
            video.to_owned(),
            Fetch {
                ticket,
                began,
                rendition,
                share: 0.0,
            },
        );
        let tick = self.clock.every(
            TICK,
            Job::DownloadTick {
                video: video.to_owned(),
                ticket,
            },
        );
        self.ticking.insert(format!("{video}:{ticket}"), tick);
        self.clock.after(
            FETCH_TAKES,
            Job::DownloadReady {
                video: video.to_owned(),
                ticket,
            },
        );
    }

    fn download_ready(&mut self, video: &str, ticket: u64) {
        if self.tickets.get(video) != Some(&ticket) {
            self.download_settled(video, ticket);
            return;
        }
        let Some(fetch) = self.fetching.get(video).cloned() else {
            return;
        };
        let Some(found) = video_by_id(video) else { return };
        let clip = clip_of(found, rendition_of(fetch.rendition.id()));
        let at = self.clock.now;
        self.downloads.create(
            &mut self.clock,
            Download {
                id: video.to_owned(),
                rendition: fetch.rendition,
                bytes: clip.bytes,
                at,
            },
        );
        self.clock.after(
            LATENCY,
            Job::DownloadKept {
                video: video.to_owned(),
                ticket,
            },
        );
    }

    /// A download's end, kept or not: its timer stops, and its share goes if it is still its own.
    fn download_settled(&mut self, video: &str, ticket: u64) {
        if let Some(tick) = self.ticking.remove(&format!("{video}:{ticket}")) {
            self.clock.clear(tick);
        }
        if self.tickets.get(video) == Some(&ticket) {
            self.fetching.remove(video);
        }
    }

    /// Stops a download under way.
    pub fn cancel_download(&mut self, video: &str) {
        let ticket = self.tickets.get(video).copied().unwrap_or(0) + 1;
        self.tickets.insert(video.to_owned(), ticket);
        self.fetching.remove(video);
        self.toasts.push(Toast::plain("Download cancelled"));
    }

    /// Takes a video's download away.
    pub fn remove_download(&mut self, video: &str, toast: Option<Toast>, before: bool) {
        if before && let Some(toast) = toast.clone() {
            self.toasts.push(toast);
        }
        self.downloads.remove(&mut self.clock, video);
        if !before && let Some(toast) = toast {
            self.then_toast(toast);
        }
    }
}

/// A change to a playlist: the fields given, the rest kept.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlaylistPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub videos: Option<Vec<String>>,
}

/// What follows a change once it is answered: a note, or a place gone to.
#[derive(Clone, Debug, PartialEq)]
pub enum Then {
    Toast(Toast),
    Go(String),
}

/// A number in base 36, as `Number.prototype.toString(36)` writes a whole one.
pub fn base36(value: i64) -> String {
    if value == 0 {
        return "0".to_owned();
    }
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut left = value.unsigned_abs();
    let mut out = Vec::new();
    while left > 0 {
        out.push(digits[(left % 36) as usize]);
        left /= 36;
    }
    out.reverse();
    let text = String::from_utf8(out).unwrap_or_default();
    if value < 0 { format!("-{text}") } else { text }
}
