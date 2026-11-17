//! The card for one thing, after sokkel-ui's `ItemCard`, and a grid of them the reader puts in
//! order by dragging, after its `SortableGrid`.

use egui::{Rect, Response, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{TextRole, tokens};

/// A fact at a card's foot: what it is, and its value.
pub struct CardFact {
    pub label: String,
    pub value: String,
}

/// The card for one thing: a form, a project, a job. Its title is the thing's name, its code
/// under it once in the code face; a status line and a description follow, and facts sit at the
/// foot over a rule. Pressed as one, it washes on hover and draws the focus ring.
pub struct ItemCard<'a> {
    title: String,
    code: Option<String>,
    description: Option<String>,
    status: Option<super::section::Tools<'a>>,
    facts: Vec<CardFact>,
    selected: bool,
}

impl<'a> ItemCard<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            code: None,
            description: None,
            status: None,
            facts: Vec::new(),
            selected: false,
        }
    }

    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// A line of small parts under the heading, 8 apart: a chip and a count.
    pub fn status(mut self, status: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.status = Some(Box::new(status));
        self
    }

    pub fn fact(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.facts.push(CardFact {
            label: label.into(),
            value: value.into(),
        });
        self
    }

    /// Marks the chosen card of a set.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Draws the card `width` wide, at least `height` tall; its response is the press on it.
    pub fn show(self, ui: &mut Ui, width: f32, height: f32) -> Response {
        let t = tokens(ui.ctx());
        let inner = (width - 32.0).max(1.0);
        let title = WidgetText::from(TextRole::SectionTitle.rich(&t, &self.title)).into_galley(
            ui,
            Some(TextWrapMode::Wrap),
            inner,
            TextRole::SectionTitle.font(&t),
        );
        let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
        let code = self.code.as_ref().map(|c| {
            WidgetText::from(egui::RichText::new(c).font(mono.clone())).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                inner,
                mono.clone(),
            )
        });
        let description = self.description.as_ref().map(|d| {
            WidgetText::from(TextRole::Body.rich(&t, d)).into_galley(
                ui,
                Some(TextWrapMode::Wrap),
                inner,
                TextRole::Body.font(&t),
            )
        });
        // the body: 16 in, the heading, then 8 to the status and the description
        let status_height = if self.status.is_some() { 20.0 } else { 0.0 };
        let mut body = title.size().y + code.as_ref().map_or(0.0, |c| c.size().y + 4.0);
        if self.status.is_some() {
            body += 8.0 + status_height;
        }
        if let Some(d) = &description {
            body += 8.0 + d.size().y;
        }
        let foot = if self.facts.is_empty() {
            0.0
        } else {
            1.0 + 10.0 + 36.0 + 10.0
        };
        let natural = 16.0 + body + 16.0 + foot;
        let (rect, response) = ui.allocate_exact_size(vec2(width, height.max(natural)), Sense::click());
        response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, self.selected, &self.title));
        let painter = ui.painter().clone();
        let radius = t.radius("surface");
        let hot = response.hovered();
        painter.rect_filled(rect, radius, if hot { t.color("hover") } else { t.color("card") });
        let edge = if self.selected {
            Stroke::new(2.0, t.color("primary"))
        } else if hot {
            Stroke::new(1.0, t.color_at("foreground", 0.25))
        } else {
            Stroke::new(1.0, t.color("border-strong"))
        };
        painter.rect_stroke(rect, radius, edge, egui::StrokeKind::Inside);
        let x = rect.left() + 16.0;
        let mut y = rect.top() + 16.0;
        let title_height = title.size().y;
        painter.galley(pos2(x, y), title, t.color("card-foreground"));
        y += title_height;
        if let Some(code) = code {
            y += 4.0;
            let h = code.size().y;
            painter.galley(pos2(x, y), code, t.color("muted-foreground"));
            y += h;
        }
        if let Some(status) = self.status {
            y += 8.0;
            let line = Rect::from_min_size(pos2(x, y), vec2(inner, status_height));
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(line)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            child.spacing_mut().item_spacing.x = 8.0;
            status(&mut child);
            y += status_height;
        }
        if let Some(description) = description {
            y += 8.0;
            painter.galley(pos2(x, y), description, t.color("muted-foreground"));
        }
        if !self.facts.is_empty() {
            let top = rect.bottom() - foot;
            painter.hline(rect.x_range(), top + 0.5, Stroke::new(1.0, t.color("border")));
            let each = inner / self.facts.len() as f32;
            for (i, fact) in self.facts.iter().enumerate() {
                let fx = x + i as f32 * each;
                painter.text(
                    pos2(fx, top + 11.0),
                    egui::Align2::LEFT_TOP,
                    &fact.label,
                    TextRole::Help.font(&t),
                    t.color("muted-foreground"),
                );
                painter.text(
                    pos2(fx, top + 27.0),
                    egui::Align2::LEFT_TOP,
                    &fact.value,
                    TextRole::Body.font(&t),
                    t.color("card-foreground"),
                );
            }
        }
        super::focus_ring(&painter, &t, rect, radius, &response);
        response
    }
}

