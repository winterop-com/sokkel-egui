//! sokkel-ui's status example ported one to one (`examples/status/web/src` in sokkel-ui): a live
//! board of Tidevann's services, regions and incidents, on call, and the public status page. Each
//! page grows here as the parts it needs land in the library; `docs/parity/status.md` says where it
//! stands.

pub mod data;
pub mod dialogs;
pub mod history;
pub mod live;
pub mod new_incident;
pub mod settings;
pub mod watch;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, AppBar, Appearance, AppearanceCommandWords, AsideWidth, Banner, BannerTone, Button,
    ButtonSize, ButtonVariant, Cell, ChipBar, ChipTone, Choice, ChoiceList, Column, Columns, CommandGroup,
    CommandPalette, DashboardArrange, DashboardEdit, DashboardEditActions, DashboardGrid, DashboardLayoutItem,
    DashboardPanel, DashboardWidget, DataTable, Day, DescriptionList, DesktopWindow, Editor, Gap, Icon, IconButton,
    KpiRow, ListRow, ListRows, MenuItem, MetaLine, Page, PageHeader, PanelSpan, PersonRow, Rail, RailEntry, RailGroup,
    RowAside, RowMark, Section, Segmented, Select, Shell, ShellPart, Split, SplitPane, Stack, StackLayout, StackPlace,
    Stat, StatusLine, StatusTone, TableState, TableVariant, TextRole, Timeline, TimelineEvent, TopBar, UptimeBar,
    UptimeDay, appearance_commands, checkbox_list, form_bar, format_uptime, install, overall_uptime, palette_shortcut,
    place_commands, place_of, relative_time, run_appearance_command,
};

use data::SERVICES;
use live::Live;
use sokkel_egui_charts::{Series, TimeSeriesPanel};
use sokkel_egui_graph::{GraphCanvas, GraphLink, GraphNode, GraphWords};
use sokkel_egui_maps::{MapLegend, MapPopup, MapPopupRow, MapView, Marker};

/// Tidevann's mark, as sokkel-ui's examples draw it (`tidevann-shared/mark`).
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// Who is signed in to the board.
const ME: &str = "ada";

/// How often a reading comes, in seconds.
/// The interval the board starts reading at, in seconds.
const INTERVAL: u32 = 2;

/// Which changes of state a notice tells, as the web's `notify`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notify {
    All,
    Outages,
    Off,
}

/// How often the simulation makes something go wrong, as the web's `Weather`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    Calm,
    Normal,
    Stormy,
}

/// What the board's settings set, as the web's `Settings` (`live` is `StatusApp::live`).
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// How often the board takes a reading, in seconds: 1, 2 or 5.
    pub interval: u32,
    /// Times of day on a 12-hour clock.
    pub twelve_hour: bool,
    pub notify: Notify,
    pub weather: Weather,
    /// Whether a service going wrong opens an incident by itself.
    pub auto_incidents: bool,
    /// Whether the on-call person's updates move an incident the board opened along.
    pub responder: bool,
    /// The status page's title.
    pub page_title: String,
    /// Whether the status page shows each service's last ninety days.
    pub show_uptime: bool,
    /// How many days of past incidents the status page lists: 7 or 14.
    pub history_days: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            interval: INTERVAL,
            twelve_hour: false,
            notify: Notify::All,
            weather: Weather::Normal,
            auto_incidents: true,
            responder: true,
            page_title: "Tidevann status".into(),
            show_uptime: true,
            history_days: 7,
        }
    }
}

/// The board's places, as the web's `PLACES`.
const PLACES: [(&str, &str); 5] = [
    ("overview", "Overview"),
    ("incidents", "Incidents"),
    ("services", "Services"),
    ("regions", "Regions"),
    ("on-call", "On call"),
];

/// The status board: its place, a service or incident opened in it, the live clock and the look.
pub struct StatusApp {
    /// The frame stands in a desktop window with its title bar hidden: set by the running app, never
    /// in its screenshots, which draw the frame as the browser does.
    pub desktop: bool,
    pub place: String,
    /// The service a service page shows.
    pub service: Option<String>,
    /// The day pressed on the service's uptime bar, its incidents listed under it.
    pub service_day: Option<Day>,
    /// Settings open over the page.
    pub settings: bool,
    /// The settings dialog's section, the Demo's choices and its confirm.
    pub settings_state: settings::SettingsState,
    /// Notices waiting to be shown, and where each shown one's Open goes.
    pub notices: Vec<watch::Notice>,
    pub notice_routes: Vec<(u64, String)>,
    /// The scenarios an incident was opened for, so each opens one.
    pub opened_for: Vec<String>,
    pub appearance: Appearance,
    /// The command palette's query and highlighted row, while it is open.
    pub palette: Option<(String, usize)>,
    /// Readings live, or paused.
    pub live: bool,
    /// The board's clock: seconds since midnight of the first reading, and the readings since.
    pub start: u32,
    pub readings: u32,
    /// Seconds of the board's clock since the first reading.
    pub elapsed: u32,
    /// What the settings set: how the board reads and tells, the simulation and the status page.
    pub options: Options,
    /// Who answers now.
    pub on_call: String,
    /// Every incident, open ones first.
    pub incidents: Vec<data::Incident>,
    /// Which incidents the incidents page shows: open, resolved or all.
    pub shown: usize,
    /// The incidents' table, and what its search holds.
    pub incidents_table: TableState,
    pub incident_search: String,
    /// The services in the person's own order, by their index in `SERVICES`, and their table.
    pub service_order: Vec<usize>,
    /// The region the Regions page frames, or "All".
    pub region: String,
    /// The site chosen on the map or in the table.
    pub selected_site: Option<usize>,
    pub sites_table: TableState,
    pub sites_map: MapView,
    /// The overview's map of the sites, every site framed.
    pub overview_map: MapView,
    pub open_table: TableState,
    pub services_table: TableState,
    /// The board's day, the rotation, the handovers and the rotation's table.
    pub today: Day,
    pub shifts: Vec<data::Shift>,
    pub handovers: Vec<data::Handover>,
    /// The hand over dialog, while it is open.
    pub hand_over: Option<dialogs::HandOverForm>,
    /// What the person on call now was told at the handover.
    pub on_call_note: String,
    pub maintenance: Vec<data::Maintenance>,
    /// Every service's deploys, the newest last.
    pub deploys: Vec<data::Deploy>,
    /// The service page's tables of deploys and windows.
    pub deploys_table: TableState,
    pub windows_table: TableState,
    /// A service's dialogs, while one is open.
    pub maintenance_form: Option<dialogs::MaintenanceForm>,
    pub deploy_form: Option<dialogs::DeployForm>,
    /// When the last reading came, on the window's clock, to take the next on time.
    last_tick: Option<f64>,
    /// The incident an incident's page shows.
    pub incident: Option<String>,
    /// Its page's next update: the state after it, and what it says.
    pub post_state: usize,
    pub post_message: String,
    /// The new incident dialog, while it is open.
    pub new_incident: Option<new_incident::NewIncident>,
    /// The public page's form, and the addresses it has subscribed.
    pub subscribe: Subscribe,
    pub rotation_table: TableState,
    /// The overview's panels as kept: their order and widths.
    pub overview_layout: Vec<DashboardLayoutItem>,
    /// The overview's edit mode, its draft while editing.
    pub overview_edit: DashboardEdit,
    /// The live world the readings come from.
    pub live_world: Live,
}

