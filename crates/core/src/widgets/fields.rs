//! Small parts a page's toolbar holds: the toolbar itself, a quick add, a switch with its label, and
//! the due date's text, after sokkel-ui's `Toolbar`, `QuickAdd`, `SwitchField` and `DueText`.

use egui::{Align, Frame, Layout, Margin, Rect, Response, Sense, Stroke, TextEdit, Ui, Widget, pos2, vec2};

use crate::{Day, Icon, Switch, TextRole, icons, tokens};

/// A row of a page's tools: `start` from the start, `end` laid out from the end, a control tall.
pub fn toolbar(ui: &mut Ui, start: impl FnOnce(&mut Ui), end: impl FnOnce(&mut Ui)) {
    let t = tokens(ui.ctx());
    let height = t.density("control");
    // narrower than 28rem, as the web's `@md/toolbar`: the start across the whole row, and the end
    // on a row of its own under it, 8 down, at its end
    if ui.available_width() < 448.0 {
        let gap = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), height),
            Layout::left_to_right(Align::Center).with_main_justify(true),
            start,
        );
        // the page's own gap after the toolbar
        ui.spacing_mut().item_spacing.y = gap;
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), height),
            Layout::right_to_left(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                end(ui);
            },
        );
        return;
    }
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
        // across the row it is given where that row asks it to fill (a narrow toolbar)
        let width = if ui.layout().main_justify {
            ui.available_width()
        } else {
            self.width
        };
        let (rect, _) = ui.allocate_exact_size(vec2(width, t.density("control")), Sense::hover());
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
    keys: Option<String>,
    /// Its width; the room it is given down a column, and 256 in a row such as a toolbar.
    width: Option<f32>,
}

impl<'a> SearchField<'a> {
    pub fn new(id: impl egui::AsId, text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: "Search".into(),
            id: egui::Id::new(id),
            keys: None,
            width: None,
        }
    }

    /// A width of its own.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// The key that reaches the field, drawn at its end ("/"), and pressed while nothing else has
    /// the focus, moves the focus into it.
    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let height = super::finger_tall(ui.ctx(), &t, t.density("control"));
        let width = self.width.unwrap_or_else(|| {
            if ui.layout().is_horizontal() && !ui.layout().horizontal_justify() {
                256.0_f32.min(ui.available_width())
            } else {
                ui.available_width()
            }
        });
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let field = self.id.with("field");
        let response = icon_field(ui, rect, self.text, Icon::Search, &self.placeholder, field);
        if let Some(keys) = &self.keys {
            if self.text.is_empty() && !response.has_focus() {
                let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
                let galley = egui::WidgetText::from(egui::RichText::new(keys).font(mono.clone())).into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    mono,
                );
                ui.painter().galley(
                    pos2(
                        rect.right() - 10.0 - galley.size().x,
                        rect.center().y - galley.size().y / 2.0,
                    ),
                    galley,
                    t.color("muted-foreground"),
                );
            }
            let nothing_focused = ui.memory(|m| m.focused().is_none());
            let pressed = nothing_focused
                && ui.input(|i| {
                    i.events
                        .iter()
                        .any(|e| matches!(e, egui::Event::Text(typed) if typed == keys))
                });
            if pressed {
                ui.input_mut(|i| {
                    i.events
                        .retain(|e| !matches!(e, egui::Event::Text(typed) if typed == keys))
                });
                response.request_focus();
            }
        }
        response
    }
}

/// The search field a part draws in the room it is given.
pub(crate) fn search_field(ui: &mut Ui, text: &mut String, placeholder: &str) -> Response {
    let id = ui.id().with("search");
    SearchField::new(id, text).placeholder(placeholder).show(ui)
}

/// A switch with its label before it, a control tall: a setting turned on and off in place. In a
/// column it takes the row across, the switch at the right edge; in a row, its own width.
#[must_use = "add it with `ui.add(...)`"]
pub struct SwitchField<'a> {
    on: &'a mut bool,
    label: String,
    explain: Option<String>,
    enabled: bool,
}

