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

/// The series color for the n-th series: the theme's five chart colors, in turn.
pub fn series_color(t: &Tokens, index: usize) -> Color32 {
    t.color(&format!("chart-{}", index % 5 + 1))
}

fn axis_font(t: &Tokens) -> FontId {
    FontId::new(t.length("chart-text-size"), FontFamily::Proportional)
}

/// A step for `ticks` lines between 0 and `max`: 1, 2 or 5 times a power of ten.
fn nice_step(max: f64, ticks: usize) -> f64 {
    let raw = (max / ticks as f64).max(f64::MIN_POSITIVE);
    let power = 10f64.powf(raw.log10().floor());
    let fraction = raw / power;
    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * power
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
}

impl Frame {
    fn draw(ui: &Ui, rect: Rect, t: &Tokens, categories: &[String], max: f64) -> Self {
        let painter = ui.painter();
        let font = axis_font(t);
        let ink = t.color("muted-foreground");
        let rule = t.color("border");
        let step = nice_step(max.max(1.0), 4);
        let top = (max / step).ceil().max(1.0) * step;
        let widest = (0..=((top / step) as usize))
            .map(|i| {
                painter
                    .layout_no_wrap(label(i as f64 * step), font.clone(), ink)
                    .size()
                    .x
            })
            .fold(0.0, f32::max);
        let plot = Rect::from_min_max(
            pos2(rect.left() + widest + 8.0, rect.top() + 6.0),
            pos2(rect.right(), rect.bottom() - 20.0),
        );
        for i in 0..=((top / step) as usize) {
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
                label(value),
                font.clone(),
                ink,
            );
        }
        let band = plot.width() / categories.len().max(1) as f32;
        // as many x labels as fit, evenly
        let fits = (plot.width() / 64.0).max(1.0) as usize;
        let every = categories.len().div_ceil(fits).max(1);
        for (i, category) in categories.iter().enumerate().filter(|(i, _)| i % every == 0) {
            let x = plot.left() + band * (i as f32 + 0.5);
            painter.text(
                pos2(x, plot.bottom() + 6.0),
                Align2::CENTER_TOP,
                category,
                font.clone(),
                ink,
            );
        }
        Self { plot, top }
    }

    fn y(&self, value: f64) -> f32 {
        self.plot.bottom() - (value / self.top) as f32 * self.plot.height()
    }

    fn band(&self, count: usize) -> f32 {
        self.plot.width() / count.max(1) as f32
    }

    /// The category under the pointer, if it is over the plot.
    fn hovered(&self, response: &Response, count: usize) -> Option<usize> {
        let pos = response.hover_pos()?;
        if !self.plot.expand2(vec2(0.0, 8.0)).contains(pos) {
            return None;
        }
        Some((((pos.x - self.plot.left()) / self.band(count)) as usize).min(count.saturating_sub(1)))
    }
}

/// A legend: a swatch and a name per series, in a row.
pub fn legend(ui: &mut Ui, names: &[&str]) {
    let t = tokens(ui.ctx());
    ui.horizontal_wrapped(|ui| {
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

fn tooltip(response: Response, t: Arc<Tokens>, category: &str, rows: Vec<(Color32, String, Option<f64>)>) {
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
                            .rich(&t, label(value))
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
    height: f32,
}

impl LineChart {
    pub fn new(categories: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            categories: categories.into_iter().map(Into::into).collect(),
            series: Vec::new(),
            band: None,
            area: false,
            height: 240.0,
        }
    }

    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
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
        let frame = Frame::draw(ui, rect, &t, &self.categories, max);
        let step = frame.band(count);
        let x = |i: usize| frame.plot.left() + step * (i as f32 + 0.5);
        let painter = ui.painter();
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
            if self.area && index == 0 {
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
            tooltip(response.clone(), t.clone(), &category, rows);
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
        let frame = Frame::draw(ui, rect, &t, &self.categories, max);
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
            tooltip(response.clone(), t.clone(), &category, rows);
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
    fn steps_are_one_two_or_five_times_a_power_of_ten() {
        assert_eq!(nice_step(100.0, 4), 50.0);
        assert_eq!(nice_step(37.0, 4), 10.0);
        assert_eq!(nice_step(1284.0, 4), 500.0);
        assert_eq!(nice_step(0.8, 4), 0.2);
    }

    #[test]
    fn labels_group_thousands() {
        assert_eq!(label(1284.0), "1,284");
        assert_eq!(label(12.0), "12");
        assert_eq!(label(0.5), "0.5");
        assert_eq!(label(-25000.0), "-25,000");
    }
}
