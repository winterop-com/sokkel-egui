use egui::{
    Color32, CornerRadius, Response, Role, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Widget, WidgetInfo, WidgetText,
    pos2, vec2,
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

impl ButtonVariant {
    /// The variant's name in sokkel-design's paints: `button.<key>.<state>.container`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Outline => "outline",
            Self::Secondary => "secondary",
            Self::Ghost => "ghost",
            Self::Destructive => "destructive",
            Self::Link => "link",
        }
    }
}

/// How big a button is, as sokkel-ui's `Button` `size`: as tall as the density's
/// `control-xs`, `control-sm`, `control` and `control-lg` (24, 28, 32 and 36 at regular).
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
    trailing: Option<Icon>,
    muted: bool,
    note: Option<String>,
    pressed: Option<bool>,
    /// The icon in an ink of its own, as `<Check className="text-good-ink" />`.
    pub(crate) icon_ink: Option<Color32>,
    /// The words and icon in an ink of their own, as `text-critical-ink`.
    pub(crate) ink: Option<Color32>,
    /// A progress ring in the icon's place, at this share from 0 to 1.
    pub(crate) ring: Option<f32>,
    /// Drawn as a button at rest that takes no press, as a `pointer-events-none` status.
    pub(crate) still: bool,
    /// The label said to a screen reader and not drawn, as `sr-only`: the icon alone.
    pub(crate) unseen_label: bool,
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
            trailing: None,
            muted: false,
            note: None,
            pressed: None,
            icon_ink: None,
            ink: None,
            ring: None,
            still: false,
            unseen_label: false,
        }
    }

    /// A toggle: pressed, it stands on the muted ground, as the web's `Toggle` with
    /// `aria-pressed`, and says so to a screen reader.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    /// Its words in the muted ink, as a quiet ghost button's `text-muted-foreground`.
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    /// A few words after its label at the regular weight in the muted ink, as a trigger saying its
    /// view has changed.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// An icon after the words in the muted ink, as a trigger's chevron.
    pub fn trailing(mut self, icon: Icon) -> Self {
        self.trailing = Some(icon);
        self
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
            ButtonSize::Xs => (t.density("control-xs"), 8.0, TextRole::Badge, 10.0, 12.0),
            ButtonSize::Sm => (t.density("control-sm"), 10.0, TextRole::Badge, 12.0, 14.0),
            ButtonSize::Default => (t.density("control"), 10.0, TextRole::Action, f32::INFINITY, 16.0),
            ButtonSize::Lg => (t.density("control-lg"), 10.0, TextRole::Action, f32::INFINITY, 16.0),
        };
        // a finger tall on a phone, the extra small size aside
        let height = if self.size == ButtonSize::Xs {
            height
        } else {
            super::finger_tall(ui.ctx(), &t, height)
        };
        // the icon's room, or the spinner's in its place: the glyph and the gap after it
        let gap = if self.size == ButtonSize::Xs || self.size == ButtonSize::Sm {
            4.0
        } else {
            6.0
        };
        let lead = if self.busy || self.icon.is_some() || self.ring.is_some() {
            if self.unseen_label { glyph } else { glyph + gap }
        } else {
            0.0
        };
        let note = self.note.as_ref().map(|n| {
            WidgetText::from(TextRole::Body.rich(&t, n)).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextRole::Body.font(&t),
            )
        });
        let tail =
            if self.trailing.is_some() { gap + glyph } else { 0.0 } + note.as_ref().map_or(0.0, |n| gap + n.size().x);
        let pad = if lead > 0.0 || tail > 0.0 { pad - 2.0 } else { pad };
        let radius = CornerRadius::same(control.min(cap).floor() as u8);
        let shown = if self.unseen_label { "" } else { self.label.as_str() };
        let galley = WidgetText::from(role.rich(&t, shown)).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            role.font(&t),
        );
        let size = vec2(galley.size().x + lead + tail + pad * 2.0, height);
        // a busy button, and one disabled with a reason, keep their place in the tab order so the
        // reason can be read; a plain disabled button leaves it
        let sense = if self.still {
            Sense::hover()
        } else if enabled && !self.busy {
            Sense::click()
        } else if self.busy || self.reason.is_some() {
            Sense::focusable_noninteractive()
        } else {
            Sense::hover()
        };
        let (rect, mut response) = ui.allocate_exact_size(size, sense);
        response.widget_info(|| match self.pressed {
            Some(pressed) => WidgetInfo::selected(Role::Button, enabled, pressed, &self.label),
            None => WidgetInfo::labeled(Role::Button, enabled, &self.label),
        });
        if let Some(reason) = &self.reason {
            response = super::tooltip(response, reason.clone());
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        let hot = response.hovered() && enabled && !self.busy && !self.still;
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
        // sokkel-design's resolved paint for the variant in this state, where the tokens carry it
        let state = if hot { "hover" } else { "rest" };
        let (ground, border, ink) = match t.paint(&format!("button.{}.{state}.container", self.variant.key())) {
            Some(paint) => (
                paint.ground.unwrap_or(ground),
                paint.edge.unwrap_or(border),
                paint.ink.unwrap_or(ink),
            ),
            None => (ground, border, ink),
        };
        let ground = if self.pressed == Some(true) {
            t.color("muted")
        } else {
            ground
        };
        let painter = ui.painter();
        painter.rect(
            rect,
            radius,
            dim(ground, enabled),
            Stroke::new(1.0, dim(border, enabled)),
            StrokeKind::Inside,
        );
        focus_ring(painter, &t, rect, radius, &response);

        let ink = dim(
            if let Some(own) = self.ink {
                own
            } else if self.muted && !hot {
                t.color("muted-foreground")
            } else {
                ink
            },
            enabled,
        );
        let mut x = rect.center().x - (galley.size().x + lead + tail) / 2.0;
        if self.busy {
            super::spinner(ui, painter, pos2(x + glyph / 2.0, rect.center().y), glyph * 0.4, ink);
        } else if let Some(share) = self.ring {
            super::download_button::paint_progress_ring(
                painter,
                &t,
                pos2(x + glyph / 2.0, rect.center().y),
                glyph,
                share,
            );
        } else if let Some(icon) = self.icon {
            let icon_ink = self.icon_ink.map_or(ink, |own| dim(own, enabled));
            icons::paint(painter, pos2(x + glyph / 2.0, rect.center().y), icon, glyph, icon_ink);
        }
        x += lead;
        let pos = pos2(x, rect.center().y - galley.size().y / 2.0);
        if self.variant == ButtonVariant::Link && hot {
            let y = pos.y + galley.size().y - 3.0;
            painter.hline(pos.x..=pos.x + galley.size().x, y, Stroke::new(1.0, ink));
        }
        let mut words_end = pos.x + galley.size().x;
        painter.galley(pos, galley, ink);
        if let Some(note) = note {
            let at = pos2(words_end + gap, rect.center().y - note.size().y / 2.0);
            words_end = at.x + note.size().x;
            painter.galley(at, note, dim(t.color("muted-foreground"), enabled));
        }
        if let Some(icon) = self.trailing {
            icons::paint(
                painter,
                pos2(words_end + gap + glyph / 2.0, rect.center().y),
                icon,
                glyph,
                dim(t.color("muted-foreground"), enabled),
            );
        }
        response
    }
}

