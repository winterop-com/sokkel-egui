//! What a saved or drafted chart is drawn as, as the web's `ChartView.tsx` decides it: a chart
//! kind with the spec the charts add-on draws from, the map, the figures as a table, or one figure;
//! with the words said when there is nothing to draw.

use serde::Serialize;

use crate::chart_data::{Cell, Row};
use crate::model::{Aggregate, Preferences, ViewKind, ViewSpec, palette_of};
use crate::shape::Shaped;

/// The chart kind each charted view is drawn by.
pub fn chart_kind(view: ViewKind) -> Option<&'static str> {
    match view {
        ViewKind::Bar => Some("bar"),
        ViewKind::Line => Some("line"),
        ViewKind::Area => Some("area"),
        ViewKind::Scatter => Some("scatter"),
        ViewKind::Pie => Some("pie"),
        ViewKind::Heatmap => Some("heatmap"),
        _ => None,
    }
}

/// Whether a view is drawn by a chart kind, and so exports as a picture.
pub fn is_charted(view: ViewKind) -> bool {
    chart_kind(view).is_some()
}

/// The spec a chart kind draws from: the fields `shape` named, the order, the palette and how it
/// lays the series out. Figures are written by the settings' formatter.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartSpec {
    #[serde(skip)]
    pub rows: Vec<Row>,
    pub x: Option<String>,
    pub y: Option<String>,
    pub series: Option<String>,
    pub value: Option<String>,
    pub order: Option<Vec<String>>,
    pub aggregate: Aggregate,
    /// `stacked` where the series stack.
    pub stack: Option<&'static str>,
    pub horizontal: bool,
    pub palette: Option<&'static [&'static str]>,
    /// The measure's field and the name it is shown by.
    pub field_names: Option<(String, String)>,
    pub show_scale: bool,
    pub fit: bool,
}

/// The chart spec for a view's figures.
pub fn chart_spec(shaped: &Shaped, spec: &ViewSpec, prefs: &Preferences, measure: &str) -> ChartSpec {
    ChartSpec {
        rows: shaped.rows.clone(),
        x: shaped.x.clone(),
        y: shaped.y.clone(),
        series: shaped.series.clone(),
        value: shaped.value.clone(),
        order: shaped.order.clone(),
        aggregate: shaped.aggregate,
        stack: (spec.stack && shaped.series.is_some() && matches!(spec.view, ViewKind::Bar | ViewKind::Area))
            .then_some("stacked"),
        horizontal: spec.view == ViewKind::Bar && spec.horizontal,
        palette: palette_of(&spec.palette, prefs),
        field_names: shaped.y.clone().map(|y| (y, measure.to_owned())),
        show_scale: spec.view == ViewKind::Heatmap,
        fit: spec.view == ViewKind::Scatter,
    }
}

/// Whether there is nothing to draw: the view cannot be drawn, or no figures are left.
pub fn is_empty(shaped: &Shaped) -> bool {
    shaped.problem.is_some() || shaped.empty
}

/// What an empty view says: why it cannot be drawn, that no rows match, or that nothing is left.
pub fn empty_words(shaped: &Shaped) -> String {
    match &shaped.problem {
        Some(problem) => problem.clone(),
        None if shaped.read == 0 => "No rows match the filters.".to_owned(),
        None => "Nothing to draw from these rows.".to_owned(),
    }
}

/// A charted view's label for a screen reader: `Title: Sum of Rainfall`.
pub fn chart_label(title: &str, measure: &str) -> String {
    format!("{title}: {measure}")
}

/// The figure a stat tile shows: none where the view cannot be drawn.
pub fn stat_value(shaped: &Shaped) -> Cell {
    if shaped.problem.is_some() {
        Cell::Null
    } else {
        shaped.stat.value.clone()
    }
}

/// The note under a stat tile's figure: the measure, and the subtitle after it.
pub fn stat_note(measure: &str, subtitle: Option<&str>) -> String {
    match subtitle {
        Some(subtitle) => format!("{measure} · {subtitle}"),
        None => measure.to_owned(),
    }
}

/// The color token a stat tile and a map are drawn in: the palette's first, or the theme's.
pub fn lead_color(palette: Option<&'static [&'static str]>) -> &'static str {
    palette.and_then(|tokens| tokens.first().copied()).unwrap_or("chart-2")
}

/// A map's label for a screen reader.
pub fn map_label(measure: &str) -> String {
    format!("{measure} by region")
}
