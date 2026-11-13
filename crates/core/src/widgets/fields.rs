//! Small parts a page's toolbar holds: the toolbar itself, a quick add, a switch with its label, and
//! the due date's text, after sokkel-ui's `Toolbar`, `QuickAdd`, `SwitchField` and `DueText`.

use egui::{Align, Frame, Layout, Margin, Rect, Response, Sense, Stroke, TextEdit, Ui, Widget, pos2, vec2};

use crate::{Day, Icon, Switch, TextRole, icons, tokens};

/// A row of a page's tools: `start` from the start, `end` laid out from the end, a control tall.
pub fn toolbar(ui: &mut Ui, start: impl FnOnce(&mut Ui), end: impl FnOnce(&mut Ui)) {
    let t = tokens(ui.ctx());
    let height = t.density("control");
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), height),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            start(ui);
            ui.with_layout(Layout::right_to_left(Align::Center), end);
        },
    );
}

/// A field that adds what is typed into it on Enter, a plus at its start: the quickest way to add a
/// row. Returns the text added this frame, trimmed, and empties itself.
pub struct QuickAdd<'a> {
    text: &'a mut String,
    placeholder: String,
    width: f32,
    id: egui::Id,
}

impl<'a> QuickAdd<'a> {
    pub fn new(id: impl egui::AsId, text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: "Add".into(),
            width: 256.0,
            id: egui::Id::new(id),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// The id of its text field, to move the focus to it from elsewhere (a command).
    pub fn field_id(id: impl egui::AsId) -> egui::Id {
        egui::Id::new(id).with("field")
    }

    pub fn show(self, ui: &mut Ui) -> Option<String> {
        let t = tokens(ui.ctx());
        let (rect, _) = ui.allocate_exact_size(vec2(self.width, t.density("control")), Sense::hover());
        let response = icon_field(
            ui,
            rect,
            self.text,
            Icon::Plus,
            &self.placeholder,
            Self::field_id(self.id),
        );
        let entered = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let added = self.text.trim().to_owned();
        if entered && !added.is_empty() {
            self.text.clear();
            response.request_focus();
            return Some(added);
        }
        None
    }
}

/// A one-line field with a glyph at its start, filling `rect`: the field's ground and edge, the
/// ring while focused, the text after the glyph.
fn icon_field(ui: &mut Ui, rect: Rect, text: &mut String, icon: Icon, placeholder: &str, id: egui::Id) -> Response {
    let t = tokens(ui.ctx());
    let body = TextRole::Body;
    let inner = Rect::from_min_max(
        pos2(rect.left() + 34.0, rect.top()),
        pos2(rect.right() - 10.0, rect.bottom()),
    );
    // the ground under the field, painted before the text it holds
    let ground = ui.painter().add(egui::Shape::Noop);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let edit = TextEdit::singleline(text)
        .id(id)
        .font(body.font(&t))
        .text_color(t.color("foreground"))
        .frame(Frame::NONE)
        .margin(Margin::ZERO)
        .desired_width(inner.width())
        .hint_text(body.rich(&t, placeholder).color(t.color("muted-foreground")));
    let response = child.add(edit);
    let focused = response.has_focus();
    ui.painter().set(
        ground,
        egui::Shape::rect_filled(rect, t.radius("control"), t.color("field")),
    );
    let edge = if focused { t.color("ring") } else { t.color("input") };
    ui.painter().rect_stroke(
        rect,
        t.radius("control"),
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    if focused {
        ui.painter().rect_stroke(
            rect,
            t.radius("control"),
            Stroke::new(1.0, t.color("ring")),
            egui::StrokeKind::Outside,
        );
    }
    icons::paint(
        ui.painter(),
        pos2(rect.left() + 18.0, rect.center().y),
        icon,
        16.0,
        t.color("muted-foreground"),
    );
    response
}

/// A search field: a magnifier, the words searched for, filling the width it is given.
pub struct SearchField<'a> {
    text: &'a mut String,
    placeholder: String,
    id: egui::Id,
}

impl<'a> SearchField<'a> {
    pub fn new(id: impl egui::AsId, text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: "Search".into(),
            id: egui::Id::new(id),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), t.density("control")), Sense::hover());
        icon_field(
            ui,
            rect,
            self.text,
            Icon::Search,
            &self.placeholder,
            self.id.with("field"),
        )
    }
}

