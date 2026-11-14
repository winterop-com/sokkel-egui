//! A titled part of a page, after sokkel-ui's `Section` (its spec, `parts/section`), and a person
//! on a line of a list, after its `PersonRow`.

use egui::{CornerRadius, Rect, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{AvatarSize, TextRole, tokens};

/// Something drawn into a part's room later: a section head's controls, an event's content.
pub type Tools<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A boxed section: a head bar with its title, a meta beside it and a description under it, over
/// its body, in one box with the surface's corners.
pub struct Section<'a> {
    title: String,
    meta: Option<String>,
    description: Option<String>,
    flush: bool,
    actions: Option<Tools<'a>>,
}

impl<'a> Section<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            meta: None,
            description: None,
            flush: false,
            actions: None,
        }
    }

    /// Controls at the head's end, such as a state's chip.
    pub fn actions(mut self, actions: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.actions = Some(Box::new(actions));
        self
    }

    /// A count or a short fact 8 after the title, quieter than it.
    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    /// A sentence under the title saying what the section holds; the head grows to hold it.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The body to the box's edges, for a table, rather than padded.
    pub fn flush(mut self, flush: bool) -> Self {
        self.flush = flush;
        self
    }

    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let pad = t.density("panel-pad");
        let radius = t.radius("surface");
        let frame = egui::Frame::new()
            .fill(t.color("card"))
            .stroke(Stroke::new(1.0, t.color("border-strong")))
            .corner_radius(radius);
        frame
            .show(ui, |ui| {
                ui.set_width(width - 2.0);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                // the head: at least panel-head tall; 8 over and under a described one, the title 4
                // over and under in a bare one
                let inner = width - 2.0 - 2.0 * pad;
                let title = WidgetText::from(TextRole::Head.rich(&t, &self.title)).into_galley(
                    ui,
                    Some(TextWrapMode::Truncate),
                    inner.max(1.0),
                    TextRole::Head.font(&t),
                );
                let meta = self.meta.as_ref().map(|m| {
                    WidgetText::from(TextRole::Help.rich(&t, m)).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        inner.max(1.0),
                        TextRole::Help.font(&t),
                    )
                });
                let description = self.description.as_ref().map(|d| {
                    WidgetText::from(TextRole::Help.rich(&t, d)).into_galley(
                        ui,
                        Some(TextWrapMode::Wrap),
                        inner.max(1.0),
                        TextRole::Help.font(&t),
                    )
                });
                let line = 20.0_f32.max(title.size().y);
                let block = line + description.as_ref().map_or(0.0, |d| d.size().y);
                let height = match &description {
                    Some(_) => block + 16.0,
                    None => (block + 8.0).max(t.density("panel-head")),
                };
                let (head, _) = ui.allocate_exact_size(vec2(width - 2.0, height), Sense::hover());
                let top_corners = CornerRadius {
                    sw: 0,
                    se: 0,
                    ..CornerRadius::from(radius.nw.saturating_sub(1))
                };
                ui.painter()
                    .rect_filled(head, top_corners, t.color("section-head-ground"));
                ui.painter().hline(
                    head.x_range(),
                    head.bottom() - 0.5,
                    Stroke::new(1.0, t.color("section-head-line")),
                );
                let top = head.top() + (height - block) / 2.0;
                let x = head.left() + pad;
                let title_width = title.size().x;
                ui.painter().galley(
                    pos2(x, top + line / 2.0 - title.size().y / 2.0),
                    title,
                    t.color("section-head-ink"),
                );
                if let Some(meta) = meta {
                    ui.painter().galley(
                        pos2(x + title_width + 8.0, top + line / 2.0 - meta.size().y / 2.0),
                        meta,
                        t.color("muted-foreground"),
                    );
                }
                if let Some(description) = description {
                    ui.painter()
                        .galley(pos2(x, top + line), description, t.color("muted-foreground"));
                }
                if let Some(actions) = self.actions {
                    // level with the title's line, at the head's end
                    let end =
                        egui::Rect::from_min_max(pos2(head.left() + pad, top), pos2(head.right() - pad, top + line));
                    let mut tools = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt("section-actions")
                            .max_rect(end)
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                    );
                    tools.spacing_mut().item_spacing.x = 8.0;
                    actions(&mut tools);
                }
                // the body, padded unless flush
                let margin = if self.flush { 0 } else { pad as i8 };
                egui::Frame::new()
                    .inner_margin(egui::Margin::same(margin))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        body(ui)
                    })
                    .inner
            })
            .inner
    }
}

/// A person on a line: their avatar, their name in the code face over a muted detail, and an aside
/// at the end (a chip, a time), after sokkel-ui's `PersonRow`.
pub struct PersonRow {
    name: String,
    detail: Option<String>,
}

impl PersonRow {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            detail: None,
        }
    }

    /// The line under the name: what the person is to this place.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Draw it across `ui`'s width; `aside` draws what stands at its end, from the end.
    pub fn show(self, ui: &mut Ui, aside: impl FnOnce(&mut Ui)) -> egui::Response {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let height = 20.0 + if self.detail.is_some() { 20.0 } else { 0.0 };
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let avatar = Rect::from_center_size(pos2(rect.left() + 12.0, rect.center().y), vec2(24.0, 24.0));
        super::person::paint_avatar(ui, &t, avatar, &self.name, AvatarSize::Sm, false);
        let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
        let name = WidgetText::from(egui::RichText::new(&self.name).font(mono.clone())).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            (width - 140.0).max(1.0),
            mono,
        );
        let x = avatar.right() + 12.0;
        ui.painter().galley(
            pos2(x, rect.top() + 10.0 - name.size().y / 2.0),
            name,
            t.color("foreground"),
        );
        if let Some(detail) = &self.detail {
            let galley = WidgetText::from(TextRole::Help.rich(&t, detail)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (width - 140.0).max(1.0),
                TextRole::Help.font(&t),
            );
            ui.painter().galley(
                pos2(x, rect.top() + 30.0 - galley.size().y / 2.0),
                galley,
                t.color("muted-foreground"),
            );
        }
        let mut end = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        aside(&mut end);
        response
    }
}

