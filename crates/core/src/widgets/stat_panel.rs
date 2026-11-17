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

/// A figure written short, as `Intl.NumberFormat`'s compact notation writes it with one decimal at
/// most: `999`, `1.2K`, `30K`, `577.2K`, `12.3K`, `1.3M`.
pub fn format_compact(value: f64) -> String {
    let size = value.abs();
    let sign = if value < 0.0 { "-" } else { "" };
    let steps = [(1e12, "T"), (1e9, "B"), (1e6, "M"), (1e3, "K")];
    let short = |v: f64| {
        let rounded = (v * 10.0).round() / 10.0;
        if rounded.fract() == 0.0 {
            format!("{}", rounded as i64)
        } else {
            format!("{rounded:.1}")
        }
    };
    for (index, (scale, suffix)) in steps.iter().enumerate() {
        if size >= *scale {
            let figure = (size / scale * 10.0).round() / 10.0;
            // 999.95K rounds to 1000K, which reads as the next step up
            if figure >= 1000.0 && index > 0 {
                let (up, word) = steps[index - 1];
                return format!("{sign}{}{word}", short(size / up));
            }
            return format!("{sign}{}{suffix}", short(size / scale));
        }
    }
    // 999.95 rounds to 1000, which compact notation writes as the next step up
    if (1000.0 - 0.05..1000.0).contains(&size) {
        return format!("{sign}1K");
    }
    format!("{sign}{}", short(size))
}

/// The words a stat tile says.
#[derive(Clone, Debug)]
pub struct StatWords {
    pub no_data: String,
    /// What the info mark is called for a screen reader.
    pub info: String,
}

impl Default for StatWords {
    fn default() -> Self {
        Self {
            no_data: "No data".into(),
            info: "About this panel".into(),
        }
    }
}

/// One reading as a dashboard tile, as sokkel-ui's `StatPanel`: a small title, the value written
/// large, a trend drawn faintly behind it and a change against an earlier period. `thresholds`
/// pick the reading's color, which inks the value and its trend; `color` does when it has none. A
/// `None` value says there is no data, in neutral ink and without its trend. In a dashboard grid it
/// stretches to its row, and an arrangement's tools sit at the end of its head.
/// Where a stat tile shows its threshold color, as sokkel-ui's `colorMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatColorMode {
    /// The value and its trend in the color.
    #[default]
    Value,
    /// The whole tile filled with it, its words in `on-fill`.
    Background,
    /// No color: the value in the card's ink.
    None,
}

pub struct StatPanel<'a> {
    color_mode: StatColorMode,
    title: String,
    value: Option<f64>,
    unit: Option<String>,
    decimals: Option<usize>,
    thresholds: &'a [Threshold],
    color: Option<&'static str>,
    sparkline: &'a [f64],
    delta: Option<String>,
    good: Option<bool>,
    format: Option<&'a dyn Fn(f64) -> String>,
    target: Option<(f64, Option<(f64, f64)>)>,
    info: Option<String>,
    note: Option<String>,
    words: StatWords,
}

impl<'a> StatPanel<'a> {
    pub fn new(title: impl Into<String>, value: Option<f64>) -> Self {
        Self {
            color_mode: StatColorMode::Value,
            title: title.into(),
            value,
            unit: None,
            decimals: None,
            thresholds: &[],
            color: None,
            sparkline: &[],
            delta: None,
            good: None,
            format: None,
            target: None,
            info: None,
            note: None,
            words: StatWords::default(),
        }
    }

