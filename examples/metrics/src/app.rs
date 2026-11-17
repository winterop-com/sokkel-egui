//! The metrics app's frame and its dashboards, as the web's `Metrics.tsx` and `Dashboard.tsx`:
//! the rail of dashboards, the top bar, the foot strip with the newest scrape, and a dashboard's
//! panels in a grid, each read from the simulated source up to the newest second.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, Cell, Column, DashboardArrange, DashboardEdit, DashboardPanel, DataTable,
    DesktopWindow, Icon, LogLevel, LogLine, LogView, Page, PageHeader, PanelMenu, Rail, RailEntry, RailGroup,
    RefreshControl, Shell, ShellPart, StatColorMode, StatPanel, StatusLine, StatusTone, TableState, TimeRangePicker,
    Toast, TopBar, format_value, install,
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

/// Where the times on the axes, the logs and the alerts are read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Zone {
    /// The computer's own zone.
    #[default]
    Local,
    Utc,
}

/// The app's preferences, as the web's `Preferences`.
#[derive(Clone, Debug, PartialEq)]
pub struct Preferences {
    /// The range a dashboard opens on until the reader picks one, by its preset's id.
    pub range: &'static str,
    /// The refresh a dashboard opens on until the reader picks one, in seconds.
    pub refresh: u32,
    pub zone: Zone,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            range: "15m",
            refresh: 5,
            zone: Zone::Local,
        }
    }
}

/// The place of the Dashboards page, beside the dashboards' own ids.
pub const DASHBOARDS: &str = "dashboards";

/// The place of the settings, opened over the page shown.
pub const SETTINGS: &str = "settings";

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
    pub tables: std::collections::HashMap<String, TableState>,
    /// A panel open in its window, inspected, or asked to be deleted.
    pub panels: crate::panels::PanelDialogs,
    /// A panel being edited or made, its page in the dashboard's place.
    pub editor: Option<crate::editor::EditorState>,
    /// The Dashboards page's list.
    pub(crate) list: TableState,
    pub prefs: Preferences,
    /// How far the computer's zone is from UTC, in seconds, read as the app starts.
    pub local_offset: f64,
    /// The reader picked the range or the refresh, which the preferences then leave be.
    pub range_chosen: bool,
    pub(crate) refresh_chosen: bool,
    /// The settings over the page, open on the section named.
    pub settings: Option<String>,
    pub(crate) resetting: bool,
    /// The panels being arranged: a draft Done keeps and Cancel throws away.
    pub edit: DashboardEdit<Panel>,
    /// How many panels and copies have been made, for their ids.
    pub(crate) made: u32,
    /// What the app has said since the last frame, shown as toasts on the next.
    pub(crate) said: Vec<Toast>,
    /// The dashboard's details being changed.
    pub details: Option<crate::board::DetailsForm>,
    pub asking: Option<crate::board::Asking>,
}

