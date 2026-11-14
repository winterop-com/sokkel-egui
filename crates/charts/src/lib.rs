//! sokkel-ui-egui's charts: one engine, egui's painter, as sokkel-ui draws every chart with one
//! engine. Each chart reads the `chart-1` to `chart-5` series colors, the chart text sizes and the
//! frame's muted ink and rules from the tokens, so it follows the theme and the scheme.
//!
//! A chart takes categories along x and named series of values, fills the width it is given and
//! takes its height from the caller. Under the pointer a line or a bar chart shows a guide and the
//! values at that category.

use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Widget, pos2,
    vec2,
};
use std::sync::Arc;

mod panel;
pub use panel::TimeSeriesPanel;

use sokkel_ui_egui::{TextRole, Tokens, tokens};

/// One named run of values, one per category; `None` leaves a gap.
#[derive(Clone, Debug)]
pub struct Series {
    pub name: String,
    pub values: Vec<Option<f64>>,
}

impl Series {
    pub fn new(name: impl Into<String>, values: impl IntoIterator<Item = f64>) -> Self {
        Self {
            name: name.into(),
            values: values.into_iter().map(Some).collect(),
        }
    }
}

/// The order series take their colors in, as sokkel-ui's charts take them (`SERIES_ORDER`):
/// `chart-1` is left to a single figure's own color.
pub const SERIES_ORDER: [&str; 8] = [
    "chart-2",
    "chart-3",
    "chart-4",
    "kind-teal",
    "kind-pink",
    "kind-green",
    "kind-blue",
    "chart-5",
];

/// The color of the n-th series, in `SERIES_ORDER`, round again past the eighth.
pub fn series_color(t: &Tokens, index: usize) -> Color32 {
    t.color(SERIES_ORDER[index % SERIES_ORDER.len()])
}

fn axis_font(t: &Tokens) -> FontId {
    FontId::new(t.length("chart-text-size"), FontFamily::Proportional)
}

/// A step for about `ticks` intervals between 0 and `max`, rounded as ECharts rounds its axis
/// interval: 1, 2, 3 or 5 times a power of ten.
fn nice_step(max: f64, ticks: usize) -> f64 {
    let raw = (max / ticks as f64).max(f64::MIN_POSITIVE);
    let power = 10f64.powf(raw.log10().floor());
    let fraction = raw / power;
    let nice = if fraction < 1.5 {
        1.0
    } else if fraction < 2.5 {
        2.0
    } else if fraction < 4.0 {
        3.0
    } else if fraction < 7.0 {
        5.0
    } else {
        10.0
    };
    nice * power
}

/// How a value axis writes its values: a unit after each, and a fixed number of decimals.
#[derive(Clone, Default)]
struct Writing {
    unit: String,
    decimals: Option<usize>,
}

impl Writing {
    /// A value on the axis: its decimals with the trailing zeros dropped, and the unit.
    fn tick(&self, value: f64) -> String {
        let number = match self.decimals {
            Some(d) => {
                let fixed = format!("{value:.d$}");
                if fixed.contains('.') {
                    fixed.trim_end_matches('0').trim_end_matches('.').to_owned()
                } else {
                    fixed
                }
            }
            None => label(value),
        };
        format!("{number}{}", self.unit)
    }

    /// A value read out, as under the pointer: all its decimals, and the unit.
    fn value(&self, value: f64) -> String {
        match self.decimals {
            Some(d) => format!("{value:.d$}{}", self.unit),
            None => format!("{}{}", label(value), self.unit),
        }
    }
}

/// A value as an axis label: whole numbers grouped by thousands, small ones with a decimal.
fn label(value: f64) -> String {
    if value.abs() >= 1000.0 && value.fract() == 0.0 {
        let whole = format!("{}", value.abs() as i64);
        let grouped: Vec<String> = whole
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect();
        format!("{}{}", if value < 0.0 { "-" } else { "" }, grouped.join(","))
    } else if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}

/// The plot's frame: the y scale from 0 to a nice top, its rules and labels, the x labels under
/// it, and the rect left for the marks.
struct Frame {
    plot: Rect,
    top: f64,
    /// Points on the axis' ends, as a line's are, rather than in the middle of each category's band.
    edge: bool,
    count: usize,
}

/// What a frame is drawn with beyond its categories and top value.
#[derive(Default)]
struct Axes {
    writing: Writing,
    /// Every nth category labeled; as many as fit when `None`.
    every: Option<usize>,
    edge: bool,
}

