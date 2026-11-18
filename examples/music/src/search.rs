//! One search over the library, as the web's `search.tsx` makes it for the top bar's field, the
//! palette and the results page: artists, albums and songs whose names hold every word typed, a
//! name starting with the query first. The results' artwork and icons are the app's to draw.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::catalog::{Album, Artist, Song, album_by_id, albums, artist_name, artists, songs};
use crate::js;

/// A text as the search compares it: its accents dropped, in lower case.
fn fold(text: &str) -> String {
    let bare: String = text.nfd().filter(|c| !is_combining_mark(*c)).collect();
    bare.to_lowercase()
}

/// How well `text` answers the query's words: 0 not at all, higher better.
fn score(text: &str, words: &[String], query: &str) -> f64 {
    let folded = fold(text);
    if !words.iter().all(|word| folded.contains(word.as_str())) {
        return 0.0;
    }
    if folded.starts_with(query) {
        return 3.0;
    }
    let first = words.first().map_or("", String::as_str);
    if folded.split(js::is_space).any(|part| part.starts_with(first)) {
        2.0
    } else {
        1.0
    }
}

/// What in the library answers a query, best first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Found {
    pub artists: Vec<&'static Artist>,
    pub albums: Vec<&'static Album>,
    pub songs: Vec<&'static Song>,
}

fn ranked<T>(
    items: &'static [T],
    words: &[String],
    query: &str,
    limit: usize,
    text: impl Fn(&T) -> String,
    also: impl Fn(&T) -> String,
) -> Vec<&'static T> {
    let mut scored: Vec<(&'static T, f64)> = items
        .iter()
        .map(|item| {
            let own = score(&text(item), words, query);
            let wider = if score(&format!("{} {}", text(item), also(item)), words, query) > 0.0 {
                0.5
            } else {
                0.0
            };
            (item, own.max(wider))
        })
        .filter(|(_, score)| *score > 0.0)
        .collect();
    scored.sort_by(|one, other| other.1.total_cmp(&one.1));
    scored.into_iter().take(limit).map(|(item, _)| item).collect()
}

/// What in the library answers `query`, best first, up to `limit` of each kind.
pub fn find(query: &str, limit: usize) -> Found {
    let folded = fold(js::trim(query));
    let words: Vec<String> = folded
        .split(js::is_space)
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect();
    if words.is_empty() {
        return Found::default();
    }
    Found {
        artists: ranked(
            artists(),
            &words,
            &folded,
            limit,
            |artist| artist.name.clone(),
            |_| String::new(),
        ),
        albums: ranked(
            albums(),
            &words,
            &folded,
            limit,
            |album| album.title.clone(),
            |album| artist_name(&album.artist),
        ),
        songs: ranked(
            songs(),
            &words,
            &folded,
            limit,
            |song| song.title.clone(),
            |song| {
                let album = album_by_id(&song.album).map_or("", |album| album.title.as_str());
                format!("{} {album}", artist_name(&song.artist))
            },
        ),
    }
}

/// How many of each kind the field and the palette list.
pub const LISTED: usize = 5;

/// What kind a group of results is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupKind {
    Artists,
    Albums,
    Songs,
}

/// One result in a group: what it opens, its title and the line under it.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub detail: String,
    /// Whether its artwork is round, as an artist's is.
    pub round: bool,
}

/// One group of the search's answer, with how many match in all.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchGroup {
    pub kind: GroupKind,
    pub title: &'static str,
    pub total: usize,
    pub results: Vec<SearchResult>,
}

/// The library's answer as the search's groups: artists, albums, songs, each with how many match
/// in all; a group with none left out.
pub fn search_groups(query: &str) -> Vec<SearchGroup> {
    let all = find(query, usize::MAX);
    let groups = vec![
        SearchGroup {
            kind: GroupKind::Artists,
            title: "Artists",
            total: all.artists.len(),
            results: all
                .artists
                .iter()
                .take(LISTED)
                .map(|artist| SearchResult {
                    id: artist.id.clone(),
                    title: artist.name.clone(),
                    detail: "Artist".to_owned(),
                    round: true,
                })
                .collect(),
        },
        SearchGroup {
            kind: GroupKind::Albums,
            title: "Albums",
            total: all.albums.len(),
            results: all
                .albums
                .iter()
                .take(LISTED)
                .map(|album| SearchResult {
                    id: album.id.clone(),
                    title: album.title.clone(),
                    detail: format!("{}, {}", artist_name(&album.artist), album.year),
                    round: false,
                })
                .collect(),
        },
        SearchGroup {
            kind: GroupKind::Songs,
            title: "Songs",
            total: all.songs.len(),
            results: all
                .songs
                .iter()
                .take(LISTED)
                .map(|song| SearchResult {
                    id: song.id.clone(),
                    title: song.title.clone(),
                    detail: format!(
                        "{}, {}",
                        artist_name(&song.artist),
                        album_by_id(&song.album).map_or("", |album| album.title.as_str())
                    ),
                    round: false,
                })
                .collect(),
        },
    ];
    groups.into_iter().filter(|group| !group.results.is_empty()).collect()
}
