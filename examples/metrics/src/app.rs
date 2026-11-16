//! The metrics app's frame and its dashboards, as the web's `Metrics.tsx` and `Dashboard.tsx`:
//! the rail of dashboards, the top bar, the foot strip with the newest scrape, and a dashboard's
//! panels in a grid, each read from the simulated source up to the newest second.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, Cell, Column, DashboardGrid, DashboardPanel, DataTable, DesktopWindow,
    Icon, LogLevel, LogLine, LogView, Page, PageHeader, Rail, RailEntry, RailGroup, RefreshControl, Shell, ShellPart,
    StatColorMode, StatPanel, StatusLine, StatusTone, TableState, TimeRangePicker, TopBar, format_value, install,
};
use sokkel_egui_charts::{
    BarGauge, BarPanel, Fill as ChartFill, GaugePanel, HeatmapPanel, LegendStat, Series, TimeSeriesPanel,
};

use crate::model::{Dashboard, Fill, Panel, seed_dashboards};
use crate::panel_data::{Shape, panel_data};
use crate::query::{Range, clock_of};
use crate::source::Variables;

/// 09:00:05 UTC on the demo's day, 2026-11-18, the newest scrape the board opens on, as the web's
/// shots are taken.
pub const DEMO_NOW: f64 = 1_794_992_405.0;

/// The Tidevann mark the rail heads with.
const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The metrics app: its dashboards, the one shown, its range and variables, and the newest scrape.
pub struct MetricsApp {
    pub desktop: bool,
    pub appearance: Appearance,
    pub dashboards: Vec<Dashboard>,
    /// The dashboard shown, by its id.
    pub place: String,
    pub range: Range,
    pub variables: Variables,
    /// The newest scrape, in seconds since 1970: one lands every second while the board is live.
    pub at: f64,
    pub live: bool,
    /// How often the dashboard reads its panels again by itself, in seconds; 0 is never.
    pub refresh: u32,
    /// Refreshing by itself held, the interval kept for when it resumes.
    pub paused: bool,
    /// The second the panels were last read at: the newest scrape at the last refresh.
    pub view_at: f64,
    /// When the window last took a scrape, and last refreshed, on its own clock.
    last_scrape: Option<f64>,
    last_view: Option<f64>,
    tables: std::collections::HashMap<String, TableState>,
}

