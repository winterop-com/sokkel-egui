//! Every kind of widget a dashboard can hold, as the web's `widgets.tsx`, each drawn from the live
//! source with its options: stat tiles, line, area and bar charts, a gauge, the recent runs, the
//! activity, a service's log, the sites on a map, a note, a to-do list, what is coming up, uptime,
//! who is on call, and the projects' share of the runs.

use std::collections::HashMap;

use eframe::egui::{self, Ui};
use sokkel_ui_egui::{
    ActivityItem, ActivityList, ActivityTone, BarList, BarListItem, Button, ButtonSize, ButtonVariant, Cell, Checkbox,
    ChipTone, Column, DashboardPanel, DataTable, Empty, Gap, Icon, ListRow, ListRows, LogView, Markdown, QuickAdd,
    RowAside, RowMark, Stack, StackLayout, StackPlace, StatPanel, TableState, TableVariant, TextRole, Threshold,
    UptimeBar, tokens,
};
use sokkel_ui_egui_charts::{BarPanel, Fill, GaugePanel, Series, TimeSeriesPanel};
use sokkel_ui_egui_maps::{MapLegend, MapPopup, MapPopupRow, MapView, Marker};

use crate::live::{Health, Live, RunState, Tone, clock_of, on_call, upcoming};
use crate::model::{Group, JOBS, Metric, PEOPLE, PROJECTS, SERVICES, Style, Todo, Widget, WidgetKind};

/// Each kind's name and icon, as the palette and the configure dialog say them.
pub fn kind_info(kind: WidgetKind) -> (&'static str, Icon) {
    match kind {
        WidgetKind::Stat => ("Stat", Icon::Gauge),
        WidgetKind::Trend => ("Chart", Icon::ChartLine),
        WidgetKind::Bars => ("Bar chart", Icon::ChartColumn),
        WidgetKind::Gauge => ("Gauge", Icon::CircleGauge),
        WidgetKind::Runs => ("Recent runs", Icon::Table2),
        WidgetKind::Activity => ("Activity", Icon::Activity),
        WidgetKind::Log => ("Log", Icon::ScrollText),
        WidgetKind::Map => ("Sites map", Icon::Globe),
        WidgetKind::Note => ("Note", Icon::NotebookPen),
        WidgetKind::Todo => ("To-do list", Icon::ListChecks),
        WidgetKind::Upcoming => ("Upcoming", Icon::CalendarClock),
        WidgetKind::Uptime => ("Uptime", Icon::SignalHigh),
        WidgetKind::Oncall => ("On call", Icon::PhoneCall),
        WidgetKind::Projects => ("Projects", Icon::FolderKanban),
    }
}

/// The icon of a palette entry, by the look it adds, as the web's `PALETTE_ICONS`.
pub fn palette_icon(entry: &str, kind: WidgetKind) -> Icon {
    match entry {
        "area" => Icon::ChartArea,
        "line" => Icon::ChartLine,
        _ => kind_info(kind).1,
    }
}

/// The title a widget shows: its own, or what its kind and options make it.
pub fn title_of(widget: &Widget) -> String {
    if let Some(title) = widget.title.as_ref().filter(|t| !t.trim().is_empty()) {
        return title.clone();
    }
    let o = &widget.options;
    let metric = |fallback| o.metric.unwrap_or(fallback).info().label.to_owned();
    match widget.kind {
        WidgetKind::Stat => metric(Metric::Runs),
        WidgetKind::Gauge => metric(Metric::Cpu),
        WidgetKind::Trend => match o.compare.flatten() {
            Some(compare) => format!("{} and {}", metric(Metric::Runs), compare.info().label.to_lowercase()),
            None => metric(Metric::Runs),
        },
        WidgetKind::Bars => format!(
            "Runs by {}",
            match o.group.unwrap_or(Group::Job) {
                Group::Job => "job",
                Group::Project => "project",
                Group::Person => "person",
            }
        ),
        WidgetKind::Log => format!("{} log", o.service.unwrap_or("api")),
        WidgetKind::Runs if o.failed_only == Some(true) => "Recent failures".into(),
        WidgetKind::Runs => "Recent runs".into(),
        kind => kind_info(kind).0.into(),
    }
}