/// A button that is an icon alone, after sokkel-ui's `IconButton`: a ghost square `control-sm` tall
/// (`icon-sm`, the web's default),
/// the icon in `muted-foreground`, under the pointer the `hover` ground and the foreground ink; its
/// label is what a screen reader hears and what the pointer is told.
pub struct IconButton {
    icon: crate::Icon,
    label: String,
    glyph: Option<f32>,
    xs: bool,
    md: bool,
    outline: bool,
    destructive: bool,
}

impl IconButton {
    pub fn new(icon: crate::Icon, label: impl Into<String>) -> Self {
        Self {
            icon,
            label: label.into(),
            glyph: None,
            xs: false,
            md: false,
            outline: false,
            destructive: false,
        }
    }

    /// The icon at `size` rather than the size's own (12 at `xs`, else 16), as a `size-3.5` icon.
    pub fn glyph(mut self, size: f32) -> Self {
        self.glyph = Some(size);
        self
    }

    /// The smallest size, `icon-xs`: `control-xs` square, the icon 12, its corners at most 10.
    pub fn xs(mut self) -> Self {
        self.xs = true;
        self
    }

    /// The control's size, `icon`: `control` square (a finger on a phone), for a button beside
    /// controls of the default size. Without it the button is `icon-sm`: `control-sm` square (a
    /// finger on a phone), its corners at most 12.
    pub fn md(mut self) -> Self {
        self.md = true;
        self
    }

    /// The outline variant: the outlined button's ground and border, the icon in the foreground ink.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Under the pointer the critical tint, for a button that removes or deletes.
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
}

impl egui::Widget for IconButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let t = crate::tokens(ui.ctx());
        let side = if self.xs {
            t.density("control-xs")
        } else if self.md {
            super::finger_tall(ui.ctx(), &t, t.density("control"))
        } else {
            super::finger_tall(ui.ctx(), &t, t.density("control-sm"))
        };
        let radius = if self.xs {
            let r = t.radius("control");
            egui::CornerRadius::same(r.nw.min(10))
        } else if self.md {
            t.radius("control")
        } else {
            let r = t.radius("control");
            egui::CornerRadius::same(r.nw.min(12))
        };
        let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.label));
        let hot = response.hovered();
        if self.outline {
            let (fill, border) = if t.dark {
                (t.color_at("input", 0.3), t.color("input"))
            } else {
                (t.color("background"), t.color("border"))
            };
            ui.painter().rect(
                rect,
                radius,
                fill,
                egui::Stroke::new(1.0, border),
                egui::StrokeKind::Inside,
            );
        }
        if hot && self.destructive {
            ui.painter().rect_filled(rect, radius, t.color_at("critical", 0.10));
        } else if hot {
            ui.painter().rect_filled(rect, radius, t.color("hover"));
        }
        let ink = match (hot, self.destructive) {
            (true, true) => t.color("critical-ink"),
            (true, false) => t.color("foreground"),
            _ if self.outline => t.color("foreground"),
            _ => t.color("muted-foreground"),
        };
        let glyph = self.glyph.unwrap_or(if self.xs { 12.0 } else { 16.0 });
        crate::icons::paint(ui.painter(), rect.center(), self.icon, glyph, ink);
        super::focus_ring(ui.painter(), &t, rect, radius, &response);
        let label = self.label;
        response.on_hover_text(label)
    }
}
