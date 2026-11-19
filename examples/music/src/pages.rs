//! The pages, as the web's `pages/*.tsx`: the albums and one album, the artists and one artist,
//! every song, the favorites, the downloads, radio, the playlists and one playlist, and the
//! search's results.

use std::collections::HashMap;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Cell, Column, ConfirmAnswer, ConfirmDialog, DataTable, DownloadAsked,
    DownloadButton, DownloadState, DropdownMenu, Icon, IconButton, InlineEdit, MenuItem, MetaLine, Page, PageHeader,
    SearchField, Section, Segmented, StarButton, relative_time, toolbar,
};
use sokkel_egui_media::{
    Artwork, DraggableTrackList, MediaCard, MediaGrid, MediaShape, ReorderableTrackList, Track, TrackList, cover_for,
};

use crate::act::{Act, songs_word};
use crate::app::Views;
use crate::catalog::{
    Album, Song, Station, album_by_id, albums, albums_of_artist, artist_by_id, artist_name, artists, format_bytes,
    length_of_songs, minutes, song_of, songs, songs_of_album, songs_of_artist, stations,
};
use crate::data::{AlbumOrder, DownloadState as Kept, FavoriteKind, Library, Playlist, Source, SourceKind};
use crate::player::{Place, Player};
use crate::songs::{album_card, album_menu, artist_card, as_track, nothing, play_album, play_artist, song_actions};

/// What a page reads: the library, the player and the time.
#[derive(Clone, Copy)]
pub struct Here<'a> {
    pub library: &'a Library,
    pub player: &'a Player,
    pub now: i64,
}

impl Here<'_> {
    /// The song playing or paused, and whether it plays.
    pub fn now_playing(&self) -> (Option<Song>, bool) {
        (self.player.current_song(self.library), self.player.playing)
    }
}

/// The page the address names, in the page's scroller.
pub fn page(ui: &mut Ui, here: &Here, place: &str, second: Option<&str>, views: &mut Views, acts: &mut Vec<Act>) {
    Page::scroll(ui, |ui| match (place, second) {
        ("albums", None) => albums_page(ui, here, views, acts),
        ("albums", Some(id)) => album_page(ui, here, id, acts),
        ("artists", None) => artists_page(ui, here, views, acts),
        ("artists", Some(id)) => artist_page(ui, here, id, acts),
        ("tracks", _) => tracks_page(ui, here, views, acts),
        ("favorites", _) => favorites_page(ui, here, acts),
        ("downloads", _) => downloads_page(ui, here, views, acts),
        ("radio", _) => radio_page(ui, here, acts),
        ("playlists", None) => playlists_page(ui, here, acts),
        ("playlists", Some(id)) => playlist_page(ui, here, id, acts),
        (_, query) => results_page(ui, here, query.unwrap_or_default(), acts),
    });
}

/// Text compared as the web's `localeCompare` orders names: without regard to case.
fn by_name(one: &str, other: &str) -> std::cmp::Ordering {
    one.to_lowercase()
        .cmp(&other.to_lowercase())
        .then_with(|| one.cmp(other))
}

fn ids<'a>(songs: impl IntoIterator<Item = &'a Song>) -> Vec<String> {
    songs.into_iter().map(|song| song.id.clone()).collect()
}

/// A filter field at a toolbar's start, as the web's `SearchField` there.
fn filter(ui: &mut Ui, id: &str, query: &mut String) {
    SearchField::new(id, query)
        .placeholder("Filter")
        .keys("/")
        .width(256.0)
        .show(ui);
}

/// An outline button with an icon, small.
fn outline(label: &str, icon: Icon) -> Button {
    Button::new(label)
        .icon(icon)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
}

/// A filled button with an icon, small.
fn filled(label: &str, icon: Icon) -> Button {
    Button::new(label).icon(icon).filled_icon(true).size(ButtonSize::Sm)
}

/// Every album as covers, filtered by what is typed and in the order chosen, kept between visits.
fn albums_page(ui: &mut Ui, here: &Here, views: &mut Views, acts: &mut Vec<Act>) {
    let settings = &here.library.settings;
    PageHeader::new("Albums")
        .count(format!("{} albums", albums().len()))
        .show(ui, |_| {});
    let orders = [
        (AlbumOrder::Recent, "Recent"),
        (AlbumOrder::Title, "Title"),
        (AlbumOrder::Artist, "Artist"),
        (AlbumOrder::Year, "Year"),
    ];
    let mut chosen = orders
        .iter()
        .position(|(order, _)| *order == settings.album_order)
        .unwrap_or(0);
    toolbar(
        ui,
        |ui| filter(ui, "albums-filter", &mut views.albums_query),
        |ui| {
            if ui.add(outline("Shuffle all", Icon::Shuffle)).clicked() {
                acts.push(Act::Play {
                    songs: ids(songs()),
                    start: 0,
                    source: Source::new(SourceKind::Tracks, "Every song", Some("/tracks".into())),
                    shuffle: Some(true),
                });
            }
            if ui
                .add(Segmented::new(&mut chosen, orders.map(|(_, label)| label)))
                .changed()
            {
                let mut next = settings.clone();
                next.album_order = orders[chosen].0;
                acts.push(Act::Settings(next));
            }
        },
    );
    let words = views.albums_query.trim().to_lowercase();
    let mut shown: Vec<&Album> = albums()
        .iter()
        .filter(|album| {
            words.is_empty()
                || format!("{} {}", album.title, artist_name(&album.artist))
                    .to_lowercase()
                    .contains(&words)
        })
        .collect();
    shown.sort_by(|one, other| match settings.album_order {
        AlbumOrder::Title => by_name(&one.title, &other.title),
        AlbumOrder::Artist => {
            by_name(&artist_name(&one.artist), &artist_name(&other.artist)).then(other.year.cmp(&one.year))
        }
        AlbumOrder::Year => one
            .year
            .cmp(&other.year)
            .then_with(|| by_name(&one.title, &other.title)),
        AlbumOrder::Recent => other
            .year
            .cmp(&one.year)
            .then_with(|| by_name(&one.title, &other.title)),
    });
    if shown.is_empty() {
        let query = views.albums_query.clone();
        if nothing(
            ui,
            Icon::Disc3,
            "No album matches",
            &format!("Nothing in the library is called \u{201c}{query}\u{201d}."),
            Some("Clear the filter"),
        ) {
            views.albums_query.clear();
        }
        return;
    }
    MediaGrid::new(MediaShape::Square).show(ui, shown.len(), |ui, index| {
        album_card(ui, here, shown[index], None, acts);
    });
}

