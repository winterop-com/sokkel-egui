//! The video site inside the shell, as the web's `Site.tsx` and `App.tsx`: the rail of its places
//! (collapsed to its icons, with their counts at their corners, while a video plays), the top bar's
//! search and trail, the page the address names, the status line, and the settings, the palette,
//! the dialogs and the questions over them. The clock is the app's: the data layer's timers and
//! the player move with it.

use std::collections::{HashMap, VecDeque};

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, ConfirmAnswer, ConfirmDialog, Icon, Rail, RailEntry, RailGroup,
    SearchField, Shell, ShellPart, StatusLine, Toast, Toaster, TopBar, install, toast_acted,
};

use crate::act::{Act, Asking};
use crate::data::{Data, Kept};
use crate::pages::watch::{COUNTDOWN, PlayerState, Watching};
use crate::site;

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The pages the address names.
pub const PAGES: [&str; 12] = [
    "home",
    "watch",
    "channel",
    "subscriptions",
    "library",
    "history",
    "later",
    "liked",
    "playlists",
    "playlist",
    "downloads",
    "search",
];

/// What a page holds while it is shown, as the web's pages keep in their own state.
#[derive(Default)]
pub struct Views {
    /// Home's chip.
    pub chip: String,
    /// Subscriptions' chip, and whether Manage is open.
    pub channel_chip: String,
    pub managing: bool,
    /// The channel's sort.
    pub sort: crate::pages::channel::Sort,
    /// History's search.
    pub history_search: String,
    /// The search page's field, the kind it shows and its sort.
    pub search_text: String,
    pub kind: crate::pages::search::Kind,
    pub search_sort: crate::pages::search::Sort,
    /// What plays next narrowed to.
    pub filter: crate::pages::watch::aside::Filter,
    /// The comments' order.
    pub order: crate::pages::watch::comments::Order,
    /// Playlists' new playlist, and Save to's.
    pub new_playlist: String,
    pub save_name: String,
    pub table: sokkel_egui::TableState,
}

/// The player's own state: where it is, whether it plays, its sound, and when it ended.
#[derive(Clone, Debug)]
pub struct Player {
    pub time: f64,
    pub playing: bool,
    /// Whether it has played at all.
    pub started: bool,
    /// Whether the clip is handed to it yet.
    pub attached: bool,
    pub volume: f32,
    pub muted: bool,
    pub rate: f64,
    /// When it played to its end, for Up next's countdown.
    pub ended_at: Option<i64>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            time: 0.0,
            playing: false,
            started: false,
            attached: false,
            volume: 1.0,
            muted: false,
            rate: 1.0,
            ended_at: None,
        }
    }
}

pub struct VideoApp {
    pub appearance: Appearance,
    pub data: Data,
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
    /// The video playing, while its page is open.
    pub watching: Option<Watching>,
    pub player: Player,
    /// Save to, for this video.
    pub saving: Option<String>,
    /// Share, for this video, from this second, and whether the link starts there.
    pub sharing: Option<(String, Option<f64>, bool)>,
    pub fullscreen: bool,
    pub picture_in_picture: bool,
    /// What the action of the next toast with one does, in the order they are said.
    pub toast_acts: VecDeque<Act>,
    toast_actions: HashMap<u64, Act>,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    /// When the player and the data were last moved on.
    ticked: i64,
    held: bool,
    /// Whether a video may start by itself as its page opens: a browser lets one only after the
    /// viewer has pressed something on the site, as the web's screenshots are taken without.
    pub may_autoplay: bool,
    pub desktop: bool,
}

