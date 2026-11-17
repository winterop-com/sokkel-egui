//! A dashboard and its panels, as the web's `model.ts`, and the demo's three dashboards, as its
//! `data.ts` seeds them.

use sokkel_egui::{PanelSpan, Threshold};

use crate::query::Reduce;
use crate::source::{Aggregation, Label, MetricId};

/// How a panel draws its query.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelKind {
    TimeSeries,
    Stat,
    Gauge,
    BarGauge,
    Bar,
    Heatmap,
    Table,
    Logs,
}

pub const KINDS: [(PanelKind, &str); 8] = [
    (PanelKind::TimeSeries, "Time series"),
    (PanelKind::Stat, "Stat"),
    (PanelKind::Gauge, "Gauge"),
    (PanelKind::BarGauge, "Bar gauge"),
    (PanelKind::Bar, "Bars"),
    (PanelKind::Heatmap, "Heatmap"),
    (PanelKind::Table, "Table"),
    (PanelKind::Logs, "Logs"),
];

/// A time series' fill under its lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    None,
    Gradient,
    Solid,
}

/// Where a stat tile shows its threshold color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Value,
    Background,
    None,
}

/// One panel, every field the web's `Panel` has.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    pub id: String,
    pub title: String,
    pub kind: PanelKind,
    pub metric: MetricId,
    pub aggregation: Aggregation,
    pub group_by: Option<Label>,
    pub reduce: Reduce,
    /// Labels the panel always filters by, before the dashboard's variables.
    pub filter: Vec<(Label, String)>,
    /// The unit written after each figure; the metric's own when `None`.
    pub unit: Option<String>,
    /// Decimals; the metric's own when `None`.
    pub decimals: Option<usize>,
    pub thresholds: Vec<Threshold>,
    /// The color token of a single series, a tile or a dial with no thresholds.
    pub color: &'static str,
    pub span: PanelSpan,
    pub fill: Fill,
    pub stacked: bool,
    pub color_mode: ColorMode,
    /// A gauge's and a bar gauge's ends.
    pub min: f64,
    pub max: f64,
}

impl Panel {
    /// A panel with every field filled as the web's `panel()` fills it.
    pub fn new(title: &str, kind: PanelKind, metric: MetricId) -> Self {
        Self {
            id: String::new(),
            title: title.into(),
            kind,
            metric,
            aggregation: Aggregation::Avg,
            group_by: None,
            reduce: Reduce::Last,
            filter: Vec::new(),
            unit: None,
            decimals: None,
            thresholds: Vec::new(),
            color: "chart-2",
            span: if matches!(kind, PanelKind::Stat | PanelKind::Gauge) {
                PanelSpan::Sixth
            } else {
                PanelSpan::Half
            },
            fill: Fill::Gradient,
            stacked: false,
            color_mode: ColorMode::Value,
            min: 0.0,
            max: 100.0,
        }
    }

    fn agg(mut self, aggregation: Aggregation) -> Self {
        self.aggregation = aggregation;
        self
    }
    fn by(mut self, label: Label) -> Self {
        self.group_by = Some(label);
        self
    }
    fn only(mut self, label: Label, value: &str) -> Self {
        self.filter.push((label, value.into()));
        self
    }
    fn scale(mut self, thresholds: Vec<Threshold>) -> Self {
        self.thresholds = thresholds;
        self
    }
    fn span(mut self, span: PanelSpan) -> Self {
        self.span = span;
        self
    }
    fn color(mut self, color: &'static str) -> Self {
        self.color = color;
        self
    }
    fn reduce(mut self, reduce: Reduce) -> Self {
        self.reduce = reduce;
        self
    }
    fn stacked(mut self) -> Self {
        self.stacked = true;
        self
    }
    fn fill(mut self, fill: Fill) -> Self {
        self.fill = fill;
        self
    }
    fn background(mut self) -> Self {
        self.color_mode = ColorMode::Background;
        self
    }
    fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }
    fn ends(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// A panel as it starts in the editor: requests over time, summed by service.
    pub fn fresh() -> Self {
        Self::new("New panel", PanelKind::TimeSeries, MetricId::Requests)
            .agg(Aggregation::Sum)
            .by(Label::Service)
    }
}

/// A dashboard: its id, title, what it shows and its panels in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Dashboard {
    pub id: String,
    pub title: String,
    pub description: String,
    pub panels: Vec<Panel>,
}

/// Good below `warning`, then warning, then critical from `critical` up.
fn good_low(warning: f64, critical: f64) -> Vec<Threshold> {
    vec![
        Threshold::base("good"),
        Threshold::from(warning, "warning"),
        Threshold::from(critical, "critical"),
    ]
}

