//! sokkel-ui's status example ported one to one (`examples/status/web/src` in sokkel-ui): a live
//! board of Tidevann's services, regions and incidents, on call, and the public status page. Each
//! page grows here as the parts it needs land in the library; `docs/parity/status.md` says where it
//! stands.

pub mod data;
pub mod history;
pub mod live;

use eframe::egui::{self, Ui};
use sokkel_ui_egui::{
    AccountAction, AccountMenu, Appearance, AsideWidth, Button, ButtonSize, ButtonVariant, Cell, ChipBar, ChipTone,
    Column, Columns, DashboardGrid, DashboardPanel, DataTable, Day, Editor, Gap, Icon, KpiRow, MenuItem, Page,
    PageHeader, PanelSpan, PersonRow, Rail, RailEntry, RailGroup, Section, Segmented, Shell, ShellPart, Split,
    SplitPane, Stack, StackLayout, StackPlace, Stat, StatusLine, StatusTone, TableState, TableVariant, TextRole,
    Timeline, TimelineEvent, TopBar, UptimeBar, UptimeDay, format_uptime, install, overall_uptime, relative_time,
};

use data::SERVICES;
use live::Live;
use sokkel_ui_egui_charts::{Series, TimeSeriesPanel};
use sokkel_ui_egui_maps::{MapLegend, MapView, Marker};

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
    /// The day pressed on the service's uptime bar, its incidents listed under it.
    pub service_day: Option<Day>,
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
    pub services_table: TableState,
    /// The board's day, the rotation, the handovers and the rotation's table.
    pub today: Day,
    pub shifts: Vec<data::Shift>,
    pub handovers: Vec<data::Handover>,
    pub rotation_table: TableState,
    /// The live world the readings come from.
    pub live_world: Live,
}

