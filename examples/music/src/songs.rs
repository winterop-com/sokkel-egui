//! What the pages share, as the web's `parts/songs.tsx` and `parts/actions.tsx`: a song as a track
//! list's row, an album's and an artist's card with their menus, a song's star and menu, and what a
//! page with nothing in it says.

use eframe::egui::Ui;
use sokkel_egui::{Button, ButtonSize, ButtonVariant, DropdownMenu, Empty, Icon, IconButton, MenuItem, StarButton};
use sokkel_egui_media::{Artwork, MediaCard, Track};

use crate::act::{Act, songs_word};
use crate::catalog::{Album, Artist, Song, StationKind, album_of, artist_name, songs_of_album, songs_of_artist};
use crate::data::{FavoriteKind, Source, SourceKind};
use crate::pages::Here;
use crate::player::Place;

/// A song as a track list's row: its number on its album, or its cover; the artist under the title
/// where it differs, or always; the album in the quiet column where asked.
pub fn as_track(here: &Here, song: &Song, artwork: bool, album: bool) -> Track {
    let on = album_of(song, &here.library.stations);
    let by = artist_name(&song.artist);
    let mut track = Track::new(&song.id, &song.title, song.duration);
    let subtitle = match (artwork, &song.with) {
        (true, None) => Some(by),
        (true, Some(with)) => Some(format!("{by} with {with}")),
        (false, Some(with)) => Some(format!("with {with}")),
        (false, None) => None,
    };
    if let Some(subtitle) = subtitle {
        track = track.subtitle(subtitle);
    }
    if artwork {
        track = track.artwork(on.cover);
    } else {
        track = track.number(song.number).disc(song.disc);
    }
    if album {
        track = track.meta(on.title);
    }
    track
}

fn ids<'a>(songs: impl IntoIterator<Item = &'a Song>) -> Vec<String> {
    songs.into_iter().map(|song| song.id.clone()).collect()
}

/// Plays an album from its first song, or shuffled.
pub fn play_album(album: &Album, shuffle: bool) -> Act {
    Act::Play {
        songs: ids(songs_of_album(&album.id)),
        start: 0,
        source: Source::new(SourceKind::Album, &album.title, Some(format!("/albums/{}", album.id))),
        shuffle: Some(shuffle),
    }
}

pub fn play_artist(artist: &Artist, shuffle: bool) -> Act {
    Act::Play {
        songs: ids(songs_of_artist(&artist.id)),
        start: 0,
        source: Source::new(
            SourceKind::Artist,
            &artist.name,
            Some(format!("/artists/{}", artist.id)),
        ),
        shuffle: Some(shuffle),
    }
}

/// A menu's items, each with what it asks; a rule or a heading asks nothing.
#[derive(Default)]
pub struct Menu {
    pub items: Vec<MenuItem>,
    pub acts: Vec<Option<Act>>,
}

impl Menu {
    pub fn item(&mut self, item: MenuItem, act: Act) {
        self.items.push(item);
        self.acts.push(Some(act));
    }

    pub fn rule(&mut self) {
        self.items.push(MenuItem::Separator);
        self.acts.push(None);
    }

    pub fn label(&mut self, label: &str) {
        self.items.push(MenuItem::Label(label.to_owned()));
        self.acts.push(None);
    }

    /// What the entry at `index` asks.
    pub fn act(&self, index: Option<usize>) -> Option<Act> {
        index.and_then(|index| self.acts.get(index).cloned().flatten())
    }

    /// The items of "Add to playlist": a new playlist with the songs, then each playlist.
    pub fn playlists(&mut self, here: &Here, songs: &[String], what: &str) {
        self.label("Add to playlist");
        self.item(
            MenuItem::action("New playlist").icon(Icon::Plus),
            Act::Ask(crate::dialogs::Asking::New(songs.to_vec(), Some(what.to_owned()))),
        );
        for playlist in &here.library.playlists {
            self.item(
                MenuItem::action(&playlist.name).icon(Icon::ListMusic),
                Act::AddTo(playlist.id.clone(), songs.to_vec(), Some(what.to_owned())),
            );
        }
    }
}

/// What an album's menu offers, on its card and its page.
pub fn album_menu(here: &Here, album: &Album, page: bool) -> Menu {
    let songs = ids(songs_of_album(&album.id));
    let kept = here.library.download_of(&album.id).is_some();
    let starred = here.library.is_starred(FavoriteKind::Album, &album.id);
    let mut menu = Menu::default();
    menu.item(MenuItem::action("Shuffle").icon(Icon::Shuffle), play_album(album, true));
    menu.item(
        MenuItem::action("Play next").icon(Icon::ListStart),
        Act::Enqueue(songs.clone(), Place::Next, album.title.clone()),
    );
    menu.item(
        MenuItem::action("Add to queue").icon(Icon::ListEnd),
        Act::Enqueue(songs.clone(), Place::End, album.title.clone()),
    );
    menu.rule();
    menu.playlists(here, &songs, &format!("{}, {}", album.title, songs_word(songs.len())));
    menu.rule();
    if !page {
        menu.item(
            MenuItem::action(if starred {
                "Remove from favorites"
            } else {
                "Add to favorites"
            })
            .icon(Icon::Star),
            Act::Star(FavoriteKind::Album, album.id.clone(), !starred),
        );
        if !kept {
            menu.item(
                MenuItem::action("Download").icon(Icon::Download),
                Act::Download(album.id.clone()),
            );
        }
    }
    menu.item(
        MenuItem::action(format!("Go to {}", artist_name(&album.artist))).icon(Icon::MicVocal),
        Act::Go(format!("artists/{}", album.artist)),
    );
    menu.item(
        MenuItem::action("Start radio").icon(Icon::Radio),
        Act::Radio(StationKind::Album, album.id.clone()),
    );
    menu
}