/// The demo's dashboards, each panel under an id of its place, as the web's `seedDashboards`.
pub fn seed_dashboards() -> Vec<Dashboard> {
    use Label::{Host, Job, Project, Service};
    use MetricId as M;
    use PanelKind as K;
    let board = |id: &str, title: &str, description: &str, panels: Vec<Panel>| Dashboard {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        panels: panels
            .into_iter()
            .enumerate()
            .map(|(i, mut p)| {
                p.id = format!("{id}-{}", i + 1);
                p
            })
            .collect(),
    };
    vec![
        board(
            "services",
            "Services overview",
            "Requests, errors and latency across the api, the workers and the scheduler.",
            vec![
                Panel::new("Requests", K::Stat, M::Requests)
                    .agg(Aggregation::Sum)
                    .decimals(0)
                    .color("chart-2"),
                Panel::new("Error rate", K::Stat, M::Errors)
                    .scale(good_low(1.0, 3.0))
                    .background(),
                Panel::new("api p95", K::Stat, M::LatencyP95)
                    .only(Service, "api")
                    .scale(good_low(200.0, 300.0)),
                Panel::new("Queue depth", K::Stat, M::QueueDepth)
                    .agg(Aggregation::Sum)
                    .scale(good_low(90.0, 130.0)),
                Panel::new("api CPU", K::Gauge, M::Cpu)
                    .only(Service, "api")
                    .scale(good_low(70.0, 85.0)),
                Panel::new("Worker CPU", K::Gauge, M::Cpu)
                    .only(Service, "worker")
                    .scale(good_low(70.0, 85.0)),
                Panel::new("Requests by service", K::TimeSeries, M::Requests)
                    .agg(Aggregation::Sum)
                    .by(Service)
                    .stacked()
                    .span(PanelSpan::Half),
                Panel::new("Latency p95 by service", K::TimeSeries, M::LatencyP95)
                    .by(Service)
                    .fill(Fill::None)
                    .scale(vec![Threshold::base("good"), Threshold::from(250.0, "critical")])
                    .span(PanelSpan::Half),
                Panel::new("Error rate by service", K::TimeSeries, M::Errors)
                    .by(Service)
                    .scale(vec![Threshold::base("good"), Threshold::from(3.0, "critical")])
                    .span(PanelSpan::Third),
                Panel::new("CPU by host", K::BarGauge, M::Cpu)
                    .by(Host)
                    .scale(good_low(70.0, 85.0))
                    .span(PanelSpan::Third),
                Panel::new("Requests by project", K::Bar, M::Requests)
                    .agg(Aggregation::Sum)
                    .by(Project)
                    .stacked()
                    .span(PanelSpan::Third),
                Panel::new("Latency p95 by host", K::Heatmap, M::LatencyP95)
                    .by(Host)
                    .scale(vec![
                        Threshold::base("chart-2"),
                        Threshold::from(150.0, "warning"),
                        Threshold::from(250.0, "critical"),
                    ])
                    .span(PanelSpan::Half),
                Panel::new("Hosts by p95", K::Table, M::LatencyP95)
                    .by(Host)
                    .scale(good_low(200.0, 300.0))
                    .span(PanelSpan::Half),
                Panel::new("Logs", K::Logs, M::Requests).span(PanelSpan::Full),
            ],
        ),
        board(
            "workers",
            "Workers",
            "The workers\u{2019} queue, load and memory, and what they log.",
            vec![
                Panel::new("Queue depth", K::Stat, M::QueueDepth)
                    .agg(Aggregation::Sum)
                    .scale(good_low(90.0, 130.0))
                    .background(),
                Panel::new("Mean run", K::Stat, M::JobDuration)
                    .reduce(Reduce::Mean)
                    .color("chart-3"),
                Panel::new("CPU", K::Gauge, M::Cpu)
                    .only(Service, "worker")
                    .scale(good_low(70.0, 85.0)),
                Panel::new("Memory", K::Gauge, M::Memory)
                    .decimals(1)
                    .only(Service, "worker")
                    .ends(0.0, 4.0)
                    .scale(good_low(3.0, 3.5)),
                Panel::new("Memory by host", K::BarGauge, M::Memory)
                    .by(Host)
                    .only(Service, "worker")
                    .ends(0.0, 4.0)
                    .scale(good_low(3.0, 3.5))
                    .span(PanelSpan::Third),
                Panel::new("Queue depth by host", K::TimeSeries, M::QueueDepth)
                    .agg(Aggregation::Sum)
                    .by(Host)
                    .stacked()
                    .fill(Fill::Solid)
                    .span(PanelSpan::Half),
                Panel::new("CPU by host", K::TimeSeries, M::Cpu)
                    .by(Host)
                    .only(Service, "worker")
                    .fill(Fill::None)
                    .scale(vec![Threshold::base("good"), Threshold::from(85.0, "critical")])
                    .span(PanelSpan::Half),
                Panel::new("CPU heat", K::Heatmap, M::Cpu)
                    .by(Host)
                    .only(Service, "worker")
                    .scale(vec![
                        Threshold::base("chart-2"),
                        Threshold::from(70.0, "warning"),
                        Threshold::from(85.0, "critical"),
                    ])
                    .span(PanelSpan::Half),
                Panel::new("Worker logs", K::Logs, M::Requests)
                    .only(Service, "worker")
                    .span(PanelSpan::Half),
            ],
        ),
        board(
            "jobs",
            "Jobs",
            "How long each job\u{2019}s runs take, and how many runs wait for a slot.",
            vec![
                Panel::new("Slowest run", K::Stat, M::JobDuration)
                    .agg(Aggregation::Max)
                    .reduce(Reduce::Max)
                    .scale(good_low(120.0, 160.0)),
                Panel::new("Mean run", K::Stat, M::JobDuration)
                    .reduce(Reduce::Mean)
                    .color("chart-3"),
                Panel::new("Waiting", K::Stat, M::QueueDepth)
                    .agg(Aggregation::Sum)
                    .color("chart-4"),
                Panel::new("Worker errors", K::Stat, M::Errors)
                    .only(Service, "worker")
                    .scale(good_low(0.5, 1.0))
                    .background(),
                Panel::new("Duration by job", K::TimeSeries, M::JobDuration)
                    .by(Job)
                    .fill(Fill::None)
                    .span(PanelSpan::TwoThirds),
                Panel::new("Mean by job", K::BarGauge, M::JobDuration)
                    .by(Job)
                    .reduce(Reduce::Mean)
                    .ends(0.0, 150.0)
                    .scale(good_low(60.0, 120.0))
                    .span(PanelSpan::Third),
                Panel::new("Queue by project", K::Bar, M::QueueDepth)
                    .agg(Aggregation::Sum)
                    .by(Project)
                    .stacked()
                    .span(PanelSpan::Half),
                Panel::new("Jobs", K::Table, M::JobDuration)
                    .by(Job)
                    .scale(good_low(60.0, 120.0))
                    .span(PanelSpan::Half),
            ],
        ),
    ]
}

