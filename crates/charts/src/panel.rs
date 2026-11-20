//! A dashboard's chart panels, after sokkel-ui's `TimeSeriesPanel` in its `ChartFrame`.

use egui::{Color32, Rect, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};
use sokkel_egui::{
    Cell, Column, DataTable, TableState, TableVariant, TextRole, Threshold, cell_height, format_value,
    report_natural_height, threshold_color, tokens,
};

use crate::{BarChart, Fill, Gauge, LineChart, Series, color_of, legend_of};

/// A figure a legend's table works out from each series' values, as sokkel-ui's `LegendStat`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendStat {
    Min,
    Max,
    Avg,
    Current,
}

impl LegendStat {
    /// The figure from `values`, the missing ones passed over; `None` when there are none.
    pub fn of(self, values: &[Option<f64>]) -> Option<f64> {
        let numbers: Vec<f64> = values.iter().flatten().copied().filter(|v| v.is_finite()).collect();
        let last = *numbers.last()?;
        Some(match self {
            LegendStat::Min => numbers.iter().copied().fold(f64::INFINITY, f64::min),
            LegendStat::Max => numbers.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            LegendStat::Avg => numbers.iter().sum::<f64>() / numbers.len() as f64,
            LegendStat::Current => last,
        })
    }
}

/// The words a legend's table says, English by default.
#[derive(Clone, Debug)]
pub struct LegendWords {
    pub series: String,
    pub min: String,
    pub max: String,
    pub avg: String,
    pub current: String,
}

impl Default for LegendWords {
    fn default() -> Self {
        Self {
            series: "Series".into(),
            min: "Min".into(),
            max: "Max".into(),
            avg: "Avg".into(),
            current: "Current".into(),
        }
    }
}

impl LegendWords {
    fn of(&self, stat: LegendStat) -> &str {
        match stat {
            LegendStat::Min => &self.min,
            LegendStat::Max => &self.max,
            LegendStat::Avg => &self.avg,
            LegendStat::Current => &self.current,
        }
    }
}

/// More entries than this and a legend's table scrolls, 160 at most, as the web's `SHORT_LEGEND`.
const SHORT_LEGEND: usize = 6;

/// A legend as a table, as sokkel-ui's `ChartLegend` in `mode="table"`: each series a row of a
/// listed `Table`, its swatch and name, then `stats` worked out from its values and written with
/// `write`, in the mono face at the end of their columns.
fn legend_table(
    ui: &mut Ui,
    entries: &[(String, Color32, Vec<Option<f64>>)],
    stats: &[LegendStat],
    words: &LegendWords,
    write: impl Fn(f64) -> String,
) {
    let t = tokens(ui.ctx());
    let mono = sokkel_egui::mono_font(
        ui.ctx(),
        sokkel_egui::mono_small_size(&t, TextRole::Help.font(&t).size),
        400.0,
    );
    let rows: Vec<Vec<Cell>> = entries
        .iter()
        .map(|(name, color, values)| {
            let mut row = vec![Cell::Swatch {
                text: name.clone(),
                color: *color,
            }];
            row.extend(
                stats
                    .iter()
                    .map(|stat| Cell::Mono(stat.of(values).map_or_else(|| "-".to_owned(), &write))),
            );
            row
        })
        .collect();
    // a figure's column as wide as its widest figure or head, 64 at least with the room between
    // columns, as `minmax(4rem,auto)`
    let width_of = |text: &str, font: egui::FontId| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font, Color32::WHITE)
            .size()
            .x
    };
    let mut columns = vec![Column::new(&words.series, 116.0).fill().sortable(false)];
    for (index, stat) in stats.iter().enumerate() {
        let head = width_of(words.of(*stat), TextRole::Head.font(&t));
        let widest = rows
            .iter()
            .map(|row| match &row[index + 1] {
                Cell::Mono(text) => width_of(text, mono.clone()),
                _ => 0.0,
            })
            .fold(head, f32::max);
        columns.push(
            Column::new(words.of(*stat), (widest + 12.0).max(52.0))
                .numeric()
                .sortable(false),
        );
    }
    let id = ui.scope_id().with("legend-table");
    let mut state: TableState = ui.data_mut(|d| d.get_temp_mut_or_default::<TableState>(id).clone());
    let mut table = DataTable::new(id, &columns, &rows, &mut state)
        .variant(TableVariant::List)
        .framed(false)
        .bar(false)
        .footer(false);
    if entries.len() > SHORT_LEGEND {
        table = table.height(160.0);
    }
    table.show(ui);
    ui.data_mut(|d| d.insert_temp(id, state));
}

/// A dashboard panel of values over time: lines over fading areas, dashed threshold lines, and a
/// legend under it, in a chart's frame (a card with the plain border, its title and subtitle in a
/// 32 tall head, the chart 192 tall under it). Laid in a `DashboardGrid`, it stretches to its row.
#[must_use = "draw it with `show`"]
pub struct TimeSeriesPanel {
    title: String,
    subtitle: Option<String>,
    info: Option<String>,
    empty: bool,
    words: PanelWords,
    chart: LineChart,
    series: Vec<Series>,
    legend: bool,
    table: Option<(Vec<LegendStat>, LegendWords)>,
    height: f32,
}