/// One album: back, the title, the artist, year and length, and Download, the star, Add to
/// playlist and More; the cover with Play and Shuffle beside the songs, the one playing marked, and
/// more by the artist under them.
fn album_page(ui: &mut Ui, here: &Here, id: &str, acts: &mut Vec<Act>) {
    let Some(album) = album_by_id(id) else {
        if nothing(
            ui,
            Icon::Disc3,
            "No such album",
            "It may have been taken out of the library.",
            Some("Go to the albums"),
        ) {
            acts.push(Act::Go("albums".into()));
        }
        return;
    };
    let (now, playing) = here.now_playing();
    let songs = songs_of_album(&album.id);
    let song_ids = ids(songs.iter().copied());
    let source = Source::new(SourceKind::Album, &album.title, Some(format!("/albums/{}", album.id)));
    let current_here = now.as_ref().is_some_and(|song| song.album == album.id);
    let others: Vec<&Album> = albums_of_artist(&album.artist)
        .into_iter()
        .filter(|other| other.id != album.id)
        .collect();
    let by = artist_name(&album.artist);
    let kept = here.library.download_of(&album.id);
    let starred = here.library.is_starred(FavoriteKind::Album, &album.id);

    let mut playlist_menu = crate::songs::Menu::default();
    playlist_menu.playlists(
        here,
        &song_ids,
        &format!("{}, {}", album.title, songs_word(songs.len())),
    );
    let more_menu = album_menu(here, album, true);
    let header = PageHeader::new(&album.title)
        .back("Back")
        .meta_line(
            MetaLine::new([
                by.clone(),
                album.year.to_string(),
                format!(
                    "{}, {}",
                    songs_word(songs.len()),
                    minutes(length_of_songs(songs.iter().copied()))
                ),
            ])
            .sm()
            .link(0),
        )
        .show(ui, |ui| {
            let more = ui.add(IconButton::new(Icon::Ellipsis, "More").outline());
            acts.extend(more_menu.act(DropdownMenu::new(&more_menu.items).end(true).width(208.0).show(&more)));
            let add = ui.add(outline("Add to playlist", Icon::ListPlus));
            acts.extend(
                playlist_menu.act(
                    DropdownMenu::new(&playlist_menu.items)
                        .end(true)
                        .width(208.0)
                        .show(&add),
                ),
            );
            let mut star = starred;
            ui.add(StarButton::new(&mut star).outline());
            if star != starred {
                acts.push(Act::Star(FavoriteKind::Album, album.id.clone(), star));
            }
            let state = match kept.map(|row| row.state) {
                None => DownloadState::Idle,
                Some(Kept::Running) => DownloadState::Running,
                Some(Kept::Kept) => DownloadState::Kept,
            };
            let mut button = DownloadButton::new(("album-download", &album.id), state)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Sm)
                .compact_on_phone()
                .cancellable()
                .removable()
                .parts(
                    kept.map_or(0, |row| row.done),
                    kept.map_or(songs.len() as u32, |row| row.total),
                );
            if let Some(row) = kept.filter(|row| row.state == Kept::Kept) {
                button = button.kept_detail(format!("On this device, {}", format_bytes(row.bytes)));
            }
            match button.show(ui).asked {
                Some(DownloadAsked::Download) => acts.push(Act::Download(album.id.clone())),
                Some(DownloadAsked::Cancel | DownloadAsked::Remove) => {
                    acts.push(Act::RemoveDownload(album.id.clone()));
                }
                _ => {}
            }
        });
    if header.back {
        acts.push(Act::Back);
    }
    if header.meta_link == Some(0) {
        acts.push(Act::Go(format!("artists/{}", album.artist)));
    }

    // the cover with Play and Shuffle, beside the songs and more by the artist
    let gap = 32.0;
    let side = 240.0;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.allocate_ui_with_layout(egui::vec2(side, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 16.0;
            ui.add(Artwork::of(album.cover).size(egui::vec2(side, side)).framed(true));
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if current_here && playing {
                    if ui.add(filled("Pause", Icon::Pause)).clicked() {
                        acts.push(Act::SetPlaying(false));
                    }
                } else if ui
                    .add(filled(if current_here { "Resume" } else { "Play" }, Icon::Play))
                    .clicked()
                {
                    acts.push(if current_here {
                        Act::SetPlaying(true)
                    } else {
                        play_album(album, false)
                    });
                }
                if ui.add(outline("Shuffle", Icon::Shuffle)).clicked() {
                    acts.push(play_album(album, true));
                }
            });
        });
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 32.0;
            let tracks: Vec<Track> = songs.iter().map(|song| as_track(here, song, false, false)).collect();
            let current = now.as_ref().filter(|_| current_here).map(|song| song.id.clone());
            let mut row_acts = Vec::new();
            let out = DraggableTrackList::new(
                TrackList::new(("album-songs", &album.id), &tracks)
                    .current(current.as_deref())
                    .playing(playing)
                    .actions(|ui, track, _| song_actions(ui, here, &track.id, false, None, &mut row_acts)),
                "song",
            )
            .show(ui);
            acts.extend(row_acts);
            if let Some(index) = out.played {
                if current.as_deref() == Some(tracks[index].id.as_str()) {
                    acts.push(Act::SetPlaying(!playing));
                } else {
                    acts.push(Act::Play {
                        songs: song_ids.clone(),
                        start: index,
                        source: source.clone(),
                        shuffle: None,
                    });
                }
            }
            if !others.is_empty() {
                Section::new(format!("More by {by}")).flat(true).show(ui, |ui| {
                    MediaGrid::new(MediaShape::Square).show(ui, others.len(), |ui, index| {
                        let other = others[index];
                        album_card(ui, here, other, Some(other.year.to_string()), acts);
                    });
                });
            }
        });
    });
}

