//! A horizontal slider for a media position or a level, after sokkel-ui's `media/MediaSlider`.

use egui::{Color32, Key, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::{Tokens, tokens};

/// Where media controls sit: on the app's own surface, or over a picture, light on dark in either
/// scheme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MediaTone {
    #[default]
    Surface,
    Overlay,
}

/// A stretch of media that is loaded, in seconds from the start (or the slider's units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaRange {
    pub start: f64,
    pub end: f64,
}

/// What a media slider was asked this frame.
#[derive(Clone, Debug)]
pub struct MediaSliderResponse {
    pub response: Response,
    /// The value chosen: once, when a drag is let go, and at once on a key (the web's `onChange`).
    pub changed: Option<f64>,
    /// The value under the pointer while the thumb is dragged (the web's `onScrub`).
    pub scrub: Option<f64>,
    /// The drag ended this frame (the web's `onScrub(null)`).
    pub scrub_ended: bool,
}

/// A horizontal slider for a media position or a level: a 4 tall track on `muted` (`terminal-
/// foreground/25` over a picture) with the loaded stretches in `border-strong` under the played part
/// in `primary`, and a 12 round thumb kept inside the track's ends. The arrows step by `step`, Page
/// Up and Page Down by `large_step`, Home and End go to either end. While the thumb is dragged the
/// slider shows the dragged value and reports it as `scrub`; `changed` comes once, when the drag is
/// let go, and at once for a key.
#[must_use = "show it with `.show(ui)`"]
pub struct MediaSlider<'a> {
    id_salt: egui::Id,
    value: f64,
    max: f64,
    label: String,
    value_text: Option<Box<dyn Fn(f64) -> String + 'a>>,
    buffered: &'a [MediaRange],
    step: f64,
    large_step: f64,
    tone: MediaTone,
    enabled: bool,
    width: Option<f32>,
}

impl<'a> MediaSlider<'a> {
    /// `label` is the slider's accessible name.
    pub fn new(id_salt: impl egui::AsIdSalt, value: f64, max: f64, label: impl Into<String>) -> Self {
        Self {
            id_salt: egui::Id::unique(id_salt),
            value,
            max,
            label: label.into(),
            value_text: None,
            buffered: &[],
            step: 1.0,
            large_step: 10.0,
            tone: MediaTone::Surface,
            enabled: true,
            width: None,
        }
    }

    /// What a screen reader says for a value.
    pub fn value_text(mut self, text: impl Fn(f64) -> String + 'a) -> Self {
        self.value_text = Some(Box::new(text));
        self
    }