impl TimeSeriesPanel {
    /// A panel titled `title` over the times in `x`, written for a reader.
    pub fn new(title: impl Into<String>, x: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            info: None,
            empty: false,
            words: PanelWords::default(),
            chart: LineChart::new(x).fill(Fill::Gradient),
            series: Vec::new(),
            legend: true,
            table: None,
            height: 192.0,
        }
    }

    /// The legend as a table of `stats` worked out from each series' values, as the web's
    /// `legend="table"` with its `legendStats`.
    pub fn legend_table(mut self, stats: &[LegendStat]) -> Self {
        self.table = Some((stats.to_vec(), LegendWords::default()));
        self
    }

    /// The words of the legend's table.
    pub fn legend_words(mut self, words: LegendWords) -> Self {
        if let Some((_, held)) = &mut self.table {
            *held = words;
        }
        self
    }

    /// What the panel shows, behind the Info mark beside its title.
    pub fn info(mut self, info: impl Into<String>) -> Self {
        self.info = Some(info.into());
        self
    }

    /// Nothing to draw, as a segment with no figures: the body says so instead.
    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Writes each figure under the pointer, and the axis' ticks short, as the web's `format`.
    pub fn format(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.chart = self.chart.format(std::sync::Arc::new(format));
        self
    }

    pub fn words(mut self, words: PanelWords) -> Self {
        self.words = words;
        self
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series.clone());
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

    /// A target: a dashed line across at `value` in `color`, the axis reaching it, its value
    /// written at its end, as a goal's line.
    pub fn mark(mut self, value: f64, color: Color32) -> Self {
        self.chart = self.chart.mark(value, color);
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
        self.show_with(ui, None::<fn(&mut Ui)>);
    }

    /// Draws the panel with `actions` at its head's end, as the web's `actions`: a time range.
    pub fn show_with(self, ui: &mut Ui, actions: Option<impl FnOnce(&mut Ui)>) {
        let actions: Option<HeadActions<'_>> = actions.map(|a| Box::new(a) as HeadActions<'_>);
        let (chart, height, legend_on, names, table) = (self.chart, self.height, self.legend, self.series, self.table);
        let head = Head {
            subtitle: self.subtitle.as_deref(),
            info: self.info.as_deref(),
            empty: self.empty.then_some(self.words.empty.as_str()),
            ..Head::new(&self.title)
        };
        chart_frame_with(ui, head, actions, |ui| {
            let writing = chart.writing.clone();
            let height = fill_height(ui, height);
            ui.add(chart.height(height));
            let under = ui.min_rect().bottom();
            if let (true, Some((stats, words))) = (legend_on && !names.is_empty(), &table) {
                ui.add_space(4.0);
                let t = tokens(ui.ctx());
                let entries: Vec<(String, Color32, Vec<Option<f64>>)> = names
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (s.name.clone(), color_of(&t, s, i), s.values.clone()))
                    .collect();
                legend_table(ui, &entries, stats, words, |v| writing.value(v));
            } else if legend_on && !names.is_empty() {
                ui.add_space(4.0);
                let t = tokens(ui.ctx());
                let items: Vec<(&str, Color32)> = names
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (s.name.as_str(), color_of(&t, s, i)))
                    .collect();
                legend_of(ui, &items);
            }
            drawn_under(ui, under);
        });
    }
}

/// A chart's height: `height`, or in its panel's window all the window has less what was drawn
/// under the chart the last time (its legend).
pub(crate) fn fill_height(ui: &Ui, height: f32) -> f32 {
    if !sokkel_egui::windowed(ui) {
        return height;
    }
    let under = ui
        .data(|d| d.get_temp::<f32>(ui.scope_id().with("under-chart")))
        .unwrap_or(0.0);
    (ui.available_height() - under).max(height)
}

/// Keeps how much was drawn under a chart since `top`, for [`fill_height`].
pub(crate) fn drawn_under(ui: &Ui, top: f32) {
    let under = ui.min_rect().bottom() - top;
    ui.data_mut(|d| d.insert_temp(ui.scope_id().with("under-chart"), under));
}

/// A chart's frame, as sokkel-ui's `ChartFrame`: a card with the plain border, its title and
/// subtitle in a 32 tall head with an arrangement's tools at its end, the body 8 in and under. In
/// a `DashboardGrid` it stretches to its row.
/// What a chart frame's head says, and whether its body has anything to draw.
#[derive(Clone, Copy, Default)]
pub(crate) struct Head<'a> {
    pub(crate) title: &'a str,
    pub(crate) subtitle: Option<&'a str>,
    /// What the panel shows, behind the Info mark beside the title.
    pub(crate) info: Option<&'a str>,
    /// Nothing to draw: the body says so instead, as the web's `empty`.
    pub(crate) empty: Option<&'a str>,
}

impl<'a> Head<'a> {
    pub(crate) fn new(title: &'a str) -> Self {
        Self {
            title,
            ..Default::default()
        }
    }
}

