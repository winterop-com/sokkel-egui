//! What a search finds, as the web's `pages/Search.tsx` finds it: channels with Follow, and
//! videos, narrowed to one kind and sorted by relevance, date or views.

use serde::Serialize;

use crate::catalog::{CHANNELS, Channel, VIDEOS, Video, videos_of};
use crate::data::{Data, Toast};
use crate::format::{plural, short};
use crate::js::words_of;
use crate::site::video_text;

/// Which results a search shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    All,
    Videos,
    Channels,
}

impl Kind {
    pub fn value(self) -> &'static str {
        match self {
            Kind::All => "all",
            Kind::Videos => "videos",
            Kind::Channels => "channels",
        }
    }
}

/// How the videos found are sorted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Relevance,
    Date,
    Views,
}

/// The sorts offered, with their words.
pub const SORTS: [(Sort, &str); 3] = [
    (Sort::Relevance, "Relevance"),
    (Sort::Date, "Upload date"),
    (Sort::Views, "Views"),
];

/// How well a text answers the words: each word found counts, more in the title; a word not found
/// at all makes it none.
pub fn score(title: &str, rest: &str, words: &[String]) -> i64 {
    let head = title.to_lowercase();
    let body = rest.to_lowercase();
    let mut total = 0;
    for word in words {
        if head.contains(word.as_str()) {
            total += 3;
        } else if body.contains(word.as_str()) {
            total += 1;
        } else {
            return 0;
        }
    }
    total
}

/// The videos a query finds, in the sort's order.
pub fn videos_found(query: &str, sort: Sort) -> Vec<&'static Video> {
    let words = words_of(query);
    let mut found: Vec<(&Video, i64)> = VIDEOS
        .iter()
        .map(|video| (video, score(video.title, &video_text(video), &words)))
        .filter(|(_, score)| *score > 0)
        .collect();
    match sort {
        Sort::Date => found.sort_by(|left, right| left.0.days.total_cmp(&right.0.days)),
        Sort::Views => found.sort_by_key(|one| std::cmp::Reverse(one.0.views)),
        Sort::Relevance => found.sort_by_key(|one| std::cmp::Reverse(one.1)),
    }
    found.into_iter().map(|(video, _)| video).collect()
}

/// The channels a query finds.
pub fn channels_found(query: &str) -> Vec<&'static Channel> {
    let words = words_of(query);
    CHANNELS
        .iter()
        .filter(|channel| {
            score(
                channel.name,
                &format!("{} {} {}", channel.handle, channel.about, channel.region),
                &words,
            ) > 0
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChannelResult {
    pub id: &'static str,
    pub detail: String,
    pub following: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SearchView {
    pub title: String,
    pub count: String,
    /// The chips' words: All, Videos and Channels with how many of each.
    pub chips: Vec<String>,
    pub channels: Vec<ChannelResult>,
    pub videos: Vec<&'static str>,
    /// What it says when it shows nothing: its title and words.
    pub empty: Option<(&'static str, String)>,
}

pub fn view(data: &Data, query: &str, kind: Kind, sort: Sort) -> SearchView {
    let videos = videos_found(query, sort);
    let channels = channels_found(query);
    let show_channels = kind != Kind::Videos && !channels.is_empty();
    let show_videos = kind != Kind::Channels && !videos.is_empty();
    let empty = (!show_channels && !show_videos).then(|| {
        if query.is_empty() {
            ("Search the site", "Type a few words above and press Enter.".to_owned())
        } else {
            (
                "Nothing found",
                format!(
                    "No {} match \"{query}\". Try fewer or other words.",
                    if kind == Kind::All {
                        "videos or channels"
                    } else {
                        kind.value()
                    }
                ),
            )
        }
    });
    SearchView {
        title: if query.is_empty() {
            "Search".to_owned()
        } else {
            format!("Results for \"{query}\"")
        },
        count: plural((videos.len() + channels.len()) as f64, "result", None),
        chips: vec![
            "All".to_owned(),
            format!("Videos {}", videos.len()),
            format!("Channels {}", channels.len()),
        ],
        channels: if show_channels {
            channels
                .iter()
                .map(|channel| ChannelResult {
                    id: channel.id,
                    detail: format!(
                        "{}, {} followers, {}",
                        channel.handle,
                        short(channel.followers as f64),
                        plural(videos_of(channel).len() as f64, "video", None)
                    ),
                    following: data.follows.seen_get(channel.id).is_some(),
                })
                .collect()
        } else {
            Vec::new()
        },
        videos: if show_videos {
            videos.iter().map(|video| video.id).collect()
        } else {
            Vec::new()
        },
        empty,
    }
}

/// Follows a channel or stops, from a channel's row, and says so at once.
pub fn follow(data: &mut Data, channel: &Channel, on: bool) {
    data.set_following(channel.id, on);
    data.toast(Toast::plain(if on {
        format!("Following {}", channel.name)
    } else {
        format!("Stopped following {}", channel.name)
    }));
}
