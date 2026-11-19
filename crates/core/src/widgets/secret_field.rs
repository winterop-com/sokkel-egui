//! A field for a secret (a token, a key, a password), after sokkel-ui's `SecretField` (its spec,
//! `parts/secret-field`): masked while typed with a reveal, and sealed once a secret is set, so the
//! secret already set is never read back.

use egui::{Key, Rect, Response, Sense, Stroke, TextEdit, Ui, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// The words a secret field says, beside its field's own.
#[derive(Clone, Debug)]
pub struct SecretFieldWords {
    /// The note of a sealed field set at no time given.
    pub set: String,
    /// The note of a sealed field, `{when}` when it was set, relative to now: `Set 2 days ago`.
    pub set_when: String,
    pub replace: String,
    /// Replace's name, `{label}` the field's label: `Replace API token`.
    pub replace_label: String,
    pub cancel: String,
    pub show: String,
    pub hide: String,
}

impl Default for SecretFieldWords {
    fn default() -> Self {
        Self {
            set: "Set".into(),
            set_when: "Set {when}".into(),
            replace: "Replace".into(),
            replace_label: "Replace {label}".into(),
            cancel: "Cancel".into(),
            show: "Show".into(),
            hide: "Hide".into(),
        }
    }
}

/// What a secret field keeps between frames: whether a sealed field is open for a new secret,
/// whether the secret typed shows, and where the focus goes next.
#[derive(Clone, Copy, Debug, Default)]
struct Held {
    replacing: Option<bool>,
    revealed: bool,
    focus_input: bool,
    focus_replace: bool,
}

/// A field for a secret: its label (with a required mark, a hint), the input masked as it is typed
/// in the code face, a reveal at its end, and help or an issue under it. Sealed (`sealed`, a secret
/// already set), it shows a key, eight dots and when it was set, and Replace opens it for a new
/// secret; Cancel or Escape seals it again, the typed secret dropped. The value is the new secret
/// typed, empty for none: a sealed secret is never read back.
#[must_use = "add it with `ui.add(...)`"]
pub struct SecretField<'a> {
    id: egui::Id,
    label: String,
    value: &'a mut String,
    sealed: bool,
    set_at: Option<String>,
    default_replacing: bool,
    placeholder: Option<String>,
    mono: bool,
    required: bool,
    hint: Option<String>,
    help: Option<String>,
    error: Option<String>,
    read_only: bool,
    enabled: bool,
    bare: bool,
    focus: bool,
    words: SecretFieldWords,
}

impl<'a> SecretField<'a> {
    pub fn new(id: impl egui::AsIdSalt, label: impl Into<String>, value: &'a mut String) -> Self {
        Self {
            id: egui::Id::unique(id),
            label: label.into(),
            value,
            sealed: false,
            set_at: None,
            default_replacing: false,
            placeholder: None,
            mono: true,
            required: false,
            hint: None,
            help: None,
            error: None,
            read_only: false,
            enabled: true,
            bare: false,
            focus: false,
            words: SecretFieldWords::default(),
        }
    }

    /// A secret is already set where the app keeps it: the field shows it as set until Replace.
    pub fn sealed(mut self, sealed: bool) -> Self {
        self.sealed = sealed;
        self
    }

    /// When the secret was set, relative to now (`2 days ago`), said after `words.set_when`.
    pub fn set_at(mut self, ago: impl Into<String>) -> Self {
        self.set_at = Some(ago.into());
        self
    }

    /// A sealed field starts open for a new secret, as if Replace was pressed.
    pub fn default_replacing(mut self, replacing: bool) -> Self {
        self.default_replacing = replacing;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// The secret in the code face (the default), as keys and tokens are.
    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// A star after the label.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// A short note beside the label: a format, a unit.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// A sentence under the input.
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Why the value is refused, as one critical issue.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// No reveal, no Cancel and no Replace; an open input is focusable and selectable.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Cannot be used: faded, no buttons.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The control alone, without the label and the issue mark, for a form that draws its own; the
    /// label names the input, and an error colors its edge.
    pub fn bare(mut self, bare: bool) -> Self {
        self.bare = bare;
        self
    }

    /// Give the input the focus this frame, as a dialog does when it opens.
    pub fn focus(mut self, focus: bool) -> Self {
        self.focus = focus;
        self
    }

    pub fn words(mut self, words: SecretFieldWords) -> Self {
        self.words = words;
        self
    }

    fn changeable(&self) -> bool {
        self.enabled && !self.read_only
    }
}

impl egui::Widget for SecretField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        if self.bare {
            return self.control(ui);
        }
        let field = super::Field::new(self.label.clone())
            .required(self.required)
            .hint(self.hint.clone())
            .help_if(self.help.clone())
            .error(self.error.clone());
        field.show(ui, |ui| self.control(ui))
    }
}

