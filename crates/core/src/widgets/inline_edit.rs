//! A value edited where it stands, after sokkel-ui's `InlineEdit`.

use egui::{Align, Frame, Id, Key, Layout, Margin, Rect, Sense, Stroke, TextEdit, Ui, UiBuilder, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// How an inline edit is drawn: a field's value in body text, or a page title in its type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InlineEditLook {
    #[default]
    Field,
    Title,
}

/// The words an inline edit says.
#[derive(Clone, Debug)]
pub struct InlineEditWords {
    /// What pressing the value does, said to a screen reader after its label.
    pub edit: String,
    /// Drawn for a value that is empty.
    pub empty: String,
}

impl Default for InlineEditWords {
    fn default() -> Self {
        Self {
            edit: "Edit".into(),
            empty: "Empty".into(),
        }
    }
}

/// What refuses a value, with the message said.
type Validate = Box<dyn Fn(&str) -> Option<String>>;

/// Where an inline edit stands between frames.
#[derive(Clone, Debug, Default)]
struct Editing {
    draft: String,
    /// Why the last draft was refused, said by the mark at the editor's end.
    error: Option<String>,
    /// The draft that refusal was about, so leaving it unchanged does not ask again.
    refused: Option<String>,
    fresh: bool,
}

/// A line of text shown as text that turns into an editor on a press or F2, and back once saved,
/// as sokkel-ui's `InlineEdit`. Enter saves, Escape drops what was typed, and leaving the editor
/// saves it; an unchanged value closes with no save. `validate` refuses a value with a message:
/// the editor stays open on it, its rule critical and the message at its end, until the text
/// changes. `title` draws the value and its editor in a page title's type with only a rule under
/// the editor, so nothing moves as it opens.
pub struct InlineEdit {
    id: Id,
    label: String,
    value: String,
    look: InlineEditLook,
    validate: Option<Validate>,
    disabled: bool,
    words: InlineEditWords,
}

