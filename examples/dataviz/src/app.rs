//! The visualizer inside the shell, as the web's `App.tsx` and `Visualizer.tsx`: the rail with its
//! places, their counts and the boards, the top bar's trail and search, the page the address names,
//! the status line's counts, and the settings, the palette and the dialogs over them. The clock is
//! the app's: the backend's answers come as it moves on.

use std::collections::HashMap;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, Rail,
    RailEntry, RailGroup, Shell, ShellPart, StatusLine, StatusTone, TableState, Toast, Toaster, TopBar,
    appearance_commands, install, place_commands, place_of, run_appearance_command, toast_acted,
};
use sokkel_egui_maps::MapView;

use crate::act::{Act, Asking};
use crate::backend::{Backend, ToastAction};
use crate::import_dialog::ImportDialog;
use crate::model::{Board, Dataset, Preferences, SavedChart, ViewKind};
use crate::pages::board::BoardPage;
use crate::pages::boards::BoardsPage;
use crate::pages::builder::Builder;
use crate::themes::CARRIED;
use crate::visualizer::{Frame, status};

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// A view's icon, as the web's `VIEW_ICONS`.
pub fn view_icon(view: ViewKind) -> Icon {
    match view {
        ViewKind::Bar => Icon::ChartColumn,
        ViewKind::Line => Icon::ChartLine,
        ViewKind::Area => Icon::ChartArea,
        ViewKind::Scatter => Icon::ChartScatter,
        ViewKind::Pie => Icon::ChartPie,
        ViewKind::Heatmap => Icon::Grid3x3,
        ViewKind::Map => Icon::Map,
        ViewKind::Table => Icon::Table2,
        ViewKind::Stat => Icon::SquareSigma,
    }
}

/// The dataset page's own state, as the web's `DatasetPage` keeps it.
#[derive(Default)]
pub struct DatasetView {
    pub id: String,
    pub tab: usize,
    pub filters: Vec<sokkel_egui::Filter>,
    pub query: String,
    pub rows: TableState,
    pub columns: TableState,
    pub charts: TableState,
}

/// What a page holds while it is shown, as the web's pages keep in their own state; each starts
/// afresh on another page, as the web's keys do.
#[derive(Default)]
pub struct Views {
    pub datasets_query: String,
    pub datasets_table: Option<TableState>,
    pub dataset: Option<DatasetView>,
    /// The builder, under the route it was made for.
    pub builder: Option<(String, Builder)>,
    pub charts_query: String,
    pub charts_table: Option<TableState>,
    pub boards: BoardsPage,
    pub boards_table: TableState,
    pub board: Option<BoardPage>,
    /// Each map's view, by where it is drawn.
    pub maps: HashMap<String, MapView>,
}

/// A dialog open over the page.
#[derive(Clone, Debug)]
pub enum Dialog {
    Import(Box<ImportDialog>),
    /// Making a board (starting with a chart), or changing one's details: its name and description.
    Board {
        editing: Option<Box<Board>>,
        adding: Option<Box<SavedChart>>,
        name: String,
        description: String,
        then_open: bool,
    },
    /// Adding a saved chart to a board: the one picked and its width.
    AddChart {
        board: String,
        picked: Option<String>,
        span: usize,
    },
    /// Sharing a chart: its name, its spec and its dataset, and the link made.
    Share {
        name: String,
        link: String,
        size: String,
        dataset: Box<Dataset>,
    },
}

pub struct VisualizerApp {
    pub appearance: Appearance,
    pub backend: Backend,
    /// The address, split at its slashes.
    pub route: Vec<String>,
    /// The settings over the page, and the section they show.
    pub settings: Option<String>,
    /// The palette while it is open: what is typed and the row lit.
    pub palette: Option<(String, usize)>,
    pub dialog: Option<Dialog>,
    /// The question asked before something is done.
    pub asking: Option<Asking>,
    pub views: Views,
    toast_actions: HashMap<u64, ToastAction>,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    held: bool,
    pub desktop: bool,
}

/// What eframe's storage keeps between visits: the rows of each collection.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Kept {
    pub datasets: Vec<Dataset>,
    pub charts: Vec<SavedChart>,
    pub boards: Vec<Board>,
    pub preferences: Vec<Preferences>,
}

