//! A small round mark in a color, with words beside it, after sokkel-ui's `Dot`.

use egui::{Color32, Response, Sense, TextWrapMode, Ui, Widget, WidgetText, pos2, vec2};

use crate::{TextRole, tokens};

/// A 10 dot in a color, 4 before small words in the muted ink, as sokkel-ui's `Dot` with words
/// beside it in a legend's row.
#[must_use = "add it with `ui.add(...)`"]
pub struct Dot {
    color: Color32,
    label: String,
}

impl Dot {
    pub fn new(color: Color32, label: impl Into<String>) -> Self {
        Self {
            color,
            label: label.into(),
        }
    }
}

impl Widget for Dot {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let galley = WidgetText::from(TextRole::Help.rich(&t, &self.label)).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextRole::Help.font(&t),
        );
        let size = 10.0;
        let (rect, response) = ui.allocate_exact_size(
            vec2(size + 4.0 + galley.size().x, galley.size().y.max(size)),
            Sense::hover(),
        );
        ui.painter()
            .circle_filled(pos2(rect.left() + size / 2.0, rect.center().y), size / 2.0, self.color);
        ui.painter().galley(
            pos2(rect.left() + size + 4.0, rect.center().y - galley.size().y / 2.0),
            galley,
            t.color("muted-foreground"),
        );
        response
    }
}
