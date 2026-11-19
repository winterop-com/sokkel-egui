//! A saved or drafted chart drawn as its view, as the web's `ChartView.tsx` draws it: a chart kind
//! in its panel (or its frame on a page), the regions shaded on the map with its legend and the
//! picked region's figure, the figures as a table, or one figure; and a chart drawn small for a card.

use std::sync::OnceLock;

use eframe::egui::{Ui, vec2};
use sokkel_egui::{
    Cell as TableCell, Column, DashboardPanel, DataTable, MenuItem, PanelAsked, PanelMenu, StackLayout, StatPanel,
    TableState, TableVariant, tokens,
};
use sokkel_egui_charts::{Chart, ChartKind, ChartPanel, ChartSpec, PanelWords, Stack};
use sokkel_egui_maps::{Area, Emphasis, MapLegend, MapPopup, MapPopupRow, MapView, SequentialScale, areas_bounds};

use crate::chart_view::{chart_spec, empty_words, is_empty, lead_color, stat_note};
use crate::model::{Dataset, Decimals, NumberStyle, Preferences, ViewKind, ViewSpec, palette_of};
use crate::shape::{Shaped, breaks_of, format_figure, formatter, measure_name, shape};

/// The chart kind each charted view is drawn by.
pub fn kind_of(view: ViewKind) -> Option<ChartKind> {
    crate::chart_view::chart_kind(view).and_then(ChartKind::from_id)
}

/// The spec the charts add-on draws a view's figures from: the web's `chartSpec`.
pub fn kind_spec(shaped: &Shaped, spec: &ViewSpec, prefs: &Preferences, measure: &str) -> ChartSpec {
    let view = chart_spec(shaped, spec, prefs, measure);
    let mut out = ChartSpec::new(view.rows)
        .aggregate(view.aggregate.combine())
        .horizontal(view.horizontal)
        .show_scale(view.show_scale)
        .fit(view.fit)
        .format(formatter(prefs));
    if let Some(x) = view.x {
        out = out.x(x);
    }
    if let Some(y) = view.y {
        out = out.y(y);
    }
    if let Some(series) = view.series {
        out = out.series(series);
    }
    if let Some(value) = view.value {
        out = out.value(value);
    }
    if let Some(order) = view.order {
        out = out.order(order);
    }
    if view.stack.is_some() {
        out = out.stack(Stack::Stacked);
    }
    if let Some(palette) = view.palette {
        out = out.palette(palette.iter().copied());
    }
    if let Some((field, name)) = view.field_names {
        out = out.field_name(field, name);
    }
    out
}

/// The regions as the map draws them, read once.
pub fn regions() -> &'static [Area] {
    static REGIONS: OnceLock<Vec<Area>> = OnceLock::new();
    REGIONS.get_or_init(|| {
        sokkel_egui_maps::areas_from_geojson_text(include_str!("../../records/data/regions.json"), "id", "name")
    })
}

/// How a view is drawn: in a board's grid, or filling a page's pane at `height`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Panel,
    Page(f32),
}

/// What a chart is drawn from and called.
pub struct Drawn<'a> {
    pub dataset: &'a Dataset,
    pub spec: &'a ViewSpec,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub prefs: &'a Preferences,
    pub filters: &'a [sokkel_egui::Filter],
    pub mode: Mode,
    /// Drawn larger, as a board presented full screen.
    pub large: bool,
}

/// A view drawn as the web's `ChartView`, its panel's menu `items` in its head: what its tools
/// asked. `map` is the view's map, kept between frames; `table` its table's state.
pub fn chart_view(
    ui: &mut Ui,
    drawn: &Drawn,
    items: Vec<MenuItem>,
    map: &mut MapView,
    table: &mut TableState,
) -> Option<PanelAsked> {
    let shaped = shape(drawn.dataset, drawn.spec, drawn.filters);
    let measure = measure_name(drawn.dataset, drawn.spec);
    let palette = palette_of(&drawn.spec.palette, drawn.prefs);
    let empty = is_empty(&shaped);
    let words = empty_words(&shaped);
    let height = match drawn.mode {
        Mode::Page(height) => height,
        Mode::Panel if drawn.large => 384.0,
        Mode::Panel => 256.0,
    };
    let panel = drawn.mode == Mode::Panel;
    if let Some(kind) = kind_of(drawn.spec.view) {
        let spec = kind_spec(&shaped, drawn.spec, drawn.prefs, &measure);
        let draw = |ui: &mut Ui| {
            let mut chart = ChartPanel::new(drawn.title, kind, &spec)
                .empty(empty)
                .words(PanelWords {
                    empty: words.clone(),
                    ..Default::default()
                })
                .legend(kind != ChartKind::Heatmap)
                .height(height);
            if let Some(subtitle) = drawn.subtitle {
                chart = chart.subtitle(subtitle);
            }
            chart.show(ui);
        };
        return if panel {
            PanelMenu::new(items).expandable(true).show(ui, draw)
        } else {
            draw(ui);
            None
        };
    }
    if drawn.spec.view == ViewKind::Stat {
        let format = formatter(drawn.prefs);
        let value = if shaped.problem.is_none() {
            shaped.stat.value.as_number()
        } else {
            None
        };
        let line = shaped.stat.sparkline.clone().unwrap_or_default();
        let note = stat_note(&measure, drawn.subtitle);
        let draw = |ui: &mut Ui| {
            let mut stat = StatPanel::new(drawn.title, value)
                .format(&format)
                .color(lead_color(palette))
                .note(note);
            if !line.is_empty() {
                stat = stat.sparkline(&line);
            }
            if let Some(delta) = &shaped.stat.delta {
                stat = stat.delta(delta.clone());
            }
            stat.show(ui);
        };
        return PanelMenu::new(items).show(ui, draw);
    }
    let draw = |ui: &mut Ui| {
        let mut panel = DashboardPanel::new(drawn.title).flush(true);
        if let Some(subtitle) = drawn.subtitle {
            panel = panel.subtitle(subtitle);
        }
        panel.show(ui, |ui| {
            if empty {
                let t = tokens(ui.ctx());
                ui.allocate_ui(vec2(ui.available_width(), height), |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            sokkel_egui::TextRole::Body
                                .rich(&t, &words)
                                .color(t.color("muted-foreground")),
                        );
                    });
                });
            } else if drawn.spec.view == ViewKind::Map {
                region_map(ui, map, &shaped, drawn.prefs, palette, &measure, height);
            } else {
                figure_table(ui, &shaped, drawn.prefs, table, height);
            }
        });
    };
    PanelMenu::new(items).expandable(true).show(ui, draw)
}