impl Frame {
    fn draw(ui: &Ui, rect: Rect, t: &Tokens, categories: &[String], max: f64, axes: &Axes) -> Self {
        let painter = ui.painter();
        let font = axis_font(t);
        let ink = t.color("muted-foreground");
        let rule = t.color("border");
        let max = if max > 0.0 { max } else { 1.0 };
        let step = nice_step(max, 5);
        let top = (max / step - 1e-9).ceil().max(1.0) * step;
        let ticks = (top / step).round() as usize;
        let widest = (0..=ticks)
            .map(|i| {
                painter
                    .layout_no_wrap(axes.writing.tick(i as f64 * step), font.clone(), ink)
                    .size()
                    .x
            })
            .fold(0.0, f32::max);
        // the x labels 8 under the plot, the chart's foot 4 under them, as ECharts lays its grid
        let line = painter.layout_no_wrap("0".into(), font.clone(), ink).size().y;
        let plot = Rect::from_min_max(
            pos2(rect.left() + widest + 8.0, rect.top() + 6.0),
            pos2(rect.right() - 10.0, rect.bottom() - 12.0 - line),
        );
        for i in 0..=ticks {
            let value = i as f64 * step;
            let y = plot.bottom() - (value / top) as f32 * plot.height();
            // the baseline is a rule; the lines above it are quieter
            let stroke = if i == 0 {
                Stroke::new(1.0, rule)
            } else {
                Stroke::new(1.0, rule.gamma_multiply(0.5))
            };
            painter.hline(plot.x_range(), y, stroke);
            painter.text(
                pos2(plot.left() - 8.0, y),
                Align2::RIGHT_CENTER,
                axes.writing.tick(value),
                font.clone(),
                ink,
            );
        }
        let frame = Self {
            plot,
            top,
            edge: axes.edge,
            count: categories.len(),
        };
        // every nth label as asked, or as many as fit, evenly
        let label_width = categories
            .iter()
            .map(|c| painter.layout_no_wrap(c.clone(), font.clone(), ink).size().x)
            .fold(0.0, f32::max);
        let fits = (plot.width() / (label_width + 16.0)).max(1.0) as usize;
        let every = axes.every.unwrap_or_else(|| categories.len().div_ceil(fits)).max(1);
        for (i, category) in categories.iter().enumerate().filter(|(i, _)| i % every == 0) {
            let x = frame.x(i);
            painter.text(
                pos2(x, plot.bottom() + 8.0),
                Align2::CENTER_TOP,
                category,
                font.clone(),
                ink,
            );
        }
        frame
    }

    /// Where category `i` stands across: on the axis' ends for a line, in its band's middle otherwise.
    fn x(&self, i: usize) -> f32 {
        if self.edge {
            let gaps = self.count.saturating_sub(1).max(1) as f32;
            self.plot.left() + self.plot.width() * i as f32 / gaps
        } else {
            self.plot.left() + self.band(self.count) * (i as f32 + 0.5)
        }
    }

    fn y(&self, value: f64) -> f32 {
        self.plot.bottom() - (value / self.top) as f32 * self.plot.height()
    }

    fn band(&self, count: usize) -> f32 {
        self.plot.width() / count.max(1) as f32
    }

    /// The category under the pointer, if it is over the plot: the nearest point on a line.
    fn hovered(&self, response: &Response, count: usize) -> Option<usize> {
        let pos = response.hover_pos()?;
        if !self.plot.expand2(vec2(0.0, 8.0)).contains(pos) {
            return None;
        }
        let at = if self.edge {
            let gaps = count.saturating_sub(1).max(1) as f32;
            ((pos.x - self.plot.left()) / self.plot.width() * gaps).round().max(0.0) as usize
        } else {
            ((pos.x - self.plot.left()) / self.band(count)) as usize
        };
        Some(at.min(count.saturating_sub(1)))
    }
}

/// A legend: a swatch and a name per series, in a row.
pub fn legend(ui: &mut Ui, names: &[&str]) {
    let t = tokens(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        // a line of the legend is 24 tall, as the web's legend items are
        ui.spacing_mut().interact_size.y = 24.0;
        ui.spacing_mut().item_spacing.x = 16.0;
        for (i, name) in names.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(2), series_color(&t, i));
                ui.label(TextRole::Help.rich(&t, *name).color(t.color("muted-foreground")));
            });
        }
    });
}

