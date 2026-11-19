//! Videos picked for the viewer, as the web's `pages/Home.tsx` picks them: under chips that narrow
//! them to a topic, the channels they follow, or what they left halfway.

use serde::Serialize;

use crate::catalog::{TOPICS, Topic, VIDEOS, Video, seed_of};
use crate::data::Data;

/// The chip a viewer narrows Home by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    All,
    Following,
    Continue,
    Recent,
    Topic(Topic),
}

impl Chip {
    pub fn value(self) -> &'static str {
        match self {
            Chip::All => "all",
            Chip::Following => "following",
            Chip::Continue => "continue",
            Chip::Recent => "recent",
            Chip::Topic(topic) => topic.id(),
        }
    }

    pub fn from_value(value: &str) -> Option<Chip> {
        chips()
            .into_iter()
            .map(|(chip, _)| chip)
            .find(|chip| chip.value() == value)
    }

    /// What Home says when the chip holds nothing.
    pub fn says(self) -> &'static str {
        match self {
            Chip::All => "Nothing here yet.",
            Chip::Following => "Follow a channel and its videos land here.",
            Chip::Continue => "Videos you started and did not finish land here, ready to pick up where you stopped.",
            Chip::Recent => "Nothing was put up in the last two weeks.",
            Chip::Topic(_) => "No videos on this topic yet.",
        }
    }
}

/// The chips, in the web's order, with their words.
pub fn chips() -> Vec<(Chip, &'static str)> {
    let mut out = vec![
        (Chip::All, "All"),
        (Chip::Following, "Following"),
        (Chip::Continue, "Continue watching"),
        (Chip::Recent, "Recently uploaded"),
    ];
    out.extend(TOPICS.iter().map(|(topic, label)| (Chip::Topic(*topic), *label)));
    out
}

/// The home grid's order: newer first, stirred by a fixed seed so channels mix.
pub fn order() -> Vec<&'static Video> {
    let mut out: Vec<&Video> = VIDEOS.iter().collect();
    out.sort_by_key(|video| video.days as i64 + i64::from(seed_of(video.id) % 9) * 4);
    out
}

/// Whether a video is shown under the chip.
pub fn holds(data: &Data, chip: Chip, video: &Video) -> bool {
    match chip {
        Chip::All => true,
        Chip::Following => data.follows.seen_get(video.channel).is_some(),
        Chip::Continue => data
            .history
            .seen_get(video.id)
            .is_some_and(|seen| !seen.finished && seen.position > 0.0),
        Chip::Recent => video.days <= 14.0,
        Chip::Topic(topic) => video.topic == topic,
    }
}

/// What Home shows under a chip.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomeView {
    pub loading: bool,
    pub videos: Vec<&'static str>,
    /// What it says when it shows nothing.
    pub empty: Option<&'static str>,
}

pub fn view(data: &Data, chip: Chip) -> HomeView {
    let videos: Vec<&'static str> = order()
        .into_iter()
        .filter(|video| holds(data, chip, video))
        .map(|video| video.id)
        .collect();
    HomeView {
        loading: !data.history.ready(),
        empty: videos.is_empty().then(|| chip.says()),
        videos,
    }
}
