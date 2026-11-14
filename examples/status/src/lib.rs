//! sokkel-ui's status example ported one to one (`examples/status/web/src` in sokkel-ui): a live
//! board of Tidevann's services, regions and incidents, on call, and the public status page. Each
//! page grows here as the parts it needs land in the library; `docs/parity/status.md` says where it
//! stands.

pub mod data;

use eframe::egui::{self, Ui};
use sokkel_ui_egui::{
    AccountAction, AccountMenu, Appearance, Button, ButtonSize, ButtonVariant, Icon, Page, PageHeader, Rail, RailEntry,
    RailGroup, Shell, ShellPart, StatusLine, StatusTone, TopBar, install,
};

use data::SERVICES;

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
            }
        }
        if toggled {
            self.live = !self.live;
        }
    }
}

impl eframe::App for StatusApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }
}