impl MetricsApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::on(Appearance::default());
        app.desktop = true;
        app
    }

    /// The app as it opens: the Services overview, the last 15 minutes, every variable, 09:00:05.
    pub fn on(appearance: Appearance) -> Self {
        Self {
            desktop: false,
            appearance,
            dashboards: seed_dashboards(),
            place: "services".into(),
            range: Range::default(),
            variables: Variables::default(),
            at: DEMO_NOW,
            live: true,
            refresh: 5,
            paused: false,
            view_at: DEMO_NOW,
            last_scrape: None,
            last_view: None,
            tables: Default::default(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        // a scrape lands every second while live
        if self.live {
            let now = ui.input(|i| i.time);
            match self.last_scrape {
                Some(last) if now - last >= 1.0 => {
                    self.at += (now - last).floor();
                    self.last_scrape = Some(now);
                }
                None => self.last_scrape = Some(now),
                _ => {}
            }
            // the panels read again every interval, unless it is off or paused
            let due = match self.last_view {
                Some(last) => self.refresh > 0 && !self.paused && now - last >= f64::from(self.refresh),
                None => {
                    self.last_view = Some(now);
                    false
                }
            };
            if due {
                self.view_at = self.at;
                self.last_view = Some(now);
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
        }
        let groups = vec![RailGroup::new(
            Some("Dashboards"),
            self.dashboards
                .iter()
                .map(|b| RailEntry::new(b.id.clone(), b.title.clone(), Icon::LayoutDashboard))
                .collect::<Vec<_>>(),
        )];
        let mut place = self.place.clone();
        let shell = if self.desktop {
            Shell::new().desktop(DesktopWindow::native())
        } else {
            Shell::new()
        };
        let title = self
            .dashboards
            .iter()
            .find(|b| b.id == self.place)
            .map_or(String::new(), |b| b.title.clone());
        let trail = ["Metrics", title.as_str()];
        let mode = if self.refresh == 0 {
            "refresh off".to_owned()
        } else if self.paused {
            "refresh paused".to_owned()
        } else {
            format!("refresh every {}s", self.refresh)
        };
        let note = format!("Scraped {}, {mode}", clock_of(self.at));
        let mut account = None;
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Metrics", &groups, &mut place).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada").scheme(self.appearance.scheme).show(ui);
            }
            ShellPart::Top => {
                TopBar::new(&trail).show(ui, |_| {});
            }
            ShellPart::Page => Page::scroll(ui, |ui| self.board_page(ui)),
            ShellPart::Foot => {
                ui.add(
                    StatusLine::new(note.clone())
                        .tone(if self.live { StatusTone::Live } else { StatusTone::Quiet })
                        .item("firing", 0)
                        .item("source", "1s"),
                );
            }
        });
        if place != self.place {
            self.place = place;
        }
        if let Some(AccountAction::Scheme(scheme)) = account {
            self.appearance.scheme = scheme;
        }
    }

    /// A dashboard: its head, then its panels in the grid in the order kept.
    fn board_page(&mut self, ui: &mut Ui) {
        let Some(board) = self.dashboards.iter().find(|b| b.id == self.place).cloned() else {
            return;
        };
        PageHeader::new(&board.title)
            .description(&board.description)
            .show(ui, |_| {});
        // the variables right under the head, the page's own gap before the panels
        self.variables_bar(ui);
        let (variables, range, at) = (self.variables.clone(), self.range.clone(), self.view_at);
        for panel in &board.panels {
            self.tables.entry(panel.id.clone()).or_default();
        }
        // each panel's table state borrowed apart, so each cell holds its own
        let mut states: std::collections::HashMap<&String, &mut TableState> = self.tables.iter_mut().collect();
        DashboardGrid::new(("board", &board.id)).show(ui, |grid| {
            for panel in &board.panels {
                let Some(state) = states.remove(&panel.id) else {
                    continue;
                };
                let (variables, range) = (variables.clone(), range.clone());
                grid.cell(panel.span, move |ui| {
                    draw_panel(ui, panel, &variables, &range, at, state)
                });
            }
        });
    }
}

