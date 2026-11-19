//! The dialogs any page opens for a video, as the web's `dialogs.tsx` makes them: Save to, for
//! adding it to Watch later and the viewer's playlists, and Share, with its link.

use serde::Serialize;

use crate::catalog::Video;
use crate::data::{Data, Then, Toast, WATCH_LATER};
use crate::format::clock_of;
use crate::js::trim;

/// The video's own address on the site, starting at a second when given; `base` is the site's
/// address before its `#`.
pub fn link_of(base: &str, video: &Video, at: Option<f64>) -> String {
    let link = format!("{base}#/watch/{}", video.id);
    match at {
        Some(at) if at >= 1.0 => format!("{link}/at/{}", crate::js::number_to_string(at.floor())),
        _ => link,
    }
}

/// One playlist in Save to: its name, how many videos, and whether the video is in it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SaveEntry {
    pub id: String,
    pub label: String,
    pub description: String,
    pub checked: bool,
}

/// The playlists Save to offers, Watch later first and the rest as they were made.
pub fn save_entries(data: &Data, video: &Video) -> Vec<SaveEntry> {
    let mut lists: Vec<_> = data.playlists.seen().iter().collect();
    lists.sort_by(|left, right| {
        (right.id == WATCH_LATER)
            .cmp(&(left.id == WATCH_LATER))
            .then(left.created.cmp(&right.created))
    });
    lists
        .into_iter()
        .map(|list| SaveEntry {
            id: list.id.clone(),
            label: list.name.clone(),
            description: format!(
                "{} {}",
                list.videos.len(),
                if list.videos.len() == 1 { "video" } else { "videos" }
            ),
            checked: list.videos.iter().any(|one| one == video.id),
        })
        .collect()
}

/// Ticks a playlist in Save to, or unticks it, and says so once kept.
pub fn save_toggle(data: &mut Data, list: &str, video: &Video, on: bool) {
    let Some(name) = data.playlists.seen_get(list).map(|one| one.name.clone()) else {
        return;
    };
    let toast = Toast::plain(if on {
        format!("Saved to {name}")
    } else {
        format!("Removed from {name}")
    });
    data.toggle_in(list, video.id, on, Some(toast));
}

/// What is wrong with a new playlist's name, if anything, among the playlists the page shows.
pub fn validate_name(data: &Data, name: &str) -> Option<&'static str> {
    if trim(name).is_empty() {
        Some("A playlist needs a name.")
    } else if data.playlists.seen().iter().any(|list| list.name == trim(name)) {
        Some("There is a playlist with that name.")
    } else {
        None
    }
}

/// Makes a playlist from Save to, with the video in it, and says so once made; its id.
pub fn save_new(data: &mut Data, name: &str, video: &Video) -> String {
    data.new_playlist(name, Some(video.id), |list| {
        vec![Then::Toast(Toast::plain(format!("Saved to {}", list.name)))]
    })
}

/// Share's link and its one choice: starting at the second the video was shared at.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Share {
    pub title: &'static str,
    pub link: String,
    /// The words of Start at, when the video was shared a second or more in.
    pub start_at: Option<String>,
}

pub fn share(base: &str, video: &Video, at: Option<f64>, from_here: bool) -> Share {
    Share {
        title: video.title,
        link: link_of(base, video, if from_here { at } else { None }),
        start_at: at
            .filter(|at| *at >= 1.0)
            .map(|at| format!("Start at {}", clock_of(at))),
    }
}
