//! The site around every page, as the web's `Site.tsx` makes it: the rail of its places, the trail
//! along the top, the palette's own commands and its search, the search field's submit, and the
//! status line at the foot.

use serde::Serialize;

use crate::catalog::{CHANNELS, VIDEOS, Video, channel_by_id, channel_of, video_by_id};
use crate::data::{Data, PrefsPatch, RatingKind, WATCH_LATER};
use crate::format::{bytes_of, short};
use crate::js::{encode_uri_component, number_to_string, trim, words_of};

/// Whether every word of a query is found in the text; a query of no words matches nothing.
pub fn matches(text: &str, query: &str) -> bool {
    let words = words_of(query);
    let haystack = text.to_lowercase();
    !words.is_empty() && words.iter().all(|word| haystack.contains(word.as_str()))
}

/// What a video is found by: its title, channel, description and topic.
pub fn video_text(video: &Video) -> String {
    format!(
        "{} {} {} {}",
        video.title,
        channel_of(video).name,
        video.description,
        video.topic.id()
    )
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RailEntry {
    pub id: String,
    pub label: String,
    pub description: Option<&'static str>,
    pub href: String,
    pub active: bool,
    /// How many it holds, when more than none.
    pub count: Option<usize>,
    /// A channel's color, a kind color's name.
    pub dot: Option<&'static str>,
    pub keywords: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RailGroup {
    pub id: &'static str,
    pub title: Option<&'static str>,
    pub entries: Vec<RailEntry>,
}

/// The route's first two parts, `home` when it has none.
fn parts(route: &[String]) -> (&str, Option<&str>) {
    (
        route.first().map_or("home", String::as_str),
        route.get(1).map(String::as_str),
    )
}

/// The rail's groups: Browse, You, and the channels followed.
pub fn rail(data: &Data, route: &[String]) -> Vec<RailGroup> {
    let (first, second) = parts(route);
    let lists = data.playlists.seen();
    let later = lists.iter().find(|one| one.id == WATCH_LATER);
    let own = lists.iter().filter(|one| one.id != WATCH_LATER).count();
    let liked = data
        .ratings
        .seen()
        .iter()
        .filter(|one| one.rating == RatingKind::Like)
        .count();
    let active = |id: &str| first == id || (first == "playlist" && id == "playlists");
    let entry = |id: &str, label: &str, description: Option<&'static str>, count: Option<usize>| RailEntry {
        id: id.to_owned(),
        label: label.to_owned(),
        description,
        href: if id == "home" {
            "#/".to_owned()
        } else {
            format!("#/{id}")
        },
        active: active(id),
        count: count.filter(|count| *count != 0),
        dot: None,
        keywords: Vec::new(),
    };
    let followed = data.follows.seen().iter().filter_map(|one| channel_by_id(&one.id));
    vec![
        RailGroup {
            id: "browse",
            title: None,
            entries: vec![
                entry("home", "Home", Some("Videos picked for you, by topic"), None),
                entry(
                    "subscriptions",
                    "Subscriptions",
                    Some("The latest from the channels you follow"),
                    None,
                ),
                entry("library", "Library", Some("Everything you keep, in one place"), None),
            ],
        },
        RailGroup {
            id: "yours",
            title: Some("You"),
            entries: vec![
                entry("history", "History", None, None),
                entry("later", "Watch later", None, later.map(|one| one.videos.len())),
                entry("liked", "Liked videos", None, Some(liked)),
                entry("playlists", "Playlists", None, Some(own)),
                entry("downloads", "Downloads", None, Some(data.downloads.seen().len())),
            ],
        },
        RailGroup {
            id: "following",
            title: Some("Following"),
            entries: followed
                .map(|channel| RailEntry {
                    id: format!("channel-{}", channel.id),
                    label: channel.name.to_owned(),
                    description: None,
                    href: format!("#/channel/{}", channel.id),
                    active: first == "channel" && second == Some(channel.id),
                    count: None,
                    dot: Some(channel.color),
                    keywords: vec!["channel".to_owned(), channel.handle.to_owned()],
                })
                .collect(),
        },
    ]
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Crumb {
    pub label: String,
    pub href: Option<String>,
}

/// The trail along the top for a route.
pub fn trail(data: &Data, route: &[String]) -> Vec<Crumb> {
    let (first, second) = parts(route);
    let crumb = |label: &str, href: Option<String>| Crumb {
        label: label.to_owned(),
        href,
    };
    if first == "watch"
        && let Some(video) = second.and_then(video_by_id)
    {
        return vec![
            crumb(channel_of(video).name, Some(format!("#/channel/{}", video.channel))),
            crumb(video.title, None),
        ];
    }
    if first == "channel"
        && let Some(channel) = second.and_then(channel_by_id)
    {
        return vec![crumb(channel.name, None)];
    }
    if first == "playlist" {
        let name = second
            .and_then(|id| data.playlists.seen_get(id))
            .map_or("Playlist".to_owned(), |list| list.name.clone());
        return vec![crumb("Playlists", Some("#/playlists".to_owned())), crumb(&name, None)];
    }
    if first == "search" {
        return vec![crumb("Search", None)];
    }
    let label = rail(data, route)
        .into_iter()
        .flat_map(|group| group.entries)
        .find(|one| one.id == first)
        .map_or("Not found".to_owned(), |one| one.label);
    vec![crumb(&label, None)]
}

/// One of the palette's own commands.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub hint: Option<String>,
    pub keywords: Vec<&'static str>,
}

/// What the palette's own commands do.
#[derive(Clone, Debug, PartialEq)]
pub enum CommandAction {
    Go(String),
    Prefs(PrefsPatch),
}

/// The palette's Playlists group: the viewer's own playlists.
pub fn playlist_commands(data: &Data) -> Vec<(Command, CommandAction)> {
    data.playlists
        .seen()
        .iter()
        .filter(|one| one.id != WATCH_LATER)
        .map(|list| {
            (
                Command {
                    id: format!("playlist-{}", list.id),
                    label: list.name.clone(),
                    hint: Some(format!("{} videos", list.videos.len())),
                    keywords: vec!["playlist"],
                },
                CommandAction::Go(format!("/playlist/{}", list.id)),
            )
        })
        .collect()
}

/// The palette's Player group: autoplay, theater, the stats, the settings and clearing the history.
pub fn player_commands(data: &Data) -> Vec<(Command, CommandAction)> {
    let prefs = &data.prefs;
    let command = |id: &str, label: &str, keywords: Vec<&'static str>| Command {
        id: id.to_owned(),
        label: label.to_owned(),
        hint: None,
        keywords,
    };
    vec![
        (
            command(
                "autoplay",
                if prefs.autoplay {
                    "Turn autoplay off"
                } else {
                    "Turn autoplay on"
                },
                vec!["next", "up next"],
            ),
            CommandAction::Prefs(PrefsPatch {
                autoplay: Some(!prefs.autoplay),
                ..PrefsPatch::default()
            }),
        ),
        (
            command(
                "theater",
                if prefs.theater {
                    "Leave theater mode"
                } else {
                    "Theater mode"
                },
                Vec::new(),
            ),
            CommandAction::Prefs(PrefsPatch {
                theater: Some(!prefs.theater),
                ..PrefsPatch::default()
            }),
        ),
        (
            command(
                "stats",
                if prefs.stats {
                    "Hide the stats"
                } else {
                    "Show the stats"
                },
                Vec::new(),
            ),
            CommandAction::Prefs(PrefsPatch {
                stats: Some(!prefs.stats),
                ..PrefsPatch::default()
            }),
        ),
        (
            command("settings", "Settings", vec!["preferences", "captions", "quality"]),
            CommandAction::Go("/settings".to_owned()),
        ),
        (
            command("clear-history", "Clear watch history", vec!["history"]),
            CommandAction::Go("/history/clear".to_owned()),
        ),
    ]
}