/// The colors a reading takes as it rises, where a high one is bad (a low one, for success).
pub fn high_is_bad(metric: Metric) -> &'static [Threshold] {
    const FAILURES: [Threshold; 3] = [
        Threshold::base("good"),
        Threshold::from(4.0, "warning"),
        Threshold::from(8.0, "critical"),
    ];
    const QUEUE: [Threshold; 3] = [
        Threshold::base("good"),
        Threshold::from(25.0, "warning"),
        Threshold::from(45.0, "critical"),
    ];
    const LATENCY: [Threshold; 3] = [
        Threshold::base("good"),
        Threshold::from(200.0, "warning"),
        Threshold::from(300.0, "critical"),
    ];
    const CPU: [Threshold; 3] = [
        Threshold::base("good"),
        Threshold::from(70.0, "warning"),
        Threshold::from(85.0, "critical"),
    ];
    const MEMORY: [Threshold; 3] = [
        Threshold::base("good"),
        Threshold::from(75.0, "warning"),
        Threshold::from(88.0, "critical"),
    ];
    const SUCCESS: [Threshold; 3] = [
        Threshold::base("critical"),
        Threshold::from(90.0, "warning"),
        Threshold::from(95.0, "good"),
    ];
    match metric {
        Metric::Failures => &FAILURES,
        Metric::Queue => &QUEUE,
        Metric::Latency => &LATENCY,
        Metric::Cpu => &CPU,
        Metric::Memory => &MEMORY,
        Metric::Success => &SUCCESS,
        Metric::Runs => &[],
    }
}

/// How long ago, said shortly, as the web's `ago`.
pub fn ago(at: f64, now: f64) -> String {
    let seconds = ((now - at) / 1000.0).round().max(0.0);
    if seconds < 10.0 {
        return "just now".into();
    }
    if seconds < 60.0 {
        return format!("{seconds} s ago");
    }
    let minutes = (seconds / 60.0).round();
    if minutes < 60.0 {
        return format!("{minutes} min ago");
    }
    format!("{} h ago", (minutes / 60.0).round())
}

/// How long until, said shortly, as the web's `within`.
pub fn within(at: f64, now: f64) -> String {
    let minutes = ((at - now) / 60_000.0).round().max(0.0) as i64;
    if minutes < 1 {
        return "now".into();
    }
    if minutes < 60 {
        return format!("in {minutes} min");
    }
    let hours = minutes / 60;
    if minutes % 60 == 0 {
        format!("in {hours} h")
    } else {
        format!("in {hours} h {} min", minutes % 60)
    }
}

/// What the widgets keep between frames, beside the dashboard: each map's view, each runs table,
/// each to-do list's text being typed, and the to-dos themselves.
#[derive(Default)]
pub struct WidgetState {
    pub maps: HashMap<String, MapView>,
    pub tables: HashMap<String, TableState>,
    pub typing: HashMap<String, String>,
}

/// What a widget asked of the page this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WidgetAsk {
    /// Open its options, as an empty note's button does while editing.
    Configure,
}