impl<'a> SwitchField<'a> {
    pub fn new(on: &'a mut bool, label: impl Into<String>) -> Self {
        Self {
            on,
            label: label.into(),
            explain: None,
            enabled: true,
        }
    }

    /// What turning it on does, where the label alone does not say: a help mark beside the label,
    /// the words shown while it is pointed at.
    pub fn explain(mut self, explain: impl Into<String>) -> Self {
        self.explain = Some(explain.into());
        self
    }

    /// Whether it can be turned; off, the switch is dimmed and a press does nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
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
        // the help mark, 16 square 6 after the label
        let mark = if self.explain.is_some() { 6.0 + 16.0 } else { 0.0 };
        // its size known before it is placed, so a toolbar's end can lay it out from the right; in
        // a column it is the row across, the switch at its right edge, as the web's `justify-between`
        let least = label.size().x + mark + 16.0 + 32.0;
        let across = if ui.layout().main_dir().is_vertical() {
            ui.available_width().max(least)
        } else {
            least
        };
        let size = vec2(across, t.density("control"));
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let (rect, hit) = ui.allocate_exact_size(size, sense);
        let label_width = label.size().x;
        ui.painter().galley(
            pos2(rect.left(), rect.center().y - label.size().y / 2.0),
            label,
            crate::widgets::dim(t.color("foreground"), self.enabled),
        );
        if let Some(explain) = &self.explain {
            let spot = Rect::from_center_size(
                pos2(rect.left() + label_width + 6.0 + 8.0, rect.center().y),
                vec2(16.0, 16.0),
            );
            let help = ui.interact(spot, ui.id().with(("switch-help", &self.label)), Sense::hover());
            let ink = if help.hovered() {
                t.color("muted-foreground")
            } else {
                t.color("faint")
            };
            icons::paint(ui.painter(), spot.center(), Icon::CircleQuestionMark, 14.0, ink);
            help.on_hover_text(explain.as_str());
        }
        let place = Rect::from_min_size(pos2(rect.right() - 32.0, rect.center().y - 9.2), vec2(32.0, 18.4));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
        let mut response = child.add(Switch::new(self.on, self.label.clone()).enabled(self.enabled));
        if self.enabled && hit.clicked() && !response.clicked() {
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
    role: TextRole,
}

impl DueText {
    pub fn new(date: Option<Day>, now: Day) -> Self {
        Self {
            date,
            now,
            done: false,
            icon: false,
            none: "None".into(),
            role: TextRole::Body,
        }
    }

    /// The text role it is set in: the body's by default, the help's on a board's card.
    pub fn role(mut self, role: TextRole) -> Self {
        self.role = role;
        self
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
        (self.role.rich(t, text).variation("wght", weight).color(ink), ink)
    }
}

impl Widget for DueText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (rich, ink) = self.rich(&t);
        // a row as tall as its line, not a control's height
        let line = t.text(self.role.key()).line_height;
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), line),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
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
            },
        )
        .inner
    }
}

/// A form's actions at its end, after sokkel-ui's `FormBar` (`placement="end"`): a muted note at the
/// row's start, the actions at its end 8 apart, the row wrapping where they do not fit together.
pub fn form_bar<R>(ui: &mut egui::Ui, note: Option<&str>, actions: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let t = crate::tokens(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if let Some(note) = note {
            ui.label(crate::TextRole::Body.rich(&t, note).color(t.color("muted-foreground")));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| actions(ui))
            .inner
    })
    .inner
}

/// A label over any control, after sokkel-ui's `Field`: the label, a quiet hint beside it (such as
/// "on call" for the person a choice defaults to), and the control 6 under them.
pub fn field<R>(ui: &mut egui::Ui, label: &str, hint: Option<&str>, control: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let t = crate::tokens(ui.ctx());
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        // the label and its hint as one line of words, 6 apart, so the line is as tall as its words
        let mut job = egui::text::LayoutJob::default();
        crate::TextRole::Label
            .rich(&t, label)
            .color(t.color("foreground"))
            .append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::Center);
        if let Some(hint) = hint {
            job.append("", 6.0, egui::TextFormat::default());
            crate::TextRole::Help
                .rich(&t, hint)
                .color(t.color("muted-foreground"))
                .append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::Center);
        }
        ui.label(job);
        control(ui)
    })
    .inner
}