impl MetricsApp {
    /// The app in its window, its times read `local_offset` seconds from UTC, the computer's zone.
    pub fn new(_cc: &eframe::CreationContext<'_>, local_offset: f64) -> Self {
        let mut app = Self::on(Appearance::default());
        app.desktop = true;
        app.local_offset = local_offset;
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
            panels: Default::default(),
            editor: None,
            list: TableState::default(),
            prefs: Preferences::default(),
            local_offset: 0.0,
            range_chosen: false,
            refresh_chosen: false,
            settings: None,
            resetting: false,
            edit: DashboardEdit::default(),
            made: 0,
            said: Vec::new(),
            details: None,
            asking: None,
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
        let groups = vec![
            RailGroup::new(None, [RailEntry::new(DASHBOARDS, "Dashboards", Icon::LayoutGrid)]),
            RailGroup::new(
                Some("Dashboards"),
                self.dashboards
                    .iter()
                    .map(|b| RailEntry::new(b.id.clone(), b.title.clone(), Icon::LayoutDashboard))
                    .collect::<Vec<_>>(),
            ),
            RailGroup::new(Some("System"), [RailEntry::new(SETTINGS, "Settings", Icon::Settings2)]),
        ];
        let mut place = self.place.clone();
        let was = place.clone();
        let shell = if self.desktop {
            Shell::new().desktop(DesktopWindow::native())
        } else {
            Shell::new()
        };
        let title = if self.place == DASHBOARDS {
            "Dashboards".to_owned()
        } else {
            self.dashboards
                .iter()
                .find(|b| b.id == self.place)
                .map_or(String::new(), |b| b.title.clone())
        };
        let mut trail = vec!["Metrics", title.as_str()];
        if self.editor.is_some() {
            trail.push("Edit panel");
        }
        let mode = if self.refresh == 0 {
            "refresh off".to_owned()
        } else if self.paused {
            "refresh paused".to_owned()
        } else {
            format!("refresh every {}s", self.refresh)
        };
        let utc = if self.prefs.zone == Zone::Utc { " UTC" } else { "" };
        let note = format!("Scraped {}{utc}, {mode}", clock_of(self.at + self.offset()));
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
            ShellPart::Page if self.editor.is_some() => Page::show(ui, |ui| self.editor_page(ui)),
            ShellPart::Page if self.place == DASHBOARDS => Page::show(ui, |ui| self.dashboards_page(ui)),
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
        // the rail's press moves the place, leaving the editor; what the page did to it stands otherwise
        if place == SETTINGS {
            self.settings = Some("dashboards".into());
        } else if place != was {
            self.place = place;
            self.editor = None;
        }
        self.panel_dialogs(&ui.ctx().clone());
        self.editor_dialogs(&ui.ctx().clone());
        self.settings_dialog(&ui.ctx().clone());
        self.board_dialogs(&ui.ctx().clone());
        match account {
            Some(AccountAction::Scheme(scheme)) => self.appearance.scheme = scheme,
            Some(AccountAction::Settings) => self.settings = Some("dashboards".into()),
            None => {}
        }
    }

    /// A dashboard: its head, then its panels in the grid in the order kept.
    fn board_page(&mut self, ui: &mut Ui) {
        let Some(board) = self.dashboards.iter().find(|b| b.id == self.place).cloned() else {
            return;
        };
        let header = PageHeader::new(&board.title)
            .description(&board.description)
            .edit_title("Title", |value| {
                value.trim().is_empty().then(|| "A dashboard needs a title.".to_owned())
            })
            .show(ui, |ui| self.board_actions(ui, &board));
        if let Some(title) = header.renamed
            && let Some(kept) = self.dashboards.iter_mut().find(|b| b.id == board.id)
        {
            kept.title = title.trim().to_owned();
        }
        // the variables right under the head, the page's own gap before the panels
        self.variables_bar(ui);
        let view = self.view();
        let palette = self.palette(&board);
        let editing = self.edit.editing();
        let Some(index) = self.dashboards.iter().position(|b| b.id == board.id) else {
            return;
        };
        if !editing && self.dashboards[index].panels.is_empty() {
            let add = sokkel_egui::Empty::new("No panels yet")
                .icon(Icon::LayoutDashboard)
                .description("Add a panel, pick what it reads and how it draws it, and it lands here.")
                .show(ui, |ui| {
                    ui.add(
                        sokkel_egui::Button::new("Add panel")
                            .icon(Icon::Plus)
                            .size(sokkel_egui::ButtonSize::Sm),
                    )
                    .clicked()
                });
            if add {
                self.open_editor(None);
            }
            return;
        }
        let layout = self.edit.layout(&mut self.dashboards[index].panels);
        for panel in layout.iter() {
            self.tables.entry(panel.id.clone()).or_default();
        }
        let label = |p: &Panel| p.title.clone();
        let tables = &mut self.tables;
        let viewing = self.panels.viewing.clone();
        let focus = self.panels.focus.take();
        let mut asked = None;
        let arranged = DashboardArrange::new(("board", &board.id), layout, &label)
            .arranging(editing)
            .dense(true)
            .palette(&palette)
            .show(ui, |ui, panel| {
                if let Some(state) = tables.get_mut(&panel.id) {
                    let (items, _) = crate::panels::panel_menu(panel, editing);
                    // a panel the editor just saved is brought into view
                    if focus.as_ref() == Some(&panel.id) {
                        ui.scroll_to_rect(ui.max_rect(), Some(egui::Align::Center));
                    }
                    let said = PanelMenu::new(items)
                        .expandable(panel.kind != crate::model::PanelKind::Stat)
                        .expanded(viewing.as_ref() == Some(&panel.id))
                        .show(ui, |ui| draw_panel(ui, panel, &view, state));
                    if let Some(said) = said {
                        asked = Some((panel.clone(), said));
                    }
                }
            });
        if let Some((panel, said)) = asked {
            self.panel_asked(&panel, said);
        }
        if let Some((id, at)) = arranged.added
            && let Some(mut made) = crate::model::widget_catalog().into_iter().find(|p| p.id == id)
        {
            made.id = self.panel_id();
            let layout = self.edit.layout(&mut self.dashboards[index].panels);
            let at = at.min(layout.len());
            layout.insert(at, made);
        }
    }
}

