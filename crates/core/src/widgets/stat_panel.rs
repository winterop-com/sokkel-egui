//! One reading as a dashboard tile, after sokkel-ui's `StatPanel`, and the threshold scale its
//! color comes from (`dashboard/thresholds`).

use egui::{Color32, Mesh, Sense, Shape, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use super::dashboard::{cell_height, report_natural_height};
use crate::{Icon, TextRole, icons, tokens};

/// One step of a threshold scale, as sokkel-ui's `Threshold`: from `value` up, a reading takes the
/// color token `color`. The first step's `value` is ignored, so it covers everything below the second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Threshold {
    pub value: Option<f64>,
    pub color: &'static str,
}

impl Threshold {
    /// The first step: everything below the next.
    pub const fn base(color: &'static str) -> Self {
        Self { value: None, color }
    }

    pub const fn from(value: f64, color: &'static str) -> Self {
        Self {
            value: Some(value),
            color,
        }
    }
}

/// The token a reading takes on a threshold scale, as sokkel-ui's `thresholdColor`.
pub fn threshold_color(value: Option<f64>, thresholds: &[Threshold]) -> Option<&'static str> {
    let first = thresholds.first()?;
    let Some(value) = value.filter(|v| v.is_finite()) else {
        return Some(first.color);
    };
    let mut color = first.color;
    for step in &thresholds[1..] {
        if step.value.is_some_and(|at| value >= at) {
            color = step.color;
        }
    }
    Some(color)
}

/// A number written for a reader, as sokkel-ui's `formatValue`: `decimals` places (none from 100
/// up or for a whole number when not given), thousands grouped with commas.
pub fn format_value(value: f64, decimals: Option<usize>) -> String {
    let places = decimals.unwrap_or(if value.abs() >= 100.0 || value.fract() == 0.0 {
        0
    } else {
        1
    });
    let written = format!("{:.*}", places, value.abs());
    let (whole, fraction) = written
        .split_once('.')
        .map_or((written.as_str(), None), |(w, f)| (w, Some(f)));
    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let sign = if value < 0.0 && written.chars().any(|c| c.is_ascii_digit() && c != '0') {
        "-"
    } else {
        ""
    };
    match fraction {
        Some(f) => format!("{sign}{grouped}.{f}"),
        None => format!("{sign}{grouped}"),
    }
}

/// The words a stat tile says.
#[derive(Clone, Debug)]
pub struct StatWords {
    pub no_data: String,
}

impl Default for StatWords {
    fn default() -> Self {
        Self {
            no_data: "No data".into(),
        }
    }
}

/// One reading as a dashboard tile, as sokkel-ui's `StatPanel`: a small title, the value written
/// large, a trend drawn faintly behind it and a change against an earlier period. `thresholds`
/// pick the reading's color, which inks the value and its trend; `color` does when it has none. A
/// `None` value says there is no data, in neutral ink and without its trend. In a dashboard grid it
/// stretches to its row, and an arrangement's tools sit at the end of its head.
pub struct StatPanel<'a> {
    title: String,
    value: Option<f64>,
    unit: Option<String>,
    decimals: Option<usize>,
    thresholds: &'a [Threshold],
    color: Option<&'static str>,
    sparkline: &'a [f64],
    delta: Option<String>,
    good: Option<bool>,
    words: StatWords,
}

impl<'a> StatPanel<'a> {
    pub fn new(title: impl Into<String>, value: Option<f64>) -> Self {
        Self {
            title: title.into(),
            value,
            unit: None,
            decimals: None,
            thresholds: &[],
            color: None,
            sparkline: &[],
            delta: None,
            good: None,
            words: StatWords::default(),
        }
    }

