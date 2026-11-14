//! Choosing: Checkbox, Tabs and Segmented, after sokkel-ui's components of the same names.

use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Widget, WidgetInfo, WidgetText,
    WidgetType, pos2, vec2,
};

use super::{dim, focus_ring};
use crate::{Icon, TextRole, icons, tokens};

/// A 16px checkbox, with its label beside it when given; `indeterminate` shows a dash for a set
/// partly chosen.
#[must_use = "add it with `ui.add(...)`"]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: String,
    show_label: bool,
    indeterminate: bool,
    enabled: bool,
}

impl<'a> Checkbox<'a> {
    /// A checkbox with its label beside it.
    pub fn new(checked: &'a mut bool, label: impl Into<String>) -> Self {
        Self {
            checked,
            label: label.into(),
            show_label: true,
            indeterminate: false,
            enabled: true,
        }
    }

    /// The label only names the box for assistive technology, as in a table's row.
    pub fn hide_label(mut self) -> Self {
        self.show_label = false;
        self
    }

    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let galley = self.show_label.then(|| {
            WidgetText::from(TextRole::Body.rich(&t, &self.label)).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextRole::Body.font(&t),
            )
        });
        let text_width = galley.as_ref().map_or(0.0, |g| g.size().x + 8.0);
        let height = galley.as_ref().map_or(16.0, |g| g.size().y.max(16.0));
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let (rect, mut response) = ui.allocate_exact_size(vec2(16.0 + text_width, height), sense);
        if response.clicked() {
            *self.checked = !*self.checked;
            response.mark_changed();
        }
        let on = *self.checked || self.indeterminate;
        response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, self.enabled, on, &self.label));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let mark = Rect::from_min_size(pos2(rect.left(), rect.center().y - 8.0), vec2(16.0, 16.0));
        let radius = t.radius("sm");
        let (fill, edge) = if on {
            (t.color("primary"), t.color("primary"))
        } else if t.dark {
            (t.color_at("input", 0.3), t.color("input"))
        } else {
            (Color32::TRANSPARENT, t.color("input"))
        };
        painter.rect(
            mark,
            radius,
            dim(fill, self.enabled),
            Stroke::new(1.0, dim(edge, self.enabled)),
            StrokeKind::Inside,
        );
        if on {
            let glyph = if self.indeterminate { Icon::Minus } else { Icon::Check };
            icons::paint(
                painter,
                mark.center(),
                glyph,
                14.0,
                dim(t.color("primary-foreground"), self.enabled),
            );
        }
        focus_ring(painter, &t, mark, radius, &response);
        if let Some(galley) = galley {
            let ink = dim(t.color("foreground"), self.enabled);
            painter.galley(
                pos2(mark.right() + 8.0, rect.center().y - galley.size().y / 2.0),
                galley,
                ink,
            );
        }
        response
    }
}

/// How a tab list looks: a muted track with the chosen tab raised, or a line under the chosen tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabsVariant {
    #[default]
    Default,
    Line,
}

/// A tab list: one of `tabs` chosen, by index.
#[must_use = "add it with `ui.add(...)`"]
pub struct Tabs<'a> {
    chosen: &'a mut usize,
    tabs: Vec<String>,
    variant: TabsVariant,
}