/// The words a chart panel says, English by default.
#[derive(Clone, Debug)]
pub struct PanelWords {
    /// What the Info mark is called for a screen reader.
    pub info: String,
    /// Said in the body of a panel with nothing to draw.
    pub empty: String,
}

impl Default for PanelWords {
    fn default() -> Self {
        Self {
            info: "About this panel".into(),
            empty: "No data for this range.".into(),
        }
    }
}

pub(crate) fn chart_frame(ui: &mut Ui, head_of: Head<'_>, body: impl FnOnce(&mut Ui)) {
    chart_frame_with(ui, head_of, None, body);
}

/// What a chart's head draws at its end, before an arrangement's tools.
type HeadActions<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

fn chart_frame_with(ui: &mut Ui, head_of: Head<'_>, actions: Option<HeadActions<'_>>, body: impl FnOnce(&mut Ui)) {
    let (title, subtitle) = (head_of.title, head_of.subtitle);
    // in its window the chart alone fills what the window has
    if sokkel_egui::windowed(ui) {
        body(ui);
        return;
    }
    let t = tokens(ui.ctx());
    let width = ui.available_width();
    let stretch = cell_height(ui).unwrap_or(0.0);
    let cell = ui.scope_id();
    let ink = t.color("muted-foreground");
    egui::Frame::new()
        .fill(t.color("card"))
        .stroke(Stroke::new(1.0, t.color("border")))
        .corner_radius(t.radius("surface"))
        .show(ui, |ui| {
            ui.set_width(width - 2.0);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            // the head: 32 tall, 12 in, the title and subtitle 6 apart
            let (head, _) = ui.allocate_exact_size(vec2(width - 2.0, 32.0), Sense::hover());
            // the tools at the head's end first, shown while the pointer is over the frame, so
            // the title gives way to them
            let over = ui
                .data(|d| d.get_temp::<egui::Rect>(cell.with("frame-rect")))
                .is_some_and(|rect| ui.rect_contains_pointer(rect));
            sokkel_egui::quiet_tools(ui, cell, !over);
            let mut tools = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("chart-panel-tools")
                    .max_rect(head.shrink2(vec2(6.0, 0.0)))
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            sokkel_egui::panel_tools(&mut tools, cell);
            if let Some(actions) = actions {
                tools.spacing_mut().item_spacing.x = 4.0;
                actions(&mut tools);
            }
            let taken = tools.min_rect().width();
            let inner = width - 2.0 - 18.0 - if taken > 1.0 { taken + 4.0 } else { 0.0 };
            let title = WidgetText::from(TextRole::Help.rich(&t, title).variation("wght", 500.0)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                inner.max(1.0),
                TextRole::Help.font(&t),
            );
            // the title, then the Info mark 20 wide, then the subtitle, each 6 apart
            let info_room = if head_of.info.is_some() { 26.0 } else { 0.0 };
            let subtitle = subtitle.map(|s| {
                WidgetText::from(TextRole::Help.rich(&t, s)).into_galley(
                    ui,
                    Some(TextWrapMode::Truncate),
                    (inner - title.size().x - 6.0 - info_room).max(1.0),
                    TextRole::Help.font(&t),
                )
            });
            let top = head.center().y - title.size().y / 2.0;
            let title_width = title.size().x;
            ui.painter()
                .galley(pos2(head.left() + 12.0, top), title, t.color("card-foreground"));
            if let Some(info) = head_of.info {
                let center = pos2(head.left() + 12.0 + title_width + 6.0 + 10.0, head.center().y);
                sokkel_egui::info_tip(ui, center, info, &PanelWords::default().info);
            }
            if let Some(subtitle) = subtitle {
                ui.painter().galley(
                    pos2(head.left() + 12.0 + title_width + 6.0 + info_room, top),
                    subtitle,
                    ink,
                );
            }
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 8,
                    top: 0,
                    bottom: 8,
                })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if let Some(empty) = head_of.empty {
                        // 16 in and 32 over and under, centered, as the web's empty body
                        ui.add_space(24.0);
                        ui.vertical_centered(|ui| {
                            ui.label(TextRole::Body.rich(&t, empty).color(ink));
                        });
                        ui.add_space(32.0);
                    } else if !sokkel_egui::panel_away(ui, cell) {
                        let top = ui.cursor().top();
                        // the room a taller row gives past the panel's own height, for a body that
                        // grows into it
                        let natural: f32 = ui.data(|d| d.get_temp(cell.with("natural"))).unwrap_or(0.0);
                        let room = if natural > 0.0 {
                            (stretch - natural).max(0.0)
                        } else {
                            0.0
                        };
                        ui.data_mut(|d| {
                            d.insert_temp(grow_id(), room);
                            d.insert_temp(grew_id(), 0.0_f32);
                        });
                        body(ui);
                        sokkel_egui::body_drawn(ui, cell, ui.min_rect().bottom() - top);
                    }
                });
            let used = ui.min_rect().height() + 2.0;
            let frame = ui.min_rect().expand(1.0);
            ui.data_mut(|d| d.insert_temp(cell.with("frame-rect"), frame));
            let grew: f32 = ui.data_mut(|d| d.remove_temp(grew_id())).unwrap_or(0.0);
            ui.data_mut(|d| {
                d.remove::<f32>(grow_id());
                d.insert_temp(cell.with("natural"), used - grew);
            });
            report_natural_height(ui, cell, used - grew);
            if stretch > used {
                ui.add_space(stretch - used);
            }
        });
}