/// How many albums an artist made.
fn albums_by(artist: &str) -> usize {
    albums().iter().filter(|album| album.artist == artist).count()
}

/// Every artist, by name or by how many albums they made, filtered by what is typed.
fn artists_page(ui: &mut Ui, here: &Here, views: &mut Views, acts: &mut Vec<Act>) {
    PageHeader::new("Artists")
        .count(format!("{} artists, {} albums", artists().len(), albums().len()))
        .show(ui, |_| {});
    let mut order = views.artists_order;
    toolbar(
        ui,
        |ui| filter(ui, "artists-filter", &mut views.artists_query),
        |ui| {
            ui.add(Segmented::new(&mut order, ["Name", "Most albums"]));
        },
    );
    views.artists_order = order;
    let words = views.artists_query.trim().to_lowercase();
    let mut shown: Vec<&crate::catalog::Artist> = artists()
        .iter()
        .filter(|artist| words.is_empty() || artist.name.to_lowercase().contains(&words))
        .collect();
    shown.sort_by(|one, other| {
        if order == 1 {
            albums_by(&other.id)
                .cmp(&albums_by(&one.id))
                .then_with(|| by_name(&one.name, &other.name))
        } else {
            by_name(&one.name, &other.name)
        }
    });
    if shown.is_empty() {
        let query = views.artists_query.clone();
        if nothing(
            ui,
            Icon::MicVocal,
            "No artist matches",
            &format!("No artist in the library is called \u{201c}{query}\u{201d}."),
            Some("Clear the filter"),
        ) {
            views.artists_query.clear();
        }
        return;
    }
    MediaGrid::new(MediaShape::Square).show(ui, shown.len(), |ui, index| {
        let artist = shown[index];
        artist_card(ui, here, artist, albums_by(&artist.id), acts);
    });
}

/// How many of an artist's songs the page lists as the most played.
const TOP: usize = 5;

