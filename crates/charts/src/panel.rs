//! A dashboard's chart panels, after sokkel-ui's `TimeSeriesPanel` in its `ChartFrame`.

use egui::{Color32, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};
use sokkel_egui::{TextRole, Threshold, cell_height, format_value, report_natural_height, threshold_color, tokens};

use crate::{BarChart, Fill, Gauge, LineChart, Series, color_of, legend_of};

/// A dashboard panel of values over time: lines over fading areas, dashed threshold lines, and a
/// legend under it, in a chart's frame (a card with the plain border, its title and subtitle in a
/// 32 tall head, the chart 192 tall under it). Laid in a `DashboardGrid`, it stretches to its row.
#[must_use = "draw it with `show`"]
pub struct TimeSeriesPanel {
    title: String,
    subtitle: Option<String>,
    chart: LineChart,
    series: Vec<Series>,
    legend: bool,
    height: f32,
}

impl TimeSeriesPanel {
    /// A panel titled `title` over the times in `x`, written for a reader.
    pub fn new(title: impl Into<String>, x: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            chart: LineChart::new(x).fill(Fill::Gradient),
            series: Vec::new(),
            legend: true,
            height: 192.0,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn series(mut self, series: Series) -> Self {
        self.series.push(Series {
            values: Vec::new(),
            ..series.clone()
        });
        self.chart = self.chart.series(series);
        self
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.chart = self.chart.unit(unit);
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.chart = self.chart.decimals(decimals);
        self
    }

    /// Lines alone, or the areas filled evenly, rather than fading.
    pub fn fill(mut self, fill: Fill) -> Self {
        self.chart = self.chart.fill(fill);
        self
    }

    pub fn label_every(mut self, every: usize) -> Self {
        self.chart = self.chart.label_every(every);
        self
    }

    /// A threshold's step: a dashed line across at `value`, in `color`.
    pub fn threshold(mut self, value: f64, color: Color32) -> Self {
        self.chart = self.chart.threshold(value, color);
        self
    }

    /// Whether the legend is drawn under the chart.
    /// Each series on top of those before it, as shares of a whole.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.chart = self.chart.stacked(stacked);
        self
    }

    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// The chart's height, 192 (`h-48`) when not given.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let (chart, height, legend_on, names) = (self.chart, self.height, self.legend, self.series);
        chart_frame(ui, &self.title, self.subtitle.as_deref(), |ui| {
            ui.add(chart.height(height));
            if legend_on && !names.is_empty() {
                ui.add_space(4.0);
                let t = tokens(ui.ctx());
                let items: Vec<(&str, Color32)> = names
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (s.name.as_str(), color_of(&t, s, i)))
                    .collect();
                legend_of(ui, &items);
            }
        });
    }
}

/// A chart's frame, as sokkel-ui's `ChartFrame`: a card with the plain border, its title and
/// subtitle in a 32 tall head with an arrangement's tools at its end, the body 8 in and under. In
/// a `DashboardGrid` it stretches to its row.
fn chart_frame(ui: &mut Ui, title: &str, subtitle: Option<&str>, body: impl FnOnce(&mut Ui)) {
    let t = tokens(ui.ctx());
    let width = ui.available_width();
    let stretch = cell_height(ui).unwrap_or(0.0);
    let cell = ui.id();
    let ink = t.color("muted-foreground");
    egui::Frame::new()
        .fill(t.color("card"))
        .stroke(Stroke::new(1.0, t.color("border")))
        .corner_radius(t.radius("surface"))
        .show(ui, |ui| {
            ui.set_width(width - 2.0);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            // the head: 32 tall, 12 in, the title and subtitle 6 apart
            let inner = width - 2.0 - 18.0;
            let title = WidgetText::from(TextRole::Help.rich(&t, title).variation("wght", 500.0)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                inner.max(1.0),
                TextRole::Help.font(&t),
            );
            let subtitle = subtitle.map(|s| {
                WidgetText::from(TextRole::Help.rich(&t, s)).into_galley(
                    ui,
                    Some(TextWrapMode::Truncate),
                    (inner - title.size().x - 6.0).max(1.0),
                    TextRole::Help.font(&t),
                )
            });
            let (head, _) = ui.allocate_exact_size(vec2(width - 2.0, 32.0), Sense::hover());
            let top = head.center().y - title.size().y / 2.0;
            let title_width = title.size().x;
            ui.painter()
                .galley(pos2(head.left() + 12.0, top), title, t.color("card-foreground"));
            if let Some(subtitle) = subtitle {
                ui.painter()
                    .galley(pos2(head.left() + 12.0 + title_width + 6.0, top), subtitle, ink);
            }
            // an arrangement's tools at the head's end
            let mut tools = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("chart-panel-tools")
                    .max_rect(head.shrink2(vec2(6.0, 0.0)))
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            sokkel_egui::panel_tools(&mut tools, cell);
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 8,
                    top: 0,
                    bottom: 8,
                })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    body(ui);
                });
            let used = ui.min_rect().height() + 2.0;
            report_natural_height(ui, cell, used);
            if stretch > used {
                ui.add_space(stretch - used);
            }
        });
}