/// Draws a widget from the live source, as the web's `WidgetView`.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    ui: &mut Ui,
    widget: &Widget,
    live: &Live,
    twelve: bool,
    editing: bool,
    todos: &mut Vec<Todo>,
    state: &mut WidgetState,
) -> Option<WidgetAsk> {
    let title = title_of(widget);
    let o = &widget.options;
    match widget.kind {
        WidgetKind::Stat => {
            let metric = o.metric.unwrap_or(Metric::Runs);
            let info = metric.info();
            let (now, values) = live.metric(metric);
            let before = values[values.len().saturating_sub(11)];
            let change = if before == 0.0 {
                0.0
            } else {
                (now - before) / before * 100.0
            };
            let better = if matches!(metric, Metric::Runs | Metric::Success) {
                change >= 0.0
            } else {
                change <= 0.0
            };
            let thresholds = high_is_bad(metric);
            let mut panel = StatPanel::new(title, Some(now))
                .unit(info.unit)
                .decimals(info.decimals)
                .thresholds(thresholds)
                .sparkline(&values)
                .delta(format!("{}{:.0}%", if change >= 0.0 { "+" } else { "" }, change))
                .good((change.abs() >= 1.0).then_some(better));
            if thresholds.is_empty() {
                panel = panel.color(info.color);
            }
            panel.show(ui);
        }
        WidgetKind::Gauge => {
            let metric = o.metric.unwrap_or(Metric::Cpu);
            let info = metric.info();
            let (now, _) = live.metric(metric);
            let max = match metric {
                Metric::Latency => 500.0,
                Metric::Queue => 60.0,
                _ => 100.0,
            };
            GaugePanel::new(title, Some(now))
                .range(0.0, max)
                .unit(info.unit)
                .decimals(info.decimals)
                .thresholds(high_is_bad(metric))
                .color(info.color)
                .show(ui);
        }
        WidgetKind::Trend => {
            let metric = o.metric.unwrap_or(Metric::Runs);
            let compare = o.compare.flatten().filter(|c| *c != metric);
            let x: Vec<String> = live.readings.iter().map(|r| clock_of(r.at, twelve, true)).collect();
            let unit = match compare {
                Some(c) if c.info().unit != metric.info().unit => "",
                _ => metric.info().unit,
            };
            let mut panel = TimeSeriesPanel::new(title, x)
                .subtitle("live")
                .unit(unit)
                .fill(if o.style == Some(Style::Area) {
                    Fill::Gradient
                } else {
                    Fill::None
                })
                .label_every(15)
                .legend(compare.is_some());
            for one in std::iter::once(metric).chain(compare) {
                panel = panel.series(Series::new(one.info().label, live.metric(one).1).color(one.info().color));
            }
            panel.show(ui);
        }
        WidgetKind::Bars => {
            let group = o.group.unwrap_or(Group::Job);
            let (names, counts): (Vec<&str>, Vec<u32>) = match group {
                Group::Job => (JOBS.to_vec(), live.counts.job.to_vec()),
                Group::Project => (PROJECTS.to_vec(), live.counts.project.to_vec()),
                Group::Person => (PEOPLE.to_vec(), live.counts.person.to_vec()),
            };
            BarPanel::new(title, names)
                .subtitle("since midnight")
                .series(Series::new("Runs", counts.iter().map(|c| f64::from(*c))).color("chart-1"))
                .horizontal(group != Group::Project)
                .legend(false)
                .show(ui);
        }
        WidgetKind::Runs => {
            let failed_only = o.failed_only == Some(true);
            let shown: Vec<_> = live
                .runs
                .iter()
                .filter(|r| !failed_only || r.state == RunState::Failed)
                .take(o.rows.unwrap_or(6))
                .collect();
            let empty = if failed_only {
                "Nothing has failed lately."
            } else {
                "No run yet."
            };
            DashboardPanel::new(title).subtitle("live").flush(true).show(ui, |ui| {
                if shown.is_empty() {
                    Empty::new(empty).show(ui, |_| {});
                    return;
                }
                let columns = [
                    Column::new("Job", 128.0).fill().stack(StackPlace::Primary),
                    Column::new("Project", 96.0),
                    Column::new("State", 112.0).stack(StackPlace::Secondary),
                    Column::new("Started", 80.0).numeric(),
                ];
                let rows: Vec<Vec<Cell>> = shown
                    .iter()
                    .map(|r| {
                        vec![
                            Cell::Mono(r.job.into()),
                            Cell::Text(r.project.into()),
                            Cell::Chip(r.state.word().into(), ChipTone::Status(r.state.word())),
                            Cell::Text(clock_of(r.started, twelve, false)),
                        ]
                    })
                    .collect();
                let table = state.tables.entry(widget.id.clone()).or_default();
                DataTable::new(("runs", &widget.id), &columns, &rows, table)
                    .variant(TableVariant::List)
                    .stack_layout(StackLayout::Labeled)
                    .stack_below(360.0)
                    .framed(false)
                    .footer(false)
                    .show(ui);
            });
        }
        WidgetKind::Activity => {
            let items: Vec<ActivityItem> = live
                .happenings
                .iter()
                .take(o.rows.unwrap_or(8))
                .map(|h| ActivityItem {
                    id: h.id.clone(),
                    text: h.text.clone(),
                    when: ago(h.at, live.at),
                    tone: match h.tone {
                        Tone::Critical => ActivityTone::Critical,
                        Tone::Warning => ActivityTone::Warning,
                        Tone::Info => ActivityTone::Info,
                        Tone::Good => ActivityTone::Good,
                        Tone::Quiet => ActivityTone::Quiet,
                    },
                    ..Default::default()
                })
                .collect();
            DashboardPanel::new(title).subtitle("live").flush(true).show(ui, |ui| {
                if items.is_empty() {
                    Empty::new("Nothing has happened yet.").show(ui, |_| {});
                } else {
                    ActivityList::new(&items).head(false).show(ui);
                }
            });
        }
        WidgetKind::Log => {
            let service = o.service.unwrap_or("api");
            let index = SERVICES.iter().position(|s| *s == service).unwrap_or(0);
            DashboardPanel::new(title)
                .subtitle("following")
                .flush(true)
                .show(ui, |ui| {
                    LogView::new(("log", &widget.id), &live.logs[index])
                        .follow(true)
                        .framed(false)
                        .show(ui);
                });
        }
        WidgetKind::Map => {
            let t = tokens(ui.ctx());
            let color = |h: Health| match h {
                Health::Operational => t.color("good"),
                Health::Degraded => t.color("warning"),
                Health::Outage => t.color("critical"),
            };
            let markers: Vec<Marker> = live
                .sites
                .iter()
                // points alone, as the web's point layer; a point pressed opens its card
                .map(|s| Marker::new(s.at[1], s.at[0], "").id(s.id).color(color(s.state)))
                .collect();
            let count = |h: Health| live.sites.iter().filter(|s| s.state == h).count();
            let legend = MapLegend::new("Sites")
                .description(format!("{} cities", live.sites.len()))
                .row(color(Health::Operational), "Operational", count(Health::Operational))
                .row(color(Health::Degraded), "Slow", count(Health::Degraded))
                .row(color(Health::Outage), "Down", count(Health::Outage));
            let map = state
                .maps
                .entry(widget.id.clone())
                .or_insert_with(|| MapView::new(25.0, 10.0, 1.0).framed(false));
            DashboardPanel::new(title)
                .subtitle("round trips, live")
                .flush(true)
                .show(ui, |ui| {
                    let size = egui::vec2(ui.available_width(), 320.0);
                    let response = map.show(ui, size, &markers, &[]);
                    legend.show(ui, response.rect);
                    let chosen = map.selected.clone();
                    if let Some(site) = live.sites.iter().find(|s| Some(s.id) == chosen.as_deref()) {
                        let words = match site.state {
                            Health::Operational => "Operational",
                            Health::Degraded => "Slow",
                            Health::Outage => "Down",
                        };
                        MapPopup::new(site.city)
                            .description(site.country)
                            .row(MapPopupRow::new("State", words).color(color(site.state)))
                            .row(MapPopupRow::new("Round trip", format!("{:.0} ms", site.latency)))
                            .show(ui, map);
                    }
                });
        }
        WidgetKind::Note => {
            let text = o.text.clone().unwrap_or_default();
            let mut ask = None;
            DashboardPanel::new(title).show(ui, |ui| {
                if !text.trim().is_empty() {
                    Markdown::new(&text).show(ui);
                } else if editing {
                    let button = Button::new("Write the note")
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm);
                    if ui.add(button).clicked() {
                        ask = Some(WidgetAsk::Configure);
                    }
                } else {
                    let t = tokens(ui.ctx());
                    ui.label(
                        TextRole::Body
                            .rich(&t, "This note is empty.")
                            .color(t.color("muted-foreground")),
                    );
                }
            });
            return ask;
        }
        WidgetKind::Todo => todo(ui, widget, &title, todos, state),
        WidgetKind::Upcoming => {
            let fires = upcoming(live.at, o.rows.unwrap_or(6));
            let rows: Vec<ListRow> = fires
                .iter()
                .map(|f| {
                    ListRow::new(f.job)
                        .mono(true)
                        .detail(format!("{}, {}", f.project, f.what))
                        .mark(RowMark::Icon(Icon::CalendarClock, None))
                        .aside(RowAside::Text(clock_of(f.at, twelve, false)))
                })
                .collect();
            DashboardPanel::new(title)
                .subtitle("on the schedules")
                .flush(true)
                .show(ui, |ui| {
                    ListRows::new(rows).flush(true).show(ui);
                });
        }
        WidgetKind::Uptime => {
            DashboardPanel::new(title).subtitle("the last 90 days").show(ui, |ui| {
                let t = tokens(ui.ctx());
                Stack::new(Gap::Lg).show(ui, |ui| {
                    for (i, service) in SERVICES.iter().enumerate() {
                        Stack::new(Gap::Xs).show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(*service)
                                    .font(egui::FontId::monospace(TextRole::Body.font(&t).size))
                                    .color(t.color("foreground")),
                            );
                            UptimeBar::new(("uptime", &widget.id, i), &live.uptime[i]).show(ui);
                        });
                    }
                });
            });
        }
        WidgetKind::Oncall => {
            let (now, next, _, until) = on_call(live.at);
            let mut rows = vec![
                ListRow::new(now)
                    .detail(format!("On call until {} tomorrow", clock_of(until, twelve, false)))
                    .mark(RowMark::Person(now.into()))
                    .aside(RowAside::Chip("now".into(), ChipTone::Status("running"))),
            ];
            if o.show_next != Some(false) {
                rows.push(
                    ListRow::new(next)
                        .detail("Next on call")
                        .mark(RowMark::Person(next.into()))
                        .aside(RowAside::Text(within(until, live.at))),
                );
            }
            DashboardPanel::new(title)
                .subtitle("hands over at 09:00")
                .flush(true)
                .show(ui, |ui| {
                    ListRows::new(rows).flush(true).show(ui);
                });
        }
        WidgetKind::Projects => {
            let total: u32 = live.counts.project.iter().sum();
            let items: Vec<BarListItem> = PROJECTS
                .iter()
                .zip(live.counts.project)
                .map(|(project, count)| BarListItem {
                    label: (*project).into(),
                    value: if total == 0 {
                        0.0
                    } else {
                        f64::from(count) / f64::from(total)
                    },
                    chosen: false,
                    note: Some(format!("{count} runs")),
                })
                .collect();
            DashboardPanel::new(title)
                .subtitle("share of today's runs")
                .show(ui, |ui| {
                    BarList::new(&items).show(ui);
                });
        }
    }
    None
}