/// One artist: who they are, play and shuffle everything by them, their most played songs, and
/// their albums.
fn artist_page(ui: &mut Ui, here: &Here, id: &str, acts: &mut Vec<Act>) {
    let Some(artist) = artist_by_id(id) else {
        if nothing(
            ui,
            Icon::MicVocal,
            "No such artist",
            "They may have been taken out of the library.",
            Some("Go to the artists"),
        ) {
            acts.push(Act::Go("artists".into()));
        }
        return;
    };
    let (now, playing) = here.now_playing();
    let albums = albums_of_artist(&artist.id);
    let songs = songs_of_artist(&artist.id);
    // their songs played most lately first, then the first songs of their albums
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for play in &here.library.history {
        *counts.entry(play.song.as_str()).or_default() += 1;
    }
    let mut top: Vec<&Song> = songs.clone();
    top.sort_by(|one, other| {
        counts
            .get(other.id.as_str())
            .unwrap_or(&0)
            .cmp(counts.get(one.id.as_str()).unwrap_or(&0))
            .then(one.number.cmp(&other.number))
    });
    top.truncate(TOP);
    let top_ids = ids(top.iter().copied());
    let starred = here.library.is_starred(FavoriteKind::Artist, &artist.id);
    let header = PageHeader::new(&artist.name)
        .back("Back")
        .description(&artist.about)
        .meta_line(
            MetaLine::new([
                artist.style.name().to_owned(),
                format!(
                    "{} {}",
                    albums.len(),
                    if albums.len() == 1 { "album" } else { "albums" }
                ),
                format!(
                    "{}, {}",
                    songs_word(songs.len()),
                    minutes(length_of_songs(songs.iter().copied()))
                ),
            ])
            .sm(),
        )
        .show(ui, |ui| {
            if ui.add(filled("Play", Icon::Play)).clicked() {
                acts.push(play_artist(artist, false));
            }
            if ui.add(outline("Shuffle", Icon::Shuffle)).clicked() {
                acts.push(play_artist(artist, true));
            }
            if ui.add(outline("Radio", Icon::Radio)).clicked() {
                acts.push(Act::Radio(crate::catalog::StationKind::Artist, artist.id.clone()));
            }
            let mut star = starred;
            ui.add(StarButton::new(&mut star).outline());
            if star != starred {
                acts.push(Act::Star(FavoriteKind::Artist, artist.id.clone(), star));
            }
        });
    if header.back {
        acts.push(Act::Back);
    }
    Section::new("Most played").flat(true).show(ui, |ui| {
        let tracks: Vec<Track> = top.iter().map(|song| as_track(here, song, true, true)).collect();
        let current = now.as_ref().map(|song| song.id.clone());
        let mut row_acts = Vec::new();
        let out = DraggableTrackList::new(
            TrackList::new(("artist-top", &artist.id), &tracks)
                .current(current.as_deref())
                .playing(playing)
                .follow(false)
                .actions(|ui, track, _| song_actions(ui, here, &track.id, true, None, &mut row_acts)),
            "song",
        )
        .show(ui);
        acts.extend(row_acts);
        if let Some(index) = out.played {
            if current.as_deref() == Some(tracks[index].id.as_str()) {
                acts.push(Act::SetPlaying(!playing));
            } else {
                acts.push(Act::Play {
                    songs: top_ids.clone(),
                    start: index,
                    source: Source::new(
                        SourceKind::Artist,
                        &artist.name,
                        Some(format!("/artists/{}", artist.id)),
                    ),
                    shuffle: None,
                });
            }
        }
    });
    Section::new("Albums")
        .flat(true)
        .meta(albums.len().to_string())
        .show(ui, |ui| {
            MediaGrid::new(MediaShape::Square).show(ui, albums.len(), |ui, index| {
                let album = albums[index];
                album_card(ui, here, album, Some(album.year.to_string()), acts);
            });
        });
}

/// Every song in the library, filtered and ordered, played from the one pressed through the rest
/// of the list.
fn tracks_page(ui: &mut Ui, here: &Here, views: &mut Views, acts: &mut Vec<Act>) {
    let (now, playing) = here.now_playing();
    PageHeader::new("Tracks")
        .count(format!(
            "{} songs, {}",
            songs().len(),
            minutes(length_of_songs(songs()))
        ))
        .show(ui, |_| {});
    let words = views.tracks_query.trim().to_lowercase();
    let album_title = |song: &Song| album_by_id(&song.album).map_or_else(String::new, |album| album.title.clone());
    let mut shown: Vec<&Song> = songs()
        .iter()
        .filter(|song| {
            words.is_empty()
                || format!("{} {} {}", song.title, artist_name(&song.artist), album_title(song))
                    .to_lowercase()
                    .contains(&words)
        })
        .collect();
    let order = views.tracks_order;
    shown.sort_by(|one, other| match order {
        1 => by_name(&one.title, &other.title),
        2 => by_name(&album_title(one), &album_title(other))
            .then(one.disc.cmp(&other.disc))
            .then(one.number.cmp(&other.number)),
        3 => other.duration.total_cmp(&one.duration),
        _ => by_name(&artist_name(&one.artist), &artist_name(&other.artist))
            .then_with(|| by_name(&one.title, &other.title)),
    });
    let shown_ids = ids(shown.iter().copied());
    let label = if words.is_empty() {
        "Every song".to_owned()
    } else {
        format!("Songs matching \u{201c}{}\u{201d}", views.tracks_query.trim())
    };
    let source = Source::new(SourceKind::Tracks, label, Some("/tracks".into()));
    let mut chosen = order;
    toolbar(
        ui,
        |ui| filter(ui, "tracks-filter", &mut views.tracks_query),
        |ui| {
            if ui
                .add(outline("Shuffle", Icon::Shuffle).enabled(!shown_ids.is_empty()))
                .clicked()
            {
                acts.push(Act::Play {
                    songs: shown_ids.clone(),
                    start: 0,
                    source: source.clone(),
                    shuffle: Some(true),
                });
            }
            ui.add(Segmented::new(&mut chosen, ["Artist", "Title", "Album", "Longest"]));
        },
    );
    views.tracks_order = chosen;
    if shown.is_empty() {
        let query = views.tracks_query.clone();
        if nothing(
            ui,
            Icon::Music2,
            "No song matches",
            &format!("No song, artist or album in the library is called \u{201c}{query}\u{201d}."),
            Some("Clear the filter"),
        ) {
            views.tracks_query.clear();
        }
        return;
    }
    let tracks: Vec<Track> = shown.iter().map(|song| as_track(here, song, true, true)).collect();
    let current = now.as_ref().map(|song| song.id.clone());
    let mut row_acts = Vec::new();
    let out = DraggableTrackList::new(
        TrackList::new("tracks", &tracks)
            .current(current.as_deref())
            .playing(playing)
            .follow(false)
            .windowed(true)
            .actions(|ui, track, _| song_actions(ui, here, &track.id, true, None, &mut row_acts)),
        "song",
    )
    .show(ui);
    acts.extend(row_acts);
    if let Some(index) = out.played {
        acts.push(Act::Play {
            songs: shown_ids,
            start: index,
            source,
            shuffle: None,
        });
    }
}