/// A view's figures as a table, each figure in the settings' format.
fn figure_table(ui: &mut Ui, shaped: &Shaped, prefs: &Preferences, state: &mut TableState, height: f32) {
    let format = formatter(prefs);
    let columns: Vec<Column> = shaped
        .table
        .names
        .iter()
        .enumerate()
        .map(|(at, name)| {
            if at == 0 {
                Column::new(name.clone(), 128.0).fill()
            } else {
                Column::new(name.clone(), 112.0).numeric()
            }
        })
        .collect();
    let rows: Vec<Vec<TableCell>> = shaped
        .table
        .rows
        .iter()
        .map(|cells| {
            cells
                .iter()
                .map(|cell| match cell {
                    sokkel_egui_charts::data::Cell::Number(value) => TableCell::Number {
                        value: *value,
                        text: format(*value),
                    },
                    sokkel_egui_charts::data::Cell::Text(text) => TableCell::Text(text.clone()),
                    sokkel_egui_charts::data::Cell::Null => TableCell::Muted("-".to_owned()),
                })
                .collect()
        })
        .collect();
    ui.allocate_ui(vec2(ui.available_width(), height), |ui| {
        DataTable::new(ui.scope_id().with("figures"), &columns, &rows, state)
            .framed(false)
            .footer(false)
            .stack_layout(StackLayout::Labeled)
            .stack_below(360.0)
            .variant(TableVariant::Table)
            .height(height)
            .show(ui);
    });
}

/// The view's figures shaded on the regions they name, with a legend and the picked region's figure.
pub fn region_map(
    ui: &mut Ui,
    map: &mut MapView,
    shaped: &Shaped,
    prefs: &Preferences,
    palette: Option<&'static [&'static str]>,
    measure: &str,
    height: f32,
) {
    let t = tokens(ui.ctx());
    let by_name = |name: &str| -> Option<Option<f64>> {
        shaped
            .regions
            .iter()
            .find(|(one, _)| one.to_lowercase() == name.to_lowercase())
            .map(|(_, value)| *value)
    };
    let values: Vec<f64> = shaped.regions.iter().filter_map(|(_, value)| *value).collect();
    // the breaks are round already; the legend writes them short
    let scale = SequentialScale::new(&t, breaks_of(&values), lead_color(palette))
        .format(|value| format_figure(NumberStyle::Compact, Decimals::Auto, value));
    let all = regions();
    if map.areas.is_empty() {
        map.bare = true;
        map.frame(areas_bounds(all));
    }
    let selected = map.selected.clone();
    map.areas = all
        .iter()
        .map(|area| {
            let emphasis = if selected.as_deref() == Some(area.id.as_str()) {
                Emphasis::Selected
            } else {
                Emphasis::Plain
            };
            Area::clone(area)
                .class_color(scale.color_of(by_name(&area.label).flatten()))
                .emphasis(emphasis)
        })
        .collect();
    let missing = all
        .iter()
        .filter(|area| by_name(&area.label).flatten().is_none())
        .count();
    let width = ui.available_width();
    let response = map.show(ui, vec2(width, height), &[], &[]);
    let mut legend = MapLegend::new(measure).scale(&scale).captions("Less", "More");
    if missing > 0 {
        legend = legend.no_data("No figure");
    }
    legend.show(ui, &response);
    if let Some(area) = map
        .selected
        .as_deref()
        .and_then(|chosen| all.iter().find(|area| area.id == chosen))
    {
        let format = formatter(prefs);
        let figure = by_name(&area.label)
            .flatten()
            .map_or_else(|| "No figure".to_owned(), format);
        MapPopup::new(&area.label)
            .description("Region")
            .row(MapPopupRow::new(measure, figure))
            .show(ui, map);
    }
}

/// A small chart of a view, for a card: its kind's chart alone, `height` tall, or nothing for a
/// view no kind draws.
pub fn thumb(ui: &mut Ui, dataset: &Dataset, spec: &ViewSpec, prefs: &Preferences, height: f32) {
    let Some(kind) = kind_of(spec.view) else { return };
    let shaped = shape(dataset, spec, &[]);
    if is_empty(&shaped) {
        return;
    }
    let measure = measure_name(dataset, spec);
    let chart = kind_spec(&shaped, spec, prefs, &measure);
    ui.add(Chart::new(kind, &chart).height(height));
}
