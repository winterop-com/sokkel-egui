//! The records app, as the web's `App.tsx`: the rail of its pages, the server menu in the top bar,
//! the page the address names, and the status line each page fills.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, Icon, MenuItem, Page, Rail, RailEntry, RailGroup, ServerMenu, ServerState,
    Shell, ShellPart, StatusLine, Toast, ToastKind, Toaster, TopBar, install,
};

use crate::backend::{Backend, ME, ReceiptState, SERVER};

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// How long the server takes to answer whether it answers.
const PROBE: i64 = 600;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The place each first part of the route opens, and what the trail calls it.
fn place_of_route(first: &str) -> Option<(&'static str, &'static str)> {
    Some(match first {
        "" => ("overview", "Overview"),
        "forms" => ("forms", "Forms"),
        "responses" => ("responses", "Responses"),
        "people" => ("people", "Households"),
        "places" => ("places", "Places"),
        "playground" => ("playground", "Playground"),
        "server" => ("server", "Server"),
        "health" => ("health", "Metadata health"),
        _ => return None,
    })
}

/// What the status line says for the page: a note at its start and facts at its end.
#[derive(Clone, Debug, Default)]
pub struct Foot {
    pub note: String,
    pub items: Vec<(String, String)>,
}

pub struct RecordsApp {
    pub appearance: Appearance,
    pub backend: Backend,
    /// The address, split at its slashes.
    pub path: Vec<String>,
    /// The settings over the page, and the section they show.
    pub settings: Option<String>,
    /// Whether the next form opens reporting from the place the last one was filed for.
    pub keep_place: bool,
    /// The place the last form was filed for.
    pub last_place: Option<String>,
    /// The form being filled.
    pub fill: Option<crate::form_fill::Fill>,
    /// The places page's column, tree and map.
    pub places: crate::places_page::PlacesView,
    /// What the households' list narrows to.
    pub households: crate::households::HouseholdsView,
    /// What the responses' list holds open.
    pub responses: crate::responses::ResponsesView,
    /// How a periodic report's grid of counts is laid.
    pub grid_view: crate::count_grid::GridView,
    /// When the server was last asked whether it answers, and whether it did.
    probed_at: i64,
    answered: Option<bool>,
    pub foot: Foot,
    /// Each table's sort, widths and the row opened, by the table's name.
    pub tables: std::collections::HashMap<String, sokkel_egui::TableState>,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    /// Whether the clock stands still, as the tests hold it.
    held: bool,
}

impl RecordsApp {
    pub fn new(cc: &eframe::CreationContext<'_>, local_offset: i64) -> Self {
        crate::days::set_local_offset(local_offset);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let appearance = cc
            .storage
            .and_then(|s| eframe::get_value(s, "appearance"))
            .unwrap_or_default();
        let mut app = Self::at(now, appearance);
        app.held = false;
        app.answered = None;
        app
    }

    /// The app at a moment, its clock held there and the server's answer already in, as the tests
    /// and the screenshots take it.
    pub fn at(now: i64, appearance: Appearance) -> Self {
        Self {
            appearance,
            backend: Backend::seeded(now),
            path: Vec::new(),
            settings: None,
            keep_place: true,
            last_place: None,
            fill: None,
            grid_view: Default::default(),
            responses: Default::default(),
            households: Default::default(),
            places: Default::default(),
            probed_at: now - PROBE,
            answered: Some(true),
            foot: Foot::default(),
            tables: Default::default(),
            started: now,
            started_at: None,
            elapsed: 0,
            held: true,
        }
    }

    /// The moment it is now.
    pub fn now(&self) -> i64 {
        self.started + self.elapsed
    }

    /// Whether the server answers, as the top bar says it.
    pub fn server_state(&self) -> ServerState {
        match self.answered {
            None => ServerState::Checking,
            Some(true) => ServerState::Up,
            Some(false) => ServerState::Down,
        }
    }

    /// Asks the server again whether it answers.
    pub fn check_server(&mut self) {
        self.probed_at = self.now();
        self.answered = None;
    }

    fn settle_probe(&mut self) {
        if self.answered.is_none() && self.now() >= self.probed_at + PROBE {
            self.answered = Some(!self.backend.down);
        }
    }

    pub fn go(&mut self, path: &str) {
        if path.trim_matches('/') == "settings" {
            self.settings = Some("appearance".into());
            return;
        }
        self.path = path
            .trim_matches('/')
            .split('/')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
    }

    /// The first part of the address.
    pub fn first(&self) -> &str {
        self.path.first().map_or("", String::as_str)
    }

    /// How many responses wait to be sent on.
    pub fn waiting(&self) -> usize {
        self.backend
            .receipts
            .iter()
            .filter(|r| r.state == ReceiptState::Received)
            .count()
    }

