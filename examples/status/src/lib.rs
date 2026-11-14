//! sokkel-ui's status example ported one to one (`examples/status/web/src` in sokkel-ui): a live
//! board of Tidevann's services, regions and incidents, on call, and the public status page. Each
//! page grows here as the parts it needs land in the library; `docs/parity/status.md` says where it
//! stands.

pub mod data;
pub mod live;

use eframe::egui::{self, Ui};
use sokkel_ui_egui::{
    AccountAction, AccountMenu, Appearance, Button, ButtonSize, ButtonVariant, Cell, ChipTone, Column, Columns,
    DataTable, Gap, Icon, KpiRow, MenuItem, Page, PageHeader, Rail, RailEntry, RailGroup, Segmented, Shell, ShellPart,
    Stack, StackLayout, StackPlace, Stat, StatusLine, StatusTone, TableState, TopBar, install, relative_time,
};

use data::SERVICES;
use live::Live;

/// Tidevann's mark, as sokkel-ui's examples draw it (`tidevann-shared/mark`).
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// Who is signed in to the board.
const ME: &str = "ada";

/// How often a reading comes, in seconds.
const INTERVAL: u32 = 2;

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
    pub place: String,
    /// The service a service page shows.
    pub service: Option<String>,
    /// Settings open over the page.
    pub settings: bool,
    pub appearance: Appearance,
    /// Readings live, or paused.
    pub live: bool,
    /// The board's clock: seconds since midnight of the first reading, and the readings since.
    pub start: u32,
    pub readings: u32,
    /// Who answers now.
    pub on_call: String,
    /// Every incident, open ones first.
    pub incidents: Vec<data::Incident>,
    /// Which incidents the incidents page shows: open, resolved or all.
    pub shown: usize,
    /// The incidents' table.
    pub incidents_table: TableState,
    /// The live world the readings come from.
    pub live_world: Live,
}

