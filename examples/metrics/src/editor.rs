//! Editing one panel, or making a new one, as the web's `Editor.tsx`: the options beside a live
//! preview that moves with the source, saved back to the dashboard.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Field, FormBar, Gap, Icon, IconButton, NumberField, PageHeader, PanelSpan,
    Section, Segmented, Select, SplitPane, Stack, SwatchGroup, SwitchField, TableState, TextField, Threshold, Toast,
    UnsavedChangesWords, unsaved_changes,
};

use crate::MetricsApp;
use crate::model::{ColorMode, Fill, KINDS, Panel, PanelKind};
use crate::query::{PRESETS, REDUCES, Range, bounds, span_of};
use crate::source::{Aggregation, Label, METRICS, MetricId, PROJECTS, SERVICES, labels_of, metric_of};

/// The units a panel can write its figures in, as the web's `UNITS`: `None` is the metric's own.
const UNITS: [(Option<&str>, &str); 7] = [
    (None, "The metric’s own"),
    (Some(""), "None"),
    (Some("%"), "Percent (%)"),
    (Some(" ms"), "Milliseconds (ms)"),
    (Some(" s"), "Seconds (s)"),
    (Some(" req/s"), "Requests per second"),
    (Some(" GB"), "Gigabytes (GB)"),
];

const DECIMALS: [(Option<usize>, &str); 5] = [
    (None, "Auto"),
    (Some(0), "0"),
    (Some(1), "1"),
    (Some(2), "2"),
    (Some(3), "3"),
];

const SPANS: [(PanelSpan, &str); 6] = [
    (PanelSpan::Sixth, "1/6"),
    (PanelSpan::Quarter, "1/4"),
    (PanelSpan::Third, "1/3"),
    (PanelSpan::Half, "1/2"),
    (PanelSpan::TwoThirds, "2/3"),
    (PanelSpan::Full, "Full"),
];

/// The colors a panel can be drawn in, as theme tokens, as the web's `COLORS`.
pub const COLORS: [(&str, &str); 9] = [
    ("chart-1", "Chart 1"),
    ("chart-2", "Chart 2"),
    ("chart-3", "Chart 3"),
    ("chart-4", "Chart 4"),
    ("chart-5", "Chart 5"),
    ("good", "Good"),
    ("info", "Info"),
    ("warning", "Warning"),
    ("critical", "Critical"),
];

fn aggregation_label(aggregation: Aggregation) -> &'static str {
    match aggregation {
        Aggregation::Sum => "Sum",
        Aggregation::Avg => "Mean",
        Aggregation::Max => "Max",
        Aggregation::Min => "Min",
    }
}

fn group_label(group: Option<Label>) -> &'static str {
    match group {
        None => "None",
        Some(Label::Service) => "Service",
        Some(Label::Host) => "Host",
        Some(Label::Project) => "Project",
        Some(Label::Job) => "Job",
    }
}

/// A panel as it starts in the editor: requests over time, summed by service, as the web's `newPanel`.
pub fn new_panel() -> Panel {
    let mut panel = Panel::new("New panel", PanelKind::TimeSeries, MetricId::Requests);
    panel.aggregation = Aggregation::Sum;
    panel.group_by = Some(Label::Service);
    panel
}

/// What the preview reads, in a sentence, as the web's `readsLine`.
pub fn reads_line(panel: &Panel, range: &Range, at: f64) -> String {
    if panel.kind == PanelKind::Logs {
        return "The newest lines the hosts wrote in the range, under the variables and the panel’s filter.".into();
    }
    let metric = metric_of(panel.metric);
    let span = span_of(range, at, 150.0);
    let (start, end) = bounds(range, at);
    let range = match range {
        Range::Preset(id) => PRESETS
            .iter()
            .find(|p| p.id == *id)
            .map(|p| p.label.to_lowercase())
            .unwrap_or_default(),
        Range::Between { .. } => format!("{} minutes", ((end - start) / 60.0).round()),
    };
    let group = match panel.group_by {
        Some(label) if !matches!(panel.kind, PanelKind::Stat | PanelKind::Gauge) => format!(" by {}", label.key()),
        _ => String::new(),
    };
    // a name in capitals, such as CPU, keeps them
    let mut chars = metric.label.chars();
    let name = match (chars.next(), chars.next()) {
        (Some(a), Some(b)) if a.is_uppercase() && b.is_lowercase() => metric.label.to_lowercase(),
        _ => metric.label.to_owned(),
    };
    format!(
        "Reads {name}, {}{group}, over the {range} in {} steps of {}s.",
        aggregation_label(panel.aggregation).to_lowercase(),
        span.times.len(),
        span.step
    )
}

