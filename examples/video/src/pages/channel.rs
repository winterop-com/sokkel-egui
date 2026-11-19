//! A channel's own page, as the web's `pages/Channel.tsx` shows it: its name with Follow, then its
//! videos, sorted, or what it is about.

use serde::Serialize;

use crate::catalog::{Channel, Video, videos_of};
use crate::data::Data;
use crate::format::{days_ago, plural, short};
use crate::parts::{path_of, view_count, watch_href};

/// How the channel's videos are sorted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Latest,
    Popular,
    Oldest,
}

pub const SORTS: [(Sort, &str); 3] = [
    (Sort::Latest, "Latest"),
    (Sort::Popular, "Popular"),
    (Sort::Oldest, "Oldest"),
];

/// The channel's videos in the sort's order.
pub fn sorted(channel: &Channel, sort: Sort) -> Vec<&'static Video> {
    let mut videos = videos_of(channel);
    match sort {
        Sort::Latest => videos.sort_by(|left, right| left.days.total_cmp(&right.days)),
        Sort::Oldest => videos.sort_by(|left, right| right.days.total_cmp(&left.days)),
        Sort::Popular => videos.sort_by_key(|one| std::cmp::Reverse(one.views)),
    }
    videos
}

/// The channel's newest video.
pub fn newest(channel: &Channel) -> Option<&'static Video> {
    sorted(channel, Sort::Latest).first().copied()
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChannelView {
    pub detail: String,
    pub following: bool,
    pub videos: Vec<&'static str>,
    /// About's terms and what each says.
    pub about: Vec<(&'static str, String)>,
}

/// What the page shows; `link` is the channel's own address.
pub fn view(data: &Data, channel: &Channel, sort: Sort) -> ChannelView {
    let following = data.follows.seen_get(channel.id).is_some();
    let videos = videos_of(channel);
    let total: f64 = videos
        .iter()
        .map(|video| view_count(video, data.history.seen_get(video.id)))
        .sum();
    let newest = newest(channel);
    ChannelView {
        detail: format!(
            "{}, {} followers, {}",
            channel.handle,
            short((channel.followers + i64::from(following)) as f64),
            plural(videos.len() as f64, "video", None)
        ),
        following,
        videos: sorted(channel, sort).iter().map(|one| one.id).collect(),
        about: vec![
            ("Where it films", channel.region.to_owned()),
            ("Started", days_ago(channel.started as f64)),
            ("Videos", videos.len().to_string()),
            ("Views", format!("{} in all", short(total))),
            (
                "Latest video",
                newest.map_or("None yet".to_owned(), |one| {
                    format!("{}, {}", one.title, days_ago(one.days))
                }),
            ),
            ("Link", channel.handle.to_owned()),
        ],
    }
}

/// The channel's own address, after the site's address `base`.
pub fn link_of(base: &str, channel: &Channel) -> String {
    format!("{base}#/channel/{}", channel.id)
}

/// Play: the first video in the sort's order.
pub fn play(data: &mut Data, channel: &Channel, sort: Sort) {
    if let Some(first) = sorted(channel, sort).first() {
        data.go(path_of(&watch_href(first, None)));
    }
}

/// The tabs: Videos, or About.
pub fn tab(data: &mut Data, channel: &Channel, about: bool) {
    data.go(if about {
        format!("/channel/{}/about", channel.id)
    } else {
        format!("/channel/{}", channel.id)
    });
}