impl StatusApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::on(Appearance::default());
        // the street map only in the running app: tests draw the map without the network
        sokkel_egui_maps::install(cc.wgpu_render_state.as_ref());
        app.overview_map = std::mem::replace(&mut app.overview_map, MapView::new(0.0, 0.0, 1.0)).with_tiles(
            &cc.egui_ctx,
            "tidevann-status/0.1 (an example app of a component library)",
            std::env::temp_dir().join("sokkel-egui-tiles").into(),
        );
        app.sites_map = std::mem::replace(&mut app.sites_map, MapView::new(0.0, 0.0, 1.0)).with_tiles(
            &cc.egui_ctx,
            "tidevann-status/0.1 (an example app of a component library)",
            std::env::temp_dir().join("sokkel-egui-tiles").into(),
        );
        app.desktop = true;
        app
    }

    /// The board as the demo starts: 09:00, the seeded incidents, grace on call.
    pub fn on(appearance: Appearance) -> Self {
        // the demo's day, as the web's shots are taken on it
        let today = Day::parse("2026-11-16").expect("a day");
        let start_ms = 9.0 * 3600.0 * 1000.0;
        Self {
            desktop: false,
            place: "overview".into(),
            service: None,
            service_day: None,
            settings: false,
            appearance,
            palette: None,
            live: true,
            start: 9 * 3600,
            readings: 0,
            elapsed: 0,
            options: Options::default(),
            on_call: "grace".into(),
            incidents: data::seed_incidents(9.0 * 3600.0 * 1000.0),
            shown: 0,
            // newest first, as the web's `defaultSort` on Opened
            incidents_table: TableState::sorted(6, false),
            incident_search: String::new(),
            service_order: data::board_order(),
            services_table: TableState::default(),
            today,
            shifts: data::seed_shifts(today),
            handovers: data::seed_handovers(9.0 * 3600.0 * 1000.0),
            hand_over: None,
            on_call_note: data::ON_CALL_NOTE.into(),
            maintenance: data::seed_maintenance(),
            subscribe: Subscribe {
                // the web's seed: two addresses, a month and eleven days before
                subscribers: vec![
                    Subscriber {
                        email: "ops@example.org".into(),
                        services: Vec::new(),
                        at: start_ms - 30.0 * 86_400_000.0,
                    },
                    Subscriber {
                        email: "harbor-team@example.org".into(),
                        services: vec!["api", "web"],
                        at: start_ms - 11.0 * 86_400_000.0,
                    },
                ],
                ..Subscribe::default()
            },
            settings_state: settings::SettingsState::default(),
            notices: Vec::new(),
            notice_routes: Vec::new(),
            opened_for: Vec::new(),
            new_incident: None,
            deploys: data::seed_deploys(9.0 * 3600.0 * 1000.0),
            maintenance_form: None,
            deploy_form: None,
            deploys_table: TableState::default(),
            windows_table: TableState::default(),
            last_tick: None,
            incident: None,
            post_state: 0,
            post_message: String::new(),
            rotation_table: TableState::default(),
            overview_layout: default_layout(),
            overview_edit: DashboardEdit::default(),
            live_world: Live::new(9.0 * 3600.0 * 1000.0, INTERVAL as f64 * 1000.0),
            region: "All".into(),
            selected_site: None,
            sites_table: TableState::default(),
            overview_map: {
                let mut map = MapView::new(30.0, 10.0, 1.0).framed(false);
                map.frame(Some(bounds_of(&data::SITES.iter().collect::<Vec<_>>())));
                map
            },
            open_table: TableState::default(),
            sites_map: {
                let mut map = MapView::new(30.0, 10.0, 1.0).framed(false);
                map.frame(Some(bounds_of(&data::SITES.iter().collect::<Vec<_>>())));
                map
            },
        }
    }

    /// Go to a route as the web's hash routes name it: `overview`, `services/api`, `settings`.
    pub fn go(&mut self, route: &str) {
        let mut parts = route.split('/');
        match parts.next().unwrap_or("overview") {
            "settings" => self.settings = true,
            // what customers see: a page of its own, outside the board's frame
            "public" => self.place = "public".into(),
            place if PLACES.iter().any(|(id, _)| *id == place) => {
                self.place = place.to_owned();
                let second = parts.next();
                self.service = second
                    .filter(|id| place == "services" && SERVICES.iter().any(|s| s.id == *id))
                    .map(str::to_owned);
                self.incident = second
                    .filter(|id| place == "incidents" && self.incidents.iter().any(|i| i.id == *id))
                    .map(str::to_owned);
                if let Some(incident) = self
                    .incident
                    .as_deref()
                    .and_then(|id| self.incidents.iter().find(|i| i.id == id))
                {
                    // the form starts on the state after the one the incident is in
                    self.post_state = (state_index(incident.state) + 1).min(3);
                    self.post_message.clear();
                }
                self.service_day = parts.next().and_then(Day::parse);
            }
            _ => self.place = "overview".into(),
        }
    }

    /// Take the next reading, the interval the settings give on.
    pub fn tick(&mut self) {
        self.readings += 1;
        self.elapsed += self.options.interval;
        let at = (self.start + self.elapsed) as f64 * 1000.0;
        let known = live::Known {
            deploying: SERVICES
                .iter()
                .map(|s| {
                    self.deploys
                        .iter()
                        .any(|d| d.service == s.id && d.state == data::DeployState::Running)
                })
                .collect(),
            maintenance: SERVICES
                .iter()
                .map(|s| self.maintenance.iter().any(|w| w.service == s.id && w.going_on(at)))
                .collect(),
        };
        let (health, site_health) = (self.live_world.health.clone(), self.live_world.site_health.clone());
        self.weather(at);
        self.live_world.tick(at, &known);
        // a deploy runs 8 s of the board's clock, then succeeds, as the web's `DEPLOY_TIME`
        for deploy in &mut self.deploys {
            if deploy.state == data::DeployState::Running && at - deploy.at >= 8_000.0 {
                deploy.state = data::DeployState::Succeeded;
            }
        }
        self.watch(at, &health, &site_health);
    }

    /// The version a service runs: its last deploy that succeeded, or the board's own.
    pub fn running(&self, service: &str) -> String {
        self.deploys
            .iter()
            .filter(|d| d.service == service && d.state == data::DeployState::Succeeded)
            .max_by(|a, b| a.at.total_cmp(&b.at))
            .map_or_else(
                || {
                    SERVICES
                        .iter()
                        .find(|s| s.id == service)
                        .map_or("", |s| s.version)
                        .to_owned()
                },
                |d| d.version.clone(),
            )
    }

    /// The last reading's time, `HH:MM:SS`.
    pub fn clock(&self) -> String {
        time_of_day(self.start + self.elapsed, self.options.twelve_hour, true)
    }

    /// Every service's last ninety days, in `SERVICES`' order.
    pub fn uptime(&self) -> Vec<Vec<UptimeDay>> {
        let now = ((self.start + self.elapsed) as f64) * 1000.0;
        SERVICES
            .iter()
            .map(|s| history::uptime_days(s.id, &self.incidents, self.today, now, 90))
            .collect()
    }

    /// The incidents not yet resolved.
    pub fn open(&self) -> Vec<&data::Incident> {
        self.incidents
            .iter()
            .filter(|i| i.state != data::State::Resolved)
            .collect()
    }

    fn groups(&self) -> Vec<RailGroup> {
        let open = self.open().len() as u32;
        let mut incidents = RailEntry::new("incidents", "Incidents", Icon::Siren).description("Open and past");
        if open > 0 {
            incidents = incidents.count(open);
        }
        vec![
            RailGroup::new(
                None,
                [
                    RailEntry::new("overview", "Overview", Icon::LayoutDashboard).description("Everything at a glance"),
                    incidents,
                    RailEntry::new("services", "Services", Icon::Boxes).description("Latency and deploys"),
                    RailEntry::new("regions", "Regions", Icon::Globe).description("Sites on a map"),
                    RailEntry::new("on-call", "On call", Icon::PhoneCall).description(format!("{} now", self.on_call)),
                ],
            ),
            RailGroup::new(
                Some("Customers"),
                [RailEntry::new("public", "Status page", Icon::SquareArrowOutUpRight)
                    .description("What customers see")],
            ),
            RailGroup::new(
                None,
                [RailEntry::new("settings", "Settings", Icon::Settings).description("Readings, notices, demo")],
            ),
        ]
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        if self.place == "public" {
            self.public_page(ui);
            return;
        }
        let groups = self.groups();
        // the settings open over the page, which the rail keeps marked
        let mut place = self.place.clone();
        // what the rail started the frame on, so only a place chosen there moves the app after it
        let shown = place.clone();
        let label = PLACES
            .iter()
            .find(|(id, _)| *id == self.place)
            .map_or("Overview", |(_, label)| *label);
        let service = self.service.clone();
        let code = self
            .incident
            .as_deref()
            .filter(|_| self.place == "incidents")
            .and_then(|id| self.incidents.iter().find(|i| i.id == id))
            .map(|i| i.code.clone());
        let trail: Vec<&str> = match (&service, &code) {
            (Some(id), _) => vec!["Status", "Services", id],
            (None, Some(code)) => vec!["Status", "Incidents", code],
            (None, None) => vec!["Status", label],
        };
        let mut account = None;
        let mut toggled = false;
        let mut searched = false;
        if palette_shortcut(ui.ctx()) {
            self.palette = Some((String::new(), 0));
        }
        let clock = self.clock();
        let live = self.live;
        let operational = SERVICES.len();
        let open = self.open().len();
        let on_call = self.on_call.clone();
        let shell = if self.desktop {
            Shell::new().desktop(DesktopWindow::native())
        } else {
            Shell::new()
        };
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Status", &groups, &mut place).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new(ME)
                    .place("tidevann.example.org")
                    .appearance(true)
                    .show(ui);
            }
            ShellPart::Top => {
                let mut bar = TopBar::new(&trail).search("Search", "⌘K");
                // an incident's code is drawn in the code face, as the web's crumb
                if service.is_none() && code.is_some() {
                    bar = bar.code(2);
                }
                searched = bar
                    .show(ui, |ui| {
                        let (icon, word) = if live {
                            (Icon::Pause, "Live")
                        } else {
                            (Icon::Play, "Paused")
                        };
                        let button = Button::new(word)
                            .icon(icon)
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Sm);
                        toggled = ui.add(button).clicked();
                    })
                    .search;
            }
            ShellPart::Page => {
                // the listings fill the page, their tables scrolling inside it; the other pages
                // scroll as a whole
                let fills = matches!(
                    (self.place.as_str(), self.service.as_ref()),
                    ("services", None) | ("regions", _)
                ) || (self.place == "incidents" && self.incident.is_none());
                if fills {
                    Page::show(ui, |ui| self.page(ui));
                } else {
                    Page::scroll(ui, |ui| self.page(ui));
                }
            }
            ShellPart::Foot => {
                let note = if live {
                    format!("Live, last reading {clock}")
                } else {
                    "Paused".to_owned()
                };
                ui.add(
                    StatusLine::new(note)
                        .tone(if live { StatusTone::Live } else { StatusTone::Quiet })
                        .live(live)
                        .item("operational", format!("{operational}/{}", SERVICES.len()))
                        .item("open", open)
                        .item("on call", on_call.clone()),
                );
            }
        });
        if toggled {
            self.live = !self.live;
        }
        if searched {
            self.palette = Some((String::new(), 0));
        }
        self.palette(ui.ctx(), &groups);
        if place == "settings" {
            self.settings = true;
        } else if place != shown {
            self.go(&place);
        }
        // a service's dialogs over its page
        let now = (self.start + self.elapsed) as f64 * 1000.0;
        if let Some(form) = self.maintenance_form.as_mut() {
            let wall = sokkel_egui::Moment {
                day: self.today,
                minutes: (now / 60_000.0) as u32,
            };
            match form.show(ui.ctx(), self.today, wall) {
                Some(dialogs::Answer::Done(())) => {
                    if let Some(window) = form.make(self.today, ME, self.maintenance.len()) {
                        self.maintenance.push(window);
                    }
                    self.maintenance_form = None;
                }
                Some(dialogs::Answer::Cancel) => self.maintenance_form = None,
                None => {}
            }
        }
        if let Some(form) = self.deploy_form.as_mut() {
            match form.show(ui.ctx()) {
                Some(dialogs::Answer::Done(())) => {
                    let made = form.make(now, ME, self.deploys.len());
                    self.deploys.push(made);
                    self.deploy_form = None;
                }
                Some(dialogs::Answer::Cancel) => self.deploy_form = None,
                None => {}
            }
        }
        // the new incident dialog over whatever the board shows; opened, it goes to the incidents
        if let Some(form) = self.new_incident.as_mut() {
            let now = (self.start + self.elapsed) as f64 * 1000.0;
            match form.show(ui.ctx(), &self.incidents, now, ME) {
                Some(new_incident::Answer::Open(made)) => {
                    let id = made.id.clone();
                    self.incidents.insert(0, made);
                    self.new_incident = None;
                    self.go(&format!("incidents/{id}"));
                }
                Some(new_incident::Answer::Cancel) => self.new_incident = None,
                None => {}
            }
        }
        if let Some(form) = self.hand_over.as_mut() {
            let held: Vec<String> = self
                .incidents
                .iter()
                .filter(|i| i.state != data::State::Resolved && i.assignee == form.from)
                .map(|i| i.code.clone())
                .collect();
            match form.show(ui.ctx(), &held) {
                Some(dialogs::Answer::Done(())) => {
                    let (from, to, give) = (form.from, form.taker(), form.give);
                    let note = form.note.trim().to_owned();
                    self.hand_over = None;
                    self.hand_over_to(from, to, &note, give, now);
                }
                Some(dialogs::Answer::Cancel) => self.hand_over = None,
                None => {}
            }
        }
        self.settings_dialog(ui.ctx());
        self.show_notices(ui.ctx());
        sokkel_egui::Toaster::new().show(ui.ctx());
        match account {
            Some(AccountAction::Settings) => self.settings = true,
            Some(AccountAction::Appearance) => {
                self.settings = true;
                self.settings_state.active = "appearance".into();
            }
            None => {}
        }
    }

    /// The command palette, as the web's: the board's own commands, the rail's places, every
    /// service, the open incidents and then the resolved ones, and the look.
    fn palette(&mut self, ctx: &egui::Context, rail: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let (icon, label) = if self.live {
            (Icon::Pause, "Pause live readings")
        } else {
            (Icon::Play, "Resume live readings")
        };
        let mut groups = vec![CommandGroup::new(
            "Status",
            [
                MenuItem::action("Report an incident").icon(Icon::Siren).id("report"),
                MenuItem::action("Hand over on call")
                    .icon(Icon::ArrowRightLeft)
                    .id("hand-over"),
                MenuItem::action(label).icon(icon).id("live"),
                MenuItem::action("Open the status page")
                    .icon(Icon::SquareArrowOutUpRight)
                    .id("public"),
            ],
        )];
        groups.extend(place_commands(rail, "Go to"));
        groups.push(CommandGroup::new(
            "Services",
            SERVICES.iter().enumerate().map(|(index, one)| {
                MenuItem::action(one.id)
                    .icon(if one.store { Icon::Database } else { Icon::Server })
                    .hint(self.live_world.health[index].words().to_lowercase())
                    .keywords([one.host, one.group, one.description])
                    .id(format!("service:{}", one.id))
            }),
        ));
        let incidents = self
            .incidents
            .iter()
            .filter(|i| i.state != data::State::Resolved)
            .chain(self.incidents.iter().filter(|i| i.state == data::State::Resolved));
        groups.push(CommandGroup::new(
            "Incidents",
            incidents.map(|one| {
                let mut words: Vec<String> = vec![one.code.clone(), one.assignee.to_owned()];
                words.extend(one.services.iter().map(|s| (*s).to_owned()));
                words.extend(one.sites.iter().map(|id| {
                    data::SITES
                        .iter()
                        .find(|site| site.id == *id)
                        .map_or(*id, |site| site.city)
                        .to_owned()
                }));
                MenuItem::action(&one.title)
                    .icon(Icon::CircleAlert)
                    .hint(state_words(one.state))
                    .keywords(words)
                    .id(format!("incident:{}", one.id))
            }),
        ));
        let themes: Vec<_> = sokkel_egui::themes().collect();
        groups.extend(appearance_commands(
            &self.appearance,
            &themes,
            &AppearanceCommandWords::default(),
        ));
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight).show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        match command.as_deref() {
            Some("report") => self.report_incident(None),
            Some("hand-over") => self.hand_over = Some(dialogs::HandOverForm::new(&self.on_call)),
            Some("live") => self.live = !self.live,
            Some("public") => self.go("public"),
            Some(command) => {
                if let Some(id) = command.strip_prefix("service:") {
                    self.go(&format!("services/{id}"));
                } else if let Some(id) = command.strip_prefix("incident:") {
                    self.go(&format!("incidents/{id}"));
                } else if let Some(place) = place_of(command) {
                    let place = place.to_owned();
                    self.go(&place);
                } else {
                    run_appearance_command(&mut self.appearance, command);
                }
            }
            None => {}
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }

    /// The page the place names: its head now, its parts as the library gains them.
    fn page(&mut self, ui: &mut Ui) {
        let clock = self.clock();
        let live = self.live;
        let mut toggled = false;
        match (self.place.as_str(), self.service.as_deref()) {
            ("services", Some(id)) => {
                let id = id.to_owned();
                Stack::new(Gap::Page).show(ui, |ui| self.service_page(ui, &id));
            }
            ("services", None) => {
                Stack::new(Gap::Section).show(ui, |ui| self.services_page(ui));
            }
            ("incidents", _) if self.incident.is_some() => {
                Stack::new(Gap::Page).show(ui, |ui| self.incident_page(ui));
            }
            ("incidents", _) => {
                Stack::new(Gap::Section).show(ui, |ui| self.incidents_page(ui));
            }
            ("regions", _) => {
                Stack::new(Gap::Section).show(ui, |ui| self.regions_page(ui));
            }
            ("on-call", _) => {
                Stack::new(Gap::Page).show(ui, |ui| self.on_call_page(ui));
            }
            _ => {
                let editing = self.overview_edit.editing();
                let description = if editing {
                    "Drag a panel by its grip to move it, set its width or remove it, and drag a widget in to add it."
                        .to_owned()
                } else if live {
                    format!(
                        "Live, a reading every {} s, the last at {clock}.",
                        self.options.interval
                    )
                } else {
                    format!("Paused at {clock}.")
                };
                let mut report = false;
                let (edit, kept) = (&mut self.overview_edit, &mut self.overview_layout);
                let default = default_layout();
                PageHeader::new("Overview").description(description).show(ui, |ui| {
                    // right to left: the primary action last in reading order, hidden while editing
                    if !editing {
                        report = ui.add(Button::new("Report incident").icon(Icon::Siren)).clicked();
                    }
                    DashboardEditActions::new(edit, kept)
                        .default_layout(&default)
                        .size(ButtonSize::Default)
                        .show(ui);
                    if !editing {
                        let (icon, word) = if live {
                            (Icon::Pause, "Pause")
                        } else {
                            (Icon::Play, "Resume")
                        };
                        toggled = ui
                            .add(Button::new(word).icon(icon).variant(ButtonVariant::Outline))
                            .clicked();
                    }
                });
                if report {
                    self.report_incident(None);
                }
                self.kpis(ui);
                if let Some(id) = self.tiles(ui) {
                    self.go(&format!("services/{id}"));
                }
                if let Some(route) = self.board(ui) {
                    self.go(&route);
                }
            }
        }
        if toggled {
            self.live = !self.live;
        }
    }
}