    /// Where the threshold color shows: the value (the default), the whole tile, or nowhere.
    pub fn color_mode(mut self, mode: StatColorMode) -> Self {
        self.color_mode = mode;
        self
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

    /// Writes the value; `decimals` and grouped thousands when not given.
    pub fn format(mut self, format: &'a dyn Fn(f64) -> String) -> Self {
        self.format = Some(format);
        self
    }

    /// The value the reading aims for, drawn as a meter under it with the target written out, as
    /// the web's `target`; the meter runs from 0 to the larger of the two unless `range` is given.
    pub fn target(mut self, target: f64, range: Option<(f64, f64)>) -> Self {
        self.target = Some((target, range));
        self
    }

    /// What the tile counts, said in a tooltip on the Info mark in its head.
    pub fn info(mut self, info: impl Into<String>) -> Self {
        self.info = Some(info.into());
        self
    }

    /// A line under the value, muted: `MRR times twelve`, `3 invoices`.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn words(mut self, words: StatWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let cell = ui.scope_id();
        let width = ui.available_width();
        // taller with a target's meter, as the web's `min-h-36`
        let natural = if self.target.is_some() && self.value.is_some() {
            144.0
        } else {
            112.0
        };
        let height = cell_height(ui).unwrap_or(0.0).max(natural);
        report_natural_height(ui, cell, natural);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let radius = t.radius("surface");
        let tone = self
            .value
            .and_then(|v| threshold_color(Some(v), self.thresholds).or(self.color));
        let filled = self.color_mode == StatColorMode::Background && tone.is_some();
        let ink = match self.color_mode {
            StatColorMode::Value => tone.map(|k| t.color(k)),
            StatColorMode::Background if filled => Some(t.color("on-fill")),
            _ => None,
        };
        let muted = if filled {
            t.color("on-fill").gamma_multiply(0.9)
        } else {
            t.color("muted-foreground")
        };
        let painter = ui.painter().clone();
        match tone.filter(|_| filled) {
            Some(tone) => fill_toward(&painter, &t, rect, radius, t.color(tone)),
            None => {
                painter.rect_filled(rect, radius, t.color("card"));
            }
        }
        // the trend behind the value, the lower 45%, fading toward the foot
        if self.value.is_some() && self.sparkline.len() >= 2 {
            let color = ink.unwrap_or(muted);
            let area = egui::Rect::from_min_max(pos2(rect.left(), rect.bottom() - rect.height() * 0.45), rect.max);
            spark_area(&painter.with_clip_rect(rect.shrink(1.0)), area, self.sparkline, color);
        }
        if !filled {
            painter.rect_stroke(
                rect,
                radius,
                Stroke::new(1.0, t.color("border")),
                egui::StrokeKind::Inside,
            );
        }
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
        if let Some(info) = &self.info {
            super::info_tip(ui, pos2(end - 10.0, head.center().y), info, &self.words.info);
            end -= 24.0;
        }
        if let (Some(delta), Some(_)) = (&self.delta, self.value) {
            let color = match self.good {
                _ if filled => muted,
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
                let written = match self.format {
                    Some(format) => format(value),
                    None => format_value(value, self.decimals),
                };
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
                let note = self.note.as_ref().map(|n| {
                    WidgetText::from(TextRole::Help.rich(&t, n)).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        body.width(),
                        TextRole::Help.font(&t),
                    )
                });
                let note_height = note.as_ref().map_or(0.0, |n| n.size().y);
                // the meter: 8 down, a 6 tall track, 4, the target's line, 12 under it
                let target_line = self.target.map(|(target, _)| {
                    let written = match self.format {
                        Some(format) => format(target),
                        None => format_value(target, self.decimals),
                    };
                    let written = format!("{written}{}", self.unit.clone().unwrap_or_default());
                    (
                        WidgetText::from(TextRole::Help.rich(&t, "Target")).into_galley(
                            ui,
                            Some(TextWrapMode::Extend),
                            f32::INFINITY,
                            TextRole::Help.font(&t),
                        ),
                        WidgetText::from(
                            egui::RichText::new(written).font(egui::FontId::monospace(TextRole::Help.font(&t).size)),
                        )
                        .into_galley(
                            ui,
                            Some(TextWrapMode::Extend),
                            f32::INFINITY,
                            TextRole::Help.font(&t),
                        ),
                    )
                });
                let meter_height = target_line
                    .as_ref()
                    .map_or(0.0, |(label, _)| 8.0 + 6.0 + 4.0 + label.size().y + 12.0);
                let total = figure.size().x + unit.as_ref().map_or(0.0, |u| u.size().x + 4.0);
                let x = body.center().x - total / 2.0;
                // the value, the note and the meter under it centered together
                let top = body.center().y - (figure.size().y + note_height + meter_height) / 2.0;
                if let Some(note) = note {
                    let at = pos2(body.center().x - note.size().x / 2.0, top + figure.size().y);
                    painter.galley(at, note, muted);
                }
                if let (Some((target, range)), Some((label, figure_text))) = (self.target, target_line) {
                    let reading = ink.unwrap_or(muted);
                    let width = body.width().min(192.0);
                    let track_top = top + figure.size().y + note_height + 8.0;
                    let track =
                        egui::Rect::from_min_size(pos2(body.center().x - width / 2.0, track_top), vec2(width, 6.0));
                    let (low, high) = range.unwrap_or((0.0, value.max(target)));
                    let at = |v: f64| (((v - low) / (high - low).max(f64::MIN_POSITIVE)).clamp(0.0, 1.0)) as f32;
                    let corner = t.radius("mini");
                    painter.rect_filled(track, corner, reading.gamma_multiply(0.15));
                    let reach = egui::Rect::from_min_size(track.min, vec2(width * at(value), 6.0));
                    painter.rect_filled(reach, corner, reading);
                    let mark = track.left() + width * at(target);
                    let tick = egui::Rect::from_min_max(
                        pos2(mark - 1.0, track.top() - 4.0),
                        pos2(mark + 1.0, track.bottom() + 4.0),
                    );
                    let tick_ink = if filled {
                        t.color("on-fill")
                    } else {
                        t.color("foreground")
                    };
                    painter.rect_filled(tick, corner, tick_ink);
                    let line_top = track.bottom() + 4.0;
                    let both = label.size().x + 4.0 + figure_text.size().x;
                    let left = body.center().x - both / 2.0;
                    let label_width = label.size().x;
                    painter.galley(pos2(left, line_top), label, muted);
                    painter.galley(pos2(left + label_width + 4.0, line_top), figure_text, muted);
                }
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

/// A tile filled with its tone, as the web's background mode: a 135 degree gradient from the tone
/// mixed 8% toward `on-fill` to the tone itself by 70% of the way; in the dark scheme from 80% to
/// 62% of the tone over the background.
fn fill_toward(
    painter: &egui::Painter,
    t: &crate::Tokens,
    rect: egui::Rect,
    radius: egui::CornerRadius,
    tone: Color32,
) {
    let mix = |a: Color32, b: Color32, share: f32| {
        let c = |x: u8, y: u8| (f32::from(x) * share + f32::from(y) * (1.0 - share)).round() as u8;
        Color32::from_rgb(c(a.r(), b.r()), c(a.g(), b.g()), c(a.b(), b.b()))
    };
    let (from, to, stop) = if t.dark {
        (
            mix(tone, t.color("background"), 0.8),
            mix(tone, t.color("background"), 0.62),
            1.0,
        )
    } else {
        (mix(tone, t.color("on-fill"), 0.92), tone, 0.7)
    };
    // the rounded tile tessellated, each vertex colored by where it falls along the diagonal
    let mut mesh = Mesh::default();
    let mut tessellator = egui::epaint::Tessellator::new(
        painter.ctx().pixels_per_point(),
        egui::epaint::TessellationOptions::default(),
        [1, 1],
        Vec::new(),
    );
    tessellator.tessellate_shape(egui::Shape::rect_filled(rect, radius, Color32::WHITE), &mut mesh);
    let span = rect.width() + rect.height();
    for vertex in &mut mesh.vertices {
        let along = ((vertex.pos.x - rect.left()) + (vertex.pos.y - rect.top())) / span;
        let share = (along / stop).clamp(0.0, 1.0);
        let color = mix(to, from, share);
        let alpha = vertex.color.a();
        vertex.color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
    }
    painter.add(Shape::mesh(mesh));
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
