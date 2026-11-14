//! Tidevann's runs, on sokkel-ui-egui's frame: the rail of places, the top bar with the trail, a
//! page per place, the status line, the settings over the page on Mod+comma and the palette on
//! Mod+K. Every part is the library's; the app holds only its data and where it is.

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, Ui, vec2};
use sokkel_ui_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, Card, Chip, Column, CommandGroup, CommandPalette, DataTable,
    DesktopWindow, Icon, MenuItem, PageHeader, Rail, RailEntry, RailGroup, SettingsDialog, SettingsSection, Shell,
    ShellPart, StatusLine, StatusTone, TableState, TableVariant, TextRole, ThemeWords, TopBar, install,
    palette_shortcut, setting_row, theme_settings, tokens,
};
use sokkel_ui_egui_charts::{LineChart, Series, Sparkline, legend};

mod data;

const APPEARANCE_KEY: &str = "tidevann/appearance";

/// The app's places, in the rail's groups.
fn places() -> Vec<RailGroup> {
    vec![
        RailGroup::new(None, [RailEntry::new("overview", "Overview", Icon::LayoutDashboard)]),
        RailGroup::new(
            Some("Operate"),
            [
                RailEntry::new("runs", "Runs", Icon::Play).count(3),
                RailEntry::new("schedules", "Schedules", Icon::CalendarClock),
            ],
        ),
        RailGroup::new(Some("Build"), [RailEntry::new("jobs", "Jobs", Icon::Box)]),
        RailGroup::new(Some("System"), [RailEntry::new("settings", "Settings", Icon::Settings)]),
    ]
}

pub struct RunsApp {
    pub appearance: Appearance,
    /// The frame stands in a desktop window with its title bar hidden: set by the running app, never
    /// in its screenshots, which draw the frame as the browser does.
    pub desktop: bool,
    /// Whether the reader narrowed the rail to its icons.
    pub rail_collapsed: bool,
    /// The place the page shows.
    pub place: String,
    runs: Vec<data::Run>,
    runs_table: TableState,
    schedules_table: TableState,
    jobs_table: TableState,
    /// The settings over the page, and the section shown.
    settings: Option<String>,
    /// The palette over the page: its query and highlight.
    palette: Option<(String, usize)>,
    notify: bool,
    paused: bool,
}