    /// Loaded stretches, in the slider's units.
    pub fn buffered(mut self, ranges: &'a [MediaRange]) -> Self {
        self.buffered = ranges;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    pub fn large_step(mut self, step: f64) -> Self {
        self.large_step = step;
        self
    }

    pub fn tone(mut self, tone: MediaTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// How wide it is drawn; the width the `ui` has left by default, as `w-full`.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn show(self, ui: &mut Ui) -> MediaSliderResponse {
        let t = tokens(ui.ctx());
        let top = if self.max > 0.0 { self.max } else { 1.0 };
        let enabled = self.enabled && self.max > 0.0;
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let id = ui.make_persistent_id(self.id_salt);
        let (rect, _) = ui.allocate_exact_size(vec2(width, 16.0), Sense::hover());
        let sense = if enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let response = ui.interact(rect, id, sense);
        let scrub_key = id.with("scrub");
        let mut scrub: Option<f64> = ui.data(|d| d.get_temp(scrub_key)).flatten();
        let mut out = MediaSliderResponse {
            response: response.clone(),
            changed: None,
            scrub: None,
            scrub_ended: false,
        };
        let value_at = |x: f32| -> f64 {
            // the thumb stays inside the track, its center 6 in from either end
            let inner = (rect.width() - 12.0).max(1.0);
            (((x - rect.left() - 6.0) / inner).clamp(0.0, 1.0) as f64) * top
        };
        if enabled {
            if let Some(p) = response.interact_pointer_pos()
                && (response.is_pointer_button_down_on() || response.dragged())
            {
                response.request_focus();
                let v = value_at(p.x);
                scrub = Some(v);
                out.scrub = Some(v);
            }
            if (response.drag_stopped() || response.clicked())
                && let Some(v) = scrub.take()
            {
                out.changed = Some(v);
                out.scrub_ended = true;
            }
            if response.has_focus() {
                let now = self.value.clamp(0.0, top);
                let next = ui.input(|i| {
                    if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::ArrowUp) {
                        Some(now + self.step)
                    } else if i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowDown) {
                        Some(now - self.step)
                    } else if i.key_pressed(Key::PageUp) {
                        Some(now + self.large_step)
                    } else if i.key_pressed(Key::PageDown) {
                        Some(now - self.large_step)
                    } else if i.key_pressed(Key::Home) {
                        Some(0.0)
                    } else if i.key_pressed(Key::End) {
                        Some(top)
                    } else {
                        None
                    }
                });
                if let Some(next) = next {
                    out.changed = Some(next.clamp(0.0, top));
                }
            }
        }
        ui.data_mut(|d| d.insert_temp(scrub_key, scrub));
        let given = if self.value.is_finite() { self.value } else { 0.0 };
        let shown = scrub.unwrap_or(given).clamp(0.0, top);
        response.widget_info(|| {
            let mut info = egui::WidgetInfo::labeled(egui::Role::Slider, enabled, &self.label);
            info.value = Some(shown);
            if let Some(text) = &self.value_text {
                info.current_text_value = Some(text(shown));
            }
            info
        });
        if ui.is_rect_visible(rect) {
            let (track, loaded, thumb_ground, thumb_edge) = match self.tone {
                MediaTone::Surface => (
                    t.color("muted"),
                    t.color("border-strong"),
                    t.color("background"),
                    t.color("primary"),
                ),
                MediaTone::Overlay => (
                    t.color_at("terminal-foreground", 0.25),
                    t.color_at("terminal-foreground", 0.45),
                    t.color("terminal-foreground"),
                    t.color("terminal-foreground"),
                ),
            };
            let fraction = (shown / top) as f32;
            let lit = response.hovered() || response.has_focus() || scrub.is_some();
            paint_slider(
                ui,
                &t,
                rect,
                fraction,
                SliderPaint {
                    track,
                    fill: t.color("primary"),
                    thumb_ground,
                    thumb_edge,
                    radius: t.radius("mini"),
                    halo: lit && enabled,
                    focus_outline: false,
                    enabled,
                },
                |painter, bar| {
                    for range in self.buffered {
                        let from = (range.start.max(0.0) / top) as f32;
                        let to = (range.end.min(top).max(range.start.max(0.0)) / top) as f32;
                        let r = Rect::from_x_y_ranges(
                            (bar.left() + bar.width() * from)..=(bar.left() + bar.width() * to),
                            bar.y_range(),
                        );
                        painter.rect_filled(r, 0.0, super::dim(loaded, enabled));
                    }
                },
                &response,
            );
        }
        out
    }
}

/// How a slider is painted.
pub(crate) struct SliderPaint {
    pub track: Color32,
    pub fill: Color32,
    pub thumb_ground: Color32,
    pub thumb_edge: Color32,
    pub radius: egui::CornerRadius,
    /// The `ring/50` halo 3 wide round the thumb, on hover, focus and while dragged.
    pub halo: bool,
    /// The frame's focus outline round the thumb rather than the halo, as `primitives/slider` draws it.
    pub focus_outline: bool,
    pub enabled: bool,
}

/// A slider's track, its fill to `fraction` and its 12 round thumb, the thumb's center kept 6 in from
/// either end, as `thumbAlignment="edge"` keeps it; `under` paints what lies under the fill.
pub(crate) fn paint_slider(
    ui: &Ui,
    t: &Tokens,
    rect: Rect,
    fraction: f32,
    paint: SliderPaint,
    under: impl FnOnce(&egui::Painter, Rect),
    response: &Response,
) {
    let painter = ui.painter().with_clip_rect(ui.clip_rect());
    let dim = |c: Color32| super::dim(c, paint.enabled);
    let bar = Rect::from_center_size(rect.center(), vec2(rect.width(), 4.0));
    painter.rect_filled(bar, paint.radius, dim(paint.track));
    let x = rect.left() + 6.0 + (rect.width() - 12.0).max(0.0) * fraction.clamp(0.0, 1.0);
    let clipped = painter.with_clip_rect(bar.intersect(painter.clip_rect()));
    under(&clipped, bar);
    let fill = Rect::from_min_max(bar.min, pos2(x, bar.bottom()));
    clipped.rect_filled(fill, paint.radius, dim(paint.fill));
    let center = pos2(x, rect.center().y);
    if paint.halo {
        painter.circle_filled(center, 6.0 + 3.0, t.color_at("ring", 0.5));
    }
    painter.circle(
        center,
        6.0,
        dim(paint.thumb_ground),
        Stroke::new(1.0, dim(paint.thumb_edge)),
    );
    if paint.focus_outline && response.has_focus() {
        let width = t.length("focus-ring-width");
        let offset = t.length("focus-ring-offset");
        let ring = Rect::from_center_size(center, vec2(12.0, 12.0)).expand(offset);
        painter.rect_stroke(
            ring,
            egui::CornerRadius::same(255),
            Stroke::new(width, t.color("ring")),
            StrokeKind::Outside,
        );
    }
}
