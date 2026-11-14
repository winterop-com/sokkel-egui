//! What the dashboards hold, as the web's `model.ts`: each dashboard is a list of widgets in their
//! order, each widget its kind, its width, its own title when given one, and its options. The
//! dashboards Tidevann starts with, and what each kind of widget starts as when added.

use sokkel_ui_egui::{ArrangeItem, PanelSpan};

pub const JOBS: [&str; 5] = [
    "nightly-export",
    "sync-inventory",
    "rebuild-index",
    "send-digest",
    "import-orders",
];
pub const PROJECTS: [&str; 3] = ["harbor", "juniper", "larkspur"];
pub const PEOPLE: [&str; 4] = ["ada", "grace", "linus", "margaret"];
pub const SERVICES: [&str; 3] = ["api", "worker", "scheduler"];
pub const STEPS: [&str; 5] = ["fetch", "transform", "validate", "upload", "notify"];

/// A figure the live source reads every tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Metric {
    Runs,
    Failures,
    Success,
    Queue,
    Latency,
    Cpu,
    Memory,
}

/// A metric's label, unit, decimals and color token, as the web's `METRICS`.
pub struct MetricInfo {
    pub label: &'static str,
    pub unit: &'static str,
    pub decimals: usize,
    pub color: &'static str,
}

impl Metric {
    pub const ALL: [Metric; 7] = [
        Metric::Runs,
        Metric::Failures,
        Metric::Success,
        Metric::Queue,
        Metric::Latency,
        Metric::Cpu,
        Metric::Memory,
    ];

    pub fn info(self) -> MetricInfo {
        let (label, unit, decimals, color) = match self {
            Metric::Runs => ("Runs", "/min", 0, "chart-1"),
            Metric::Failures => ("Failures", "/min", 0, "status-failed"),
            Metric::Success => ("Success rate", "%", 1, "chart-3"),
            Metric::Queue => ("Queue depth", "", 0, "chart-4"),
            Metric::Latency => ("api latency p95", "ms", 0, "chart-2"),
            Metric::Cpu => ("Worker CPU", "%", 0, "chart-5"),
            Metric::Memory => ("Worker memory", "%", 0, "chart-1"),
        };
        MetricInfo {
            label,
            unit,
            decimals,
            color,
        }
    }

    /// Its place among the readings' values.
    pub fn index(self) -> usize {
        Metric::ALL.iter().position(|m| *m == self).unwrap_or(0)
    }
}

/// The kinds of widget a dashboard holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetKind {
    Stat,
    Trend,
    Bars,
    Gauge,
    Runs,
    Activity,
    Log,
    Map,
    Note,
    Todo,
    Upcoming,
    Uptime,
    Oncall,
    Projects,
}

/// A trend drawn as a line or a filled area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Line,
    Area,
}

/// What the bars count runs by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Job,
    Project,
    Person,
}

/// What a widget can be set to, each kind reading its own, as the web's `WidgetOptions`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetOptions {
    /// The figure a stat, a trend or a gauge reads.
    pub metric: Option<Metric>,
    /// A second figure a trend draws beside the first; `None` inside for none.
    pub compare: Option<Option<Metric>>,
    pub style: Option<Style>,
    pub group: Option<Group>,
    /// The service whose log is followed.
    pub service: Option<&'static str>,
    /// How many rows a list shows.
    pub rows: Option<usize>,
    /// Only failed runs in the runs table.
    pub failed_only: Option<bool>,
    /// The note's text, in markdown.
    pub text: Option<String>,
    /// Hides the to-dos already done.
    pub hide_done: Option<bool>,
    /// Shows who is on call next.
    pub show_next: Option<bool>,
}

/// A widget in its place: its kind, its width, its own title when given one, and its options.
#[derive(Clone, Debug, PartialEq)]
pub struct Widget {
    pub id: String,
    pub kind: WidgetKind,
    pub span: PanelSpan,
    pub title: Option<String>,
    pub options: WidgetOptions,
}

impl ArrangeItem for Widget {
    fn id(&self) -> &str {
        &self.id
    }

    fn span(&self) -> PanelSpan {
        self.span
    }