impl StatusApp {
    /// The overview's figures: services and sites working, the open incidents, uptime, requests and
    /// who is on call. Until the live readings are ported, the requests are each service's resting
    /// load and the api's resting latency, and uptime has no history to read.
    fn kpis(&self, ui: &mut Ui) {
        let requests: f64 = SERVICES.iter().map(|s| s.load).sum();
        let api = SERVICES.iter().find(|s| s.id == "api").map_or(0.0, |s| s.latency);
        let all: Vec<f64> = self.uptime().iter().filter_map(|d| overall_uptime(d)).collect();
        let average = (!all.is_empty()).then(|| all.iter().sum::<f64>() / all.len() as f64);
        let opened = self.open();
        let mut open = Stat::new("Open incidents", opened.len());
        if let Some(first) = opened.first() {
            open = open.note(first.code.clone());
        }
        KpiRow::new([
            Stat::new("Services operational", format!("{0} of {0}", SERVICES.len())),
            Stat::new("Sites operational", format!("{0} of {0}", data::SITES.len())),
            open,
            Stat::new("Uptime, 90 days", average.map_or("-".to_owned(), format_uptime)),
            Stat::new("Requests", group(requests.round() as u64))
                .unit("/s")
                .note(format!("api p95 {} ms", api.round())),
            Stat::new("On call", &self.on_call).note("Hand over on the On call page"),
        ])
        .show(ui);
    }
}

impl StatusApp {
    /// The incidents: its head, the filter of open, resolved and all, and the table of them.
    fn incidents_page(&mut self, ui: &mut Ui) {
        let open = self.open().len();
        PageHeader::new("Incidents")
            .count(format!("{open} open"))
            .show(ui, |ui| {
                if ui.add(Button::new("New").icon(Icon::Plus)).clicked() {
                    self.report_incident(None);
                }
            });
        let t = sokkel_egui::tokens(ui.ctx());
        let all = self.incidents.len();
        ui.add(
            Segmented::new(&mut self.shown, ["Open", "Resolved", "All"])
                .counts([Some(open), Some(all - open), Some(all)])
                .dots([Some(t.color("critical")), Some(t.color("good")), None]),
        );
        self.incident_table(ui);
    }

    /// The incidents the filter shows, newest first, as the web's `IncidentTable`: stacked as
    /// labeled cards where its columns do not fit.
    fn incident_table(&mut self, ui: &mut Ui) {
        let now = (self.start + self.elapsed) as f64 * 1000.0;
        // every word searched for in what an incident says: its code, title, services, sites and
        // assignee, as the web's page matches them
        let words: Vec<String> = self.incident_search.split_whitespace().map(str::to_lowercase).collect();
        let listed: Vec<&data::Incident> = self
            .incidents
            .iter()
            .filter(|i| match self.shown {
                0 => i.state != data::State::Resolved,
                1 => i.state == data::State::Resolved,
                _ => true,
            })
            .filter(|i| {
                let text = format!(
                    "{} {} {} {} {}",
                    i.code,
                    i.title,
                    i.services.join(" "),
                    i.sites.join(" "),
                    i.assignee
                )
                .to_lowercase();
                words.iter().all(|w| text.contains(w))
            })
            .collect();
        let columns = [
            Column::new("Incident", 224.0).fill(),
            Column::new("Code", 88.0).stack(StackPlace::Secondary),
            Column::new("State", 136.0).stack(StackPlace::Secondary),
            Column::new("Severity", 104.0).stack(StackPlace::Secondary),
            Column::new("Touches", 144.0),
            Column::new("Assignee", 136.0),
            Column::new("Opened", 128.0).stack(StackPlace::Secondary),
            Column::new("Lasted", 104.0),
        ];
        let rows: Vec<Vec<Cell>> = listed.iter().map(|incident| incident_cells(incident, now)).collect();
        let keys: Vec<String> = listed.iter().map(|i| i.id.clone()).collect();
        // each row's menu as the web's `incidentActions`: resolve it or reopen it, give it to the
        // person on call
        let menus: Vec<(bool, bool)> = listed
            .iter()
            .map(|i| (i.state == data::State::Resolved, i.assignee == self.on_call))
            .collect();
        let on_call = self.on_call.clone();
        let output = DataTable::new("incidents", &columns, &rows, &mut self.incidents_table)
            .fits(true)
            .keys(keys.clone())
            .search(&mut self.incident_search)
            .stack_layout(StackLayout::Labeled)
            .actions(|row| {
                let (resolved, theirs) = menus[row];
                vec![
                    if resolved {
                        MenuItem::action("Reopen").icon(Icon::RotateCcw)
                    } else {
                        MenuItem::action("Resolve").icon(Icon::Check)
                    },
                    MenuItem::action(format!("Assign to {on_call}"))
                        .icon(Icon::PhoneCall)
                        .disabled(theirs),
                ]
            })
            .footer(false)
            .stretch(true)
            .show(ui);
        // a row pressed opens its incident
        if let Some(row) = self.incidents_table.opened.take()
            && let Some(id) = keys.get(row)
        {
            self.go(&format!("incidents/{id}"));
        }
        if let Some((row, action)) = output.action
            && let Some(id) = keys.get(row)
        {
            match action {
                0 => self.resolve_or_reopen(id),
                _ => self.assign_to_on_call(id),
            }
        }
    }

    /// A moment of the board's clock as the web's `formatMoment`: `Mon, Nov 16, 08:51`.
    pub fn moment(&self, at: f64) -> String {
        moment_of(self.today, at, self.options.twelve_hour)
    }

    /// One incident, as the web's `Incident` page: its head with its state, severity and how long
    /// it has been open, Resolve or Reopen; a form to post the next update and the updates so far;
    /// and beside them its details, each changed in place.
    fn incident_page(&mut self, ui: &mut Ui) {
        let t = sokkel_egui::tokens(ui.ctx());
        let now = (self.start + self.elapsed) as f64 * 1000.0;
        let Some(index) = self
            .incident
            .as_deref()
            .and_then(|id| self.incidents.iter().position(|i| i.id == id))
        else {
            return;
        };
        let id = self.incidents[index].id.clone();
        let (resolved, public) = {
            let i = &self.incidents[index];
            (i.state == data::State::Resolved, i.public)
        };
        let lasted = self.incidents[index].resolved.unwrap_or(now) - self.incidents[index].opened;
        let (mut status_page, mut toggle) = (false, false);
        let head = {
            let i = &self.incidents[index];
            PageHeader::new(&i.title)
                .back("Incidents")
                .edit_title("Title", |title| {
                    title.trim().is_empty().then(|| "An incident needs a title.".to_owned())
                })
                .code(&i.code)
                .meta_row(true)
                .chip(state_words(i.state), state_tone(i.state))
                .chip(severity_words(i.severity).to_lowercase(), severity_tone(i.severity))
                .meta(format!(
                    "{} {}",
                    if resolved { "Lasted" } else { "Open for" },
                    duration(lasted)
                ))
                .show(ui, |ui| {
                    let (icon, word, variant) = if resolved {
                        (Icon::RotateCcw, "Reopen", ButtonVariant::Outline)
                    } else {
                        (Icon::Check, "Resolve", ButtonVariant::Default)
                    };
                    toggle = ui.add(Button::new(word).icon(icon).variant(variant)).clicked();
                    if public {
                        status_page = ui
                            .add(
                                Button::new("Status page")
                                    .icon(Icon::ExternalLink)
                                    .variant(ButtonVariant::Outline),
                            )
                            .clicked();
                    }
                })
        };
        if head.back {
            self.go("incidents");
            return;
        }
        if status_page {
            self.go("public");
            return;
        }
        if toggle {
            self.resolve_or_reopen(&id);
        }
        if let Some(title) = &head.renamed {
            self.incidents[index].title = title.trim().to_owned();
        }
        let on_call = self.on_call.clone();
        let mut post = false;
        let moments: Vec<String> = self.incidents[index]
            .updates
            .iter()
            .map(|u| self.moment(u.at))
            .collect();
        let opened_at = self.moment(self.incidents[index].opened);
        let resolved_at = self.incidents[index].resolved.map(|r| self.moment(r));
        let (post_state, post_message) = (&mut self.post_state, &mut self.post_message);
        let incident = &mut self.incidents[index];
        Split::new(AsideWidth::Lg).show(
            ui,
            |ui| {
                Stack::new(Gap::Section).show(ui, |ui| {
                    Section::new("Post an update")
                        .description(
                            "It goes on the timeline, and on the status page while the incident is shown there.",
                        )
                        .show(ui, |ui| {
                            Stack::new(Gap::Md).show(ui, |ui| {
                                ui.add(Segmented::new(
                                    post_state,
                                    ["Investigating", "Identified", "Monitoring", "Resolved"],
                                ));
                                let placeholder = if *post_state == 3 {
                                    "What fixed it."
                                } else {
                                    "What you know now, and what happens next."
                                };
                                ui.add(
                                    sokkel_egui::TextField::new("Update", post_message)
                                        .placeholder(placeholder)
                                        .multiline(true),
                                );
                                form_bar(ui, Some(&format!("Posted as {ME}")), |ui| {
                                    let mut button = Button::new("Post").icon(Icon::Send);
                                    if post_message.trim().is_empty() {
                                        button = button.reason("Write what changed first.");
                                    }
                                    post = ui.add(button).clicked() && !post_message.trim().is_empty();
                                });
                            });
                        });
                    let events: Vec<TimelineEvent> = incident
                        .updates
                        .iter()
                        .zip(&moments)
                        .map(|(u, at)| TimelineEvent {
                            title: u.message.clone(),
                            at: relative_time(u.at - now),
                            detail: Some(format!("{} · {} · {at}", u.by, state_name(u.state))),
                            person: Some(u.by.to_string()),
                            tone: Some(t.color(state_color(u.state))),
                            ..Default::default()
                        })
                        .collect();
                    Section::new("Updates")
                        .meta(incident.updates.len().to_string())
                        .show(ui, |ui| Timeline::new(events).show(ui));
                });
            },
            |ui| {
                Section::new("Details").show(ui, |ui| {
                    let mut severity = match incident.severity {
                        data::Severity::Minor => 0,
                        data::Severity::Major => 1,
                        data::Severity::Critical => 2,
                    };
                    let mut assignee = data::PEOPLE.iter().position(|p| *p == incident.assignee).unwrap_or(0);
                    let mut shown = incident.public;
                    let mut take = false;
                    let touches: Vec<String> = incident
                        .services
                        .iter()
                        .map(|s| s.to_string())
                        .chain(incident.sites.iter().map(|id| {
                            data::SITES
                                .iter()
                                .find(|s| s.id == *id)
                                .map_or(*id, |s| s.city)
                                .to_string()
                        }))
                        .collect();
                    let theirs = incident.assignee == on_call;
                    DescriptionList::new()
                        .stacked()
                        .item("Severity", |ui| {
                            ui.add(Segmented::new(&mut severity, ["Minor", "Major", "Critical"]));
                        })
                        .item("Assignee", |ui| {
                            let width = ui.available_width();
                            ui.add(Select::new(&mut assignee, data::PEOPLE, "Assignee").width(width));
                            if !theirs {
                                ui.add_space(4.0);
                                take = ui
                                    .add(
                                        Button::new(format!("Assign to {on_call}, on call"))
                                            .icon(Icon::PhoneCall)
                                            .variant(ButtonVariant::Outline)
                                            .size(ButtonSize::Sm),
                                    )
                                    .clicked();
                            }
                        })
                        .item("Touches", |ui| {
                            ui.add(sokkel_egui::TagChips::new(&touches));
                        })
                        .text("Opened", opened_at.clone())
                        .text("Resolved", resolved_at.clone().unwrap_or_else(|| "Not yet".into()))
                        .item("On the status page", |ui| {
                            ui.add(sokkel_egui::Switch::new(&mut shown, "On the status page"));
                        })
                        .show(ui);
                    incident.severity =
                        [data::Severity::Minor, data::Severity::Major, data::Severity::Critical][severity];
                    incident.assignee = if take {
                        data::PEOPLE
                            .iter()
                            .find(|p| **p == on_call)
                            .copied()
                            .unwrap_or(incident.assignee)
                    } else {
                        data::PEOPLE[assignee]
                    };
                    incident.public = shown;
                });
            },
        );
        if post {
            let state = [
                data::State::Investigating,
                data::State::Identified,
                data::State::Monitoring,
                data::State::Resolved,
            ][self.post_state];
            let message = self.post_message.trim().to_owned();
            let incident = &mut self.incidents[index];
            incident.state = state;
            incident.resolved = (state == data::State::Resolved).then_some(now);
            // a person's update takes the incident over from the responder
            incident.cause = None;
            incident.updates.insert(
                0,
                data::Update {
                    at: now,
                    state,
                    message,
                    by: ME,
                },
            );
            self.post_message.clear();
            self.post_state = (state_index(state) + 1).min(3);
        }
    }

    /// Open the new incident dialog, fresh, the service it was opened from ticked.
    pub fn report_incident(&mut self, service: Option<&str>) {
        self.new_incident = Some(new_incident::NewIncident::new(service, &self.on_call));
    }

    /// Resolve an open incident, or reopen a resolved one, with an update saying so, as the web's
    /// `postUpdate`.
    pub fn resolve_or_reopen(&mut self, id: &str) {
        let now = (self.start + self.elapsed) as f64 * 1000.0;
        let Some(incident) = self.incidents.iter_mut().find(|i| i.id == id) else {
            return;
        };
        let (state, message) = if incident.state == data::State::Resolved {
            (data::State::Investigating, "We are looking into this again.")
        } else {
            (data::State::Resolved, "This incident is resolved.")
        };
        incident.state = state;
        incident.resolved = (state == data::State::Resolved).then_some(now);
        incident.cause = None;
        incident.updates.insert(
            0,
            data::Update {
                at: now,
                state,
                message: message.into(),
                by: ME,
            },
        );
    }

