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
    flat: bool,
    actions: Option<Tools<'a>>,
}

impl<'a> Section<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            meta: None,
            description: None,
            flush: false,
            flat: false,
            actions: None,
        }
    }

    /// A heading over a rule with the body on the page's own ground, as the web's
    /// `variant="flat"`: the title in the section title's type, the body `section-gap` under it.
    pub fn flat(mut self, flat: bool) -> Self {
        self.flat = flat;
        self
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
        if self.flat {
            return self.show_flat(ui, body);
        }
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
                // the description keeps clear of the actions at the head's end, 12 before them, as
                // the web's head row lays its words beside them; their width as last drawn
                let tools_id = ui.scope_id().with("section-actions-width");
                let tools_width: f32 = if self.actions.is_some() {
                    ui.data(|d| d.get_temp(tools_id)).unwrap_or(0.0)
                } else {
                    0.0
                };
                let words_width = if tools_width > 0.0 {
                    (inner - tools_width - 12.0).max(inner / 2.0)
                } else {
                    inner
                };
                let description = self.description.as_ref().map(|d| {
                    WidgetText::from(TextRole::Help.rich(&t, d)).into_galley(
                        ui,
                        Some(TextWrapMode::Wrap),
                        words_width.max(1.0),
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
                    // centered on the title's line, at the head's end, with room for a control
                    // taller than the line
                    let middle = top + line / 2.0;
                    let half = (t.density("panel-head") / 2.0).max(line / 2.0);
                    let end = egui::Rect::from_min_max(
                        pos2(head.left() + pad, middle - half),
                        pos2(head.right() - pad, middle + half),
                    );
                    let mut tools = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt("section-actions")
                            .max_rect(end)
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                    );
                    tools.spacing_mut().item_spacing.x = 8.0;
                    actions(&mut tools);
                    let drawn = tools.min_rect().width();
                    if (drawn - tools_width).abs() > 0.5 {
                        ui.data_mut(|d| d.insert_temp(tools_id, drawn));
                        ui.ctx().request_repaint();
                    }
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

impl Section<'_> {
    fn show_flat<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let title = WidgetText::from(TextRole::SectionTitle.rich(&t, &self.title)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                width.max(1.0),
                TextRole::SectionTitle.font(&t),
            );
            let meta = self.meta.as_ref().map(|m| {
                WidgetText::from(TextRole::Help.rich(&t, m)).into_galley(
                    ui,
                    Some(TextWrapMode::Truncate),
                    width.max(1.0),
                    TextRole::Help.font(&t),
                )
            });
            let description = self.description.as_ref().map(|d| {
                WidgetText::from(TextRole::Help.rich(&t, d)).into_galley(
                    ui,
                    Some(TextWrapMode::Wrap),
                    width.max(1.0),
                    TextRole::Help.font(&t),
                )
            });
            // the head: at least panel-head tall, 8 over and under a described one, its rule under it
            let line = title.size().y.max(20.0);
            let block = line + description.as_ref().map_or(0.0, |d| d.size().y);
            let height = match &description {
                Some(_) => block + 16.0,
                None => (block + 8.0).max(t.density("panel-head")),
            };
            let (head, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            ui.painter()
                .hline(head.x_range(), head.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
            let top = head.top() + (height - block) / 2.0;
            let title_width = title.size().x;
            ui.painter().galley(
                pos2(head.left(), top + line / 2.0 - title.size().y / 2.0),
                title,
                t.color("section-head-ink"),
            );
            if let Some(meta) = meta {
                ui.painter().galley(
                    pos2(head.left() + title_width + 8.0, top + line / 2.0 - meta.size().y / 2.0),
                    meta,
                    t.color("muted-foreground"),
                );
            }
            if let Some(description) = description {
                ui.painter()
                    .galley(pos2(head.left(), top + line), description, t.color("muted-foreground"));
            }
            if let Some(actions) = self.actions {
                let end = Rect::from_min_max(pos2(head.left(), head.top()), pos2(head.right(), head.bottom()));
                let mut tools = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("section-actions")
                        .max_rect(end)
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                tools.spacing_mut().item_spacing.x = 4.0;
                actions(&mut tools);
            }
            ui.add_space(t.density("section-gap"));
            body(ui)
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
    /// Who the event is by: their avatar before the detail, as the web's detail with a
    /// `PersonAvatar` of `2xs`.
    pub person: Option<String>,
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
            // the person's avatar, 16 and 4 before the detail
            let avatar = if event.person.is_some() { 16.0 + 4.0 } else { 0.0 };
            let detail = event.detail.as_ref().map(|d| {
                WidgetText::from(TextRole::Help.rich(&t, d)).into_galley(
                    ui,
                    Some(TextWrapMode::Wrap),
                    (width - text_x - avatar).max(1.0),
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
                if let Some(person) = &event.person {
                    let first = detail.rows.first().map_or(detail.size().y, |r| r.rect().height());
                    let spot = egui::Rect::from_center_size(
                        pos2(rect.left() + text_x + 8.0, rect.top() + line + first / 2.0),
                        vec2(16.0, 16.0),
                    );
                    crate::widgets::person::paint_avatar(ui, &t, spot, person, crate::AvatarSize::Xxs, false);
                }
                ui.painter().galley(
                    pos2(rect.left() + text_x + avatar, rect.top() + line),
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

/// How a description list sets its pairs, as sokkel-ui's `DescriptionList` `layout`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DescriptionLayout {
    /// Each term in a column beside its value.
    #[default]
    Side,
    /// Each term over its value.
    Stacked,
    /// The pairs one after another in a line that wraps, for a strip of facts under a title.
    Row,
}

/// A fact's value: words, a code, nothing (a dash in faint), or what the app draws.
enum FactValue<'a> {
    Text(String),
    Code(String),
    Missing,
    /// What the app draws; `true` for a value edited in place, whose box reaches into the gap.
    Drawn(Tools<'a>, bool),
}

struct Fact<'a> {
    term: String,
    value: FactValue<'a>,
    wide: bool,
}

/// Facts about one thing as terms and values, after sokkel-ui's `DescriptionList`: side by side
/// by default, each term in a column 96 wide (or as wide as the longest term) 12 before its value;
/// stacked, each term 2 over its value; or in a row that wraps. The terms are small and muted, the
/// values `text.sm` on 20 lines (`sm`: `text.xs` on 16), a code in the code face, a missing value a
/// dash in faint.
pub struct DescriptionList<'a> {
    items: Vec<Fact<'a>>,
    layout: DescriptionLayout,
    /// The term column's width side by side; `None` for as wide as the longest term.
    term_width: Option<f32>,
    two: bool,
    small: bool,
    /// The terms in the mono face, as terms that are names in code.
    mono: bool,
}

impl<'a> Default for DescriptionList<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DescriptionList<'a> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            layout: DescriptionLayout::Side,
            term_width: Some(96.0),
            two: false,
            small: false,
            mono: false,
        }
    }

    /// How the pairs are set: side by side (the default), stacked, or in a row that wraps.
    pub fn layout(mut self, layout: DescriptionLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Each term over its value, as the web's `layout="stacked"`.
    pub fn stacked(self) -> Self {
        self.layout(DescriptionLayout::Stacked)
    }

    /// Each term beside its value, its column `width` wide, as the web's default layout with a
    /// `termWidth`.
    pub fn side(mut self, width: f32) -> Self {
        self.layout = DescriptionLayout::Side;
        self.term_width = Some(width);
        self
    }

    /// Side by side, the term column as wide as the longest term (at most two fifths of the list),
    /// as the web's `termWidth="auto"`.
    pub fn term_width_auto(mut self) -> Self {
        self.layout = DescriptionLayout::Side;
        self.term_width = None;
        self
    }

    /// The values at `text.xs` on 16 lines with tighter gaps, for a narrow inspector, as the web's
    /// `size="sm"`.
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// The side and stacked pairs two to a row from `md`, one below it, as the web's `columns={2}`.
    pub fn columns(mut self, columns: usize) -> Self {
        self.two = columns >= 2;
        self
    }

    /// The terms in the mono face at their small size, as terms that are names in code (a
    /// parameter's name, a field's key).
    pub fn mono_terms(mut self) -> Self {
        self.mono = true;
        self
    }

    /// A pair whose value is a code, in the code face at the list's size; empty, a dash in faint.
    pub fn code(mut self, term: impl Into<String>, value: impl Into<String>) -> Self {
        let value = value.into();
        let value = if value.is_empty() {
            FactValue::Missing
        } else {
            FactValue::Code(value)
        };
        self.push(term.into(), value);
        self
    }

    /// A pair whose value is plain words; empty, a dash in faint.
    pub fn text(mut self, term: impl Into<String>, value: impl Into<String>) -> Self {
        let value = value.into();
        let value = if value.is_empty() {
            FactValue::Missing
        } else {
            FactValue::Text(value)
        };
        self.push(term.into(), value);
        self
    }

    /// A pair whose value may be missing: a dash in faint when it is.
    pub fn maybe(self, term: impl Into<String>, value: Option<impl Into<String>>) -> Self {
        match value {
            Some(value) => self.text(term, value),
            None => self.text(term, String::new()),
        }
    }

    /// A pair whose value is a number, time or duration, as the web's `numeric`. The web sets its
    /// figures at one width; egui's text has no OpenType features, so the figures are the face's own.
    pub fn numeric(self, term: impl Into<String>, value: impl Into<String>) -> Self {
        self.text(term, value)
    }

    /// A pair whose value is drawn by `value`, such as a select or a switch.
    pub fn item(mut self, term: impl Into<String>, value: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.push(term.into(), FactValue::Drawn(Box::new(value), false));
        self
    }

    /// A pair whose value is edited in place (an `InlineEdit`): its box reaches 11 into the gap
    /// before it, so its text starts where a plain value's does.
    pub fn editable(mut self, term: impl Into<String>, value: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.push(term.into(), FactValue::Drawn(Box::new(value), true));
        self
    }

    /// The pair added last takes the list's full width: side by side its value goes under its
    /// term; in a two-column stacked list it spans both columns.
    pub fn wide(mut self) -> Self {
        if let Some(last) = self.items.last_mut() {
            last.wide = true;
        }
        self
    }

    fn push(&mut self, term: String, value: FactValue<'a>) {
        self.items.push(Fact {
            term,
            value,
            wide: false,
        });
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let small = self.small;
        let ink = ui.visuals().text_color();
        let (value_size, line) = if small {
            (t.text("help").size, 16.0)
        } else {
            (t.text("sm").size, 20.0)
        };
        let ctx = ui.ctx().clone();
        let mono = self.mono;
        let term_rich = |term: &str| {
            let rich = if mono {
                egui::RichText::new(term).font(crate::mono_font(&ctx, crate::mono_size(&t, t.text("help").size), 400.0))
            } else {
                TextRole::Help.rich(&t, term)
            };
            rich.line_height(Some(line))
        };
        let lay_term = |ui: &Ui, term: &str, width: f32| {
            WidgetText::from(term_rich(term)).into_galley(
                ui,
                Some(TextWrapMode::Wrap),
                width.max(1.0),
                TextRole::Help.font(&t),
            )
        };
        // a value's words, its code or its dash, wrapping in `width`
        let lay_value = |ui: &Ui,
                         value: &FactValue,
                         width: f32|
         -> Option<(std::sync::Arc<egui::Galley>, egui::Color32)> {
            let (rich, color) = match value {
                FactValue::Text(text) => (egui::RichText::new(text).size(value_size), ink),
                FactValue::Code(code) => (
                    egui::RichText::new(code).font(crate::mono_font(ui.ctx(), crate::mono_size(&t, value_size), 400.0)),
                    ink,
                ),
                FactValue::Missing => (egui::RichText::new("\u{2014}").size(value_size), t.color("faint")),
                FactValue::Drawn(..) => return None,
            };
            let galley = WidgetText::from(rich.line_height(Some(line))).into_galley(
                ui,
                Some(TextWrapMode::Wrap),
                width.max(1.0),
                egui::FontId::proportional(value_size),
            );
            Some((galley, color))
        };
        let width = ui.available_width();
        let row_gap = if small { 4.0 } else { 8.0 };
        let two = self.two && !super::is_phone(ui.ctx());
        let term_ink = t.color("muted-foreground");
        let reaches = self.layout != DescriptionLayout::Row;
        // draw a value at `at`, `room` wide; returns its height
        let draw_value = |ui: &mut Ui, value: FactValue<'_>, at: egui::Pos2, room: f32| -> f32 {
            match value {
                FactValue::Drawn(draw, edited) => {
                    // an edited value's box reaches into the gap: its edge and padding, 11
                    let reach = if edited && reaches { 11.0 } else { 0.0 };
                    let at = pos2(at.x - reach, at.y);
                    let room = room + reach;
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(Rect::from_min_size(at, vec2(room.max(1.0), f32::INFINITY)))
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                    );
                    child.spacing_mut().interact_size.y = 0.0;
                    draw(&mut child);
                    child.min_rect().height()
                }
                other => {
                    let (galley, color) = lay_value(ui, &other, room).expect("a laid value");
                    let height = galley.size().y;
                    ui.painter().galley(at, galley, color);
                    height
                }
            }
        };
        match self.layout {
            DescriptionLayout::Row => {
                // the pairs in a line that wraps: 20 apart (sm 16), 4 between lines (sm 2), a term
                // and its value 6 apart
                let (gap, line_gap) = if small { (16.0, 2.0) } else { (20.0, 4.0) };
                let top = ui.cursor().top();
                let left = ui.cursor().left();
                let (mut x, mut y, mut tallest) = (0.0_f32, 0.0_f32, 0.0_f32);
                for fact in self.items {
                    let term = WidgetText::from(term_rich(&fact.term)).into_galley(
                        ui,
                        Some(TextWrapMode::Extend),
                        f32::INFINITY,
                        TextRole::Help.font(&t),
                    );
                    let value_width = match &fact.value {
                        FactValue::Drawn(..) => (width - x - term.size().x - 6.0).max(width / 4.0),
                        other => lay_value(ui, other, f32::INFINITY).map_or(0.0, |(g, _)| g.size().x),
                    };
                    let pair = term.size().x + 6.0 + value_width;
                    if x > 0.0 && x + pair > width {
                        x = 0.0;
                        y += tallest + line_gap;
                        tallest = 0.0;
                    }
                    let at = pos2(left + x, top + y);
                    let term_width = term.size().x;
                    let term_height = term.size().y;
                    ui.painter().galley(at, term, term_ink);
                    let room = match fact.value {
                        FactValue::Drawn(..) => (width - x - term_width - 6.0).max(1.0),
                        _ => value_width + 1.0,
                    };
                    let height = draw_value(ui, fact.value, pos2(at.x + term_width + 6.0, at.y), room);
                    tallest = tallest.max(height).max(term_height);
                    x += pair + gap;
                }
                ui.allocate_space(vec2(width, y + tallest));
            }
            DescriptionLayout::Stacked => {
                let column_gap = if small { 16.0 } else { 24.0 };
                let columns = if two { 2 } else { 1 };
                let column = (width - (columns - 1) as f32 * column_gap) / columns as f32;
                let left = ui.cursor().left();
                let mut y = ui.cursor().top();
                let mut items = self.items.into_iter().peekable();
                let mut first = true;
                while items.peek().is_some() {
                    // a wide fact spans the row; else up to `columns` side by side
                    let mut row = Vec::new();
                    while row.len() < columns
                        && let Some(next) = items.peek()
                    {
                        if next.wide && !row.is_empty() {
                            break;
                        }
                        let fact = items.next().expect("peeked");
                        let wide = fact.wide;
                        row.push(fact);
                        if wide {
                            break;
                        }
                    }
                    if !first {
                        y += row_gap;
                    }
                    first = false;
                    let mut tallest = 0.0_f32;
                    for (index, fact) in row.into_iter().enumerate() {
                        let room = if fact.wide { width } else { column };
                        let x = left + index as f32 * (column + column_gap);
                        let term = lay_term(ui, &fact.term, room);
                        let term_height = term.size().y;
                        ui.painter().galley(pos2(x, y), term, term_ink);
                        let height = draw_value(ui, fact.value, pos2(x, y + term_height + 2.0), room);
                        tallest = tallest.max(term_height + 2.0 + height);
                    }
                    y += tallest;
                }
                let used = (y - ui.cursor().top()).max(0.0);
                ui.allocate_space(vec2(width, used));
            }
            DescriptionLayout::Side => {
                let pair_gap = if small { 8.0 } else { 12.0 };
                let column_gap = if small { 16.0 } else { 24.0 };
                let columns = if two { 2 } else { 1 };
                let pair_width = (width - (columns - 1) as f32 * column_gap) / columns as f32;
                // the term column: as given, or as wide as the longest term, at most two fifths
                let term_width = self.term_width.unwrap_or_else(|| {
                    self.items
                        .iter()
                        .filter(|f| !f.wide)
                        .map(|f| {
                            WidgetText::from(term_rich(&f.term))
                                .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextRole::Help.font(&t))
                                .size()
                                .x
                                .ceil()
                        })
                        .fold(0.0, f32::max)
                        .min(width * 0.4)
                });
                let value_width = pair_width - term_width - pair_gap;
                let left = ui.cursor().left();
                let mut y = ui.cursor().top();
                let mut items = self.items.into_iter().peekable();
                let mut first = true;
                while items.peek().is_some() {
                    let mut row = Vec::new();
                    while row.len() < columns
                        && let Some(next) = items.peek()
                    {
                        if next.wide && !row.is_empty() {
                            break;
                        }
                        let fact = items.next().expect("peeked");
                        let wide = fact.wide;
                        row.push(fact);
                        if wide {
                            break;
                        }
                    }
                    if !first {
                        y += row_gap;
                    }
                    first = false;
                    let mut tallest = 0.0_f32;
                    for (index, fact) in row.into_iter().enumerate() {
                        let x = left + index as f32 * (pair_width + column_gap);
                        if fact.wide {
                            // its value under its term, across the full width
                            let term = lay_term(ui, &fact.term, width);
                            let term_height = term.size().y;
                            ui.painter().galley(pos2(x, y), term, term_ink);
                            let height = draw_value(ui, fact.value, pos2(x, y + term_height + 2.0), width);
                            tallest = tallest.max(term_height + 2.0 + height);
                            continue;
                        }
                        // the term wraps in its column, its first line level with the value's
                        let term = lay_term(ui, &fact.term, term_width);
                        let term_height = term.size().y;
                        let value_x = x + term_width + pair_gap;
                        let drawn = matches!(fact.value, FactValue::Drawn(..));
                        let height = draw_value(ui, fact.value, pos2(value_x, y), value_width);
                        let term_top = if drawn {
                            // a control's first line: its middle, or a control's middle when taller
                            let middle = (height / 2.0).min(t.density("control") / 2.0);
                            y + (middle - line / 2.0).max(0.0)
                        } else {
                            y
                        };
                        ui.painter().galley(pos2(x, term_top), term, term_ink);
                        tallest = tallest.max(height).max(term_top - y + term_height);
                    }
                    y += tallest;
                }
                let used = (y - ui.cursor().top()).max(0.0);
                ui.allocate_space(vec2(width, used));
            }
        }
    }
}