/// A list of songs to play from, its rows each with a star and a menu: the favorites', the
/// results'.
fn song_list(ui: &mut Ui, here: &Here, id: &str, songs: &[Song], source: &Source, acts: &mut Vec<Act>) {
    let (now, playing) = here.now_playing();
    let tracks: Vec<Track> = songs.iter().map(|song| as_track(here, song, true, true)).collect();
    let current = now.as_ref().map(|song| song.id.clone());
    let mut row_acts = Vec::new();
    let out = DraggableTrackList::new(
        TrackList::new(id, &tracks)
            .current(current.as_deref())
            .playing(playing)
            .follow(false)
            .actions(|ui, track, _| song_actions(ui, here, &track.id, true, None, &mut row_acts)),
        "song",
    )
    .show(ui);
    acts.extend(row_acts);
    if let Some(index) = out.played {
        if current.as_deref() == Some(tracks[index].id.as_str()) {
            acts.push(Act::SetPlaying(!playing));
        } else {
            acts.push(Act::Play {
                songs: songs.iter().map(|song| song.id.clone()).collect(),
                start: index,
                source: source.clone(),
                shuffle: None,
            });
        }
    }
}

/// What the listener starred: songs, played from here as a list, albums and artists, newest first.
fn favorites_page(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let rows = &here.library.favorites;
    let songs: Vec<Song> = rows
        .iter()
        .filter(|row| row.kind == FavoriteKind::Song)
        .filter_map(|row| song_of(&row.reference, &here.library.stations))
        .collect();
    let starred_albums: Vec<&Album> = rows
        .iter()
        .filter(|row| row.kind == FavoriteKind::Album)
        .filter_map(|row| album_by_id(&row.reference))
        .collect();
    let starred_artists: Vec<&crate::catalog::Artist> = rows
        .iter()
        .filter(|row| row.kind == FavoriteKind::Artist)
        .filter_map(|row| artist_by_id(&row.reference))
        .collect();
    let source = Source::new(SourceKind::Favorites, "Favorites", Some("/favorites".into()));
    let song_ids: Vec<String> = songs.iter().map(|song| song.id.clone()).collect();
    PageHeader::new("Favorites")
        .count(format!(
            "{} songs, {} albums, {} artists",
            songs.len(),
            starred_albums.len(),
            starred_artists.len()
        ))
        .show(ui, |ui| {
            if songs.is_empty() {
                return;
            }
            if ui.add(filled("Play", Icon::Play)).clicked() {
                acts.push(Act::Play {
                    songs: song_ids.clone(),
                    start: 0,
                    source: source.clone(),
                    shuffle: Some(false),
                });
            }
            if ui.add(outline("Shuffle", Icon::Shuffle)).clicked() {
                acts.push(Act::Play {
                    songs: song_ids.clone(),
                    start: 0,
                    source: source.clone(),
                    shuffle: Some(true),
                });
            }
        });
    if songs.is_empty() && starred_albums.is_empty() && starred_artists.is_empty() {
        if nothing(
            ui,
            Icon::Star,
            "Nothing starred yet",
            "Star a song, an album or an artist and it is kept here.",
            Some("Browse the albums"),
        ) {
            acts.push(Act::Go("albums".into()));
        }
        return;
    }
    if !songs.is_empty() {
        Section::new("Songs")
            .flat(true)
            .meta(songs.len().to_string())
            .show(ui, |ui| song_list(ui, here, "favorite-songs", &songs, &source, acts));
    }
    if !starred_albums.is_empty() {
        Section::new("Albums")
            .flat(true)
            .meta(starred_albums.len().to_string())
            .show(ui, |ui| {
                MediaGrid::new(MediaShape::Square).show(ui, starred_albums.len(), |ui, index| {
                    album_card(ui, here, starred_albums[index], None, acts);
                });
            });
    }
    if !starred_artists.is_empty() {
        Section::new("Artists")
            .flat(true)
            .meta(starred_artists.len().to_string())
            .show(ui, |ui| {
                MediaGrid::new(MediaShape::Square).show(ui, starred_artists.len(), |ui, index| {
                    let artist = starred_artists[index];
                    artist_card(ui, here, artist, albums_by(&artist.id), acts);
                });
            });
    }
}

