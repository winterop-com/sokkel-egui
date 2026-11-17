use egui::{Frame, Margin, Response, Stroke, TextEdit, Ui, Widget};

use crate::{TextRole, tokens};

/// A labeled one-line text field: the label, the input (32 tall, `field` ground, `input` edge,
/// `ring` while focused), and help or an error under it.
#[must_use = "add it with `ui.add(...)`"]
pub struct TextField<'a> {
    label: String,
    value: &'a mut String,
    placeholder: Option<String>,
    help: Option<String>,
    error: Option<String>,
    /// Its width; when `None` the room it is given down a column, as the web's `w-full`, and 280 in a row.
    width: Option<f32>,
    /// A text area that grows with its text, at least 112 tall, rather than one line.
    multiline: bool,
}

impl<'a> TextField<'a> {
    pub fn new(label: impl Into<String>, value: &'a mut String) -> Self {
        Self {
            label: label.into(),
            value,
            placeholder: None,
            help: None,
            error: None,
            width: None,
            multiline: false,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// An error: the input's edge turns critical and the error stands in for the help.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// A width of its own, rather than the room it is given.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// A text area that grows with its text, at least 112 tall (`min-h-28`), as the web's
    /// `multiline` field.
    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }
}

impl Widget for TextField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        // the room it is given down a column; in a row, 280 unless given a width
        let width = self.width.unwrap_or_else(|| {
            if ui.layout().is_horizontal() {
                280.0
            } else {
                ui.available_width()
            }
        });
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let label = ui.label(TextRole::Label.rich(&t, &self.label).color(t.color("foreground")));
            let mut input = Input::new(self.value).invalid(self.error.is_some());
            input.width = Some(width);
            input.multiline = self.multiline;
            if let Some(placeholder) = &self.placeholder {
                input = input.placeholder(placeholder.clone());
            }
            let response = ui.add(input).labelled_by(label.id);
            match (&self.error, &self.help) {
                (Some(error), _) => {
                    ui.label(TextRole::Help.rich(&t, error).color(t.color("critical-ink")));
                }
                (None, Some(help)) => {
                    ui.label(TextRole::Help.rich(&t, help).color(t.color("muted-foreground")));
                }
                (None, None) => {}
            }
            response
        })
        .inner
    }
}

/// A one-line text input: 32 tall in the control density, the `field` ground and `input` edge, the
/// `ring` edge and a 1px ring while focused, the critical edge when `invalid`.
#[must_use = "add it with `ui.add(...)`"]
pub struct Input<'a> {
    value: &'a mut String,
    placeholder: Option<String>,
    invalid: bool,
    enabled: bool,
    /// Its width; when `None` the room it is given down a column, as the web's `w-full`, and 280 in a row.
    width: Option<f32>,
    /// A text area that grows with its text, at least 112 tall, rather than one line.
    multiline: bool,
    /// Muted words after the text at the input's end, such as a unit.
    suffix: Option<String>,
    /// What a screen reader calls it, where no label beside it says.
    name: Option<String>,
    /// Shown, never typed in, on the muted ground, as a worked-out value's `readOnly` input.
    read_only: bool,
}

impl<'a> Input<'a> {
    pub fn new(value: &'a mut String) -> Self {
        Self {
            value,
            placeholder: None,
            invalid: false,
            read_only: false,
            enabled: true,
            width: None,
            multiline: false,
            suffix: None,
            name: None,
        }
    }

    /// What a screen reader calls the input, where no label beside it says, as the web's
    /// `aria-label`; its placeholder when not given.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Muted words at the input's end, after the text, such as its unit: `ms`, `%`.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        let suffix = suffix.into();
        self.suffix = (!suffix.is_empty()).then_some(suffix);
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Shown and never typed in, on the muted ground, in the full ink: a value worked out.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// A width of its own, rather than the room it is given.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// A text area that grows with its text, at least 112 tall (`min-h-28`), as the web's
    /// `multiline` field.
    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }
}

