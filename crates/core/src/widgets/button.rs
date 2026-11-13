use egui::{
    Color32, CornerRadius, Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Widget, WidgetInfo, WidgetText,
    WidgetType, pos2, vec2,
};

use super::{dim, focus_ring};
use crate::{Icon, TextRole, icons, tokens};

/// How a button looks, as sokkel-ui's `Button` `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Outline,
    Secondary,
    Ghost,
    Destructive,
    Link,
}

/// How big a button is, as sokkel-ui's `Button` `size`: 24, 28, 32 and 36 tall.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    Xs,
    Sm,
    #[default]
    Default,
    Lg,
}

/// A button. `busy` shows a spinner and takes no press; `reason` disables it and says why on hover.
#[must_use = "add it with `ui.add(...)`"]
pub struct Button {
    label: String,
    variant: ButtonVariant,
    size: ButtonSize,
    busy: bool,
    enabled: bool,
    reason: Option<String>,
    icon: Option<Icon>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Default,
            size: ButtonSize::Default,
            busy: false,
            enabled: true,
            reason: None,
            icon: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// A Lucide icon before the label; a busy button shows its spinner in the icon's place.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Disable the button and say why in a tooltip.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let enabled = self.enabled && self.reason.is_none();
        let control = t.length("radius-control");
        // height, side padding, text role and the corner cap the small sizes keep
        let (height, pad, role, cap, glyph) = match self.size {
            ButtonSize::Xs => (24.0, 8.0, TextRole::Badge, 10.0, 12.0),
            ButtonSize::Sm => (28.0, 10.0, TextRole::Badge, 12.0, 14.0),
            ButtonSize::Default => (32.0, 10.0, TextRole::Action, f32::INFINITY, 16.0),
            ButtonSize::Lg => (36.0, 10.0, TextRole::Action, f32::INFINITY, 16.0),
        };
        // the icon's room, or the spinner's in its place: the glyph and the gap after it
        let gap = if self.size == ButtonSize::Xs || self.size == ButtonSize::Sm {
            4.0
        } else {
            6.0
        };
        let lead = if self.busy || self.icon.is_some() {
            glyph + gap
        } else {
            0.0
        };
        let pad = if lead > 0.0 { pad - 2.0 } else { pad };
        let radius = CornerRadius::same(control.min(cap).floor() as u8);
        let galley = WidgetText::from(role.rich(&t, &self.label)).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            role.font(&t),
        );
        let size = vec2(galley.size().x + lead + pad * 2.0, height);
        // a busy button, and one disabled with a reason, keep their place in the tab order so the
        // reason can be read; a plain disabled button leaves it
        let sense = if enabled && !self.busy {
            Sense::click()
        } else if self.busy || self.reason.is_some() {
            Sense::focusable_noninteractive()
        } else {
            Sense::hover()
        };
        let (rect, mut response) = ui.allocate_exact_size(size, sense);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &self.label));
        if let Some(reason) = &self.reason {
            response = super::tooltip(response, reason.clone());
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        let hot = response.hovered() && enabled && !self.busy;
        let none = Color32::TRANSPARENT;
        let dark = t.dark;
        // fill, border and ink at rest, and the fill under the pointer
        let (fill, border, ink, hover) = match self.variant {
            ButtonVariant::Default => (
                t.color("primary"),
                none,
                t.color("primary-foreground"),
                t.color_at("primary", 0.8),
            ),
            ButtonVariant::Outline => {
                let (fill, border) = if dark {
                    (t.color_at("input", 0.3), t.color("input"))
                } else {
                    (t.color("background"), t.color("border"))
                };
                (fill, border, t.color("foreground"), t.color("hover"))
            }
            ButtonVariant::Secondary => {
                let secondary = t.color("secondary");
                (
                    secondary,
                    none,
                    t.color("secondary-foreground"),
                    secondary.lerp_to_gamma(t.color("foreground"), 0.05),
                )
            }
            ButtonVariant::Ghost => (none, none, t.color("foreground"), t.color("hover")),
            ButtonVariant::Destructive => {
                let (rest, over) = if dark { (0.2, 0.3) } else { (0.1, 0.2) };
                (
                    t.color_at("destructive", rest),
                    none,
                    t.color("destructive"),
                    t.color_at("destructive", over),
                )
            }
            ButtonVariant::Link => (none, none, t.color("primary"), none),
        };
        // a pressed button sinks a pixel, as `active:translate-y-px` moves it
        let rect = if hot && response.is_pointer_button_down_on() {
            rect.translate(vec2(0.0, 1.0))
        } else {
            rect
        };
        // under the pointer the hover ground takes the place of the rest ground, as `hover:bg-*` does
        let ground = if hot && hover != none { hover } else { fill };
        let painter = ui.painter();
        painter.rect(
            rect,
            radius,
            dim(ground, enabled),
            Stroke::new(1.0, dim(border, enabled)),
            StrokeKind::Inside,
        );
        focus_ring(painter, &t, rect, radius, &response);

        let ink = dim(ink, enabled);
        let mut x = rect.center().x - (galley.size().x + lead) / 2.0;
        if self.busy {
            super::spinner(ui, painter, pos2(x + glyph / 2.0, rect.center().y), glyph * 0.4, ink);
        } else if let Some(icon) = self.icon {
            icons::paint(painter, pos2(x + glyph / 2.0, rect.center().y), icon, glyph, ink);
        }
        x += lead;
        let pos = pos2(x, rect.center().y - galley.size().y / 2.0);
        if self.variant == ButtonVariant::Link && hot {
            let y = pos.y + galley.size().y - 3.0;
            painter.hline(pos.x..=pos.x + galley.size().x, y, Stroke::new(1.0, ink));
        }
        painter.galley(pos, galley, ink);
        response
    }
}
