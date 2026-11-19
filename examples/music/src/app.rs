//! The music app inside the shell, as the web's `Music.tsx` and `App.tsx`: the rail of places with
//! the library's tree while the listener browses by artist, the search and the trail, the page the
//! address names, the queue beside it, the player along the foot and the status line under it, and
//! the settings, the palette and the dialogs over them.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, Icon, NotFound, Page, Rail, RailBody, RailDrop, RailEntry, RailGroup,
    Search, SearchField, SearchGroup, SearchResult, Shell, ShellPart, StatusLine, StatusTone, TableState, Toaster,
    TopBar, Tree, TreeNode, install,
};

use crate::act::Act;
use crate::catalog::{album_by_id, albums, albums_of_artist, artist_by_id, artists, songs};
use crate::data::{DownloadState, Library};
use crate::dialogs::{Asking, DialogState};
use crate::player::Player;
use crate::search::{GroupKind, search_groups};
use crate::synth::Seeded;

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The places the address names.
pub const PLACES: [&str; 8] = [
    "albums",
    "artists",
    "tracks",
    "favorites",
    "downloads",
    "radio",
    "playlists",
    "search",
];

/// What each place is called in the trail.
pub fn label_of(place: &str) -> &'static str {
    match place {
        "artists" => "Artists",
        "tracks" => "Tracks",
        "favorites" => "Favorites",
        "downloads" => "Downloads",
        "radio" => "Radio",
        "playlists" => "Playlists",
        "search" => "Search",
        _ => "Albums",
    }
}

/// The kinds a song's row is dragged as from a page's list, the queue and a playlist (the web's
/// `SONG_DRAG`), each of which a playlist in the rail takes.
pub const SONG_DRAG: [&str; 3] = ["song", "queued-song", "playlist-song"];

/// What a page holds while it is shown: what is typed in its filter and the order chosen, as the
/// web's pages keep in their own state.
#[derive(Default)]
pub struct Views {
    pub albums_query: String,
    pub artists_query: String,
    pub artists_order: usize,
    pub tracks_query: String,
    pub tracks_order: usize,
    /// The rail's filter over the library's tree, and its open artists.
    pub tree_query: String,
    pub tree_open: HashSet<String>,
    /// The downloads' table.
    pub table: TableState,
}

/// A song's sound made by the synthesizer, and its waveform for the grown bar's scrubber.
pub struct Sound {
    pub samples: Vec<f32>,
    pub waveform: Vec<f32>,
}

pub struct MusicApp {
    pub appearance: Appearance,
    pub library: Library,
    pub player: Player,
    /// The address, split at its slashes.
    pub path: Vec<String>,
    /// The addresses before this one, for a page's back button.
    pub history: Vec<Vec<String>>,
    /// The settings over the page, and the section they show.
    pub settings: Option<String>,
    /// The palette while it is open: what is typed and the row lit.
    pub palette: Option<(String, usize)>,
    /// The question asked in a dialog, and the dialog's fields.
    pub asking: Option<Asking>,
    pub dialog: DialogState,
    /// The question before every download is removed, and before the demo's data goes back.
    pub clearing: bool,
    pub resetting: bool,
    pub views: Views,
    /// What is typed in the top bar's search.
    pub top_query: String,
    /// The queue panel's tab: the queue or what was played.
    pub panel_tab: usize,
    pub collapsed: bool,
    pub fullscreen: bool,
    /// The songs' sounds made so far, by song.
    pub sounds: HashMap<String, Arc<Sound>>,
    /// Where the shuffles are drawn from.
    pub random: Seeded,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    /// When the player and the downloads were last moved on.
    ticked: i64,
    held: bool,
    pub desktop: bool,
}

