//! The metrics app's frame and its dashboards, as the web's `Metrics.tsx` and `Dashboard.tsx`:
//! the rail of dashboards, the top bar, the foot strip with the newest scrape, and a dashboard's
//! panels in a grid, each read from the simulated source up to the newest second.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, Cell, Column, DashboardGrid, DashboardPanel, DataTable, DesktopWindow,
    Icon, LogLevel, LogLine, LogView, Page, PageHeader, Rail, RailEntry, RailGroup, Shell, ShellPart, StatColorMode,
    StatPanel, StatusLine, StatusTone, TableState, TopBar, format_value, install,
};
use sokkel_egui_charts::{BarGauge, BarPanel, Fill as ChartFill, GaugePanel, HeatmapPanel, Series, TimeSeriesPanel};

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
    /// How often the dashboard redraws by itself, in seconds.
    pub refresh: u32,
    /// When the window last took a scrape, on its own clock.
    last_scrape: Option<f64>,
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
            last_scrape: None,
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
        let note = if self.live {
            format!("Scraped {}, refresh every {}s", clock_of(self.at), self.refresh)
        } else {
            format!("Scraped {}, refresh paused", clock_of(self.at))
        };
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
        let (variables, range, at) = (self.variables.clone(), self.range.clone(), self.at);
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
            let every = (x.len() / 6).max(1);
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
                .stacked(panel.stacked)
                .label_every(every);
            for step in thresholds.iter().skip(1) {
                if let Some(value) = step.value {
                    chart = chart.threshold(value, t.color(step.color));
                }
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
            let write = |v: f64| format!("{}{}", format_value(v, Some(data.decimals)), data.unit);
            let columns = [
                Column::new("Name", 128.0).fill(),
                Column::new("Last", 96.0).numeric(),
                Column::new("Mean", 96.0).numeric(),
                Column::new("Max", 96.0).numeric(),
                Column::new("Min", 96.0).numeric(),
            ];
            let cells: Vec<Vec<Cell>> = rows
                .iter()
                .map(|r| {
                    vec![
                        Cell::Mono(r.name.clone()),
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
