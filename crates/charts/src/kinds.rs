//! A chart drawn from rows and a spec, as sokkel-ui's `Chart` with its `kind`: the rows are shaped
//! by [`crate::data`] and drawn by the same engine as the charts of categories and series (bars and
//! lines by [`BarChart`] and [`LineChart`]), the kinds that draw something else (slices, a heatmap's
//! grid, a cloud of points) by the painter here. [`chart_legend`] lists what a kind's legend lists,
//! and [`ChartPanel`] draws one in a chart's frame with its legend under it, as the web's
//! `ChartPanel` and `ChartFrame`.

use std::sync::Arc;

use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Mesh, Pos2, Rect, Response, Sense, Shape, Stroke, TextWrapMode,
    Ui, Widget, WidgetText, pos2, vec2,
};
use sokkel_egui::{TextRole, Tokens, tokens};

use crate::data::{
    Aggregate, Pivot, PivotSpec, Row, as_shares, distinct, extent, format_number, format_tick, label_of, number_of,
    pivot,
};
use crate::panel::{Head, PanelWords, chart_frame, drawn_under, fill_height};
use crate::{BarChart, Fill, Format, LineChart, SERIES_ORDER, Series, legend_of};

/// The kinds a chart of rows is drawn as, as the web's `ChartKind` names the ones the visualizer
/// and the dashboards draw from rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChartKind {
    /// Bars per category, the series side by side, stacked, or lying down with `horizontal`.
    Bar,
    /// A line per series across the categories.
    Line,
    /// A line per series with the area under it filled, stacked with `stack`.
    Area,
    /// A full circle of slices, one per category.
    Pie,
    /// A ring of slices, one per category, with the total in its middle where `total` names it.
    Donut,
    /// A grid of cells, `x` along and `series` down, each shaded by its figure.
    Heatmap,
    /// A point per row, `x` against `y`, colored by `series` and sized by `value`.
    Scatter,
}

impl ChartKind {
    /// The kind by the web's name for it: `bar`, `line`, `area`, `pie`, `donut`, `heatmap`, `scatter`.
    pub fn from_id(id: &str) -> Option<ChartKind> {
        Some(match id {
            "bar" => ChartKind::Bar,
            "line" => ChartKind::Line,
            "area" => ChartKind::Area,
            "pie" => ChartKind::Pie,
            "donut" => ChartKind::Donut,
            "heatmap" => ChartKind::Heatmap,
            "scatter" => ChartKind::Scatter,
            _ => return None,
        })
    }

    pub fn id(self) -> &'static str {
        match self {
            ChartKind::Bar => "bar",
            ChartKind::Line => "line",
            ChartKind::Area => "area",
            ChartKind::Pie => "pie",
            ChartKind::Donut => "donut",
            ChartKind::Heatmap => "heatmap",
            ChartKind::Scatter => "scatter",
        }
    }
}

/// How bars and areas stack: on each other, or as shares of each category's total.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stack {
    Stacked,
    Percent,
}

/// The words a chart writes itself, beside the names its rows give, as the web's `ChartWords` as
/// far as these kinds say them. English by default.
#[derive(Clone, Debug)]
pub struct ChartWords {
    /// The one series where rows are counted.
    pub count: String,
    /// The one group a scatter draws without `series`.
    pub all: String,
}

impl Default for ChartWords {
    fn default() -> Self {
        Self {
            count: "Count".into(),
            all: "All".into(),
        }
    }
}

/// What a chart draws, read from rows, as the web's `ChartSpec` as far as these kinds read it.
/// Each kind says which fields it reads: most put `x`'s labels along an axis, combine `y` by
/// `aggregate` and split the rows into series by `series`.
#[derive(Clone, Default)]
pub struct ChartSpec {
    pub rows: Vec<Row>,
    /// The field along the category axis, or the first number of a scatter.
    pub x: Option<String>,
    /// The fields whose numbers are drawn, one series each; without one the rows are counted.
    pub y: Vec<String>,
    /// The field that splits the rows into series.
    pub series: Option<String>,
    /// A third field: a scatter's point size.
    pub value: Option<String>,
    /// How the values that fall in one place are combined, one way for all or one per `y` field;
    /// the sum when none is given.
    pub aggregate: Vec<Aggregate>,
    pub stack: Option<Stack>,
    /// Bars running sideways, the categories down the side.
    pub horizontal: bool,
    /// Categories in this order first.
    pub order: Option<Vec<String>>,
    /// Series in this order first.
    pub series_order: Option<Vec<String>>,
    /// A color token for a series or a category by its name; `name/45` draws it that opaque.
    pub colors: Vec<(String, String)>,
    /// The color tokens series and categories take in turn where `colors` names none; the theme's
    /// order ([`SERIES_ORDER`]) by default. The first is also a heatmap's scale.
    pub palette: Option<Vec<String>>,
    /// How a figure is written in tooltips, and short on the value axis' ticks.
    pub format: Option<Format>,
    /// A unit written after each figure, on the value axis too.
    pub unit: String,
    /// What a field is called where the chart names it: a scatter's axes and tooltip, a heatmap's
    /// tooltip.
    pub field_names: Vec<(String, String)>,
    /// A heatmap's color scale shown beside the grid.
    pub show_scale: bool,
    /// A scatter's axes fitted to the points rather than starting at zero.
    pub fit: bool,
    /// A ring's total written in its middle, with this word under it.
    pub total: Option<String>,
    /// The color tokens a heatmap's scale runs through: one shades from the card to it, several
    /// are stepped through. The palette's first by default.
    pub scale: Vec<String>,
    /// Where a heatmap's scale starts and ends; the smallest and largest figure by default.
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub words: ChartWords,
}

