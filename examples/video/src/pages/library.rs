//! Everything the viewer keeps on one page, as the web's `pages/Library.tsx` shows it: the latest
//! of their history, Watch later, their playlists, likes and downloads, each with See all.

use serde::Serialize;

use crate::catalog::video_by_id;
use crate::data::{Data, RatingKind, WATCH_LATER};
use crate::format::plural;
use crate::pages::playlists::{PlaylistCard, card};

/// How many cards each part of the library shows before See all.
pub const SHOWN: usize = 4;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Part {
    pub title: &'static str,
    pub meta: String,
    /// Where See all goes.
    pub all: &'static str,
    /// The first few of its videos.
    pub videos: Vec<&'static str>,
    /// Its playlists' cards, for Playlists.
    pub lists: Vec<PlaylistCard>,
    /// What it says when it holds nothing.
    pub empty: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LibraryView {
    pub loading: bool,
    pub parts: Vec<Part>,
}

fn known<'a>(ids: impl Iterator<Item = &'a str>) -> Vec<&'static str> {
    ids.filter_map(video_by_id).map(|one| one.id).collect()
}

pub fn view(data: &Data) -> LibraryView {
    let mut recent: Vec<_> = data.history.seen().iter().collect();
    recent.sort_by_key(|one| std::cmp::Reverse(one.at));
    let recent = known(recent.iter().map(|one| one.id.as_str()));
    let lists = data.playlists.seen();
    let later = known(
        lists
            .iter()
            .find(|one| one.id == WATCH_LATER)
            .map(|one| one.videos.iter().map(String::as_str).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter(),
    );
    let likes = known(
        data.ratings
            .seen()
            .iter()
            .filter(|one| one.rating == RatingKind::Like)
            .map(|one| one.id.as_str()),
    );
    let own: Vec<_> = lists.iter().filter(|one| one.id != WATCH_LATER).collect();
    let offline = known(data.downloads.seen().iter().map(|one| one.id.as_str()));
    let part = |title, all, videos: Vec<&'static str>, empty| Part {
        title,
        meta: plural(videos.len() as f64, "video", None),
        all,
        empty: videos.is_empty().then_some(empty),
        videos: videos.into_iter().take(SHOWN).collect(),
        lists: Vec::new(),
    };
    LibraryView {
        loading: !data.history.ready() || !data.playlists.ready(),
        parts: vec![
            part("History", "/history", recent, "Videos you watch land here."),
            part("Watch later", "/later", later, "Save a video for later from its menu."),
            Part {
                title: "Playlists",
                meta: plural(own.len() as f64, "playlist", None),
                all: "/playlists",
                videos: Vec::new(),
                empty: own.is_empty().then_some("Make a playlist from Save on any video."),
                lists: own.iter().take(SHOWN).map(|list| card(list, data.now())).collect(),
            },
            part("Liked videos", "/liked", likes, "Videos you like land here."),
            part(
                "Downloads",
                "/downloads",
                offline,
                "Download a video from its page to keep it on this device.",
            ),
        ],
    }
}