impl MusicApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let now = crate::js::now_ms();
        let appearance = cc
            .storage
            .and_then(|s| eframe::get_value(s, "appearance"))
            .unwrap_or_default();
        let library: Library = cc
            .storage
            .and_then(|s| eframe::get_value(s, "library"))
            .unwrap_or_default();
        let mut app = Self::with(now, appearance, library);
        app.held = false;
        app.desktop = true;
        app
    }

    /// The app at a moment with a new listener's library, its clock held there, as the tests and
    /// the screenshots take it.
    pub fn at(now: i64, appearance: Appearance) -> Self {
        Self::with(now, appearance, Library::default())
    }

    fn with(now: i64, appearance: Appearance, mut library: Library) -> Self {
        // the player picked up where it was left, paused
        let mut player = Player::default();
        let row = library.player.clone();
        let resume = library.settings.resume;
        player.restore(&mut library, &row, resume, now);
        Self {
            appearance,
            library,
            player,
            path: Vec::new(),
            history: Vec::new(),
            settings: None,
            palette: None,
            asking: None,
            dialog: DialogState::default(),
            clearing: false,
            resetting: false,
            views: Views::default(),
            top_query: String::new(),
            panel_tab: 0,
            collapsed: false,
            fullscreen: false,
            sounds: HashMap::new(),
            random: Seeded::new((now % 1_000_000_007) as u32),
            started: now,
            started_at: None,
            elapsed: 0,
            ticked: now,
            held: true,
            desktop: false,
        }
    }

    pub fn now(&self) -> i64 {
        self.started + self.elapsed
    }

    /// Moves a held clock on, as the tests watch a song play or a download come in.
    pub fn wait(&mut self, ms: i64) {
        self.elapsed += ms;
    }

    /// Goes to an address: a place and what in it (`albums/<id>`, `search/<words>`), or `settings`
    /// over the page.
    pub fn go(&mut self, path: &str) {
        let path = path.trim_matches('/');
        if let Some(rest) = path.strip_prefix("settings") {
            let section = rest.trim_start_matches('/');
            self.settings = Some(if section.is_empty() { "playback" } else { section }.to_owned());
            return;
        }
        let next: Vec<String> = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
        if next == self.path {
            return;
        }
        // a page's own filter starts empty on another place, as the web's page state does
        if next.first() != self.path.first() || next.len() != self.path.len() {
            let tree = (
                std::mem::take(&mut self.views.tree_query),
                std::mem::take(&mut self.views.tree_open),
            );
            self.views = Views {
                tree_query: tree.0,
                tree_open: tree.1,
                ..Views::default()
            };
        }
        // the tree opens on the artist the page is of
        let opened = match (next.first().map(String::as_str), next.get(1)) {
            (Some("albums"), Some(id)) => album_by_id(id).map(|album| album.artist.clone()),
            (Some("artists"), Some(id)) => artist_by_id(id).map(|artist| artist.id.clone()),
            _ => None,
        };
        if let Some(artist) = opened {
            self.views.tree_open.insert(format!("artist:{artist}"));
        }
        if !self.path.is_empty() {
            self.history.push(std::mem::take(&mut self.path));
        }
        self.path = next;
    }

    /// Back to the address before this one, as the browser's back; the albums when there is none.
    pub fn back(&mut self) {
        match self.history.pop() {
            Some(path) => self.path = path,
            None => self.path = vec!["albums".into()],
        }
    }

    /// The place the address names, and what in it.
    pub fn place(&self) -> (String, Option<String>) {
        let first = self.path.first().cloned().unwrap_or_else(|| "albums".into());
        (first, self.path.get(1).cloned())
    }

    /// A song's sound and waveform, made the first time it is asked for.
    pub fn sound_of(&mut self, song: &crate::catalog::Song) -> Arc<Sound> {
        self.sounds
            .entry(song.id.clone())
            .or_insert_with(|| {
                let samples = crate::synth::render(song.seed, song.style);
                let waveform = crate::synth::waveform_of(&samples, crate::synth::WAVEFORM_BINS)
                    .into_iter()
                    .map(|level| level as f32)
                    .collect();
                Arc::new(Sound { samples, waveform })
            })
            .clone()
    }

    /// Moves the player and the downloads on to now: the song plays on at its speed, and the next
    /// starts as it ends.
    fn tick(&mut self, now: i64) {
        let since = (now - self.ticked).max(0);
        self.ticked = now;
        self.library.advance(now);
        if !self.player.playing || since == 0 {
            return;
        }
        let rate = self.library.settings.rate;
        let at = self.player.position + since as f64 / 1000.0 * rate;
        let duration = self.player.duration;
        if let Some(next) = self
            .player
            .crossfade_due(self.library.settings.crossfade, duration - at)
        {
            self.player.fade_to(&mut self.library, next, now);
        } else if duration > 0.0 && at >= duration {
            self.player.ended(&mut self.library, now);
        } else {
            self.player.heard(&mut self.library, at, now);
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            if self.player.playing || self.library.downloading() {
                ctx.request_repaint_after(std::time::Duration::from_millis(33));
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(1000));
            }
        }
        let now = self.now();
        self.tick(now);
        // a bare address opens the albums
        if self.path.is_empty() {
            self.path.push("albums".into());
        }
        let (place, second) = self.place();
        let missing = !PLACES.contains(&place.as_str());
        let album = (place == "albums")
            .then(|| second.as_deref().and_then(album_by_id))
            .flatten();
        let artist = (place == "artists")
            .then(|| second.as_deref().and_then(artist_by_id))
            .flatten();
        let playlist = (place == "playlists")
            .then(|| second.as_deref().and_then(|id| self.library.playlist(id)).cloned())
            .flatten();

        let groups = self.rail_groups();
        let mut active = match (&playlist, album) {
            (Some(playlist), _) => format!("playlist:{}", playlist.id),
            (None, Some(_)) => "artists".to_owned(),
            _ => place.clone(),
        };
        let was = active.clone();

        // the trail, its crumbs before the last going back up
        let up = label_of(&place).to_owned();
        let mut trail: Vec<(String, Option<String>)> = Vec::new();
        if missing {
            trail.push(("Not found".into(), None));
        } else if second.is_none() {
            trail.push((up, None));
        } else if let Some(album) = album {
            trail.push((up, Some(place.clone())));
            trail.push((
                crate::catalog::artist_name(&album.artist),
                Some(format!("artists/{}", album.artist)),
            ));
            trail.push((album.title.clone(), None));
        } else if let Some(artist) = artist {
            trail.push((up, Some(place.clone())));
            trail.push((artist.name.clone(), None));
        } else if let Some(playlist) = &playlist {
            trail.push((up, Some(place.clone())));
            trail.push((playlist.name.clone(), None));
        } else if place == "search" {
            trail.push(("Search".into(), None));
            trail.push((format!("\u{201c}{}\u{201d}", second.clone().unwrap_or_default()), None));
        } else {
            trail.push((up, Some(place.clone())));
        }
        let trail_words: Vec<&str> = trail.iter().map(|(label, _)| label.as_str()).collect();

        // the library's tree stands in the rail while the listener browses it by artist
        let browsing = !missing && (place == "artists" || album.is_some());
        let tree = library_tree();
        let mut picked = album
            .map(|album| format!("album:{}", album.id))
            .or_else(|| artist.map(|artist| format!("artist:{}", artist.id)));
        let was_picked = picked.clone();

        let mut acts: Vec<Act> = Vec::new();
        let mut account = None;
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut crumb = None;
        let mut dropped: Option<RailDrop> = None;
        let mut top_asked = sokkel_egui::SearchResponse::default();
        let top_found = search_results(&self.top_query);
        let mut queue_open = self.library.settings.queue_open;
        let was_open = queue_open;
        let mut collapsed = self.collapsed;
        let mut panel_acts: Vec<Act> = Vec::new();
        let mut bar_acts: Vec<Act> = Vec::new();

        // the player's sound, for the grown bar's waveform and the visualizer
        let song = self.player.current_song(&self.library);
        let grown = self.library.settings.bar_size.is_some_and(|size| size > 96.0);
        let sound = song
            .as_ref()
            .filter(|_| grown || self.fullscreen)
            .map(|song| self.sound_of(song));

        let mut status = Some(self.status_line());
        let here = crate::pages::Here {
            library: &self.library,
            player: &self.player,
            now,
        };
        let mut shell = Shell::new().collapsed(collapsed);
        if self.desktop {
            shell = shell.desktop(sokkel_egui::DesktopWindow::native());
        }
        let panel_tab = &mut self.panel_tab;
        if queue_open {
            let acts = &mut panel_acts;
            shell = shell.panel(&mut queue_open, move |ui| {
                crate::queue_panel::queue_panel(ui, &here, panel_tab, acts);
            });
        }
        if song.is_some() {
            let fullscreen = self.fullscreen;
            let acts = &mut bar_acts;
            let sound = sound.clone();
            shell = shell.bottom(move |ui| {
                crate::player_bar::player_bar(ui, &here, fullscreen, sound.as_deref(), acts);
            });
        }
        let views = &mut self.views;
        let top_query = &mut self.top_query;
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                let mut rail = Rail::new("Music", &groups, &mut active)
                    .mark(TIDEVANN_MARK)
                    .collapsible(&mut collapsed)
                    .drops(&mut dropped);
                if browsing {
                    let query = views.tree_query.clone();
                    let (tree_query, tree_open) = (&mut views.tree_query, &mut views.tree_open);
                    let (tree, picked) = (&tree, &mut picked);
                    rail = rail.body(RailBody {
                        title: Some("Library".into()),
                        search: Some(Box::new(move |ui: &mut Ui| {
                            SearchField::new("library-filter", tree_query)
                                .placeholder("Filter")
                                .width(ui.available_width())
                                .show(ui);
                        })),
                        places_above: false,
                        content: Box::new(move |ui: &mut Ui| {
                            Tree::new("library-tree", tree, picked, tree_open)
                                .query(&query)
                                .show(ui);
                        }),
                    });
                }
                ui.add(rail);
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada")
                    .place("tidevann.example.org")
                    .local(true)
                    .appearance(true)
                    .show(ui);
            }
            ShellPart::Top => {
                let out = TopBar::new(&trail_words)
                    .linked(true)
                    .start(|ui| {
                        top_asked = Search::new("top-search", top_query, &top_found)
                            .see_all(true)
                            .width(256.0)
                            .show(ui);
                    })
                    .search("Search or run a command", "⌘K")
                    .show(ui, |_| {});
                searched |= out.search;
                crumb = out.crumb;
            }
            ShellPart::Page => {
                if missing {
                    Page::scroll(ui, |ui| {
                        if NotFound::new(format!("#/{}", trail_words.join("/"))).show(ui) {
                            acts.push(Act::Go("albums".into()));
                        }
                    });
                } else {
                    crate::pages::page(ui, &here, &place, second.as_deref(), views, &mut acts);
                }
            }
            ShellPart::Foot => {
                if let Some(status) = status.take() {
                    ui.add(status);
                }
            }
        });
        acts.extend(panel_acts);
        acts.extend(bar_acts);

        self.collapsed = collapsed;
        if queue_open != was_open {
            let mut settings = self.library.settings.clone();
            settings.queue_open = queue_open;
            acts.push(Act::Settings(settings));
        }
        if active != was {
            match active.strip_prefix("playlist:") {
                Some(id) => acts.push(Act::Go(format!("playlists/{id}"))),
                None => acts.push(Act::Go(active.clone())),
            }
        }
        if picked != was_picked
            && let Some(id) = &picked
            && let Some((kind, reference)) = id.split_once(':')
        {
            let place = if kind == "album" { "albums" } else { "artists" };
            acts.push(Act::Go(format!("{place}/{reference}")));
        }
        if let Some(index) = crumb
            && let Some((_, Some(path))) = trail.get(index)
        {
            acts.push(Act::Go(path.clone()));
        }
        if let Some(drop) = dropped
            && let Some(id) = drop.entry.strip_prefix("playlist:")
        {
            let song = drop.data.value.split('~').next().unwrap_or_default().to_owned();
            if crate::catalog::song_of(&song, &self.library.stations).is_some() {
                acts.push(Act::AddTo(id.to_owned(), vec![song], Some(drop.data.label.clone())));
            }
        }
        match account {
            Some(AccountAction::Settings) => acts.push(Act::Go("settings".into())),
            Some(AccountAction::Appearance) => acts.push(Act::Go("settings/appearance".into())),
            None => {}
        }
        if let Some((kind, id)) = top_asked.opened {
            self.top_query.clear();
            acts.extend(open_search(&kind, &id));
        }
        if let Some(query) = top_asked.see_all {
            self.top_query.clear();
            acts.push(Act::Go(format!("search/{query}")));
        }
        if searched {
            self.palette = Some(Default::default());
        }
        let overlay = self.palette.is_some() || self.settings.is_some() || self.asking.is_some() || self.clearing;
        if !overlay {
            acts.extend(self.keys(&ctx));
        }
        for act in acts {
            self.apply(&ctx, act);
        }
        self.search_palette(&ctx, &groups);
        self.dialogs(&ctx);
        self.settings_dialog(&ctx);
        Toaster::new().show(&ctx);
    }

    /// The rail's places: the library, the playlists by name, and the settings.
    pub fn rail_groups(&self) -> Vec<RailGroup> {
        let running = self
            .library
            .downloads
            .iter()
            .filter(|row| row.state == DownloadState::Running)
            .count();
        let mut downloads = RailEntry::new("downloads", "Downloads", Icon::CloudDownload);
        if running > 0 {
            downloads = downloads
                .description(format!("{running} coming in"))
                .count(running as u32);
        } else {
            downloads = downloads.description("Kept on this device");
        }
        let radio = match (&self.player.radio, &self.player.source) {
            (None, _) => "Stations without end".to_owned(),
            (Some(_), source) => format!(
                "On air: {}",
                source.as_ref().map_or("a station", |source| source.label.as_str())
            ),
        };
        let mut lists: Vec<&crate::data::Playlist> = self.library.playlists.iter().collect();
        lists.sort_by_key(|one| one.name.to_lowercase());
        let mut playlist_entries = vec![
            RailEntry::new("playlists", "All playlists", Icon::ListMusic)
                .description(format!("{} playlists", self.library.playlists.len())),
        ];
        for row in lists {
            let mut entry = RailEntry::new(format!("playlist:{}", row.id), &row.name, Icon::ListMusic);
            for kind in SONG_DRAG {
                entry = entry.takes(kind);
            }
            playlist_entries.push(entry);
        }
        vec![
            RailGroup::new(
                None,
                [
                    RailEntry::new("albums", "Albums", Icon::Disc3).description(format!("{} albums", albums().len())),
                    RailEntry::new("artists", "Artists", Icon::MicVocal).description("The library as a tree"),
                    RailEntry::new("tracks", "Tracks", Icon::Music2).description(format!("{} songs", songs().len())),
                    RailEntry::new("favorites", "Favorites", Icon::Star)
                        .description(format!("{} starred", self.library.favorites.len())),
                    downloads,
                    RailEntry::new("radio", "Radio", Icon::Radio).description(radio),
                ],
            ),
            RailGroup::new(Some("Playlists"), playlist_entries),
            RailGroup::new(
                None,
                [RailEntry::new("settings", "Settings", Icon::Settings).description("Playback, visualizer, look")],
            ),
        ]
    }

    /// The status line: what plays and from where, the crossfade and speed when set, the library
    /// and what is kept.
    fn status_line(&self) -> StatusLine {
        let song = self.player.current_song(&self.library);
        let playing = self.player.playing;
        let note = match &song {
            None => "Nothing playing".to_owned(),
            Some(_) => format!(
                "{} on this computer, from {}",
                if playing { "Playing" } else { "Paused" },
                self.player
                    .source
                    .as_ref()
                    .map_or("the queue", |source| source.label.as_str())
            ),
        };
        let settings = &self.library.settings;
        let mut line =
            StatusLine::new(note)
                .live(playing)
                .tone(if playing { StatusTone::Live } else { StatusTone::Quiet });
        if settings.crossfade > 0.0 {
            line = line.item(
                "crossfade",
                format!("{} s", crate::js::number_to_string(settings.crossfade)),
            );
        }
        if settings.rate != 1.0 {
            line = line.item("speed", format!("{}x", crate::js::number_to_string(settings.rate)));
        }
        let kept = self
            .library
            .downloads
            .iter()
            .filter(|row| row.state == DownloadState::Kept)
            .count();
        line.item("library", format!("{} albums", albums().len()))
            .item("kept", kept)
    }
}