impl ChartSpec {
    pub fn new(rows: Vec<Row>) -> Self {
        Self {
            rows,
            ..Default::default()
        }
    }

    pub fn x(mut self, field: impl Into<String>) -> Self {
        self.x = Some(field.into());
        self
    }

    /// A field whose numbers are drawn; called again, a series for each.
    pub fn y(mut self, field: impl Into<String>) -> Self {
        self.y.push(field.into());
        self
    }

    pub fn series(mut self, field: impl Into<String>) -> Self {
        self.series = Some(field.into());
        self
    }

    pub fn value(mut self, field: impl Into<String>) -> Self {
        self.value = Some(field.into());
        self
    }

    pub fn aggregate(mut self, how: Aggregate) -> Self {
        self.aggregate = vec![how];
        self
    }

    pub fn stack(mut self, stack: Stack) -> Self {
        self.stack = Some(stack);
        self
    }

    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    pub fn order(mut self, order: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.order = Some(order.into_iter().map(Into::into).collect());
        self
    }

    pub fn series_order(mut self, order: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.series_order = Some(order.into_iter().map(Into::into).collect());
        self
    }

    /// `name` drawn in the color `token`.
    pub fn color(mut self, name: impl Into<String>, token: impl Into<String>) -> Self {
        self.colors.push((name.into(), token.into()));
        self
    }