fn tooltip(
    response: Response,
    t: Arc<Tokens>,
    category: &str,
    rows: Vec<(Color32, String, Option<f64>)>,
    writing: &Writing,
) {
    response.on_hover_ui_at_pointer(|ui| {
        ui.label(TextRole::Label.rich(&t, category).color(t.color("popover-foreground")));
        for (color, name, value) in rows {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                ui.painter().circle_filled(rect.center(), 4.0, color);
                ui.label(TextRole::Help.rich(&t, name).color(t.color("muted-foreground")));
                if let Some(value) = value {
                    ui.label(
                        TextRole::Help
                            .rich(&t, writing.value(value))
                            .color(t.color("popover-foreground")),
                    );
                }
            });
        }
    });
}

fn max_of<'a>(values: impl Iterator<Item = &'a Option<f64>>) -> f64 {
    values.flatten().copied().fold(0.0, f64::max)
}

/// Lines over categories, one per series, with an optional shaded band (a range such as a forecast's
/// interval) and an optional area under the first series.
#[must_use = "add it with `ui.add(...)`"]
pub struct LineChart {
    categories: Vec<String>,
    series: Vec<Series>,
    band: Option<(Vec<f64>, Vec<f64>)>,
    area: bool,
    fill: Fill,
    writing: Writing,
    every: Option<usize>,
    thresholds: Vec<(f64, Color32)>,
    height: f32,
}

/// How the areas under a line chart's series are filled: not at all, fading from each line toward
/// the axis (40% of its color to 4%), or evenly (15%).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fill {
    #[default]
    None,
    Gradient,
    Solid,
}