impl sokkel_egui::ArrangeItem for Panel {
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

/// The panels a dashboard being edited can add from its palette, by title, as the web's `CATALOG`.
const CATALOG: [&str; 14] = [
    "Requests",
    "Error rate",
    "Queue depth",
    "api CPU",
    "Memory",
    "Mean run",
    "Requests by service",
    "Latency p95 by service",
    "Duration by job",
    "CPU by host",
    "Requests by project",
    "Latency p95 by host",
    "Hosts by p95",
    "Logs",
];

/// The seeded panels the palette offers, as the web's `widgetCatalog`.
pub fn widget_catalog() -> Vec<Panel> {
    let seeded: Vec<Panel> = seed_dashboards().into_iter().flat_map(|b| b.panels).collect();
    CATALOG
        .iter()
        .filter_map(|title| seeded.iter().find(|p| p.title == *title).cloned())
        .collect()
}

/// A kind's icon, as the web's `KIND_ICONS`.
pub fn kind_icon(kind: PanelKind) -> sokkel_egui::Icon {
    use sokkel_egui::Icon;
    match kind {
        PanelKind::TimeSeries => Icon::ChartArea,
        PanelKind::Stat => Icon::SquareSigma,
        PanelKind::Gauge => Icon::Gauge,
        PanelKind::BarGauge => Icon::TextAlignStart,
        PanelKind::Bar => Icon::ChartColumn,
        PanelKind::Heatmap => Icon::Grid3x3,
        PanelKind::Table => Icon::Table2,
        PanelKind::Logs => Icon::ScrollText,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_dashboards_are_the_web_s() {
        let boards = seed_dashboards();
        let counts: Vec<(&str, usize)> = boards.iter().map(|b| (b.id.as_str(), b.panels.len())).collect();
        assert_eq!(counts, [("services", 14), ("workers", 9), ("jobs", 8)]);
        assert_eq!(boards[0].panels[1].id, "services-2");
        assert_eq!(boards[0].panels[0].span, PanelSpan::Sixth, "a stat a sixth wide");
    }
}