    pub fn palette(mut self, tokens: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.palette = Some(tokens.into_iter().map(Into::into).collect());
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Some(Arc::new(format));
        self
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// `field` called `name` where the chart names it.
    pub fn field_name(mut self, field: impl Into<String>, name: impl Into<String>) -> Self {
        self.field_names.push((field.into(), name.into()));
        self
    }

    pub fn show_scale(mut self, show: bool) -> Self {
        self.show_scale = show;
        self
    }

    pub fn fit(mut self, fit: bool) -> Self {
        self.fit = fit;
        self
    }

    pub fn total(mut self, word: impl Into<String>) -> Self {
        self.total = Some(word.into());
        self
    }

    pub fn scale(mut self, tokens: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.scale = tokens.into_iter().map(Into::into).collect();
        self
    }

    pub fn words(mut self, words: ChartWords) -> Self {
        self.words = words;
        self
    }

    /// The rows as categories and series, as shares where the spec stacks them to 100%.
    pub fn pivot(&self) -> Option<Pivot> {
        let x = self.x.as_deref()?;
        let data = pivot(&PivotSpec {
            rows: &self.rows,
            x,
            y: &self.y,
            series: self.series.as_deref(),
            aggregate: &self.aggregate,
            order: self.order.as_deref(),
            series_order: self.series_order.as_deref(),
            count: Some(&self.words.count),
        });
        Some(if self.stack == Some(Stack::Percent) {
            Pivot {
                series: as_shares(&data.series),
                ..data
            }
        } else {
            data
        })
    }

    /// What a field is called: its name in `field_names`, or the field itself.
    pub fn named<'a>(&'a self, field: &'a str) -> &'a str {
        self.field_names
            .iter()
            .find(|(one, _)| one == field)
            .map_or(field, |(_, name)| name.as_str())
    }

    /// The `index`th token of the palette, where it gives one.
    fn palette_token(&self, index: usize) -> Option<&str> {
        let palette = self.palette.as_ref().filter(|p| !p.is_empty())?;
        Some(palette[index % palette.len()].as_str())
    }

    /// A series' or a category's color: the token `colors` names for it, or the next in the palette
    /// or the theme's order, as the web's `colorOf`.
    pub fn color_of(&self, t: &Tokens, name: &str, index: usize) -> Color32 {
        let named = self
            .colors
            .iter()
            .find(|(one, _)| one == name)
            .map(|(_, token)| token.as_str());
        match named.or_else(|| self.palette_token(index)) {
            Some(token) => token_color(t, token),
            None => t.color(SERIES_ORDER[index % SERIES_ORDER.len()]),
        }
    }

    /// A figure as a tooltip writes it: by `format`, or grouped with at most two decimals, then the
    /// unit; `-` for none.
    pub fn written(&self, value: Option<f64>) -> String {
        match value {
            None => "-".to_owned(),
            Some(value) => format!("{}{}", self.written_bare(value), self.unit),
        }
    }

    fn written_bare(&self, value: f64) -> String {
        match &self.format {
            Some(format) => format(value),
            None => format_number(value),
        }
    }

    /// A value axis' tick: by `format`, its figure short, then `unit`, as the web's `tickWriter`.
    fn tick(&self, value: f64, unit: &str) -> String {
        match &self.format {
            Some(format) => format!("{}{unit}", crate::compact_written(&format(value), value)),
            None => format!("{}{unit}", format_tick(value)),
        }
    }
}

/// A color token, with an alpha where it is written `name/45`.
pub fn token_color(t: &Tokens, token: &str) -> Color32 {
    match token.split_once('/') {
        Some((name, alpha)) => t.color_at(name, alpha.parse::<f32>().map_or(1.0, |a| a / 100.0)),
        None => t.color(token),
    }
}

/// `share` of `a` over `b`, mixed in oklab as `color-mix(in oklab, a share, b)`.
pub fn mix_oklab(a: Color32, b: Color32, share: f32) -> Color32 {
    fn lin(c: u8) -> f32 {
        let c = f32::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    fn gamma(c: f32) -> u8 {
        let c = c.clamp(0.0, 1.0);
        let c = if c <= 0.003_130_8 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (c * 255.0).round() as u8
    }
    let to = |c: Color32| {
        let (r, g, b) = (lin(c.r()), lin(c.g()), lin(c.b()));
        let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
        let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
        let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
        [
            0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
            1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
            0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
        ]
    };
    let (x, y) = (to(a), to(b));
    let k = share.clamp(0.0, 1.0);
    let [l, aa, bb] = [0, 1, 2].map(|i| x[i] * k + y[i] * (1.0 - k));
    let l_ = (l + 0.396_337_78 * aa + 0.215_803_76 * bb).powi(3);
    let m_ = (l - 0.105_561_346 * aa - 0.063_854_17 * bb).powi(3);
    let s_ = (l - 0.089_484_18 * aa - 1.291_485_5 * bb).powi(3);
    Color32::from_rgb(
        gamma(4.076_741_7 * l_ - 3.307_711_6 * m_ + 0.230_969_94 * s_),
        gamma(-1.268_438 * l_ + 2.609_757_4 * m_ - 0.341_319_38 * s_),
        gamma(-0.004_196_086_3 * l_ - 0.703_418_6 * m_ + 1.707_614_7 * s_),
    )
}

/// One entry of a chart's legend: its name, its color and, for a table's figures, its values.
#[derive(Clone, Debug)]
pub struct LegendItem {
    pub key: String,
    pub label: String,
    pub color: Color32,
    pub values: Vec<Option<f64>>,
}

/// The legend a chart of `kind` lists for `spec`, as the web's `chartLegend`: a series per entry
/// for bars, lines and areas; the slices of a pie or a ring; the groups of a scatter; and nothing
/// for a heatmap, whose scale says what its shades mean.
pub fn chart_legend(t: &Tokens, kind: ChartKind, spec: &ChartSpec) -> Vec<LegendItem> {
    match kind {
        ChartKind::Bar | ChartKind::Line | ChartKind::Area => {
            // the series' own figures, not their shares
            let plain = ChartSpec {
                stack: None,
                ..spec.clone()
            };
            plain.pivot().map_or_else(Vec::new, |data| {
                data.series
                    .into_iter()
                    .enumerate()
                    .map(|(index, entry)| LegendItem {
                        key: entry.name.clone(),
                        color: spec.color_of(t, &entry.name, index),
                        label: entry.name,
                        values: entry.values,
                    })
                    .collect()
            })
        }
        ChartKind::Pie | ChartKind::Donut => spec.pivot().map_or_else(Vec::new, |data| {
            let values = data.series.first().map(|s| s.values.clone()).unwrap_or_default();
            data.categories
                .into_iter()
                .enumerate()
                .map(|(index, name)| LegendItem {
                    key: name.clone(),
                    color: spec.color_of(t, &name, index),
                    values: vec![values.get(index).copied().flatten()],
                    label: name,
                })
                .collect()
        }),
        ChartKind::Scatter => scatter_groups(spec)
            .into_iter()
            .enumerate()
            .map(|(index, name)| LegendItem {
                key: name.clone(),
                color: spec.color_of(t, &name, index),
                values: Vec::new(),
                label: name,
            })
            .collect(),
        ChartKind::Heatmap => Vec::new(),
    }
}

/// A scatter's groups: the labels of `series`, or one for all.
fn scatter_groups(spec: &ChartSpec) -> Vec<String> {
    match &spec.series {
        Some(field) => distinct(&spec.rows, field, spec.series_order.as_deref()),
        None => vec![spec.words.all.clone()],
    }
}

/// A chart of rows of one kind, as the web's `Chart`: fills the width it is given, `height` tall
/// (240 by default). Under the pointer it shows the figures there.
#[must_use = "add it with `ui.add(...)`"]
pub struct Chart<'a> {
    kind: ChartKind,
    spec: &'a ChartSpec,
    height: f32,
}

impl<'a> Chart<'a> {
    pub fn new(kind: ChartKind, spec: &'a ChartSpec) -> Self {
        Self {
            kind,
            spec,
            height: 240.0,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl Widget for Chart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (kind, spec, height) = (self.kind, self.spec, self.height);
        let t = tokens(ui.ctx());
        match kind {
            ChartKind::Bar | ChartKind::Line | ChartKind::Area => {
                let Some(data) = spec.pivot() else {
                    return ui
                        .allocate_exact_size(vec2(ui.available_width(), height), Sense::hover())
                        .1;
                };
                let painted: Vec<Series> = data
                    .series
                    .into_iter()
                    .enumerate()
                    .map(|(index, entry)| {
                        let color = spec.color_of(&t, &entry.name, index);
                        entry.painted(color)
                    })
                    .collect();
                let unit = if spec.stack == Some(Stack::Percent) {
                    "%".to_owned()
                } else {
                    spec.unit.clone()
                };
                if kind == ChartKind::Bar {
                    let mut chart = BarChart::new(data.categories)
                        .stacked(spec.stack.is_some())
                        .horizontal(spec.horizontal)
                        .height(height)
                        .parted();
                    chart.writing.unit = unit;
                    chart.writing.format = spec.format.clone();
                    for series in painted {
                        chart = chart.series(series);
                    }
                    ui.add(chart)
                } else {
                    let mut chart = LineChart::new(data.categories)
                        .stacked(spec.stack.is_some())
                        .fill(if kind == ChartKind::Area {
                            Fill::Solid
                        } else {
                            Fill::None
                        })
                        .height(height);
                    chart.writing.unit = unit;
                    chart.writing.format = spec.format.clone();
                    for series in painted {
                        chart = chart.series(series);
                    }
                    ui.add(chart)
                }
            }
            ChartKind::Pie | ChartKind::Donut => slices_chart(ui, &t, spec, kind == ChartKind::Donut, height),
            ChartKind::Heatmap => heatmap_chart(ui, &t, spec, height),
            ChartKind::Scatter => scatter_chart(ui, &t, spec, height),
        }
    }
}

/// The grid's lines across: the border 65% into the card, as the web's `look.grid`.
fn grid_color(t: &Tokens) -> Color32 {
    mix_oklab(t.color("border"), t.color("card"), 0.65)
}

fn axis_font(t: &Tokens) -> FontId {
    FontId::new(t.length("chart-text-size"), FontFamily::Proportional)
}

fn small_font(t: &Tokens) -> FontId {
    FontId::new(t.length("chart-text-size-small"), FontFamily::Proportional)
}

/// A mark's corner, as the web's `look.corner`: 0.4 of the radius token, to the whole pixel.
fn mark_corner(t: &Tokens) -> CornerRadius {
    CornerRadius::same((t.length("radius") * 0.4).floor().max(0.0) as u8)
}

/// One slice of a ring or a pie: its color and its figure.
pub(crate) struct Slice {
    pub color: Color32,
    pub value: f64,
}

/// Slices of a circle from the top clockwise, each its share of the whole, 1 degree apart and
/// edged in the card `edge` wide: a ring from `inner` to `outer`, or a pie with `inner` 0. The
/// slice under the pointer, if any.
pub(crate) fn draw_slices(
    painter: &egui::Painter,
    card: Color32,
    center: Pos2,
    (inner, outer): (f32, f32),
    slices: &[Slice],
    edge: f32,
    pointer: Option<Pos2>,
) -> Option<usize> {
    let total: f64 = slices.iter().map(|s| s.value.max(0.0)).sum();
    let pad = 1f32.to_radians();
    let mut start = -std::f32::consts::FRAC_PI_2;
    let mut hovered = None;
    for (index, slice) in slices.iter().enumerate() {
        if total <= 0.0 || slice.value <= 0.0 {
            continue;
        }
        let sweep = (slice.value / total) as f32 * std::f32::consts::TAU;
        let (from, to) = (start + pad / 2.0, start + sweep - pad / 2.0);
        start += sweep;
        if to <= from {
            continue;
        }
        let steps = ((to - from) / 0.03).ceil().max(2.0) as usize;
        let at = |r: f32, a: f32| pos2(center.x + r * a.cos(), center.y + r * a.sin());
        let mut points: Vec<Pos2> = (0..=steps)
            .map(|i| at(outer, from + (to - from) * i as f32 / steps as f32))
            .collect();
        if inner > 0.0 {
            points.extend(
                (0..=steps)
                    .rev()
                    .map(|i| at(inner, from + (to - from) * i as f32 / steps as f32)),
            );
        } else {
            points.push(center);
        }
        let mut mesh = Mesh::default();
        for i in 0..=steps {
            let a = from + (to - from) * i as f32 / steps as f32;
            mesh.colored_vertex(at(outer, a), slice.color);
            mesh.colored_vertex(at(inner, a), slice.color);
        }
        for i in 0..steps as u32 {
            let k = i * 2;
            mesh.add_triangle(k, k + 1, k + 2);
            mesh.add_triangle(k + 1, k + 3, k + 2);
        }
        painter.add(Shape::mesh(mesh));
        painter.add(Shape::closed_line(points, Stroke::new(edge, card)));
        if let Some(p) = pointer {
            let d = p.distance(center);
            let mut angle = (p.y - center.y).atan2(p.x - center.x);
            if angle < -std::f32::consts::FRAC_PI_2 {
                angle += std::f32::consts::TAU;
            }
            if d >= inner && d <= outer && angle >= from && angle <= to {
                hovered = Some(index);
            }
        }
    }
    hovered
}

/// A pie, or a ring with its total in the middle, as the web's `pie` and `donut` kinds: the circle
/// 82% of the room's half, a ring's hole 58%, a slice per category from the top clockwise.
fn slices_chart(ui: &mut Ui, t: &Arc<Tokens>, spec: &ChartSpec, hole: bool, height: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let Some(data) = spec.pivot() else {
        return response;
    };
    let values = data.series.first().map(|s| s.values.clone()).unwrap_or_default();
    let slices: Vec<Slice> = data
        .categories
        .iter()
        .enumerate()
        .map(|(index, name)| Slice {
            color: spec.color_of(t, name, index),
            value: values.get(index).copied().flatten().unwrap_or(0.0),
        })
        .collect();
    let center = rect.center();
    let radius = rect.width().min(rect.height()) / 2.0;
    let (inner, outer) = if hole {
        (radius * 0.58, radius * 0.82)
    } else {
        (0.0, radius * 0.82)
    };
    let painter = ui.painter();
    let hovered = draw_slices(
        painter,
        t.color("card"),
        center,
        (inner, outer),
        &slices,
        if hole { 2.0 } else { 1.0 },
        response.hover_pos(),
    );
    let total: f64 = values.iter().flatten().sum();
    if hole && let Some(word) = &spec.total {
        let figure = WidgetText::from(
            TextRole::Statistic
                .rich(t, spec.written(Some(total)))
                .size(t.length("chart-stat-size"))
                .variation("wght", 600.0),
        )
        .into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextRole::Statistic.font(t),
        );
        let below = WidgetText::from(TextRole::Help.rich(t, word)).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextRole::Help.font(t),
        );
        let tall = figure.size().y + 2.0 + below.size().y;
        let top = center.y - tall / 2.0;
        let (fw, bw, bh) = (figure.size().x, below.size().x, below.size().y);
        painter.galley(pos2(center.x - fw / 2.0, top), figure, t.color("foreground"));
        painter.galley(
            pos2(center.x - bw / 2.0, top + tall - bh),
            below,
            t.color("muted-foreground"),
        );
    }
    if let Some(index) = hovered {
        let value = slices[index].value;
        let share = if total == 0.0 { 0.0 } else { value / total * 100.0 };
        let row = (
            slices[index].color,
            data.categories[index].clone(),
            format!("{} ({share:.1}%)", spec.written(Some(value))),
        );
        tooltip(response.clone(), t, "", vec![row]);
    }
    response
}