fn grow_id() -> egui::Id {
    egui::Id::unique("sokkel-chart-grow")
}

fn grew_id() -> egui::Id {
    egui::Id::unique("sokkel-chart-grew")
}

/// The room a chart's body may grow into where its panel stands stretched in a taller row; the
/// body takes it all, and its row is not sized by it.
fn grow_room(ui: &Ui) -> f32 {
    let room: f32 = ui.data(|d| d.get_temp(grow_id())).unwrap_or(0.0);
    ui.data_mut(|d| d.insert_temp(grew_id(), room));
    room
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
    height: f32,
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
            height: 128.0,
        }
    }

    /// The dial's height, 128 when not given, as the web's `bodyClassName` sets it.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
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
        chart_frame(ui, Head::new(&self.title), |ui| {
            let height = fill_height(ui, self.height);
            ui.add(gauge.height(height));
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
        let mono = sokkel_egui::mono_font(
            ui.ctx(),
            sokkel_egui::mono_small_size(&t, TextRole::Help.font(&t).size),
            400.0,
        );
        let value_font = sokkel_egui::mono_font(
            ui.ctx(),
            sokkel_egui::mono_size(&t, TextRole::Help.font(&t).size),
            500.0,
        );
        chart_frame(ui, Head::new(&self.title), |ui| {
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
                // the name cut short with an ellipsis past its column of 96, as the web's `truncate`
                let mut job = egui::text::LayoutJob::single_section(
                    name.clone(),
                    egui::TextFormat::simple(mono.clone(), t.color("muted-foreground")),
                );
                job.wrap = egui::text::TextWrapping::truncate_at_width(96.0);
                let name_galley = painter.layout_job(job);
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
    subtitle: Option<String>,
    info: Option<String>,
    empty: bool,
    words: PanelWords,
    height: f32,
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
            subtitle: None,
            info: None,
            empty: false,
            words: PanelWords::default(),
            height: 192.0,
            x: x.into_iter().map(Into::into).collect(),
            y: y.into_iter().map(Into::into).collect(),
            cells: cells.into_iter().collect(),
            scale: vec!["warning"],
            unit: String::new(),
            decimals: None,
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

    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// The body's height, 192 when not given.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
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
        let head = Head {
            subtitle: self.subtitle.as_deref(),
            info: self.info.as_deref(),
            empty: self.empty.then_some(self.words.empty.as_str()),
            ..Head::new(&self.title)
        };
        chart_frame(ui, head, |ui| {
            let width = ui.available_width();
            let height = fill_height(ui, self.height);
            let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
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
    info: Option<String>,
    empty: bool,
    words: PanelWords,
    chart: BarChart,
    series: Vec<Series>,
    legend: bool,
    height: f32,
}

impl BarPanel {
    pub fn new(title: impl Into<String>, x: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            info: None,
            empty: false,
            words: PanelWords::default(),
            height: 192.0,
            chart: BarChart::new(x).height(192.0),
            series: Vec::new(),
            legend: true,
        }
    }

    /// What the panel shows, behind the Info mark beside its title.
    pub fn info(mut self, info: impl Into<String>) -> Self {
        self.info = Some(info.into());
        self
    }

    /// Nothing to draw, as a segment with no figures: the body says so instead.
    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Writes each figure under the pointer, and the axis' ticks short, as the web's `format`.
    pub fn format(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.chart = self.chart.format(std::sync::Arc::new(format));
        self
    }

    pub fn words(mut self, words: PanelWords) -> Self {
        self.words = words;
        self
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

    /// The chart's height, 192 (`h-48`) when not given.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// The legend under the bars; shown for more than one series unless turned off.
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let (chart, names, legend_on, height) = (self.chart, self.series, self.legend, self.height);
        let head = Head {
            subtitle: self.subtitle.as_deref(),
            info: self.info.as_deref(),
            empty: self.empty.then_some(self.words.empty.as_str()),
            ..Head::new(&self.title)
        };
        chart_frame(ui, head, |ui| {
            let height = fill_height(ui, height);
            ui.add(chart.height(height));
            let under = ui.min_rect().bottom();
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
            drawn_under(ui, under);
        });
    }
}

/// A dashboard panel of stages narrowing by their figure, as sokkel-ui's `ChartPanel` of the
/// `funnel` kind: each stage as wide as its figure against the largest (8% at least), 2 apart, a
/// trapezoid to the next stage's width, the last a band of its own; each written inside with its
/// figure and its share of the first, the stages fading from the first series color toward the
/// card. On a 256 tall body, as the web's `h-64`.
#[must_use = "draw it with `show`"]
pub struct FunnelPanel {
    title: String,
    subtitle: Option<String>,
    info: Option<String>,
    empty: bool,
    words: PanelWords,
    stages: Vec<(String, f64)>,
    format: Option<crate::Format>,
    height: f32,
}

impl FunnelPanel {
    pub fn new(title: impl Into<String>, stages: impl IntoIterator<Item = (String, f64)>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            info: None,
            empty: false,
            words: PanelWords::default(),
            stages: stages.into_iter().collect(),
            format: None,
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

    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Writes each stage's figure; grouped thousands when not given.
    pub fn format(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Some(std::sync::Arc::new(format));
        self
    }

    pub fn words(mut self, words: PanelWords) -> Self {
        self.words = words;
        self
    }

    /// The body's height, 256 when not given.
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
        let (stages, format, height) = (&self.stages, self.format.clone(), self.height);
        chart_frame(ui, head, |ui| {
            let height = fill_height(ui, height);
            let t = tokens(ui.ctx());
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let area = rect.shrink(8.0);
            let count = stages.len();
            if count == 0 {
                return;
            }
            let gap = 2.0;
            let tall = (area.height() - gap * (count - 1) as f32) / count as f32;
            let most = stages.iter().map(|(_, v)| *v).fold(1.0, f64::max);
            let width_of = |value: f64| area.width() * (0.08 + 0.92 * (value / most).clamp(0.0, 1.0) as f32);
            let first = stages[0].1;
            let share = |value: f64| {
                let percent = if first == 0.0 { 0.0 } else { value / first * 100.0 };
                if percent >= 10.0 || percent == 0.0 {
                    format!("{}%", percent.round() as i64)
                } else {
                    format!("{percent:.1}%")
                }
            };
            let written = |value: f64| match &format {
                Some(f) => f(value),
                None => sokkel_egui::format_value(value, None),
            };
            let base = t.color(crate::SERIES_ORDER[0]);
            let card = t.color("card");
            let painter = ui.painter();
            let font = TextRole::Help.font(&t);
            let mut hovered = None;
            for (index, (name, value)) in stages.iter().enumerate() {
                let top = area.top() + index as f32 * (tall + gap);
                let bottom = top + tall;
                let upper = width_of(*value);
                let lower = stages.get(index + 1).map_or(upper, |(_, next)| width_of(*next));
                let middle = area.center().x;
                let shape = vec![
                    pos2(middle - upper / 2.0, top),
                    pos2(middle + upper / 2.0, top),
                    pos2(middle + lower / 2.0, bottom),
                    pos2(middle - lower / 2.0, bottom),
                ];
                // the first stage 80% of the color over the card, the last 35%
                let strength = if count <= 1 {
                    0.80
                } else {
                    (80.0 - 45.0 * index as f32 / (count - 1) as f32).round() / 100.0
                };
                let fill = card.lerp_to_gamma(base, strength);
                let over = response.hover_pos().is_some_and(|p| {
                    p.y >= top && p.y < bottom + gap && (p.x - middle).abs() <= upper.max(lower) / 2.0
                });
                if over {
                    hovered = Some(index);
                }
                painter.add(egui::Shape::convex_polygon(shape, fill, Stroke::new(1.0, card)));
                let mut label = TextRole::Help.rich(&t, format!("{name}  {} · {}", written(*value), share(*value)));
                if over {
                    label = label.variation("wght", 600.0);
                }
                let galley =
                    WidgetText::from(label).into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, font.clone());
                painter.galley(
                    pos2(
                        middle - galley.size().x / 2.0,
                        (top + bottom) / 2.0 - galley.size().y / 2.0,
                    ),
                    galley,
                    t.color("foreground"),
                );
            }
            if let Some(index) = hovered {
                let (name, value) = &stages[index];
                let strength = if count <= 1 {
                    0.80
                } else {
                    (80.0 - 45.0 * index as f32 / (count - 1) as f32).round() / 100.0
                };
                let color = card.lerp_to_gamma(base, strength);
                let rows = vec![(color, name.clone(), Some(*value))];
                let writing = crate::Writing {
                    format: format.clone(),
                    ..Default::default()
                };
                crate::tooltip(response.clone(), t.clone(), name, rows, &writing);
            }
        });
    }
}

/// A dashboard panel of a ring of slices, each category's share of the whole, as sokkel-ui's
/// `ChartPanel` of the `donut` kind: the ring from 58% to 82% of the radius, the slices 1 degree
/// apart and edged in the card, from the top clockwise, the total written in its middle over its
/// word, and a legend under it. On a 256 tall body, as the web's `h-64`.
#[must_use = "draw it with `show`"]
pub struct DonutPanel {
    title: String,
    subtitle: Option<String>,
    info: Option<String>,
    empty: bool,
    words: PanelWords,
    slices: Vec<(String, f64, &'static str)>,
    total: Option<String>,
    format: Option<crate::Format>,
    height: f32,
}

impl DonutPanel {
    /// Each slice's name, value and color token.
    pub fn new(title: impl Into<String>, slices: impl IntoIterator<Item = (String, f64, &'static str)>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            info: None,
            empty: false,
            words: PanelWords::default(),
            slices: slices.into_iter().collect(),
            total: None,
            format: None,
            height: 256.0,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn info(mut self, info: impl Into<String>) -> Self {
        self.info = Some(info.into());
        self
    }

    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// The total written in the ring's middle, over this word.
    pub fn total(mut self, word: impl Into<String>) -> Self {
        self.total = Some(word.into());
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Some(std::sync::Arc::new(format));
        self
    }

    pub fn words(mut self, words: PanelWords) -> Self {
        self.words = words;
        self
    }

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
        let (slices, total_word, format, height) = (&self.slices, &self.total, self.format.clone(), self.height);
        chart_frame(ui, head, |ui| {
            let height = fill_height(ui, height);
            let t = tokens(ui.ctx());
            // the legend stands inside the body, under the ring, as the web's chart holds its legend
            let items: Vec<(&str, Color32)> = slices.iter().map(|(n, _, k)| (n.as_str(), t.color(k))).collect();
            let legend = crate::legend_height(ui, &items, ui.available_width()) + 4.0;
            let ring = (height - legend).max(height / 2.0) + grow_room(ui);
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ring), Sense::hover());
            let center = rect.center();
            let radius = rect.width().min(rect.height()) / 2.0;
            let (inner, outer) = (radius * 0.58, radius * 0.82);
            let total: f64 = slices.iter().map(|s| s.1.max(0.0)).sum();
            let written = |v: f64| match &format {
                Some(f) => f(v),
                None => sokkel_egui::format_value(v, None),
            };
            let painter = ui.painter();
            let shapes: Vec<crate::kinds::Slice> = slices
                .iter()
                .map(|(_, value, token)| crate::kinds::Slice {
                    color: t.color(token),
                    value: *value,
                })
                .collect();
            let hovered = crate::kinds::draw_slices(
                painter,
                t.color("card"),
                center,
                (inner, outer),
                &shapes,
                2.0,
                response.hover_pos(),
            )
            .map(|index| (index, shapes[index].color));
            if let Some(word) = total_word {
                let figure = WidgetText::from(
                    TextRole::Statistic
                        .rich(&t, written(total))
                        .size(t.length("chart-stat-size"))
                        .variation("wght", 600.0),
                )
                .into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextRole::Statistic.font(&t),
                );
                let below = WidgetText::from(TextRole::Help.rich(&t, word)).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextRole::Help.font(&t),
                );
                let tall = figure.size().y + 2.0 + below.size().y;
                let top = center.y - tall / 2.0;
                let (fw, bw) = (figure.size().x, below.size().x);
                painter.galley(pos2(center.x - fw / 2.0, top), figure, t.color("foreground"));
                painter.galley(
                    pos2(center.x - bw / 2.0, top + tall - below.size().y),
                    below,
                    t.color("muted-foreground"),
                );
            }
            if let Some((index, color)) = hovered {
                let (name, value, _) = &slices[index];
                let share = if total == 0.0 { 0.0 } else { value / total * 100.0 };
                let writing = crate::Writing {
                    format: Some(std::sync::Arc::new({
                        let format = format.clone();
                        move |v: f64| {
                            let text = match &format {
                                Some(f) => f(v),
                                None => sokkel_egui::format_value(v, None),
                            };
                            format!("{text} ({share:.1}%)")
                        }
                    })),
                    ..Default::default()
                };
                crate::tooltip(
                    response.clone(),
                    t.clone(),
                    "",
                    vec![(color, name.clone(), Some(*value))],
                    &writing,
                );
            }
            ui.add_space(4.0);
            legend_of(ui, &items);
            drawn_under(ui, rect.bottom());
        });
    }
}

/// A dashboard panel of a running total, as sokkel-ui's `ChartPanel` of the `waterfall` kind: each
/// step a bar rising or falling from where the one before ended, joined by a dotted line, and the
/// total's bar at the end; rises in `good`, falls in `critical` and the total in `chart-2`. On a
/// 256 tall body, as the web's `h-64`.
#[must_use = "draw it with `show`"]
pub struct WaterfallPanel {
    title: String,
    subtitle: Option<String>,
    info: Option<String>,
    empty: bool,
    words: PanelWords,
    steps: Vec<(String, f64)>,
    total: String,
    format: Option<crate::Format>,
    height: f32,
}

impl WaterfallPanel {
    pub fn new(title: impl Into<String>, steps: impl IntoIterator<Item = (String, f64)>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            info: None,
            empty: false,
            words: PanelWords::default(),
            steps: steps.into_iter().collect(),
            total: "Total".into(),
            format: None,
            height: 256.0,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn info(mut self, info: impl Into<String>) -> Self {
        self.info = Some(info.into());
        self
    }

    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// The total's bar's name, `Total` when not given.
    pub fn total(mut self, word: impl Into<String>) -> Self {
        self.total = word.into();
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Some(std::sync::Arc::new(format));
        self
    }

    pub fn words(mut self, words: PanelWords) -> Self {
        self.words = words;
        self
    }

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
        let (steps, total_word, format, height) = (&self.steps, &self.total, self.format.clone(), self.height);
        chart_frame(ui, head, |ui| {
            let height = fill_height(ui, height);
            let t = tokens(ui.ctx());
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            // each bar from where the one before ended, then the total from 0
            let mut bars: Vec<(String, f64, f64, f64)> = Vec::new();
            let mut level = 0.0;
            for (name, change) in steps {
                bars.push((name.clone(), level, level + change, *change));
                level += change;
            }
            bars.push((total_word.clone(), 0.0, level, level));
            let names: Vec<String> = bars.iter().map(|b| b.0.clone()).collect();
            let max = bars.iter().map(|b| b.1.max(b.2)).fold(0.0, f64::max);
            let writing = crate::Writing {
                format: format.clone(),
                ..Default::default()
            };
            let axes = crate::Axes {
                writing: writing.clone(),
                ..Default::default()
            };
            let frame = crate::Frame::draw(ui, rect, &t, &names, max, &axes);
            let band = frame.band(names.len());
            let width = (band * 0.62).min(40.0);
            let painter = ui.painter();
            let last = bars.len() - 1;
            let hovered = frame.hovered(&response, names.len());
            for (index, (_, from, to, change)) in bars.iter().enumerate() {
                let x = frame.x(index);
                let top = frame.y(from.max(*to));
                let bottom = frame.y(from.min(*to));
                let token = if index == last {
                    "chart-2"
                } else if *change >= 0.0 {
                    "good"
                } else {
                    "critical"
                };
                let bar =
                    egui::Rect::from_min_max(pos2(x - width / 2.0, top), pos2(x + width / 2.0, bottom.max(top + 1.0)));
                painter.rect_filled(bar, egui::CornerRadius::same(2), t.color(token));
                if index < last {
                    let y = frame.y(*to);
                    painter.extend(egui::Shape::dashed_line(
                        &[pos2(x + width / 2.0, y), pos2(x + band - width / 2.0, y)],
                        Stroke::new(1.0, t.color("muted-foreground")),
                        2.0,
                        2.0,
                    ));
                }
            }
            if let Some(i) = hovered {
                let (name, _, to, change) = &bars[i];
                let rows = if i == last {
                    vec![(t.color("chart-2"), total_word.clone(), Some(*to))]
                } else {
                    let token = if *change >= 0.0 { "good" } else { "critical" };
                    vec![
                        (t.color(token), "Change".to_owned(), Some(*change)),
                        (t.color("muted-foreground"), total_word.clone(), Some(*to)),
                    ]
                };
                crate::tooltip(response.clone(), t.clone(), name, rows, &writing);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::LegendStat;

    #[test]
    fn a_legend_works_out_its_figures_past_the_gaps() {
        let values = [Some(4.0), None, Some(1.0), Some(7.0), None];
        assert_eq!(LegendStat::Min.of(&values), Some(1.0));
        assert_eq!(LegendStat::Max.of(&values), Some(7.0));
        assert_eq!(LegendStat::Avg.of(&values), Some(4.0));
        assert_eq!(LegendStat::Current.of(&values), Some(7.0));
        assert_eq!(LegendStat::Avg.of(&[None]), None);
    }
}

/// A spread of values: the furthest within one and a half box heights, the quartiles and the
/// median, the values beyond, and how many there were, as the web's `spreadOf`.
#[derive(Clone, Debug, PartialEq)]
pub struct Spread {
    pub low: f64,
    pub lower: f64,
    pub median: f64,
    pub upper: f64,
    pub high: f64,
    pub outliers: Vec<f64>,
    pub count: usize,
}

/// The value at `at` (0 to 1) along sorted values, between two neighbours linearly, as the web's
/// `quantile`.
pub fn quantile(sorted: &[f64], at: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let index = (sorted.len() - 1) as f64 * at;
    let (low, high) = (index.floor() as usize, index.ceil() as usize);
    Some(sorted[low] + (sorted[high] - sorted[low]) * (index - low as f64))
}

/// The spread of `values`, or `None` for none, as the web's `spreadOf`.
pub fn spread_of(values: &[f64]) -> Option<Spread> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let lower = quantile(&sorted, 0.25)?;
    let median = quantile(&sorted, 0.5)?;
    let upper = quantile(&sorted, 0.75)?;
    let reach = (upper - lower) * 1.5;
    let inside: Vec<f64> = sorted
        .iter()
        .copied()
        .filter(|v| *v >= lower - reach && *v <= upper + reach)
        .collect();
    Some(Spread {
        low: inside[0],
        lower,
        median,
        upper,
        high: inside[inside.len() - 1],
        outliers: sorted
            .iter()
            .copied()
            .filter(|v| *v < lower - reach || *v > upper + reach)
            .collect(),
        count: sorted.len(),
    })
}

/// A dashboard panel of each category's spread, as sokkel-ui's `ChartPanel` of the `boxplot` kind:
/// a box from the lower to the upper quartile (the first series color at 22% inside its 1.25 edge)
/// with the median across it, whiskers out to the furthest values within one and a half box
/// heights, the values beyond as hollow points, on a value axis fitted to the values. On a 192
/// tall body, as the web's `h-48`.
pub struct BoxPlotPanel {
    title: String,
    subtitle: Option<String>,
    info: Option<String>,
    empty: bool,
    words: PanelWords,
    groups: Vec<(String, Vec<f64>)>,
    unit: String,
    height: f32,
}

impl BoxPlotPanel {
    /// Each category's name and its values, in order.
    pub fn new(title: impl Into<String>, groups: impl IntoIterator<Item = (String, Vec<f64>)>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            info: None,
            empty: false,
            words: PanelWords::default(),
            groups: groups.into_iter().collect(),
            unit: String::new(),
            height: 192.0,
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

    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Written after each tick, as `%`.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let head = Head {
            subtitle: self.subtitle.as_deref(),
            info: self.info.as_deref(),
            empty: self.empty.then_some(self.words.empty.as_str()),
            ..Head::new(&self.title)
        };
        let (groups, unit, height) = (self.groups, self.unit, self.height);
        chart_frame(ui, head, |ui| {
            let t = tokens(ui.ctx());
            let height = fill_height(ui, height);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
            let spreads: Vec<Option<Spread>> = groups.iter().map(|(_, v)| spread_of(v)).collect();
            let values = spreads
                .iter()
                .flatten()
                .flat_map(|s| s.outliers.iter().copied().chain([s.low, s.high]));
            let (mut least, mut most) = (f64::INFINITY, f64::NEG_INFINITY);
            for v in values {
                least = least.min(v);
                most = most.max(v);
            }
            if !least.is_finite() {
                return;
            }
            // a value axis fitted to the values, as `scale: true`
            let step = crate::nice_step((most - least).max(1.0), 5);
            let bottom = (least / step).floor() * step;
            let top = (most / step).ceil() * step;
            let font = crate::axis_font(&t);
            let muted = t.color("muted-foreground");
            let painter = ui.painter_at(rect);
            let tick_words: Vec<String> = {
                let mut out = Vec::new();
                let mut v = bottom;
                while v <= top + step / 2.0 {
                    out.push(format!("{}{unit}", crate::format_tick(v)));
                    v += step;
                }
                out
            };
            let gutter = tick_words
                .iter()
                .map(|w| painter.layout_no_wrap(w.clone(), font.clone(), muted).size().x)
                .fold(0.0_f32, f32::max)
                + 8.0
                + 4.0;
            let label_room = 20.0;
            let plot = Rect::from_min_max(
                rect.min + egui::vec2(gutter, 10.0),
                rect.max - egui::vec2(10.0, label_room + 4.0),
            );
            let y_of =
                |v: f64| plot.bottom() - ((v - bottom) / (top - bottom).max(f64::MIN_POSITIVE)) as f32 * plot.height();
            for (index, words) in tick_words.iter().enumerate() {
                let v = bottom + step * index as f64;
                let y = y_of(v);
                painter.hline(plot.x_range(), y, Stroke::new(1.0, t.color("border")));
                painter.text(
                    egui::pos2(plot.left() - 8.0, y),
                    egui::Align2::RIGHT_CENTER,
                    words,
                    font.clone(),
                    muted,
                );
            }
            let color = crate::series_color(&t, 0);
            let card = t.color("card");
            let mix = |a: u8, b: u8| (a as f32 * 0.22 + b as f32 * 0.78).round() as u8;
            let fill = Color32::from_rgb(
                mix(color.r(), card.r()),
                mix(color.g(), card.g()),
                mix(color.b(), card.b()),
            );
            let band = plot.width() / groups.len().max(1) as f32;
            let box_width = (band * 0.5).clamp(8.0, 40.0);
            let mut last_label = f32::NEG_INFINITY;
            for (index, ((name, _), spread)) in groups.iter().zip(&spreads).enumerate() {
                let x = plot.left() + band * (index as f32 + 0.5);
                // a label where it clears the one before it
                let galley = painter.layout_no_wrap(name.clone(), font.clone(), muted);
                if x - galley.size().x / 2.0 > last_label + 8.0 {
                    painter.galley(
                        egui::pos2(x - galley.size().x / 2.0, plot.bottom() + 4.0),
                        galley.clone(),
                        muted,
                    );
                    last_label = x + galley.size().x / 2.0;
                }
                let Some(s) = spread else { continue };
                let edge = Stroke::new(1.25, color);
                painter.vline(x, y_of(s.upper)..=y_of(s.high), edge);
                painter.vline(x, y_of(s.low)..=y_of(s.lower), edge);
                painter.hline(x - box_width / 2.0..=x + box_width / 2.0, y_of(s.high), edge);
                painter.hline(x - box_width / 2.0..=x + box_width / 2.0, y_of(s.low), edge);
                let body = Rect::from_min_max(
                    egui::pos2(x - box_width / 2.0, y_of(s.upper)),
                    egui::pos2(x + box_width / 2.0, y_of(s.lower)),
                );
                painter.rect(body, 0.0, fill, edge, egui::StrokeKind::Middle);
                painter.hline(body.x_range(), y_of(s.median), edge);
                for out in &s.outliers {
                    painter.circle(egui::pos2(x, y_of(*out)), 2.5, t.color("card"), edge);
                }
            }
        });
    }
}