impl MetricsApp {
    /// What the panels read now.
    pub fn view(&self) -> View {
        View {
            variables: self.variables.clone(),
            range: self.range.clone(),
            at: self.view_at,
            offset: self.offset(),
        }
    }

    /// How far the zone the reader chose is from UTC, in seconds.
    pub fn offset(&self) -> f64 {
        match self.prefs.zone {
            Zone::Local => self.local_offset,
            Zone::Utc => 0.0,
        }
    }

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
                self.time_controls(ui);
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

    /// The range and the refresh, as the web's `TimeControls`, drawn right to left: the refresh,
    /// then the range before it. A new range reads the panels at once.
    pub(crate) fn time_controls(&mut self, ui: &mut Ui) {
        let before = self.range.clone();
        let refresh = self.refresh;
        let updated = if self.refresh > 0 && self.paused {
            "paused".to_owned()
        } else {
            format!("updated {}", clock_of(self.view_at + self.offset()))
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
        if self.range != before {
            self.view_at = self.at;
            self.range_chosen = true;
        }
        if self.refresh != refresh {
            self.refresh_chosen = true;
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

/// What the panels read, as the web's `View`: the variables, the range, the second they are read
/// up to, and how far the reader's zone is from UTC, in seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub variables: Variables,
    pub range: Range,
    pub at: f64,
    pub offset: f64,
}

/// One panel, read from the source and drawn by its kind, as the web's `PanelView`.
pub(crate) fn draw_panel(ui: &mut Ui, panel: &Panel, view: &View, table: &mut TableState) {
    draw_panel_as(ui, panel, view, table, false);
}

/// One panel as `draw_panel` draws it, or with `preview` as the editor previews it, as the web's
/// `PanelView` with `preview`: taller, and a time series' legend always a table.
pub(crate) fn draw_panel_as(ui: &mut Ui, panel: &Panel, view: &View, table: &mut TableState, preview: bool) {
    let data = panel_data(panel, &view.variables, &view.range, view.at, view.offset);
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
                // h-72 in a preview
                .height(if preview { 288.0 } else { 128.0 })
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
            // more than three series read better as a table of their figures, as the web's; a
            // preview's always is, and it stands h-80
            if preview {
                chart = chart.height(320.0);
            }
            if series.len() > 3 || preview {
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
                // h-56 (h-80 in a preview), or all its window has, as the web's `Sized`
                let height = if sokkel_egui::windowed(ui) {
                    ui.available_height()
                } else if preview {
                    320.0
                } else {
                    224.0
                };
                ui.set_height(height);
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
                    .height(if sokkel_egui::windowed(ui) {
                        ui.available_height()
                    } else if preview {
                        320.0
                    } else {
                        256.0
                    })
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
