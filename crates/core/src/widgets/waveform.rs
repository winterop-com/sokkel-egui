//! A recording's loudness as a row of bars, after sokkel-ui's `primitives/waveform`: `Waveform`, the
//! levels a seed makes (`StaticWaveform`'s), and `AudioScrubber`, a seek control drawn as one.

use egui::{Color32, CornerRadius, Key, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget, pos2, vec2};

use crate::tokens;

/// Bars drawn from levels between 0 and 1, centered down their room: a bar `bar_width` wide every
/// `bar_width + bar_gap`, each level's share of 80 percent of the height and never under
/// `bar_height`; a quieter bar fainter too while `shade`, and both ends faded over `fade_width`.
#[must_use = "add it with `ui.add(...)`, or paint it with `paint`"]
#[derive(Clone, Debug)]
pub struct Waveform<'a> {
    data: &'a [f32],
    bar_width: f32,
    bar_height: f32,
    bar_gap: f32,
    bar_radius: f32,
    color: Option<Color32>,
    fade_edges: bool,
    fade_width: f32,
    shade: bool,
    height: Option<f32>,
}

impl<'a> Waveform<'a> {
    pub fn new(data: &'a [f32]) -> Self {
        Self {
            data,
            bar_width: 4.0,
            bar_height: 4.0,
            bar_gap: 2.0,
            bar_radius: 2.0,
            color: None,
            fade_edges: true,
            fade_width: 24.0,
            shade: true,
            height: Some(128.0),
        }
    }

    pub fn bar_width(mut self, width: f32) -> Self {
        self.bar_width = width;
        self
    }

    /// The shortest a bar is drawn.
    pub fn bar_height(mut self, height: f32) -> Self {
        self.bar_height = height;
        self
    }

    pub fn bar_gap(mut self, gap: f32) -> Self {
        self.bar_gap = gap;
        self
    }

    pub fn bar_radius(mut self, radius: f32) -> Self {
        self.bar_radius = radius;
        self
    }

    /// The bars' color; the foreground by default, as the web's bars take the text's color.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn fade_edges(mut self, fade: bool) -> Self {
        self.fade_edges = fade;
        self
    }

    pub fn fade_width(mut self, width: f32) -> Self {
        self.fade_width = width;
        self
    }

    /// Whether a quieter bar is drawn fainter as well as shorter.
    pub fn shade(mut self, shade: bool) -> Self {
        self.shade = shade;
        self
    }

    /// How tall it is drawn; `None` takes the height the `ui` has left, as `height="100%"`.
    pub fn height(mut self, height: Option<f32>) -> Self {
        self.height = height;
        self
    }

    /// Paint the bars into `rect`.
    pub fn paint(&self, painter: &Painter, rect: Rect) {
        let t = tokens(painter.ctx());
        let color = self.color.unwrap_or_else(|| t.color("foreground"));
        let step = self.bar_width + self.bar_gap;
        if step <= 0.0 || rect.width() <= 0.0 || self.data.is_empty() {
            return;
        }
        let count = (rect.width() / step).floor() as usize;
        let fade = if self.fade_edges && self.fade_width > 0.0 {
            (self.fade_width / rect.width()).min(0.2)
        } else {
            0.0
        };
        for i in 0..count {
            let index = ((i as f32 / count as f32) * self.data.len() as f32).floor() as usize;
            let value = self.data.get(index).copied().unwrap_or(0.0).max(0.0);
            let height = (value * rect.height() * 0.8).max(self.bar_height);
            let x = rect.left() + i as f32 * step;
            let bar = Rect::from_min_size(pos2(x, rect.center().y - height / 2.0), vec2(self.bar_width, height));
            let mut alpha = if self.shade { 0.3 + value.min(1.0) * 0.7 } else { 1.0 };
            if fade > 0.0 {
                // erased through a gradient, opaque at either end and clear past `fade` of the width
                let share = (bar.center().x - rect.left()) / rect.width();
                let edge = share.min(1.0 - share);
                alpha *= (edge / fade).clamp(0.0, 1.0);
            }
            let radius = CornerRadius::same(self.bar_radius.min(self.bar_width / 2.0).round() as u8);
            painter.rect_filled(bar, radius, color.gamma_multiply(alpha));
        }
    }
}