/// An album's card in a grid: its cover, title and artist, played from its button, its menu at its
/// corner; `line` in place of the artist and the year.
pub fn album_card(ui: &mut Ui, here: &Here, album: &Album, line: Option<String>, acts: &mut Vec<Act>) {
    let menu = album_menu(here, album, false);
    let song = here.player.current_song(here.library);
    let cover = album.cover;
    let out = MediaCard::new(("album", &album.id), &album.title)
        .subtitle(line.unwrap_or_else(|| format!("{} \u{b7} {}", artist_name(&album.artist), album.year)))
        .cover(move |painter, rect| Artwork::of(cover).paint(painter, rect))
        .playable(true)
        .playing(here.player.playing && song.is_some_and(|song| song.album == album.id))
        .menu(&menu.items)
        .show(ui);
    if out.selected {
        acts.push(Act::Go(format!("albums/{}", album.id)));
    }
    if out.play {
        acts.push(play_album(album, false));
    }
    acts.extend(menu.act(out.menu));
}

/// An artist's card: their drawn cover, name and how many albums, played from its button.
pub fn artist_card(ui: &mut Ui, here: &Here, artist: &Artist, albums: usize, acts: &mut Vec<Act>) {
    let starred = here.library.is_starred(FavoriteKind::Artist, &artist.id);
    let mut menu = Menu::default();
    menu.item(
        MenuItem::action("Shuffle").icon(Icon::Shuffle),
        play_artist(artist, true),
    );
    menu.item(
        MenuItem::action(if starred {
            "Remove from favorites"
        } else {
            "Add to favorites"
        })
        .icon(Icon::Star),
        Act::Star(FavoriteKind::Artist, artist.id.clone(), !starred),
    );
    menu.item(
        MenuItem::action("Start radio").icon(Icon::Radio),
        Act::Radio(StationKind::Artist, artist.id.clone()),
    );
    let song = here.player.current_song(here.library);
    let cover = artist.cover;
    let out = MediaCard::new(("artist", &artist.id), &artist.name)
        .subtitle(format!("{albums} {}", if albums == 1 { "album" } else { "albums" }))
        .cover(move |painter, rect| Artwork::of(cover).paint(painter, rect))
        .playable(true)
        .playing(
            here.player.playing
                && song.is_some_and(|song| song.artist == artist.id && !song.album.starts_with("radio:")),
        )
        .menu(&menu.items)
        .show(ui);
    if out.selected {
        acts.push(Act::Go(format!("artists/{}", artist.id)));
    }
    if out.play {
        acts.push(play_artist(artist, false));
    }
    acts.extend(menu.act(out.menu));
}

/// What a song's menu offers: the queue, a playlist, its album and artist, a station from it, and
/// `remove` where the song can leave a list.
pub fn song_menu(here: &Here, id: &str, on_album: bool, remove: Option<(&str, Act)>) -> Menu {
    let mut menu = Menu::default();
    let Some(song) = crate::catalog::song_of(id, &here.library.stations) else {
        return menu;
    };
    let radio = song.id.starts_with("radio:");
    menu.item(
        MenuItem::action("Play next").icon(Icon::ListStart),
        Act::Enqueue(vec![song.id.clone()], Place::Next, song.title.clone()),
    );
    menu.item(
        MenuItem::action("Add to queue").icon(Icon::ListEnd),
        Act::Enqueue(vec![song.id.clone()], Place::End, song.title.clone()),
    );
    menu.rule();
    menu.playlists(here, std::slice::from_ref(&song.id), &song.title);
    menu.rule();
    if on_album && !radio {
        menu.item(
            MenuItem::action("Go to album").icon(Icon::Disc3),
            Act::Go(format!("albums/{}", song.album)),
        );
    }
    menu.item(
        MenuItem::action(format!("Go to {}", artist_name(&song.artist))).icon(Icon::MicVocal),
        Act::Go(format!("artists/{}", song.artist)),
    );
    if !radio {
        menu.item(
            MenuItem::action("Start radio").icon(Icon::Radio),
            Act::Radio(StationKind::Song, song.id.clone()),
        );
    }
    if let Some((label, act)) = remove {
        menu.rule();
        menu.item(MenuItem::action(label).icon(Icon::Trash2).destructive(), act);
    }
    menu
}

/// A song row's own controls: its star and its menu behind "More".
pub fn song_actions(
    ui: &mut Ui,
    here: &Here,
    song: &str,
    on_album: bool,
    remove: Option<(&str, Act)>,
    acts: &mut Vec<Act>,
) {
    let starred = here.library.is_starred(FavoriteKind::Song, song);
    let mut star = starred;
    ui.add(StarButton::new(&mut star));
    if star != starred {
        acts.push(Act::Star(FavoriteKind::Song, song.to_owned(), star));
    }
    let title = crate::catalog::song_of(song, &here.library.stations).map_or_else(|| song.to_owned(), |s| s.title);
    let more = ui.add(IconButton::new(Icon::Ellipsis, format!("More for {title}")));
    let menu = song_menu(here, song, on_album, remove);
    let chosen = DropdownMenu::new(&menu.items).end(true).width(208.0).show(&more);
    acts.extend(menu.act(chosen));
}

/// What a page with nothing in it says, and the way to fill it: true when that was pressed.
pub fn nothing(ui: &mut Ui, icon: Icon, title: &str, description: &str, action: Option<&str>) -> bool {
    Empty::new(title).icon(icon).description(description).show(ui, |ui| {
        action.is_some_and(|label| {
            ui.add(Button::new(label).variant(ButtonVariant::Outline).size(ButtonSize::Sm))
                .clicked()
        })
    })
}
