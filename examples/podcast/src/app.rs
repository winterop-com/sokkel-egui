//! The podcasts app inside the shell, as the web's `Podcasts.tsx` and `App.tsx`: the rail of places
//! with their counts, taking an episode's row dropped on the queue, Starred or Downloads; the top
//! bar's search and trail; the page the address names; the player along the foot and the status
//! line under it; and the settings, the palette and the questions over them.

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui::{self, Ui};
use serde::{Deserialize, Serialize};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, ConfirmAnswer, ConfirmDialog, Icon, Rail, RailDrop, RailEntry, RailGroup,
    Search, SearchGroup, SearchResult, SearchWords, Shell, ShellPart, StatusLine, TableState, Toast, Toaster, TopBar,
    install,
};

use crate::act::{Act, Asking};
use crate::backend::{DownloadState, EpisodeState, Listen, PlayerRow, Prefs, ShowState};
use crate::catalog::{episode_by_id, show_by_id};
use crate::discover::find;
use crate::parts::megabytes;
use crate::script::{length_of, script_of, spoken_script};
use crate::state::{Podcasts, Where};

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The pages the address names, as the web's `PAGES`.
pub const PAGES: [&str; 10] = [
    "home",
    "inbox",
    "queue",
    "discover",
    "shows",
    "show",
    "episode",
    "starred",
    "downloads",
    "stats",
];

/// What each page is called in the trail.
pub fn title_of(page: &str) -> &'static str {
    match page {
        "home" => "Listen now",
        "inbox" => "New",
        "queue" => "Queue",
        "discover" => "Discover",
        "shows" => "Shows",
        "starred" => "Starred",
        "downloads" => "Downloads",
        "stats" => "Stats",
        _ => "Not found",
    }
}

/// What an episode's row is dragged as, onto the queue, Starred or Downloads in the rail, as the
/// web's `EPISODE_DRAG`.
pub const EPISODE_DRAG: &str = "episode";

/// What a page holds while it is shown, as the web's pages keep in their own state.
#[derive(Default)]
pub struct Views {
    /// What is typed in Discover's search.
    pub discover: String,
    /// Which of a show's episodes are listed: all, unplayed, downloaded or starred.
    pub shown: usize,
    pub table: TableState,
}

/// An episode's sound made by its voices, and its loudness for the grown bar's scrubber.
pub struct Sound {
    pub samples: Vec<f32>,
    pub waveform: Vec<f32>,
}

/// What is kept between visits: the backend's rows, as the web's backend keeps them.
#[derive(Serialize, Deserialize)]
pub struct Kept {
    pub shows: Vec<ShowState>,
    pub episodes: Vec<EpisodeState>,
    pub player: PlayerRow,
    pub prefs: Prefs,
    pub listens: Vec<Listen>,
}

pub struct PodcastApp {
    pub appearance: Appearance,
    pub state: Podcasts,
    /// The address, split at its slashes.
    pub path: Vec<String>,
    /// The settings over the page, and the section they show.
    pub settings: Option<String>,
    /// The palette while it is open: what is typed and the row lit.
    pub palette: Option<(String, usize)>,
    /// The question asked before something is done.
    pub asking: Option<Asking>,
    pub views: Views,
    /// What is typed in the top bar's search.
    pub top_query: String,
    pub fullscreen: bool,
    /// Check for new while it asks: how many answers had come before.
    pub checking: Option<usize>,
    /// The summary read aloud: the episode's, and when it ends.
    pub speaking: Option<(String, i64)>,
    /// The episodes' sounds made so far, by episode.
    pub sounds: HashMap<String, Arc<Sound>>,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    /// When the player was last moved on.
    ticked: i64,
    held: bool,
    pub desktop: bool,
}

