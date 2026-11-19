//! One playlist, or Watch later, as the web's `pages/Playlist.tsx` keeps it: played in order or
//! shuffled, renamed, its videos put in order by drag or from their menus, removed, or deleted.

use serde::Serialize;

use crate::catalog::{Video, seed_of, video_by_id};
use crate::data::{Data, Playlist, PlaylistPatch, Then, Toast, WATCH_LATER};
use crate::format::plural;
use crate::js::trim;
use crate::parts::{path_of, watch_href};

/// One video's menu entries on the page: Move up and Move down, and whether each is off.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Entry {
    pub id: &'static str,
    pub up_disabled: bool,
    pub down_disabled: bool,
    pub remove: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlaylistView {
    pub name: String,
    /// Whether it is Watch later, whose name and words are not the viewer's to change.
    pub later: bool,
    pub count: String,
    pub description: String,
    pub play_disabled: bool,
    pub shuffle_disabled: bool,
    pub deletable: bool,
    pub videos: Vec<Entry>,
    /// What it says when it holds nothing.
    pub empty: Option<String>,
    /// What Delete asks.
    pub question: String,
}

/// What the page shows: none while the playlists are not yet answered or there is no such list.
pub fn view(data: &Data, id: &str) -> Option<PlaylistView> {
    let list = data.playlists.seen_get(id)?;
    let later = list.id == WATCH_LATER;
    let videos = videos_of(list);
    let last = videos.len().saturating_sub(1);
    Some(PlaylistView {
        name: list.name.clone(),
        later,
        count: plural(videos.len() as f64, "video", None),
        description: if later {
            "Videos you saved to watch later.".to_owned()
        } else {
            list.description.clone()
        },
        play_disabled: videos.is_empty(),
        shuffle_disabled: videos.len() < 2,
        deletable: !later,
        videos: videos
            .iter()
            .enumerate()
            .map(|(index, video)| Entry {
                id: video.id,
                up_disabled: index == 0,
                down_disabled: index == last,
                remove: format!("Remove from {}", list.name),
            })
            .collect(),
        empty: videos
            .is_empty()
            .then(|| format!("Save a video to {} from its menu or from Save on its page.", list.name)),
        question: format!("Delete {}?", list.name),
    })
}

/// The list's videos the catalog has, in its order.
pub fn videos_of(list: &Playlist) -> Vec<&'static Video> {
    list.videos.iter().filter_map(|id| video_by_id(id)).collect()
}

fn shown(data: &Data, id: &str) -> Option<Playlist> {
    data.playlists.seen_get(id).cloned()
}

/// What is wrong with a new name, if anything.
pub fn validate_name(name: &str) -> Option<&'static str> {
    trim(name).is_empty().then_some("A playlist needs a name.")
}

/// The title's inline edit.
pub fn rename(data: &mut Data, id: &str, name: &str) {
    let patch = PlaylistPatch {
        name: Some(trim(name).to_owned()),
        ..PlaylistPatch::default()
    };
    data.save_playlist(id, patch, Vec::new());
}

/// The description's inline edit.
pub fn describe(data: &mut Data, id: &str, description: &str) {
    let patch = PlaylistPatch {
        description: Some(trim(description).to_owned()),
        ..PlaylistPatch::default()
    };
    data.save_playlist(id, patch, Vec::new());
}

fn save_videos(data: &mut Data, id: &str, videos: Vec<String>, then: Vec<Then>) {
    let patch = PlaylistPatch {
        videos: Some(videos),
        ..PlaylistPatch::default()
    };
    data.save_playlist(id, patch, then);
}

/// A video's Move up (`by` -1) or Move down (`by` 1), by its place among the list's ids.
pub fn move_by(data: &mut Data, id: &str, index: usize, by: i64) {
    let Some(list) = shown(data, id) else { return };
    let mut order = list.videos.clone();
    if index >= order.len() {
        return;
    }
    let taken = order.remove(index);
    // `splice` counts a place before the start from the end, and holds one past it at the end.
    let start = index as i64 + by;
    let len = order.len() as i64;
    let at = if start < 0 {
        (len + start).max(0)
    } else {
        start.min(len)
    } as usize;
    order.insert(at, taken);
    save_videos(data, id, order, Vec::new());
}

/// A drag in the list, from one place to another among its videos.
pub fn drag(data: &mut Data, id: &str, from: usize, to: usize) {
    let Some(list) = shown(data, id) else { return };
    let mut videos: Vec<String> = videos_of(&list).iter().map(|one| one.id.to_owned()).collect();
    sokkel_egui::move_item(&mut videos, from, to);
    save_videos(data, id, videos, Vec::new());
}

/// A video's Remove from, with a note once kept.
pub fn remove(data: &mut Data, id: &str, video: &str) {
    let Some(list) = shown(data, id) else { return };
    let videos = list.videos.iter().filter(|one| *one != video).cloned().collect();
    save_videos(
        data,
        id,
        videos,
        vec![Then::Toast(Toast::plain(format!("Removed from {}", list.name)))],
    );
}

/// The shuffle's seed: the clock in steps of 1024 milliseconds, as `Date.now() >> 10` reads it.
pub fn shuffle_seed(now: i64) -> i32 {
    (now as i32) >> 10
}

/// Shuffle: the list in a stirred order, kept, then its first video played.
pub fn shuffle(data: &mut Data, id: &str) {
    let Some(list) = shown(data, id) else { return };
    let seed = shuffle_seed(data.now());
    let mut order = videos_of(&list);
    order.sort_by_key(|video| seed_of(&format!("{}{seed}", video.id)) % 101);
    let then = order
        .first()
        .map(|first| Then::Go(path_of(&watch_href(first, Some(&list.id)))))
        .into_iter()
        .collect();
    save_videos(data, id, order.iter().map(|one| one.id.to_owned()).collect(), then);
}

/// Play all: the first video, inside the list.
pub fn play_all(data: &mut Data, id: &str) {
    let Some(list) = shown(data, id) else { return };
    if let Some(first) = videos_of(&list).first() {
        data.go(path_of(&watch_href(first, Some(&list.id))));
    }
}

/// Delete's confirm: the list goes, with a note once it is gone, and the page goes to Playlists.
pub fn delete(data: &mut Data, id: &str) {
    let Some(list) = shown(data, id) else { return };
    data.delete_playlist(id, Some(Toast::plain(format!("{} deleted", list.name))));
    data.go("/playlists");
}
