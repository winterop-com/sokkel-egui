//! sokkel-ui's status example ported one to one (`examples/status/web/src` in sokkel-ui): a live
//! board of Tidevann's services, regions and incidents, on call, and the public status page. Each
//! page grows here as the parts it needs land in the library; `docs/parity/status.md` says where it
//! stands.

pub mod data;
pub mod live;

use eframe::egui::{self, Ui};
use sokkel_ui_egui::{
    AccountAction, AccountMenu, Appearance, Button, ButtonSize, ButtonVariant, Columns, Icon, KpiRow, Page, PageHeader,
    Rail, RailEntry, RailGroup, Shell, ShellPart, Stat, StatusLine, StatusTone, TopBar, install,
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
    /// The incidents still open, by code.
    pub open: Vec<String>,
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
            open: vec!["INC-41".into()],
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

    fn groups(&self) -> Vec<RailGroup> {
        let open = self.open.len() as u32;
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
        let open = self.open.len();
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
            ("incidents", _) => PageHeader::new("Incidents")
                .count(format!("{} open", self.open.len()))
                .show(ui, |ui| {
                    ui.add(Button::new("New").icon(Icon::Plus));
                }),
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
        let mut open = Stat::new("Open incidents", self.open.len());
        if let Some(first) = self.open.first() {
            open = open.note(first.clone());
        }
        KpiRow::new([
            Stat::new("Services operational", format!("{0} of {0}", SERVICES.len())),
            Stat::new("Sites operational", format!("{0} of {0}", data::SITES)),
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