    /// The pager goes from `from` to `to` at `now`, kept in the handovers, and `from`'s open
    /// incidents with it when `give`.
    pub fn hand_over_to(&mut self, from: &'static str, to: &'static str, note: &str, give: bool, now: f64) {
        self.on_call = to.to_owned();
        self.on_call_note = note.to_owned();
        let id = format!("handover-new-{}", self.handovers.len());
        self.handovers.insert(
            0,
            data::Handover {
                id,
                at: now,
                from,
                to,
                note: note.to_owned(),
            },
        );
        if give {
            for incident in &mut self.incidents {
                if incident.state != data::State::Resolved && incident.assignee == from {
                    incident.assignee = to;
                }
            }
        }
    }

    /// Give an incident to the person on call.
    pub fn assign_to_on_call(&mut self, id: &str) {
        let on_call = self.on_call.clone();
        if let Some(incident) = self.incidents.iter_mut().find(|i| i.id == id)
            && let Some(person) = data::PEOPLE.iter().find(|p| **p == on_call)
        {
            incident.assignee = person;
        }
    }

    /// The services: their head with Reset order, and the table of them in the person's order.
    fn services_page(&mut self, ui: &mut Ui) {
        let t = sokkel_egui::tokens(ui.ctx());
        let down = self
            .live_world
            .health
            .iter()
            .filter(|h| **h != live::Health::Operational)
            .count();
        let count = if down == 0 {
            "all operational".to_owned()
        } else {
            format!("{down} not operational")
        };
        let ordered = self.service_order != data::board_order();
        let mut reset = false;
        PageHeader::new("Services").count(count).show(ui, |ui| {
            let mut button = Button::new("Reset order")
                .icon(Icon::RotateCcw)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Sm);
            if !ordered {
                button = button.reason("The services are in the board’s own order.");
            }
            reset = ui.add(button).clicked();
        });
        if reset {
            self.service_order = data::board_order();
        }
        let columns = [
            Column::new("Service", 128.0),
            Column::new("State", 136.0).stack(StackPlace::Secondary),
            Column::new("What it does", 224.0).fill().stack(StackPlace::Primary),
            Column::new("Latency", 144.0).sortable(false).stack(StackPlace::None),
            Column::new("p95", 96.0).numeric().stack(StackPlace::Secondary),
            Column::new("Errors", 88.0).numeric().stack(StackPlace::Secondary),
            Column::new("Requests", 104.0).numeric().stack(StackPlace::None),
            Column::new("Uptime, 90 days", 136.0)
                .numeric()
                .stack(StackPlace::Secondary),
            Column::new("Host", 192.0).stack(StackPlace::None),
        ];
        let uptime = self.uptime();
        let rows: Vec<Vec<Cell>> = self
            .service_order
            .iter()
            .map(|&index| {
                let service = SERVICES[index];
                let health = self.live_world.health[index];
                let readings = &self.live_world.readings[index];
                let last = readings.last().copied();
                vec![
                    Cell::Text(service.id.into()),
                    Cell::Chip(health.words().to_lowercase(), health_tone(health)),
                    Cell::Text(service.description.into()),
                    Cell::Trend(readings.iter().map(|r| r.p95).collect(), t.color(health.tone())),
                    Cell::Text(format!("{} ms", last.map_or(0.0, |r| r.p95).round())),
                    Cell::Text(rate(last.map_or(0.0, |r| r.errors))),
                    Cell::Text(format!("{}/s", group(last.map_or(0.0, |r| r.load).round() as u64))),
                    Cell::Text(overall_uptime(&uptime[index]).map_or("-".to_owned(), format_uptime)),
                    Cell::Mono(service.host.into()),
                ]
            })
            .collect();
        let keys: Vec<&str> = self.service_order.iter().map(|i| SERVICES[*i].id).collect();
        let output = DataTable::new("services", &columns, &rows, &mut self.services_table)
            .fits(true)
            .keys(keys)
            .bar(true)
            .reorder(true)
            .stack_layout(StackLayout::Labeled)
            .actions(|_| vec![MenuItem::action("Open"), MenuItem::action("Copy the host")])
            .footer(false)
            .stretch(true)
            .show(ui);
        if let Some((from, to)) = output.reordered {
            let moved = self.service_order.remove(from);
            self.service_order.insert(to, moved);
        }
    }

    /// What customers see, as the web's `PublicPage`: the bar with the mark and the page's title, a
    /// line on how everything is, then in a column 768 wide the incidents going on with their
    /// updates, the maintenance ahead, each service with its last ninety days, and what happened
    /// each of the last seven days.
    fn public_page(&mut self, ui: &mut Ui) {
        let t = sokkel_egui::tokens(ui.ctx());
        let now = (self.start + self.elapsed) as f64 * 1000.0;
        let window = ui.ctx().content_rect();
        let mut page = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(window)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        page.set_clip_rect(window);
        let ui = &mut page;
        ui.painter().rect_filled(window, 0.0, t.color("background"));
        ui.spacing_mut().item_spacing.y = 0.0;
        let title = self.options.page_title.clone();
        let trail = [title.as_str()];
        let mut bar = AppBar::new(&trail).mark(TIDEVANN_MARK);
        if self.desktop {
            bar = bar.desktop(DesktopWindow::native());
        }
        let mut board = false;
        let mut reveal = false;
        bar.show(ui, |ui| {
            board = ui
                .add(IconButton::new(Icon::LayoutDashboard, "Open the board"))
                .clicked();
            reveal = ui
                .add(
                    Button::new("Subscribe")
                        .icon(Icon::BellRing)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm),
                )
                .clicked();
        });
        // Subscribe brings the form into view, as the web's scrolls to it
        if reveal {
            self.subscribe.reveal = true;
        }
        let overall = self
            .live_world
            .health
            .iter()
            .copied()
            .max_by_key(|h| match h {
                live::Health::Operational => 0,
                live::Health::Maintenance => 1,
                live::Health::Degraded => 2,
                live::Health::Outage => 3,
            })
            .unwrap_or(live::Health::Operational);
        let (tone, words) = match overall {
            live::Health::Operational => (BannerTone::Good, "All systems operational"),
            live::Health::Maintenance => (BannerTone::Info, "Maintenance in progress"),
            live::Health::Degraded => (BannerTone::Warning, "Some systems are degraded"),
            live::Health::Outage => (BannerTone::Critical, "Some systems are down"),
        };
        Banner::new(tone, format!("{words}, as of {}", self.clock())).show(ui);
        let today = self.today;
        let twelve = self.options.twelve_hour;
        let moment = move |at: f64| moment_of(today, at, twelve);
        let affects = |incident: &data::Incident| {
            let mut parts: Vec<String> = incident.services.iter().map(|s| s.to_string()).collect();
            parts.extend(incident.sites.iter().map(|id| {
                data::SITES
                    .iter()
                    .find(|s| s.id == *id)
                    .map_or(*id, |s| s.city)
                    .to_string()
            }));
            if parts.is_empty() {
                String::new()
            } else {
                format!("Affects {}", parts.join(", "))
            }
        };
        let uptime = self.uptime();
        let health = self.live_world.health.clone();
        let services: Vec<usize> = (0..SERVICES.len()).collect();
        egui::ScrollArea::vertical()
            .id_salt("public")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let room = ui.available_width();
                let column = room.min(768.0);
                let inset = ((room - column) / 2.0).max(0.0);
                ui.add_space(32.0);
                ui.horizontal(|ui| {
                    ui.add_space(inset + 16.0);
                    ui.vertical(|ui| {
                        ui.set_width(column - 32.0);
                        Stack::new(Gap::Xl).show(ui, |ui| {
                            for incident in self
                                .incidents
                                .iter()
                                .filter(|i| i.public && i.state != data::State::Resolved)
                            {
                                let mut updates: Vec<&data::Update> = incident.updates.iter().collect();
                                updates.sort_by(|a, b| b.at.total_cmp(&a.at));
                                let events: Vec<TimelineEvent> = updates
                                    .iter()
                                    .map(|u| TimelineEvent {
                                        title: u.message.clone(),
                                        at: moment(u.at),
                                        detail: Some(state_name(u.state).into()),
                                        tone: Some(t.color(state_color(u.state))),
                                        ..Default::default()
                                    })
                                    .collect();
                                let state = incident.state;
                                Section::new(&incident.title)
                                    .description(affects(incident))
                                    .actions(move |ui| {
                                        ui.add(
                                            sokkel_egui::Chip::new(state_words(state).to_lowercase())
                                                .tone(state_tone(state)),
                                        );
                                    })
                                    .show(ui, |ui| Timeline::new(events).show(ui));
                            }
                            let mut ahead: Vec<&data::Maintenance> = self
                                .maintenance
                                .iter()
                                .filter(|w| !w.cancelled && w.ends > now)
                                .collect();
                            ahead.sort_by(|a, b| a.starts.total_cmp(&b.starts));
                            if !ahead.is_empty() {
                                let events: Vec<TimelineEvent> = ahead
                                    .iter()
                                    .map(|w| TimelineEvent {
                                        title: w.title.clone(),
                                        at: if w.going_on(now) {
                                            "now".into()
                                        } else {
                                            moment(w.starts)
                                        },
                                        detail: Some(format!("{}, until {}", w.service, moment(w.ends))),
                                        tone: Some(t.color("info")),
                                        ..Default::default()
                                    })
                                    .collect();
                                Section::new("Scheduled maintenance").show(ui, |ui| Timeline::new(events).show(ui));
                            }
                            let show_uptime = self.options.show_uptime;
                            let mut services_section = Section::new("Services");
                            if show_uptime {
                                services_section = services_section.meta("the last 90 days");
                            }
                            services_section.show(ui, |ui| {
                                if show_uptime {
                                    uptime_list(ui, &services, &health, &uptime);
                                } else {
                                    // each service and its state, a line each, as the web's without the bars
                                    Stack::new(Gap::Sm).show(ui, |ui| {
                                        for (index, service) in SERVICES.iter().enumerate() {
                                            ui.horizontal(|ui| {
                                                ui.label(TextRole::Body.rich(&t, service.id));
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        let state = health[index];
                                                        ui.add(
                                                            sokkel_egui::Chip::new(state.words().to_lowercase())
                                                                .tone(health_tone(state)),
                                                        );
                                                    },
                                                );
                                            });
                                        }
                                    });
                                }
                            });
                            let history_days = self.options.history_days as i32;
                            let events: Vec<TimelineEvent> = (0..history_days)
                                .map(|back| {
                                    let start = -f64::from(back) * 86_400_000.0;
                                    let happened: Vec<&data::Incident> = self
                                        .incidents
                                        .iter()
                                        .filter(|i| i.public && i.opened >= start && i.opened < start + 86_400_000.0)
                                        .collect();
                                    let title = self.today.plus(-back).full();
                                    if happened.is_empty() {
                                        return TimelineEvent {
                                            title,
                                            detail: Some("No incidents reported.".into()),
                                            ..Default::default()
                                        };
                                    }
                                    let worst = if happened.iter().any(|i| i.severity != data::Severity::Minor) {
                                        "critical"
                                    } else {
                                        "warning"
                                    };
                                    let lines: Vec<(String, data::State, [String; 2])> = happened
                                        .iter()
                                        .map(|i| {
                                            let latest = i
                                                .updates
                                                .iter()
                                                .max_by(|a, b| a.at.total_cmp(&b.at))
                                                .map_or(String::new(), |u| u.message.clone());
                                            let meta = [latest, affects(i)];
                                            (i.title.clone(), i.state, meta)
                                        })
                                        .collect();
                                    TimelineEvent {
                                        title,
                                        tone: Some(t.color(worst)),
                                        content: Some(Box::new(move |ui: &mut Ui| {
                                            let t = sokkel_egui::tokens(ui.ctx());
                                            Stack::new(Gap::Md).show(ui, |ui| {
                                                for (title, state, meta) in lines {
                                                    ui.vertical(|ui| {
                                                        ui.spacing_mut().item_spacing.y = 2.0;
                                                        ui.horizontal(|ui| {
                                                            // the title and its chip `gap-xs` apart
                                                            ui.spacing_mut().item_spacing.x = 6.0;
                                                            ui.label(TextRole::Body.rich(&t, title).variation(
                                                                "wght",
                                                                t.weight(sokkel_egui::Weight::Medium),
                                                            ));
                                                            ui.add(
                                                                sokkel_egui::Chip::new(
                                                                    state_words(state).to_lowercase(),
                                                                )
                                                                .tone(state_tone(state)),
                                                            );
                                                        });
                                                        MetaLine::new(meta).show(ui);
                                                    });
                                                }
                                            });
                                        })),
                                        ..Default::default()
                                    }
                                })
                                .collect();
                            Section::new("Past incidents")
                                .meta(format!("the last {history_days} days"))
                                .show(ui, |ui| Timeline::new(events).show(ui));
                            self.subscribe.now = (self.start + self.elapsed) as f64 * 1000.0;
                            subscribe_form(ui, &mut self.subscribe);
                            MetaLine::new([
                                "Times are in your own time zone".to_owned(),
                                format!("{} services", SERVICES.len()),
                            ])
                            .show(ui);
                        });
                    });
                });
                ui.add_space(32.0);
            });
        if board {
            self.go("overview");
        }
    }

    /// The board under the tiles, as it comes: the sites on a map beside what is open, then every
    /// service's last ninety days. Returns the route a panel asked to go to.
    fn board(&mut self, ui: &mut Ui) -> Option<String> {
        let t = sokkel_egui::tokens(ui.ctx());
        let now = (self.start + self.elapsed) as f64 * 1000.0;
        let mut to_all = None;
        let mut to_day = None;
        let markers = self.site_markers(&t);
        let legend = self.sites_legend(&t);
        let open: Vec<&data::Incident> = self
            .incidents
            .iter()
            .filter(|i| i.state != data::State::Resolved)
            .collect();
        let columns = [
            Column::new("Incident", 224.0).fill(),
            Column::new("Code", 88.0).stack(StackPlace::Secondary),
            Column::new("State", 136.0).stack(StackPlace::Secondary),
            Column::new("Severity", 104.0).stack(StackPlace::Secondary),
            Column::new("Opened", 128.0).stack(StackPlace::Secondary),
        ];
        // the compact table: the title, code, state, severity and when it opened
        let rows: Vec<Vec<Cell>> = open
            .iter()
            .map(|i| {
                incident_cells(i, now)
                    .into_iter()
                    .enumerate()
                    .filter(|(column, _)| [0, 1, 2, 3, 6].contains(column))
                    .map(|(_, cell)| cell)
                    .collect()
            })
            .collect();
        let open_count = open.len();
        let keys: Vec<String> = open.iter().map(|i| i.id.clone()).collect();
        let uptime = self.uptime();
        let order = self.service_order.clone();
        let health = self.live_world.health.clone();
        // the live charts: every service in the person's order, by its last readings
        let index_of = |id: &str| SERVICES.iter().position(|s| s.id == id);
        let api = index_of("api").unwrap_or(0);
        let x: Vec<String> = self.live_world.readings[api].iter().map(|r| clock_of(r.at)).collect();
        let series = |pick: fn(&live::Reading) -> f64| -> Vec<Series> {
            order
                .iter()
                .map(|&i| Series::new(SERVICES[i].id, self.live_world.readings[i].iter().map(pick)))
                .collect()
        };
        let latency = series(|r| r.p95.round());
        let requests = series(|r| r.load.round());
        let errors = series(|r| (r.errors * 100.0).round() / 100.0);
        // who is on call now and next, the windows coming up and the latest deploys
        let today = self.today;
        let next_shift = self.shifts.iter().find(|s| s.starts > today).cloned();
        let monday = today.plus(-(today.weekday() as i32));
        let on_call = self.on_call.clone();
        let mut windows: Vec<&data::Maintenance> = self
            .maintenance
            .iter()
            .filter(|w| !w.cancelled && w.ends > now)
            .collect();
        windows.sort_by(|a, b| a.starts.total_cmp(&b.starts));
        let windows: Vec<ListRow> = windows
            .iter()
            .take(5)
            .map(|w| {
                ListRow::new(w.title.clone())
                    .detail(format!(
                        "{}, {}",
                        w.service,
                        moment_of(today, w.starts, self.options.twelve_hour)
                    ))
                    .mark(RowMark::Icon(Icon::CalendarClock, None))
                    .pressable(true)
            })
            .collect();
        let window_services: Vec<&str> = {
            let mut w: Vec<&data::Maintenance> = self
                .maintenance
                .iter()
                .filter(|w| !w.cancelled && w.ends > now)
                .collect();
            w.sort_by(|a, b| a.starts.total_cmp(&b.starts));
            w.iter().take(5).map(|w| w.service).collect()
        };
        let mut deploys: Vec<&data::Deploy> = self.deploys.iter().collect();
        deploys.sort_by(|a, b| b.at.total_cmp(&a.at));
        let deploy_services: Vec<&str> = deploys.iter().take(6).map(|d| d.service).collect();
        let deploys: Vec<ListRow> = deploys
            .iter()
            .take(6)
            .map(|d| {
                let (word, tone) = match d.state {
                    data::DeployState::Running => ("running", "running"),
                    data::DeployState::Succeeded => ("succeeded", "succeeded"),
                    data::DeployState::Failed => ("failed", "failed"),
                };
                ListRow::new(format!("{} {}", d.service, d.version))
                    .detail(relative_time(d.at - now))
                    .mark(RowMark::Icon(Icon::Rocket, None))
                    .aside(RowAside::Chip(word.into(), ChipTone::Status(tone)))
                    .pressable(true)
            })
            .collect();
        let cards = self.site_cards(&t);
        let mut to_card: Option<String> = None;
        let opened = &mut to_card;
        let map = &mut self.overview_map;
        let table = &mut self.open_table;
        let mut to_page: Option<String> = None;
        let shown: Vec<String> = self
            .overview_edit
            .layout(&mut self.overview_layout)
            .iter()
            .map(|i| i.id.clone())
            .collect();
        let palette: Vec<DashboardWidget> = WIDGETS
            .iter()
            .filter(|(id, ..)| !shown.iter().any(|s| s == id))
            .map(|(id, label, icon, span)| DashboardWidget::new(*id, *label).icon(*icon).span(*span))
            .collect();
        let editing = self.overview_edit.editing();
        let layout = self.overview_edit.layout(&mut self.overview_layout);
        let label = |item: &DashboardLayoutItem| widget_label(&item.id).to_owned();
        let arranged = DashboardArrange::new("overview", layout, &label)
            .arranging(editing)
            .palette(&palette)
            .show(ui, |ui, item| match item.id.as_str() {
                "sites" => {
                    DashboardPanel::new("Sites")
                        .subtitle("by state, live")
                        .flush(true)
                        .show(ui, |ui| {
                            let size = egui::vec2(ui.available_width(), 344.0);
                            let response = map.show(ui, size, &markers, &[]);
                            legend.clone().show(ui, &response);
                            if let Some(incident) = site_card(ui, map, &cards) {
                                *opened = Some(format!("incidents/{incident}"));
                            }
                        });
                }
                "open" => {
                    let subtitle = if open_count == 0 {
                        "none".to_owned()
                    } else {
                        open_count.to_string()
                    };
                    let all = &mut to_all;
                    DashboardPanel::new("Open incidents")
                        .subtitle(subtitle)
                        .flush(true)
                        .actions(|ui| {
                            let button = Button::new("All").variant(ButtonVariant::Ghost).size(ButtonSize::Sm);
                            if ui.add(button).clicked() {
                                *all = Some("incidents".to_owned());
                            }
                        })
                        .show(ui, |ui| {
                            if rows.is_empty() {
                                sokkel_egui::Empty::new("Nothing open")
                                    .icon(Icon::CircleCheck)
                                    .description("Every incident is resolved.")
                                    .show(ui, |_| {});
                            } else {
                                DataTable::new("open-incidents", &columns, &rows, table)
                                    .fits(true)
                                    .keys(keys.clone())
                                    .stack_layout(StackLayout::Labeled)
                                    .footer(false)
                                    .show(ui);
                            }
                        });
                }
                "uptime" => {
                    DashboardPanel::new("Uptime")
                        .subtitle("the last 90 days")
                        .show(ui, |ui| {
                            if let Some((service, day)) = uptime_list(ui, &order, &health, &uptime) {
                                to_day = Some(format!("services/{service}/{day}"));
                            }
                        });
                }
                "latency" => {
                    let mut panel = TimeSeriesPanel::new("Latency", x.clone())
                        .subtitle("p95 of each service, live")
                        .unit("ms")
                        .label_every(15);
                    for s in &latency {
                        panel = panel.series(s.clone());
                    }
                    panel.show(ui);
                }
                "requests" => {
                    let mut panel = TimeSeriesPanel::new("Requests", x.clone())
                        .subtitle("per second, live")
                        .stacked(true)
                        .label_every(15);
                    for s in &requests {
                        panel = panel.series(s.clone());
                    }
                    panel.show(ui);
                }
                "errors" => {
                    let mut panel = TimeSeriesPanel::new("Errors", x.clone())
                        .subtitle("share of requests, live")
                        .unit("%")
                        .decimals(2)
                        .label_every(15);
                    for s in &errors {
                        panel = panel.series(s.clone());
                    }
                    panel.show(ui);
                }
                "on-call" => {
                    let page = &mut to_page;
                    DashboardPanel::new("On call")
                        .subtitle("now")
                        .flush(true)
                        .actions(|ui| {
                            let button = Button::new("Rotation")
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Sm);
                            if ui.add(button).clicked() {
                                *page = Some("on-call".to_owned());
                            }
                        })
                        .show(ui, |ui| {
                            let mut rows = vec![
                                ListRow::new(on_call.clone())
                                    .detail("On call now")
                                    .mark(RowMark::Person(on_call.clone()))
                                    .aside(RowAside::Text(format!("since {}, 09:00", monday.short())))
                                    .wrap(true),
                            ];
                            if let Some(next) = &next_shift {
                                rows.push(
                                    ListRow::new(next.person)
                                        .detail("Next on call")
                                        .mark(RowMark::Person(next.person.into()))
                                        .aside(RowAside::Text(next.starts.short())),
                                );
                            }
                            ListRows::new(rows).flush(true).show(ui);
                        });
                }
                "maintenance" => {
                    DashboardPanel::new("Maintenance")
                        .subtitle("coming up")
                        .flush(true)
                        .show(ui, |ui| {
                            if windows.is_empty() {
                                sokkel_egui::Empty::new("No maintenance is planned.").show(ui, |_| {});
                            } else if let Some(i) = ListRows::new(windows.clone()).flush(true).show(ui) {
                                to_page = Some(format!("services/{}", window_services[i]));
                            }
                        });
                }
                "deploys" => {
                    DashboardPanel::new("Deploys")
                        .subtitle("the latest")
                        .flush(true)
                        .show(ui, |ui| {
                            if deploys.is_empty() {
                                sokkel_egui::Empty::new("No deploys yet.").show(ui, |_| {});
                            } else if let Some(i) = ListRows::new(deploys.clone()).flush(true).show(ui) {
                                to_page = Some(format!("services/{}", deploy_services[i]));
                            }
                        });
                }
                _ => {}
            });
        if let Some((widget, index)) = arranged.added
            && let Some((id, _, _, span)) = WIDGETS.iter().find(|(id, ..)| *id == widget)
        {
            let layout = self.overview_edit.layout(&mut self.overview_layout);
            let at = index.min(layout.len());
            layout.insert(at, DashboardLayoutItem::new(*id, *span));
        }
        to_all.or(to_day).or(to_page).or(to_card)
    }

    /// Each site as a point in its state's color.
    fn site_markers(&self, t: &sokkel_egui::Tokens) -> Vec<Marker> {
        data::SITES
            .iter()
            .enumerate()
            .map(|(i, site)| {
                Marker::new(site.at[1], site.at[0], "")
                    .id(site.id)
                    .color(t.color(self.live_world.site_health[i].color()))
            })
            .collect()
    }

    /// Each site's card, as the web's `SitesMap` opens it: where it is, its state, its round trip
    /// now and usually, and the open incident that covers it.
    fn site_cards(&self, t: &sokkel_egui::Tokens) -> Vec<SiteCard> {
        data::SITES
            .iter()
            .enumerate()
            .map(|(i, site)| {
                let health = self.live_world.site_health[i];
                let covering = self
                    .incidents
                    .iter()
                    .find(|incident| incident.state != data::State::Resolved && incident.sites.contains(&site.id));
                SiteCard {
                    id: site.id,
                    city: site.city,
                    place: format!("{}, {}", site.country, site.region),
                    state: health.words(),
                    color: t.color(health.color()),
                    now: self.live_world.sites[i]
                        .last()
                        .map_or("-".to_owned(), |ms| format_ms(*ms)),
                    usually: format_ms(site.latency),
                    incident: covering.map(|incident| (incident.code.clone(), incident.id.clone())),
                }
            })
            .collect()
    }

    /// The sites map's legend: every site counted by its state.
    fn sites_legend(&self, t: &sokkel_egui::Tokens) -> MapLegend {
        let count = |h: live::Health| self.live_world.site_health.iter().filter(|x| **x == h).count();
        MapLegend::new("Sites")
            .description(format!("{} cities", data::SITES.len()))
            .row(t.color("good"), "Operational", count(live::Health::Operational))
            .row(t.color("warning"), "Degraded", count(live::Health::Degraded))
            .row(t.color("critical"), "Outage", count(live::Health::Outage))
    }

    /// Every site on a map and in a table, both live: a region frames its sites, and a site chosen
    /// in the table is chosen on the map.
    fn regions_page(&mut self, ui: &mut Ui) {
        let t = sokkel_egui::tokens(ui.ctx());
        let working = self
            .live_world
            .site_health
            .iter()
            .filter(|h| **h == live::Health::Operational)
            .count();
        PageHeader::new("Regions")
            .count(format!("{working} of {} sites operational", data::SITES.len()))
            .show(ui, |_| {});
        let mut regions: Vec<&str> = vec!["All"];
        for site in &data::SITES {
            if !regions.contains(&site.region) {
                regions.push(site.region);
            }
        }
        let chips: Vec<(&str, &str)> = regions.iter().map(|r| (*r, *r)).collect();
        if ChipBar::new("regions", &chips).show(ui, &mut self.region) {
            self.selected_site = None;
            let listed: Vec<&data::Site> = data::SITES
                .iter()
                .filter(|s| self.region == "All" || s.region == self.region)
                .collect();
            self.sites_map.frame(Some(bounds_of(&listed)));
        }
        let listed: Vec<usize> = (0..data::SITES.len())
            .filter(|i| self.region == "All" || data::SITES[*i].region == self.region)
            .collect();
        let markers = self.site_markers(&t);
        let legend = self.sites_legend(&t);
        let columns = [
            Column::new("City", 128.0).fill(),
            Column::new("State", 136.0).stack(StackPlace::Secondary),
            Column::new("Country", 128.0).stack(StackPlace::Secondary),
            Column::new("Round trip", 120.0).sortable(false).stack(StackPlace::None),
            Column::new("Now", 88.0).numeric().stack(StackPlace::Secondary),
        ];
        let rows: Vec<Vec<Cell>> = listed
            .iter()
            .map(|&i| {
                let site = &data::SITES[i];
                let health = self.live_world.site_health[i];
                let trips = &self.live_world.sites[i];
                let tone = if health == live::Health::Operational {
                    "primary"
                } else {
                    "critical"
                };
                vec![
                    Cell::Text(site.city.into()),
                    Cell::Chip(health.words().to_lowercase(), health_tone(health)),
                    Cell::Text(site.country.into()),
                    Cell::Trend(trips.clone(), t.color(tone)),
                    Cell::Text(format!("{} ms", trips.last().copied().unwrap_or(0.0).round())),
                ]
            })
            .collect();
        let keys: Vec<&str> = listed.iter().map(|i| data::SITES[*i].id).collect();
        let cards = self.site_cards(&t);
        // the site chosen, drawn as opened, as the web's `opened`
        let open_row = self
            .selected_site
            .and_then(|site| listed.iter().position(|i| *i == site));
        let mut to_incident = None;
        let opened = &mut to_incident;
        let map = &mut self.sites_map;
        let table = &mut self.sites_table;
        let mut chosen = None;
        let mut troubled = false;
        SplitPane::new("status-regions")
            .default_share(0.6)
            .min_share(0.35)
            .max_share(0.75)
            .show(
                ui,
                |ui| {
                    let size = ui.available_size().max(egui::vec2(1.0, 320.0));
                    let response = map.show(ui, size, &markers, &[]);
                    legend.clone().show(ui, &response);
                    if let Some(incident) = site_card(ui, map, &cards) {
                        *opened = Some(incident);
                    }
                },
                |ui| {
                    let output = DataTable::new("sites", &columns, &rows, table)
                        .fits(true)
                        .keys(keys)
                        .opened(open_row)
                        .bar(true)
                        .stack_layout(StackLayout::Labeled)
                        .actions(|_| {
                            vec![
                                MenuItem::action("Show on the map"),
                                MenuItem::action("Simulate trouble here").icon(Icon::Activity),
                            ]
                        })
                        .footer(false)
                        .stretch(true)
                        .show(ui);
                    if let Some((row, which)) = output.action {
                        chosen = Some(listed[row]);
                        troubled = which == 1;
                    }
                    // a press on a site chooses it, as the web's `onOpen`
                    if let Some(row) = table.opened.take() {
                        chosen = Some(listed[row]);
                    }
                },
            );
        if let Some(incident) = to_incident {
            self.go(&format!("incidents/{incident}"));
            return;
        }
        if let Some(site) = chosen {
            if troubled {
                let now = (self.start + self.elapsed) as f64 * 1000.0;
                self.live_world.trigger(live::ScenarioKind::Region, Some(site), now);
            }
            self.selected_site = Some(site);
            let s = &data::SITES[site];
            self.sites_map.selected = Some(s.id.to_owned());
            self.sites_map
                .frame(Some([s.at[0] - 6.0, s.at[1] - 6.0, s.at[0] + 6.0, s.at[1] + 6.0]));
        }
    }

    /// One service: its head with its host, what it does and its state, the way back to the
    /// services and its actions; its figures; its charts, graph and uptime as they are ported.
    fn service_page(&mut self, ui: &mut Ui, id: &str) {
        let index = SERVICES.iter().position(|s| s.id == id).unwrap_or(0);
        let service = SERVICES[index];
        let t = sokkel_egui::tokens(ui.ctx());
        let days = self.uptime().swap_remove(index);
        let overall = overall_uptime(&days);
        let health = self.live_world.health[index];
        let last = self.live_world.readings[index].last().copied();
        let mut report = false;
        let (mut deploy, mut schedule) = (false, false);
        let head = PageHeader::new(service.id)
            .back("Services")
            .code(service.host)
            .description(service.description)
            .chip(health.words().to_lowercase(), health_tone(health))
            .show(ui, |ui| {
                deploy = ui.add(Button::new("Deploy").icon(Icon::Rocket)).clicked();
                schedule = ui
                    .add(
                        Button::new("Schedule maintenance")
                            .icon(Icon::CalendarClock)
                            .variant(ButtonVariant::Outline),
                    )
                    .clicked();
                report = ui
                    .add(
                        Button::new("Report incident")
                            .icon(Icon::Siren)
                            .variant(ButtonVariant::Outline),
                    )
                    .clicked();
            });
        if head.back {
            self.go("services");
        }
        if report {
            self.report_incident(Some(service.id));
        }
        if schedule {
            self.maintenance_form = Some(dialogs::MaintenanceForm::new(service.id, self.today));
        }
        if deploy {
            self.deploy_form = Some(dialogs::DeployForm::new(service.id, &self.running(service.id)));
        }
        let value = |f: &dyn Fn(live::Reading) -> String| last.map_or("-".to_owned(), f);
        KpiRow::new([
            Stat::new("State", health.words()),
            Stat::new("Latency p95", value(&|r| format!("{}", r.p95.round())))
                .unit("ms")
                .note(format!("usually {} ms", service.latency.round())),
            Stat::new("Errors", value(&|r| rate(r.errors))).note(format!("usually {}", rate(service.errors))),
            Stat::new("Requests", value(&|r| group(r.load.round() as u64))).unit("/s"),
            Stat::new("Uptime, 90 days", overall.map_or("-".to_owned(), format_uptime)),
            Stat::new("Running", self.running(service.id)),
        ])
        .show(ui);
        let readings = &self.live_world.readings[index];
        let x: Vec<String> = readings.iter().map(|r| clock_of(r.at)).collect();
        let warning = t.color("warning");
        let critical = t.color("critical");
        let picked = self.service_day;
        let on_day: Vec<TimelineEvent> = picked
            .map(|day| {
                let back = f64::from(self.today.0 - day.0);
                let from = -back * 24.0 * 3600.0 * 1000.0;
                let to = from + 24.0 * 3600.0 * 1000.0;
                let now = ((self.start + self.elapsed) as f64) * 1000.0;
                self.incidents
                    .iter()
                    .filter(|i| i.services.contains(&id) && i.opened < to && i.resolved.unwrap_or(now) > from)
                    .map(|i| TimelineEvent {
                        tone: Some(t.color(state_color(i.state))),
                        title: format!("{} {}", i.code, i.title),
                        at: day.medium(),
                        detail: Some(format!(
                            "{}, {}",
                            severity_words(i.severity).to_lowercase(),
                            state_words(i.state).to_lowercase()
                        )),
                        ..Default::default()
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut pressed = None;
        // the service's deploys, newest first, and its windows, latest first, as the web's
        let now = (self.start + self.elapsed) as f64 * 1000.0;
        let running = self.running(id);
        let mut mine: Vec<&data::Deploy> = self.deploys.iter().filter(|d| d.service == id).collect();
        mine.sort_by(|a, b| b.at.total_cmp(&a.at));
        let deploy_rows: Vec<Vec<Cell>> = mine
            .iter()
            .map(|d| {
                vec![
                    Cell::Mono(d.version.clone()),
                    deploy_chip(d.state),
                    Cell::Text(if d.note.is_empty() { "-".into() } else { d.note.clone() }),
                    Cell::Person(d.by.into()),
                    Cell::Relative { at: d.at, now },
                ]
            })
            .collect();
        let deploy_menus: Vec<Option<String>> = mine
            .iter()
            .map(|d| {
                (d.state == data::DeployState::Succeeded && d.version == running)
                    .then(|| data::previous_version(&d.version))
            })
            .collect();
        let deploy_keys: Vec<String> = mine.iter().map(|d| d.id.clone()).collect();
        let mut windows: Vec<&data::Maintenance> = self.maintenance.iter().filter(|w| w.service == id).collect();
        windows.sort_by(|a, b| b.starts.total_cmp(&a.starts));
        let window_states: Vec<WindowState> = windows.iter().map(|w| window_state(w, now)).collect();
        let window_rows: Vec<Vec<Cell>> = windows
            .iter()
            .zip(&window_states)
            .map(|(w, state)| {
                let ends = self.moment(w.ends);
                vec![
                    Cell::Text(w.title.clone()),
                    state.chip(),
                    Cell::Text(format!("{} to {}", self.moment(w.starts), &ends[ends.len() - 5..])),
                ]
            })
            .collect();
        let window_keys: Vec<String> = windows.iter().map(|w| w.id.clone()).collect();
        let scheduled = window_states.iter().filter(|s| **s == WindowState::Scheduled).count();
        let (deploys_table, windows_table) = (&mut self.deploys_table, &mut self.windows_table);
        let mut deploy_action = None;
        let mut window_action = None;
        let mut schedule_more = false;
        let (deploy_menus_r, deploy_keys_r, window_keys_r, window_states_r) =
            (&deploy_menus, &deploy_keys, &window_keys, &window_states);
        let mut graph_to = None;
        let (graph_nodes, graph_links) = service_graph(&self.live_world.health, &t);
        DashboardGrid::new("service").show(ui, |grid| {
            let x2 = x.clone();
            grid.cell(PanelSpan::Half, move |ui| {
                TimeSeriesPanel::new("Latency", x2)
                    .subtitle("live, in milliseconds")
                    .series(Series::new("p95", readings.iter().map(|r| r.p95.round())))
                    .series(Series::new("p50", readings.iter().map(|r| r.p50.round())))
                    .unit("ms")
                    .threshold((service.latency * 1.8).round(), warning)
                    .threshold((service.latency * 3.5).round(), critical)
                    .label_every(15)
                    .show(ui);
            });
            grid.cell(PanelSpan::Half, move |ui| {
                TimeSeriesPanel::new("Errors", x)
                    .subtitle("live, share of requests")
                    .series(Series::new(
                        "Errors",
                        readings.iter().map(|r| (r.errors * 100.0).round() / 100.0),
                    ))
                    .unit("%")
                    .decimals(2)
                    .threshold(1.5, warning)
                    .threshold(5.0, critical)
                    .label_every(15)
                    .show(ui);
            });
            let graph_to = &mut graph_to;
            let (graph_nodes, graph_links) = (&graph_nodes, &graph_links);
            grid.cell(PanelSpan::Half, move |ui| {
                DashboardPanel::new("Dependencies")
                    .subtitle("every service, this one marked")
                    .flush(true)
                    .footer(dependencies_footer(&service))
                    .show(ui, |ui| {
                        *graph_to = GraphCanvas::new("services", graph_nodes, graph_links)
                            .selected(Some(service.id))
                            .words(GraphWords {
                                label: "Services and what they call on".into(),
                                ..Default::default()
                            })
                            .show(ui);
                    });
            });
            let pressed = &mut pressed;
            let days = &days;
            grid.cell(PanelSpan::Half, move |ui| {
                let subtitle = overall.map_or("the last 90 days".to_owned(), |o| {
                    format!("{} over 90 days", format_uptime(o))
                });
                DashboardPanel::new("Uptime").subtitle(subtitle).show(ui, |ui| {
                    Stack::new(Gap::Lg).show(ui, |ui| {
                        *pressed = UptimeBar::new(("service", id), days).show(ui).map(|d| d.date);
                        match picked {
                            None => {
                                MetaLine::new(["Press a day to see what happened on it."]).show(ui);
                            }
                            Some(day) if on_day.is_empty() => {
                                MetaLine::new([format!("Nothing happened to {id} on {}.", day.medium())]).show(ui);
                            }
                            Some(_) => Timeline::new(on_day).show(ui),
                        }
                    });
                });
            });
            let deploy_action = &mut deploy_action;
            grid.cell(PanelSpan::Half, move |ui| {
                DashboardPanel::new("Deploys")
                    .subtitle(format!("{running} running"))
                    .flush(true)
                    .show(ui, |ui| {
                        let columns = [
                            Column::new("Version", 104.0),
                            Column::new("State", 120.0).stack(StackPlace::Secondary),
                            Column::new("What it changed", 128.0).fill().stack(StackPlace::Primary),
                            Column::new("By", 104.0).stack(StackPlace::Secondary),
                            Column::new("When", 120.0).stack(StackPlace::Secondary),
                        ];
                        let output = DataTable::new("deploys", &columns, &deploy_rows, deploys_table)
                            .fits(true)
                            .keys(deploy_keys_r.to_vec())
                            .variant(TableVariant::List)
                            .stack_layout(StackLayout::Labeled)
                            .stack_below(420.0)
                            .actions(|row| {
                                deploy_menus_r[row]
                                    .iter()
                                    .map(|back| MenuItem::action(format!("Roll back to {back}")).icon(Icon::Undo2))
                                    .collect()
                            })
                            .words(sokkel_egui::TableWords {
                                empty: "No deploys yet.".into(),
                                ..Default::default()
                            })
                            .footer(false)
                            .show(ui);
                        if let Some((row, _)) = output.action {
                            *deploy_action = Some(row);
                        }
                    });
            });
            let window_action = &mut window_action;
            let schedule_more = &mut schedule_more;
            grid.cell(PanelSpan::Half, move |ui| {
                DashboardPanel::new("Maintenance")
                    .subtitle(format!("{scheduled} scheduled"))
                    .flush(true)
                    .actions(|ui| {
                        *schedule_more = ui
                            .add(
                                Button::new("Schedule")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm),
                            )
                            .clicked();
                    })
                    .show(ui, |ui| {
                        let columns = [
                            Column::new("Work", 128.0).fill(),
                            Column::new("State", 128.0).stack(StackPlace::Secondary),
                            Column::new("When", 192.0).stack(StackPlace::Secondary),
                        ];
                        let output = DataTable::new("windows", &columns, &window_rows, windows_table)
                            .fits(true)
                            .keys(window_keys_r.to_vec())
                            .variant(TableVariant::List)
                            .stack_layout(StackLayout::Labeled)
                            .stack_below(420.0)
                            .actions(|row| window_states_r[row].actions())
                            .words(sokkel_egui::TableWords {
                                empty: "Nothing scheduled.".into(),
                                ..Default::default()
                            })
                            .footer(false)
                            .show(ui);
                        if let Some((row, which)) = output.action {
                            *window_action = Some((row, which));
                        }
                    });
            });
        });
        if let Some(next) = graph_to
            && next != id
        {
            self.go(&format!("services/{next}"));
        }
        if let Some(row) = deploy_action
            && let Some(back) = &deploy_menus[row]
        {
            let made = data::Deploy {
                id: format!("deploy-{id}-new-{}", self.deploys.len()),
                service: service.id,
                version: back.clone(),
                at: now,
                by: ME,
                state: data::DeployState::Running,
                note: format!("Rolled back from {}", mine_version(&deploy_keys[row], &self.deploys)),
            };
            self.deploys.push(made);
        }
        if schedule_more {
            self.maintenance_form = Some(dialogs::MaintenanceForm::new(service.id, self.today));
        }
        if let Some((row, which)) = window_action
            && let Some(key) = window_keys.get(row)
            && let Some(window) = self.maintenance.iter_mut().find(|w| &w.id == key)
        {
            match (window_state(window, now), which) {
                // start now, for two minutes
                (WindowState::Scheduled, 0) => {
                    window.starts = now;
                    window.ends = now + 2.0 * 60_000.0;
                }
                (WindowState::Scheduled, _) => window.cancelled = true,
                (WindowState::InProgress, _) => window.ends = now,
                _ => {
                    let key = key.clone();
                    self.maintenance.retain(|w| w.id != key);
                }
            }
        }
        if let Some(day) = pressed {
            self.go(&format!("services/{id}/{day}"));
        }
    }

    /// On call: who answers now, the escalation and the next week; the rotation, each week's person
    /// chosen in place; and the handovers beside them.
    fn on_call_page(&mut self, ui: &mut Ui) {
        PageHeader::new("On call")
            .description("Who answers when something goes wrong, and the weeks ahead.")
            .show(ui, |ui| {
                if ui.add(Button::new("Hand over").icon(Icon::ArrowRightLeft)).clicked() {
                    self.hand_over = Some(dialogs::HandOverForm::new(&self.on_call));
                }
            });
        let today = self.today;
        let current = self.shifts.iter().position(|s| s.starts <= today && today < s.ends);
        let next = self.shifts.iter().find(|s| s.starts > today).cloned();
        // the backup is whoever had last week, unless that is who has the pager now or next
        let before = current.and_then(|i| i.checked_sub(1)).map(|i| self.shifts[i].person);
        let now = self.on_call.clone();
        let escalates = match before {
            Some(b) if b != now && Some(b) != next.as_ref().map(|n| n.person) => Some(b),
            _ => data::PEOPLE
                .iter()
                .copied()
                .find(|p| *p != now && Some(*p) != next.as_ref().map(|n| n.person)),
        };
        let handovers: Vec<TimelineEvent> = self
            .handovers
            .iter()
            .map(|h| TimelineEvent {
                icon: Some(Icon::PhoneCall),
                title: format!("{} to {}", h.from, h.to),
                at: {
                    let words = relative_time(h.at - self.start as f64 * 1000.0);
                    if words == "now" { "now".into() } else { words }
                },
                detail: Some(h.note.clone()),
                ..Default::default()
            })
            .collect();
        let handover_count = self.handovers.len().to_string();
        let since = format!("{}, 09:00", today.plus(-(today.weekday() as i32)).with_weekday());
        Split::new(AsideWidth::Lg).show(
            ui,
            |ui| {
                Stack::new(Gap::Section).show(ui, |ui| {
                    Section::new("Now")
                        .description(if self.on_call_note.is_empty() {
                            "Nothing was said at the handover.".to_owned()
                        } else {
                            format!("From the handover: {}", self.on_call_note)
                        })
                        .show(ui, |ui| {
                            Stack::new(Gap::Md).show(ui, |ui| {
                                PersonRow::new(&now)
                                    .detail(format!("on call since {since}"))
                                    .show(ui, |ui| chip(ui, "on call", ChipTone::Good));
                                if let Some(person) = escalates {
                                    PersonRow::new(person)
                                        .detail("takes it if nobody answers in 15 minutes")
                                        .show(ui, |ui| chip(ui, "escalation", ChipTone::Status("waiting")));
                                }
                                if let Some(next) = &next {
                                    PersonRow::new(next.person)
                                        .detail(format!("on call from {}", next.starts.with_weekday()))
                                        .show(ui, |ui| chip(ui, "next week", ChipTone::Status("queued")));
                                }
                            });
                        });
                    Section::new("Rotation")
                        .description("A week each, Monday to Monday. Press a name to change who takes the week.")
                        .flush(true)
                        .show(ui, |ui| self.rotation(ui));
                });
            },
            |ui| {
                Section::new("Handovers")
                    .meta(handover_count)
                    .show(ui, |ui| Timeline::new(handovers).show(ui));
            },
        );
    }

    /// The rotation week by week, each week's person chosen in place.
    fn rotation(&mut self, ui: &mut Ui) {
        let today = self.today;
        let columns = [
            Column::new("Week", 192.0).fill(),
            Column::new("When", 128.0).stack(StackPlace::Secondary),
            Column::new("On call", 160.0).editable(Editor::Select(data::PEOPLE.map(String::from).to_vec())),
        ];
        let rows: Vec<Vec<Cell>> = self
            .shifts
            .iter()
            .map(|s| {
                let when = if s.starts <= today && today < s.ends {
                    Cell::Chip("this week".into(), ChipTone::Good)
                } else if s.ends <= today {
                    Cell::Text("past".into())
                } else {
                    Cell::Text("ahead".into())
                };
                vec![
                    Cell::Text(format!(
                        "{} to {}",
                        s.starts.with_weekday(),
                        s.ends.plus(-1).with_weekday()
                    )),
                    when,
                    Cell::Person(s.person.into()),
                ]
            })
            .collect();
        let keys: Vec<String> = self.shifts.iter().map(|s| s.id.clone()).collect();
        let output = DataTable::new("rotation", &columns, &rows, &mut self.rotation_table)
            .fits(true)
            .keys(keys)
            .variant(TableVariant::List)
            .stack_layout(StackLayout::Labeled)
            // every week shown, as the web's table holds them all
            .height(f32::INFINITY)
            .footer(false)
            .today(today)
            .show(ui);
        if let Some(edit) = output.edited
            && let Some(person) = data::PEOPLE.iter().find(|p| **p == edit.value)
        {
            self.shifts[edit.row].person = person;
        }
    }

    /// Each service as a tile: its state, its latency now with its last readings as a line, and its
    /// uptime (none yet: the incident history it is read from is still to port).
    fn tiles(&self, ui: &mut Ui) -> Option<&'static str> {
        let t = sokkel_egui::tokens(ui.ctx());
        let mut opened = None;
        let uptime = self.uptime();
        Columns::new(4)
            .min(160.0)
            .gap(12.0)
            .show(ui, SERVICES.len(), |ui, at, width| {
                // the tiles stand in the person's order, as the services table does
                let index = self.service_order[at];
                let service = SERVICES[index];
                let health = self.live_world.health[index];
                let readings = &self.live_world.readings[index];
                let last = readings.last().map_or("-".to_owned(), |r| format!("{}", r.p95.round()));
                let stat = Stat::new(service.id, last)
                    .dot(t.color(health.color()))
                    .emphasis(health != live::Health::Operational)
                    .unit("ms p95")
                    .note(format!(
                        "{} · {} over 90 days",
                        health.words(),
                        overall_uptime(&uptime[index]).map_or("no data".to_owned(), format_uptime)
                    ))
                    .trend(readings.iter().map(|r| r.p95), t.color(health.tone()))
                    .selectable(true);
                if stat.tile(ui, width).clicked() {
                    opened = Some(service.id);
                }
            });
        opened
    }
}

/// An incident's state in words, as the web's status map says it.
/// An incident's state as a sentence names it, as the web's `INCIDENT_WORDS`.
fn state_name(state: data::State) -> &'static str {
    match state {
        data::State::Investigating => "Investigating",
        data::State::Identified => "Identified",
        data::State::Monitoring => "Monitoring",
        data::State::Resolved => "Resolved",
    }
}