impl<'a> Tabs<'a> {
    pub fn new(chosen: &'a mut usize, tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            chosen,
            tabs: tabs.into_iter().map(Into::into).collect(),
            variant: TabsVariant::Default,
        }
    }

    pub fn variant(mut self, variant: TabsVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Widget for Tabs<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let role = TextRole::Badge;
        let line = self.variant == TabsVariant::Line;
        let galleys: Vec<_> = self
            .tabs
            .iter()
            .map(|tab| {
                WidgetText::from(role.rich(&t, tab)).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    role.font(&t),
                )
            })
            .collect();
        let (inset, gap) = if line { (0.0, 4.0) } else { (3.0, 0.0) };
        let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 14.0).collect();
        let width = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32 + 2.0 * inset;
        let (rect, mut response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
        if !line {
            ui.painter().rect_filled(rect, t.radius("control"), t.color("muted"));
        }
        let mut x = rect.left() + inset;
        for (index, (galley, tab_width)) in galleys.into_iter().zip(widths).enumerate() {
            let tab = Rect::from_min_size(
                pos2(x, rect.top() + inset),
                vec2(tab_width, rect.height() - 2.0 * inset - 1.0),
            );
            let id = response.id.with(index);
            let hit = ui.interact(tab, id, Sense::click());
            let chosen = *self.chosen == index;
            hit.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, chosen, &self.tabs[index]));
            if hit.clicked() && !chosen {
                *self.chosen = index;
                response.mark_changed();
            }
            let painter = ui.painter();
            if chosen && !line {
                let (ground, edge) = if t.dark {
                    (t.color_at("input", 0.3), t.color("input"))
                } else {
                    (t.color("background"), Color32::TRANSPARENT)
                };
                painter.rect(tab, t.radius("md"), ground, Stroke::new(1.0, edge), StrokeKind::Inside);
            }
            if chosen && line {
                let y = rect.bottom() + 4.0;
                painter.line_segment(
                    [pos2(tab.left(), y), pos2(tab.right(), y)],
                    Stroke::new(2.0, t.color("foreground")),
                );
            }
            let ink = if chosen || hit.hovered() {
                t.color("foreground")
            } else if t.dark {
                t.color("muted-foreground")
            } else {
                t.color_at("foreground", 0.6)
            };
            painter.galley(tab.center() - galley.size() / 2.0, galley, ink);
            focus_ring(painter, &t, tab, t.radius("md"), &hit);
            x += tab_width + gap;
        }
        response
    }
}

/// How a segmented control looks, as sokkel-ui's `Segmented` `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SegmentedVariant {
    /// The options on a muted track, the chosen one raised on the card ground.
    #[default]
    Track,
    /// Loose options, the chosen one washed in the primary color.
    Tint,
    /// Options in an outlined row, divided, the chosen one on the secondary ground.
    Outline,
    /// Options in an outlined row, the chosen one filled with the primary color.
    Solid,
}

/// One choice among a few options, all in view.
#[must_use = "add it with `ui.add(...)`"]
pub struct Segmented<'a> {
    chosen: &'a mut usize,
    options: Vec<String>,
    variant: SegmentedVariant,
    counts: Vec<Option<usize>>,
    dots: Vec<Option<Color32>>,
}