    /// Sends every received response on, saying what came of it in a toast.
    pub fn send_on(&mut self, ctx: &egui::Context) {
        match self.backend.forward_all(self.now()) {
            Ok((0, 0)) => {
                Toast::new("Nothing waits to be sent on").show(ctx);
            }
            Ok((taken, refused)) => {
                Toast::success(format!("{} sent on to the register", taken + refused))
                    .description(if refused == 0 {
                        "The register took them all.".to_owned()
                    } else {
                        format!("{refused} refused; each receipt says why.")
                    })
                    .show(ctx);
            }
            Err(failure) => {
                Toast::new("Nothing was sent on")
                    .kind(ToastKind::Error)
                    .description(failure.to_string())
                    .show(ctx);
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        if !self.held {
            let time = ui.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
        self.settle_probe();
        let first = self.first().to_owned();
        let known = place_of_route(&first);
        let (place, title) = known.unwrap_or(("overview", "Not found"));
        let waiting = self.waiting();
        let entry = |id: &str, label: &str, icon: Icon, description: &str| {
            RailEntry::new(id, label, icon).description(description)
        };
        let mut responses = entry("responses", "Responses", Icon::Inbox, "What was filed");
        if waiting > 0 {
            responses = responses.count(waiting as u32);
        }
        let groups = vec![
            RailGroup::new(
                None,
                [
                    entry("overview", "Overview", Icon::LayoutDashboard, "What waits for review"),
                    entry("forms", "Forms", Icon::ClipboardList, "Start a record"),
                    responses,
                    entry("people", "Households", Icon::Users, "Who the survey follows"),
                    entry("places", "Places", Icon::MapPin, "Regions and sites"),
                ],
            ),
            RailGroup::new(
                Some("Server"),
                [
                    entry("playground", "Playground", Icon::Braces, "Try the API"),
                    entry("server", "Server", Icon::ServerCog, "What the server offers"),
                    entry(
                        "health",
                        "Metadata health",
                        Icon::HeartPulse,
                        "Names, codes and translations",
                    ),
                ],
            ),
        ];
        let mut chosen = place.to_owned();
        let was = chosen.clone();
        let trail = ["juniper", title];
        let state = self.server_state();
        let facts = [
            SERVER.address,
            "Serves forms, places, households and responses",
            "Keeps every response it receives",
        ];
        let server_items = [
            MenuItem::action("What this server offers").icon(Icon::ServerCog),
            MenuItem::action("Demo server settings").icon(Icon::Settings),
        ];
        let mut server = sokkel_egui::ServerMenuResponse::default();
        let mut account = None;
        // the page fills the foot while it is drawn; what it said last frame is drawn this one
        let foot = std::mem::take(&mut self.foot);
        let mut searched = sokkel_egui::palette_shortcut(ui.ctx());
        Shell::new().show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Records", &groups, &mut chosen).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new(ME)
                    .place(SERVER.address)
                    .local(true)
                    .scheme(self.appearance.scheme)
                    .show(ui);
            }
            ShellPart::Top => {
                searched |= TopBar::new(&trail)
                    .code(0)
                    .search("Search", "⌘K")
                    .show(ui, |ui| {
                        server = ServerMenu::new(SERVER.name, state)
                            .version((state == ServerState::Up).then_some(SERVER.version))
                            .facts(&facts)
                            .check(true)
                            .items(&server_items)
                            .show(ui);
                    })
                    .search;
            }
            ShellPart::Page => match (known.is_some(), first.as_str()) {
                (true, "") => Page::scroll(ui, |ui| self.overview(ui)),
                (true, "forms") if self.path.len() == 1 => Page::scroll(ui, |ui| self.forms_page(ui)),
                (true, "responses") if self.path.get(1).is_none_or(|p| p == "in") => {
                    let state = self.path.get(2).cloned();
                    Page::show(ui, |ui| self.responses_page(ui, state.as_deref()));
                }
                (true, "places") => {
                    let given = (self.path.len() > 1).then(|| self.path[1..].join("/"));
                    self.places_page(ui, given.as_deref());
                }
                (true, "people") if self.path.len() == 1 => Page::show(ui, |ui| self.households_page(ui)),
                (true, "people") => {
                    let id = self.path[1].clone();
                    Page::scroll(ui, |ui| self.household_page(ui, &id));
                }
                (true, "responses") => {
                    let code = self.path[1].clone();
                    Page::scroll(ui, |ui| self.receipt_page(ui, &code));
                }
                (true, "forms") if self.path.get(2).is_some_and(|p| p == "layout") => {
                    let form = self.path[1].clone();
                    Page::scroll(ui, |ui| self.form_layout(ui, &form));
                }
                (true, "forms") if self.path.get(2).is_none_or(|p| p == "for") => {
                    let form = self.path[1].clone();
                    let household = (self.path.get(2).map(String::as_str) == Some("for"))
                        .then(|| self.path.get(3).cloned())
                        .flatten();
                    self.form_fill(ui, &form, household.as_deref());
                }
                _ => Page::scroll(ui, |ui| {
                    sokkel_egui::Empty::new("Not found")
                        .description("No page has this address.")
                        .show(ui, |_| {});
                }),
            },
            ShellPart::Foot => {
                let mut line = StatusLine::new(&foot.note);
                for (label, value) in &foot.items {
                    line = line.item(label, value);
                }
                if waiting > 0 && place != "responses" {
                    line = line.item("received", waiting.to_string());
                }
                ui.add(line);
            }
        });
        let _ = searched;
        if server.check {
            self.check_server();
        }
        match server.item {
            Some(0) => self.go("server"),
            Some(1) => self.settings = Some("demo".into()),
            _ => {}
        }
        if chosen != was {
            self.go(if chosen == "overview" { "" } else { &chosen });
        }
        match account {
            Some(AccountAction::Scheme(scheme)) => self.appearance.scheme = scheme,
            Some(AccountAction::Settings) => self.settings = Some("appearance".into()),
            None => {}
        }
        Toaster::new().show(ui.ctx());
    }
}

impl eframe::App for RecordsApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "appearance", &self.appearance);
    }
}