/// A form's actions in a band at the foot of its page, after sokkel-ui's `FormBar` with
/// `placement="foot"`: the shell's foot strip tall less its rule, on the `sidebar` ground under a
/// `border-strong` rule, a muted note at its start and the actions at its end, 8 apart, inset as
/// the page is. [`form_bar`] is the plain row closing a short form.
pub struct FormBar {
    note: Option<String>,
}

impl FormBar {
    pub fn new() -> Self {
        Self { note: None }
    }

    /// Muted words at the start, such as whether anything is unsaved.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// The band's height: the shell's foot strip less its rule.
    pub fn height(ctx: &egui::Context) -> f32 {
        crate::tokens(ctx).length("spacing-shell-foot") - 1.0
    }

    /// Draws the band across the room; `actions` are laid out from its end.
    pub fn show<R>(self, ui: &mut egui::Ui, actions: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let t = crate::tokens(ui.ctx());
        let height = Self::height(ui.ctx());
        let (band, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
        ui.painter().rect_filled(band, 0.0, t.color("sidebar"));
        ui.painter().hline(
            band.x_range(),
            band.top() + 0.5,
            egui::Stroke::new(1.0, t.color("border-strong")),
        );
        let inset = t.density("page-inset-wide");
        let row = band.shrink2(vec2(inset, 0.0));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(Layout::right_to_left(Align::Center)),
        );
        child.spacing_mut().item_spacing.x = 8.0;
        let out = actions(&mut child);
        if let Some(note) = &self.note {
            let room = (child.min_rect().left() - row.left() - 8.0).max(1.0);
            let galley = egui::WidgetText::from(crate::TextRole::Body.rich(&t, note)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                room,
                crate::TextRole::Body.font(&t),
            );
            ui.painter().galley(
                egui::pos2(row.left(), row.center().y - galley.size().y / 2.0),
                galley,
                t.color("muted-foreground"),
            );
        }
        out
    }
}

impl Default for FormBar {
    fn default() -> Self {
        Self::new()
    }
}

/// A label over a control with a line saying what it sets, after sokkel-ui's `Field` with
/// `description` (under the label, 4 down, muted) and `help` (under the control, small and muted).
pub struct Field {
    label: String,
    description: Option<String>,
    help: Option<String>,
}

impl Field {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            help: None,
        }
    }

    /// A sentence under the label saying what the control sets.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Small words under the control, such as what the chosen value means.
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn show<R>(self, ui: &mut egui::Ui, control: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let t = crate::tokens(ui.ctx());
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.label(
                crate::TextRole::Label
                    .rich(&t, &self.label)
                    .color(t.color("foreground")),
            );
            if let Some(description) = &self.description {
                ui.add_space(-2.0);
                ui.add(
                    egui::Label::new(
                        crate::TextRole::Body
                            .rich(&t, description)
                            .color(t.color("muted-foreground")),
                    )
                    .wrap(),
                );
            }
            let out = control(ui);
            if let Some(help) = &self.help {
                ui.label(crate::TextRole::Help.rich(&t, help).color(t.color("muted-foreground")));
            }
            out
        })
        .inner
    }
}

/// A [`toolbar`] as a strip across the top of a pane, as the web's `Toolbar` with
/// `variant="pane"`: `panel-pad` across, 8 over and under, a rule under it.
pub fn pane_toolbar(ui: &mut Ui, start: impl FnOnce(&mut Ui), end: impl FnOnce(&mut Ui)) {
    let t = tokens(ui.ctx());
    let pad = t.density("panel-pad") as i8;
    let strip = Frame::new()
        .inner_margin(Margin::symmetric(pad, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            toolbar(ui, start, end);
        })
        .response
        .rect;
    ui.painter().hline(
        strip.x_range(),
        strip.bottom() - 0.5,
        Stroke::new(1.0, t.color("border")),
    );
}