/// A panel being edited or made.
pub struct EditorState {
    /// The dashboard it is saved to, by its id.
    pub board: String,
    /// The panel as it was, or `None` for a new one.
    pub original: Option<Panel>,
    pub draft: Panel,
    /// Leaving with changes is being asked about.
    pub(crate) leaving: bool,
    pub(crate) table: TableState,
}

impl EditorState {
    pub fn dirty(&self) -> bool {
        self.original.as_ref() != Some(&self.draft)
    }
}

impl MetricsApp {
    /// Opens the editor on a panel of the board shown, or on a new one.
    pub(crate) fn open_editor(&mut self, panel: Option<&Panel>) {
        let draft = match panel {
            Some(panel) => panel.clone(),
            None => {
                let mut made = new_panel();
                made.id = self.panel_id();
                made
            }
        };
        self.editor = Some(EditorState {
            board: self.place.clone(),
            original: panel.cloned(),
            draft,
            leaving: false,
            table: TableState::default(),
        });
    }

    /// The editor's page: its head, the preview beside the options, and the form bar.
    pub(crate) fn editor_page(&mut self, ui: &mut Ui) {
        let Some(mut editor) = self.editor.take() else {
            return;
        };
        let title = match &editor.original {
            None => "New panel".to_owned(),
            Some(original) => format!("Edit {}", original.title),
        };
        let header = PageHeader::new(title).back("Back").show(ui, |_| {});
        // leaving with changes asks first; a new panel always counts as changed
        let mut close = false;
        if header.back {
            if editor.dirty() {
                editor.leaving = true;
            } else {
                close = true;
            }
        }
        let bar = FormBar::height(ui.ctx());
        let gap = ui.spacing().item_spacing.y;
        let room = (ui.available_height() - bar - gap).max(240.0);
        let (variables, range, at) = (self.variables.clone(), self.range.clone(), self.view_at);
        // the preview draws the draft as it stood when the frame began
        let shown = editor.draft.clone();
        ui.allocate_ui(egui::vec2(ui.available_width(), room), |ui| {
            ui.set_min_height(room);
            SplitPane::new("metrics-editor")
                .default_share(0.6)
                .min_share(0.35)
                .max_share(0.75)
                .scroll(true)
                .show(
                    ui,
                    |ui| {
                        egui::Frame::new().inner_margin(4).show(ui, |ui| {
                            Stack::new(Gap::Md).show(ui, |ui| {
                                sokkel_egui::pane_toolbar(ui, |_| {}, |ui| self.time_controls(ui));
                                crate::app::draw_panel_as(ui, &shown, &variables, &range, at, &mut editor.table, true);
                                let t = sokkel_egui::tokens(ui.ctx());
                                ui.label(
                                    sokkel_egui::TextRole::Help
                                        .rich(&t, reads_line(&shown, &range, at))
                                        .color(t.color("muted-foreground")),
                                );
                            });
                        });
                    },
                    |ui| {
                        egui::Frame::new()
                            .inner_margin(4)
                            .show(ui, |ui| options(ui, &mut editor.draft));
                    },
                );
        });
        let new = editor.original.is_none();
        let dirty = editor.dirty();
        let title = editor.draft.title.trim().to_owned();
        let note = if new {
            "A new panel, not yet on the dashboard"
        } else if dirty {
            "Unsaved changes"
        } else {
            "No changes"
        };
        let mut save = false;
        let mut discard = false;
        FormBar::new().note(note).show(ui, |ui| {
            let mut button = Button::new(if new { "Add to dashboard" } else { "Save" }).size(ButtonSize::Sm);
            if title.is_empty() {
                button = button.reason("A panel needs a title.");
            } else if !dirty {
                button = button.reason("Nothing has changed yet.");
            }
            if ui.add(button).clicked() && !title.is_empty() && dirty {
                save = true;
            }
            if ui
                .add(
                    Button::new("Discard")
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm),
                )
                .clicked()
            {
                discard = true;
            }
        });
        if discard {
            if dirty {
                editor.leaving = true;
            } else {
                close = true;
            }
        }
        if save {
            self.save_editor(&editor);
            return;
        }
        if !close {
            self.editor = Some(editor);
        }
    }

    /// Saves the draft to its dashboard: in its place, or at the end for a new one.
    fn save_editor(&mut self, editor: &EditorState) {
        let Some(board) = self.dashboards.iter_mut().find(|b| b.id == editor.board) else {
            return;
        };
        let mut panel = editor.draft.clone();
        panel.title = panel.title.trim().to_owned();
        match board.panels.iter_mut().find(|p| p.id == panel.id) {
            Some(kept) => *kept = panel.clone(),
            None => board.panels.push(panel.clone()),
        }
        self.said
            .push(Toast::success(format!("{} saved to {}", panel.title, board.title)));
        self.panels.focus = Some(panel.id);
    }

    /// The question before leaving the editor with changes.
    pub(crate) fn editor_dialogs(&mut self, ctx: &egui::Context) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        if !editor.leaving {
            return;
        }
        match unsaved_changes(ctx, "leave-editor", &UnsavedChangesWords::leave()) {
            Some(true) => self.editor = None,
            Some(false) => editor.leaving = false,
            None => {}
        }
    }
}