impl StatusApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::on(Appearance::default())
    }

    /// The board as the demo starts: 09:00, the seeded incidents, grace on call.
    pub fn on(appearance: Appearance) -> Self {
        Self {
            place: "overview".into(),
            service: None,
            settings: false,
            appearance,
            live: true,
            start: 9 * 3600,
            readings: 0,
            on_call: "grace".into(),
            incidents: data::seed_incidents(9.0 * 3600.0 * 1000.0),
            shown: 0,
            incidents_table: TableState::default(),
            live_world: Live::new(9.0 * 3600.0 * 1000.0, INTERVAL as f64 * 1000.0),
        }
    }

    /// Go to a route as the web's hash routes name it: `overview`, `services/api`, `settings`.
    pub fn go(&mut self, route: &str) {
        let mut parts = route.split('/');
        match parts.next().unwrap_or("overview") {
            "settings" => self.settings = true,
            place if PLACES.iter().any(|(id, _)| *id == place) => {
                self.place = place.to_owned();
                self.service = parts
                    .next()
                    .filter(|id| place == "services" && SERVICES.iter().any(|s| s.id == *id))
                    .map(str::to_owned);
            }
            _ => self.place = "overview".into(),
        }
    }

    /// Take the next reading, `INTERVAL` on.
    pub fn tick(&mut self) {
        self.readings += 1;
        let at = (self.start + self.readings * INTERVAL) as f64 * 1000.0;
        self.live_world.tick(at);
    }

    /// The last reading's time, `HH:MM:SS`.
    pub fn clock(&self) -> String {
        let at = self.start + self.readings * INTERVAL;
        format!("{:02}:{:02}:{:02}", at / 3600, at / 60 % 60, at % 60)
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
        let groups = self.groups();
        let mut place = if self.settings {
            "settings".to_owned()
        } else {
            self.place.clone()
        };
        let label = PLACES
            .iter()
            .find(|(id, _)| *id == self.place)
            .map_or("Overview", |(_, label)| *label);
        let service = self.service.clone();
        let trail: Vec<&str> = match &service {
            Some(id) => vec!["Status", "Services", id],
            None => vec!["Status", label],
        };
        let mut account = None;
        let mut toggled = false;
        let clock = self.clock();
        let live = self.live;
        let operational = SERVICES.len();
        let open = self.open().len();
        let on_call = self.on_call.clone();
        Shell::new().show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Status", &groups, &mut place).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new(ME)
                    .place("tidevann.example.org")
                    .scheme(self.appearance.scheme)
                    .show(ui);
            }
            ShellPart::Top => {
                TopBar::new(&trail).search("Search", "⌘K").show(ui, |ui| {
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
                });
            }
            ShellPart::Page => {
                Page::show(ui, |ui| self.page(ui));
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
        if place == "settings" {
            self.settings = true;
        } else if place != self.place && place != "public" {
            self.go(&place);
        }
        match account {
            Some(AccountAction::Settings) => self.settings = true,
            Some(AccountAction::Scheme(scheme)) => self.appearance.scheme = scheme,
            None => {}
        }
    }

    /// The page the place names: its head now, its parts as the library gains them.
    fn page(&mut self, ui: &mut Ui) {
        let clock = self.clock();
        let live = self.live;
        let mut toggled = false;
        match (self.place.as_str(), self.service.as_deref()) {
            ("services", Some(id)) => {
                let service = SERVICES.iter().find(|s| s.id == id).copied().unwrap_or(SERVICES[0]);
                PageHeader::new(service.id)
                    .description(service.description)
                    .show(ui, |_| {});
            }
            ("services", None) => PageHeader::new("Services").count("all operational").show(ui, |_| {}),
            ("incidents", _) => {
                Stack::new(Gap::Section).show(ui, |ui| self.incidents_page(ui));
            }
            ("regions", _) => PageHeader::new("Regions")
                .count("11 of 11 sites operational")
                .show(ui, |_| {}),
            ("on-call", _) => PageHeader::new("On call")
                .description("Who answers when something goes wrong, and the weeks ahead.")
                .show(ui, |ui| {
                    ui.add(Button::new("Hand over").icon(Icon::ArrowRightLeft));
                }),
            _ => {
                let description = if live {
                    format!("Live, a reading every {INTERVAL} s, the last at {clock}.")
                } else {
                    format!("Paused at {clock}.")
                };
                PageHeader::new("Overview").description(description).show(ui, |ui| {
                    ui.add(Button::new("Report incident").icon(Icon::Siren));
                    ui.add(Button::new("Edit").icon(Icon::Pencil).variant(ButtonVariant::Outline));
                    let (icon, word) = if live {
                        (Icon::Pause, "Pause")
                    } else {
                        (Icon::Play, "Resume")
                    };
                    toggled = ui
                        .add(Button::new(word).icon(icon).variant(ButtonVariant::Outline))
                        .clicked();
                });
                self.kpis(ui);
                if let Some(id) = self.tiles(ui) {
                    self.go(&format!("services/{id}"));
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
        let opened = self.open();
        let mut open = Stat::new("Open incidents", opened.len());
        if let Some(first) = opened.first() {
            open = open.note(first.code.clone());
        }
        KpiRow::new([
            Stat::new("Services operational", format!("{0} of {0}", SERVICES.len())),
            Stat::new("Sites operational", format!("{0} of {0}", data::SITES.len())),
            open,
            Stat::new("Uptime, 90 days", "-"),
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
                ui.add(Button::new("New").icon(Icon::Plus));
            });
        let t = sokkel_ui_egui::tokens(ui.ctx());
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
        let now = (self.start + self.readings * INTERVAL) as f64 * 1000.0;
        let mut listed: Vec<&data::Incident> = self
            .incidents
            .iter()
            .filter(|i| match self.shown {
                0 => i.state != data::State::Resolved,
                1 => i.state == data::State::Resolved,
                _ => true,
            })
            .collect();
        listed.sort_by(|a, b| b.opened.total_cmp(&a.opened));
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
        let rows: Vec<Vec<Cell>> = listed
            .iter()
            .map(|incident| {
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
                    Cell::Text(relative_time(incident.opened - now)),
                    Cell::Text(duration(lasted)),
                ]
            })
            .collect();
        let keys: Vec<String> = listed.iter().map(|i| i.id.clone()).collect();
        DataTable::new("incidents", &columns, &rows, &mut self.incidents_table)
            .keys(keys)
            .stack_layout(StackLayout::Labeled)
            .actions(|_| {
                vec![
                    MenuItem::action("Resolve").icon(Icon::Check),
                    MenuItem::action("Assign to on call").icon(Icon::PhoneCall),
                ]
            })
            .footer(false)
            .stretch(true)
            .show(ui);
    }

    /// Each service as a tile: its state, its latency now with its last readings as a line, and its
    /// uptime (none yet: the incident history it is read from is still to port).
    fn tiles(&self, ui: &mut Ui) -> Option<&'static str> {
        let t = sokkel_ui_egui::tokens(ui.ctx());
        let mut opened = None;
        Columns::new(4)
            .min(160.0)
            .gap(12.0)
            .show(ui, SERVICES.len(), |ui, index, width| {
                let service = SERVICES[index];
                let health = self.live_world.health[index];
                let readings = &self.live_world.readings[index];
                let last = readings.last().map_or("-".to_owned(), |r| format!("{}", r.p95.round()));
                let stat = Stat::new(service.id, last)
                    .dot(t.color(health.color()))
                    .emphasis(health != live::Health::Operational)
                    .unit("ms p95")
                    .note(format!("{} · no data over 90 days", health.words()))
                    .trend(readings.iter().map(|r| r.p95), t.color(health.tone()));
                if stat.tile(ui, width).clicked() {
                    opened = Some(service.id);
                }
            });
        opened
    }
}

/// An incident's state in words, as the web's status map says it.
fn state_words(state: data::State) -> &'static str {
    match state {
        data::State::Investigating => "investigating",
        data::State::Identified => "identified",
        data::State::Monitoring => "monitoring",
        data::State::Resolved => "resolved",
    }
}

/// An incident state's tone, as the web's `STATUSES` colors it.
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
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }
}