    /// Written after the value: `%`, `ms`, `/min`.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        let unit = unit.into();
        self.unit = (!unit.is_empty()).then_some(unit);
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }

    pub fn thresholds(mut self, thresholds: &'a [Threshold]) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// A fixed color token, when the tile has no thresholds.
    pub fn color(mut self, color: &'static str) -> Self {
        self.color = Some(color);
        self
    }

    /// The values behind the reading, drawn as a faint area.
    pub fn sparkline(mut self, values: &'a [f64]) -> Self {
        self.sparkline = values;
        self
    }

    /// The change written for a reader: `+12%`, `-0.4s`.
    pub fn delta(mut self, delta: impl Into<String>) -> Self {
        self.delta = Some(delta.into());
        self
    }

    /// Whether the change is an improvement; neutral when not given.
    pub fn good(mut self, good: Option<bool>) -> Self {
        self.good = good;
        self
    }

    pub fn words(mut self, words: StatWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let cell = ui.id();
        let width = ui.available_width();
        let natural = 112.0;
        let height = cell_height(ui).unwrap_or(0.0).max(natural);
        report_natural_height(ui, cell, natural);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let radius = t.radius("surface");
        let tone = self
            .value
            .and_then(|v| threshold_color(Some(v), self.thresholds).or(self.color));
        let ink = tone.map(|k| t.color(k));
        let muted = t.color("muted-foreground");
        let painter = ui.painter().clone();
        painter.rect_filled(rect, radius, t.color("card"));
        // the trend behind the value, the lower 45%, fading toward the foot
        if self.value.is_some() && self.sparkline.len() >= 2 {
            let color = ink.unwrap_or(muted);
            let area = egui::Rect::from_min_max(pos2(rect.left(), rect.bottom() - rect.height() * 0.45), rect.max);
            spark_area(&painter.with_clip_rect(rect.shrink(1.0)), area, self.sparkline, color);
        }
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, t.color("border")),
            egui::StrokeKind::Inside,
        );
        // the head: the title muted, the change at its end, then an arrangement's tools
        let head = egui::Rect::from_min_size(rect.min + vec2(10.0, 4.0), vec2(rect.width() - 20.0, 24.0));
        let mut tools = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("stat-panel-tools")
                .max_rect(head)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        super::panel_tools(&mut tools, cell);
        let tools_width = tools.min_rect().width();
        let mut end = head.right() - if tools_width > 1.0 { tools_width + 4.0 } else { 0.0 };
        if let (Some(delta), Some(_)) = (&self.delta, self.value) {
            let color = match self.good {
                None => muted,
                Some(true) => t.color("good"),
                Some(false) => t.color("critical"),
            };
            let galley = WidgetText::from(TextRole::Help.rich(&t, delta).variation("wght", 500.0)).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextRole::Help.font(&t),
            );
            let x = end - galley.size().x;
            painter.galley(pos2(x, head.center().y - galley.size().y / 2.0), galley, color);
            let arrow = if delta.starts_with('-') {
                Icon::ArrowDownRight
            } else {
                Icon::ArrowUpRight
            };
            icons::paint(&painter, pos2(x - 9.0, head.center().y), arrow, 14.0, color);
            end = x - 20.0;
        }
        let title = WidgetText::from(TextRole::Help.rich(&t, &self.title).variation("wght", 500.0)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            (end - head.left()).max(1.0),
            TextRole::Help.font(&t),
        );
        painter.galley(pos2(head.left(), head.center().y - title.size().y / 2.0), title, muted);
        // the value, centered in what is left, 28 over the foot
        let body = egui::Rect::from_min_max(
            pos2(rect.left() + 10.0, head.bottom()),
            pos2(rect.right() - 10.0, rect.bottom() - 28.0),
        );
        match self.value {
            None => {
                let galley = WidgetText::from(TextRole::DetailTitle.rich(&t, &self.words.no_data)).into_galley(
                    ui,
                    Some(TextWrapMode::Truncate),
                    body.width(),
                    TextRole::DetailTitle.font(&t),
                );
                painter.galley(body.center() - galley.size() / 2.0, galley, muted);
            }
            Some(value) => {
                let written = format_value(value, self.decimals);
                let figure = WidgetText::from(TextRole::Statistic.rich(&t, written)).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextRole::Statistic.font(&t),
                );
                let unit = self.unit.as_ref().map(|u| {
                    WidgetText::from(TextRole::Body.rich(&t, u).variation("wght", 500.0)).into_galley(
                        ui,
                        Some(TextWrapMode::Extend),
                        f32::INFINITY,
                        TextRole::Body.font(&t),
                    )
                });
                let total = figure.size().x + unit.as_ref().map_or(0.0, |u| u.size().x + 4.0);
                let x = body.center().x - total / 2.0;
                let top = body.center().y - figure.size().y / 2.0;
                let base = top + figure.rows.first().map_or(0.0, |r| r.max_y());
                let figure_width = figure.size().x;
                painter.galley(pos2(x, top), figure, ink.unwrap_or(t.color("card-foreground")));
                if let Some(unit) = unit {
                    let unit_base = unit.rows.first().map_or(0.0, |r| r.max_y());
                    painter.galley(
                        pos2(x + figure_width + 4.0, base - unit_base),
                        unit,
                        ink.unwrap_or(muted),
                    );
                }
            }
        }
    }
}

/// An area under a line of values, stretched to `area`: 32% of the color to 4%, the line at 70%.
fn spark_area(painter: &egui::Painter, area: egui::Rect, values: &[f64], color: Color32) {
    let (min, max) = values.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(*v), hi.max(*v))
    });
    let span = if max - min == 0.0 { 1.0 } else { max - min };
    let last = (values.len() - 1) as f32;
    // the web's 40-high box: values from 36 down to 4
    let point = |i: usize, v: f64| {
        let y = 36.0 - ((v - min) / span) as f32 * 32.0;
        pos2(
            area.left() + i as f32 / last * area.width(),
            area.top() + y / 40.0 * area.height(),
        )
    };
    let mut mesh = Mesh::default();
    let top = color.gamma_multiply(0.32);
    let foot = color.gamma_multiply(0.04);
    let shade = |y: f32| {
        let at = ((y - area.top()) / area.height().max(1.0)).clamp(0.0, 1.0);
        top.lerp_to_gamma(foot, at)
    };
    for i in 0..values.len() - 1 {
        let (a, b) = (point(i, values[i]), point(i + 1, values[i + 1]));
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(a, shade(a.y));
        mesh.colored_vertex(b, shade(b.y));
        mesh.colored_vertex(pos2(b.x, area.bottom()), foot);
        mesh.colored_vertex(pos2(a.x, area.bottom()), foot);
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(Shape::mesh(mesh));
    let line: Vec<_> = values.iter().enumerate().map(|(i, v)| point(i, *v)).collect();
    painter.add(Shape::line(line, Stroke::new(1.25, color.gamma_multiply(0.7))));
}