impl<'a> Segmented<'a> {
    pub fn new(chosen: &'a mut usize, options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            chosen,
            options: options.into_iter().map(Into::into).collect(),
            variant: SegmentedVariant::Track,
            counts: Vec::new(),
            dots: Vec::new(),
        }
    }

    /// How many each option holds, after its label, for options that filter a list.
    pub fn counts(mut self, counts: impl IntoIterator<Item = Option<usize>>) -> Self {
        self.counts = counts.into_iter().collect();
        self
    }

    /// A small dot before each option's label in the color of the state it filters to.
    pub fn dots(mut self, dots: impl IntoIterator<Item = Option<Color32>>) -> Self {
        self.dots = dots.into_iter().collect();
        self
    }

    pub fn variant(mut self, variant: SegmentedVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Widget for Segmented<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let variant = self.variant;
        let galley = |ui: &Ui, option: &str, role: TextRole| {
            WidgetText::from(role.rich(&t, option)).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                role.font(&t),
            )
        };
        // the chosen option at medium weight, the others at regular
        let galleys: Vec<_> = self
            .options
            .iter()
            .enumerate()
            .map(|(index, option)| {
                let role = if index == *self.chosen {
                    TextRole::Badge
                } else {
                    TextRole::Help
                };
                galley(ui, option, role)
            })
            .collect();
        // every option as wide as the widest label at medium weight, as the web lays each label
        // over the others
        // an option's face: its dot (6, then 6), its label, and its count (6, then the count in
        // the code face at 70 percent)
        let count_galley = |ui: &Ui, index: usize| {
            self.counts.get(index).copied().flatten().map(|n| {
                let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
                WidgetText::from(egui::RichText::new(n.to_string()).font(mono.clone())).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    mono,
                )
            })
        };
        let face = |ui: &Ui, index: usize, label: f32| {
            let dot = if self.dots.get(index).copied().flatten().is_some() {
                12.0
            } else {
                0.0
            };
            dot + label + count_galley(ui, index).map_or(0.0, |c| 6.0 + c.size().x)
        };
        let widest = self
            .options
            .iter()
            .enumerate()
            .map(|(index, option)| face(ui, index, galley(ui, option, TextRole::Badge).size().x))
            .fold(0.0, f32::max);
        let widths: Vec<f32> = galleys.iter().map(|_| widest + 20.0).collect();
        // the frame's edge and padding, the gap between options, and an option's height
        let (inset, gap, height) = match variant {
            SegmentedVariant::Track => (2.0, 0.0, 24.0),
            SegmentedVariant::Tint => (0.0, 2.0, 28.0),
            SegmentedVariant::Outline | SegmentedVariant::Solid => (1.0, 0.0, 26.0),
        };
        // each option a finger tall on a phone, where it is pressed
        let height = if super::is_phone(ui.ctx()) {
            t.length("spacing-finger")
        } else {
            height
        };
        let width = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32 + 2.0 * inset;
        let (rect, mut response) = ui.allocate_exact_size(vec2(width, height + 2.0 * inset), Sense::hover());
        let control = t.radius("control");
        let painter = ui.painter().clone();
        match variant {
            SegmentedVariant::Track => {
                let (ground, edge) = if t.dark {
                    (t.color("background"), t.color("border-strong"))
                } else {
                    (t.color("muted"), t.color("border"))
                };
                painter.rect(rect, control, ground, Stroke::new(1.0, edge), StrokeKind::Inside);
            }
            SegmentedVariant::Outline => {
                painter.rect_stroke(
                    rect,
                    control,
                    Stroke::new(1.0, t.color("border-strong")),
                    StrokeKind::Inside,
                );
            }
            SegmentedVariant::Solid => {
                painter.rect_stroke(rect, control, Stroke::new(1.0, t.color("border")), StrokeKind::Inside);
            }
            SegmentedVariant::Tint => {}
        }
        let count = galleys.len();
        let mut x = rect.left() + inset;
        for (index, (galley, option_width)) in galleys.into_iter().zip(widths).enumerate() {
            let option = Rect::from_min_size(pos2(x, rect.top() + inset), vec2(option_width, height));
            let hit = ui.interact(option, response.id.with(index), Sense::click());
            let chosen = *self.chosen == index;
            hit.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, chosen, &self.options[index]));
            if hit.clicked() && !chosen {
                *self.chosen = index;
                response.mark_changed();
            }
            // the outer options of an outlined row keep the frame's corners on their outer side
            let edge_radius = |first: bool, last: bool| {
                let r = control.nw.saturating_sub(1);
                CornerRadius {
                    nw: if first { r } else { 0 },
                    sw: if first { r } else { 0 },
                    ne: if last { r } else { 0 },
                    se: if last { r } else { 0 },
                }
            };
            let radius = match variant {
                SegmentedVariant::Track => t.radius("mini"),
                SegmentedVariant::Tint => control,
                _ => edge_radius(index == 0, index + 1 == count),
            };
            let (ground, ink) = match (variant, chosen) {
                (SegmentedVariant::Track, true) => (
                    if t.dark {
                        t.color_at("input", 0.7)
                    } else {
                        t.color("card")
                    },
                    t.color("foreground"),
                ),
                (SegmentedVariant::Tint, true) => (t.color_at("primary", 0.15), t.color("primary-ink")),
                (SegmentedVariant::Outline, true) => (t.color("secondary"), t.color("foreground")),
                (SegmentedVariant::Solid, true) => (t.color("primary"), t.color("primary-foreground")),
                (_, false) if hit.hovered() => (t.color("hover"), t.color("foreground")),
                (_, false) => (Color32::TRANSPARENT, t.color("muted-foreground")),
            };
            if chosen && variant == SegmentedVariant::Track {
                let ring = if t.dark {
                    t.color("input")
                } else {
                    t.color("border-strong")
                };
                painter.rect(option, radius, ground, Stroke::new(1.0, ring), StrokeKind::Inside);
            } else {
                painter.rect_filled(option, radius, ground);
            }
            if variant == SegmentedVariant::Outline && index > 0 {
                painter.vline(
                    option.left(),
                    option.y_range(),
                    Stroke::new(1.0, t.color("border-strong")),
                );
            }
            // the face centered: the dot, the label, the count
            let mut fx = option.center().x - face(ui, index, galley.size().x) / 2.0;
            if let Some(dot) = self.dots.get(index).copied().flatten() {
                painter.circle_filled(pos2(fx + 3.0, option.center().y), 3.0, dot);
                fx += 12.0;
            }
            let label_width = galley.size().x;
            painter.galley(pos2(fx, option.center().y - galley.size().y / 2.0), galley, ink);
            fx += label_width;
            if let Some(count) = count_galley(ui, index) {
                painter.galley(
                    pos2(fx + 6.0, option.center().y - count.size().y / 2.0),
                    count,
                    ink.gamma_multiply(0.7),
                );
            }
            focus_ring(&painter, &t, option, radius, &hit);
            x += option_width + gap;
        }
        response
    }
}

