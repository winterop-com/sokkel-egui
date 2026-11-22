//! Choosing: Checkbox, Tabs and Segmented, after sokkel-ui's components of the same names.

use egui::{
    Color32, CornerRadius, Rect, Response, Role, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Widget, WidgetInfo,
    WidgetText, pos2, vec2,
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
    field: bool,
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
            field: false,
        }
    }

    /// As sokkel-ui's `CheckboxField`: the whole row is the label, as wide as its room, 8 in and
    /// 6 down, the box 10 before the label, the `hover` ground under the pointer.
    pub fn field(mut self) -> Self {
        self.field = true;
        self
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
        let (inset, gap) = if self.field {
            (vec2(8.0, 6.0), 10.0)
        } else {
            (vec2(0.0, 0.0), 8.0)
        };
        // the label wraps in the room it is given, so a long one never pushes past its column
        let room = (ui.available_width() - inset.x * 2.0 - 16.0 - gap).max(48.0);
        let galley = self.show_label.then(|| {
            let text = TextRole::Body.rich(&t, &self.label);
            // a field's label lines are 20 tall, as the web's `leading-5`
            let text = if self.field { text.line_height(Some(20.0)) } else { text };
            WidgetText::from(text).into_galley(ui, Some(TextWrapMode::Wrap), room, TextRole::Body.font(&t))
        });
        let first_row = galley
            .as_ref()
            .and_then(|g| g.rows.first().map(|r| r.height()))
            .unwrap_or(16.0);
        let text_width = galley.as_ref().map_or(0.0, |g| g.size().x + gap);
        let height = galley.as_ref().map_or(16.0, |g| g.size().y.max(16.0));
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let width = if self.field {
            ui.available_width()
        } else {
            16.0 + text_width
        };
        let (outer, mut response) = ui.allocate_exact_size(vec2(width, height + inset.y * 2.0), sense);
        let rect = Rect::from_min_size(outer.min + inset, vec2(16.0 + text_width, height));
        if response.clicked() {
            *self.checked = !*self.checked;
            response.mark_changed();
        }
        let on = *self.checked || self.indeterminate;
        response.widget_info(|| WidgetInfo::selected(Role::CheckBox, self.enabled, on, &self.label));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        if self.field && response.hovered() && self.enabled {
            painter.rect_filled(outer, t.radius("control"), t.color("hover"));
        }
        // the box beside the label's first line
        let middle = if height > first_row + 1.0 {
            rect.top() + first_row / 2.0
        } else {
            rect.center().y
        };
        let mark = Rect::from_min_size(pos2(rect.left(), middle - 8.0), vec2(16.0, 16.0));
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
                pos2(mark.right() + gap, rect.center().y - galley.size().y / 2.0),
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
            hit.widget_info(|| WidgetInfo::selected(Role::Button, true, chosen, &self.tabs[index]));
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
    fill: bool,
    off: Vec<Option<String>>,
}