fn state_words(state: data::State) -> &'static str {
    match state {
        data::State::Investigating => "investigating",
        data::State::Identified => "identified",
        data::State::Monitoring => "monitoring",
        data::State::Resolved => "resolved",
    }
}

/// An incident state's tone, as the web's `STATUSES` colors it.
/// An incident state's color token, as the board's status map gives it.
fn state_color(state: data::State) -> &'static str {
    match state {
        data::State::Investigating => "critical",
        data::State::Identified => "warning",
        data::State::Monitoring => "info",
        data::State::Resolved => "good",
    }
}

fn state_tone(state: data::State) -> ChipTone {
    match state {
        data::State::Investigating => ChipTone::Critical,
        data::State::Identified => ChipTone::Warning,
        data::State::Monitoring => ChipTone::Info,
        data::State::Resolved => ChipTone::Good,
    }
}

fn severity_words(severity: data::Severity) -> &'static str {
    match severity {
        data::Severity::Minor => "minor",
        data::Severity::Major => "major",
        data::Severity::Critical => "critical",
    }
}

fn severity_tone(severity: data::Severity) -> ChipTone {
    match severity {
        data::Severity::Minor => ChipTone::Warning,
        _ => ChipTone::Critical,
    }
}

/// How long something lasted, as the web's `formatDuration`: "48 min", "2 h 10 min", "3 days".
fn duration(ms: f64) -> String {
    let minutes = (ms / 60_000.0).round().max(1.0) as u64;
    if minutes < 60 {
        return format!("{minutes} min");
    }
    let hours = minutes / 60;
    if hours < 48 {
        return if minutes.is_multiple_of(60) {
            format!("{hours} h")
        } else {
            format!("{hours} h {} min", minutes % 60)
        };
    }
    format!("{} days", ((hours as f64) / 24.0).round())
}