impl RunsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let appearance = cc
            .storage
            .and_then(|s| eframe::get_value(s, APPEARANCE_KEY))
            .unwrap_or_default();
        let mut app = Self::with(appearance);
        app.desktop = true;
        app
    }

    pub fn with(appearance: Appearance) -> Self {
        Self {
            appearance,
            desktop: false,
            rail_collapsed: false,
            place: "overview".into(),
            runs: data::runs(),
            runs_table: TableState::sorted(0, false),
            schedules_table: TableState::default(),
            jobs_table: TableState::default(),
            settings: None,
            palette: None,
            notify: true,
            paused: false,
        }
    }

    /// Go to a place; Settings opens over the page instead of replacing it.
    pub fn go(&mut self, place: &str) {
        if place == "settings" {
            self.settings = Some("appearance".into());
        } else {
            self.place = place.to_owned();
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        if palette_shortcut(ui.ctx()) {
            self.palette = Some((String::new(), 0));
        }
        if ui.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma))) {
            self.settings = Some("appearance".into());
        }
        let groups = places();
        let mut place = self.place.clone();
        let label = groups
            .iter()
            .flat_map(|g| &g.entries)
            .find(|e| e.id == self.place)
            .map_or(String::new(), |e| e.label.clone());
        let mut open_palette = false;
        let shell = if self.desktop {
            Shell::new().desktop(DesktopWindow::native())
        } else {
            Shell::new()
        }
        .collapsed(self.rail_collapsed);
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Tidevann", &groups, &mut place).collapsible(&mut self.rail_collapsed));
            }
            ShellPart::RailFoot => {
                sokkel_ui_egui::AccountMenu::new("ada")
                    .place("tidevann.example.org")
                    .show(ui);
            }
            ShellPart::Top => {
                TopBar::new(&["Tidevann", &label]).show(ui, |ui| {
                    let search = Button::new("Search")
                        .icon(Icon::Search)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm);
                    if ui.add(search).clicked() {
                        open_palette = true;
                    }
                });
            }
            ShellPart::Page => {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| self.page(ui));
            }
            ShellPart::Foot => {
                let running = self.runs.iter().filter(|r| r.state == 2).count();
                let line = StatusLine::new(format!("{running} runs running"))
                    .tone(StatusTone::Live)
                    .live(true)
                    .fact("api, worker, scheduler")
                    .fact("v2.4.0");
                ui.add(line);
            }
        });
        if place != self.place {
            self.go(&place);
        }
        if open_palette {
            self.palette = Some((String::new(), 0));
        }
        self.overlays(ui);
    }

    fn page(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let inset = t.density("page-inset-wide");
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: inset as i8,
                right: inset as i8,
                top: t.density("page-top") as i8,
                bottom: 40,
            })
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = t.density("page-gap") / 2.0;
                match self.place.as_str() {
                    "overview" => self.overview(ui),
                    "runs" => self.runs_page(ui),
                    "schedules" => self.schedules(ui),
                    _ => self.jobs(ui),
                }
            });
    }

    fn overview(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        PageHeader::new("Overview")
            .description("How Tidevann's jobs ran over the last two weeks.")
            .show(ui, |_| {});
        let (succeeded, failed) = data::per_day();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            for (i, (title, figure, values)) in [
                ("Runs today", "1,284".to_owned(), succeeded.clone()),
                ("Failed today", "37".to_owned(), failed.clone()),
                (
                    "Running now",
                    self.runs.iter().filter(|r| r.state == 2).count().to_string(),
                    vec![2.0, 3.0, 2.0, 4.0, 3.0, 5.0, 4.0],
                ),
            ]
            .into_iter()
            .enumerate()
            {
                ui.allocate_ui(vec2(240.0, 140.0), |ui| {
                    Card::new().show(ui, |ui| {
                        ui.label(TextRole::Help.rich(&t, title).color(t.color("muted-foreground")));
                        ui.label(TextRole::Statistic.rich(&t, figure).color(t.color("foreground")));
                        ui.add(Sparkline::new(values).size(206.0, 32.0).color(i));
                    });
                });
            }
        });
        Card::new().title("Runs per day").show(ui, |ui| {
            legend(ui, &["succeeded", "failed"]);
            let days: Vec<String> = (1..=14).map(|d| d.to_string()).collect();
            ui.add(
                LineChart::new(days)
                    .series(Series::new("succeeded", succeeded))
                    .series(Series::new("failed", failed))
                    .area(true)
                    .height(220.0),
            );
        });
    }

    fn runs_page(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        PageHeader::new("Runs").count(self.runs.len()).show(ui, |ui| {
            ui.add(Button::new("Run now").icon(Icon::Play));
        });
        let columns = [
            Column::new("Run", 120.0),
            Column::new("Job", 170.0),
            Column::new("State", 120.0),
            Column::new("By", 100.0),
            Column::new("Took", 100.0).numeric(),
            Column::new("Rows", 100.0).numeric(),
        ];
        let cells = data::run_cells(&self.runs);
        let opened = self.runs_table.opened;
        ui.horizontal_top(|ui| {
            let width = if opened.is_some() {
                ui.available_width() - 336.0
            } else {
                ui.available_width()
            };
            ui.allocate_ui(vec2(width, 560.0), |ui| {
                DataTable::new("runs", &columns, &cells, &mut self.runs_table)
                    .selectable(true)
                    .height(480.0)
                    .show(ui);
            });
            if let Some(index) = opened {
                let run = &self.runs[index];
                ui.add_space(16.0);
                ui.allocate_ui(vec2(320.0, 400.0), |ui| {
                    Card::new().title(run.id.clone()).bar(true).show(ui, |ui| {
                        let (state, tone) = data::STATES[run.state];
                        egui::Grid::new("run-detail")
                            .num_columns(2)
                            .spacing(vec2(24.0, 10.0))
                            .show(ui, |ui| {
                                for (term, value) in [
                                    ("Job", run.job.to_owned()),
                                    ("By", run.by.to_owned()),
                                    ("Took", run.took.clone()),
                                    ("Rows", run.rows.to_string()),
                                ] {
                                    ui.label(TextRole::Help.rich(&t, term).color(t.color("muted-foreground")));
                                    ui.label(TextRole::Body.rich(&t, value).color(t.color("foreground")));
                                    ui.end_row();
                                }
                                ui.label(TextRole::Help.rich(&t, "State").color(t.color("muted-foreground")));
                                ui.add(Chip::new(state).tone(tone));
                                ui.end_row();
                            });
                    });
                });
            }
        });
    }

    fn schedules(&mut self, ui: &mut Ui) {
        PageHeader::new("Schedules").count(data::JOBS.len()).show(ui, |ui| {
            ui.add(Button::new("New").icon(Icon::Plus));
        });
        let rows: Vec<Vec<_>> = data::JOBS
            .iter()
            .map(|(job, _, _, when)| vec![(*job).into(), (*when).into()])
            .collect();
        let columns = [Column::new("Job", 240.0), Column::new("When", 200.0)];
        DataTable::new("schedules", &columns, &rows, &mut self.schedules_table)
            .variant(TableVariant::List)
            .show(ui);
    }

    fn jobs(&mut self, ui: &mut Ui) {
        PageHeader::new("Jobs")
            .count(data::JOBS.len())
            .description("What runs, and who keeps it.")
            .show(ui, |ui| {
                ui.add(Button::new("New").icon(Icon::Plus));
            });
        let rows: Vec<Vec<_>> = data::JOBS
            .iter()
            .map(|(job, project, owner, _)| vec![(*job).into(), (*project).into(), (*owner).into()])
            .collect();
        let columns = [
            Column::new("Job", 240.0),
            Column::new("Project", 160.0),
            Column::new("Owner", 160.0),
        ];
        DataTable::new("jobs", &columns, &rows, &mut self.jobs_table).show(ui);
    }

    fn overlays(&mut self, ui: &mut Ui) {
        if let Some(mut active) = self.settings.take() {
            let sections = [
                SettingsSection::new("appearance", "Appearance", Icon::Palette),
                SettingsSection::new("runs", "Runs", Icon::Play),
            ];
            let (appearance, notify, paused) = (&mut self.appearance, &mut self.notify, &mut self.paused);
            let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ui.ctx(), |section, ui| {
                if section == "appearance" {
                    theme_settings(ui, appearance, true, &ThemeWords::default());
                } else {
                    setting_row(
                        ui,
                        "Tell ada about failures",
                        "A message when a run fails, with its first problem.",
                        |ui| {
                            ui.add(sokkel_ui_egui::Switch::new(notify, "Tell ada about failures"));
                        },
                    );
                    setting_row(
                        ui,
                        "Pause every schedule",
                        "No job starts on its own until this is off again.",
                        |ui| {
                            ui.add(sokkel_ui_egui::Switch::new(paused, "Pause every schedule"));
                        },
                    );
                }
            });
            if outcome.reset && active == "appearance" {
                self.appearance = Appearance::default();
            }
            if !outcome.close {
                self.settings = Some(active);
            }
        }
        if let Some((mut query, mut highlight)) = self.palette.take() {
            let places = places();
            let groups = [CommandGroup::new(
                "Go to",
                places
                    .iter()
                    .flat_map(|g| &g.entries)
                    .map(|e| MenuItem::action(e.label.clone()).icon(e.icon)),
            )];
            let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight).show(ui.ctx());
            if let Some((_, index)) = outcome.run {
                let id = places
                    .iter()
                    .flat_map(|g| &g.entries)
                    .nth(index)
                    .map(|e| e.id.clone())
                    .unwrap_or_default();
                self.go(&id);
            }
            if !outcome.close {
                self.palette = Some((query, highlight));
            }
        }
    }
}

impl eframe::App for RunsApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, APPEARANCE_KEY, &self.appearance);
    }
}