/// A tooltip at the pointer: a title, then a row per figure with its swatch, its name and the
/// figure as written.
fn tooltip(response: Response, t: &Arc<Tokens>, title: &str, rows: Vec<(Color32, String, String)>) {
    let t = t.clone();
    let title = title.to_owned();
    response.on_hover_ui_at_pointer(move |ui| {
        if !title.is_empty() {
            ui.label(TextRole::Label.rich(&t, &title).color(t.color("popover-foreground")));
        }
        for (color, name, value) in rows {
            ui.horizontal(|ui| {
                if color != Color32::TRANSPARENT {
                    let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, color);
                }
                ui.label(TextRole::Help.rich(&t, name).color(t.color("muted-foreground")));
                ui.label(TextRole::Help.rich(&t, value).color(t.color("popover-foreground")));
            });
        }
    });
}

/// A heatmap's scale's colors, as the web's `scaleColors`: one token shaded from the card to it in
/// seven steps (12% to all of it), or the first at 10% and 45% over the card, then each in turn.
pub(crate) fn scale_stops(t: &Tokens, scale: &[String]) -> Vec<Color32> {
    let card = t.color("card");
    match scale {
        [] => vec![card, t.color("chart-2")],
        [one] => {
            let color = token_color(t, one);
            (0..7)
                .map(|i| mix_oklab(color, card, (12.0 + 88.0 * i as f32 / 6.0).round() / 100.0))
                .collect()
        }
        [first, rest @ ..] => {
            let first = token_color(t, first);
            let mut stops = vec![mix_oklab(first, card, 0.1), mix_oklab(first, card, 0.45), first];
            stops.extend(rest.iter().map(|token| token_color(t, token)));
            stops
        }
    }
}

