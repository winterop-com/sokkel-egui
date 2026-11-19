//! What the backend keeps for the listener, as the web's `backend.ts` seeds it: which shows they
//! follow, where they are in each episode, the queue and the player, their settings and the time
//! they listened. The shows and episodes themselves are the catalog's. Days are read in UTC, as the
//! web's tests run it.

use serde::{Deserialize, Serialize};

use crate::catalog::{episode_by_id, episodes, shows};
use crate::script::{Seeded, length_of};

/// Where a download stands, as sokkel-ui's `DownloadButton` names it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadState {
    #[default]
    Idle,
    Running,
    Paused,
    Kept,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShowState {
    pub id: String,
    pub subscribed: bool,
    /// When it was followed, in milliseconds.
    pub since: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeState {
    pub id: String,
    /// Whether it has come out, and when, in milliseconds.
    pub out: bool,
    pub out_at: i64,
    /// Where the listener left it, in seconds.
    pub position: f64,
    pub played: bool,
    pub starred: bool,
    /// Whether it waits in New.
    pub inbox: bool,
    pub download: DownloadState,
    /// How far a download has come, from 0 to 1.
    pub progress: f64,
    /// When it was last heard, in milliseconds.
    pub heard: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerRow {
    pub id: String,
    pub current: Option<String>,
    /// The episodes to play after it, in order.
    pub queue: Vec<String>,
    pub rate: f64,
    pub volume: f64,
    pub muted: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub id: String,
    /// The speed a new episode starts at.
    pub speed: f64,
    /// How far the skip buttons move, in seconds.
    pub skip: f64,
    /// Whether the next in the queue plays when an episode ends.
    pub continue_queue: bool,
    /// Whether new episodes of followed shows download as they come out.
    pub auto_download: bool,
    /// Whether new episodes of followed shows go to New.
    pub inbox: bool,
    /// Whether an episode leaves New once played.
    pub archive_played: bool,
}

/// A change to the settings: the fields given, the rest kept.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_queue: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_download: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inbox: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_played: Option<bool>,
}

impl Prefs {
    pub fn apply(&mut self, patch: &PrefsPatch) {
        if let Some(speed) = patch.speed {
            self.speed = speed;
        }
        if let Some(skip) = patch.skip {
            self.skip = skip;
        }
        if let Some(on) = patch.continue_queue {
            self.continue_queue = on;
        }
        if let Some(on) = patch.auto_download {
            self.auto_download = on;
        }
        if let Some(on) = patch.inbox {
            self.inbox = on;
        }
        if let Some(on) = patch.archive_played {
            self.archive_played = on;
        }
    }
}

/// Time listened on one day to one show.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Listen {
    pub id: String,
    pub day: String,
    pub show: String,
    /// Seconds of the listener's time.
    pub seconds: i64,
    /// Seconds of the episodes heard, more than `seconds` when played faster.
    pub content: i64,
    /// Episodes played to the end.
    pub finished: i64,
}

pub const DAY: i64 = 86_400_000;