impl<'a> Segmented<'a> {
    pub fn new(chosen: &'a mut usize, options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            chosen,
            options: options.into_iter().map(Into::into).collect(),
            variant: SegmentedVariant::Track,
            counts: Vec::new(),
            dots: Vec::new(),
            fill: false,
            off: Vec::new(),
        }
    }

    /// Options that cannot be chosen now, each with what to do first (shown while it is pointed
    /// at), as the web's option `disabled` with its `hint`: drawn in the faint ink, a press does
    /// nothing.
    pub fn disabled(mut self, off: impl IntoIterator<Item = Option<String>>) -> Self {
        self.off = off.into_iter().collect();
        self
    }

    /// Stretch the control across its room, as the web's `fill`, so controls stacked in one
    /// column line up on both edges. Where the room is narrower than the options they wrap onto
    /// more lines, each holding as many and shared out evenly.
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
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
                let mono = crate::mono_font(
                    ui.ctx(),
                    crate::mono_small_size(&t, TextRole::Help.font(&t).size),
                    400.0,
                );
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
        let count = galleys.len().max(1);
        let natural = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32 + 2.0 * inset;
        // as many options on a line as the room holds at their width, the lines then evened out
        let room = ui.available_width();
        let unit = widest + 20.0;
        let fits = (((room - 2.0 * inset + gap) / (unit + gap)).floor() as usize).clamp(1, count);
        let lines = count.div_ceil(fits);
        let per_line = count.div_ceil(lines);
        let width = if self.fill { room } else { natural.min(room) };
        let (rect, mut response) =
            ui.allocate_exact_size(vec2(width, lines as f32 * height + 2.0 * inset), Sense::hover());
        // an option's place: its line, and its share of that line
        let place = |index: usize| {
            let line = index / per_line;
            let on_line = (count - line * per_line).min(per_line);
            let column = index % per_line;
            let share = (width - 2.0 * inset - gap * (on_line - 1) as f32) / on_line as f32;
            Rect::from_min_size(
                pos2(
                    rect.left() + inset + column as f32 * (share + gap),
                    rect.top() + inset + line as f32 * height,
                ),
                vec2(share, height),
            )
        };
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
        for (index, galley) in galleys.into_iter().enumerate() {
            let option = place(index);
            let off = self.off.get(index).cloned().flatten();
            let sense = if off.is_some() { Sense::hover() } else { Sense::click() };
            let mut hit = ui.interact(option, response.id.with(index), sense);
            let chosen = *self.chosen == index;
            hit.widget_info(|| WidgetInfo::selected(Role::Button, off.is_none(), chosen, &self.options[index]));
            if let Some(hint) = &off {
                hit = hit.on_hover_text(hint);
            }
            if hit.clicked() && !chosen && off.is_none() {
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
                (_, false) if off.is_some() => (Color32::TRANSPARENT, t.color("faint")),
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
    swatches: Option<Vec<Option<crate::Swatch>>>,
}

impl<'a> ChoiceList<'a> {
    /// `label` names the list for a screen reader.
    pub fn new(label: &'a str, choices: &'a [Choice<'a>]) -> Self {
        Self {
            label,
            choices,
            swatches: None,
        }
    }

    /// A theme's sample before each choice's words, 8 before them, as a theme list draws its
    /// themes; the words are then at the body's own weight, as the web's label that is not words
    /// alone.
    pub fn swatches(mut self, swatches: impl IntoIterator<Item = Option<crate::Swatch>>) -> Self {
        self.swatches = Some(swatches.into_iter().collect());
        self
    }

    /// Draws the list; true when another choice was made, `value` then holding it.
    ///
    /// The list is one Tab stop, at the chosen choice (the first when none is): the arrows move to
    /// the next or the one before and choose it, wrapping, and Space chooses the focused one.
    pub fn show(self, ui: &mut egui::Ui, value: &mut String) -> bool {
        let t = crate::tokens(ui.ctx());
        let width = ui.available_width();
        let mut changed = false;
        let ink = ui.visuals().text_color();
        let list_id = ui.make_persistent_id(("choice-list", self.label));
        let ids: Vec<egui::Id> = (0..self.choices.len()).map(|i| list_id.with(i)).collect();
        // the one Tab stop: the chosen choice, or the first
        let stop = self.choices.iter().position(|c| c.value == value.as_str()).unwrap_or(0);
        let focused = ids.iter().position(|id| ui.memory(|m| m.has_focus(*id)));
        // arrows move and choose, wrapping; Space chooses the focused choice
        if let Some(at) = focused
            && !self.choices.is_empty()
        {
            let count = self.choices.len();
            let (next, before, space) = ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                        || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
                        || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft),
                    i.key_pressed(egui::Key::Space),
                )
            });
            let to = if next {
                Some((at + 1) % count)
            } else if before {
                Some((at + count - 1) % count)
            } else if space {
                Some(at)
            } else {
                None
            };
            if let Some(to) = to {
                if value != self.choices[to].value {
                    *value = self.choices[to].value.to_owned();
                    changed = true;
                }
                ui.memory_mut(|m| m.request_focus(ids[to]));
            }
        }
        let stop = self
            .choices
            .iter()
            .position(|c| c.value == value.as_str())
            .unwrap_or(stop);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (index, choice) in self.choices.iter().enumerate() {
                let chosen = value == choice.value;
                let text = (width - 20.0 - 28.0).max(1.0);
                let sample = self.swatches.as_ref().map(|all| all.get(index).copied().flatten());
                let lead = if sample.is_some() { 28.0 + 8.0 } else { 0.0 };
                let weight = if sample.is_some() {
                    400.0
                } else {
                    t.weight(crate::Weight::Medium)
                };
                let sm = t.text("sm");
                let label = egui::WidgetText::from(
                    egui::RichText::new(choice.label)
                        .size(sm.size)
                        .line_height(Some(sm.line_height))
                        .variation("wght", weight),
                )
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Wrap),
                    (text - lead).max(1.0),
                    egui::FontId::proportional(sm.size),
                );
                let line = choice.description.map(|d| {
                    egui::WidgetText::from(crate::TextRole::Help.rich(&t, d)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Wrap),
                        text,
                        crate::TextRole::Help.font(&t),
                    )
                });
                let height = 16.0 + label.size().y + line.as_ref().map_or(0.0, |l| l.size().y);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                // only the Tab stop takes focus from Tab; every row takes the pointer
                let sense = if index == stop {
                    egui::Sense::click()
                } else {
                    egui::Sense::CLICK
                };
                let response = ui.interact(rect, ids[index], sense);
                response
                    .widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, chosen, choice.label));
                if response.has_focus() {
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            ids[index],
                            egui::EventFilter {
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                ..Default::default()
                            },
                        )
                    });
                }
                let painter = ui.painter();
                if chosen {
                    painter.rect_filled(rect, t.radius("control"), t.color("secondary"));
                } else if response.hovered() {
                    painter.rect_filled(rect, t.radius("control"), t.color("hover"));
                }
                // the radio: level with the label's first line (2 down) where a description
                // follows it, centered on the row otherwise
                let middle = if line.is_some() {
                    rect.top() + 8.0 + 2.0 + 8.0
                } else {
                    rect.center().y
                };
                let dot = egui::pos2(rect.left() + 10.0 + 8.0, middle);
                if chosen {
                    painter.circle_filled(dot, 8.0, t.color("primary"));
                    painter.circle_filled(dot, 4.0, t.color("primary-foreground"));
                } else {
                    if t.dark {
                        painter.circle_filled(dot, 8.0, t.color_at("input", 0.3));
                    }
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
                if let Some(sample) = sample {
                    super::overlay::paint_swatch(
                        painter,
                        &t,
                        egui::pos2(x, rect.top() + 8.0 + label_height / 2.0),
                        sample,
                    );
                }
                painter.galley(egui::pos2(x + lead, rect.top() + 8.0), label, ink);
                if let Some(line) = line {
                    painter.galley(
                        egui::pos2(x, rect.top() + 8.0 + label_height),
                        line,
                        t.color("muted-foreground"),
                    );
                }
                // a press chooses (the pointer's, or a screen reader's); of the keys only Space
                // does, never Enter
                if response.clicked() && !ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if !chosen {
                        *value = choice.value.to_owned();
                        changed = true;
                    }
                    ui.memory_mut(|m| m.request_focus(ids[index]));
                }
            }
        })
        .response
        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::RadioGroup, true, self.label));
        changed
    }
}