impl SecretField<'_> {
    fn control(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let held_id = self.id.with("held");
        let mut held: Held = ui.data(|d| d.get_temp(held_id)).unwrap_or_default();
        let replacing = held.replacing.unwrap_or(self.default_replacing);
        let sealed = self.sealed && !replacing;
        let width = ui.available_width();
        let height = super::finger_tall(ui.ctx(), &t, t.density("control"));
        let (frame, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let radius = t.radius("control");
        let painter = ui.painter().clone();
        let ground_slot = painter.add(egui::Shape::Noop);
        let side = t.density("control-xs");
        let mut end = frame.right() - 4.0;
        let response;
        if sealed {
            // the key, eight dots, when it was set, and Replace at the end
            let mut replace = None;
            if self.changeable() {
                let button = super::Button::new(&self.words.replace)
                    .variant(super::ButtonVariant::Ghost)
                    .size(super::ButtonSize::Xs);
                let name = self.words.replace_label.replace("{label}", &self.label);
                let place = Rect::from_min_max(pos2(frame.left(), frame.top()), pos2(end, frame.bottom()));
                let hit = ui
                    .scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(place)
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                        |ui| ui.add(button),
                    )
                    .inner;
                hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
                end = hit.rect.left() - 4.0;
                if held.focus_replace {
                    hit.request_focus();
                    held.focus_replace = false;
                }
                if hit.clicked() {
                    held.replacing = Some(true);
                    held.focus_input = true;
                    self.value.clear();
                }
                replace = Some(hit);
            }
            let ink = t.color("muted-foreground");
            let mut x = frame.left() + 10.0;
            icons::paint(&painter, pos2(x + 7.0, frame.center().y), Icon::KeyRound, 14.0, ink);
            x += 14.0 + 4.0;
            let dots_font = crate::mono_font(ui.ctx(), crate::mono_size(&t, t.text("sm").size), 400.0);
            let dots = painter.layout_no_wrap("\u{2022}".repeat(8), dots_font, ink);
            // widely tracked: each dot a quarter of the size apart
            let step = dots.size().x / 8.0 + 2.0;
            for i in 0..8 {
                painter.circle_filled(pos2(x + step * i as f32 + step / 2.0, frame.center().y), 2.0, ink);
            }
            x += step * 8.0 + 8.0;
            let note = match &self.set_at {
                Some(when) => self.words.set_when.replace("{when}", when),
                None => self.words.set.clone(),
            };
            let galley = egui::WidgetText::from(TextRole::Help.rich(&t, note)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                (end - x).max(1.0),
                TextRole::Help.font(&t),
            );
            painter.galley(pos2(x, frame.center().y - galley.size().y / 2.0), galley, ink);
            let ground = t.color_at("muted", 0.4);
            painter.set(
                ground_slot,
                egui::Shape::rect_filled(frame, radius, super::dim(ground, self.enabled)),
            );
            painter.rect_stroke(
                frame,
                radius,
                Stroke::new(1.0, t.color("input")),
                egui::StrokeKind::Inside,
            );
            response = replace.unwrap_or_else(|| ui.interact(frame, self.id.with("sealed"), Sense::hover()));
        } else {
            // Cancel, while a sealed field is open for a new secret
            if self.sealed && self.changeable() {
                let place = Rect::from_min_max(pos2(frame.left(), frame.top()), pos2(end, frame.bottom()));
                let hit = ui
                    .scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(place)
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                        |ui| {
                            ui.add(
                                super::Button::new(&self.words.cancel)
                                    .variant(super::ButtonVariant::Ghost)
                                    .size(super::ButtonSize::Xs),
                            )
                        },
                    )
                    .inner;
                end = hit.rect.left() - 4.0;
                if hit.clicked() {
                    seal(&mut held, self.value);
                }
            }
            // room for the field's issue mark at the end
            if self.error.is_some() && !self.bare {
                end -= 28.0;
            }
            // the reveal: an eye, or a crossed eye while the secret shows
            if self.changeable() {
                let place = Rect::from_min_size(pos2(end - side, frame.center().y - side / 2.0), vec2(side, side));
                let hit = ui.interact(place, self.id.with("reveal"), Sense::click());
                let (icon, word) = if held.revealed {
                    (Icon::EyeOff, &self.words.hide)
                } else {
                    (Icon::Eye, &self.words.show)
                };
                hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, held.revealed, word));
                if hit.hovered() {
                    painter.rect_filled(place, t.radius("sm"), t.color("hover"));
                }
                icons::paint(
                    &painter,
                    place.center(),
                    icon,
                    14.0,
                    t.color(if hit.hovered() {
                        "foreground"
                    } else {
                        "muted-foreground"
                    }),
                );
                if hit.has_focus() {
                    painter.rect_stroke(
                        place,
                        t.radius("sm"),
                        Stroke::new(t.length("focus-ring-width"), t.color_at("ring", 0.5)),
                        egui::StrokeKind::Outside,
                    );
                }
                let hit = hit.on_hover_text(word.clone());
                if hit.clicked() {
                    held.revealed = !held.revealed;
                }
                end = place.left() - 4.0;
            }
            let input_id = self.id.with("input");
            let before = self.value.clone();
            let font = if self.mono {
                crate::mono_font(ui.ctx(), crate::mono_size(&t, t.text("body").size), 400.0)
            } else {
                TextRole::Body.font(&t)
            };
            let font_for_height = font.clone();
            let mut edit = TextEdit::singleline(self.value)
                .id(input_id)
                .password(!held.revealed)
                .font(font)
                .text_color(super::dim(t.color("foreground"), self.enabled))
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .desired_width((end - frame.left() - 10.0).max(1.0));
            if let Some(placeholder) = &self.placeholder {
                edit = edit.hint_text(
                    egui::RichText::new(placeholder)
                        .font(font_for_height.clone())
                        .color(t.color("muted-foreground")),
                );
            }
            // one line of the text, centered in the frame
            let line = ui.fonts_mut(|f| f.row_height(&font_for_height));
            let place = Rect::from_min_max(
                pos2(frame.left() + 10.0, frame.center().y - line / 2.0),
                pos2(end, frame.center().y + line / 2.0),
            );
            let mut inner = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(place)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            inner.spacing_mut().interact_size.y = 0.0;
            let field = inner.add_enabled(self.enabled, edit);
            if self.read_only && *self.value != before {
                // read only: shown and selectable, never changed
                *self.value = before;
            }
            let field = field.accessible_name(&self.label);
            if self.focus || held.focus_input {
                field.request_focus();
                held.focus_input = false;
            }
            // Escape in a sealed field open for a new secret seals it again
            if self.sealed
                && self.changeable()
                && (field.has_focus() || field.lost_focus())
                && ui.input(|i| i.key_pressed(Key::Escape))
            {
                seal(&mut held, self.value);
            }
            let focused = field.has_focus();
            response = field;
            // the frame in its state: the field's ground and edge, the ring edge with focus in it
            let state = match (self.error.is_some(), focused) {
                (true, true) => "invalid+focus",
                (true, false) => "invalid",
                (false, true) => "focus",
                (false, false) => "rest",
            };
            let paint = t.paint(&format!("text-field.base.{state}.control")).unwrap_or_default();
            let ground = if self.enabled {
                paint.ground.unwrap_or_else(|| t.color("field"))
            } else {
                t.color_at("input", 0.5)
            };
            painter.set(ground_slot, egui::Shape::rect_filled(frame, radius, ground));
            let edge = paint.edge.unwrap_or_else(|| t.color("input"));
            painter.rect_stroke(frame, radius, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
            if let Some((color, width)) = paint.ring.filter(|_| focused) {
                painter.rect_stroke(frame, radius, Stroke::new(width, color), egui::StrokeKind::Outside);
            }
        }
        ui.data_mut(|d| d.insert_temp(held_id, held));
        response
    }
}

/// Seal a field open for a new secret again: the typed secret dropped and masked, the focus on
/// Replace.
fn seal(held: &mut Held, value: &mut String) {
    held.replacing = Some(false);
    held.revealed = false;
    held.focus_replace = true;
    value.clear();
}
