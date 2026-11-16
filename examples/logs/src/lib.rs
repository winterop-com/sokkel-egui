//! A live log viewer across Tidevann's services, an app of sokkel-egui's own: every line as it
//! comes, newest first; searched, filtered by level and by service; a line's fields beside the
//! stream. It uses the library's public parts only.

pub mod stream;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, Button, ButtonVariant, Cell, ChipTone, Column, DataTable, DescriptionList,
    DesktopWindow, Empty, Icon, Page, PageHeader, Rail, RailEntry, RailGroup, SearchField, Segmented, Shell, ShellPart,
    SplitPane, StackLayout, StackPlace, StatusLine, StatusTone, TableState, TableVariant, TagChips, TextRole, TopBar,
    install, toolbar,
};

use stream::{Level, Line, SERVICES, Stream};

/// Tidevann's mark, as sokkel-ui's examples draw it (`tidevann-shared/mark`).
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// Seconds between readings, as the status board's.
pub const INTERVAL: u32 = 2;

/// The log viewer: the stream, where it looks, and the line opened beside it.
pub struct LogsApp {
    pub appearance: Appearance,
    /// The frame stands in a desktop window with its title bar hidden: the running app's, never
    /// its screenshots'.
    pub desktop: bool,
    pub stream: Stream,
    /// The service the rail chose, or "all".
    pub place: String,
    /// Every level, info, warnings or errors.
    pub level: usize,
    pub search: String,
    pub live: bool,
    /// The line opened beside the stream, by its id.
    pub opened: Option<u64>,
    pub table: TableState,
    /// Seconds of the day of the last reading.
    pub at: u32,
    /// When the last reading came, on the window's clock, to take the next on time.
    last: Option<f64>,
}

