//! A number typed in with its unit after it, after sokkel-ui's `NumberField`.

use egui::{Id, Key, Modifiers, Response, Ui, Widget};

use super::Input;

/// The words a number field says.
#[derive(Clone, Debug)]
pub struct NumberFieldWords {
    /// Said when what is typed is not a number.
    pub unreadable: String,
    /// Said when a whole number is wanted and what is typed has a fraction.
    pub whole: String,
}

impl Default for NumberFieldWords {
    fn default() -> Self {
        Self {
            unreadable: "Enter a number".into(),
            whole: "Enter a whole number".into(),
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
    value: Held<'a>,
    suffix: String,
    step: f64,
    width: Option<f32>,
    integer: bool,
    required: bool,
    hint: Option<String>,
    description: Option<String>,
    help: Option<String>,
    error: Option<String>,
    issues: Vec<(super::IssueTone, String)>,
    words: NumberFieldWords,
    format: Option<Write<'a>>,
    parse: Option<Read<'a>>,
    mono: bool,
    bare: bool,
}

/// How a number field writes its value, as the web's `format`.
type Write<'a> = Box<dyn Fn(f64) -> String + 'a>;
/// How it reads what is typed, as the web's `parse`.
type Read<'a> = Box<dyn Fn(&str) -> Option<f64> + 'a>;

/// The value a number field holds: always a number, or a number or none.
enum Held<'a> {
    Always(&'a mut f64),
    Maybe(&'a mut Option<f64>),
}

impl Held<'_> {
    fn get(&self) -> Option<f64> {
        match self {
            Held::Always(v) => Some(**v),
            Held::Maybe(v) => **v,
        }
    }

    fn set(&mut self, value: Option<f64>) {
        match self {
            Held::Always(v) => {
                if let Some(value) = value {
                    **v = value;
                }
            }
            Held::Maybe(v) => **v = value,
        }
    }
}

impl<'a> NumberField<'a> {
    pub fn new(id: impl egui::AsIdSalt, label: impl Into<String>, value: &'a mut f64) -> Self {
        Self::holding(id, label, Held::Always(value))
    }

    /// A number that may be left empty: emptied, it holds none, as an unanswered question.
    pub fn optional(id: impl egui::AsIdSalt, label: impl Into<String>, value: &'a mut Option<f64>) -> Self {
        Self::holding(id, label, Held::Maybe(value))
    }

    fn holding(id: impl egui::AsIdSalt, label: impl Into<String>, value: Held<'a>) -> Self {
        Self {
            id: Id::unique(id),
            label: label.into(),
            value,
            suffix: String::new(),
            step: 1.0,
            width: None,
            integer: false,
            required: false,
            hint: None,
            description: None,
            help: None,
            error: None,
            issues: Vec::new(),
            words: NumberFieldWords::default(),
            format: None,
            parse: None,
            mono: false,
            bare: false,
        }
    }

    /// How the value is written in the field, as the web's `format`, such as a time as a timecode;
    /// plainly when not given.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'a) -> Self {
        self.format = Some(Box::new(format));
        self
    }

    /// How what is typed is read, as the web's `parse`, such as a timecode read back to seconds;
    /// as a plain number when not given.
    pub fn parse(mut self, parse: impl Fn(&str) -> Option<f64> + 'a) -> Self {
        self.parse = Some(Box::new(parse));
        self
    }

    /// The number in the code face, as the web's `mono`, for a timecode or an id.
    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// The input alone, its label naming it to a screen reader only, as the web's `bare`: for a
    /// field whose row already says what it is, such as a setting's.
    pub fn bare(mut self, bare: bool) -> Self {
        self.bare = bare;
        self
    }

    /// Whole numbers only: a fraction typed is said to be one.
    pub fn integer(mut self, integer: bool) -> Self {
        self.integer = integer;
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// A short word after the label, such as the unit.
    pub fn hint(mut self, hint: Option<impl Into<String>>) -> Self {
        self.hint = hint.map(Into::into);
        self
    }

    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = description.map(Into::into);
        self
    }

    pub fn help(mut self, help: Option<impl Into<String>>) -> Self {
        self.help = help.map(Into::into);
        self
    }

    /// The app's own word on the value, such as one past the range a form takes.
    /// What is wrong with the value, each in its tone, on the field's issue mark.
    pub fn issues(mut self, issues: impl IntoIterator<Item = (super::IssueTone, String)>) -> Self {
        self.issues = issues.into_iter().collect();
        self
    }

    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
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
    fn ui(mut self, ui: &mut Ui) -> Response {
        let key = self.id.with("typing");
        let shown = self.value.get();
        let format = self.format.take();
        let parse = self.parse.take();
        let write = |value: f64| format.as_ref().map_or_else(|| write_number(value), |f| f(value));
        let read = |text: &str| parse.as_ref().map_or_else(|| read_number(text), |p| p(text));
        let mut typing = ui.data(|d| d.get_temp::<Typing>(key)).unwrap_or_else(|| Typing {
            text: shown.map(write).unwrap_or_default(),
            unreadable: false,
        });
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let mut changed = false;
        let whole = |v: f64| !self.integer || v.fract() == 0.0;
        let issue = if typing.unreadable {
            Some(self.words.unreadable.clone())
        } else if read(&typing.text).is_some_and(|v| !whole(v)) {
            Some(self.words.whole.clone())
        } else {
            self.error.clone()
        };
        let input = |ui: &mut Ui, typing: &mut Typing, value: &mut Held<'_>, changed: &mut bool| {
            let mut text = typing.text.clone();
            let mut response = ui
                .push_id(self.id, |ui| {
                    ui.add(
                        Input::new(&mut text)
                            .name(&self.label)
                            .width(width)
                            .mono(self.mono)
                            .suffix(self.suffix.trim()),
                    )
                })
                .inner;
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
                    let from = read(&typing.text).or(shown).unwrap_or(0.0);
                    let next = ((from / self.step).round() + steps) * self.step;
                    value.set(Some(next));
                    typing.text = write(next);
                    typing.unreadable = false;
                    *changed = true;
                }
            }
            if response.lost_focus() || response.changed() {
                let empty = typing.text.trim().is_empty();
                match read(&typing.text) {
                    Some(next) => {
                        if Some(next) != value.get() {
                            value.set(Some(next));
                            *changed = true;
                        }
                        typing.unreadable = false;
                        if response.lost_focus() {
                            typing.text = write(next);
                        }
                    }
                    None if empty && matches!(value, Held::Maybe(_)) => {
                        if value.get().is_some() {
                            value.set(None);
                            *changed = true;
                        }
                        typing.unreadable = false;
                    }
                    None => typing.unreadable = response.lost_focus() || typing.unreadable,
                }
            }
            if *changed {
                response.mark_changed();
            }
            response
        };
        let response = if self.bare {
            input(ui, &mut typing, &mut self.value, &mut changed)
        } else {
            let mut field = super::Field::new(&self.label)
                .required(self.required)
                .hint(self.hint.clone())
                .help_if(self.help.clone())
                .error(issue.clone())
                .issues(self.issues.clone());
            if let Some(description) = &self.description {
                field = field.description(description);
            }
            field.show(ui, |ui| input(ui, &mut typing, &mut self.value, &mut changed))
        };
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