impl Widget for Waveform<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let height = self.height.unwrap_or_else(|| ui.available_height());
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect);
        }
        response
    }
}

/// The made-up levels `StaticWaveform` draws: `bars` of them, the same for the same `seed`.
pub fn static_levels(bars: usize, seed: u32) -> Vec<f32> {
    let random = |value: f64| {
        let x = value.sin() * 10000.0;
        x - x.floor()
    };
    (0..bars)
        .map(|i| (0.2 + random(seed as f64 + i as f64) * 0.6) as f32)
        .collect()
}

/// The levels drawn while a recording's own are not known: an even row, so no shape is made up.
const FLAT_LEVELS: [f32; 64] = [0.3; 64];

/// What a scrubber was asked this frame.
#[derive(Clone, Debug)]
pub struct AudioScrubberResponse {
    pub response: Response,
    /// The time chosen: on a press let go, at the end of a drag, and on each key.
    pub seek: Option<f64>,
    /// The time under the pointer while it drags.
    pub scrub: Option<f64>,
    /// The drag ended this frame (the web's `onScrub(null)`).
    pub scrub_ended: bool,
}

/// A scrubber drawn as a recording's waveform, after sokkel-ui's `AudioScrubber`: the bars already
/// played filled in the accent and the rest in `faint/50`, a round handle at the playhead while it
/// is hovered, focused or dragged. A press or a drag picks a time; the arrows step by `step`, Page
/// Up and Page Down (or a shifted arrow) by `large_step`, Home and End to either end.
#[must_use = "show it with `.show(ui)`"]
pub struct AudioScrubber<'a> {
    id_salt: egui::Id,
    data: &'a [f32],
    current: f64,
    duration: f64,
    show_handle: bool,
    step: f64,
    large_step: f64,
    label: String,
    value_text: Option<Box<dyn Fn(f64) -> String + 'a>>,
    bar_width: f32,
    bar_gap: f32,
    bar_radius: f32,
    color: Option<Color32>,
    played: Option<Color32>,
    shade: bool,
    height: f32,
}

