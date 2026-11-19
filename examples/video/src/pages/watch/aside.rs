//! What plays next beside the player, as the web's `pages/watch/Aside.tsx` orders it: its
//! channel's first, then its topic's, then the rest, the same order every time; narrowed by chips,
//! or the playlist being played.

use serde::Serialize;

use crate::catalog::{VIDEOS, Video, channel_of, seed_of, video_by_id};
use crate::data::{Data, Playlist};

/// What plays after `video`: its channel's first, then its topic's, then the rest, stirred by a
/// seed of both ids.
pub fn up_next_for(video: &Video) -> Vec<&'static Video> {
    let score = |one: &Video| i64::from(one.channel == video.channel) * 2 + i64::from(one.topic == video.topic);
    let mut out: Vec<&Video> = VIDEOS.iter().filter(|one| one.id != video.id).collect();
    out.sort_by_key(|one| (-score(one), seed_of(&format!("{}{}", one.id, video.id)) % 997));
    out
}

/// The next video: the playlist's next while playing a playlist, else the first of what plays next.
pub fn next_of(video: &Video, list: Option<&Playlist>, shown: &[&'static Video]) -> Option<&'static Video> {
    match list {
        Some(list) => {
            let at = list.videos.iter().position(|one| one == video.id)?;
            list.videos.get(at + 1).and_then(|id| video_by_id(id))
        }
        None => shown.first().copied(),
    }
}

/// What Up next is narrowed to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Channel,
    Topic,
    Unwatched,
}

impl Filter {
    pub fn value(self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::Channel => "channel",
            Filter::Topic => "topic",
            Filter::Unwatched => "unwatched",
        }
    }
}

/// One video beside the player: drawn as a row's card (`parts::video_card`), inside the playlist
/// when one plays.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AsideEntry {
    pub id: &'static str,
    pub list: Option<String>,
    pub playing: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AsideView {
    /// `Up next`, or the playlist's name.
    pub title: String,
    /// The playlist's place: `Playlist` and `2 of 5`.
    pub meta: Option<Vec<String>>,
    /// The chips' words, when not in a playlist.
    pub chips: Vec<String>,
    pub autoplay: bool,
    pub videos: Vec<AsideEntry>,
}

/// What the aside shows beside `video`, inside `list` when given, narrowed by `filter`.
pub fn view(data: &Data, video: &Video, list: Option<&Playlist>, filter: Filter) -> AsideView {
    let autoplay = data.prefs.autoplay;
    if let Some(list) = list {
        let videos: Vec<&'static Video> = list.videos.iter().filter_map(|id| video_by_id(id)).collect();
        let at = videos
            .iter()
            .position(|one| one.id == video.id)
            .map_or(-1, |at| at as i64);
        return AsideView {
            title: list.name.clone(),
            meta: Some(vec!["Playlist".to_owned(), format!("{} of {}", at + 1, videos.len())]),
            chips: Vec::new(),
            autoplay,
            videos: videos
                .iter()
                .map(|one| AsideEntry {
                    id: one.id,
                    list: Some(list.id.clone()),
                    playing: one.id == video.id,
                })
                .collect(),
        };
    }
    let shown = up_next_for(video).into_iter().filter(|one| match filter {
        Filter::All => true,
        Filter::Channel => one.channel == video.channel,
        Filter::Topic => one.topic == video.topic,
        Filter::Unwatched => data.history.seen_get(one.id).is_none_or(|seen| !seen.finished),
    });
    AsideView {
        title: "Up next".to_owned(),
        meta: None,
        chips: vec![
            "All".to_owned(),
            format!("From {}", channel_of(video).name),
            video.topic.label().to_owned(),
            "Not watched".to_owned(),
        ],
        autoplay,
        videos: shown
            .map(|one| AsideEntry {
                id: one.id,
                list: None,
                playing: false,
            })
            .collect(),
    }
}
