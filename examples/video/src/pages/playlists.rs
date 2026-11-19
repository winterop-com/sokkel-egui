//! The viewer's playlists, as the web's `pages/Playlists.tsx` shows them: Watch later first, the
//! rest by when they last changed, each as a card, and a field to make a new one.

use serde::Serialize;

use crate::catalog::video_by_id;
use crate::data::{DAY, Data, Playlist, Then, Toast, WATCH_LATER};
use crate::format::{days_ago, plural};
use crate::parts::{path_of, watch_href};

/// A playlist as a card: its first video's still, its name, how many videos and when it last
/// changed.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlaylistCard {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    /// The video whose still is its cover, if it holds any.
    pub cover: Option<&'static str>,
    pub badge: String,
    pub href: String,
    pub deletable: bool,
}

pub fn card(list: &Playlist, now: i64) -> PlaylistCard {
    PlaylistCard {
        id: list.id.clone(),
        title: list.name.clone(),
        subtitle: format!(
            "{}, updated {}",
            plural(list.videos.len() as f64, "video", None),
            days_ago((now - list.updated) as f64 / DAY as f64)
        ),
        cover: list.videos.first().and_then(|id| video_by_id(id)).map(|one| one.id),
        badge: list.videos.len().to_string(),
        href: if list.id == WATCH_LATER {
            "#/later".to_owned()
        } else {
            format!("#/playlist/{}", list.id)
        },
        deletable: list.id != WATCH_LATER,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlaylistsView {
    pub loading: bool,
    pub count: String,
    pub cards: Vec<PlaylistCard>,
}

pub fn view(data: &Data) -> PlaylistsView {
    let rows = data.playlists.seen();
    let mut ordered: Vec<&Playlist> = rows.iter().collect();
    ordered.sort_by(|left, right| {
        (right.id == WATCH_LATER)
            .cmp(&(left.id == WATCH_LATER))
            .then(right.updated.cmp(&left.updated))
    });
    PlaylistsView {
        loading: !data.playlists.ready(),
        count: plural(
            rows.iter().filter(|one| one.id != WATCH_LATER).count() as f64,
            "playlist",
            None,
        ),
        cards: ordered.into_iter().map(|list| card(list, data.now())).collect(),
    }
}

/// The field's add: makes the playlist, and once made says so with Open.
pub fn add(data: &mut Data, name: &str) -> String {
    data.new_playlist(name, None, |list| {
        vec![Then::Toast(Toast {
            action: Some("Open".to_owned()),
            ..Toast::plain(format!("{} made", list.name))
        })]
    })
}

/// A card's Play all: its first video, inside the list.
pub fn play(data: &mut Data, list: &Playlist) {
    if let Some(first) = list.videos.first().and_then(|id| video_by_id(id)) {
        data.go(path_of(&watch_href(first, Some(&list.id))));
    }
}

/// A card's Delete, with a note once it is gone.
pub fn delete(data: &mut Data, list: &Playlist) {
    data.delete_playlist(&list.id, Some(Toast::plain(format!("{} deleted", list.name))));
}