impl LogsApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::on(Appearance::default());
        app.desktop = true;
        app
    }

    /// The viewer as it starts: at 09:00:04, five minutes of lines behind it, live.
    pub fn on(appearance: Appearance) -> Self {
        let at = 9 * 3600 + 4;
        Self {
            appearance,
            desktop: false,
            stream: Stream::new(at, 150),
            place: "all".into(),
            level: 0,
            search: String::new(),
            live: true,
            opened: None,
            table: TableState::default(),
            at,
            last: None,
        }
    }

    /// Take the next reading, `INTERVAL` on.
    pub fn tick(&mut self) {
        self.at += INTERVAL;
        self.stream.reading(self.at);
    }

    /// The lines the viewer shows, newest first: the service the rail chose, the level, and every
    /// word searched for in the line.
    pub fn shown(&self) -> Vec<&Line> {
        let words: Vec<String> = self.search.split_whitespace().map(str::to_lowercase).collect();
        self.stream
            .lines
            .iter()
            .rev()
            .filter(|l| self.place == "all" || l.service == self.place)
            .filter(|l| match self.level {
                1 => l.level == Level::Info,
                2 => l.level == Level::Warn,
                3 => l.level == Level::Error,
                _ => true,
            })
            .filter(|l| {
                let text = format!(
                    "{} {} {} {}",
                    l.service,
                    l.host,
                    l.message,
                    l.trace.as_deref().unwrap_or("")
                )
                .to_lowercase();
                words.iter().all(|w| text.contains(w))
            })
            .collect()
    }

    fn groups(&self) -> Vec<RailGroup> {
        let errors = |service: &str| {
            self.stream
                .lines
                .iter()
                .filter(|l| l.service == service && l.level == Level::Error)
                .count() as u32
        };
        let mut services = Vec::new();
        for service in SERVICES {
            let mut entry = RailEntry::new(service, service, Icon::Server);
            let count = errors(service);
            if count > 0 {
                entry = entry.count(count);
            }
            services.push(entry);
        }
        vec![
            RailGroup::new(
                None,
                [RailEntry::new("all", "All services", Icon::ScrollText).description("Every line as it comes")],
            ),
            RailGroup::new(Some("Services"), services),
        ]
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let groups = self.groups();
        let mut place = self.place.clone();
        let trail: Vec<String> = if self.place == "all" {
            vec!["Logs".into(), "All services".into()]
        } else {
            vec!["Logs".into(), self.place.clone()]
        };
        let trail: Vec<&str> = trail.iter().map(String::as_str).collect();
        let shell = if self.desktop {
            Shell::new().desktop(DesktopWindow::native())
        } else {
            Shell::new()
        };
        let live = self.live;
        let total = self.stream.lines.len();
        let errors = self.stream.lines.iter().filter(|l| l.level == Level::Error).count();
        let clock = format!("{:02}:{:02}:{:02}", self.at / 3600, self.at / 60 % 60, self.at % 60);
        let mut account = None;
        let mut toggled = false;
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Logs", &groups, &mut place).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada")
                    .place("tidevann.example.org")
                    .scheme(self.appearance.scheme)
                    .show(ui);
            }
            ShellPart::Top => {
                TopBar::new(&trail).show(ui, |ui| {
                    let (icon, word) = if live {
                        (Icon::Pause, "Pause")
                    } else {
                        (Icon::Play, "Resume")
                    };
                    toggled = ui
                        .add(Button::new(word).icon(icon).variant(ButtonVariant::Ghost))
                        .clicked();
                });
            }
            ShellPart::Page => Page::show(ui, |ui| self.page(ui)),
            ShellPart::Foot => {
                let note = if live {
                    format!("Live, last line {clock}")
                } else {
                    format!("Paused at {clock}")
                };
                ui.add(
                    StatusLine::new(note)
                        .tone(if live { StatusTone::Live } else { StatusTone::Quiet })
                        .live(live)
                        .item("lines", total)
                        .item("errors", errors),
                );
            }
        });
        if toggled {
            self.live = !self.live;
        }
        if place != self.place {
            self.place = place;
            self.opened = None;
        }
        match account {
            Some(AccountAction::Scheme(scheme)) => self.appearance.scheme = scheme,
            Some(AccountAction::Settings) | None => {}
        }
    }

    fn page(&mut self, ui: &mut Ui) {
        let t = sokkel_egui::tokens(ui.ctx());
        let title = if self.place == "all" {
            "All services".to_owned()
        } else {
            self.place.clone()
        };
        let shown_count = self.shown().len();
        PageHeader::new(title)
            .count(format!("{shown_count} lines"))
            .description("Newest first, a reading every 2 s.")
            .show(ui, |_| {});
        let in_place: Vec<&Line> = self
            .stream
            .lines
            .iter()
            .filter(|l| self.place == "all" || l.service == self.place)
            .collect();
        let count = |level: Option<Level>| in_place.iter().filter(|l| level.is_none_or(|v| l.level == v)).count();
        let counts = [
            Some(count(None)),
            Some(count(Some(Level::Info))),
            Some(count(Some(Level::Warn))),
            Some(count(Some(Level::Error))),
        ];
        let (search, level) = (&mut self.search, &mut self.level);
        toolbar(
            ui,
            |ui| {
                SearchField::new("logs-search", search)
                    .placeholder("Search the lines")
                    .keys("/")
                    .show(ui);
            },
            |ui| {
                ui.add(
                    Segmented::new(level, ["All", "Info", "Warnings", "Errors"])
                        .counts(counts)
                        .dots([None, None, Some(t.color("warning")), Some(t.color("critical"))]),
                );
            },
        );
        ui.add_space(t.density("section-gap") - ui.spacing().item_spacing.y);
        let lines: Vec<Line> = self.shown().into_iter().cloned().collect();
        let opened = self
            .opened
            .and_then(|id| self.stream.lines.iter().find(|l| l.id == id).cloned());
        let table = &mut self.table;
        let mut chose = None;
        SplitPane::new("logs-split")
            .default_share(0.66)
            .min_share(0.45)
            .max_share(0.8)
            .show(
                ui,
                |ui| {
                    let columns = [
                        Column::new("Time", 88.0),
                        Column::new("Level", 80.0).stack(StackPlace::Secondary),
                        Column::new("Service", 96.0).stack(StackPlace::Secondary),
                        Column::new("Message", 288.0).fill().stack(StackPlace::Primary),
                    ];
                    let rows: Vec<Vec<Cell>> = lines
                        .iter()
                        .map(|l| {
                            vec![
                                Cell::Mono(l.clock()),
                                level_chip(l.level),
                                Cell::Tags(vec![l.service.into()]),
                                Cell::Mono(l.message.clone()),
                            ]
                        })
                        .collect();
                    let keys: Vec<u64> = lines.iter().map(|l| l.id).collect();
                    DataTable::new("logs", &columns, &rows, table)
                        .keys(keys.clone())
                        .variant(TableVariant::List)
                        .stack_layout(StackLayout::Labeled)
                        .stack_below(520.0)
                        .words(sokkel_egui::TableWords {
                            empty: "No line says that.".into(),
                            ..Default::default()
                        })
                        .footer(false)
                        .stretch(true)
                        .show(ui);
                    if let Some(row) = table.opened.take() {
                        chose = keys.get(row).copied();
                    }
                },
                |ui| match &opened {
                    Some(line) => line_detail(ui, line),
                    None => {
                        ui.add_space(48.0);
                        Empty::new("Pick a line")
                            .icon(Icon::ScrollText)
                            .description("Its fields stand here: the host, the trace, the whole message.")
                            .show(ui, |_| {});
                    }
                },
            );
        if chose.is_some() {
            self.opened = chose;
        }
    }
}

/// A line's fields, beside the stream.
fn line_detail(ui: &mut Ui, line: &Line) {
    let t = sokkel_egui::tokens(ui.ctx());
    egui::Frame::new().inner_margin(egui::Margin::same(16)).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            ui.label(
                TextRole::Head
                    .rich(&t, line.message.clone())
                    .color(t.color("foreground")),
            );
            let service = vec![line.service.to_owned()];
            DescriptionList::new()
                .text("Time", line.clock())
                .item("Level", |ui| {
                    if let Cell::Chip(words, tone) = level_chip(line.level) {
                        ui.add(sokkel_egui::Chip::new(words).tone(tone));
                    }
                })
                .item("Service", |ui| {
                    ui.add(TagChips::new(&service));
                })
                .text("Host", line.host.clone())
                .text("Trace", line.trace.clone().unwrap_or_else(|| "None".into()))
                .show(ui);
        });
    });
}

/// A level as its chip, in the status map's colors.
fn level_chip(level: Level) -> Cell {
    let tone = match level {
        Level::Info => ChipTone::Neutral,
        Level::Warn => ChipTone::Warning,
        Level::Error => ChipTone::Critical,
    };
    Cell::Chip(level.word().into(), tone)
}

impl eframe::App for LogsApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        // a reading every INTERVAL seconds of the window's clock while live
        let now = ui.ctx().input(|i| i.time);
        let last = *self.last.get_or_insert(now);
        if self.live && now - last >= f64::from(INTERVAL) {
            self.tick();
            self.last = Some(now);
        } else if !self.live {
            self.last = Some(now);
        }
        if self.live {
            ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(
                (f64::from(INTERVAL) - (now - last)).max(0.05),
            ));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }
}
