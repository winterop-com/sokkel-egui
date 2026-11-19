//! Charts drawn from rows and a spec, each kind in a chart panel, and a choropleth of the regions:
//! what a visualizer draws from a table's columns.

use std::cell::RefCell;

use eframe::egui::{Ui, vec2};
use sokkel_egui::{MenuItem, PanelMenu, tokens};
use sokkel_egui_charts::data::{Aggregate, Cell, Row};
use sokkel_egui_charts::{ChartKind, ChartPanel, ChartSpec, PanelWords, Stack};
use sokkel_egui_maps::{
    Area, Emphasis, MapLegend, MapPopup, MapPopupRow, MapView, SequentialScale, areas_bounds, areas_from_geojson_text,
};

use super::{State, section};

const JOBS: [&str; 5] = [
    "nightly-export",
    "sync-inventory",
    "rebuild-index",
    "send-digest",
    "import-orders",
];
const STATES: [&str; 3] = ["succeeded", "failed", "cancelled"];
const PROJECTS: [&str; 3] = ["harbor", "juniper", "larkspur"];
const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// A figure that wanders but always reads the same, from a few whole numbers.
fn wander(a: usize, b: usize, spread: usize) -> f64 {
    ((a * 37 + b * 101 + a * b * 13) % spread) as f64
}

/// Tidevann's runs in October: a row per job, day and state, with how many and how long.
fn runs() -> Vec<Row> {
    let mut rows = Vec::new();
    for (j, job) in JOBS.iter().enumerate() {
        for day in 1..=14 {
            for (s, state) in STATES.iter().enumerate() {
                // fewer at the weekend, a swell through the fortnight, each job its own size
                let base = [40.0, 4.0, 1.0][s];
                let weekend = if (day + 2) % 7 >= 5 { 0.55 } else { 1.0 };
                let swell = 1.0 + 0.35 * (day as f64 * 0.8 + j as f64).sin();
                let runs = (base * (5 - j) as f64 / 3.0 * weekend * swell + wander(j, day + s, 3)).round();
                let duration = 120.0 + 40.0 * j as f64 + 45.0 * (day as f64 * 0.7 + j as f64 * 1.3).sin();
                rows.push(Row::of([
                    ("job", Cell::from(*job)),
                    ("project", Cell::from(PROJECTS[j % 3])),
                    ("day", Cell::from(format!("Oct {day}"))),
                    ("weekday", Cell::from(DAYS[(day + 2) % 7])),
                    ("state", Cell::from(*state)),
                    ("runs", Cell::Number(runs)),
                    ("duration", Cell::Number((duration + wander(j, day, 20)).round())),
                ]));
            }
        }
    }
    rows
}

/// A panel of a kind, with a menu and expand in its head as a board's panels have them.
fn panel(ui: &mut Ui, title: &str, subtitle: &str, kind: ChartKind, spec: &ChartSpec) {
    PanelMenu::new(vec![MenuItem::action("Edit"), MenuItem::action("Remove")])
        .expandable(true)
        .show(ui, |ui| ChartPanel::new(title, kind, spec).subtitle(subtitle).show(ui));
}

/// Two panels side by side.
fn pair(ui: &mut Ui, left: impl FnOnce(&mut Ui), right: impl FnOnce(&mut Ui)) {
    ui.columns(2, |columns| {
        columns[0].push_id("left", left);
        columns[1].push_id("right", right);
    });
}

fn figures() -> impl Fn(f64) -> String + Send + Sync + 'static {
    |v| sokkel_egui::format_value(v, None)
}