/// A dashboard panel of one reading on an arc, as sokkel-ui's `GaugePanel`: the arc filled in the
/// color its thresholds give it, the thresholds as a thin band outside it, on a 128 tall body.
#[must_use = "draw it with `show`"]
pub struct GaugePanel<'a> {
    title: String,
    value: Option<f64>,
    min: f64,
    max: f64,
    thresholds: &'a [Threshold],
    color: &'static str,
    unit: String,
    decimals: Option<usize>,
}

impl<'a> GaugePanel<'a> {
    pub fn new(title: impl Into<String>, value: Option<f64>) -> Self {
        Self {
            title: title.into(),
            value,
            min: 0.0,
            max: 100.0,
            thresholds: &[],
            color: "chart-2",
            unit: String::new(),
            decimals: None,
        }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn thresholds(mut self, thresholds: &'a [Threshold]) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// The arc's color token when there are no thresholds.
    pub fn color(mut self, color: &'static str) -> Self {
        self.color = color;
        self
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let tone = t.color(threshold_color(self.value, self.thresholds).unwrap_or(self.color));
        let written = self
            .value
            .map(|v| format!("{}{}", format_value(v, self.decimals), self.unit))
            .unwrap_or_default();
        let steps = self.thresholds.iter().map(|s| (s.value, t.color(s.color))).collect();
        let gauge = Gauge::new(self.value, self.min, self.max, tone, written).steps(steps);
        chart_frame(ui, &self.title, None, |ui| {
            ui.add(gauge);
        });
    }
}

/// A dashboard panel of readings as bars against one scale from `min` to `max`, as sokkel-ui's
/// `BarGauge`: a row each, its name in the code face, a bar on the muted track fading in from the
/// track to the color its thresholds give it, and its value in that color.
#[must_use = "draw it with `show`"]
pub struct BarGauge<'a> {
    title: String,
    bars: Vec<(String, Option<f64>)>,
    min: f64,
    max: f64,
    thresholds: &'a [Threshold],
    color: &'static str,
    unit: String,
    decimals: Option<usize>,
}

impl<'a> BarGauge<'a> {
    pub fn new(title: impl Into<String>, bars: impl IntoIterator<Item = (String, Option<f64>)>) -> Self {
        Self {
            title: title.into(),
            bars: bars.into_iter().collect(),
            min: 0.0,
            max: 100.0,
            thresholds: &[],
            color: "chart-2",
            unit: String::new(),
            decimals: None,
        }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn thresholds(mut self, thresholds: &'a [Threshold]) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// The bars' color token when there are no thresholds.
    pub fn color(mut self, color: &'static str) -> Self {
        self.color = color;
        self
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
        let value_font = sokkel_egui::mono_font(
            ui.ctx(),
            sokkel_egui::mono_size(&t, TextRole::Help.font(&t).size),
            500.0,
        );
        chart_frame(ui, &self.title, None, |ui| {
            ui.add_space(4.0);
            let width = ui.available_width();
            // each row as tall as the small text's line (the bar 16 in it), 6 between, as the
            // web's `text-xs` and `gap-1.5`
            let row = t.text("help").line_height.max(16.0);
            let pitch = row + 6.0;
            let height = self.bars.len() as f32 * pitch - if self.bars.is_empty() { 0.0 } else { 6.0 };
            let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            let painter = ui.painter();
            let corner = egui::CornerRadius::same((t.length("radius") * 0.4).round() as u8);
            for (i, (name, value)) in self.bars.iter().enumerate() {
                let top = rect.top() + i as f32 * pitch;
                let middle = top + row / 2.0;
                let tone = t.color(threshold_color(*value, self.thresholds).unwrap_or(self.color));
                let written = value.map_or("-".to_owned(), |v| {
                    format!("{}{}", format_value(v, self.decimals), self.unit)
                });
                let value_galley = painter.layout_no_wrap(written, value_font.clone(), tone);
                let value_width = value_galley.size().x.max(40.0);
                let name_galley = painter.layout_no_wrap(name.clone(), mono.clone(), t.color("muted-foreground"));
                // the name in a column of 96, the bar 8 after it, the value 8 after the bar
                painter.galley(
                    pos2(rect.left(), middle - name_galley.size().y / 2.0),
                    name_galley,
                    t.color("muted-foreground"),
                );
                painter.galley(
                    pos2(
                        rect.right() - value_galley.size().x,
                        middle - value_galley.size().y / 2.0,
                    ),
                    value_galley,
                    tone,
                );
                let track = egui::Rect::from_min_max(
                    pos2(rect.left() + 96.0 + 8.0, middle - 8.0),
                    pos2(rect.right() - value_width - 8.0, middle + 8.0),
                );
                painter.rect_filled(track, corner, t.color("muted"));
                let share = value.map_or(0.0, |v| ((v - self.min) / (self.max - self.min)).clamp(0.0, 1.0)) as f32;
                if share > 0.0 {
                    let bar = egui::Rect::from_min_size(track.min, vec2(track.width() * share, track.height()));
                    // from the tone at 30% over the track to the tone itself
                    let faint = tone.gamma_multiply(0.3);
                    let mut mesh = egui::Mesh::default();
                    mesh.colored_vertex(bar.left_top(), faint);
                    mesh.colored_vertex(bar.right_top(), tone);
                    mesh.colored_vertex(bar.right_bottom(), tone);
                    mesh.colored_vertex(bar.left_bottom(), faint);
                    mesh.add_triangle(0, 1, 2);
                    mesh.add_triangle(0, 2, 3);
                    painter.add(egui::Shape::mesh(mesh));
                }
            }
            ui.add_space(4.0);
        });
    }
}

/// A dashboard panel of a heatmap, as sokkel-ui's `HeatmapPanel`: a value for each pair of an x
/// and a y category, each cell shaded on a scale from 0 to the largest value, from the card
/// through `scale`'s tokens, with the scale and its ends beside the grid, on a 192 tall body.
#[must_use = "draw it with `show`"]
pub struct HeatmapPanel {
    title: String,
    x: Vec<String>,
    y: Vec<String>,
    cells: Vec<(usize, usize, f64)>,
    scale: Vec<&'static str>,
    unit: String,
    decimals: Option<usize>,
}

impl HeatmapPanel {
    pub fn new(
        title: impl Into<String>,
        x: impl IntoIterator<Item = impl Into<String>>,
        y: impl IntoIterator<Item = impl Into<String>>,
        cells: impl IntoIterator<Item = (usize, usize, f64)>,
    ) -> Self {
        Self {
            title: title.into(),
            x: x.into_iter().map(Into::into).collect(),
            y: y.into_iter().map(Into::into).collect(),
            cells: cells.into_iter().collect(),
            scale: vec!["warning"],
            unit: String::new(),
            decimals: None,
        }
    }

    /// The scale's tokens: one to shade toward from the card, or several to step through.
    pub fn scale(mut self, scale: impl IntoIterator<Item = &'static str>) -> Self {
        self.scale = scale.into_iter().collect();
        self
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let card = t.color("card");
        let mix = |a: Color32, share: f32| {
            let c = |x: u8, y: u8| (f32::from(x) * share + f32::from(y) * (1.0 - share)).round() as u8;
            Color32::from_rgb(c(a.r(), card.r()), c(a.g(), card.g()), c(a.b(), card.b()))
        };
        // the scale's stops: the first token at 10% and 45% over the card, then each token, as
        // the web's `scaleColors`; one token alone shades from the card to it in seven steps
        let stops: Vec<Color32> = match self.scale.as_slice() {
            [one] => (0..7).map(|i| mix(t.color(one), i as f32 / 6.0)).collect(),
            [first, rest @ ..] => {
                let mut stops = vec![mix(t.color(first), 0.1), mix(t.color(first), 0.45), t.color(first)];
                stops.extend(rest.iter().map(|token| t.color(token)));
                stops
            }
            [] => vec![card, t.color("chart-2")],
        };
        let at = |share: f32| {
            let share = share.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
            let i = (share.floor() as usize).min(stops.len() - 2);
            let f = share - i as f32;
            let (a, b) = (stops[i], stops[i + 1]);
            let c = |x: u8, y: u8| (f32::from(x) * (1.0 - f) + f32::from(y) * f).round() as u8;
            Color32::from_rgb(c(a.r(), b.r()), c(a.g(), b.g()), c(a.b(), b.b()))
        };
        let high = self
            .cells
            .iter()
            .map(|c| c.2)
            .fold(0.0_f64, f64::max)
            .max(f64::MIN_POSITIVE);
        let font = TextRole::Help.font(&t);
        let ink = t.color("muted-foreground");
        // the scale's ends as the web's ticks: at most one decimal, none trailing
        let written = |v: f64| {
            let one = format!("{v:.1}");
            format!("{}{}", one.trim_end_matches('0').trim_end_matches('.'), self.unit)
        };
        chart_frame(ui, &self.title, None, |ui| {
            let width = ui.available_width();
            let (rect, _) = ui.allocate_exact_size(vec2(width, 192.0), Sense::hover());
            let painter = ui.painter();
            let label_width = self
                .y
                .iter()
                .map(|name| painter.layout_no_wrap(name.clone(), font.clone(), ink).size().x)
                .fold(0.0, f32::max);
            // the grid 10 from the top, the x labels under it, the scale 44 at the right
            let grid = egui::Rect::from_min_max(
                pos2(rect.left() + 4.0 + label_width + 8.0, rect.top() + 10.0),
                pos2(rect.right() - 44.0, rect.bottom() - 4.0 - 20.0),
            );
            let (columns, rows) = (self.x.len().max(1), self.y.len().max(1));
            let cell = vec2(grid.width() / columns as f32, grid.height() / rows as f32);
            // the web's `look.corner`, a share of the radius, to the whole pixel
            let corner = egui::CornerRadius::same((t.length("radius") * 0.4).floor() as u8);
            for (column, row, value) in &self.cells {
                // the last series at the top, as the web's reversed order on an inverted axis
                let shown_row = rows - 1 - row;
                let min = pos2(
                    grid.left() + *column as f32 * cell.x,
                    grid.top() + shown_row as f32 * cell.y,
                );
                // a 1 border in the card between cells, as the web's
                let one = egui::Rect::from_min_size(min, cell).shrink(0.5);
                painter.rect_filled(one, corner, at((*value / high) as f32));
            }
            for (row, name) in self.y.iter().enumerate() {
                let shown_row = rows - 1 - row;
                let middle = grid.top() + (shown_row as f32 + 0.5) * cell.y;
                painter.text(
                    pos2(grid.left() - 8.0, middle),
                    egui::Align2::RIGHT_CENTER,
                    name,
                    font.clone(),
                    ink,
                );
            }
            // a time every so many columns, about one per 80
            let every = ((80.0 / cell.x).ceil() as usize).max(1);
            for (column, name) in self.x.iter().enumerate().step_by(every) {
                let center = grid.left() + (column as f32 + 0.5) * cell.x;
                painter.text(
                    pos2(center, grid.bottom() + 4.0),
                    egui::Align2::CENTER_TOP,
                    name,
                    font.clone(),
                    ink,
                );
            }
            // the scale: a bar 8 wide and 100 tall, high at its top, its ends written beside it
            let bar = egui::Rect::from_min_size(
                pos2(rect.right() - 44.0 + 4.0, rect.top() + 4.0 + 14.0),
                vec2(8.0, 100.0),
            );
            let mut mesh = egui::Mesh::default();
            let steps = 24;
            for i in 0..=steps {
                let share = i as f32 / steps as f32;
                let y = bar.bottom() - share * bar.height();
                let color = at(share);
                mesh.colored_vertex(pos2(bar.left(), y), color);
                mesh.colored_vertex(pos2(bar.right(), y), color);
            }
            for i in 0..steps {
                let k = (i * 2) as u32;
                mesh.add_triangle(k, k + 1, k + 2);
                mesh.add_triangle(k + 1, k + 3, k + 2);
            }
            painter.add(egui::Shape::mesh(mesh));
            painter.text(
                pos2(bar.center().x, bar.top() - 2.0),
                egui::Align2::CENTER_BOTTOM,
                written(high),
                font.clone(),
                ink,
            );
            painter.text(
                pos2(bar.center().x, bar.bottom() + 2.0),
                egui::Align2::CENTER_TOP,
                written(0.0),
                font.clone(),
                ink,
            );
        });
    }
}

/// A dashboard panel of bars, as sokkel-ui's `BarPanel`: one or more series per category, side
/// by side or stacked, upright or lying down, on a 192 tall body.
#[must_use = "draw it with `show`"]
pub struct BarPanel {
    title: String,
    subtitle: Option<String>,
    chart: BarChart,
    series: Vec<Series>,
    legend: bool,
}

impl BarPanel {
    pub fn new(title: impl Into<String>, x: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            chart: BarChart::new(x).height(192.0),
            series: Vec::new(),
            legend: true,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn series(mut self, series: Series) -> Self {
        self.series.push(Series {
            values: Vec::new(),
            ..series.clone()
        });
        self.chart = self.chart.series(series);
        self
    }

    pub fn stacked(mut self, stacked: bool) -> Self {
        self.chart = self.chart.stacked(stacked);
        self
    }

    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.chart = self.chart.horizontal(horizontal);
        self
    }

    /// The legend under the bars; shown for more than one series unless turned off.
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let (chart, names, legend_on) = (self.chart, self.series, self.legend);
        chart_frame(ui, &self.title, self.subtitle.as_deref(), |ui| {
            ui.add(chart);
            if legend_on && names.len() > 1 {
                ui.add_space(4.0);
                let t = tokens(ui.ctx());
                let items: Vec<(&str, Color32)> = names
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (s.name.as_str(), color_of(&t, s, i)))
                    .collect();
                legend_of(ui, &items);
            }
        });
    }
}