/// Check boxes under a label of their own, after sokkel-ui's `CheckboxList`: the label medium, 6
/// over the boxes.
pub fn checkbox_list<R>(ui: &mut egui::Ui, label: &str, boxes: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let t = crate::tokens(ui.ctx());
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(
            crate::TextRole::Body
                .rich(&t, label)
                .variation("wght", t.weight(crate::Weight::Medium)),
        );
        ui.add_space(6.0);
        boxes(ui)
    })
    .inner
}

/// One color among a few, each a swatch pressed in a row that wraps, as sokkel-ui's outline
/// `ToggleGroup` of `Dot`s at `sm`: each a small control square with the `input` edge, its dot of
/// 10 centered, 4 apart, the chosen one on the `muted` ground. Each is named by its label for a
/// screen reader and the pointer.
#[must_use = "add it with `ui.add(...)`"]
pub struct SwatchGroup<'a> {
    chosen: &'a mut usize,
    swatches: &'a [(Color32, String)],
}

impl<'a> SwatchGroup<'a> {
    pub fn new(chosen: &'a mut usize, swatches: &'a [(Color32, String)]) -> Self {
        Self { chosen, swatches }
    }
}

impl Widget for SwatchGroup<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let side = super::finger_tall(ui.ctx(), &t, t.density("control-sm"));
        // a dot of 10 between 10 on each side, at least the control square
        let width = (10.0 + 20.0_f32).max(side);
        let gap = 4.0;
        let room = ui.available_width();
        let per_line = (((room + gap) / (width + gap)).floor() as usize).clamp(1, self.swatches.len().max(1));
        let lines = self.swatches.len().div_ceil(per_line).max(1);
        let used = per_line.min(self.swatches.len()) as f32 * (width + gap) - gap;
        let (rect, mut response) =
            ui.allocate_exact_size(vec2(used.max(0.0), lines as f32 * (side + gap) - gap), Sense::hover());
        let r = t.radius("control");
        let radius = CornerRadius::same(r.nw.min(12));
        for (index, (color, label)) in self.swatches.iter().enumerate() {
            let place = Rect::from_min_size(
                pos2(
                    rect.left() + (index % per_line) as f32 * (width + gap),
                    rect.top() + (index / per_line) as f32 * (side + gap),
                ),
                vec2(width, side),
            );
            let hit = ui.interact(place, response.id.with(index), Sense::click());
            let chosen = *self.chosen == index;
            hit.widget_info(|| WidgetInfo::selected(Role::Button, true, chosen, label));
            if hit.clicked() && !chosen {
                *self.chosen = index;
                response.mark_changed();
            }
            let ground = if chosen {
                t.color("muted")
            } else if hit.hovered() {
                t.color("hover")
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect(
                place,
                radius,
                ground,
                Stroke::new(1.0, t.color("input")),
                StrokeKind::Inside,
            );
            ui.painter().circle_filled(place.center(), 5.0, *color);
            focus_ring(ui.painter(), &t, place, radius, &hit);
            let _ = hit.on_hover_text(label.clone());
        }
        response
    }
}