/// The search field a part draws in the room it is given.
pub(crate) fn search_field(ui: &mut Ui, text: &mut String, placeholder: &str) -> Response {
    let id = ui.id().with("search");
    SearchField::new(id, text).placeholder(placeholder).show(ui)
}

/// A switch with its label before it, a control tall: a setting turned on and off in place.
#[must_use = "add it with `ui.add(...)`"]
pub struct SwitchField<'a> {
    on: &'a mut bool,
    label: String,
}

impl<'a> SwitchField<'a> {
    pub fn new(on: &'a mut bool, label: impl Into<String>) -> Self {
        Self {
            on,
            label: label.into(),
        }
    }
}

impl Widget for SwitchField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let label = egui::WidgetText::from(TextRole::Body.rich(&t, &self.label)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            TextRole::Body.font(&t),
        );
        // its size known before it is placed, so a toolbar's end can lay it out from the right
        let size = vec2(label.size().x + 16.0 + 32.0, t.density("control"));
        let (rect, hit) = ui.allocate_exact_size(size, Sense::click());
        ui.painter().galley(
            pos2(rect.left(), rect.center().y - label.size().y / 2.0),
            label,
            t.color("foreground"),
        );
        let place = Rect::from_min_size(pos2(rect.right() - 32.0, rect.center().y - 9.2), vec2(32.0, 18.4));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
        let mut response = child.add(Switch::new(self.on, self.label.clone()));
        if hit.clicked() && !response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        response
    }
}

/// Where a due date falls against today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DueState {
    Overdue,
    Near,
    Later,
}

/// Where `date` falls against `now`: before today overdue, today to `near_days` ahead near.
pub fn due_state(date: Day, now: Day, near_days: i32) -> DueState {
    let days = date.0 - now.0;
    if days < 0 {
        DueState::Overdue
    } else if days <= near_days {
        DueState::Near
    } else {
        DueState::Later
    }
}

/// A due date as text: `Nov 15`, in the critical ink and medium weight when overdue, the warning ink
/// when due today, muted later; `None` without a date. A done task's date is quiet.
#[must_use = "add it with `ui.add(...)`"]
pub struct DueText {
    date: Option<Day>,
    now: Day,
    done: bool,
    icon: bool,
    none: String,
}

impl DueText {
    pub fn new(date: Option<Day>, now: Day) -> Self {
        Self {
            date,
            now,
            done: false,
            icon: false,
            none: "None".into(),
        }
    }

    pub fn done(mut self, done: bool) -> Self {
        self.done = done;
        self
    }

    /// A calendar before the date, as a board's card shows it.
    pub fn icon(mut self, icon: bool) -> Self {
        self.icon = icon;
        self
    }

    pub(crate) fn rich(&self, t: &crate::Tokens) -> (egui::RichText, egui::Color32) {
        let state = match self.date {
            Some(date) if !self.done => due_state(date, self.now, 0),
            _ => DueState::Later,
        };
        let text = self.date.map_or(self.none.clone(), Day::short);
        let (ink, weight) = match state {
            DueState::Overdue => (t.color("critical-ink"), 500.0),
            DueState::Near => (t.color("warning-ink"), 500.0),
            DueState::Later => (t.color("muted-foreground"), 400.0),
        };
        (TextRole::Body.rich(t, text).variation("wght", weight).color(ink), ink)
    }
}

impl Widget for DueText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (rich, ink) = self.rich(&t);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if self.icon {
                let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                icons::paint(ui.painter(), rect.center(), Icon::CalendarDays, 14.0, ink);
            }
            let response = ui.label(rich);
            match self.date {
                Some(date) => response.on_hover_text(format!(
                    "{} {}",
                    if due_state(date, self.now, 0) == DueState::Overdue && !self.done {
                        "Overdue, was due"
                    } else {
                        "Due"
                    },
                    date.long()
                )),
                None => response,
            }
        })
        .inner
    }
}