/// The albums kept on this device and those coming in, with their sizes; each removed from its
/// menu, or all at once.
fn downloads_page(ui: &mut Ui, here: &Here, views: &mut Views, acts: &mut Vec<Act>) {
    let lines: Vec<(&crate::data::Download, &Album)> = here
        .library
        .downloads
        .iter()
        .filter_map(|row| album_by_id(&row.id).map(|album| (row, album)))
        .collect();
    let kept: Vec<_> = lines.iter().filter(|(row, _)| row.state == Kept::Kept).collect();
    let running = lines.len() - kept.len();
    let bytes: f64 = kept.iter().map(|(row, _)| row.bytes).sum();
    let mut count = format!("{} albums, {}", kept.len(), format_bytes(bytes));
    if running > 0 {
        count.push_str(&format!(", {running} coming in"));
    }
    PageHeader::new("Downloads")
        .count(count)
        .description("Albums kept on this device play without the server.")
        .show(ui, |ui| {
            if !lines.is_empty() && ui.add(outline("Remove all", Icon::Trash2)).clicked() {
                acts.push(Act::AskRemoveAll);
            }
        });
    if lines.is_empty() {
        if nothing(
            ui,
            Icon::CloudDownload,
            "Nothing downloaded yet",
            "Open an album and press Download to keep it on this device.",
            Some("Browse the albums"),
        ) {
            acts.push(Act::Go("albums".into()));
        }
        return;
    }
    let columns = [
        Column::new("Album", 224.0).fill_share(1.4),
        Column::new("Artist", 128.0).fill(),
        Column::new("Size", 96.0).numeric(),
        Column::new("Kept", 128.0),
        Column::new("State", 176.0),
    ];
    let rows: Vec<Vec<Cell>> = lines
        .iter()
        .map(|(row, album)| {
            let kept = row.state == Kept::Kept;
            vec![
                Cell::Strong(album.title.clone()),
                Cell::Text(artist_name(&album.artist)),
                Cell::Text(if kept {
                    format_bytes(row.bytes)
                } else {
                    "\u{2026}".into()
                }),
                Cell::Text(if kept {
                    relative_time((row.at - here.now) as f64)
                } else {
                    "Now".into()
                }),
                Cell::Muted(if kept {
                    format!("On this device, {}", format_bytes(row.bytes))
                } else {
                    format!("{} of {}", row.done, row.total)
                }),
            ]
        })
        .collect();
    let out = DataTable::new("downloads", &columns, &rows, &mut views.table)
        .keys(lines.iter().map(|(_, album)| album.id.clone()))
        .actions(|index| {
            let kept = lines.get(index).is_some_and(|(row, _)| row.state == Kept::Kept);
            vec![
                MenuItem::action("Play").icon(Icon::Play),
                MenuItem::action(if kept { "Remove" } else { "Cancel" })
                    .icon(Icon::Trash2)
                    .destructive(),
            ]
        })
        .show(ui);
    if let Some((index, action)) = out.action
        && let Some((_, album)) = lines.get(index)
    {
        acts.push(if action == 0 {
            play_album(album, false)
        } else {
            Act::RemoveDownload(album.id.clone())
        });
    }
    if let Some(index) = views.table.opened.take()
        && let Some((_, album)) = lines.get(index)
    {
        acts.push(Act::Go(format!("albums/{}", album.id)));
    }
}

/// Radio: stations that never run out, each making new songs in its style one after another, and
/// the stations the listener started from an artist, an album or a song. What the station playing
/// makes next is listed while it plays.
fn radio_page(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let player = here.player;
    let made: Vec<Station> = here.library.stations.iter().map(|row| row.station()).collect();
    let playing_station = player.radio.as_ref().map(|radio| radio.station.clone());
    PageHeader::new("Radio")
        .description("Stations that never run out: each one makes a new song in its style as the last one ends.")
        .show(ui, |_| {});
    let coming: Vec<Song> = if playing_station.is_some() {
        player
            .up_next()
            .iter()
            .filter_map(|(entry, _)| song_of(&entry.song, &here.library.stations))
            .collect()
    } else {
        Vec::new()
    };
    if !coming.is_empty() {
        let mut section = Section::new("Coming up").flat(true);
        if let Some(source) = &player.source {
            section = section.meta(source.label.clone());
        }
        section.show(ui, |ui| {
            let tracks: Vec<Track> = coming.iter().map(|song| as_track(here, song, true, false)).collect();
            TrackList::new("coming-up", &tracks)
                .follow(false)
                .playable(false)
                .show(ui);
        });
    }
    let card = |ui: &mut Ui, station: &Station, removable: bool, acts: &mut Vec<Act>| {
        let menu = if removable {
            vec![MenuItem::action("Remove station").icon(Icon::Trash2).destructive()]
        } else {
            Vec::new()
        };
        let cover = station.cover;
        let out = MediaCard::new(("station", &station.id), &station.name)
            .subtitle(&station.about)
            .cover(move |painter, rect| Artwork::of(cover).paint(painter, rect))
            .badge(station.style.name())
            .playable(true)
            .playing(player.playing && playing_station.as_deref() == Some(station.id.as_str()))
            .menu(&menu)
            .show(ui);
        if out.selected || out.play {
            acts.push(Act::Station(station.id.clone(), station.name.clone()));
        }
        if out.menu == Some(0) {
            acts.push(Act::RemoveStation(station.id.clone()));
        }
    };
    Section::new("Stations")
        .flat(true)
        .meta(stations().len().to_string())
        .show(ui, |ui| {
            MediaGrid::new(MediaShape::Square).show(ui, stations().len(), |ui, index| {
                card(ui, &stations()[index], false, acts);
            });
        });
    let mut section = Section::new("Your stations").flat(true).meta(made.len().to_string());
    if made.is_empty() {
        section =
            section.description("Start radio from an artist, an album or a song, from its menu, and it is kept here.");
    }
    section.show(ui, |ui| {
        if !made.is_empty() {
            MediaGrid::new(MediaShape::Square).show(ui, made.len(), |ui, index| {
                card(ui, &made[index], true, acts);
            });
        }
    });
}

/// A playlist's songs that are there.
fn songs_on(here: &Here, playlist: &Playlist) -> Vec<Song> {
    playlist
        .entries
        .iter()
        .filter_map(|entry| song_of(&entry.song, &here.library.stations))
        .collect()
}

/// How a playlist plays: its songs in its order, as the queue from it.
fn play_playlist(playlist: &Playlist, start: usize, shuffle: Option<bool>) -> Act {
    Act::Play {
        songs: playlist.entries.iter().map(|entry| entry.song.clone()).collect(),
        start,
        source: Source::new(
            SourceKind::Playlist,
            &playlist.name,
            Some(format!("/playlists/{}", playlist.id)),
        ),
        shuffle,
    }
}