impl InlineEdit {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: Id::new(("sokkel-inline-edit", id)),
            label: label.into(),
            value: value.into(),
            look: InlineEditLook::Field,
            validate: None,
            disabled: false,
            words: InlineEditWords::default(),
        }
    }

    /// In a page title's type, as the web's `variant="title"`.
    pub fn title(mut self) -> Self {
        self.look = InlineEditLook::Title;
        self
    }

    /// Checks a value before it is saved: a message refuses it.
    pub fn validate(mut self, validate: impl Fn(&str) -> Option<String> + 'static) -> Self {
        self.validate = Some(Box::new(validate));
        self
    }

    /// Shown read only, with no way to edit it.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn words(mut self, words: InlineEditWords) -> Self {
        self.words = words;
        self
    }

    /// Draw it; the value saved this frame, if one was.
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        let t = tokens(ui.ctx());
        let title = self.look == InlineEditLook::Title;
        let role = if title { TextRole::PageTitle } else { TextRole::Body };
        let ink = if title {
            t.color_or("page-title-ink", t.color("foreground"))
        } else {
            t.color("foreground")
        };
        // a title has no box, only its rule; a field keeps a control's height, padded across
        let (pad_x, pad_y, rule) = if title { (0.0, 2.0, 2.0) } else { (10.0, 4.0, 1.0) };
        let mut editing: Option<Editing> = ui.data(|d| d.get_temp(self.id));
        if self.disabled {
            editing = None;
        }

        let Some(mut open) = editing else {
            // at rest: the value, and a pencil on pointing
            let empty = self.value.is_empty();
            let shown = if empty { &self.words.empty } else { &self.value };
            let galley = egui::WidgetText::from(role.rich(&t, shown)).into_galley(
                ui,
                Some(if title {
                    egui::TextWrapMode::Wrap
                } else {
                    egui::TextWrapMode::Truncate
                }),
                (ui.available_width() - 2.0 * pad_x - if self.disabled { 0.0 } else { 8.0 + 16.0 }).max(1.0),
                role.font(&t),
            );
            let pencil = if self.disabled {
                0.0
            } else {
                8.0 + if title { 16.0 } else { 14.0 }
            };
            let width = if title {
                galley.size().x + pencil
            } else {
                ui.available_width()
            };
            let height = (galley.size().y + 2.0 * pad_y + rule).max(if title { 0.0 } else { t.density("control") });
            let sense = if self.disabled { Sense::hover() } else { Sense::click() };
            let (rect, response) = ui.allocate_exact_size(vec2(width, height), sense);
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    !self.disabled,
                    format!("{} {}", self.label, self.words.edit),
                )
            });
            let hovered = response.hovered() && !self.disabled;
            if hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                if title {
                    ui.painter().hline(
                        rect.x_range(),
                        rect.bottom() - rule / 2.0,
                        Stroke::new(rule, t.color("input")),
                    );
                } else {
                    ui.painter().rect(
                        rect,
                        t.radius("control"),
                        t.color("hover"),
                        Stroke::new(1.0, t.color("input")),
                        egui::StrokeKind::Inside,
                    );
                }
            }
            let text_at = pos2(
                rect.left() + pad_x,
                rect.top()
                    + pad_y
                    + if title {
                        0.0
                    } else {
                        (height - 2.0 * pad_y - rule - galley.size().y) / 2.0
                    },
            );
            let text_rect = Rect::from_min_size(text_at, galley.size());
            let ink = if empty { t.color("muted-foreground") } else { ink };
            ui.painter().galley(text_at, galley, ink);
            if hovered || (response.has_focus() && !self.disabled) {
                let size = if title { 16.0 } else { 14.0 };
                icons::paint(
                    ui.painter(),
                    pos2(text_rect.right() + 8.0 + size / 2.0, text_rect.center().y),
                    Icon::Pencil,
                    size,
                    t.color("muted-foreground"),
                );
            }
            super::focus_ring(ui.painter(), &t, rect, t.radius("control"), &response);
            let f2 = response.has_focus() && ui.input(|i| i.key_pressed(Key::F2));
            if (response.clicked() || f2) && !self.disabled {
                ui.data_mut(|d| {
                    d.insert_temp(
                        self.id,
                        Editing {
                            draft: self.value.clone(),
                            fresh: true,
                            ..Default::default()
                        },
                    )
                });
                ui.ctx().request_repaint();
            }
            return None;
        };

        // open: the editor in the value's place, a rule under it, a refusal's mark at its end
        let fresh = std::mem::take(&mut open.fresh);
        let font = role.font(&t);
        let line = t.text(role.key()).line_height;
        let typed = ui
            .painter()
            .layout_no_wrap(open.draft.clone(), font.clone(), ink)
            .size()
            .x;
        let mark = if open.error.is_some() { 8.0 + 16.0 } else { 0.0 };
        let width = if title {
            (typed + 4.0).max(64.0).min(ui.available_width() - mark)
        } else {
            ui.available_width()
        };
        let height = (line + 2.0 * pad_y + rule).max(if title { 0.0 } else { t.density("control") });
        let (rect, _) = ui.allocate_exact_size(vec2(width + mark, height), Sense::hover());
        let field = Rect::from_min_size(rect.min, vec2(width, height));
        let edge = if open.error.is_some() {
            t.color("critical")
        } else {
            t.color("ring")
        };
        if title {
            ui.painter()
                .hline(field.x_range(), field.bottom() - rule / 2.0, Stroke::new(rule, edge));
        } else {
            ui.painter().rect(
                field,
                t.radius("control"),
                t.color("background"),
                Stroke::new(1.0, edge),
                egui::StrokeKind::Inside,
            );
        }
        let inner = Rect::from_min_max(
            pos2(field.left() + pad_x, field.top() + pad_y),
            pos2(field.right() - pad_x, field.bottom() - pad_y - rule),
        );
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let editor_id = self.id.with("editor");
        let before = open.draft.clone();
        let response = child.add(
            TextEdit::singleline(&mut open.draft)
                .id(editor_id)
                .font(font)
                .text_color(ink)
                .frame(Frame::NONE)
                .margin(Margin::ZERO)
                .desired_width(inner.width()),
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, &self.label));
        if fresh {
            response.request_focus();
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
            let end = egui::text::CCursor::new(open.draft.chars().count());
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), end)));
            state.store(ui.ctx(), editor_id);
        }
        // typing clears a refusal
        if open.draft != before {
            open.error = None;
        }
        if let Some(error) = &open.error {
            let spot = Rect::from_center_size(pos2(field.right() + 8.0 + 8.0, field.center().y), vec2(16.0, 16.0));
            let hit = ui.interact(spot, self.id.with("issue"), Sense::hover());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, error));
            icons::paint(
                ui.painter(),
                spot.center(),
                Icon::CircleAlert,
                16.0,
                t.color("critical"),
            );
            // the refusal is said at once, as the web opens its mark
            crate::tooltip_shown(hit, error.clone());
        }

        let mut saved = None;
        // egui lets go of the focus on Escape before the editor runs, so a focus lost to it is Escape
        let escaped = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Escape));
        let entered = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let left = response.lost_focus() && !entered && !escaped;
        let close = if escaped {
            true
        } else if entered || left {
            let unchanged = open.draft == self.value && open.error.is_none();
            if unchanged {
                true
            } else if left && open.error.is_some() && open.refused.as_ref() == Some(&open.draft) {
                false
            } else if let Some(refusal) = self.validate.as_ref().and_then(|v| v(&open.draft)) {
                open.refused = Some(open.draft.clone());
                open.error = Some(refusal);
                response.request_focus();
                false
            } else {
                saved = Some(open.draft.clone());
                true
            }
        } else {
            false
        };
        ui.data_mut(|d| {
            if close {
                d.remove::<Editing>(self.id);
            } else {
                d.insert_temp(self.id, open);
            }
        });
        if close {
            ui.ctx().request_repaint();
        }
        saved
    }
}