impl MetricsApp {
    /// The dashboard's variables, as the web's `variables()`: a service, a host of that service,
    /// and a project, each `All` or one.
    fn variables_bar(&mut self, ui: &mut Ui) {
        use crate::source::{ALL, HOSTS, PROJECTS, SERVICES};
        let choices = |values: Vec<&str>| {
            std::iter::once("All".to_owned())
                .chain(values.into_iter().map(str::to_owned))
                .collect::<Vec<_>>()
        };
        let services = choices(SERVICES.to_vec());
        let hosts = choices(
            HOSTS
                .iter()
                .filter(|h| self.variables.service == ALL || h.service == self.variables.service)
                .map(|h| h.id)
                .collect(),
        );
        let projects = choices(PROJECTS.to_vec());
        let before = (self.variables.clone(), self.range.clone());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            // the range and the refresh at the row's end, as the web's `end`
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let updated = if self.refresh > 0 && self.paused {
                    "paused".to_owned()
                } else {
                    format!("updated {}", clock_of(self.view_at))
                };
                let asked = RefreshControl::new(&mut self.refresh, &mut self.paused, &crate::query::INTERVALS)
                    .updated(updated)
                    .show(ui);
                if asked.refresh {
                    self.view_at = self.at;
                }
                let mut preset = match &self.range {
                    Range::Preset(id) => (*id).to_owned(),
                    Range::Between { .. } => String::new(),
                };
                // a custom span is of whole days: from the first's midnight to the end of the last
                let day_of = |second: f64| sokkel_egui::Day((second / 86_400.0).floor() as i32);
                let mut span = match &self.range {
                    Range::Between { from, to } => Some((day_of(*from), day_of(*to))),
                    Range::Preset(_) => None,
                };
                let presets: Vec<(&str, &str)> = crate::query::PRESETS.iter().map(|p| (p.id, p.label)).collect();
                if TimeRangePicker::new(&mut preset, &presets)
                    .custom(&mut span, day_of(self.at))
                    .show(ui)
                    .changed()
                {
                    match (crate::query::PRESETS.iter().find(|p| p.id == preset), span) {
                        (Some(p), _) => self.range = Range::Preset(p.id),
                        (None, Some((from, to))) => {
                            self.range = Range::Between {
                                from: f64::from(from.0) * 86_400.0,
                                to: f64::from(to.0) * 86_400.0 + 86_399.0,
                            };
                        }
                        (None, None) => {}
                    }
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    self.chips(ui, &services, &hosts, &projects);
                });
            });
        });
        // a new range or new variables read the panels at once
        if (self.variables.clone(), self.range.clone()) != before {
            self.view_at = self.at;
        }
    }

    fn chips(&mut self, ui: &mut Ui, services: &[String], hosts: &[String], projects: &[String]) {
        use crate::source::{ALL, HOSTS};
        let index = |options: &[String], value: &str| {
            if value == ALL {
                0
            } else {
                options.iter().position(|o| o == value).unwrap_or(0)
            }
        };
        let value = |options: &[String], at: usize| {
            if at == 0 { ALL.to_owned() } else { options[at].clone() }
        };
        {
            let services = services.to_vec();
            let hosts = hosts.to_vec();
            let projects = projects.to_vec();
            let mut at = index(&services, &self.variables.service);
            if sokkel_egui::variable_select(ui, "service", &mut at, &services).changed() {
                let service = value(&services, at);
                // a host of another service is let go
                let keeps = self.variables.host == ALL
                    || HOSTS
                        .iter()
                        .any(|h| h.id == self.variables.host && (service == ALL || h.service == service));
                if !keeps {
                    self.variables.host = ALL.into();
                }
                self.variables.service = service;
            }
            let mut at = index(&hosts, &self.variables.host);
            if sokkel_egui::variable_select(ui, "host", &mut at, &hosts).changed() {
                self.variables.host = value(&hosts, at);
            }
            let mut at = index(&projects, &self.variables.project);
            if sokkel_egui::variable_select(ui, "project", &mut at, &projects).changed() {
                self.variables.project = value(&projects, at);
            }
        }
    }
}

