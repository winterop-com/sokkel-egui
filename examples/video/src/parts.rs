//! What several pages draw, as the web's `parts.tsx` makes it: where a video is watched, how many
//! have watched it and how far the viewer got, a video's card and the entries of its menu.

use serde::Serialize;
use sokkel_egui::format_time;

use crate::catalog::{Video, channel_of};
use crate::data::{Data, Toast, WATCH_LATER, Watched};
use crate::format::{days_ago, views_of};

/// Where a video is watched, inside a playlist when given.
pub fn watch_href(video: &Video, list: Option<&str>) -> String {
    match list {
        Some(list) => format!("#/watch/{}/list/{list}", video.id),
        None => format!("#/watch/{}", video.id),
    }
}

/// The path `go` is handed for an address: the address without its `#`.
pub fn path_of(href: &str) -> String {
    href.strip_prefix('#').unwrap_or(href).to_owned()
}

/// How many have watched a video: the count it had, and the viewer's own views.
pub fn view_count(video: &Video, seen: Option<&Watched>) -> f64 {
    video.views as f64 + seen.map_or(0.0, |one| one.times as f64)
}

/// How far the viewer got: the share of a video they watched, all of it once finished.
pub fn progress_of(video: &Video, seen: Option<&Watched>) -> Option<f64> {
    let seen = seen?;
    if seen.finished {
        return Some(1.0);
    }
    Some(seen.position / video.duration)
}

/// How a card is laid out: its picture beside its words, or above them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardLayout {
    #[default]
    Stack,
    Row,
}

/// One video as a card: its words, its badge, how far the viewer got and where it goes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VideoCard {
    pub id: &'static str,
    pub title: &'static str,
    /// The channel's name on a line of its own, in a row that names it.
    pub channel: Option<&'static str>,
    /// The meta line: the channel (in a grid), the views and the age.
    pub meta: Vec<String>,
    /// The length, over the picture.
    pub badge: String,
    pub progress: Option<f64>,
    pub playing: bool,
    pub href: String,
}

/// A video's card as the page shows it: `bare` leaves out the channel's name, on the channel's
/// own page; `list` plays it inside that playlist; `playing` marks the one playing now.
pub fn video_card(
    data: &Data,
    video: &'static Video,
    layout: CardLayout,
    bare: bool,
    list: Option<&str>,
    playing: bool,
) -> VideoCard {
    let seen = data.history.seen_get(video.id);
    let channel = channel_of(video);
    let facts = vec![views_of(view_count(video, seen)), days_ago(video.days)];
    let (channel_line, meta) = if layout == CardLayout::Row && !bare {
        (Some(channel.name), facts)
    } else if bare {
        (None, facts)
    } else {
        (None, [vec![channel.name.to_owned()], facts].concat())
    };
    VideoCard {
        id: video.id,
        title: video.title,
        channel: channel_line,
        meta,
        badge: format_time(video.duration),
        progress: progress_of(video, seen),
        playing,
        href: watch_href(video, list),
    }
}

/// Whether a video is in Watch later, as the page shows the list.
pub fn in_watch_later(data: &Data, video: &Video) -> bool {
    data.playlists
        .seen_get(WATCH_LATER)
        .is_some_and(|later| later.videos.iter().any(|one| one == video.id))
}

/// The entries of a video's menu, before the page's own: Watch later, Save to, Share and its
/// channel.
pub fn video_menu(data: &Data, video: &Video) -> Vec<String> {
    vec![
        if in_watch_later(data, video) {
            "Remove from Watch later".to_owned()
        } else {
            "Save to Watch later".to_owned()
        },
        "Save to playlist".to_owned(),
        "Share".to_owned(),
        format!("Go to {}", channel_of(video).name),
    ]
}

/// The menu's first entry: saves the video to Watch later, or takes it out, and says so once kept.
pub fn toggle_watch_later(data: &mut Data, video: &Video) {
    if data.playlists.seen_get(WATCH_LATER).is_none() {
        return;
    }
    let saved = in_watch_later(data, video);
    let toast = Toast::plain(if saved {
        "Removed from Watch later"
    } else {
        "Saved to Watch later"
    });
    data.toggle_in(WATCH_LATER, video.id, !saved, Some(toast));
}

/// The menu's last entry: goes to the video's channel.
pub fn go_to_channel(data: &mut Data, video: &Video) {
    data.go(format!("/channel/{}", video.channel));
}
