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

/// Why what is typed is refused, or none to take it.
pub type Refuse<'a> = Box<dyn Fn(&str) -> Option<String> + 'a>;

/// A field that adds what is typed into it on Enter, a plus at its start: the quickest way to add a
/// row. Returns the text added this frame, trimmed, and empties itself.
pub struct QuickAdd<'a> {
    text: &'a mut String,
    placeholder: String,
    width: f32,
    id: egui::Id,
    validate: Option<Refuse<'a>>,
}

impl<'a> QuickAdd<'a> {
    pub fn new(id: impl egui::AsIdSalt, text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: "Add".into(),
            width: 256.0,
            id: egui::Id::unique(id),
            validate: None,
        }
    }

    /// Refuses what is typed with why, as the web's `validate`: the field keeps the words, its edge
    /// turns critical and a mark at its end says why while it is pointed at; typing clears it.
    pub fn validate(mut self, validate: impl Fn(&str) -> Option<String> + 'a) -> Self {
        self.validate = Some(Box::new(validate));
        self
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
    pub fn field_id(id: impl egui::AsIdSalt) -> egui::Id {
        egui::Id::unique(id).with("field")
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
        let error_id = self.id.with("refused");
        if response.changed() {
            ui.data_mut(|d| d.remove::<String>(error_id));
        }
        let entered = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let added = self.text.trim().to_owned();
        let mut out = None;
        if entered && !added.is_empty() {
            response.request_focus();
            match self.validate.as_ref().and_then(|v| v(&added)) {
                Some(refusal) => {
                    ui.data_mut(|d| d.insert_temp(error_id, refusal));
                }
                None => {
                    ui.data_mut(|d| d.remove::<String>(error_id));
                    self.text.clear();
                    out = Some(added);
                }
            }
        }
        if let Some(refusal) = ui.data(|d| d.get_temp::<String>(error_id)) {
            // the critical edge, and the mark at the end that says why
            ui.painter().rect_stroke(
                rect,
                t.radius("control"),
                Stroke::new(1.0, t.color("critical")),
                egui::StrokeKind::Inside,
            );
            let mark = Rect::from_center_size(pos2(rect.right() - 18.0, rect.center().y), vec2(20.0, 20.0));
            let hit = ui.interact(mark, self.id.with("mark"), Sense::hover());
            icons::paint(
                ui.painter(),
                mark.center(),
                Icon::CircleAlert,
                16.0,
                t.color("critical-ink"),
            );
            hit.on_hover_text(refusal);
        }
        out
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
    // centered in the floor egui 0.37 sets a text edit's height to
    let edit = TextEdit::singleline(text)
        .align(egui::Align2::LEFT_CENTER)
        .id(id)
        .font(body.font(&t))
        .text_color(t.color("foreground"))
        .frame(Frame::NONE)
        .margin(Margin::ZERO)
        .desired_width(inner.width())
        .hint_text(body.rich(&t, placeholder).color(t.color("muted-foreground")));
    let response = child.add(edit).accessible_name(placeholder);
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

/// A search field: a magnifier, the words searched for, and a clear button once there is text,
/// filling the width it is given.
pub struct SearchField<'a> {
    text: &'a mut String,
    placeholder: String,
    id: egui::Id,
    keys: Option<String>,
    /// Its width; the room it is given down a column, and 256 in a row such as a toolbar.
    width: Option<f32>,
    /// The clear button's name.
    words_clear: String,
}

impl<'a> SearchField<'a> {
    pub fn new(id: impl egui::AsIdSalt, text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: "Search".into(),
            id: egui::Id::unique(id),
            keys: None,
            width: None,
            words_clear: "Clear the search".into(),
        }
    }

    /// The clear button's name, said to a screen reader and on hover.
    pub fn clear_word(mut self, word: impl Into<String>) -> Self {
        self.words_clear = word.into();
        self
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
        // the clear button once there is text: an X of 14 at the field's end, muted, the foreground
        // under the pointer
        if !self.text.is_empty() {
            let at = Rect::from_center_size(pos2(rect.right() - 10.0 - 10.0, rect.center().y), vec2(20.0, 20.0));
            let hit = ui.interact(at, self.id.with("clear"), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words_clear));
            let ink = if hit.hovered() {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            icons::paint(ui.painter(), at.center(), Icon::X, 14.0, ink);
            if hit.clicked() {
                self.text.clear();
                response.request_focus();
            }
        }
        if let Some(keys) = &self.keys {
            if self.text.is_empty() && !response.has_focus() {
                let mono = crate::mono_font(
                    ui.ctx(),
                    crate::mono_small_size(&t, TextRole::Help.font(&t).size),
                    400.0,
                );
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
    let id = ui.scope_id().with("search");
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
            let help = ui.interact(spot, ui.scope_id().with(("switch-help", &self.label)), Sense::hover());
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
            DueState::Overdue => (t.color("critical-ink"), t.weight(crate::Weight::Medium)),
            DueState::Near => (t.color("warning-ink"), t.weight(crate::Weight::Medium)),
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
        self.show_with(ui, |_| {}, actions)
    }

    /// Draws the band with `start` before the note, kept apart from the verbs at the end, as the
    /// web's `start`: a form's Clear and Fill.
    pub fn show_with<R>(
        self,
        ui: &mut egui::Ui,
        start: impl FnOnce(&mut egui::Ui),
        actions: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
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
        let mut lead = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(Layout::left_to_right(Align::Center)),
        );
        lead.spacing_mut().item_spacing.x = 8.0;
        start(&mut lead);
        let started = lead.min_rect();
        let note_left = if started.width() > 0.0 {
            started.right() + 8.0
        } else {
            row.left()
        };
        if let Some(note) = &self.note {
            let room = (child.min_rect().left() - note_left - 8.0).max(1.0);
            let galley = egui::WidgetText::from(crate::TextRole::Body.rich(&t, note)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                room,
                crate::TextRole::Body.font(&t),
            );
            ui.painter().galley(
                egui::pos2(note_left, row.center().y - galley.size().y / 2.0),
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

/// The words a save bar says.
#[derive(Clone, Debug)]
pub struct SaveBarWords {
    /// How many edits are unsaved, `{count}` standing for the count: one and more.
    pub unsaved_one: String,
    pub unsaved_other: String,
    pub discard: String,
    pub save: String,
    pub saving: String,
}

impl Default for SaveBarWords {
    fn default() -> Self {
        Self {
            unsaved_one: "1 unsaved change".into(),
            unsaved_other: "{count} unsaved changes".into(),
            discard: "Discard".into(),
            save: "Save".into(),
            saving: "Saving".into(),
        }
    }
}

/// What a save bar was asked this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SaveBarResponse {
    pub save: bool,
    pub discard: bool,
}

/// The bar at the foot of a page while its edits are not yet saved, after sokkel-ui's `SaveBar`:
/// a [`FormBar`] with a dot in the warning color and how many edits there are at its start,
/// Discard and Save at its end, both held while a save is on its way. With no edits it draws
/// nothing.
pub struct SaveBar {
    count: usize,
    busy: bool,
    words: SaveBarWords,
}

impl SaveBar {
    pub fn new(count: usize) -> Self {
        Self {
            count,
            busy: false,
            words: SaveBarWords::default(),
        }
    }

    /// A save is on its way: Save says Saving, and neither button takes a press.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    pub fn words(mut self, words: SaveBarWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> SaveBarResponse {
        let mut out = SaveBarResponse::default();
        if self.count == 0 {
            return out;
        }
        let t = crate::tokens(ui.ctx());
        let said = if self.count == 1 {
            self.words.unsaved_one.clone()
        } else {
            self.words.unsaved_other.replace("{count}", &self.count.to_string())
        };
        FormBar::new().show_with(
            ui,
            |ui| {
                let (dot, _) = ui.allocate_exact_size(vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.0, t.color("warning"));
                ui.label(crate::TextRole::Body.rich(&t, &said).color(t.color("foreground")));
            },
            |ui| {
                let save = super::Button::new(if self.busy {
                    &self.words.saving
                } else {
                    &self.words.save
                })
                .size(super::ButtonSize::Sm)
                .enabled(!self.busy);
                out.save = ui.add(save).clicked();
                let discard = super::Button::new(&self.words.discard)
                    .size(super::ButtonSize::Sm)
                    .variant(super::ButtonVariant::Ghost)
                    .enabled(!self.busy);
                out.discard = ui.add(discard).clicked();
            },
        );
        out
    }
}

/// The heights of the four parts of the fields in one row of a fields grid: the label line, the
/// description, the control and the help, so the fields beside each other line up.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FieldRow {
    pub parts: [f32; 4],
}

/// Where the fields of the row being drawn read their parts' heights, and report their own.
pub(crate) fn field_row_id() -> egui::Id {
    egui::Id::unique("sokkel-field-row")
}

/// A label over a control, after sokkel-ui's `Field`: the label with a required mark and a hint
/// beside it, a description under it, the control, and help under that. An error edges the control
/// in the critical ink and puts a mark at its end that says the error on hover. In a fields grid
/// (`Columns::fields`) the four parts line up with the fields beside it.
pub struct Field {
    label: String,
    description: Option<String>,
    help: Option<String>,
    hint: Option<String>,
    required: bool,
    error: Option<String>,
    issues: Vec<(super::IssueTone, String)>,
    words: FieldWords,
    help_ui: Option<super::section::Tools<'static>>,
}

/// The words a field's issue mark says before each issue, for a screen reader.
#[derive(Clone, Debug)]
pub struct FieldWords {
    pub critical: String,
    pub warning: String,
}

impl Default for FieldWords {
    fn default() -> Self {
        Self {
            critical: "Error: ".into(),
            warning: "Warning: ".into(),
        }
    }
}

impl Field {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            help: None,
            hint: None,
            required: false,
            error: None,
            issues: Vec::new(),
            words: FieldWords::default(),
            help_ui: None,
        }
    }

    /// What is wrong with the value, each in its tone, as the web's `issues`: the control edged in
    /// the worst tone and the issue mark at its end, which opens the list. An `error` is listed
    /// first, as a critical issue.
    pub fn issues(mut self, issues: impl IntoIterator<Item = (super::IssueTone, String)>) -> Self {
        self.issues = issues.into_iter().collect();
        self
    }

    pub fn words(mut self, words: FieldWords) -> Self {
        self.words = words;
        self
    }

    /// Help drawn by the caller under the control, in the help's type and ink, such as a reason
    /// with its formula.
    pub fn help_ui(mut self, help: impl FnOnce(&mut egui::Ui) + 'static) -> Self {
        self.help_ui = Some(Box::new(help));
        self
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

    pub fn help_if(mut self, help: Option<impl Into<String>>) -> Self {
        self.help = help.map(Into::into);
        self
    }

    /// A short word after the label in the faint ink, such as a unit.
    pub fn hint(mut self, hint: Option<impl Into<String>>) -> Self {
        self.hint = hint.map(Into::into);
        self
    }

    /// A star after the label in the critical ink.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// What is wrong with the value: the control edged in the critical ink, a mark at its end.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    pub fn show<R>(mut self, ui: &mut egui::Ui, control: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let t = crate::tokens(ui.ctx());
        let row: Option<(egui::Id, FieldRow)> = ui.data(|d| d.get_temp(field_row_id()));
        let lined = row.is_some();
        let (row_id, wanted) = row.unwrap_or((egui::Id::NULL, FieldRow::default()));
        let mut had = [0.0_f32; 4];
        let out = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = if lined { 0.0 } else { 6.0 };
                // the label line: the label, the star, and the hint 8 after them on its baseline
                let top = ui.cursor().top();
                // the label line as one text: the label, the star, and the hint after them
                let mut line = egui::text::LayoutJob::default();
                let style = ui.style().clone();
                crate::TextRole::Label
                    .rich(&t, &self.label)
                    .color(t.color("foreground"))
                    .append_to(&mut line, &style, egui::FontSelection::Default, egui::Align::BOTTOM);
                if self.required {
                    crate::TextRole::Label
                        .rich(&t, " *")
                        .color(t.color("critical-ink"))
                        .append_to(&mut line, &style, egui::FontSelection::Default, egui::Align::BOTTOM);
                }
                if let Some(hint) = &self.hint {
                    // 8 of room before the hint, as the web's `gap-x-2`
                    crate::TextRole::Help
                        .rich(&t, format!("\u{2002}\u{2009}{hint}"))
                        .color(t.color("faint"))
                        .append_to(&mut line, &style, egui::FontSelection::Default, egui::Align::BOTTOM);
                }
                ui.label(line);
                had[0] = ui.cursor().top() - top;
                if lined && wanted.parts[0] > had[0] {
                    ui.add_space(wanted.parts[0] - had[0]);
                }
                let top = ui.cursor().top();
                if let Some(description) = &self.description {
                    // 4 under the label, as the web's description sits
                    ui.add_space(if lined { 6.0 } else { -2.0 });
                    ui.add(
                        egui::Label::new(
                            crate::TextRole::Body
                                .rich(&t, description)
                                .color(t.color("muted-foreground")),
                        )
                        .wrap(),
                    );
                }
                had[1] = ui.cursor().top() - top;
                if lined && wanted.parts[1] > had[1] {
                    ui.add_space(wanted.parts[1] - had[1]);
                }
                let top = ui.cursor().top();
                if lined {
                    ui.add_space(6.0);
                }
                let inner = ui.scope(|ui| control(ui));
                if !self.issues.is_empty() {
                    use super::schema_form::mark::{MarkWords, Shown, issue_mark, paint_edge, worst};
                    let shown: Vec<Shown> = self
                        .error
                        .iter()
                        .map(|text| (super::IssueTone::Critical, text.clone()))
                        .chain(self.issues.iter().cloned())
                        .map(|(tone, text)| Shown {
                            tone,
                            text,
                            detail: None,
                            fix: None,
                        })
                        .collect();
                    let rect = inner.response.rect;
                    // a control across the field's width (an input, an editor) wears the edge; the
                    // mark stands at the field's end either way
                    let end = ui.max_rect().right();
                    if let Some(tone) = worst(&shown)
                        && rect.right() >= end - 1.0
                    {
                        paint_edge(ui, &t, rect, tone);
                    }
                    let side = t.density("control-sm");
                    let spot = egui::Rect::from_center_size(
                        egui::pos2(end - 4.0 - side / 2.0, rect.center().y),
                        egui::vec2(side, side),
                    );
                    let id = ui.scope_id().with(("issues", &self.label));
                    let open: bool = ui.data(|d| d.get_temp(id)).unwrap_or(false);
                    let words = MarkWords {
                        critical: &self.words.critical,
                        warning: &self.words.warning,
                        use_fix: |_| String::new(),
                    };
                    let out = issue_mark(ui, id, spot, &shown, open, &words);
                    let now = if out.toggled { !open } else { open && !out.closed };
                    if now != open {
                        ui.data_mut(|d| d.insert_temp(id, now));
                    }
                } else if let Some(error) = &self.error {
                    let rect = inner.response.rect;
                    let control_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.left(), rect.top()),
                        egui::pos2(
                            rect.right(),
                            rect.top() + rect.height().min(t.density("control").max(32.0)),
                        ),
                    );
                    ui.painter().rect_stroke(
                        control_rect,
                        t.radius("control"),
                        egui::Stroke::new(1.0, t.color("critical")),
                        egui::StrokeKind::Inside,
                    );
                    let mark = egui::Rect::from_center_size(
                        egui::pos2(control_rect.right() - 18.0, control_rect.center().y),
                        egui::vec2(20.0, 20.0),
                    );
                    let hit = ui.interact(mark, ui.scope_id().with(("issue", &self.label)), egui::Sense::hover());
                    hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, format!("Issue: {error}")));
                    crate::icons::paint(
                        ui.painter(),
                        mark.center(),
                        crate::Icon::CircleAlert,
                        16.0,
                        t.color("critical-ink"),
                    );
                    hit.on_hover_text(error.clone());
                }
                had[2] = ui.cursor().top() - top;
                if lined && wanted.parts[2] > had[2] {
                    ui.add_space(wanted.parts[2] - had[2]);
                }
                let top = ui.cursor().top();
                if let Some(help) = &self.help {
                    if lined {
                        ui.add_space(6.0);
                    }
                    ui.add(
                        egui::Label::new(crate::TextRole::Help.rich(&t, help).color(t.color("muted-foreground")))
                            .wrap(),
                    );
                }
                if let Some(help) = self.help_ui.take() {
                    if lined {
                        ui.add_space(6.0);
                    }
                    help(ui);
                }
                had[3] = ui.cursor().top() - top;
                if lined && wanted.parts[3] > had[3] {
                    ui.add_space(wanted.parts[3] - had[3]);
                }
                inner.inner
            })
            .inner;
        if lined {
            // what this field took of each part, for the next frame to line the row up to
            let key = row_id.with("measured");
            ui.data_mut(|d| {
                let mut seen: FieldRow = d.get_temp(key).unwrap_or_default();
                for (part, height) in had.iter().enumerate() {
                    seen.parts[part] = seen.parts[part].max(*height);
                }
                d.insert_temp(key, seen);
            });
        }
        out
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