impl StatusApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::on(Appearance::default());
        // the street map only in the running app: tests draw the map without the network
        sokkel_ui_egui_maps::install(cc.wgpu_render_state.as_ref());
        app.sites_map = std::mem::replace(&mut app.sites_map, MapView::new(0.0, 0.0, 1.0)).with_tiles(
            &cc.egui_ctx,
            "tidevann-status/0.1 (an example app of a component library)",
            std::env::temp_dir().join("sokkel-ui-egui-tiles").into(),
        );
        app
    }

    /// The board as the demo starts: 09:00, the seeded incidents, grace on call.
    pub fn on(appearance: Appearance) -> Self {
        // the demo's day, as the web's shots are taken on it
        let today = Day::parse("2026-11-16").expect("a day");
        Self {
            place: "overview".into(),
            service: None,
            service_day: None,
            settings: false,
            appearance,
            live: true,
            start: 9 * 3600,
            readings: 0,
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
            rotation_table: TableState::default(),
            live_world: Live::new(9.0 * 3600.0 * 1000.0, INTERVAL as f64 * 1000.0),
            region: "All".into(),
            selected_site: None,
            sites_table: TableState::default(),
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
            place if PLACES.iter().any(|(id, _)| *id == place) => {
                self.place = place.to_owned();
                self.service = parts
                    .next()
                    .filter(|id| place == "services" && SERVICES.iter().any(|s| s.id == *id))
                    .map(str::to_owned);
                self.service_day = parts.next().and_then(Day::parse);
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

    /// Every service's last ninety days, in `SERVICES`' order.
    pub fn uptime(&self) -> Vec<Vec<UptimeDay>> {
        let now = ((self.start + self.readings * INTERVAL) as f64) * 1000.0;
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
                // the listings fill the page, their tables scrolling inside it; the other pages
                // scroll as a whole
                let fills = matches!(
                    (self.place.as_str(), self.service.as_ref()),
                    ("incidents", _) | ("services", None) | ("regions", _)
                );
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
                let id = id.to_owned();
                Stack::new(Gap::Page).show(ui, |ui| self.service_page(ui, &id));
            }
            ("services", None) => {
                Stack::new(Gap::Section).show(ui, |ui| self.services_page(ui));
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
                    Cell::Relative {
                        at: incident.opened,
                        now,
                    },
                    Cell::Text(duration(lasted)),
                ]
            })
            .collect();
        let keys: Vec<String> = listed.iter().map(|i| i.id.clone()).collect();
        DataTable::new("incidents", &columns, &rows, &mut self.incidents_table)
            .keys(keys)
            .search(&mut self.incident_search)
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

    /// The services: their head with Reset order, and the table of them in the person's order.
    fn services_page(&mut self, ui: &mut Ui) {
        let t = sokkel_ui_egui::tokens(ui.ctx());
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

    /// Every site on a map and in a table, both live: a region frames its sites, and a site chosen
    /// in the table is chosen on the map.
    fn regions_page(&mut self, ui: &mut Ui) {
        let t = sokkel_ui_egui::tokens(ui.ctx());
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
        let markers: Vec<Marker> = data::SITES
            .iter()
            .enumerate()
            .map(|(i, site)| {
                Marker::new(site.at[1], site.at[0], "").color(t.color(self.live_world.site_health[i].color()))
            })
            .collect();
        let count = |h: live::Health| self.live_world.site_health.iter().filter(|x| **x == h).count();
        let legend = MapLegend::new("Sites")
            .description(format!("{} cities", data::SITES.len()))
            .row(t.color("good"), "Operational", count(live::Health::Operational))
            .row(t.color("warning"), "Degraded", count(live::Health::Degraded))
            .row(t.color("critical"), "Outage", count(live::Health::Outage));
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
        let map = &mut self.sites_map;
        let table = &mut self.sites_table;
        let mut chosen = None;
        SplitPane::new("status-regions")
            .default_share(0.6)
            .min_share(0.35)
            .max_share(0.75)
            .show(
                ui,
                |ui| {
                    let size = ui.available_size().max(egui::vec2(1.0, 320.0));
                    let response = map.show(ui, size, &markers, &[]);
                    legend.show(ui, response.rect);
                },
                |ui| {
                    let output = DataTable::new("sites", &columns, &rows, table)
                        .keys(keys)
                        .bar(true)
                        .stack_layout(StackLayout::Labeled)
                        .actions(|_| vec![MenuItem::action("Show on the map")])
                        .footer(false)
                        .stretch(true)
                        .show(ui);
                    if let Some((row, _)) = output.action {
                        chosen = Some(listed[row]);
                    }
                },
            );
        if let Some(site) = chosen {
            self.selected_site = Some(site);
            let s = &data::SITES[site];
            self.sites_map
                .frame(Some([s.at[0] - 6.0, s.at[1] - 6.0, s.at[0] + 6.0, s.at[1] + 6.0]));
        }
    }

    /// One service: its head with its host, what it does and its state, the way back to the
    /// services and its actions; its figures; its charts, graph and uptime as they are ported.
    fn service_page(&mut self, ui: &mut Ui, id: &str) {
        let index = SERVICES.iter().position(|s| s.id == id).unwrap_or(0);
        let service = SERVICES[index];
        let t = sokkel_ui_egui::tokens(ui.ctx());
        let days = self.uptime().swap_remove(index);
        let overall = overall_uptime(&days);
        let health = self.live_world.health[index];
        let last = self.live_world.readings[index].last().copied();
        let head = PageHeader::new(service.id)
            .back("Services")
            .code(service.host)
            .description(service.description)
            .chip(health.words().to_lowercase(), health_tone(health))
            .show(ui, |ui| {
                ui.add(Button::new("Deploy").icon(Icon::Rocket));
                ui.add(
                    Button::new("Schedule maintenance")
                        .icon(Icon::CalendarClock)
                        .variant(ButtonVariant::Outline),
                );
                ui.add(
                    Button::new("Report incident")
                        .icon(Icon::Siren)
                        .variant(ButtonVariant::Outline),
                );
            });
        if head.back {
            self.go("services");
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
            Stat::new("Running", service.version),
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
                let now = ((self.start + self.readings * INTERVAL) as f64) * 1000.0;
                self.incidents
                    .iter()
                    .filter(|i| i.services.contains(&id) && i.opened < to && i.resolved.unwrap_or(now) > from)
                    .map(|i| TimelineEvent {
                        icon: Icon::Siren,
                        title: format!("{} {}", i.code, i.title),
                        at: day.medium(),
                        detail: Some(format!(
                            "{}, {}",
                            severity_words(i.severity).to_lowercase(),
                            state_words(i.state).to_lowercase()
                        )),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut pressed = None;
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
            let pressed = &mut pressed;
            let days = &days;
            grid.cell(PanelSpan::Half, move |ui| {
                let subtitle = overall.map_or("the last 90 days".to_owned(), |o| {
                    format!("{} over 90 days", format_uptime(o))
                });
                DashboardPanel::new("Uptime").subtitle(subtitle).show(ui, |ui| {
                    Stack::new(Gap::Lg).show(ui, |ui| {
                        *pressed = UptimeBar::new(("service", id), days).show(ui).map(|d| d.date);
                        let muted = |text: String| TextRole::Help.rich(&t, text).color(t.color("muted-foreground"));
                        match picked {
                            None => {
                                ui.label(muted("Press a day to see what happened on it.".into()));
                            }
                            Some(day) if on_day.is_empty() => {
                                ui.label(muted(format!("Nothing happened to {id} on {}.", day.medium())));
                            }
                            Some(_) => Timeline::new(on_day).show(ui),
                        }
                    });
                });
            });
        });
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
                ui.add(Button::new("Hand over").icon(Icon::ArrowRightLeft));
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
                icon: Icon::PhoneCall,
                title: format!("{} to {}", h.from, h.to),
                at: {
                    let words = relative_time(h.at - self.start as f64 * 1000.0);
                    if words == "now" { "now".into() } else { words }
                },
                detail: Some(h.note.clone()),
            })
            .collect();
        let handover_count = self.handovers.len().to_string();
        let since = format!("{}, 09:00", today.plus(-(today.weekday() as i32)).with_weekday());
        Split::new(AsideWidth::Lg).show(
            ui,
            |ui| {
                Stack::new(Gap::Section).show(ui, |ui| {
                    Section::new("Now")
                        .description(format!("From the handover: {}", data::ON_CALL_NOTE))
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
        let t = sokkel_ui_egui::tokens(ui.ctx());
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
    ui.add(sokkel_ui_egui::Chip::new(label).tone(tone));
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