pub fn bars(_: &mut State, ui: &mut Ui) {
    let rows = runs();
    section(ui, "Bars from rows: x, y, series and how they combine");
    let stacked = ChartSpec::new(rows.clone())
        .x("job")
        .y("runs")
        .series("state")
        .stack(Stack::Stacked)
        .format(figures());
    let lying = ChartSpec::new(rows.clone())
        .x("job")
        .y("duration")
        .aggregate(Aggregate::Avg)
        .horizontal(true)
        .unit(" s")
        .palette(["kind-teal"]);
    pair(
        ui,
        |ui| panel(ui, "Runs by job and state", "Stacked", ChartKind::Bar, &stacked),
        |ui| panel(ui, "Average duration", "Lying down", ChartKind::Bar, &lying),
    );
    ui.add_space(16.0);
    let side = ChartSpec::new(rows.clone())
        .x("weekday")
        .y("runs")
        .series("project")
        .order(DAYS)
        .format(figures());
    let shares = ChartSpec::new(rows)
        .x("job")
        .y("runs")
        .series("state")
        .stack(Stack::Percent)
        .color("failed", "status-failed")
        .color("cancelled", "muted-foreground/45");
    pair(
        ui,
        |ui| {
            panel(
                ui,
                "Runs by weekday",
                "Side by side, in their order",
                ChartKind::Bar,
                &side,
            )
        },
        |ui| {
            panel(
                ui,
                "Share of runs by state",
                "To 100%, a color per state",
                ChartKind::Bar,
                &shares,
            )
        },
    );
}

pub fn lines(_: &mut State, ui: &mut Ui) {
    let rows = runs();
    let days: Vec<String> = (1..=14).map(|d| format!("Oct {d}")).collect();
    section(ui, "Lines and areas from rows");
    let line = ChartSpec::new(rows.clone())
        .x("day")
        .y("duration")
        .series("project")
        .aggregate(Aggregate::Avg)
        .order(days.clone())
        .unit(" s");
    let area = ChartSpec::new(rows.clone())
        .x("day")
        .y("runs")
        .order(days.clone())
        .format(figures());
    pair(
        ui,
        |ui| panel(ui, "Duration through October", "By project", ChartKind::Line, &line),
        |ui| panel(ui, "Runs through October", "Every job", ChartKind::Area, &area),
    );
    ui.add_space(16.0);
    let stacked = ChartSpec::new(rows.clone())
        .x("day")
        .y("runs")
        .series("state")
        .order(days)
        .stack(Stack::Stacked)
        .format(figures());
    let counted = ChartSpec::new(rows).x("project");
    pair(
        ui,
        |ui| panel(ui, "Runs by state", "Stacked areas", ChartKind::Area, &stacked),
        |ui| {
            panel(
                ui,
                "Rows per project",
                "No y: the rows counted",
                ChartKind::Bar,
                &counted,
            )
        },
    );
}

pub fn slices(_: &mut State, ui: &mut Ui) {
    let rows = runs();
    section(ui, "A pie and a ring: each category's share of the whole");
    let pie = ChartSpec::new(rows.clone())
        .x("state")
        .y("runs")
        .palette(["kind-teal", "kind-green", "kind-pink"])
        .format(figures());
    let donut = ChartSpec::new(rows)
        .x("project")
        .y("runs")
        .total("runs")
        .format(figures());
    pair(
        ui,
        |ui| panel(ui, "Runs by state", "A palette of three", ChartKind::Pie, &pie),
        |ui| panel(ui, "Runs by project", "With the total", ChartKind::Donut, &donut),
    );
}

pub fn heatmap(_: &mut State, ui: &mut Ui) {
    let rows = runs();
    section(ui, "A heatmap: x along, series down, each cell its figure");
    let spec = ChartSpec::new(rows.clone())
        .x("weekday")
        .y("runs")
        .series("job")
        .order(DAYS)
        .show_scale(true)
        .field_name("runs", "Runs")
        .format(figures());
    panel(
        ui,
        "Runs by job and weekday",
        "The scale beside it",
        ChartKind::Heatmap,
        &spec,
    );
    ui.add_space(16.0);
    let days: Vec<String> = (1..=14).map(|d| format!("Oct {d}")).collect();
    let stepped = ChartSpec::new(rows)
        .x("day")
        .y("duration")
        .series("project")
        .aggregate(Aggregate::Max)
        .order(days)
        .scale(["kind-green", "warning", "critical"])
        .show_scale(true)
        .unit(" s");
    panel(
        ui,
        "Longest run by project",
        "A scale through three tokens",
        ChartKind::Heatmap,
        &stepped,
    );
}