/// The editor's options, as the web's: the panel, its query, how it is drawn, and its thresholds.
fn options(ui: &mut Ui, draft: &mut Panel) {
    let t = sokkel_egui::tokens(ui.ctx());
    let kind = draft.kind;
    let one = matches!(kind, PanelKind::Stat | PanelKind::Gauge);
    let logs = kind == PanelKind::Logs;
    let metric = metric_of(draft.metric);
    let unit = draft.unit.clone().unwrap_or_else(|| metric.unit.to_owned());
    Stack::new(Gap::Lg).show(ui, |ui| {
        Section::new("Panel").show(ui, |ui| {
            Stack::new(Gap::Md).show(ui, |ui| {
                let missing = draft.title.trim().is_empty();
                ui.add(TextField::new("Title", &mut draft.title).error(missing.then_some("A panel needs a title.")));
                Field::new("Visualization").show(ui, |ui| {
                    let mut at = KINDS.iter().position(|(k, _)| *k == kind).unwrap_or(0);
                    if ui
                        .add(Segmented::new(&mut at, KINDS.iter().map(|(_, l)| *l)).fill(true))
                        .changed()
                    {
                        draft.kind = KINDS[at].0;
                    }
                });
                Field::new("Width")
                    .description("Of the dashboard’s width on a wide screen; panels stack on a phone.")
                    .show(ui, |ui| {
                        let mut at = SPANS.iter().position(|(s, _)| *s == draft.span).unwrap_or(3);
                        if ui
                            .add(Segmented::new(&mut at, SPANS.iter().map(|(_, l)| *l)).fill(true))
                            .changed()
                        {
                            draft.span = SPANS[at].0;
                        }
                    });
            });
        });
        Section::new("Query").show(ui, |ui| {
            Stack::new(Gap::Md).show(ui, |ui| {
                if !logs {
                    Field::new("Metric").help(metric.description).show(ui, |ui| {
                        let mut at = METRICS.iter().position(|m| m.id == draft.metric).unwrap_or(0);
                        let width = ui.available_width();
                        if ui
                            .add(Select::new(&mut at, METRICS.iter().map(|m| m.label), "Metric").width(width))
                            .changed()
                        {
                            set_metric(draft, METRICS[at].id);
                        }
                    });
                    let metric = metric_of(draft.metric);
                    Field::new("Aggregation")
                        .description("How the series in a group combine at each step.")
                        .show(ui, |ui| {
                            let mut at = metric
                                .aggregations
                                .iter()
                                .position(|a| *a == draft.aggregation)
                                .unwrap_or(0);
                            let labels = metric.aggregations.iter().map(|a| aggregation_label(*a));
                            if ui.add(Segmented::new(&mut at, labels).fill(true)).changed() {
                                draft.aggregation = metric.aggregations[at];
                            }
                        });
                    if !one {
                        Field::new("Group by")
                            .description("One series for each, or one for all.")
                            .show(ui, |ui| {
                                let groups: Vec<Option<Label>> = std::iter::once(None)
                                    .chain(labels_of(draft.metric).into_iter().map(Some))
                                    .collect();
                                let mut at = groups.iter().position(|g| *g == draft.group_by).unwrap_or(0);
                                let labels = groups.iter().map(|g| group_label(*g));
                                if ui.add(Segmented::new(&mut at, labels).fill(true)).changed() {
                                    draft.group_by = groups[at];
                                }
                            });
                    }
                }
                if one || kind == PanelKind::BarGauge {
                    Field::new("Value")
                        .description("How each series over the range reads as one value.")
                        .show(ui, |ui| {
                            let mut at = REDUCES.iter().position(|(r, _)| *r == draft.reduce).unwrap_or(0);
                            if ui
                                .add(Segmented::new(&mut at, REDUCES.iter().map(|(_, l)| *l)).fill(true))
                                .changed()
                            {
                                draft.reduce = REDUCES[at].0;
                            }
                        });
                }
                // the service and the project the panel always reads, side by side
                let half = (ui.available_width() - 8.0) / 2.0;
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for (label, name, any, values) in [
                        (Label::Service, "Service", "Any service", &SERVICES),
                        (Label::Project, "Project", "Any project", &PROJECTS),
                    ] {
                        ui.allocate_ui(egui::vec2(half, 60.0), |ui| {
                            Field::new(name).show(ui, |ui| {
                                let options: Vec<&str> = std::iter::once(any).chain(values.iter().copied()).collect();
                                let current = draft.filter.iter().find(|(l, _)| *l == label).map(|(_, v)| v.as_str());
                                let mut at = current.and_then(|v| options.iter().position(|o| *o == v)).unwrap_or(0);
                                if ui
                                    .add(Select::new(&mut at, options.iter().copied(), name).width(half))
                                    .changed()
                                {
                                    draft.filter.retain(|(l, _)| *l != label);
                                    if at > 0 {
                                        draft.filter.push((label, options[at].to_owned()));
                                    }
                                }
                            });
                        });
                    }
                });
            });
        });
        if logs {
            return;
        }
        Section::new("Display").show(ui, |ui| {
            Stack::new(Gap::Md).show(ui, |ui| {
                Field::new("Unit").show(ui, |ui| {
                    let mut at = UNITS.iter().position(|(u, _)| *u == draft.unit.as_deref()).unwrap_or(0);
                    let width = ui.available_width();
                    if ui
                        .add(Select::new(&mut at, UNITS.iter().map(|(_, l)| *l), "Unit").width(width))
                        .changed()
                    {
                        draft.unit = UNITS[at].0.map(str::to_owned);
                    }
                });
                Field::new("Decimals").show(ui, |ui| {
                    let mut at = DECIMALS.iter().position(|(d, _)| *d == draft.decimals).unwrap_or(0);
                    if ui
                        .add(Segmented::new(&mut at, DECIMALS.iter().map(|(_, l)| *l)).fill(true))
                        .changed()
                    {
                        draft.decimals = DECIMALS[at].0;
                    }
                });
                if kind == PanelKind::TimeSeries {
                    Field::new("Fill").show(ui, |ui| {
                        let fills = [
                            (Fill::None, "Lines"),
                            (Fill::Gradient, "Fading"),
                            (Fill::Solid, "Solid"),
                        ];
                        let mut at = fills.iter().position(|(f, _)| *f == draft.fill).unwrap_or(1);
                        if ui
                            .add(Segmented::new(&mut at, fills.iter().map(|(_, l)| *l)).fill(true))
                            .changed()
                        {
                            draft.fill = fills[at].0;
                        }
                    });
                }
                if matches!(kind, PanelKind::TimeSeries | PanelKind::Bar) {
                    ui.add(SwitchField::new(&mut draft.stacked, "Stack the series"));
                }
                if kind == PanelKind::Stat {
                    Field::new("Threshold color on").show(ui, |ui| {
                        let modes = [
                            (ColorMode::Value, "The value"),
                            (ColorMode::Background, "The tile"),
                            (ColorMode::None, "Neither"),
                        ];
                        let mut at = modes.iter().position(|(m, _)| *m == draft.color_mode).unwrap_or(0);
                        if ui
                            .add(Segmented::new(&mut at, modes.iter().map(|(_, l)| *l)).fill(true))
                            .changed()
                        {
                            draft.color_mode = modes[at].0;
                        }
                    });
                }
                if matches!(kind, PanelKind::Gauge | PanelKind::BarGauge) {
                    let half = (ui.available_width() - 8.0) / 2.0;
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.add(
                            NumberField::new("panel-min", "Min", &mut draft.min)
                                .suffix(unit.trim())
                                .width(half),
                        );
                        ui.add(
                            NumberField::new("panel-max", "Max", &mut draft.max)
                                .suffix(unit.trim())
                                .width(half),
                        );
                    });
                }
                let grouped = draft.group_by.is_some() && !one && kind != PanelKind::BarGauge;
                Field::new("Color")
                    .description(if grouped {
                        "Grouped series take the theme’s colors in turn; this one draws a single series."
                    } else {
                        "The series, the tile or the dial where no threshold colors it."
                    })
                    .show(ui, |ui| {
                        let swatches: Vec<_> = COLORS
                            .iter()
                            .map(|(token, label)| (t.color(token), (*label).to_owned()))
                            .collect();
                        let mut at = COLORS.iter().position(|(c, _)| *c == draft.color).unwrap_or(1);
                        if ui.add(SwatchGroup::new(&mut at, &swatches)).changed() {
                            draft.color = COLORS[at].0;
                        }
                    });
            });
        });
        let about = match kind {
            PanelKind::TimeSeries => "Drawn as dashed lines across the chart.",
            PanelKind::Heatmap => "Their colors make the heatmap’s scale.",
            _ => "Color the value by the step it reaches.",
        };
        Section::new("Thresholds")
            .description(about)
            .show(ui, |ui| thresholds(ui, &mut draft.thresholds, &unit));
    });
}