/// The line under a playlist's name: how many songs and how long, or that it has none yet.
fn line_of(here: &Here, playlist: &Playlist) -> String {
    let songs = songs_on(here, playlist);
    if songs.is_empty() {
        return "No songs yet".into();
    }
    format!("{}, {}", songs_word(songs.len()), minutes(length_of_songs(&songs)))
}

/// Every playlist as a card, the newest changed first, and New to make one.
fn playlists_page(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let mut rows: Vec<&Playlist> = here.library.playlists.iter().collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.updated));
    PageHeader::new("Playlists")
        .count(format!("{} playlists", rows.len()))
        .show(ui, |ui| {
            if ui.add(filled("New", Icon::Plus)).clicked() {
                acts.push(Act::Ask(crate::dialogs::Asking::New(Vec::new(), None)));
            }
        });
    if rows.is_empty() {
        if nothing(
            ui,
            Icon::ListMusic,
            "No playlists yet",
            "Make one here, or add an album or a song to one from its menu.",
            Some("New playlist"),
        ) {
            acts.push(Act::Ask(crate::dialogs::Asking::New(Vec::new(), None)));
        }
        return;
    }
    let player = here.player;
    MediaGrid::new(MediaShape::Square).show(ui, rows.len(), |ui, index| {
        let playlist = rows[index];
        let empty = playlist.entries.is_empty();
        let menu = [
            MenuItem::action("Shuffle").icon(Icon::Shuffle).disabled(empty),
            MenuItem::action("Rename").icon(Icon::Pencil),
            MenuItem::Separator,
            MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
        ];
        let cover = cover_for(&format!("playlist:{}", playlist.id));
        let href = format!("/playlists/{}", playlist.id);
        let out = MediaCard::new(("playlist", &playlist.id), &playlist.name)
            .subtitle(line_of(here, playlist))
            .cover(move |painter, rect| Artwork::of(cover).paint(painter, rect))
            .playable(!empty)
            .playing(
                player.playing
                    && player.source.as_ref().is_some_and(|source| {
                        source.kind == SourceKind::Playlist && source.href.as_deref() == Some(&href)
                    }),
            )
            .menu(&menu)
            .show(ui);
        if out.selected {
            acts.push(Act::Go(format!("playlists/{}", playlist.id)));
        }
        if out.play {
            acts.push(play_playlist(playlist, 0, None));
        }
        match out.menu {
            Some(0) => acts.push(play_playlist(playlist, 0, Some(true))),
            Some(1) => acts.push(Act::Ask(crate::dialogs::Asking::Rename(playlist.id.clone()))),
            Some(3) => acts.push(Act::Ask(crate::dialogs::Asking::Delete(playlist.id.clone()))),
            _ => {}
        }
    });
}

