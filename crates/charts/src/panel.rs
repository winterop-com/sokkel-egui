//! A dashboard's chart panels, after sokkel-ui's `TimeSeriesPanel` in its `ChartFrame`.

use egui::{Color32, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};
use sokkel_ui_egui::{TextRole, cell_height, report_natural_height, tokens};

use crate::{Fill, LineChart, Series, legend};

/// A dashboard panel of values over time: lines over fading areas, dashed threshold lines, and a
/// legend under it, in a chart's frame (a card with the plain border, its title and subtitle in a
/// 32 tall head, the chart 192 tall under it). Laid in a `DashboardGrid`, it stretches to its row.
#[must_use = "draw it with `show`"]
pub struct TimeSeriesPanel {
    title: String,
    subtitle: Option<String>,
    chart: LineChart,
    names: Vec<String>,
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
            names: Vec::new(),
            legend: true,
            height: 192.0,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn series(mut self, series: Series) -> Self {
        self.names.push(series.name.clone());
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
                let title = WidgetText::from(TextRole::Help.rich(&t, &self.title).variation("wght", 500.0))
                    .into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        inner.max(1.0),
                        TextRole::Help.font(&t),
                    );
                let subtitle = self.subtitle.as_ref().map(|s| {
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
                sokkel_ui_egui::panel_tools(&mut tools, cell);
                // the body: 8 in and under, the chart, the legend 4 under it
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: 0,
                        bottom: 8,
                    })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add(self.chart.height(self.height));
                        if self.legend && !self.names.is_empty() {
                            ui.add_space(4.0);
                            let names: Vec<&str> = self.names.iter().map(String::as_str).collect();
                            legend(ui, &names);
                        }
                    });
                let used = ui.min_rect().height() + 2.0;
                report_natural_height(ui, cell, used);
                if stretch > used {
                    ui.add_space(stretch - used);
                }
            });
    }
}