/// A new metric keeps the aggregation and the grouping where the metric has them, as the web's `setMetric`.
fn set_metric(draft: &mut Panel, id: MetricId) {
    let next = metric_of(id);
    draft.metric = id;
    if !next.aggregations.contains(&draft.aggregation) {
        draft.aggregation = next.aggregations.first().copied().unwrap_or(Aggregation::Avg);
    }
    if let Some(group) = draft.group_by
        && !labels_of(id).contains(&group)
    {
        draft.group_by = None;
    }
}

/// The thresholds' rows, as the web's `ThresholdsInput`: the base color for everything below the
/// first step, then each step from its value up.
fn thresholds(ui: &mut Ui, steps: &mut Vec<Threshold>, unit: &str) {
    let t = sokkel_egui::tokens(ui.ctx());
    let dots: Vec<_> = COLORS.iter().map(|(token, _)| t.color(token)).collect();
    let mut remove = None;
    Stack::new(Gap::Sm).show(ui, |ui| {
        for (index, step) in steps.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let mut at = COLORS.iter().position(|(c, _)| *c == step.color).unwrap_or(0);
                let label = format!("Color of step {}", index + 1);
                if ui
                    .add(
                        Select::new(&mut at, COLORS.iter().map(|(_, l)| *l), label)
                            .dots(dots.iter().copied())
                            .width(144.0),
                    )
                    .changed()
                {
                    step.color = COLORS[at].0;
                }
                let end = 28.0 + 8.0;
                let width = (ui.available_width() - end).max(80.0);
                if index == 0 {
                    ui.allocate_ui(egui::vec2(width, 28.0), |ui| {
                        ui.set_min_width(width);
                        ui.label(
                            sokkel_egui::TextRole::Body
                                .rich(&t, "Base, below every step")
                                .color(t.color("muted-foreground")),
                        );
                    });
                } else {
                    let mut value = step.value.unwrap_or(0.0);
                    let field = NumberField::new(
                        ("threshold-value", index),
                        format!("From, step {}", index + 1),
                        &mut value,
                    )
                    .suffix(unit.trim())
                    .width(width);
                    if ui.add(field).changed() {
                        step.value = Some(value);
                    }
                }
                let label = if index == 0 {
                    "Remove every step".to_owned()
                } else {
                    format!("Remove step {}", index + 1)
                };
                if ui.add(IconButton::new(Icon::Trash2, label)).clicked() {
                    remove = Some(index);
                }
            });
        }
        let add = if steps.is_empty() { "Add thresholds" } else { "Add step" };
        if ui
            .add(
                Button::new(add)
                    .icon(Icon::Plus)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm),
            )
            .clicked()
        {
            if steps.is_empty() {
                steps.push(Threshold {
                    value: None,
                    color: "good",
                });
                steps.push(Threshold {
                    value: Some(80.0),
                    color: "critical",
                });
            } else {
                let last = steps.last().and_then(|s| s.value).unwrap_or(0.0);
                steps.push(Threshold {
                    value: Some(last + 10.0),
                    color: "critical",
                });
            }
        }
    });
    match remove {
        Some(0) => steps.clear(),
        Some(index) => {
            steps.remove(index);
        }
        None => {}
    }
}