impl LineChart {
    pub fn new(categories: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            categories: categories.into_iter().map(Into::into).collect(),
            series: Vec::new(),
            band: None,
            area: false,
            fill: Fill::None,
            writing: Writing::default(),
            every: None,
            thresholds: Vec::new(),
            height: 240.0,
        }
    }

    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// The area under every series filled, as the `area` kind fills it.
    pub fn fill(mut self, fill: Fill) -> Self {
        self.fill = fill;
        self
    }

    /// The unit written after each value, on the axis and under the pointer ("ms", "%").
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.writing.unit = unit.into();
        self
    }

    /// The decimals a value is written with; the axis drops trailing zeros.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.writing.decimals = Some(decimals);
        self
    }

    /// Labels every nth category on the axis, from the first; as many as fit when not given.
    pub fn label_every(mut self, every: usize) -> Self {
        self.every = Some(every);
        self
    }

    /// A dashed line across at `value` in `color`, a threshold's step, drawn where the axis reaches it.
    pub fn threshold(mut self, value: f64, color: Color32) -> Self {
        self.thresholds.push((value, color));
        self
    }

    /// A shaded range from `low` to `high` per category, in the first series' color.
    pub fn band(mut self, low: Vec<f64>, high: Vec<f64>) -> Self {
        self.band = Some((low, high));
        self
    }

    /// A wash under the first series.
    pub fn area(mut self, area: bool) -> Self {
        self.area = area;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl Widget for LineChart {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let count = self.categories.len();
        let band_top = self
            .band
            .as_ref()
            .map_or(0.0, |(_, high)| high.iter().copied().fold(0.0, f64::max));
        let max = self
            .series
            .iter()
            .map(|s| max_of(s.values.iter()))
            .fold(band_top, f64::max);
        let axes = Axes {
            writing: self.writing.clone(),
            every: self.every,
            edge: true,
        };
        let frame = Frame::draw(ui, rect, &t, &self.categories, max, &axes);
        let x = |i: usize| frame.x(i);
        let painter = ui.painter();
        // the areas first, so every line lies over every area; the highest point is the gradient's top
        if self.fill != Fill::None {
            let crest = frame.y(max);
            let floor = frame.plot.bottom();
            for (index, series) in self.series.iter().enumerate() {
                let color = series_color(&t, index);
                let shade = |y: f32| match self.fill {
                    Fill::Gradient => {
                        let at = ((y - crest) / (floor - crest).max(1.0)).clamp(0.0, 1.0);
                        color.gamma_multiply(0.40 + (0.04 - 0.40) * at)
                    }
                    _ => color.gamma_multiply(0.15),
                };
                let mut mesh = egui::Mesh::default();
                for i in 0..series.values.len().saturating_sub(1) {
                    if let (Some(a), Some(b)) = (series.values[i], series.values[i + 1]) {
                        let base = mesh.vertices.len() as u32;
                        let (ya, yb) = (frame.y(a), frame.y(b));
                        mesh.colored_vertex(pos2(x(i), ya), shade(ya));
                        mesh.colored_vertex(pos2(x(i + 1), yb), shade(yb));
                        mesh.colored_vertex(pos2(x(i + 1), floor), shade(floor));
                        mesh.colored_vertex(pos2(x(i), floor), shade(floor));
                        mesh.add_triangle(base, base + 1, base + 2);
                        mesh.add_triangle(base, base + 2, base + 3);
                    }
                }
                painter.add(Shape::mesh(mesh));
            }
        }
        for (value, color) in &self.thresholds {
            if *value <= frame.top {
                let y = frame.y(*value);
                painter.add(Shape::dashed_line(
                    &[pos2(frame.plot.left(), y), pos2(frame.plot.right(), y)],
                    Stroke::new(1.25, *color),
                    4.0,
                    4.0,
                ));
            }
        }
        if let Some((low, high)) = &self.band {
            let color = series_color(&t, 0).gamma_multiply(0.18);
            // a band is concave in general; fill it a slice at a time
            for i in 0..high.len().saturating_sub(1) {
                let quad = vec![
                    pos2(x(i), frame.y(high[i])),
                    pos2(x(i + 1), frame.y(high[i + 1])),
                    pos2(x(i + 1), frame.y(low[i + 1])),
                    pos2(x(i), frame.y(low[i])),
                ];
                painter.add(Shape::convex_polygon(quad, color, Stroke::NONE));
            }
        }
        for (index, series) in self.series.iter().enumerate() {
            let color = series_color(&t, index);
            if self.area && index == 0 && self.fill == Fill::None {
                for i in 0..series.values.len().saturating_sub(1) {
                    if let (Some(a), Some(b)) = (series.values[i], series.values[i + 1]) {
                        let quad = vec![
                            pos2(x(i), frame.y(a)),
                            pos2(x(i + 1), frame.y(b)),
                            pos2(x(i + 1), frame.plot.bottom()),
                            pos2(x(i), frame.plot.bottom()),
                        ];
                        painter.add(Shape::convex_polygon(quad, color.gamma_multiply(0.14), Stroke::NONE));
                    }
                }
            }
            // a run of values between gaps is one line
            let mut run: Vec<Pos2> = Vec::new();
            for (i, value) in series.values.iter().enumerate() {
                match value {
                    Some(v) => run.push(pos2(x(i), frame.y(*v))),
                    None => {
                        painter.add(Shape::line(std::mem::take(&mut run), Stroke::new(2.0, color)));
                    }
                }
            }
            painter.add(Shape::line(run, Stroke::new(2.0, color)));
        }
        if let Some(i) = frame.hovered(&response, count) {
            painter.vline(x(i), frame.plot.y_range(), Stroke::new(1.0, t.color("border-strong")));
            for (index, series) in self.series.iter().enumerate() {
                if let Some(Some(v)) = series.values.get(i) {
                    painter.circle(
                        pos2(x(i), frame.y(*v)),
                        4.0,
                        t.color("background"),
                        Stroke::new(2.0, series_color(&t, index)),
                    );
                }
            }
            let rows = self
                .series
                .iter()
                .enumerate()
                .map(|(index, s)| {
                    (
                        series_color(&t, index),
                        s.name.clone(),
                        s.values.get(i).copied().flatten(),
                    )
                })
                .collect();
            let category = self.categories[i].clone();
            tooltip(response.clone(), t.clone(), &category, rows, &self.writing);
        }
        response
    }
}

/// Bars over categories, the series side by side in each, or stacked.
#[must_use = "add it with `ui.add(...)`"]
pub struct BarChart {
    categories: Vec<String>,
    series: Vec<Series>,
    stacked: bool,
    height: f32,
}