impl PodcastApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let now = crate::js::now_ms();
        let appearance = cc
            .storage
            .and_then(|s| eframe::get_value(s, "appearance"))
            .unwrap_or_default();
        let mut app = Self::at(now, appearance);
        if let Some(kept) = cc.storage.and_then(|s| eframe::get_value::<Kept>(s, "podcasts")) {
            app.state
                .restore(kept.shows, kept.episodes, kept.player, kept.prefs, kept.listens);
        }
        app.held = false;
        app.desktop = true;
        app
    }

    /// The app at a moment with the backend as it is seeded then, its clock held there, as the
    /// tests and the screenshots take it.
    pub fn at(now: i64, appearance: Appearance) -> Self {
        Self {
            appearance,
            state: Podcasts::new(now),
            path: Vec::new(),
            settings: None,
            palette: None,
            asking: None,
            views: Views::default(),
            top_query: String::new(),
            fullscreen: false,
            checking: None,
            speaking: None,
            sounds: HashMap::new(),
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

    /// Moves a held clock on, as the tests watch an episode play.
    pub fn wait(&mut self, ms: i64) {
        self.elapsed += ms;
    }

    /// Goes to an address, as the web's hash routes: a page and what in it (`show/<id>`,
    /// `episode/<id>`, `discover/<category>/<words>`), or `settings` over the page.
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
        // a page's own state starts afresh on another page or another show, as the web's keys do
        if next.first() != self.path.first() || next.get(1) != self.path.get(1) {
            self.views = Views::default();
        }
        if next.first().map(String::as_str) == Some("discover") {
            self.views.discover = next.get(2).map(|words| decode(words)).unwrap_or_default();
        }
        self.path = next;
    }

    /// The page the address names, and what in it.
    pub fn place(&self) -> (String, Option<String>, Option<String>) {
        let first = self.path.first().cloned().unwrap_or_else(|| "home".into());
        (first, self.path.get(1).cloned(), self.path.get(2).cloned())
    }

    /// An episode's sound and its loudness, made the first time they are asked for.
    pub fn sound_of(&mut self, id: &str) -> Option<Arc<Sound>> {
        if let Some(sound) = self.sounds.get(id) {
            return Some(sound.clone());
        }
        let script = script_of(id)?;
        let samples = crate::voice::render(&script);
        let waveform = crate::voice::levels_of(&samples, crate::voice::LEVELS)
            .into_iter()
            .map(|level| level as f32)
            .collect();
        let sound = Arc::new(Sound { samples, waveform });
        self.sounds.insert(id.to_owned(), sound.clone());
        Some(sound)
    }

    /// Reads an episode's summary aloud, in its first host's voice, or stops it: no sound is
    /// made, the button playing for as long as the voice would speak.
    pub fn speak(&mut self, id: &str) {
        if self.speaking.as_ref().is_some_and(|(one, _)| one == id) {
            self.speaking = None;
            return;
        }
        let Some(episode) = episode_by_id(id) else { return };
        let host = show_by_id(&episode.show)
            .and_then(|show| show.hosts.first().copied())
            .unwrap_or(crate::catalog::Host::Ada);
        let script = spoken_script(&episode.summary, host, id);
        self.speaking = Some((id.to_owned(), self.now() + (script.duration * 1000.0) as i64));
    }

    /// Moves the player and the provider's timers on to now, a quarter second at a time: the
    /// episode plays on at its speed, and ends at its length.
    fn tick(&mut self, now: i64) {
        const STEP: i64 = 250;
        while self.ticked < now {
            let to = (self.ticked + STEP).min(now);
            let seconds = (to - self.ticked) as f64 / 1000.0;
            self.ticked = to;
            self.state.advance(to);
            if self.state.playing && self.state.media() {
                let length = self.state.current().map_or(0.0, length_of);
                let at = self.state.element.time + seconds * self.state.element.rate;
                if at >= length {
                    self.state.ended();
                } else {
                    self.state.played_to(at);
                }
            }
        }
        if self.speaking.as_ref().is_some_and(|(_, until)| *until <= now) {
            self.speaking = None;
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            let busy = self.state.playing
                || self.speaking.is_some()
                || self.checking.is_some()
                || self
                    .state
                    .episodes
                    .iter()
                    .any(|row| row.download == DownloadState::Running);
            ctx.request_repaint_after(std::time::Duration::from_millis(if busy { 33 } else { 1000 }));
        }
        let now = self.now();
        self.tick(now);
        self.say(&ctx);
        if self.path.is_empty() {
            self.path.push("home".into());
        }
        let (first, second, third) = self.place();
        let page = if PAGES.contains(&first.as_str()) {
            first.clone()
        } else {
            "missing".to_owned()
        };
        let show = (page == "show")
            .then(|| second.as_deref().and_then(show_by_id))
            .flatten();
        let episode = (page == "episode")
            .then(|| second.as_deref().and_then(episode_by_id))
            .flatten();
        let episode_show = episode.and_then(|one| show_by_id(&one.show));

        let groups = self.rail_groups(&page);
        let mut active = match page.as_str() {
            "show" | "episode" => "shows".to_owned(),
            other => other.to_owned(),
        };
        let was = active.clone();

        // the trail: Podcasts, then Shows and the show over a show's or an episode's page
        let label = match page.as_str() {
            "show" => show.map_or("Not found".to_owned(), |one| one.title.clone()),
            "episode" => episode.map_or("Not found".to_owned(), |one| one.title.clone()),
            other => title_of(other).to_owned(),
        };
        let mut trail: Vec<(String, Option<String>)> = vec![("Podcasts".into(), Some("home".into()))];
        if page == "show" || page == "episode" {
            trail.push(("Shows".into(), Some("shows".into())));
        }
        if let Some(one) = episode_show.filter(|_| page == "episode") {
            trail.push((one.title.clone(), Some(format!("show/{}", one.id))));
        }
        trail.push((label, None));
        let trail_words: Vec<&str> = trail.iter().map(|(label, _)| label.as_str()).collect();

        let mut acts: Vec<Act> = Vec::new();
        let mut account = None;
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut crumb = None;
        let mut dropped: Option<RailDrop> = None;
        let mut top_asked = sokkel_egui::SearchResponse::default();
        let top_found = self.search_results(&self.top_query);
        let mut bar_acts: Vec<Act> = Vec::new();

        let current = self.state.current().map(str::to_owned);
        let sound = match (&current, self.fullscreen) {
            (Some(id), true) => self.sound_of(id),
            (Some(id), false) => self.sounds.get(id).cloned(),
            _ => None,
        };
        let status = self.status_line();
        let here = crate::episodes::Here {
            state: &self.state,
            now,
            speaking: self.speaking.as_ref().map(|(id, _)| id.as_str()),
            checking: self.checking.is_some(),
        };
        let mut shell = Shell::new();
        if self.desktop {
            shell = shell.desktop(sokkel_egui::DesktopWindow::native());
        }
        if current.is_some() {
            let fullscreen = self.fullscreen;
            let queue_shown = page == "queue";
            let acts = &mut bar_acts;
            let sound = sound.clone();
            shell = shell.bottom(move |ui| {
                crate::player_bar::player_bar(ui, &here, fullscreen, queue_shown, sound.as_deref(), acts);
            });
        }
        let views = &mut self.views;
        let top_query = &mut self.top_query;
        let mut status = Some(status);
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                let rail = Rail::new("Podcasts", &groups, &mut active)
                    .mark(TIDEVANN_MARK)
                    .drops(&mut dropped);
                ui.add(rail);
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada")
                    .place("tidevann.example.org")
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
                            .words(SearchWords {
                                placeholder: "Search podcasts".into(),
                                ..SearchWords::default()
                            })
                            .show(ui);
                    })
                    .search("Search or run a command", "⌘K")
                    .show(ui, |_| {});
                searched |= out.search;
                crumb = out.crumb;
            }
            ShellPart::Page => {
                crate::pages::page(ui, &here, &page, second.as_deref(), third.as_deref(), views, &mut acts);
            }
            ShellPart::Foot => {
                if let Some(status) = status.take() {
                    ui.add(status);
                }
            }
        });
        acts.extend(bar_acts);

        if active != was {
            acts.push(Act::Go(active.clone()));
        }
        if let Some(index) = crumb
            && let Some((_, Some(path))) = trail.get(index)
        {
            acts.push(Act::Go(path.clone()));
        }
        if let Some(drop) = dropped {
            acts.extend(self.dropped(&drop));
        }
        match account {
            Some(AccountAction::Settings) => acts.push(Act::Go("settings".into())),
            Some(AccountAction::Appearance) => acts.push(Act::Go("settings/appearance".into())),
            None => {}
        }
        if let Some((_, id)) = top_asked.opened {
            self.top_query.clear();
            let (kind, id) = id.split_once(':').unwrap_or(("show", id.as_str()));
            acts.push(Act::Go(format!("{kind}/{id}")));
        }
        if let Some(query) = top_asked.see_all {
            self.top_query.clear();
            acts.push(Act::Go(format!("discover/all/{}", encode(query.trim()))));
        }
        if searched {
            self.palette = Some(Default::default());
        }
        let overlay = self.palette.is_some() || self.settings.is_some() || self.asking.is_some();
        if !overlay {
            acts.extend(self.keys(&ctx));
        }
        for act in acts {
            self.apply(&ctx, act);
        }
        self.search_palette(&ctx, &groups);
        self.settings_dialog(&ctx);
        self.questions(&ctx);
        Toaster::new().show(&ctx);
    }

    /// What an episode's row dropped on a place in the rail does, when that place wants it: the
    /// queue takes one not playing nor queued, Starred one not starred, Downloads one not kept.
    fn dropped(&self, drop: &RailDrop) -> Option<Act> {
        if drop.data.kind != EPISODE_DRAG {
            return None;
        }
        let id = drop.data.value.clone();
        let row = self.state.episode(&id)?;
        match drop.entry.as_str() {
            "queue" if self.state.current() != Some(id.as_str()) && !self.state.player.queue.contains(&id) => {
                Some(Act::QueueAdd(id, Where::End))
            }
            "starred" if !row.starred => Some(Act::Star(id, true)),
            "downloads" if row.download == DownloadState::Idle => Some(Act::Download(id)),
            _ => None,
        }
    }

    /// The notes the provider said since the last frame, as toasts; and Check for new's answer.
    fn say(&mut self, ctx: &egui::Context) {
        for toast in std::mem::take(&mut self.state.toasts) {
            let mut note = Toast::new(toast.title).timeout(Some(2.5));
            if let Some(description) = toast.description {
                note = note.description(description);
            }
            note.show(ctx);
        }
        if let Some(before) = self.checking
            && let Some(count) = self.state.checked.get(before).copied()
        {
            self.checking = None;
            let title = match count {
                0 => "No new episodes".to_owned(),
                1 => "1 new episode".to_owned(),
                n => format!("{n} new episodes"),
            };
            let mut note = Toast::new(title).timeout(Some(2.5));
            if count == 0 {
                note = note.description("Every show you follow is up to date.");
            } else if self.state.prefs.auto_download {
                note = note.description("They are downloading now.");
            }
            note.show(ctx);
        }
    }

    /// The rail's places with their counts, the queue, Starred and Downloads each taking an
    /// episode's row.
    pub fn rail_groups(&self, _page: &str) -> Vec<RailGroup> {
        let rows = &self.state.episodes;
        let fresh = rows.iter().filter(|row| row.inbox).count() as u32;
        let kept = rows.iter().filter(|row| row.download == DownloadState::Kept).count() as u32;
        let running = rows.iter().filter(|row| row.download == DownloadState::Running).count() as u32;
        let followed = self.state.shows.iter().filter(|show| show.subscribed).count() as u32;
        let starred = rows.iter().filter(|row| row.starred).count() as u32;
        let queued = self.state.player.queue.len() as u32;
        let entry = |id: &str, label: &str, icon: Icon, count: u32| {
            let entry = RailEntry::new(id, label, icon);
            if count == 0 { entry } else { entry.count(count) }
        };
        vec![
            RailGroup::new(
                None,
                [
                    entry("home", "Listen now", Icon::House, 0),
                    entry("inbox", "New", Icon::Inbox, fresh),
                    entry("queue", "Queue", Icon::ListOrdered, queued).takes(EPISODE_DRAG),
                    entry("discover", "Discover", Icon::Compass, 0),
                ],
            ),
            RailGroup::new(
                Some("Library"),
                [
                    entry("shows", "Shows", Icon::Podcast, followed),
                    entry("starred", "Starred", Icon::Star, starred).takes(EPISODE_DRAG),
                    entry("downloads", "Downloads", Icon::Download, kept + running).takes(EPISODE_DRAG),
                ],
            ),
            RailGroup::new(
                Some("You"),
                [
                    entry("stats", "Stats", Icon::ChartNoAxesColumn, 0),
                    entry("settings", "Settings", Icon::Settings, 0),
                ],
            ),
        ]
    }

    /// The status line: what is downloading or that everything is saved, and the counts.
    fn status_line(&self) -> StatusLine {
        let rows = &self.state.episodes;
        let fresh = rows.iter().filter(|row| row.inbox).count();
        let kept = rows.iter().filter(|row| row.download == DownloadState::Kept).count();
        let running = rows.iter().filter(|row| row.download == DownloadState::Running).count();
        let note = if running > 0 {
            format!("Downloading {running}")
        } else {
            "All changes saved".to_owned()
        };
        StatusLine::new(note)
            .live(running > 0)
            .item("new", fresh)
            .item("queued", self.state.player.queue.len())
            .item("downloaded", kept)
    }

    /// The top bar's answer: the shows and the released episodes the words find.
    pub fn search_results(&self, query: &str) -> Vec<SearchGroup> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        let (shows, episodes) = find(query, |id| self.state.episode(id).is_some_and(|row| row.out));
        vec![
            SearchGroup {
                kind: "shows".into(),
                title: "Shows".into(),
                total: Some(shows.len()),
                results: shows
                    .iter()
                    .take(4)
                    .map(|one| SearchResult {
                        id: format!("show:{}", one.id),
                        title: one.title.clone(),
                        detail: Some(one.blurb.clone()),
                        meta: None,
                        icon: Some(Icon::Podcast),
                        facets: Default::default(),
                    })
                    .collect(),
            },
            SearchGroup {
                kind: "episodes".into(),
                title: "Episodes".into(),
                total: Some(episodes.len()),
                results: episodes
                    .iter()
                    .take(6)
                    .map(|one| SearchResult {
                        id: format!("episode:{}", one.id),
                        title: one.title.clone(),
                        detail: show_by_id(&one.show).map(|show| show.title.clone()),
                        meta: None,
                        icon: Some(Icon::AudioLines),
                        facets: Default::default(),
                    })
                    .collect(),
            },
        ]
    }

    /// The questions asked before what cannot be undone.
    fn questions(&mut self, ctx: &egui::Context) {
        let Some(asking) = self.asking.clone() else { return };
        let rows = &self.state.episodes;
        let (question, detail, label, act) = match &asking {
            Asking::ClearQueue => (
                "Clear the queue?".to_owned(),
                format!(
                    "{} episodes leave the queue. They stay in their shows, and nothing is marked played.",
                    self.state.player.queue.len()
                ),
                "Clear",
                Act::QueueClear,
            ),
            Asking::RemoveDownloads => {
                let kept: Vec<&EpisodeState> = rows.iter().filter(|row| row.download == DownloadState::Kept).collect();
                let size: f64 = kept.iter().map(|row| crate::backend::size_of(&row.id)).sum();
                (
                    "Remove every download?".to_owned(),
                    format!(
                        "{} episodes, {}, leave this device, and downloads under way stop. The episodes stay in their shows.",
                        kept.len(),
                        megabytes(size)
                    ),
                    "Remove all",
                    Act::RemoveAllDownloads,
                )
            }
            Asking::Unfollow(id) => (
                format!(
                    "Unfollow {}?",
                    show_by_id(id).map_or(id.as_str(), |show| show.title.as_str())
                ),
                "Its episodes leave New and new ones stop coming. What is downloaded, starred or queued stays."
                    .to_owned(),
                "Unfollow",
                Act::Unsubscribe(id.clone()),
            ),
            Asking::ClearStats => (
                "Clear the stats?".to_owned(),
                "The time listened and the episodes finished are forgotten. Where you are in each episode is kept."
                    .to_owned(),
                "Clear",
                Act::ClearStats,
            ),
            Asking::Reset => (
                "Reset the demo data?".to_owned(),
                "Shows followed, places in episodes, the queue, downloads, stats and settings go back to how they started, and the page loads again."
                    .to_owned(),
                "Reset",
                Act::Go("home".into()),
            ),
        };
        let answer = ConfirmDialog::new(("question", format!("{asking:?}")), &question, label)
            .detail(detail)
            .destructive(true)
            .show(ctx);
        match answer {
            Some(ConfirmAnswer::Confirm) => {
                self.asking = None;
                if asking == Asking::Reset {
                    self.reset_demo();
                }
                self.apply(ctx, act);
            }
            Some(_) => self.asking = None,
            None => {}
        }
    }

    /// Everything as the demo started, the page loaded again at Listen now.
    pub fn reset_demo(&mut self) {
        let now = self.now();
        self.state = Podcasts::new(now);
        self.fullscreen = false;
        self.settings = None;
        self.speaking = None;
        self.checking = None;
        self.views = Views::default();
    }
}

/// Words written into an address, as `encodeURIComponent` writes them.
pub fn encode(words: &str) -> String {
    let mut out = String::new();
    for byte in words.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Words read back from an address, as `decodeURIComponent` reads them.
pub fn decode(words: &str) -> String {
    let bytes = words.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            let byte = (high * 16 + low) as u8;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl eframe::App for PodcastApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.state.away();
        eframe::set_value(storage, "appearance", &self.appearance);
        let state = &self.state;
        let kept = Kept {
            shows: state.shows.clone(),
            episodes: state.episodes.clone(),
            player: state.player.clone(),
            prefs: state.prefs.clone(),
            listens: state.listens.clone(),
        };
        eframe::set_value(storage, "podcasts", &kept);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_go_into_an_address_and_come_back() {
        assert_eq!(encode("low tide"), "low%20tide");
        assert_eq!(decode("low%20tide"), "low tide");
        assert_eq!(decode(&encode("ø and å")), "ø and å");
    }
}