/// The year, month (1 to 12) and day of the month of a moment, in UTC.
pub fn civil(ms: i64) -> (i64, u32, u32) {
    let days = ms.div_euclid(DAY);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The day of the week of a moment, in UTC, 0 for Sunday.
pub fn weekday(ms: i64) -> u32 {
    (ms.div_euclid(DAY) + 4).rem_euclid(7) as u32
}

/// The day `offset` days from `from`, as `YYYY-MM-DD`.
pub fn day(offset: i64, from: i64) -> String {
    let (year, month, day) = civil(from + offset * DAY);
    format!("{year}-{month:02}-{day:02}")
}

const FOLLOWED: [&str; 5] = [
    "low-tide-hours",
    "harbor-log",
    "field-notes-larkspur",
    "juniper-weather",
    "tidevann-weekly",
];

fn followed(show: &str) -> bool {
    FOLLOWED.contains(&show)
}

/// The settings a listener starts with.
pub fn default_prefs() -> Prefs {
    Prefs {
        id: "prefs".to_owned(),
        speed: 1.0,
        skip: 15.0,
        continue_queue: true,
        auto_download: true,
        inbox: true,
        archive_played: true,
    }
}

pub fn seed_shows(now: i64) -> Vec<ShowState> {
    shows()
        .iter()
        .enumerate()
        .map(|(index, show)| ShowState {
            id: show.id.clone(),
            subscribed: followed(&show.id),
            since: followed(&show.id).then(|| now - (60 - index as i64) * DAY),
        })
        .collect()
}

/// Where the listener is in a few episodes, as seconds in.
const STARTED: [(&str, f64); 3] = [
    ("low-tide-hours-3", 48.0),
    ("tidevann-weekly-2", 31.0),
    ("islands-one-by-one-2", 22.0),
];
const STARRED: [&str; 4] = [
    "low-tide-hours-1",
    "lighthouse-stories-1",
    "tidevann-weekly-1",
    "rivers-and-deltas-3",
];
const KEPT: [&str; 4] = [
    "low-tide-hours-3",
    "low-tide-hours-4",
    "harbor-log-3",
    "juniper-weather-3",
];

pub fn seed_episodes(now: i64) -> Vec<EpisodeState> {
    episodes()
        .iter()
        .map(|episode| {
            let out = episode.age.is_some();
            let age = episode.age.unwrap_or(0);
            let followed = followed(&episode.show);
            let started = STARTED.iter().find(|(id, _)| *id == episode.id).map(|(_, at)| *at);
            let played = out && started.is_none() && followed && age > 12;
            let kept = KEPT.contains(&episode.id.as_str());
            EpisodeState {
                id: episode.id.clone(),
                out,
                out_at: now - age * DAY,
                position: started.unwrap_or(0.0),
                played,
                starred: STARRED.contains(&episode.id.as_str()),
                inbox: out && followed && !played && age <= 12,
                download: if kept { DownloadState::Kept } else { DownloadState::Idle },
                progress: if kept { 1.0 } else { 0.0 },
                heard: if played {
                    Some(now - (age - 1) * DAY)
                } else {
                    started.map(|_| now - DAY * if episode.id == "low-tide-hours-3" { 0 } else { 3 })
                },
            }
        })
        .collect()
}

pub fn seed_player() -> PlayerRow {
    PlayerRow {
        id: "player".to_owned(),
        current: Some("low-tide-hours-3".to_owned()),
        queue: vec![
            "tidevann-weekly-3".to_owned(),
            "juniper-weather-3".to_owned(),
            "field-notes-larkspur-3".to_owned(),
        ],
        rate: 1.0,
        volume: 0.8,
        muted: false,
    }
}

/// Two weeks of listening, made up the same way every time.
pub fn seed_listens(now: i64) -> Vec<Listen> {
    let mut random = Seeded::new("listens");
    let mut shows: Vec<&str> = FOLLOWED.to_vec();
    shows.extend(["islands-one-by-one", "lighthouse-stories"]);
    let mut rows: Vec<Listen> = Vec::new();
    for offset in -13..=-1 {
        let count = 1 + (random.next_f64() * 3.0).floor() as usize;
        for _ in 0..count {
            let show = shows[((random.next_f64() * shows.len() as f64).floor() as usize).min(shows.len() - 1)];
            let date = day(offset, now);
            let id = format!("{date}:{show}");
            let seconds = crate::js::round(60.0 + random.next_f64() * 420.0) as i64;
            let fast = if random.next_f64() < 0.4 { 1.25 } else { 1.0 };
            let content = crate::js::round(seconds as f64 * fast) as i64;
            if let Some(found) = rows.iter_mut().find(|row| row.id == id) {
                found.seconds += seconds;
                found.content += content;
                continue;
            }
            rows.push(Listen {
                id,
                day: date,
                show: show.to_owned(),
                seconds,
                content,
                finished: (random.next_f64() * 3.0).floor() as i64,
            });
        }
    }
    rows
}

/// The size an episode takes on the device, in megabytes, as if encoded at 64 kbit/s.
pub fn size_of(id: &str) -> f64 {
    if episode_by_id(id).is_some() {
        (length_of(id) * 64.0) / 8.0 / 1024.0
    } else {
        0.0
    }
}