/// One playlist: its name, edited in place, and what it is for; Play, Shuffle and a menu to
/// rename it, put it in the queue or delete it; its songs, each dragged by its grip into the order
/// wanted or taken off from its menu.
fn playlist_page(ui: &mut Ui, here: &Here, id: &str, acts: &mut Vec<Act>) {
    let Some(playlist) = here.library.playlist(id) else {
        if nothing(
            ui,
            Icon::ListMusic,
            "No such playlist",
            "It may have been deleted.",
            Some("Go to the playlists"),
        ) {
            acts.push(Act::Go("playlists".into()));
        }
        return;
    };
    let (now, playing) = here.now_playing();
    let songs = songs_on(here, playlist);
    let href = format!("/playlists/{}", playlist.id);
    let current_here = here
        .player
        .source
        .as_ref()
        .is_some_and(|source| source.href.as_deref() == Some(&href));
    let entries = here.library.playlist_entries(&playlist.id);
    let menu = [
        MenuItem::action("Add to queue")
            .icon(Icon::Plus)
            .disabled(songs.is_empty()),
        MenuItem::action("Rename").icon(Icon::Pencil),
        MenuItem::Separator,
        MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
    ];
    let header = PageHeader::new(&playlist.name)
        .back("Back")
        .edit_title("Name", |name| {
            name.trim().is_empty().then(|| "Give it a name".to_owned())
        })
        .meta_line(
            MetaLine::new([
                line_of(here, playlist),
                format!("Changed {}", relative_time((playlist.updated - here.now) as f64)),
            ])
            .sm(),
        )
        .show(ui, |ui| {
            if ui.add(filled("Play", Icon::Play).enabled(!songs.is_empty())).clicked() {
                acts.push(play_playlist(playlist, 0, Some(false)));
            }
            if ui
                .add(outline("Shuffle", Icon::Shuffle).enabled(!songs.is_empty()))
                .clicked()
            {
                acts.push(play_playlist(playlist, 0, Some(true)));
            }
            let more = ui.add(IconButton::new(Icon::Ellipsis, "More"));
            match DropdownMenu::new(&menu).end(true).width(208.0).show(&more) {
                Some(0) => acts.push(Act::Enqueue(
                    entries.iter().map(|entry| entry.song.clone()).collect(),
                    Place::End,
                    playlist.name.clone(),
                )),
                Some(1) => acts.push(Act::Ask(crate::dialogs::Asking::Rename(playlist.id.clone()))),
                Some(3) => acts.push(Act::Ask(crate::dialogs::Asking::Delete(playlist.id.clone()))),
                _ => {}
            }
        });
    if header.back {
        acts.push(Act::Back);
    }
    if let Some(name) = header.renamed {
        acts.push(Act::Rename(playlist.id.clone(), name));
    }
    // what it is for, edited in place under the header
    if let Some(description) = InlineEdit::new(
        ("playlist-description", &playlist.id),
        "Description",
        &playlist.description,
    )
    .placeholder("What it is for")
    .show(ui)
    {
        acts.push(Act::Describe(playlist.id.clone(), description));
    }
    if entries.is_empty() {
        if nothing(
            ui,
            Icon::ListMusic,
            "Nothing on this playlist yet",
            "Add a song from its menu, or a whole album from its page.",
            Some("Browse the albums"),
        ) {
            acts.push(Act::Go("albums".into()));
        }
        return;
    }
    let tracks: Vec<Track> = entries
        .iter()
        .map(|entry| match song_of(&entry.song, &here.library.stations) {
            Some(song) => Track {
                id: entry.key.clone(),
                ..as_track(here, &song, true, true)
            },
            None => Track::new(&entry.key, &entry.song, 0.0),
        })
        .collect();
    let current = if current_here {
        entries
            .iter()
            .find(|entry| now.as_ref().is_some_and(|song| song.id == entry.song))
            .map(|entry| entry.key.clone())
    } else {
        None
    };
    let mut row_acts = Vec::new();
    let out = ReorderableTrackList::new(
        TrackList::new(("playlist-songs", &playlist.id), &tracks)
            .current(current.as_deref())
            .playing(playing)
            .follow(false)
            .actions(|ui, _, index| {
                if let Some(entry) = entries.get(index) {
                    let remove = Act::RemoveFromPlaylist(playlist.id.clone(), entry.key.clone());
                    song_actions(
                        ui,
                        here,
                        &entry.song,
                        true,
                        Some(("Remove from this playlist", remove)),
                        &mut row_acts,
                    );
                }
            }),
    )
    .show(ui);
    acts.extend(row_acts);
    if let Some((from, to)) = out.moved {
        acts.push(Act::MoveInPlaylist(playlist.id.clone(), from, to));
    }
    if let Some(index) = out.played {
        if current.as_deref() == Some(tracks[index].id.as_str()) {
            acts.push(Act::SetPlaying(!playing));
        } else {
            let start = playlist
                .entries
                .iter()
                .position(|entry| entry.key == tracks[index].id)
                .unwrap_or(index);
            acts.push(play_playlist(playlist, start, None));
        }
    }
}

/// Everything in the library that answers a search: artists, albums and songs, the songs played as
/// a list.
fn results_page(ui: &mut Ui, here: &Here, query: &str, acts: &mut Vec<Act>) {
    let found = crate::search::find(query, usize::MAX);
    let total = found.artists.len() + found.albums.len() + found.songs.len();
    PageHeader::new(format!("\u{201c}{query}\u{201d}"))
        .count(format!("{total} found"))
        .show(ui, |_| {});
    if total == 0 {
        nothing(
            ui,
            Icon::Search,
            "Nothing answers to that",
            &format!("No artist, album or song in the library is called \u{201c}{query}\u{201d}."),
            None,
        );
        return;
    }
    if !found.artists.is_empty() {
        Section::new("Artists")
            .flat(true)
            .meta(found.artists.len().to_string())
            .show(ui, |ui| {
                MediaGrid::new(MediaShape::Square).show(ui, found.artists.len(), |ui, index| {
                    let artist = found.artists[index];
                    artist_card(ui, here, artist, albums_by(&artist.id), acts);
                });
            });
    }
    if !found.albums.is_empty() {
        Section::new("Albums")
            .flat(true)
            .meta(found.albums.len().to_string())
            .show(ui, |ui| {
                MediaGrid::new(MediaShape::Square).show(ui, found.albums.len(), |ui, index| {
                    album_card(ui, here, found.albums[index], None, acts);
                });
            });
    }
    if !found.songs.is_empty() {
        let songs: Vec<Song> = found.songs.iter().map(|song| (*song).clone()).collect();
        let source = Source::new(
            SourceKind::Search,
            format!("Songs matching \u{201c}{query}\u{201d}"),
            Some(format!("/search/{query}")),
        );
        Section::new("Songs")
            .flat(true)
            .meta(songs.len().to_string())
            .show(ui, |ui| song_list(ui, here, "found-songs", &songs, &source, acts));
    }
}

impl crate::app::MusicApp {
    /// The question before every download goes.
    pub fn clearing_dialog(&mut self, ctx: &egui::Context) {
        if !self.clearing {
            return;
        }
        let kept: Vec<_> = self.library.downloads.iter().collect();
        let bytes: f64 = kept.iter().map(|row| row.bytes).sum();
        let answer = ConfirmDialog::new("remove-downloads", "Remove every download?", "Remove all")
            .detail(format!(
                "{} albums, {}, leave this device. They stay in the library.",
                kept.len(),
                format_bytes(bytes)
            ))
            .destructive(true)
            .show(ctx);
        match answer {
            Some(ConfirmAnswer::Confirm) => {
                self.library.remove_all_downloads();
                self.clearing = false;
            }
            Some(_) => self.clearing = false,
            None => {}
        }
    }
}