impl VisualizerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let now = crate::js_now();
        let appearance = cc
            .storage
            .and_then(|s| eframe::get_value(s, "appearance"))
            .unwrap_or_default();
        let mut app = Self::at(now, appearance);
        if let Some(kept) = cc.storage.and_then(|s| eframe::get_value::<Kept>(s, "dataviz")) {
            app.backend
                .restore(kept.datasets, kept.charts, kept.boards, kept.preferences);
        }
        app.held = false;
        app.desktop = true;
        app
    }

    /// The visualizer at a moment with the data as it is seeded then, its clock held there, as the
    /// tests and the screenshots take it.
    pub fn at(now: i64, appearance: Appearance) -> Self {
        Self {
            appearance,
            backend: Backend::new(now),
            route: Vec::new(),
            settings: None,
            palette: None,
            dialog: None,
            asking: None,
            views: Views::default(),
            toast_actions: HashMap::new(),
            started: now,
            started_at: None,
            elapsed: 0,
            held: true,
            desktop: false,
        }
    }

    pub fn now(&self) -> i64 {
        self.started + self.elapsed
    }

    /// Moves a held clock on, as the tests wait for the backend's answers.
    pub fn wait(&mut self, ms: i64) {
        self.elapsed += ms;
        let now = self.now();
        self.backend.advance(now);
    }

    /// Goes to an address, as the web's hash routes: `datasets[/<id>]`, `build[/<id>]`,
    /// `charts[/<id>]`, `boards[/<id>[/present]]`, `shared/<payload>`; `settings[/<section>]` opens
    /// over the page, which stays (the datasets when there is none).
    pub fn go(&mut self, path: &str) {
        let path = path
            .trim_start_matches("replace ")
            .trim_start_matches('#')
            .trim_matches('/');
        if path == "reload" {
            self.reload();
            return;
        }
        if let Some(rest) = path.strip_prefix("settings") {
            let section = rest.trim_start_matches('/');
            self.settings = Some(if section.is_empty() { "charts" } else { section }.to_owned());
            if self.route.is_empty() {
                self.route = vec!["datasets".to_owned()];
            }
            return;
        }
        let next: Vec<String> = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
        let next = if next.is_empty() {
            vec!["datasets".to_owned()]
        } else {
            next
        };
        if next == self.route {
            return;
        }
        if next.first() != self.route.first() || next.get(1) != self.route.get(1) {
            self.views = Views {
                maps: std::mem::take(&mut self.views.maps),
                boards_table: std::mem::take(&mut self.views.boards_table),
                ..Views::default()
            };
        }
        self.route = next;
    }

    /// The page loaded again, after the demo data is reset: the datasets, every page's state new.
    fn reload(&mut self) {
        self.backend = Backend::new(self.now());
        self.views = Views::default();
        self.settings = None;
        self.dialog = None;
        self.asking = None;
        self.route = vec!["datasets".to_owned()];
    }

    /// The places the backend went to since the last frame.
    fn went(&mut self) {
        for path in std::mem::take(&mut self.backend.went) {
            self.go(&path);
        }
    }

    /// The notes the backend said since the last frame, as toasts, each with its button.
    fn say(&mut self, ctx: &egui::Context) {
        for (toast, action) in std::mem::take(&mut self.backend.toasts) {
            let mut note = if toast.kind == Some("error") {
                Toast::error(toast.title)
            } else {
                Toast::new(toast.title)
            }
            .timeout(Some(4.0));
            if let Some(description) = toast.description {
                note = note.description(description);
            }
            if let Some(words) = &toast.action {
                note = note.action(words);
            }
            let id = note.show(ctx);
            if let Some(action) = action {
                self.toast_actions.insert(id, action);
            }
        }
        for id in toast_acted(ctx) {
            if let Some(action) = self.toast_actions.remove(&id) {
                self.backend.press(&action);
            }
        }
    }

    /// The rail's groups as the web's, and the entry active.
    pub fn rail_groups(&self) -> (Vec<RailGroup>, String) {
        let frame = Frame::of(&self.backend, &self.route);
        let mut active = String::new();
        let groups = frame
            .rail()
            .into_iter()
            .map(|group| {
                let entries: Vec<RailEntry> = group
                    .entries
                    .iter()
                    .map(|entry| {
                        if entry.active {
                            active = entry.id.clone();
                        }
                        let (icon, description) = match entry.id.as_str() {
                            "datasets" => (Icon::Database, Some("Tables of data to chart")),
                            "charts" => (Icon::ChartColumn, Some("Saved charts")),
                            "boards" => (Icon::LayoutGrid, Some("Charts arranged together")),
                            "settings" => (Icon::Settings2, None),
                            _ => (Icon::LayoutDashboard, None),
                        };
                        let mut one = RailEntry::new(&entry.id, &entry.label, icon);
                        if let Some(description) = description {
                            one = one.description(description);
                        }
                        if let Some(count) = entry.count {
                            one = one.count(count as u32);
                        }
                        one
                    })
                    .collect();
                RailGroup::new(group.title, entries)
            })
            .collect();
        (groups, active)
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            let busy = self.backend.next_due().is_some();
            ctx.request_repaint_after(std::time::Duration::from_millis(if busy { 33 } else { 1000 }));
        }
        if self.route.is_empty() {
            self.go("datasets");
        }
        let now = self.now();
        self.backend.advance(now);
        self.went();
        self.say(&ctx);
        self.follow_pages();

        let (groups, mut active) = self.rail_groups();
        let was = active.clone();
        let frame = Frame::of(&self.backend, &self.route);
        let crumbs = frame.trail();
        let trail: Vec<&str> = crumbs.iter().map(|one| one.label.as_str()).collect();
        let (note, tone, live, counts) = status(&self.backend);

        let mut acts: Vec<Act> = Vec::new();
        let mut account = None;
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut crumb = None;
        let mut shell = Shell::new();
        if self.desktop {
            shell = shell.desktop(sokkel_egui::DesktopWindow::native());
        }
        let mut views = std::mem::take(&mut self.views);
        let app = &*self;
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Visualizer", &groups, &mut active).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada")
                    .place("tidevann.example.org")
                    .appearance(true)
                    .show(ui);
            }
            ShellPart::Top => {
                let out = TopBar::new(&trail)
                    .linked(true)
                    .search("Search", "\u{2318}K")
                    .show(ui, |_| {});
                searched |= out.search;
                crumb = out.crumb;
            }
            ShellPart::Page => crate::screens::page(ui, app, &mut views, &mut acts),
            ShellPart::Foot => {
                let mut line = StatusLine::new(note.clone()).live(live);
                if tone == "error" {
                    line = line.tone(StatusTone::Error);
                }
                ui.add(
                    line.item("datasets", counts[0])
                        .item("charts", counts[1])
                        .item("boards", counts[2]),
                );
            }
        });
        self.views = views;
        if active != was {
            let href = groups
                .iter()
                .flat_map(|group| &group.entries)
                .find(|entry| entry.id == active)
                .map(|entry| entry.id.clone());
            if let Some(id) = href {
                acts.push(Act::Go(match id.strip_prefix("board-") {
                    Some(board) => format!("boards/{board}"),
                    None => id,
                }));
            }
        }
        if let Some(index) = crumb
            && let Some(href) = crumbs.get(index).and_then(|one| one.href.clone())
        {
            acts.push(Act::Go(href));
        }
        match account {
            Some(AccountAction::Settings) => acts.push(Act::Go("settings".into())),
            Some(AccountAction::Appearance) => acts.push(Act::Go("settings/appearance".into())),
            None => {}
        }
        if searched {
            self.palette = Some(Default::default());
        }
        for act in acts {
            self.apply(act);
        }
        self.went();
        self.search_palette(&ctx, &groups);
        self.settings_dialog(&ctx);
        self.dialogs(&ctx);
        self.questions(&ctx);
        self.went();
        Toaster::new().show(&ctx);
    }

    /// Each page's own state made for the route it shows, as the web's keys make a page anew.
    fn follow_pages(&mut self) {
        let frame = Frame::of(&self.backend, &self.route);
        let prefs = self.backend.prefs();
        let (section, id) = (
            self.route.first().map(String::as_str),
            self.route.get(1).map(String::as_str),
        );
        if frame.loading {
            return;
        }
        // the builder, for a new chart or a saved one
        let key = self.route.join("/");
        match (section, id) {
            (Some("build"), _) | (Some("charts"), Some(_)) => {
                let chart = if section == Some("charts") {
                    frame.charts.iter().find(|one| Some(one.id.as_str()) == id)
                } else {
                    None
                };
                if section == Some("build") || chart.is_some() {
                    let fresh = self.views.builder.as_ref().is_none_or(|(at, _)| *at != key);
                    if fresh {
                        let builder =
                            Builder::new(frame.datasets, &prefs, chart, if chart.is_none() { id } else { None });
                        self.views.builder = Some((key.clone(), builder));
                    } else if let Some((_, builder)) = &mut self.views.builder {
                        builder.refresh(frame.charts);
                    }
                }
            }
            (Some("boards"), Some(id)) => {
                let fresh = self.views.board.as_ref().is_none_or(|one| one.board != id);
                if fresh {
                    let board = frame.boards.iter().find(|one| one.id == id);
                    let mut page = BoardPage::new(board, frame.charts);
                    page.board = id.to_owned();
                    self.views.board = Some(page);
                }
            }
            (Some("datasets"), Some(id)) if self.views.dataset.as_ref().is_none_or(|one| one.id != id) => {
                self.views.dataset = Some(DatasetView {
                    id: id.to_owned(),
                    ..DatasetView::default()
                });
            }
            _ => {}
        }
    }

    /// The palette while it is open: the rail's places, then the actions, the datasets, the
    /// charts and the boards, then the look.
    fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let frame = Frame::of(&self.backend, &self.route);
        let mut groups: Vec<CommandGroup> = Vec::new();
        // the page's own commands first, as the web's `useCommands` ranks them
        groups.extend(crate::screens::page_commands(self));
        groups.extend(place_commands(rail, "Go to"));
        for group in frame.commands() {
            groups.push(CommandGroup::new(
                group.title,
                group.commands.iter().map(|command| {
                    let icon = match group.title {
                        "Actions" if command.id == "import" => Icon::Upload,
                        "Actions" => Icon::Plus,
                        "Datasets" => Icon::Database,
                        "Charts" => frame
                            .charts
                            .iter()
                            .find(|one| format!("chart-{}", one.id) == command.id)
                            .map_or(Icon::ChartColumn, |one| view_icon(one.spec.view)),
                        _ if command.id.starts_with("board-present-") => Icon::Presentation,
                        _ => Icon::LayoutDashboard,
                    };
                    let mut item = MenuItem::action(&command.label)
                        .icon(icon)
                        .keywords(command.keywords.iter().cloned())
                        .id(format!("do:{}", command.does));
                    if let Some(hint) = &command.hint {
                        item = item.hint(hint);
                    }
                    item
                }),
            ));
        }
        let themes: Vec<_> = sokkel_egui::themes()
            .filter(|theme| CARRIED.contains(&theme.name.as_str()))
            .collect();
        groups.extend(appearance_commands(
            &self.appearance,
            &themes,
            &AppearanceCommandWords::default(),
        ));
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight).show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        if let Some(command) = command.as_deref() {
            if let Some(does) = command.strip_prefix("do:") {
                match does {
                    "import" => self.apply(Act::Import),
                    "board-new" => self.apply(Act::NewBoard(None)),
                    path => self.apply(Act::Go(path.to_owned())),
                }
            } else if let Some(page) = command.strip_prefix("page:") {
                crate::screens::run_page_command(self, page);
            } else if let Some(place) = place_of(command) {
                self.apply(Act::Go(match place.strip_prefix("board-") {
                    Some(board) => format!("boards/{board}"),
                    None => place.to_owned(),
                }));
            } else {
                run_appearance_command(&mut self.appearance, command);
            }
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }

    /// Everything as the demo started, the page loaded again at the datasets.
    pub fn reset_demo(&mut self) {
        self.backend.reset_all();
    }
}

impl eframe::App for VisualizerApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "appearance", &self.appearance);
        let kept = Kept {
            datasets: self.backend.datasets.rows.clone(),
            charts: self.backend.charts.rows.clone(),
            boards: self.backend.boards.rows.clone(),
            preferences: self.backend.preferences.rows.clone(),
        };
        eframe::set_value(storage, "dataviz", &kept);
    }
}