/// One choice of a `ChoiceList`: its value, its words and a line under them.
pub struct Choice<'a> {
    pub value: &'a str,
    pub label: &'a str,
    pub description: Option<&'a str>,
}

/// One answer out of a few, each a row with a radio, its words and a line under them, after
/// sokkel-ui's `ChoiceList`: the chosen row on the `secondary` ground, another under the pointer on
/// `hover`, each 10 in and 8 over and under, 2 apart.
pub struct ChoiceList<'a> {
    label: &'a str,
    choices: &'a [Choice<'a>],
}

impl<'a> ChoiceList<'a> {
    /// `label` names the list for a screen reader.
    pub fn new(label: &'a str, choices: &'a [Choice<'a>]) -> Self {
        Self { label, choices }
    }

    /// Draws the list; true when another choice was made, `value` then holding it.
    pub fn show(self, ui: &mut egui::Ui, value: &mut String) -> bool {
        let t = crate::tokens(ui.ctx());
        let width = ui.available_width();
        let mut changed = false;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for choice in self.choices {
                let chosen = value == choice.value;
                let text = (width - 20.0 - 28.0).max(1.0);
                let label =
                    egui::WidgetText::from(crate::TextRole::Body.rich(&t, choice.label).variation("wght", 500.0))
                        .into_galley(ui, Some(egui::TextWrapMode::Wrap), text, crate::TextRole::Body.font(&t));
                let line = choice.description.map(|d| {
                    egui::WidgetText::from(crate::TextRole::Help.rich(&t, d)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Wrap),
                        text,
                        crate::TextRole::Help.font(&t),
                    )
                });
                let height = 16.0 + label.size().y + line.as_ref().map_or(0.0, |l| l.size().y);
                let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, chosen, choice.label)
                });
                let painter = ui.painter();
                if chosen {
                    painter.rect_filled(rect, t.radius("control"), t.color("secondary"));
                } else if response.hovered() {
                    painter.rect_filled(rect, t.radius("control"), t.color("hover"));
                }
                // the radio: level with the words' first line where a line follows them, centered otherwise
                let middle = if line.is_some() {
                    rect.top() + 8.0 + label.size().y / 2.0
                } else {
                    rect.center().y
                };
                let dot = egui::pos2(rect.left() + 10.0 + 8.0, middle);
                if chosen {
                    painter.circle_filled(dot, 8.0, t.color("primary"));
                    painter.circle_filled(dot, 4.0, t.color("primary-foreground"));
                } else {
                    painter.circle_stroke(dot, 7.5, egui::Stroke::new(1.0, t.color("input")));
                }
                super::focus_ring(
                    painter,
                    &t,
                    egui::Rect::from_center_size(dot, egui::vec2(16.0, 16.0)),
                    egui::CornerRadius::same(8),
                    &response,
                );
                let x = rect.left() + 10.0 + 16.0 + 12.0;
                let label_height = label.size().y;
                painter.galley(egui::pos2(x, rect.top() + 8.0), label, t.color("foreground"));
                if let Some(line) = line {
                    painter.galley(
                        egui::pos2(x, rect.top() + 8.0 + label_height),
                        line,
                        t.color("muted-foreground"),
                    );
                }
                if response.clicked() && !chosen {
                    *value = choice.value.to_owned();
                    changed = true;
                }
            }
        })
        .response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, self.label));
        changed
    }
}

/// Check boxes under a label of their own, after sokkel-ui's `CheckboxList`: the label medium, 6
/// over the boxes.
pub fn checkbox_list<R>(ui: &mut egui::Ui, label: &str, boxes: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let t = crate::tokens(ui.ctx());
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(crate::TextRole::Body.rich(&t, label).variation("wght", 500.0));
        ui.add_space(6.0);
        boxes(ui)
    })
    .inner
}