/// The library as the rail's tree: each artist by name with their albums under them, newest first.
pub fn library_tree() -> Vec<TreeNode> {
    let mut all: Vec<&crate::catalog::Artist> = artists().iter().collect();
    all.sort_by_key(|artist| artist.name.to_lowercase());
    all.into_iter()
        .map(|artist| {
            let albums = albums_of_artist(&artist.id);
            TreeNode::new(format!("artist:{}", artist.id), &artist.name)
                .meta(albums.len().to_string())
                .children(
                    albums
                        .iter()
                        .map(|album| {
                            TreeNode::new(format!("album:{}", album.id), &album.title).meta(album.year.to_string())
                        })
                        .collect(),
                )
        })
        .collect()
}

/// The library's answer to the top bar's search, as its groups: artists, albums and songs.
pub fn search_results(query: &str) -> Vec<SearchGroup> {
    if query.trim().chars().count() < 2 {
        return Vec::new();
    }
    search_groups(query)
        .into_iter()
        .map(|group| {
            let (kind, icon) = match group.kind {
                GroupKind::Artists => ("artists", Icon::MicVocal),
                GroupKind::Albums => ("albums", Icon::Disc3),
                GroupKind::Songs => ("songs", Icon::Music2),
            };
            SearchGroup {
                kind: kind.into(),
                title: group.title.into(),
                total: Some(group.total),
                results: group
                    .results
                    .into_iter()
                    .map(|result| SearchResult {
                        id: result.id,
                        title: result.title,
                        detail: Some(result.detail),
                        meta: None,
                        icon: Some(icon),
                        facets: Default::default(),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// What opening a search's result does: an artist or an album opens its page, a song opens its
/// album and plays it from that song.
pub fn open_search(kind: &str, id: &str) -> Vec<Act> {
    match kind {
        "artists" => vec![Act::Go(format!("artists/{id}"))],
        "albums" => vec![Act::Go(format!("albums/{id}"))],
        _ => {
            let Some(song) = crate::catalog::library_song(id) else {
                return Vec::new();
            };
            let ids: Vec<String> = crate::catalog::songs_of_album(&song.album)
                .iter()
                .map(|one| one.id.clone())
                .collect();
            let start = ids.iter().position(|one| *one == song.id).unwrap_or(0);
            let title = album_by_id(&song.album).map_or_else(|| song.album.clone(), |album| album.title.clone());
            vec![
                Act::Play {
                    songs: ids,
                    start,
                    source: crate::data::Source::new(
                        crate::data::SourceKind::Album,
                        title,
                        Some(format!("/albums/{}", song.album)),
                    ),
                    shuffle: None,
                },
                Act::Go(format!("albums/{}", song.album)),
            ]
        }
    }
}

impl eframe::App for MusicApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let now = self.now();
        self.player.stop(&mut self.library, now);
        eframe::set_value(storage, "appearance", &self.appearance);
        eframe::set_value(storage, "library", &self.library);
    }
}
