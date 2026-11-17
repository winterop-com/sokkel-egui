//! A number typed in with its unit after it, after sokkel-ui's `NumberField`.

use egui::{Id, Key, Modifiers, Response, Ui, Widget};

use super::Input;
use crate::{TextRole, tokens};

/// The words a number field says.
#[derive(Clone, Debug)]
pub struct NumberFieldWords {
    /// Said when what is typed is not a number.
    pub unreadable: String,
}

impl Default for NumberFieldWords {
    fn default() -> Self {
        Self {
            unreadable: "Enter a number".into(),
        }
    }
}

/// A number written plainly: no trailing zeros after the point, none at all for a whole number.
pub fn write_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// What is typed read as a number: digits with a point, a minus before them; `None` otherwise.
pub fn read_number(text: &str) -> Option<f64> {
    let text = text.trim().replace(',', ".");
    if text.is_empty() {
        return None;
    }
    text.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// A labeled field of one number, its unit after it as a `suffix`, as sokkel-ui's `NumberField`:
/// what is typed is read once the field is left and written again plainly; text that is not a
/// number keeps the value and says so. Up and Down step it by `step`, ten steps with Shift.
#[must_use = "add it with `ui.add(...)`"]
pub struct NumberField<'a> {
    id: Id,
    label: String,
    value: &'a mut f64,
    suffix: String,
    step: f64,
    width: Option<f32>,
    words: NumberFieldWords,
}

impl<'a> NumberField<'a> {
    pub fn new(id: impl egui::AsIdSalt, label: impl Into<String>, value: &'a mut f64) -> Self {
        Self {
            id: Id::unique(id),
            label: label.into(),
            value,
            suffix: String::new(),
            step: 1.0,
            width: None,
            words: NumberFieldWords::default(),
        }
    }

    /// Words after the number, such as its unit: `ms`, `%`.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// How far the arrow keys move the value, 1 when not given.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// A width of its own, rather than the room it is given.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn words(mut self, words: NumberFieldWords) -> Self {
        self.words = words;
        self
    }
}

/// What is being typed into a number field, kept while it has the focus.
#[derive(Clone, Default)]
struct Typing {
    text: String,
    unreadable: bool,
}

impl Widget for NumberField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let key = self.id.with("typing");
        let mut typing = ui.data(|d| d.get_temp::<Typing>(key)).unwrap_or_else(|| Typing {
            text: write_number(*self.value),
            unreadable: false,
        });
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let mut changed = false;
        let response = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                let label = ui.label(TextRole::Label.rich(&t, &self.label).color(t.color("foreground")));
                let mut text = typing.text.clone();
                let mut response = ui
                    .push_id(self.id, |ui| {
                        ui.add(
                            Input::new(&mut text)
                                .name(&self.label)
                                .width(width)
                                .suffix(self.suffix.trim())
                                .invalid(typing.unreadable),
                        )
                    })
                    .inner
                    .labelled_by(label.id);
                typing.text = text;
                if response.has_focus() {
                    // the arrows step the value, ten steps with Shift
                    let steps = ui.input_mut(|i| {
                        let mut steps = 0.0;
                        for (modifiers, by) in [(Modifiers::NONE, 1.0), (Modifiers::SHIFT, 10.0)] {
                            if i.consume_key(modifiers, Key::ArrowUp) {
                                steps += by;
                            }
                            if i.consume_key(modifiers, Key::ArrowDown) {
                                steps -= by;
                            }
                        }
                        steps
                    });
                    if steps != 0.0 {
                        let from = read_number(&typing.text).unwrap_or(*self.value);
                        let next = ((from / self.step).round() + steps) * self.step;
                        *self.value = next;
                        typing.text = write_number(next);
                        typing.unreadable = false;
                        changed = true;
                    }
                }
                if response.lost_focus() || (response.changed() && read_number(&typing.text).is_some()) {
                    match read_number(&typing.text) {
                        Some(next) => {
                            if next != *self.value {
                                *self.value = next;
                                changed = true;
                            }
                            typing.unreadable = false;
                            if response.lost_focus() {
                                typing.text = write_number(next);
                            }
                        }
                        None => typing.unreadable = true,
                    }
                }
                if typing.unreadable {
                    ui.label(
                        TextRole::Help
                            .rich(&t, &self.words.unreadable)
                            .color(t.color("critical-ink")),
                    );
                }
                if changed {
                    response.mark_changed();
                }
                response
            })
            .inner;
        // away from the field it shows the value as given, so a value set elsewhere shows
        if response.has_focus() || typing.unreadable {
            ui.data_mut(|d| d.insert_temp(key, typing));
        } else {
            ui.data_mut(|d| d.remove::<Typing>(key));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::{read_number, write_number};

    #[test]
    fn numbers_are_read_and_written_plainly() {
        assert_eq!(write_number(80.0), "80");
        assert_eq!(write_number(0.5), "0.5");
        assert_eq!(read_number(" 12,5 "), Some(12.5));
        assert_eq!(read_number("ten"), None);
        assert_eq!(read_number(""), None);
    }
}
