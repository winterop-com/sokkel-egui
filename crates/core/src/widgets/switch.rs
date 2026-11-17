use egui::{Response, Sense, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2};

use super::{dim, focus_ring};
use crate::tokens;

/// A switch, 32 by 18.4 (24 by 14 small): `primary` when on, `input` when off, its thumb in
/// `background`.
#[must_use = "add it with `ui.add(...)`"]
pub struct Switch<'a> {
    on: &'a mut bool,
    label: String,
    enabled: bool,
    small: bool,
}

impl<'a> Switch<'a> {
    /// `label` names the switch for assistive technology; show it beside the switch yourself.
    pub fn new(on: &'a mut bool, label: impl Into<String>) -> Self {
        Self {
            on,
            label: label.into(),
            enabled: true,
            small: false,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The small switch, as the web's `size="sm"`: 24 by 14, its thumb 12.
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }
}

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let (track_size, size) = if self.small {
            (vec2(24.0, 14.0), 12.0)
        } else {
            (vec2(32.0, 18.4), 16.0)
        };
        let (rect, mut response) = ui.allocate_exact_size(track_size, sense);
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        let on = *self.on;
        response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, self.enabled, on, &self.label));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let k = ui.ctx().animate_bool_responsive(response.id, on);
        let off = if t.dark {
            t.color_at("input", 0.8)
        } else {
            t.color("input")
        };
        let track = off.lerp_to_gamma(t.color("primary"), k);
        let thumb = match (t.dark, on) {
            (false, _) => t.color("background"),
            (true, true) => t.color("primary-foreground"),
            (true, false) => t.color("foreground"),
        };
        let radius = t.radius("control");
        let painter = ui.painter();
        painter.rect_filled(rect, radius, dim(track, self.enabled));
        // the thumb inside a 1px transparent border, moved by its width less 2px when on
        let x = rect.left() + 1.0 + k * (size - 2.0);
        let thumb_rect = egui::Rect::from_min_size(pos2(x, rect.center().y - size / 2.0), vec2(size, size));
        painter.rect_filled(thumb_rect, t.radius("thumb"), dim(thumb, self.enabled));
        focus_ring(painter, &t, rect, radius, &response);
        response
    }
}