/// A service's health as a chip's tone, as the web's `HEALTH_COLOR`.
fn health_tone(health: live::Health) -> ChipTone {
    match health {
        live::Health::Operational => ChipTone::Good,
        live::Health::Degraded => ChipTone::Warning,
        live::Health::Outage => ChipTone::Critical,
        live::Health::Maintenance => ChipTone::Info,
    }
}

/// A share in percent, as the web's `formatRate`: two decimals under one, one above.
fn rate(value: f64) -> String {
    if value < 1.0 {
        format!("{value:.2}%")
    } else {
        format!("{value:.1}%")
    }
}

/// A chip at the end of a row, as `StatusChip` draws a state.
fn chip(ui: &mut Ui, label: &str, tone: ChipTone) {
    ui.add(sokkel_egui::Chip::new(label).tone(tone));
}

/// A count with its thousands set apart, as the web's `formatCount`: 7,230.
fn group(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl eframe::App for StatusApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        // a reading every interval of the window's clock while live, as the web's board
        let now = ui.ctx().input(|i| i.time);
        let last = *self.last_tick.get_or_insert(now);
        let interval = f64::from(self.options.interval);
        if self.live && now - last >= interval {
            self.tick();
            self.last_tick = Some(now);
        } else if !self.live {
            self.last_tick = Some(now);
        }
        if self.live {
            let wait = (interval - (now - last)).max(0.05);
            ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(wait));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }
}