/// The color `share` (0 to 1) of the way along `stops`, mixed between the two it falls between.
pub(crate) fn along(stops: &[Color32], share: f32) -> Color32 {
    if stops.len() < 2 {
        return stops.first().copied().unwrap_or(Color32::TRANSPARENT);
    }
    let share = share.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (share.floor() as usize).min(stops.len() - 2);
    let f = share - i as f32;
    mix_oklab(stops[i + 1], stops[i], f)
}

/// Every how-many-th label of `count` fits in a row `unit` apart: the widest label and 30% more
/// over the room, rounded down, and one more, as ECharts' `interval: 'auto'`.
fn label_every(widest: f32, unit: f32) -> usize {
    ((widest * 1.3).max(7.0) / unit.max(f32::EPSILON)).floor() as usize + 1
}

/// A heatmap, as the web's `heatmap` kind: `x`'s categories along, `series` down (the first at the
/// top), each cell shaded on a scale from the smallest to the largest figure through the scale's
/// tokens (the palette's first by default), with the scale and its ends beside the grid where
/// `show_scale` asks.
fn heatmap_chart(ui: &mut Ui, t: &Arc<Tokens>, spec: &ChartSpec, height: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let Some(data) = spec.pivot() else {
        return response;
    };
    let values: Vec<f64> = data
        .series
        .iter()
        .flat_map(|s| s.values.iter().flatten().copied())
        .collect();
    let scale: Vec<String> = if spec.scale.is_empty() {
        vec![spec.palette_token(0).unwrap_or("chart-2").to_owned()]
    } else {
        spec.scale.clone()
    };
    let stops = scale_stops(t, &scale);
    let span = extent(&values);
    let low = spec.min.or(span.map(|s| s.0)).unwrap_or(0.0);
    let high = spec.max.or(span.map(|s| s.1)).unwrap_or(1.0);
    let painter = ui.painter();
    let font = axis_font(t);
    let ink = t.color("muted-foreground");
    let names: Vec<&str> = data.series.iter().map(|s| s.name.as_str()).collect();
    let width_of = |text: &str| painter.layout_no_wrap(text.to_owned(), font.clone(), ink).size().x;
    let line = painter.layout_no_wrap("0".into(), font.clone(), ink).size().y;
    let label_width = names.iter().map(|n| width_of(n)).fold(0.0, f32::max);
    // the scale's ends, written as the value axis' ticks, over and under a bar 8 wide and 100 tall
    let ends = [spec.tick(high, &spec.unit), spec.tick(low, &spec.unit)];
    let scale_width = if spec.show_scale {
        ends.iter().map(|e| width_of(e)).fold(8.0, f32::max)
    } else {
        0.0
    };
    let right = if spec.show_scale {
        (scale_width + 4.0).max(44.0)
    } else {
        10.0
    };
    let grid = Rect::from_min_max(
        pos2(rect.left() + 4.0 + label_width + 8.0, rect.top() + 10.0),
        pos2(rect.right() - right, rect.bottom() - 4.0 - line - 8.0),
    );
    let (columns, rows) = (data.categories.len().max(1), data.series.len().max(1));
    let cell = vec2(grid.width() / columns as f32, grid.height() / rows as f32);
    let corner = mark_corner(t);
    let shade = |value: f64| {
        let share = if high > low { (value - low) / (high - low) } else { 1.0 };
        along(&stops, share as f32)
    };
    let pointer = response.hover_pos();
    let mut hovered = None;
    for (row, series) in data.series.iter().enumerate() {
        for (column, value) in series.values.iter().enumerate() {
            let Some(value) = value else { continue };
            let one = Rect::from_min_size(
                pos2(grid.left() + column as f32 * cell.x, grid.top() + row as f32 * cell.y),
                cell,
            );
            painter.rect_filled(one.shrink(0.5), corner, shade(*value));
            if pointer.is_some_and(|p| one.contains(p)) {
                hovered = Some((row, column, *value));
            }
        }
    }
    if let Some((row, column, _)) = hovered {
        let one = Rect::from_min_size(
            pos2(grid.left() + column as f32 * cell.x, grid.top() + row as f32 * cell.y),
            cell,
        );
        painter.rect_stroke(
            one.shrink(0.5),
            corner,
            Stroke::new(1.0, t.color("foreground")),
            egui::StrokeKind::Inside,
        );
    }
    let every_row = label_every(line, cell.y);
    for (row, name) in names.iter().enumerate().step_by(every_row) {
        painter.text(
            pos2(grid.left() - 8.0, grid.top() + (row as f32 + 0.5) * cell.y),
            Align2::RIGHT_CENTER,
            name,
            font.clone(),
            ink,
        );
    }
    let widest = data.categories.iter().map(|c| width_of(c)).fold(0.0, f32::max);
    let every_column = label_every(widest, cell.x);
    for (column, name) in data.categories.iter().enumerate().step_by(every_column) {
        painter.text(
            pos2(grid.left() + (column as f32 + 0.5) * cell.x, grid.bottom() + 8.0),
            Align2::CENTER_TOP,
            name,
            font.clone(),
            ink,
        );
    }
    if spec.show_scale {
        let middle = rect.right() - scale_width / 2.0;
        let bar = Rect::from_min_size(pos2(middle - 4.0, rect.top() + 4.0 + line + 4.0), vec2(8.0, 100.0));
        let mut mesh = Mesh::default();
        let steps = 24;
        for i in 0..=steps {
            let share = i as f32 / steps as f32;
            let y = bar.bottom() - share * bar.height();
            let color = along(&stops, share);
            mesh.colored_vertex(pos2(bar.left(), y), color);
            mesh.colored_vertex(pos2(bar.right(), y), color);
        }
        for i in 0..steps {
            let k = (i * 2) as u32;
            mesh.add_triangle(k, k + 1, k + 2);
            mesh.add_triangle(k + 1, k + 3, k + 2);
        }
        painter.add(Shape::mesh(mesh));
        painter.text(
            pos2(middle, bar.top() - 4.0),
            Align2::CENTER_BOTTOM,
            &ends[0],
            font.clone(),
            ink,
        );
        painter.text(
            pos2(middle, bar.bottom() + 4.0),
            Align2::CENTER_TOP,
            &ends[1],
            font.clone(),
            ink,
        );
    }
    if let Some((row, column, value)) = hovered {
        let title = format!("{}, {}", data.categories[column], names[row]);
        let measure = match spec.y.first() {
            Some(field) => spec.named(field).to_owned(),
            None => spec.words.count.clone(),
        };
        tooltip(
            response.clone(),
            t,
            &title,
            vec![(Color32::TRANSPARENT, measure, spec.written(Some(value)))],
        );
    }
    response
}