/// A to-do list of the widget's own: add, tick, and clear what is done, as the web's `TodoWidget`.
fn todo(ui: &mut Ui, widget: &Widget, title: &str, todos: &mut Vec<Todo>, state: &mut WidgetState) {
    let hide_done = widget.options.hide_done == Some(true);
    let open = todos.iter().filter(|t| t.widget == widget.id && !t.done).count();
    let done = todos.iter().filter(|t| t.widget == widget.id && t.done).count();
    let mut clear = false;
    let mut added = None;
    let mut panel = DashboardPanel::new(title).subtitle(format!("{open} open"));
    if done > 0 {
        panel = panel.actions(|ui| {
            let button = Button::new("Clear done")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Xs);
            clear = ui.add(button).clicked();
        });
    }
    let typing = state.typing.entry(widget.id.clone()).or_default();
    panel.show(ui, |ui| {
        Stack::new(Gap::Md).show(ui, |ui| {
            let width = ui.available_width();
            added = QuickAdd::new(("todo", &widget.id), typing)
                .placeholder("Add a to-do")
                .width(width)
                .show(ui);
            let mine: Vec<&mut Todo> = todos
                .iter_mut()
                .filter(|t| t.widget == widget.id && (!hide_done || !t.done))
                .collect();
            let t = tokens(ui.ctx());
            if mine.is_empty() {
                let words = if open + done == 0 {
                    "Nothing to do yet."
                } else {
                    "Everything is done."
                };
                ui.label(TextRole::Body.rich(&t, words).color(t.color("muted-foreground")));
            } else {
                Stack::new(Gap::Sm).show(ui, |ui| {
                    for todo in mine {
                        ui.add(Checkbox::new(&mut todo.done, todo.text.clone()));
                    }
                });
            }
        });
    });
    if clear {
        todos.retain(|t| t.widget != widget.id || !t.done);
    }
    if let Some(text) = added {
        let n = todos.len() + 1;
        todos.push(Todo {
            id: format!("todo-{}-{n}", widget.id),
            widget: widget.id.clone(),
            text,
            done: false,
        });
    }
}