impl Widget for Input<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let body = TextRole::Body;
        let ink = if self.enabled {
            t.color("foreground")
        } else {
            t.color_at("foreground", 0.5)
        };
        // the room it is given down a column; in a row, where the room is everything after it,
        // 280 unless given a width
        let width = self.width.unwrap_or_else(|| {
            if ui.layout().is_horizontal() {
                280.0
            } else {
                ui.available_width()
            }
        });
        let edit = if self.multiline {
            TextEdit::multiline(self.value).desired_rows(1)
        } else {
            TextEdit::singleline(self.value)
        };
        let suffix = self.suffix.as_ref().map(|words| {
            egui::WidgetText::from(body.rich(&t, words)).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                body.font(&t),
            )
        });
        let room = suffix.as_ref().map_or(0.0, |g| g.size().x + 8.0);
        let mut edit = edit
            .font(body.font(&t))
            .text_color(ink)
            .frame(Frame::NONE)
            .desired_width(width - 20.0 - room)
            .interactive(!self.read_only)
            .margin(Margin::symmetric(0, 0));
        if let Some(placeholder) = &self.placeholder {
            edit = edit.hint_text(body.rich(&t, placeholder).color(t.color("muted-foreground")));
        }
        let height = super::finger_tall(ui.ctx(), &t, t.density("control"));
        let ground = match (self.enabled, t.dark) {
            _ if self.read_only => t.color("muted"),
            (true, _) => t.color("field"),
            (false, false) => t.color_at("input", 0.5),
            (false, true) => t.color_at("input", 0.8),
        };
        // the edge's width is set before the content is laid out, so the frame takes the room it
        // is given; its color follows focus once the content has drawn
        let frame = Frame::new()
            .fill(ground)
            .stroke(Stroke::new(1.0, t.color("input")))
            .corner_radius(t.radius("control"))
            .inner_margin(if self.multiline {
                Margin::symmetric(10, 8)
            } else {
                // what is left of the control's height, less its edge, around one line of the text
                Margin::symmetric(
                    10,
                    ((height - 2.0 - ui.fonts_mut(|f| f.row_height(&body.font(&t)))) / 2.0)
                        .floor()
                        .max(0.0) as i8,
                )
            });
        let mut prepared = frame.begin(ui);
        if self.multiline {
            // at least 112 tall with its edge, as the web's `min-h-28`, growing with its text
            prepared.content_ui.set_min_height(112.0 - 16.0 - 2.0);
        }
        // the line alone sets the field's height, not egui's least height for something to press,
        // nor the height of a row it stands in
        prepared.content_ui.spacing_mut().interact_size.y = 0.0;
        if !self.multiline {
            // the content as tall as leaves the control its height exactly, edge and all
            let margin = ((height - 2.0 - ui.fonts_mut(|f| f.row_height(&body.font(&t)))) / 2.0)
                .floor()
                .max(0.0);
            let inside = (height - 2.0 - 2.0 * margin).max(0.0);
            prepared.content_ui.set_min_height(inside);
            prepared.content_ui.set_max_height(inside);
        }
        let mut response = prepared.content_ui.add_enabled(self.enabled, edit);
        if let Some(name) = self.name.as_ref().or(self.placeholder.as_ref()) {
            response = response.accessible_name(name);
        }
        let focused = response.has_focus();
        // the control's edge in its state, as sokkel-design paints it
        let state = match (self.invalid, focused) {
            (true, true) => "invalid+focus",
            (true, false) => "invalid",
            (false, true) => "focus",
            (false, false) => "rest",
        };
        let edge = t
            .paint(&format!("text-field.base.{state}.control"))
            .and_then(|p| p.edge)
            .unwrap_or_else(|| match (self.invalid, focused) {
                (true, true) => t.color("critical"),
                (true, false) => t.color_at("critical", 0.8),
                (false, true) => t.color("ring"),
                (false, false) => t.color("input"),
            });
        prepared.frame.stroke = Stroke::new(1.0, edge);
        if room > 0.0 {
            // the unit's room at the end, the frame as wide as without it
            prepared.content_ui.set_min_width(width - 20.0);
        }
        let outer = prepared.end(ui);
        if let Some(suffix) = suffix {
            let at = egui::pos2(
                outer.rect.right() - 10.0 - suffix.size().x,
                outer.rect.center().y - suffix.size().y / 2.0,
            );
            ui.painter().galley(at, suffix, t.color("muted-foreground"));
        }
        if focused {
            // the 1px ring outside the edge, or the 3px critical one an invalid input wears
            let ring = if self.invalid {
                Stroke::new(3.0, t.color_at("critical", if t.dark { 0.4 } else { 0.2 }))
            } else {
                Stroke::new(1.0, t.color("ring"))
            };
            ui.painter()
                .rect_stroke(outer.rect, t.radius("control"), ring, egui::StrokeKind::Outside);
        }
        response
    }
}