impl<'a> AudioScrubber<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, data: &'a [f32], current: f64, duration: f64) -> Self {
        Self {
            id_salt: egui::Id::unique(id_salt),
            data,
            current,
            duration,
            show_handle: true,
            step: 5.0,
            large_step: 30.0,
            label: "Seek".into(),
            value_text: None,
            bar_width: 3.0,
            bar_gap: 1.0,
            bar_radius: 1.0,
            color: None,
            played: None,
            shade: false,
            height: 128.0,
        }
    }

    pub fn show_handle(mut self, show: bool) -> Self {
        self.show_handle = show;
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

    /// The scrubber's accessible name; "Seek" by default.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// What a screen reader says for a time.
    pub fn value_text(mut self, text: impl Fn(f64) -> String + 'a) -> Self {
        self.value_text = Some(Box::new(text));
        self
    }

    pub fn bars(mut self, width: f32, gap: f32, radius: f32) -> Self {
        self.bar_width = width;
        self.bar_gap = gap;
        self.bar_radius = radius;
        self
    }

    /// The bars still to play; `faint/50` by default.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    /// The played bars; the primary by default.
    pub fn played_color(mut self, color: Color32) -> Self {
        self.played = Some(color);
        self
    }

    pub fn shade(mut self, shade: bool) -> Self {
        self.shade = shade;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn show(self, ui: &mut Ui) -> AudioScrubberResponse {
        let t = tokens(ui.ctx());
        let seekable = self.duration > 0.0;
        let id = ui.make_persistent_id(self.id_salt);
        let sense = if seekable {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let response = ui.interact(rect, id, sense);
        let clamp = |time: f64| time.clamp(0.0, self.duration.max(0.0));
        let time_at = |x: f32| {
            if rect.width() <= 0.0 {
                0.0
            } else {
                ((x - rect.left()).clamp(0.0, rect.width()) / rect.width()) as f64 * self.duration
            }
        };
        let mut out = AudioScrubberResponse {
            response: response.clone(),
            seek: None,
            scrub: None,
            scrub_ended: false,
        };
        let dragged_key = id.with("dragged");
        let mut dragged: Option<f64> = ui.data(|d| d.get_temp(dragged_key)).flatten();
        if seekable {
            if let Some(p) = response.interact_pointer_pos()
                && (response.is_pointer_button_down_on() || response.dragged())
            {
                response.request_focus();
                let at = time_at(p.x);
                dragged = Some(at);
                out.scrub = Some(at);
            }
            if (response.drag_stopped() || response.clicked())
                && let Some(at) = dragged.take()
            {
                out.seek = Some(at);
                out.scrub_ended = true;
            }
            if response.has_focus() {
                let time = clamp(self.current);
                let (by, large) = (self.step, self.large_step);
                let next = ui.input(|i| {
                    let by = if i.modifiers.shift { large } else { by };
                    if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::ArrowUp) {
                        Some(time + by)
                    } else if i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowDown) {
                        Some(time - by)
                    } else if i.key_pressed(Key::PageUp) {
                        Some(time + large)
                    } else if i.key_pressed(Key::PageDown) {
                        Some(time - large)
                    } else if i.key_pressed(Key::Home) {
                        Some(0.0)
                    } else if i.key_pressed(Key::End) {
                        Some(self.duration)
                    } else {
                        None
                    }
                });
                if let Some(next) = next {
                    out.seek = Some(clamp(next));
                }
            }
        }
        ui.data_mut(|d| d.insert_temp(dragged_key, dragged));
        let time = clamp(dragged.unwrap_or(self.current));
        response.widget_info(|| {
            let mut info = egui::WidgetInfo::labeled(egui::Role::Slider, seekable, &self.label);
            info.value = Some(time);
            if let Some(text) = &self.value_text {
                info.current_text_value = Some(text(time));
            }
            info
        });
        if !ui.is_rect_visible(rect) {
            return out;
        }
        let levels: &[f32] = if self.data.is_empty() { &FLAT_LEVELS } else { self.data };
        let played = if seekable { (time / self.duration) as f32 } else { 0.0 };
        let bars = |color: Color32| {
            Waveform::new(levels)
                .bars_of(self.bar_width, self.bar_gap, self.bar_radius)
                .shade(self.shade)
                .fade_edges(false)
                .color(color)
        };
        let enabled = seekable;
        let rest = self.color.unwrap_or_else(|| t.color_at("faint", 0.5));
        bars(super::dim(rest, enabled)).paint(ui.painter(), rect);
        let cut = Rect::from_min_max(rect.min, pos2(rect.left() + rect.width() * played, rect.bottom()));
        let played_ink = self.played.unwrap_or_else(|| t.color("primary"));
        bars(super::dim(played_ink, enabled)).paint(&ui.painter().with_clip_rect(cut), rect);
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect,
                t.radius("mini"),
                Stroke::new(3.0, t.color_at("ring", 0.5)),
                StrokeKind::Outside,
            );
        }
        let shown = response.hovered() || response.has_focus() || dragged.is_some();
        if self.show_handle && seekable && shown {
            let center = pos2(cut.right(), rect.center().y);
            ui.painter()
                .circle(center, 6.0, t.color("background"), Stroke::new(1.0, t.color("primary")));
        }
        out
    }
}

impl<'a> Waveform<'a> {
    fn bars_of(self, width: f32, gap: f32, radius: f32) -> Self {
        self.bar_width(width).bar_gap(gap).bar_radius(radius)
    }
}