    fn set_span(&mut self, span: PanelSpan) {
        self.span = span;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dashboard {
    pub id: String,
    pub name: String,
    pub widgets: Vec<Widget>,
    /// What Reset puts back: the widgets it started with.
    pub base: Vec<Widget>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Todo {
    pub id: String,
    /// The to-do widget it belongs to.
    pub widget: String,
    pub text: String,
    pub done: bool,
}

/// How often the live source reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    Fast,
    Normal,
    Slow,
}

impl Pace {
    pub const ALL: [Pace; 3] = [Pace::Fast, Pace::Normal, Pace::Slow];

    pub fn label(self) -> &'static str {
        match self {
            Pace::Fast => "Every second",
            Pace::Normal => "Every 3 seconds",
            Pace::Slow => "Every 10 seconds",
        }
    }

    /// Milliseconds between readings.
    pub fn every(self) -> u64 {
        match self {
            Pace::Fast => 1000,
            Pace::Normal => 3000,
            Pace::Slow => 10_000,
        }
    }
}

/// The person's preferences, as the web's `Prefs`.
#[derive(Clone, Debug, PartialEq)]
pub struct Prefs {
    /// The dashboards in the order the rail lists them.
    pub order: Vec<String>,
    pub live: bool,
    pub pace: Pace,
    pub twelve_hour: bool,
    /// Whether leaving edit mode with changes asks first.
    pub confirm_cancel: bool,
}

impl Prefs {
    pub fn new(order: Vec<String>) -> Self {
        Self {
            order,
            live: true,
            pace: Pace::Normal,
            twelve_hour: false,
            confirm_cancel: true,
        }
    }
}

/// Makes fresh widget ids, as the web's `widgetId`, counting up so they read the same every run.
#[derive(Clone, Debug, Default)]
pub struct Ids {
    made: u32,
}

impl Ids {
    pub fn next(&mut self, kind: WidgetKind) -> String {
        self.made += 1;
        format!("{}-{}", kind_word(kind), self.made)
    }
}

/// The kind as the web writes it in ids.
pub fn kind_word(kind: WidgetKind) -> &'static str {
    match kind {
        WidgetKind::Stat => "stat",
        WidgetKind::Trend => "trend",
        WidgetKind::Bars => "bars",
        WidgetKind::Gauge => "gauge",
        WidgetKind::Runs => "runs",
        WidgetKind::Activity => "activity",
        WidgetKind::Log => "log",
        WidgetKind::Map => "map",
        WidgetKind::Note => "note",
        WidgetKind::Todo => "todo",
        WidgetKind::Upcoming => "upcoming",
        WidgetKind::Uptime => "uptime",
        WidgetKind::Oncall => "oncall",
        WidgetKind::Projects => "projects",
    }
}

fn w(id: &str, kind: WidgetKind, span: PanelSpan, options: WidgetOptions, title: Option<&str>) -> Widget {
    Widget {
        id: id.into(),
        kind,
        span,
        title: title.map(Into::into),
        options,
    }
}

fn metric(metric: Metric) -> WidgetOptions {
    WidgetOptions {
        metric: Some(metric),
        ..Default::default()
    }
}

fn trend(m: Metric, compare: Option<Metric>, style: Style) -> WidgetOptions {
    WidgetOptions {
        metric: Some(m),
        compare: Some(compare),
        style: Some(style),
        ..Default::default()
    }
}

fn rows(rows: usize) -> WidgetOptions {
    WidgetOptions {
        rows: Some(rows),
        ..Default::default()
    }
}

fn group(group: Group) -> WidgetOptions {
    WidgetOptions {
        group: Some(group),
        ..Default::default()
    }
}

fn text(text: &str) -> WidgetOptions {
    WidgetOptions {
        text: Some(text.into()),
        ..Default::default()
    }
}

fn show_next() -> WidgetOptions {
    WidgetOptions {
        show_next: Some(true),
        ..Default::default()
    }
}

const WELCOME: &str =
    "**Welcome to Tidevann.** This is the team's home: the runs, the services and who is on call, live.

- Press **Edit** to move, resize, configure or remove widgets
- Drag a widget in from the palette to add it
- **Cancel** throws the edits away, **Reset** puts the dashboard back";

const HANDOVER: &str = "### Hand-over

The person on call keeps an eye on **Recent failures** and hands over at 09:00.

1. Read the log of the worker
2. Retry what failed overnight
3. Write down what is left here";