/// A value axis' ticks: from a round start to a round end about five steps apart, reaching zero
/// unless `fit`, as ECharts' value axis with `scale`.
pub(crate) fn value_ticks(low: f64, high: f64, fit: bool) -> (f64, f64, f64) {
    value_ticks_in(low, high, fit, 5)
}

/// The ticks of a value axis of about `steps` steps.
pub(crate) fn value_ticks_in(low: f64, high: f64, fit: bool, steps: usize) -> (f64, f64, f64) {
    let (mut low, mut high) = (low, high);
    if !fit {
        low = low.min(0.0);
        high = high.max(0.0);
    }
    if high <= low {
        if high == 0.0 {
            high = 1.0;
        } else {
            let pad = high.abs() * 0.5;
            low -= pad;
            high += pad;
        }
    }
    let step = crate::nice_step(high - low, steps);
    let start = (low / step + 1e-9).floor() * step;
    let end = (high / step - 1e-9).ceil() * step;
    (start, end.max(start + step), step)
}

/// A point per row, `x` against `y`, as the web's `scatter` kind: colored by `series`' labels and
/// sized by `value`'s figure (6 to 24 across, by its square root), each point at 70% edged in the
/// card; both axes start at zero unless `fit` fits them to the points, and an axis is named where
/// `field_names` names its field.
fn scatter_chart(ui: &mut Ui, t: &Arc<Tokens>, spec: &ChartSpec, height: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let (Some(x), Some(y)) = (spec.x.as_deref(), spec.y.first().map(String::as_str)) else {
        return response;
    };
    let groups = scatter_groups(spec);
    let sizes: Vec<f64> = match &spec.value {
        Some(field) => spec
            .rows
            .iter()
            .map(|row| number_of(row.get(field)).unwrap_or(0.0))
            .collect(),
        None => Vec::new(),
    };
    let biggest = extent(&sizes).map_or(1.0, |s| s.1).max(1.0);
    // each point: its group, x, y and diameter
    let points: Vec<(usize, f64, f64, f32, usize)> = spec
        .rows
        .iter()
        .enumerate()
        .filter_map(|(at, row)| {
            let group = match &spec.series {
                Some(field) => {
                    let name = label_of(row.get(field));
                    groups.iter().position(|g| *g == name)?
                }
                None => 0,
            };
            let (px, py) = (number_of(row.get(x))?, number_of(row.get(y))?);
            let size = match &spec.value {
                Some(_) => 6.0 + 18.0 * (sizes[at].max(0.0) / biggest).sqrt() as f32,
                None => 7.0,
            };
            Some((group, px, py, size, at))
        })
        .collect();
    let xs: Vec<f64> = points.iter().map(|p| p.1).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.2).collect();
    let (x0, x1) = extent(&xs).unwrap_or((0.0, 1.0));
    let (y0, y1) = extent(&ys).unwrap_or((0.0, 1.0));
    let (xa, xb, xs_step) = value_ticks(x0, x1, spec.fit);
    let (ya, yb, ys_step) = value_ticks_in(y0, y1, spec.fit, crate::steps_for(height));
    let painter = ui.painter();
    let font = axis_font(t);
    let ink = t.color("muted-foreground");
    let line = painter.layout_no_wrap("0".into(), font.clone(), ink).size().y;
    let x_name = spec.field_names.iter().any(|(f, _)| f == x).then(|| spec.named(x));
    let y_name = spec.field_names.iter().any(|(f, _)| f == y).then(|| spec.named(y));
    let ticks = |a: f64, b: f64, step: f64| {
        let count = ((b - a) / step).round() as usize;
        (0..=count).map(move |i| a + step * i as f64)
    };
    let y_labels: Vec<(f64, String)> = ticks(ya, yb, ys_step).map(|v| (v, spec.tick(v, &spec.unit))).collect();
    let x_labels: Vec<(f64, String)> = ticks(xa, xb, xs_step).map(|v| (v, spec.tick(v, ""))).collect();
    let widest = y_labels
        .iter()
        .map(|(_, l)| painter.layout_no_wrap(l.clone(), font.clone(), ink).size().x)
        .fold(0.0, f32::max);
    let top = if y_name.is_some() { 22.0 } else { 10.0 };
    let bottom = if x_name.is_some() { 26.0 } else { 4.0 } + line + 8.0;
    let plot = Rect::from_min_max(
        pos2(rect.left() + 4.0 + widest + 8.0, rect.top() + top),
        pos2(rect.right() - 10.0, rect.bottom() - bottom),
    );
    let at_x = |v: f64| plot.left() + ((v - xa) / (xb - xa)) as f32 * plot.width();
    let at_y = |v: f64| plot.bottom() - ((v - ya) / (yb - ya)) as f32 * plot.height();
    let grid = Stroke::new(1.0, grid_color(t));
    for (value, label) in &y_labels {
        painter.hline(plot.x_range(), at_y(*value), grid);
        painter.text(
            pos2(plot.left() - 8.0, at_y(*value)),
            Align2::RIGHT_CENTER,
            label,
            font.clone(),
            ink,
        );
    }
    // the x labels as many as fit, the rest hidden as ECharts' `hideOverlap`
    let mut last_right = f32::NEG_INFINITY;
    for (value, label) in &x_labels {
        painter.vline(at_x(*value), plot.y_range(), grid);
        let galley = painter.layout_no_wrap(label.clone(), font.clone(), ink);
        let left = at_x(*value) - galley.size().x / 2.0;
        if left > last_right + 4.0 {
            painter.galley(pos2(left, plot.bottom() + 8.0), galley.clone(), ink);
            last_right = left + galley.size().x;
        }
    }
    let small = small_font(t);
    if let Some(name) = x_name {
        painter.text(
            pos2(plot.center().x, plot.bottom() + 22.0 + line / 2.0),
            Align2::CENTER_TOP,
            name,
            small.clone(),
            ink,
        );
    }
    if let Some(name) = y_name {
        painter.text(
            pos2(plot.left(), plot.top() - 6.0),
            Align2::CENTER_BOTTOM,
            name,
            small,
            ink,
        );
    }
    let card = t.color("card");
    let pointer = response.hover_pos();
    let mut nearest: Option<(f32, usize)> = None;
    for (index, (group, px, py, size, _)) in points.iter().enumerate() {
        let center = pos2(at_x(*px), at_y(*py));
        let color = spec.color_of(t, &groups[*group], *group);
        painter.circle(center, size / 2.0, color.gamma_multiply(0.7), Stroke::new(1.0, card));
        if let Some(p) = pointer {
            let d = p.distance(center);
            if d <= size / 2.0 + 3.0 && nearest.is_none_or(|(best, _)| d < best) {
                nearest = Some((d, index));
            }
        }
    }
    if let Some((_, index)) = nearest {
        let (group, px, py, size, at) = points[index];
        let color = spec.color_of(t, &groups[group], group);
        painter.circle_stroke(pos2(at_x(px), at_y(py)), size / 2.0 + 1.0, Stroke::new(1.5, color));
        let bare = |v: f64| spec.written_bare(v);
        let mut rows = vec![
            (Color32::TRANSPARENT, spec.named(x).to_owned(), bare(px)),
            (Color32::TRANSPARENT, spec.named(y).to_owned(), spec.written(Some(py))),
        ];
        if let Some(field) = &spec.value {
            rows.push((Color32::TRANSPARENT, spec.named(field).to_owned(), bare(sizes[at])));
        }
        tooltip(response.clone(), t, &groups[group], rows);
    }
    response
}