pub fn scatter(_: &mut State, ui: &mut Ui) {
    // a run per row: the rows it moved against how long it took, by project, sized by its retries
    let rows: Vec<Row> = (0..60)
        .map(|i| {
            let moved = 40.0 + 9.0 * i as f64 + wander(i, 3, 70);
            Row::of([
                ("project", Cell::from(PROJECTS[i % 3])),
                ("rows", Cell::Number(moved)),
                (
                    "duration",
                    Cell::Number(30.0 + moved * 0.05 * (1.0 + (i % 3) as f64 * 0.4) + wander(i, 7, 12)),
                ),
                ("retries", Cell::Number([0.0, 0.0, 1.0, 0.0, 3.0, 0.0, 1.0, 6.0][i % 8])),
            ])
        })
        .collect();
    section(ui, "A scatter: a point per row, colored and sized by fields");
    let plain = ChartSpec::new(rows.clone())
        .x("rows")
        .y("duration")
        .fit(true)
        .field_name("duration", "Duration")
        .unit(" s");
    let grouped = ChartSpec::new(rows)
        .x("rows")
        .y("duration")
        .series("project")
        .value("retries")
        .field_name("rows", "Rows moved")
        .field_name("duration", "Duration")
        .field_name("retries", "Retries")
        .unit(" s");
    pair(
        ui,
        |ui| {
            panel(
                ui,
                "Duration by rows moved",
                "Fitted to the points",
                ChartKind::Scatter,
                &plain,
            )
        },
        |ui| {
            panel(
                ui,
                "By project",
                "Sized by retries, from zero",
                ChartKind::Scatter,
                &grouped,
            )
        },
    );
    ui.add_space(16.0);
    let none = ChartSpec::new(Vec::new()).x("rows").y("duration");
    let words = PanelWords {
        empty: "No rows match the filters.".into(),
        ..Default::default()
    };
    PanelMenu::new(vec![MenuItem::action("Edit")])
        .expandable(true)
        .show(ui, |ui| {
            ChartPanel::new("Duration by rows moved", ChartKind::Scatter, &none)
                .subtitle("Filtered to nothing")
                .empty(true)
                .words(words)
                .show(ui);
        });
}

thread_local! {
    /// The choropleth's map, framed on the regions once.
    static CHOROPLETH: RefCell<Option<MapView>> = const { RefCell::new(None) };
}

/// The rows each region moved this month, the last two with no figure.
fn moved(index: usize, count: usize) -> Option<f64> {
    (index + 2 < count).then(|| 2200.0 + wander(index, 5, 9) * 95.0 + (index % 4) as f64 * 60.0)
}

pub fn choropleth(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    section(ui, "A choropleth: the regions shaded by their figure, with its legend");
    let regions = areas_from_geojson_text(include_str!("../../../records/data/regions.json"), "id", "name");
    let figures: Vec<Option<f64>> = (0..regions.len()).map(|i| moved(i, regions.len())).collect();
    let scale =
        SequentialScale::new(&t, vec![2300.0, 2500.0, 2600.0, 2900.0], "chart-2").format(sokkel_egui::format_compact);
    CHOROPLETH.with_borrow_mut(|held| {
        let map = held.get_or_insert_with(|| {
            let mut map = MapView::new(6.4, -9.4, 6.0).framed(true);
            map.bare = true;
            map.frame(areas_bounds(&regions));
            map.selected = Some("grand-gedeh".into());
            map
        });
        let selected = map.selected.clone();
        map.areas = regions
            .iter()
            .zip(&figures)
            .map(|(area, figure)| {
                let emphasis = if selected.as_deref() == Some(area.id.as_str()) {
                    Emphasis::Selected
                } else {
                    Emphasis::Plain
                };
                Area::clone(area)
                    .class_color(scale.color_of(*figure))
                    .emphasis(emphasis)
            })
            .collect();
        let width = ui.available_width();
        let response = map.show(ui, vec2(width, 480.0), &[], &[]);
        MapLegend::new("Rows moved")
            .scale(&scale)
            .captions("Less", "More")
            .no_data("No figure")
            .show(ui, &response);
        if let Some(index) = map
            .selected
            .as_deref()
            .and_then(|chosen| regions.iter().position(|a| a.id == chosen))
        {
            let figure = figures[index].map_or_else(|| "No figure".to_owned(), |v| sokkel_egui::format_value(v, None));
            MapPopup::new(&regions[index].label)
                .description("Region")
                .row(MapPopupRow::new("Rows moved", figure))
                .show(ui, map);
        }
    });
}