impl BarChart {
    pub fn new(categories: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            categories: categories.into_iter().map(Into::into).collect(),
            series: Vec::new(),
            stacked: false,
            height: 240.0,
        }
    }

    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl Widget for BarChart {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let count = self.categories.len();
        let total = |i: usize| {
            self.series
                .iter()
                .filter_map(|s| s.values.get(i).copied().flatten())
                .sum::<f64>()
        };
        let max = if self.stacked {
            (0..count).map(total).fold(0.0, f64::max)
        } else {
            self.series.iter().map(|s| max_of(s.values.iter())).fold(0.0, f64::max)
        };
        let frame = Frame::draw(ui, rect, &t, &self.categories, max, &Axes::default());
        let step = frame.band(count);
        let hovered = frame.hovered(&response, count);
        let painter = ui.painter();
        if let Some(i) = hovered {
            let column = Rect::from_x_y_ranges(
                frame.plot.left() + step * i as f32..=frame.plot.left() + step * (i + 1) as f32,
                frame.plot.y_range(),
            );
            painter.rect_filled(column, CornerRadius::ZERO, t.color("hover"));
        }
        let groups = if self.stacked { 1 } else { self.series.len().max(1) };
        let width = (step * 0.7 / groups as f32).min(28.0);
        let top = CornerRadius {
            nw: 2,
            ne: 2,
            sw: 0,
            se: 0,
        };
        for i in 0..count {
            let center = frame.plot.left() + step * (i as f32 + 0.5);
            let mut base = 0.0;
            for (index, series) in self.series.iter().enumerate() {
                let Some(value) = series.values.get(i).copied().flatten() else {
                    continue;
                };
                let (left, from, to) = if self.stacked {
                    let from = base;
                    base += value;
                    (center - width / 2.0, from, base)
                } else {
                    (center - width * groups as f32 / 2.0 + width * index as f32, 0.0, value)
                };
                let bar = Rect::from_min_max(pos2(left + 1.0, frame.y(to)), pos2(left + width - 1.0, frame.y(from)));
                painter.rect_filled(bar, top, series_color(&t, index));
            }
        }
        if let Some(i) = hovered {
            let rows = self
                .series
                .iter()
                .enumerate()
                .map(|(index, s)| {
                    (
                        series_color(&t, index),
                        s.name.clone(),
                        s.values.get(i).copied().flatten(),
                    )
                })
                .collect();
            let category = self.categories[i].clone();
            tooltip(response.clone(), t.clone(), &category, rows, &Writing::default());
        }
        response
    }
}

/// A small line with no axis, for a table cell or a stat: its values scaled to its box, a wash
/// under it and a dot on the last value.
#[must_use = "add it with `ui.add(...)`"]
pub struct Sparkline {
    values: Vec<f64>,
    size: egui::Vec2,
    color: Option<usize>,
}

impl Sparkline {
    pub fn new(values: impl IntoIterator<Item = f64>) -> Self {
        Self {
            values: values.into_iter().collect(),
            size: vec2(120.0, 32.0),
            color: None,
        }
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = vec2(width, height);
        self
    }

    /// The chart color to draw in, 0 to 4 for `chart-1` to `chart-5`.
    pub fn color(mut self, series: usize) -> Self {
        self.color = Some(series);
        self
    }
}

impl Widget for Sparkline {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());
        if self.values.len() < 2 {
            return response;
        }
        let color = series_color(&t, self.color.unwrap_or(0));
        let (low, high) = self
            .values
            .iter()
            .fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
        let span = (high - low).max(f64::EPSILON);
        let inner = rect.shrink2(vec2(3.0, 3.0));
        let at = |i: usize, v: f64| {
            pos2(
                inner.left() + inner.width() * i as f32 / (self.values.len() - 1) as f32,
                inner.bottom() - ((v - low) / span) as f32 * inner.height(),
            )
        };
        let points: Vec<Pos2> = self.values.iter().enumerate().map(|(i, v)| at(i, *v)).collect();
        let painter = ui.painter();
        for pair in points.windows(2) {
            let quad = vec![
                pair[0],
                pair[1],
                pos2(pair[1].x, rect.bottom()),
                pos2(pair[0].x, rect.bottom()),
            ];
            painter.add(Shape::convex_polygon(quad, color.gamma_multiply(0.12), Stroke::NONE));
        }
        painter.add(Shape::line(points.clone(), Stroke::new(1.5, color)));
        if let Some(last) = points.last() {
            painter.circle_filled(*last, 2.5, color);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_ones_twos_threes_or_fives_as_echarts_picks() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(close(nice_step(186.0, 5), 30.0));
        assert!(close(nice_step(0.14, 5), 0.03));
        assert!(close(nice_step(100.0, 5), 20.0));
        assert!(close(nice_step(1284.0, 5), 300.0));
        assert!(close(nice_step(40.0, 5), 10.0));
    }

    #[test]
    fn axis_values_drop_trailing_zeros_and_keep_the_unit() {
        let writing = Writing {
            unit: "%".into(),
            decimals: Some(2),
        };
        assert_eq!(writing.tick(0.0), "0%");
        assert_eq!(writing.tick(0.15), "0.15%");
        assert_eq!(writing.tick(0.1), "0.1%");
        assert_eq!(writing.value(0.1), "0.10%");
    }

    #[test]
    fn labels_group_thousands() {
        assert_eq!(label(1284.0), "1,284");
        assert_eq!(label(12.0), "12");
        assert_eq!(label(0.5), "0.5");
        assert_eq!(label(-25000.0), "-25,000");
    }
}