/// The dashboards Tidevann starts with, as the web's `PRESETS`.
pub fn presets() -> Vec<Dashboard> {
    use PanelSpan::*;
    use WidgetKind as K;
    let boards = [
        (
            "overview",
            "Overview",
            vec![
                w("o-runs", K::Stat, Sixth, metric(Metric::Runs), None),
                w("o-success", K::Stat, Sixth, metric(Metric::Success), None),
                w("o-failures", K::Stat, Sixth, metric(Metric::Failures), None),
                w("o-queue", K::Stat, Sixth, metric(Metric::Queue), None),
                w("o-latency", K::Stat, Sixth, metric(Metric::Latency), None),
                w("o-cpu", K::Gauge, Sixth, metric(Metric::Cpu), None),
                w(
                    "o-trend",
                    K::Trend,
                    TwoThirds,
                    trend(Metric::Runs, Some(Metric::Failures), Style::Area),
                    None,
                ),
                w("o-note", K::Note, Third, text(WELCOME), Some("Read me")),
                w("o-runs-table", K::Runs, Half, rows(6), None),
                w("o-activity", K::Activity, Half, rows(8), None),
                w("o-bars", K::Bars, Third, group(Group::Job), None),
                w("o-upcoming", K::Upcoming, Third, rows(6), None),
                w("o-oncall", K::Oncall, Third, show_next(), None),
            ],
        ),
        (
            "operations",
            "Operations",
            vec![
                w("p-latency", K::Stat, Quarter, metric(Metric::Latency), None),
                w("p-cpu", K::Gauge, Quarter, metric(Metric::Cpu), None),
                w("p-memory", K::Gauge, Quarter, metric(Metric::Memory), None),
                w("p-queue", K::Stat, Quarter, metric(Metric::Queue), None),
                w("p-map", K::Map, TwoThirds, WidgetOptions::default(), None),
                w("p-uptime", K::Uptime, Third, WidgetOptions::default(), None),
                w(
                    "p-trend",
                    K::Trend,
                    Half,
                    trend(Metric::Latency, None, Style::Line),
                    None,
                ),
                w(
                    "p-queue-trend",
                    K::Trend,
                    Half,
                    trend(Metric::Queue, None, Style::Area),
                    None,
                ),
                w(
                    "p-log",
                    K::Log,
                    TwoThirds,
                    WidgetOptions {
                        service: Some("worker"),
                        ..Default::default()
                    },
                    None,
                ),
                w(
                    "p-failed",
                    K::Runs,
                    Third,
                    WidgetOptions {
                        rows: Some(6),
                        failed_only: Some(true),
                        ..Default::default()
                    },
                    Some("Recent failures"),
                ),
            ],
        ),
        (
            "team",
            "Team",
            vec![
                w("t-oncall", K::Oncall, Third, show_next(), None),
                w(
                    "t-todo",
                    K::Todo,
                    Third,
                    WidgetOptions {
                        hide_done: Some(false),
                        ..Default::default()
                    },
                    Some("This week"),
                ),
                w("t-upcoming", K::Upcoming, Third, rows(6), None),
                w(
                    "t-people",
                    K::Bars,
                    Half,
                    group(Group::Person),
                    Some("Runs started by person"),
                ),
                w("t-projects", K::Projects, Half, WidgetOptions::default(), None),
                w("t-activity", K::Activity, TwoThirds, rows(10), None),
                w("t-note", K::Note, Third, text(HANDOVER), Some("On-call notes")),
            ],
        ),
    ];
    boards
        .into_iter()
        .map(|(id, name, widgets)| Dashboard {
            id: id.into(),
            name: name.into(),
            base: widgets.clone(),
            widgets,
        })
        .collect()
}

/// The to-dos the team dashboard starts with.
pub fn seed_todos() -> Vec<Todo> {
    [
        ("todo-1", "Move nightly-export to the new worker", false),
        ("todo-2", "Review the larkspur sync schedule with grace", false),
        ("todo-3", "Rotate the api keys for harbor", true),
        ("todo-4", "Write up why rebuild-index was slow on Monday", false),
    ]
    .into_iter()
    .map(|(id, text, done)| Todo {
        id: id.into(),
        widget: "t-todo".into(),
        text: text.into(),
        done,
    })
    .collect()
}

/// What each palette entry adds: its kind, its width and its options.
pub struct PaletteEntry {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: WidgetKind,
    pub span: PanelSpan,
    pub options: WidgetOptions,
}

/// The palette, as the web's `PALETTE`.
pub fn palette() -> Vec<PaletteEntry> {
    use PanelSpan::*;
    use WidgetKind as K;
    let e = |id, label, kind, span, options| PaletteEntry {
        id,
        label,
        kind,
        span,
        options,
    };
    vec![
        e("stat", "Stat", K::Stat, Sixth, metric(Metric::Runs)),
        e(
            "line",
            "Line chart",
            K::Trend,
            Half,
            trend(Metric::Latency, None, Style::Line),
        ),
        e(
            "area",
            "Area chart",
            K::Trend,
            Half,
            trend(Metric::Runs, Some(Metric::Failures), Style::Area),
        ),
        e("bar", "Bar chart", K::Bars, Third, group(Group::Job)),
        e("gauge", "Gauge", K::Gauge, Sixth, metric(Metric::Cpu)),
        e("runs", "Recent runs", K::Runs, Half, rows(6)),
        e("activity", "Activity", K::Activity, Third, rows(8)),
        e(
            "log",
            "Log",
            K::Log,
            Half,
            WidgetOptions {
                service: Some("api"),
                ..Default::default()
            },
        ),
        e("map", "Sites map", K::Map, Half, WidgetOptions::default()),
        e(
            "note",
            "Note",
            K::Note,
            Third,
            text("A note for the team, in **markdown**."),
        ),
        e(
            "todo",
            "To-do list",
            K::Todo,
            Third,
            WidgetOptions {
                hide_done: Some(false),
                ..Default::default()
            },
        ),
        e("upcoming", "Upcoming", K::Upcoming, Third, rows(6)),
        e("uptime", "Uptime", K::Uptime, Third, WidgetOptions::default()),
        e("oncall", "On call", K::Oncall, Third, show_next()),
        e("projects", "Projects", K::Projects, Third, WidgetOptions::default()),
    ]
}

/// A copy of widgets under fresh ids, and which new id each old one took.
pub fn copy_widgets(widgets: &[Widget], ids: &mut Ids) -> (Vec<Widget>, Vec<(String, String)>) {
    let mut taken = Vec::new();
    let copied = widgets
        .iter()
        .map(|widget| {
            let id = ids.next(widget.kind);
            taken.push((widget.id.clone(), id.clone()));
            Widget { id, ..widget.clone() }
        })
        .collect();
    (copied, taken)
}