/// One panel, read from the source and drawn by its kind, as the web's `PanelView`.
fn draw_panel(ui: &mut Ui, panel: &Panel, variables: &Variables, range: &Range, at: f64, table: &mut TableState) {
    let data = panel_data(panel, variables, range, at);
    let thresholds = &panel.thresholds;
    let unit = data.unit.trim().to_owned();
    match data.shape {
        Shape::Stat {
            value,
            sparkline,
            delta,
            good,
        } => {
            let mut stat = StatPanel::new(&panel.title, value)
                .color_mode(match panel.color_mode {
                    crate::model::ColorMode::Value => StatColorMode::Value,
                    crate::model::ColorMode::Background => StatColorMode::Background,
                    crate::model::ColorMode::None => StatColorMode::None,
                })
                .unit(unit)
                .decimals(data.decimals)
                .thresholds(thresholds)
                .sparkline(&sparkline)
                .good(good);
            if let Some(delta) = delta {
                stat = stat.delta(delta);
            }
            if thresholds.is_empty() {
                stat = stat.color(panel.color);
            }
            stat.show(ui);
        }
        Shape::Gauge { value } => {
            GaugePanel::new(&panel.title, value)
                .range(panel.min, panel.max)
                .thresholds(thresholds)
                .color(panel.color)
                .unit(data.unit.clone())
                .decimals(data.decimals)
                .show(ui);
        }
        Shape::Series { x, series } => {
            if panel.kind == crate::model::PanelKind::Bar {
                let mut bars = BarPanel::new(&panel.title, x).stacked(panel.stacked);
                for (one, color) in series {
                    let mut s = Series::new(one.name, one.values);
                    if let Some(color) = color {
                        s = s.color(color);
                    }
                    bars = bars.series(s);
                }
                bars.show(ui);
                return;
            }
            let t = sokkel_egui::tokens(ui.ctx());
            let mut chart = TimeSeriesPanel::new(&panel.title, x)
                .unit(data.unit.clone())
                .decimals(data.decimals)
                .fill(match panel.fill {
                    Fill::None => ChartFill::None,
                    Fill::Gradient => ChartFill::Gradient,
                    Fill::Solid => ChartFill::Solid,
                })
                .stacked(panel.stacked);
            for step in thresholds.iter().skip(1) {
                if let Some(value) = step.value {
                    chart = chart.threshold(value, t.color(step.color));
                }
            }
            // more than three series read better as a table of their figures, as the web's
            if series.len() > 3 {
                chart = chart.legend_table(&[LegendStat::Avg, LegendStat::Max, LegendStat::Current]);
            }
            for (one, color) in series {
                let mut s = Series::new(one.name, one.values);
                if let Some(color) = color {
                    s = s.color(color);
                }
                chart = chart.series(s);
            }
            chart.show(ui);
        }
        Shape::Table { rows } => {
            let t = sokkel_egui::tokens(ui.ctx());
            let write = |v: f64| format!("{}{}", format_value(v, Some(data.decimals)), data.unit);
            let columns = [
                // the web's `minmax(8rem,1fr)` and `6rem`, less the 12 the table keeps between columns
                Column::new("Name", 116.0).fill(),
                Column::new("Last", 84.0).numeric(),
                Column::new("Mean", 84.0).numeric(),
                Column::new("Max", 84.0).numeric(),
                Column::new("Min", 84.0).numeric(),
            ];
            let cells: Vec<Vec<Cell>> = rows
                .iter()
                .map(|r| {
                    vec![
                        // the name after a dot in its last value's threshold color, as the web's
                        Cell::Marked {
                            text: r.name.clone(),
                            dot: sokkel_egui::threshold_color(Some(r.last), thresholds).map(|k| t.color(k)),
                        },
                        Cell::Number {
                            value: r.last,
                            text: write(r.last),
                        },
                        Cell::Number {
                            value: r.mean,
                            text: write(r.mean),
                        },
                        Cell::Number {
                            value: r.max,
                            text: write(r.max),
                        },
                        Cell::Number {
                            value: r.min,
                            text: write(r.min),
                        },
                    ]
                })
                .collect();
            DashboardPanel::new(&panel.title).flush(true).show(ui, |ui| {
                ui.set_height(224.0);
                // by Last, the highest first, until the reader sorts it otherwise
                if table.sort.is_none() {
                    table.sort = Some((1, false));
                }
                DataTable::new(("panel-table", &panel.id), &columns, &cells, table)
                    .framed(false)
                    .bar(true)
                    .stack_below(400.0)
                    .footer(false)
                    .show(ui);
            });
        }
        Shape::Logs { lines } => {
            let lines: Vec<LogLine> = lines
                .into_iter()
                .map(|l| LogLine {
                    at: Some(l.at),
                    level: Some(match l.level {
                        "debug" => LogLevel::Debug,
                        "warning" => LogLevel::Warning,
                        "error" => LogLevel::Error,
                        _ => LogLevel::Info,
                    }),
                    source: Some(l.source),
                    text: l.text,
                    fields: Vec::new(),
                })
                .collect();
            DashboardPanel::new(&panel.title).flush(true).show(ui, |ui| {
                LogView::new(("panel-logs", &panel.id), &lines)
                    .follow(true)
                    .framed(false)
                    .height(256.0)
                    .show(ui);
            });
        }
        Shape::BarGauge { bars } => {
            BarGauge::new(&panel.title, bars)
                .range(panel.min, panel.max)
                .thresholds(thresholds)
                .color(panel.color)
                .unit(data.unit.clone())
                .decimals(data.decimals)
                .show(ui);
        }
        Shape::Heatmap { x, y, cells } => {
            // several threshold colors step the scale through them; else one color shades it
            let scale: Vec<&'static str> = if thresholds.len() > 1 {
                thresholds.iter().map(|step| step.color).collect()
            } else {
                vec![panel.color]
            };
            HeatmapPanel::new(&panel.title, x, y, cells)
                .scale(scale)
                .unit(data.unit.clone())
                .decimals(data.decimals)
                .show(ui);
        }
    }
}

impl eframe::App for MetricsApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }
}