/// What a sortable grid was asked this frame.
#[derive(Default)]
pub struct SortableOutcome {
    /// The index of the item pressed.
    pub pressed: Option<usize>,
    /// An item dragged from the first index to the second.
    pub moved: Option<(usize, usize)>,
}

/// Items in a grid of columns at least `min_width` wide and 12 apart, filling the row, the reader
/// moving one by dragging it onto another's place, after sokkel-ui's `SortableGrid`. Each item is
/// drawn by `item` into its cell at the row's height, and answers with its press.
pub struct SortableGrid {
    id: egui::Id,
    count: usize,
    min_width: f32,
    label: String,
}

impl SortableGrid {
    pub fn new(id: impl egui::AsIdSalt, count: usize, label: impl Into<String>) -> Self {
        Self {
            id: egui::Id::unique(id),
            count,
            min_width: 192.0,
            label: label.into(),
        }
    }

    /// The least width of a column; 192 (`12rem`) by default.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    pub fn show(self, ui: &mut Ui, mut item: impl FnMut(&mut Ui, usize, f32, f32) -> Response) -> SortableOutcome {
        let t = tokens(ui.ctx());
        let gap = 12.0;
        let width = ui.available_width();
        let columns = (((width + gap) / (self.min_width + gap)).floor() as usize).max(1);
        let cell = (width - gap * (columns - 1) as f32) / columns as f32;
        let rows = self.count.div_ceil(columns);
        // each row as tall as its tallest card, as the web's grid stretches them
        let heights_id = self.id.with("heights");
        let mut heights: Vec<f32> = ui.data(|d| d.get_temp(heights_id)).unwrap_or_default();
        heights.resize(rows, 0.0);
        let mut measured = vec![0.0_f32; rows];
        let dragging_id = self.id.with("dragging");
        let mut dragging: Option<usize> = ui.data(|d| d.get_temp(dragging_id));
        let mut out = SortableOutcome::default();
        let origin = ui.cursor().min;
        let mut rects = Vec::with_capacity(self.count);
        let mut y = origin.y;
        for row in 0..rows {
            for column in 0..columns {
                let index = row * columns + column;
                if index >= self.count {
                    break;
                }
                let at = Rect::from_min_size(
                    pos2(origin.x + column as f32 * (cell + gap), y),
                    vec2(cell, heights[row]),
                );
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(self.id.with(index))
                        .max_rect(Rect::from_min_size(at.min, vec2(cell, f32::INFINITY))),
                );
                if dragging == Some(index) {
                    child.set_opacity(0.5);
                }
                let response = item(&mut child, index, cell, heights[row]);
                measured[row] = measured[row].max(response.rect.height());
                let drag = ui.interact(response.rect, self.id.with(("drag", index)), Sense::drag());
                if drag.drag_started() {
                    dragging = Some(index);
                }
                if response.clicked() {
                    out.pressed = Some(index);
                }
                rects.push(response.rect);
            }
            y += measured[row] + gap;
        }
        let bottom = y - if rows > 0 { gap } else { 0.0 };
        ui.allocate_rect(
            Rect::from_min_max(origin, pos2(origin.x + width, bottom.max(origin.y))),
            Sense::hover(),
        );
        if let Some(from) = dragging {
            let pointer = ui.input(|i| i.pointer.interact_pos());
            let target = pointer.and_then(|p| rects.iter().position(|r| r.contains(p)));
            if let Some(to) = target.filter(|to| *to != from) {
                // where it lands: a bar at the start of the card it takes the place of
                let r = rects[to];
                ui.painter().vline(
                    if to < from {
                        r.left() - gap / 2.0
                    } else {
                        r.right() + gap / 2.0
                    },
                    r.y_range(),
                    Stroke::new(2.0, t.color("primary")),
                );
            }
            if ui.input(|i| i.pointer.any_released()) {
                if let Some(to) = target.filter(|to| *to != from) {
                    out.moved = Some((from, to));
                }
                dragging = None;
            }
        }
        ui.data_mut(|d| {
            d.insert_temp(heights_id, measured);
            match dragging {
                Some(i) => {
                    d.insert_temp(dragging_id, i);
                }
                None => d.remove::<usize>(dragging_id),
            }
        });
        let _ = &self.label;
        out
    }
}