/// One event on a `Timeline`.
#[derive(Default)]
pub struct TimelineEvent {
    /// An icon in place of the dot.
    pub icon: Option<crate::Icon>,
    /// The dot's color: the state the event left things in; a faint dot when `None`.
    pub tone: Option<egui::Color32>,
    pub title: String,
    /// When, in words ("now", "last week").
    pub at: String,
    pub detail: Option<String>,
    /// More under the detail, such as a day's incidents.
    pub content: Option<Tools<'static>>,
}

/// Events one under another, newest first, after sokkel-ui's `Timeline`: each its icon, its title
/// with when at the line's end, and its detail under it, a line joining the icons.
pub struct Timeline {
    events: Vec<TimelineEvent>,
}

impl Timeline {
    pub fn new(events: impl IntoIterator<Item = TimelineEvent>) -> Self {
        Self {
            events: events.into_iter().collect(),
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let text_x = 16.0 + 12.0;
        let mut icons_at: Vec<f32> = Vec::new();
        let count = self.events.len();
        for (index, event) in self.events.into_iter().enumerate() {
            let at = WidgetText::from(TextRole::Help.rich(&t, &event.at)).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextRole::Help.font(&t),
            );
            let title = WidgetText::from(TextRole::Body.rich(&t, &event.title)).into_galley(
                ui,
                Some(TextWrapMode::Wrap),
                (width - text_x - at.size().x - 8.0).max(1.0),
                TextRole::Body.font(&t),
            );
            let detail = event.detail.as_ref().map(|d| {
                WidgetText::from(TextRole::Help.rich(&t, d)).into_galley(
                    ui,
                    Some(TextWrapMode::Wrap),
                    (width - text_x).max(1.0),
                    TextRole::Help.font(&t),
                )
            });
            // the title's lines, a line of 24 for one, the detail under the last
            let line = title.size().y.max(20.0) + 4.0;
            let height = line + detail.as_ref().map_or(0.0, |d| d.size().y);
            // 16 under each event but the last, as the web's `pb-4`
            let gap = if index + 1 < count { 16.0 } else { 0.0 };
            // the gap after an event comes after its content, where it has some
            let after = if event.content.is_some() { 0.0 } else { gap };
            let (rect, _) = ui.allocate_exact_size(vec2(width, height + after), Sense::hover());
            let middle = rect.top() + 12.0;
            match event.icon {
                Some(icon) => crate::icons::paint(
                    ui.painter(),
                    pos2(rect.left() + 8.0, middle),
                    icon,
                    16.0,
                    event.tone.unwrap_or(t.color("muted-foreground")),
                ),
                None => {
                    ui.painter().circle_filled(
                        pos2(rect.left() + 8.0, middle),
                        4.0,
                        event.tone.unwrap_or(t.color("border-strong")),
                    );
                }
            }
            icons_at.push(middle);
            ui.painter().galley(
                pos2(rect.left() + text_x, rect.top() + 2.0),
                title,
                t.color("foreground"),
            );
            ui.painter().galley(
                pos2(rect.right() - at.size().x, middle - at.size().y / 2.0),
                at,
                t.color("muted-foreground"),
            );
            if let Some(detail) = detail {
                ui.painter().galley(
                    pos2(rect.left() + text_x, rect.top() + line),
                    detail,
                    t.color("muted-foreground"),
                );
            }
            if let Some(content) = event.content {
                // 6 under the title or the detail, as far in as the title
                let top = rect.top() + height + 6.0;
                let room = egui::Rect::from_min_max(pos2(rect.left() + text_x, top), pos2(rect.right(), f32::INFINITY));
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("timeline-content", index))
                        .max_rect(room)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                content(&mut child);
                let used = child.min_rect().height() + 6.0;
                ui.allocate_space(vec2(width, used + gap));
            }
        }
        // the line joining each icon to the next, under them
        let left = ui.min_rect().left() + 8.0;
        for pair in icons_at.windows(2) {
            ui.painter().vline(
                left,
                pair[0] + 12.0..=pair[1] - 12.0,
                Stroke::new(1.0, t.color("border")),
            );
        }
    }
}

/// A term over its value, after sokkel-ui's `DescriptionList` with `layout="stacked"`: the term in
/// the small muted words, the value 2 under it (words, or any control), each pair 8 under the one
/// before it.
pub struct DescriptionList<'a> {
    items: Vec<(String, Tools<'a>)>,
}

impl<'a> Default for DescriptionList<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DescriptionList<'a> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// A pair whose value is plain words.
    pub fn text(self, term: impl Into<String>, value: impl Into<String>) -> Self {
        let value = value.into();
        self.item(term, move |ui| {
            let t = tokens(ui.ctx());
            ui.label(TextRole::Body.rich(&t, value).color(t.color("foreground")));
        })
    }

    /// A pair whose value is drawn by `value`, such as a select or a switch.
    pub fn item(mut self, term: impl Into<String>, value: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.items.push((term.into(), Box::new(value)));
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            for (term, value) in self.items {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(TextRole::Help.rich(&t, term).color(t.color("muted-foreground")));
                    value(ui);
                });
            }
        });
    }
}