/// Runs one of the palette's own commands.
pub fn run_command(data: &mut Data, action: &CommandAction) {
    match action {
        CommandAction::Go(path) => data.go(path.clone()),
        CommandAction::Prefs(patch) => data.set_prefs(patch),
    }
}

/// The palette's search: at least this many characters before it searches.
pub const SEARCH_MIN: usize = 2;
/// How long the palette waits after a key before it searches, in milliseconds.
pub const SEARCH_DEBOUNCE: u64 = 60;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SearchResult {
    pub id: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub meta: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SearchGroup {
    pub kind: &'static str,
    pub title: &'static str,
    pub total: usize,
    pub results: Vec<SearchResult>,
}

/// The palette's search: the first six videos found and every channel found.
pub fn palette_search(text: &str) -> Vec<SearchGroup> {
    let videos: Vec<&Video> = VIDEOS
        .iter()
        .filter(|video| matches(&video_text(video), text))
        .collect();
    let channels: Vec<_> = CHANNELS
        .iter()
        .filter(|channel| matches(&format!("{} {} {}", channel.name, channel.handle, channel.about), text))
        .collect();
    vec![
        SearchGroup {
            kind: "videos",
            title: "Videos",
            total: videos.len(),
            results: videos
                .iter()
                .take(6)
                .map(|video| SearchResult {
                    id: video.id,
                    title: video.title,
                    detail: channel_of(video).name,
                    meta: format!("{} views", short(video.views as f64)),
                })
                .collect(),
        },
        SearchGroup {
            kind: "channels",
            title: "Channels",
            total: channels.len(),
            results: channels
                .iter()
                .map(|channel| SearchResult {
                    id: channel.id,
                    title: channel.name,
                    detail: channel.handle,
                    meta: format!("{} followers", short(channel.followers as f64)),
                })
                .collect(),
        },
    ]
}

/// Opens a result of the palette's search: a channel's page, or the video.
pub fn open_result(data: &mut Data, kind: &str, id: &str) {
    if kind == "channels" {
        data.go(format!("/channel/{id}"));
    } else if let Some(video) = video_by_id(id) {
        data.go(crate::parts::path_of(&crate::parts::watch_href(video, None)));
    }
}

/// The search field's Enter: goes to the search's page, unless there is nothing to search for.
pub fn submit(data: &mut Data, query: &str) {
    let text = trim(query);
    if !text.is_empty() {
        data.go(format!("/search/{}", encode_uri_component(text)));
    }
}

/// The status line at the foot.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Status {
    pub note: &'static str,
    pub live: bool,
    pub watched: usize,
    pub downloaded: String,
}

/// What the foot says: each clip is painted here, so none is ever being made.
pub fn status(data: &Data) -> Status {
    let kept = data.downloads.seen();
    Status {
        note: "Every video is made in this browser",
        live: false,
        watched: data.history.seen().len(),
        downloaded: if kept.is_empty() {
            "0".to_owned()
        } else {
            format!(
                "{}, {}",
                number_to_string(kept.len() as f64),
                bytes_of(kept.iter().map(|one| one.bytes).sum())
            )
        },
    }
}