/// A chart of rows in a chart's frame, as sokkel-ui's `ChartPanel` (and its `ChartFrame`): the
/// title and subtitle in the frame's head, the chart `height` tall (256 by default, as `h-64`),
/// and the kind's legend under it; with `empty` the body says so instead. Its expand and menu are
/// the panel's tools: draw it inside a `PanelMenu`, and in a `PanelWindow` for full screen, as any
/// dashboard panel.
#[must_use = "draw it with `show`"]
pub struct ChartPanel<'a> {
    kind: ChartKind,
    spec: &'a ChartSpec,
    title: String,
    subtitle: Option<String>,
    info: Option<String>,
    empty: bool,
    words: PanelWords,
    legend: bool,
    height: f32,
}

impl<'a> ChartPanel<'a> {
    pub fn new(title: impl Into<String>, kind: ChartKind, spec: &'a ChartSpec) -> Self {
        Self {
            kind,
            spec,
            title: title.into(),
            subtitle: None,
            info: None,
            empty: false,
            words: PanelWords::default(),
            legend: true,
            height: 256.0,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// What the panel shows, behind the Info mark beside its title.
    pub fn info(mut self, info: impl Into<String>) -> Self {
        self.info = Some(info.into());
        self
    }

    /// Nothing to draw: the body says `words.empty` instead.
    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    pub fn words(mut self, words: PanelWords) -> Self {
        self.words = words;
        self
    }

    /// The legend under the chart, shown unless turned off (a heatmap lists none).
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// The chart's height, 256 when not given.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let head = Head {
            subtitle: self.subtitle.as_deref(),
            info: self.info.as_deref(),
            empty: self.empty.then_some(self.words.empty.as_str()),
            ..Head::new(&self.title)
        };
        let (kind, spec, height, legend) = (self.kind, self.spec, self.height, self.legend);
        chart_frame(ui, head, |ui| {
            let height = fill_height(ui, height);
            ui.add(Chart::new(kind, spec).height(height));
            let under = ui.min_rect().bottom();
            if legend {
                let t = tokens(ui.ctx());
                let items = chart_legend(&t, kind, spec);
                if !items.is_empty() {
                    ui.add_space(4.0);
                    let pairs: Vec<(&str, Color32)> = items.iter().map(|i| (i.label.as_str(), i.color)).collect();
                    legend_of(ui, &pairs);
                }
            }
            drawn_under(ui, under);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Cell;

    fn runs() -> Vec<Row> {
        [
            ("nightly-export", "succeeded", 120.0),
            ("nightly-export", "failed", 40.0),
            ("sync-inventory", "succeeded", 60.0),
            ("rebuild-index", "succeeded", 30.0),
        ]
        .into_iter()
        .map(|(job, state, ms)| {
            Row::of([
                ("job", Cell::from(job)),
                ("state", Cell::from(state)),
                ("duration", Cell::Number(ms)),
            ])
        })
        .collect()
    }

    #[test]
    fn a_spec_pivots_its_rows_and_stacks_to_shares() {
        let spec = ChartSpec::new(runs()).x("job").y("duration").series("state");
        let data = spec.pivot().expect("an x");
        assert_eq!(data.categories, ["nightly-export", "sync-inventory", "rebuild-index"]);
        assert_eq!(data.series[1].values, [Some(40.0), None, None]);
        let shares = spec.clone().stack(Stack::Percent).pivot().expect("an x");
        assert_eq!(shares.series[0].values[0], Some(75.0));
        assert!(ChartSpec::new(runs()).pivot().is_none());
    }

    #[test]
    fn values_are_written_by_the_format_and_the_unit() {
        let spec = ChartSpec::new(Vec::new()).unit(" ms");
        assert_eq!(spec.written(Some(1234.5)), "1,234.5 ms");
        assert_eq!(spec.written(None), "-");
        assert_eq!(spec.tick(1500.0, &spec.unit), "1.5K ms");
        let spec = spec.format(|v| format!("${}", sokkel_egui::format_value(v, Some(0))));
        assert_eq!(spec.tick(30_000.0, ""), "$30K");
    }

    #[test]
    fn value_axes_reach_zero_unless_fitted() {
        assert_eq!(value_ticks(120.0, 540.0, false), (0.0, 600.0, 100.0));
        assert_eq!(value_ticks(120.0, 540.0, true), (100.0, 600.0, 100.0));
        assert_eq!(value_ticks(3.0, 29.0, false), (0.0, 30.0, 5.0));
        assert_eq!(value_ticks(0.0, 0.0, false), (0.0, 1.0, 0.2));
    }

    #[test]
    fn kinds_go_by_the_webs_names() {
        for kind in [
            ChartKind::Bar,
            ChartKind::Line,
            ChartKind::Area,
            ChartKind::Pie,
            ChartKind::Donut,
            ChartKind::Heatmap,
            ChartKind::Scatter,
        ] {
            assert_eq!(ChartKind::from_id(kind.id()), Some(kind));
        }
        assert_eq!(ChartKind::from_id("map"), None);
    }

    #[test]
    fn oklab_mixes_land_between_their_ends() {
        let (a, b) = (Color32::from_rgb(0, 100, 200), Color32::WHITE);
        assert_eq!(mix_oklab(a, b, 1.0), a);
        assert_eq!(mix_oklab(a, b, 0.0), b);
        let half = mix_oklab(a, b, 0.5);
        assert!(half.r() > a.r() && half.r() < b.r());
    }
}
