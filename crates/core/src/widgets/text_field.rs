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
    width: f32,
}

impl<'a> TextField<'a> {
    pub fn new(label: impl Into<String>, value: &'a mut String) -> Self {
        Self {
            label: label.into(),
            value,
            placeholder: None,
            help: None,
            error: None,
            width: 280.0,
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

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

impl Widget for TextField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let label = ui.label(TextRole::Label.rich(&t, &self.label).color(t.color("foreground")));
            let mut input = Input::new(self.value).width(self.width).invalid(self.error.is_some());
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
    width: f32,
}

impl<'a> Input<'a> {
    pub fn new(value: &'a mut String) -> Self {
        Self {
            value,
            placeholder: None,
            invalid: false,
            enabled: true,
            width: 280.0,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
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
        let mut edit = TextEdit::singleline(self.value)
            .font(body.font(&t))
            .text_color(ink)
            .frame(Frame::NONE)
            .desired_width(self.width - 20.0)
            .margin(Margin::symmetric(0, 0));
        if let Some(placeholder) = &self.placeholder {
            edit = edit.hint_text(body.rich(&t, placeholder).color(t.color("muted-foreground")));
        }
        let height = super::finger_tall(ui.ctx(), &t, t.density("control"));
        let ground = match (self.enabled, t.dark) {
            (true, _) => t.color("field"),
            (false, false) => t.color_at("input", 0.5),
            (false, true) => t.color_at("input", 0.8),
        };
        let frame = Frame::new()
            .fill(ground)
            .corner_radius(t.radius("control"))
            .inner_margin(Margin::symmetric(
                10,
                ((height - body.font(&t).size * 1.4) / 2.0).max(0.0) as i8,
            ));
        let mut prepared = frame.begin(ui);
        let response = prepared.content_ui.add_enabled(self.enabled, edit);
        let focused = response.has_focus();
        let edge = match (self.invalid, focused) {
            (true, true) => t.color("critical"),
            (true, false) => t.color_at("critical", 0.8),
            (false, true) => t.color("ring"),
            (false, false) => t.color("input"),
        };
        prepared.frame.stroke = Stroke::new(1.0, edge);
        let outer = prepared.end(ui);
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