/// A time of the board's clock, in milliseconds, as `HH:MM:SS`.
fn clock_of(at: f64) -> String {
    let at = (at / 1000.0) as u64;
    format!("{:02}:{:02}:{:02}", at / 3600 % 24, at / 60 % 60, at % 60)
}

/// The box around some sites for the map to frame, `[west, south, east, north]`: a region's with
/// a margin of 6 degrees as the web's `boundsOf`, every site's as they lie.
fn bounds_of(sites: &[&data::Site]) -> [f64; 4] {
    let margin = if sites.len() == data::SITES.len() { 0.0 } else { 6.0 };
    let lons = sites.iter().map(|s| s.at[0]);
    let lats = sites.iter().map(|s| s.at[1]);
    [
        lons.clone().fold(f64::INFINITY, f64::min) - margin,
        lats.clone().fold(f64::INFINITY, f64::min) - margin,
        lons.fold(f64::NEG_INFINITY, f64::max) + margin,
        lats.fold(f64::NEG_INFINITY, f64::max) + margin,
    ]
}

/// An incident's cells in the incidents table's columns: its title, code, state, severity, what
/// it touches, its assignee, when it opened and how long it lasted.
fn incident_cells(incident: &data::Incident, now: f64) -> Vec<Cell> {
    let mut touched: Vec<String> = incident.services.iter().map(|s| s.to_string()).collect();
    touched.extend(incident.sites.iter().map(|id| {
        data::SITES
            .iter()
            .find(|s| s.id == *id)
            .map_or(*id, |s| s.city)
            .to_string()
    }));
    let lasted = incident.resolved.unwrap_or(now) - incident.opened;
    vec![
        Cell::Text(incident.title.clone()),
        Cell::Mono(incident.code.clone()),
        Cell::Chip(state_words(incident.state).into(), state_tone(incident.state)),
        Cell::Chip(
            severity_words(incident.severity).into(),
            severity_tone(incident.severity),
        ),
        Cell::Tags(touched),
        Cell::Person(incident.assignee.into()),
        Cell::Relative {
            at: incident.opened,
            now,
        },
        Cell::Text(duration(lasted)),
    ]
}

/// Each service over its last ninety days, as the web's `UptimeList`: its name and what it is, its
/// state now, and a bar a day under them, 24 between services. Returns the service and day pressed.
fn uptime_list(
    ui: &mut Ui,
    order: &[usize],
    health: &[live::Health],
    uptime: &[Vec<UptimeDay>],
) -> Option<(&'static str, Day)> {
    let t = sokkel_egui::tokens(ui.ctx());
    let mut pressed = None;
    Stack::new(Gap::Xl).show(ui, |ui| {
        for &index in order {
            let service = SERVICES[index];
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.label(
                        TextRole::Body
                            .rich(&t, service.id)
                            .variation("wght", t.weight(sokkel_egui::Weight::Medium)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let h = health[index];
                        ui.add(sokkel_egui::Chip::new(h.words().to_lowercase()).tone(health_tone(h)));
                        // what it is, beside its name, giving way first where the line is short
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    TextRole::Help
                                        .rich(&t, service.description)
                                        .color(t.color("muted-foreground")),
                                )
                                .truncate(),
                            );
                        });
                    });
                });
                if let Some(day) = UptimeBar::new(("overview", service.id), &uptime[index]).show(ui) {
                    pressed = Some((service.id, day.date));
                }
            });
        }
    });
    pressed
}