impl VideoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let now = crate::js::now_ms();
        let appearance = cc
            .storage
            .and_then(|s| eframe::get_value(s, "appearance"))
            .unwrap_or_default();
        let mut app = Self::at(now, appearance);
        if let Some(kept) = cc.storage.and_then(|s| eframe::get_value::<Kept>(s, "video")) {
            app.data.restore(kept);
        }
        app.held = false;
        app.desktop = true;
        app.may_autoplay = true;
        app
    }

    /// The site at a moment with the data as it is seeded then, its clock held there, as the tests
    /// and the screenshots take it.
    pub fn at(now: i64, appearance: Appearance) -> Self {
        Self {
            appearance,
            data: Data::new(now),
            path: Vec::new(),
            settings: None,
            palette: None,
            asking: None,
            views: Views::default(),
            top_query: String::new(),
            watching: None,
            player: Player::default(),
            saving: None,
            sharing: None,
            fullscreen: false,
            picture_in_picture: false,
            toast_acts: VecDeque::new(),
            toast_actions: HashMap::new(),
            started: now,
            started_at: None,
            elapsed: 0,
            ticked: now,
            held: true,
            may_autoplay: false,
            desktop: false,
        }
    }

    pub fn now(&self) -> i64 {
        self.started + self.elapsed
    }

    /// Moves a held clock on, as the tests watch a video play.
    pub fn wait(&mut self, ms: i64) {
        self.elapsed += ms;
    }

    /// Goes to an address, as the web's hash routes: `watch/<id>[/list/<id>|/at/<second>]`,
    /// `channel/<id>[/about]`, `history[/clear]`, `playlist/<id>`, `search/<words>`, and the
    /// places; `settings[/<section>]` opens over the page.
    pub fn go(&mut self, path: &str) {
        let path = path.trim_start_matches('#').trim_matches('/');
        if let Some(rest) = path.strip_prefix("settings") {
            let section = rest.trim_start_matches('/');
            self.settings = Some(if section.is_empty() { "playback" } else { section }.to_owned());
            return;
        }
        let next: Vec<String> = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
        let next = if next.is_empty() { vec!["home".to_owned()] } else { next };
        if next == self.path {
            return;
        }
        // a page's own state starts afresh on another page, as the web's keys do
        if next.first() != self.path.first() || next.get(1) != self.path.get(1) {
            let keep = std::mem::take(&mut self.views.table);
            self.views = Views::default();
            self.views.table = keep;
        }
        match next.first().map(String::as_str) {
            Some("search") => {
                let words = next
                    .get(1)
                    .map(|one| crate::js::decode_uri_component(one))
                    .unwrap_or_default();
                self.views.search_text = words.clone();
                self.top_query = words;
            }
            Some("history") if next.get(1).map(String::as_str) == Some("clear") => {
                self.asking = Some(Asking::ClearHistory { clearing: true });
            }
            _ => {}
        }
        // the watch page, made again for another video or another second, as its key
        let watch = |path: &[String]| {
            (path.first().map(String::as_str) == Some("watch")).then(|| (path.get(1).cloned(), path.get(3).cloned()))
        };
        if watch(&next) != watch(&self.path) {
            self.leave_watch();
            if let Some((Some(id), _)) = watch(&next) {
                let list = (next.get(2).map(String::as_str) == Some("list"))
                    .then(|| next.get(3).cloned())
                    .flatten();
                let at = (next.get(2).map(String::as_str) == Some("at"))
                    .then(|| next.get(3).and_then(|s| s.parse::<f64>().ok()))
                    .flatten();
                self.watching = Watching::open(&id, list.as_deref(), at, 1440.0);
                self.player = Player {
                    volume: self.player.volume,
                    muted: self.player.muted,
                    rate: self.data.prefs.rate,
                    ..Player::default()
                };
            }
        }
        self.path = next;
    }

    /// The video's page is left: where the viewer was is kept, as the player's pause on unmount.
    fn leave_watch(&mut self) {
        if self.player.playing {
            self.set_playing(false);
        }
        self.watching = None;
        self.fullscreen = false;
    }

    /// The page the address names, and what in it.
    pub fn place(&self) -> (String, Option<String>, Option<String>, Option<String>) {
        let first = self.path.first().cloned().unwrap_or_else(|| "home".into());
        (
            first,
            self.path.get(1).cloned(),
            self.path.get(2).cloned(),
            self.path.get(3).cloned(),
        )
    }

    /// Where the player is, as the page asks its handle.
    pub fn player_state(&self) -> Option<PlayerState> {
        self.player.attached.then_some(PlayerState {
            time: self.player.time,
            playing: self.player.playing,
        })
    }

    /// Plays or pauses, telling the page.
    pub fn set_playing(&mut self, on: bool) {
        let Some(watching) = &mut self.watching else { return };
        if !self.player.attached || on == self.player.playing {
            return;
        }
        if on && self.player.time >= watching.video.duration {
            self.player.time = 0.0;
        }
        self.player.playing = on;
        if on {
            self.player.started = true;
            self.player.ended_at = None;
        }
        let state = PlayerState {
            time: self.player.time,
            playing: on,
        };
        watching.playing_changed(&mut self.data, on, state);
    }

    /// Moves the player to a second within the video.
    pub fn seek(&mut self, at: f64) {
        let Some(watching) = &self.watching else { return };
        self.player.time = at.clamp(0.0, watching.video.duration);
    }

    /// Moves the data's timers and the player on to now, a tenth of a second at a time: the video
    /// plays on at its speed, and ends at its length.
    fn tick(&mut self, now: i64) {
        const STEP: i64 = 100;
        while self.ticked < now {
            let to = (self.ticked + STEP).min(now);
            let seconds = (to - self.ticked) as f64 / 1000.0;
            self.ticked = to;
            self.data.advance(to);
            self.attach();
            let Some(watching) = &mut self.watching else { continue };
            if self.player.playing {
                let length = watching.video.duration;
                let at = self.player.time + seconds * self.player.rate;
                if at >= length {
                    self.player.time = length;
                    self.player.playing = false;
                    let state = PlayerState {
                        time: length,
                        playing: false,
                    };
                    watching.playing_changed(&mut self.data, false, state);
                    watching.ended(&mut self.data);
                    self.player.ended_at = Some(to);
                } else {
                    self.player.time = at;
                    watching.time(&mut self.data, at);
                }
            }
            // Up next counts down while autoplay is on, and plays the next at zero
            if watching.ended
                && self.data.prefs.autoplay
                && let Some(ended) = self.player.ended_at
                && (to - ended) as f64 >= COUNTDOWN * 1000.0
                && watching.next(&self.data).is_some()
            {
                self.player.ended_at = None;
                watching.play_next(&mut self.data);
                self.went();
            }
        }
    }

    /// The clip handed to the player once the history has come: it starts where the page says, and
    /// plays as the page opens when it may.
    fn attach(&mut self) {
        let Some(watching) = &self.watching else { return };
        if self.player.attached || !watching.attached(&self.data) {
            return;
        }
        self.player.attached = true;
        self.player.time = watching.start_at(&self.data).unwrap_or(0.0);
        if watching.play_on_load && self.may_autoplay {
            self.set_playing(true);
        }
    }

    /// The places the data layer went to since the last frame.
    fn went(&mut self) {
        for path in std::mem::take(&mut self.data.went) {
            self.go(&path);
        }
    }

    /// The notes the data layer said since the last frame, as toasts; one with an action takes the
    /// next act waiting for one.
    fn say(&mut self, ctx: &egui::Context) {
        for toast in std::mem::take(&mut self.data.toasts) {
            let mut note = if toast.kind == Some("error") {
                Toast::error(toast.title)
            } else {
                Toast::new(toast.title)
            }
            .timeout(Some(4.0));
            if let Some(description) = toast.description {
                note = note.description(description);
            }
            let act = if let Some(action) = &toast.action {
                note = note.action(action);
                self.toast_acts.pop_front()
            } else {
                None
            };
            let id = note.show(ctx);
            if let Some(act) = act {
                self.toast_actions.insert(id, act);
            }
        }
        for id in toast_acted(ctx) {
            if let Some(act) = self.toast_actions.remove(&id) {
                self.apply(ctx, act);
            }
        }
    }

    /// The rail's places, as the web's groups: Browse, You with their counts, and the channels
    /// followed, each with its color's dot; and the one active.
    pub fn rail_groups(&self) -> (Vec<RailGroup>, String) {
        let groups = site::rail(&self.data, &self.path);
        let mut active = String::new();
        let out = groups
            .iter()
            .map(|group| {
                RailGroup::new(
                    group.title,
                    group.entries.iter().map(|entry| {
                        if entry.active {
                            active = entry.id.clone();
                        }
                        let icon = match entry.id.as_str() {
                            "home" => Icon::House,
                            "subscriptions" => Icon::Users,
                            "library" => Icon::LibraryBig,
                            "history" => Icon::History,
                            "later" => Icon::Clock,
                            "liked" => Icon::ThumbsUp,
                            "playlists" => Icon::ListVideo,
                            "downloads" => Icon::Download,
                            _ => Icon::User,
                        };
                        let mut one = RailEntry::new(&entry.id, &entry.label, icon);
                        if let Some(description) = entry.description {
                            one = one.description(description);
                        }
                        if let Some(count) = entry.count {
                            one = one.count(count as u32);
                        }
                        if let Some(dot) = entry.dot {
                            one = one.dot(format!("kind-{dot}"));
                        }
                        one
                    }),
                )
            })
            .collect();
        (out, active)
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            let busy = self.player.playing
                || !self.data.fetching.is_empty()
                || self.player.ended_at.is_some()
                || self.data.next_due().is_some();
            ctx.request_repaint_after(std::time::Duration::from_millis(if busy { 33 } else { 1000 }));
        }
        if self.path.is_empty() {
            self.go("home");
        }
        let now = self.now();
        self.tick(now);
        self.went();
        self.say(&ctx);
        let (first, ..) = self.place();
        let page = if PAGES.contains(&first.as_str()) {
            first.clone()
        } else {
            "missing".to_owned()
        };
        let collapsed = page == "watch";
        let (groups, mut active) = self.rail_groups();
        let was = active.clone();
        let crumbs = site::trail(&self.data, &self.path);
        let trail: Vec<&str> = crumbs.iter().map(|one| one.label.as_str()).collect();
        let status = site::status(&self.data);

        let mut acts: Vec<Act> = Vec::new();
        let mut account = None;
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut crumb = None;
        let mut submitted = false;
        let mut shell = Shell::new().collapsed(collapsed);
        if self.desktop {
            shell = shell.desktop(sokkel_egui::DesktopWindow::native());
        }
        let mut query = std::mem::take(&mut self.top_query);
        let top_query = &mut query;
        let mut views = std::mem::take(&mut self.views);
        let app = &*self;
        let mut page_acts: Vec<Act> = Vec::new();
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                let rail = Rail::new("Video", &groups, &mut active)
                    .mark(TIDEVANN_MARK)
                    .collapsed(collapsed);
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
                let out = TopBar::new(&trail)
                    .linked(true)
                    .start(|ui| {
                        let field = SearchField::new("top-search", top_query)
                            .placeholder("Search")
                            .keys("/")
                            .width(256.0)
                            .show(ui);
                        submitted = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    })
                    .search("Search or run a command", "\u{2318}K")
                    .show(ui, |_| {});
                searched |= out.search;
                crumb = out.crumb;
            }
            ShellPart::Page => {
                crate::screens::page(ui, app, &page, &mut views, &mut page_acts);
            }
            ShellPart::Foot => {
                let downloaded = status.downloaded.clone();
                ui.add(
                    StatusLine::new(status.note)
                        .live(status.live)
                        .item("watched", status.watched)
                        .item("downloaded", downloaded),
                );
            }
        });
        self.views = views;
        self.top_query = query;
        acts.extend(page_acts);
        if active != was {
            let href = groups
                .iter()
                .flat_map(|group| &group.entries)
                .find(|entry| entry.id == active)
                .map(|entry| entry.id.clone());
            if let Some(id) = href {
                acts.push(Act::Go(match id.strip_prefix("channel-") {
                    Some(channel) => format!("channel/{channel}"),
                    None => id,
                }));
            }
        }
        if let Some(index) = crumb
            && let Some(href) = crumbs.get(index).and_then(|one| one.href.clone())
        {
            acts.push(Act::Go(href));
        }
        if submitted {
            site::submit(&mut self.data, &self.top_query);
        }
        match account {
            Some(AccountAction::Settings) => acts.push(Act::Go("settings".into())),
            Some(AccountAction::Appearance) => acts.push(Act::Go("settings/appearance".into())),
            None => {}
        }
        if searched {
            self.palette = Some(Default::default());
        }
        let overlay = self.palette.is_some()
            || self.settings.is_some()
            || self.asking.is_some()
            || self.saving.is_some()
            || self.sharing.is_some();
        if !overlay {
            acts.extend(self.keys(&ctx));
        }
        for act in acts {
            self.apply(&ctx, act);
        }
        self.went();
        self.full_screen(&ctx);
        self.search_palette(&ctx, &groups);
        self.settings_dialog(&ctx);
        self.dialogs(&ctx);
        self.questions(&ctx);
        self.went();
        Toaster::new().show(&ctx);
    }

    /// The player over the whole window while it fills the screen, or small in the window's
    /// corner while it plays in picture in picture.
    fn full_screen(&mut self, ctx: &egui::Context) {
        if self.watching.is_none() {
            return;
        }
        if self.picture_in_picture && !self.fullscreen {
            let mut acts = Vec::new();
            let rect = ctx.content_rect();
            let width = 384.0;
            let at = rect.right_bottom() - egui::vec2(width + 24.0, width * 9.0 / 16.0 + 24.0 + 46.0);
            egui::Area::new(egui::Id::unique("video-picture-in-picture"))
                .order(egui::Order::Foreground)
                .fixed_pos(at)
                .show(ctx, |ui| {
                    ui.set_width(width);
                    let t = sokkel_egui::tokens(ctx);
                    egui::Frame::new()
                        .shadow(t.shadow("elevation-pop"))
                        .corner_radius(t.radius("surface"))
                        .show(ui, |ui| {
                            ui.set_width(width);
                            crate::watch_page::player(ui, self, None, &mut acts);
                        });
                });
            for act in acts {
                self.apply(ctx, act);
            }
            return;
        }
        if !self.fullscreen {
            return;
        }
        let mut acts = Vec::new();
        let rect = ctx.content_rect();
        egui::Area::new(egui::Id::unique("video-fullscreen"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.set_min_size(rect.size());
                crate::watch_page::player(ui, self, Some(rect.height()), &mut acts);
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            acts.push(Act::Fullscreen(false));
        }
        for act in acts {
            self.apply(ctx, act);
        }
    }

    /// The questions asked before what cannot be undone.
    fn questions(&mut self, ctx: &egui::Context) {
        let Some(asking) = self.asking.clone() else { return };
        let (question, detail, label, act) = match &asking {
            Asking::ClearHistory { clearing } => (
                "Clear watch history?".to_owned(),
                "Every video you watched and where you stopped is forgotten. Your playlists and likes stay.".to_owned(),
                "Clear history",
                Act::ClearHistory(*clearing),
            ),
            Asking::DeletePlaylist(id) => (
                format!(
                    "Delete {}?",
                    self.data
                        .playlists
                        .seen_get(id)
                        .map_or("the playlist", |one| one.name.as_str())
                ),
                "The playlist goes; the videos in it stay on the site.".to_owned(),
                "Delete playlist",
                Act::DeletePlaylist(id.clone()),
            ),
            Asking::Reset => (
                "Reset the demo data?".to_owned(),
                "Everything changed since the demo started is gone, and the page loads again.".to_owned(),
                "Reset",
                Act::ResetDemo,
            ),
        };
        let answer = ConfirmDialog::new(("question", format!("{asking:?}")), &question, label)
            .detail(detail)
            .destructive(true)
            .show(ctx);
        match answer {
            Some(ConfirmAnswer::Confirm) => {
                self.asking = None;
                self.apply(ctx, act);
            }
            Some(_) => {
                self.asking = None;
                if asking == (Asking::ClearHistory { clearing: true }) {
                    self.go("history");
                }
            }
            None => {}
        }
    }

    /// Everything as the demo started, the page loaded again at Home.
    pub fn reset_demo(&mut self) {
        let now = self.now();
        self.data = Data::new(now);
        self.ticked = now;
        self.watching = None;
        self.player = Player::default();
        self.settings = None;
        self.saving = None;
        self.sharing = None;
        self.path.clear();
        self.go("home");
    }
}

impl eframe::App for VideoApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.player.playing
            && let Some(watching) = &mut self.watching
        {
            let state = PlayerState {
                time: self.player.time,
                playing: false,
            };
            watching.playing_changed(&mut self.data, false, state);
        }
        eframe::set_value(storage, "appearance", &self.appearance);
        eframe::set_value(storage, "video", &self.data.kept());
    }
}