/// An address subscribed on the public page: every service, or the ones named.
#[derive(Clone, Debug, PartialEq)]
pub struct Subscriber {
    pub email: String,
    pub services: Vec<&'static str>,
    /// When they subscribed, in milliseconds of the board's clock.
    pub at: f64,
}

/// The public page's form: the address being written, every service or some, the ones ticked, what
/// is wrong with it, and the subscribers it has made.
pub struct Subscribe {
    pub email: String,
    pub which: String,
    pub picked: Vec<bool>,
    pub error: Option<String>,
    pub subscribers: Vec<Subscriber>,
    /// The subscriber just made, shown in the form's place.
    pub done: Option<Subscriber>,
    /// The page scrolls to the form on its next frame.
    pub reveal: bool,
    /// The board's clock, which a subscription made now is dated by.
    pub now: f64,
}

impl Default for Subscribe {
    fn default() -> Self {
        Self {
            email: String::new(),
            which: "all".into(),
            picked: vec![false; SERVICES.len()],
            error: None,
            subscribers: Vec::new(),
            done: None,
            reveal: false,
            now: 0.0,
        }
    }
}

impl Subscribe {
    /// Subscribe the address written, as the web's `SubscribeForm` checks it.
    pub fn submit(&mut self) {
        let address = self.email.trim().to_lowercase();
        let looks_right = {
            let mut parts = address.split('@');
            let (local, domain) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
            !local.is_empty()
                && parts.next().is_none()
                && !address.contains(char::is_whitespace)
                && domain.split('.').count() >= 2
                && domain.split('.').all(|piece| !piece.is_empty())
        };
        if !looks_right {
            self.error = Some("Write an address such as you@example.org.".into());
            return;
        }
        if self.subscribers.iter().any(|s| s.email == address) {
            self.error = Some(format!("{address} is subscribed already."));
            return;
        }
        let services: Vec<&'static str> = if self.which == "all" {
            Vec::new()
        } else {
            SERVICES
                .iter()
                .zip(&self.picked)
                .filter(|(_, ticked)| **ticked)
                .map(|(s, _)| s.id)
                .collect()
        };
        if self.which == "some" && services.is_empty() {
            self.error = Some("Tick at least one service, or choose every service.".into());
            return;
        }
        let made = Subscriber {
            email: address,
            services,
            at: self.now,
        };
        self.subscribers.push(made.clone());
        self.done = Some(made);
        self.email.clear();
        self.picked = vec![false; SERVICES.len()];
    }
}

/// The public page's "Get notified": an address, every service or some, and Subscribe; once
/// subscribed, the address said back with Unsubscribe and Subscribe another.
fn subscribe_form(ui: &mut Ui, form: &mut Subscribe) {
    if form.reveal {
        ui.scroll_to_cursor(Some(egui::Align::TOP));
        form.reveal = false;
    }
    if let Some(done) = form.done.clone() {
        Section::new("Get notified")
            .description(format!(
                "{} hears of every new incident and update by email.",
                done.email
            ))
            .show(ui, |ui| {
                Stack::new(Gap::Lg).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.add(sokkel_egui::Chip::new("subscribed").tone(ChipTone::Good));
                        let which = if done.services.is_empty() {
                            "every service".to_owned()
                        } else {
                            done.services.join(", ")
                        };
                        MetaLine::new([which]).show(ui);
                    });
                    form_bar(ui, None, |ui| {
                        if ui
                            .add(Button::new("Subscribe another").variant(ButtonVariant::Outline))
                            .clicked()
                        {
                            form.done = None;
                        }
                        if ui
                            .add(Button::new("Unsubscribe").variant(ButtonVariant::Ghost))
                            .clicked()
                        {
                            form.subscribers.retain(|s| *s != done);
                            form.done = None;
                        }
                    });
                });
            });
        return;
    }
    Section::new("Get notified")
        .description("An email when an incident opens, with each update until it is resolved.")
        .show(ui, |ui| {
            Stack::new(Gap::Lg).show(ui, |ui| {
                let before = form.email.clone();
                ui.add(
                    sokkel_egui::TextField::new("Email", &mut form.email)
                        .placeholder("you@example.org")
                        .error(form.error.clone()),
                );
                if form.email != before {
                    form.error = None;
                }
                let choices = [
                    Choice {
                        value: "all",
                        label: "Every service",
                        description: None,
                    },
                    Choice {
                        value: "some",
                        label: "Some services",
                        description: Some("Only the ones ticked below."),
                    },
                ];
                if ChoiceList::new("Hear about", &choices).show(ui, &mut form.which) {
                    form.error = None;
                }
                if form.which == "some" {
                    let picked = &mut form.picked;
                    let error = &mut form.error;
                    checkbox_list(ui, "Services", |ui| {
                        Columns::new(4)
                            .min(120.0)
                            .gap(0.0)
                            .show(ui, SERVICES.len(), |ui, i, _| {
                                if ui
                                    .add(sokkel_egui::Checkbox::new(&mut picked[i], SERVICES[i].id))
                                    .changed()
                                {
                                    *error = None;
                                }
                            });
                    });
                }
                let mut submit = false;
                form_bar(
                    ui,
                    Some("No more than one email an update. Unsubscribe from any of them."),
                    |ui| {
                        submit = ui.add(Button::new("Subscribe")).clicked();
                    },
                );
                if submit {
                    form.submit();
                }
            });
        });
}

/// An incident state's place in the board's order: investigating, identified, monitoring, resolved.
fn state_index(state: data::State) -> usize {
    match state {
        data::State::Investigating => 0,
        data::State::Identified => 1,
        data::State::Monitoring => 2,
        data::State::Resolved => 3,
    }
}

/// A moment of the board's clock, its midnight on `today` the clock's zero, as `Mon, Nov 16, 08:51`.
fn moment_of(today: Day, at: f64, twelve: bool) -> String {
    let day = today.plus((at / 86_400_000.0).floor() as i32);
    let seconds = (at.rem_euclid(86_400_000.0) / 1000.0) as u32;
    format!("{}, {}", day.with_weekday(), time_of_day(seconds, twelve, false))
}

/// A time of day as the web's `formatClock`: `14:02:11` on a 24-hour clock, `2:02:11 PM` on a
/// 12-hour one; without seconds `14:02` or `2:02 PM`.
fn time_of_day(seconds: u32, twelve: bool, with_seconds: bool) -> String {
    let (hours, minutes, secs) = (seconds / 3600 % 24, seconds / 60 % 60, seconds % 60);
    let tail = if with_seconds {
        format!(":{secs:02}")
    } else {
        String::new()
    };
    if twelve {
        let hour = if hours % 12 == 0 { 12 } else { hours % 12 };
        let half = if hours < 12 { "AM" } else { "PM" };
        format!("{hour}:{minutes:02}{tail} {half}")
    } else {
        format!("{hours:02}:{minutes:02}{tail}")
    }
}

/// The version a deploy put out, by its id.
fn mine_version(id: &str, deploys: &[data::Deploy]) -> String {
    deploys
        .iter()
        .find(|d| d.id == id)
        .map_or_else(String::new, |d| d.version.clone())
}

/// Every panel the overview can show, as the web's `WIDGETS`: its id, label, icon and width.
const WIDGETS: [(&str, &str, Icon, PanelSpan); 9] = [
    ("sites", "Sites", Icon::Globe, PanelSpan::TwoThirds),
    ("open", "Open incidents", Icon::Siren, PanelSpan::Third),
    ("uptime", "Uptime", Icon::Activity, PanelSpan::Full),
    ("latency", "Latency", Icon::Gauge, PanelSpan::Half),
    ("requests", "Requests", Icon::Waves, PanelSpan::Half),
    ("errors", "Errors", Icon::TriangleAlert, PanelSpan::Half),
    ("on-call", "On call", Icon::PhoneCall, PanelSpan::Third),
    ("maintenance", "Maintenance", Icon::CalendarClock, PanelSpan::Third),
    ("deploys", "Deploys", Icon::Rocket, PanelSpan::Third),
];

/// What a panel is called, by its id.
fn widget_label(id: &str) -> &'static str {
    WIDGETS.iter().find(|(each, ..)| *each == id).map_or("Panel", |w| w.1)
}

/// The overview as it comes: the map beside what is open, then the last ninety days.
pub fn default_layout() -> Vec<DashboardLayoutItem> {
    WIDGETS[..3]
        .iter()
        .map(|(id, _, _, span)| DashboardLayoutItem::new(*id, *span))
        .collect()
}

/// A deploy's state as its chip, in the status map's colors.
fn deploy_chip(state: data::DeployState) -> Cell {
    match state {
        data::DeployState::Running => Cell::Chip("running".into(), ChipTone::Status("running")),
        data::DeployState::Succeeded => Cell::Chip("succeeded".into(), ChipTone::Status("succeeded")),
        data::DeployState::Failed => Cell::Chip("failed".into(), ChipTone::Status("failed")),
    }
}

/// Where a maintenance window stands now, as the web's `windowState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowState {
    Scheduled,
    InProgress,
    Completed,
    Cancelled,
}

impl WindowState {
    fn chip(self) -> Cell {
        match self {
            Self::Scheduled => Cell::Chip("scheduled".into(), ChipTone::Info),
            Self::InProgress => Cell::Chip("in progress".into(), ChipTone::Status("running")),
            Self::Completed => Cell::Chip("completed".into(), ChipTone::Status("succeeded")),
            Self::Cancelled => Cell::Chip("cancelled".into(), ChipTone::Status("cancelled")),
        }
    }

    /// The window's menu by where it stands, as the web's `windowActions`.
    fn actions(self) -> Vec<MenuItem> {
        match self {
            Self::Scheduled => vec![
                MenuItem::action("Start now, for two minutes").icon(Icon::Play),
                MenuItem::action("Cancel").icon(Icon::X).destructive(),
            ],
            Self::InProgress => vec![MenuItem::action("End now").icon(Icon::CircleStop)],
            _ => vec![MenuItem::action("Remove").icon(Icon::X).destructive()],
        }
    }
}

fn window_state(window: &data::Maintenance, now: f64) -> WindowState {
    if window.cancelled {
        WindowState::Cancelled
    } else if window.going_on(now) {
        WindowState::InProgress
    } else if window.ends <= now {
        WindowState::Completed
    } else {
        WindowState::Scheduled
    }
}

/// Every service and what it calls on, each in its state now, as the web's `ServiceGraph`: a store
/// round with a cylinder, a service a box with a server.
fn service_graph(health: &[live::Health], t: &sokkel_egui::Tokens) -> (Vec<GraphNode>, Vec<GraphLink>) {
    let nodes = SERVICES
        .iter()
        .zip(health)
        .map(|(service, health)| {
            GraphNode::new(service.id)
                .kind(if service.store { "store" } else { "service" })
                .icon(if service.store { Icon::Database } else { Icon::Server })
                .round(service.store)
                .status(health.words().to_lowercase(), t.color(health.color()))
        })
        .collect();
    let links = SERVICES
        .iter()
        .flat_map(|service| {
            service
                .depends_on
                .iter()
                .map(|dependency| GraphLink::new(service.id, *dependency).arrow(true))
        })
        .collect();
    (nodes, links)
}

/// Under the graph: what the service calls on and what calls on it.
fn dependencies_footer(service: &data::Service) -> String {
    let calls_on = if service.depends_on.is_empty() {
        "nothing".to_owned()
    } else {
        service.depends_on.join(", ")
    };
    let called_by: Vec<&str> = SERVICES
        .iter()
        .filter(|one| one.depends_on.contains(&service.id))
        .map(|one| one.id)
        .collect();
    let called_by = if called_by.is_empty() {
        "nothing calls on it".to_owned()
    } else {
        format!("called by {}", called_by.join(", "))
    };
    MetaLine::new([format!("Calls on {calls_on}"), called_by]).text()
}

/// What a site's card on the map says, made before the map is drawn.
struct SiteCard {
    id: &'static str,
    city: &'static str,
    place: String,
    state: &'static str,
    color: egui::Color32,
    now: String,
    usually: String,
    /// The open incident covering the site: its code and its id.
    incident: Option<(String, String)>,
}

/// The card of the site chosen on `map`, after the map is drawn; the incident its action opens.
fn site_card(ui: &mut Ui, map: &mut MapView, cards: &[SiteCard]) -> Option<String> {
    let card = cards.iter().find(|c| map.selected.as_deref() == Some(c.id))?;
    let mut popup = MapPopup::new(card.city)
        .description(card.place.clone())
        .row(MapPopupRow::new("State", card.state).color(card.color))
        .row(MapPopupRow::new("Round trip", card.now.clone()))
        .row(MapPopupRow::new("Usually", card.usually.clone()));
    if let Some((code, _)) = &card.incident {
        popup = popup
            .row(MapPopupRow::new("Incident", code.clone()))
            .action(format!("Open {code}"));
    }
    let answer = popup.show(ui, map);
    if answer.acted {
        return card.incident.as_ref().map(|(_, id)| id.clone());
    }
    None
}

/// Milliseconds as the web writes them: whole, grouped, with their unit.
fn format_ms(ms: f64) -> String {
    let whole = ms.round() as i64;
    let digits = whole.abs().to_string();
    let mut grouped = String::new();
    for (k, c) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{}{grouped} ms", if whole < 0 { "-" } else { "" })
}
