//! The transport's parts, after sokkel-ui's `media/MediaControls` and `media/time`: the play
//! button, the buttons beside it, the position, the speed menu, the skip glyph, the live mark, and
//! mute with the volume.

use egui::{
    Color32, CornerRadius, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Widget, WidgetInfo, pos2, vec2,
};

use super::media_slider::{MediaRange, MediaSlider, MediaTone};
use super::waveform::AudioScrubber;
use super::{DropdownMenu, MenuItem, dim, focus_ring};
use crate::{Icon, TextRole, Tokens, icons, mono_font, mono_size, tokens};

/// How far one arrow key moves the position, in seconds.
pub const SEEK_STEP: f64 = 5.0;
/// How far Page Up and Page Down move the position, in seconds.
pub const SEEK_LARGE_STEP: f64 = 30.0;

/// The playback speeds a speed menu offers unless told others, as multiples of normal speed.
pub const MEDIA_RATES: [f64; 7] = [0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];

/// A media clock: `m:ss` under an hour and `h:mm:ss` from an hour on; a value that is not a finite
/// positive number of seconds reads `0:00`. Fractions are dropped, so the clock turns over on the
/// second.
pub fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "0:00".into();
    }
    let total = seconds.floor() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let rest = total % 60;
    if hours == 0 {
        format!("{minutes}:{rest:02}")
    } else {
        format!("{hours}:{minutes:02}:{rest:02}")
    }
}

/// The time still to play, as a clock with a leading minus sign.
pub fn format_remaining(position: f64, duration: f64) -> String {
    let at = if position.is_finite() && position > 0.0 {
        position
    } else {
        0.0
    };
    format!("\u{2212}{}", format_time(duration - at))
}

/// A speed as the menu writes it, such as `1.25x`.
pub fn format_rate(rate: f64) -> String {
    format!("{rate}x")
}

/// How large the transport is drawn: `Sm` in a bar, `Md` in a tall panel, `Lg` on a screen of its
/// own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransportSize {
    #[default]
    Sm,
    Md,
    Lg,
}

/// The play button's words.
#[derive(Clone, Debug)]
pub struct PlayWords {
    pub play: String,
    pub pause: String,
}

impl Default for PlayWords {
    fn default() -> Self {
        Self {
            play: "Play".into(),
            pause: "Pause".into(),
        }
    }
}

/// The button's square and corners at a transport size: `icon-sm`, `icon` and `icon-lg` for the
/// buttons beside play.
fn square(t: &Tokens, ui: &Ui, size: TransportSize) -> (f32, CornerRadius) {
    let side = match size {
        TransportSize::Sm => t.density("control-sm"),
        TransportSize::Md => t.density("control"),
        TransportSize::Lg => t.density("control-lg"),
    };
    let radius = match size {
        TransportSize::Sm => CornerRadius::same(t.radius("control").nw.min(12)),
        _ => t.radius("control"),
    };
    (super::finger_tall(ui.ctx(), t, side), radius)
}

/// The play button at the center of the transport, the one filled control: a primary button (`icon`
/// at `Sm` and `Md`, `icon-lg` at `Lg`, which is 56 square on the surface's corners), its play or
/// pause glyph filled; over a picture, `terminal-foreground` with the `terminal` ink.
#[must_use = "add it with `ui.add(...)`"]
pub struct PlayButton {
    playing: bool,
    tone: MediaTone,
    size: TransportSize,
    words: PlayWords,
}

impl PlayButton {
    pub fn new(playing: bool) -> Self {
        Self {
            playing,
            tone: MediaTone::Surface,
            size: TransportSize::Sm,
            words: PlayWords::default(),
        }
    }

    pub fn tone(mut self, tone: MediaTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn size(mut self, size: TransportSize) -> Self {
        self.size = size;
        self
    }

    pub fn words(mut self, words: PlayWords) -> Self {
        self.words = words;
        self
    }
}

impl Widget for PlayButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (side, radius, glyph) = match self.size {
            TransportSize::Lg => (56.0, t.radius("surface"), 24.0),
            TransportSize::Md | TransportSize::Sm => (
                super::finger_tall(ui.ctx(), &t, t.density("control")),
                t.radius("control"),
                16.0,
            ),
        };
        let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
        let label = if self.playing {
            &self.words.pause
        } else {
            &self.words.play
        };
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, label));
        let hot = response.hovered();
        let (ground, ink) = match self.tone {
            MediaTone::Overlay => (
                if hot {
                    t.color_at("terminal-foreground", 0.85)
                } else {
                    t.color("terminal-foreground")
                },
                t.color("terminal"),
            ),
            MediaTone::Surface => {
                let state = if hot { "hover" } else { "rest" };
                let fallback = if hot {
                    t.color_at("primary", 0.8)
                } else {
                    t.color("primary")
                };
                match t.paint(&format!("button.default.{state}.container")) {
                    Some(p) => (
                        p.ground.unwrap_or(fallback),
                        p.ink.unwrap_or(t.color("primary-foreground")),
                    ),
                    None => (fallback, t.color("primary-foreground")),
                }
            }
        };
        let rect = if hot && response.is_pointer_button_down_on() {
            rect.translate(vec2(0.0, 1.0))
        } else {
            rect
        };
        let painter = ui.painter();
        painter.rect_filled(rect, radius, ground);
        focus_ring(painter, &t, rect, radius, &response);
        if self.playing {
            paint_pause(painter, rect.center(), glyph, ink);
        } else {
            paint_play(painter, rect.center() + vec2(1.0, 0.0), glyph, ink);
        }
        response.on_hover_text(label.clone())
    }
}

/// Lucide's play glyph filled, as `fill-current` fills it: the triangle `6 3, 20 12, 6 21` of 24.
pub fn paint_play(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let s = size / 24.0;
    let at = |x: f32, y: f32| center + vec2((x - 12.0) * s, (y - 12.0) * s);
    let points = vec![at(6.0, 3.0), at(20.0, 12.0), at(6.0, 21.0)];
    painter.add(Shape::convex_polygon(points, color, Stroke::new(2.0 * s, color)));
}

/// Lucide's pause glyph filled: two bars 4 by 16 at 6 and 14 of 24, their corners 1.
pub fn paint_pause(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let s = size / 24.0;
    for x in [6.0, 14.0] {
        let bar = Rect::from_min_size(center + vec2((x - 12.0) * s, -8.0 * s), vec2(4.0 * s, 16.0 * s)).expand(s);
        painter.rect_filled(bar, CornerRadius::same((2.0 * s).round() as u8), color);
    }
}

/// What a transport button shows: an icon, or the skip glyph.
#[derive(Clone, Copy, Debug)]
pub enum TransportGlyph {
    Icon(Icon),
    /// A turning arrow, back or forward, with the interval in seconds written inside it.
    Skip {
        seconds: f64,
        back: bool,
    },
}

impl From<Icon> for TransportGlyph {
    fn from(icon: Icon) -> Self {
        Self::Icon(icon)
    }
}

/// A transport button beside the play button (previous, next, a frame back, skip): a ghost square
/// (`icon-sm`, `icon` or `icon-lg` by size) with its icon in the foreground, 16 (20 at `Lg`);
/// `pressed` makes it a toggle, in `primary-ink` while on and the muted ink while off; over a
/// picture the icon is `terminal-foreground` on a `terminal-foreground/15` hover.
#[must_use = "add it with `ui.add(...)`"]
pub struct TransportButton {
    glyph: TransportGlyph,
    label: String,
    enabled: bool,
    pressed: Option<bool>,
    tone: MediaTone,
    size: TransportSize,
}

impl TransportButton {
    pub fn new(glyph: impl Into<TransportGlyph>, label: impl Into<String>) -> Self {
        Self {
            glyph: glyph.into(),
            label: label.into(),
            enabled: true,
            pressed: None,
            tone: MediaTone::Surface,
            size: TransportSize::Sm,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// For a toggle such as shuffle: drawn in the accent while on.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    pub fn tone(mut self, tone: MediaTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn size(mut self, size: TransportSize) -> Self {
        self.size = size;
        self
    }
}

/// A ghost square holding a glyph: the hover ground under the pointer and the ink `ink` gives.
fn ghost(
    ui: &mut Ui,
    side: f32,
    radius: CornerRadius,
    label: &str,
    enabled: bool,
    pressed: Option<bool>,
    tone: MediaTone,
) -> (Rect, Response, bool) {
    let t = tokens(ui.ctx());
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), sense);
    response.widget_info(|| match pressed {
        Some(on) => WidgetInfo::selected(egui::Role::Button, enabled, on, label),
        None => WidgetInfo::labeled(egui::Role::Button, enabled, label),
    });
    let hot = response.hovered() && enabled;
    if hot {
        let ground = match tone {
            MediaTone::Surface => t.color("hover"),
            MediaTone::Overlay => t.color_at("terminal-foreground", 0.15),
        };
        ui.painter().rect_filled(rect, radius, ground);
    }
    focus_ring(ui.painter(), &t, rect, radius, &response);
    (rect, response, hot)
}

impl Widget for TransportButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (side, radius) = square(&t, ui, self.size);
        let (rect, response, hot) = ghost(ui, side, radius, &self.label, self.enabled, self.pressed, self.tone);
        let ink = match (self.tone, self.pressed) {
            (MediaTone::Overlay, Some(true)) => t.color("terminal-accent"),
            (MediaTone::Overlay, _) => t.color("terminal-foreground"),
            (MediaTone::Surface, Some(true)) => t.color("primary-ink"),
            (MediaTone::Surface, Some(false)) if !hot => t.color("muted-foreground"),
            (MediaTone::Surface, _) => t.color("foreground"),
        };
        let ink = dim(ink, self.enabled);
        match self.glyph {
            TransportGlyph::Icon(icon) => {
                let glyph = if self.size == TransportSize::Lg { 20.0 } else { 16.0 };
                icons::paint(ui.painter(), rect.center(), icon, glyph, ink);
            }
            TransportGlyph::Skip { seconds, back } => paint_skip(ui, rect.center(), seconds, back, self.size, ink),
        }
        response.on_hover_text(self.label)
    }
}

/// The skip glyph: a turning arrow (24, 28 at `Lg`), back or forward, the interval in seconds
/// written inside it, semibold in `text.2xs` (`text.xs` at `Lg`).
pub fn paint_skip(ui: &Ui, center: Pos2, seconds: f64, back: bool, size: TransportSize, ink: Color32) {
    let t = tokens(ui.ctx());
    let arrow = if back { Icon::RotateCcw } else { Icon::RotateCw };
    let (glyph, role) = if size == TransportSize::Lg {
        (28.0, "xs")
    } else {
        (24.0, "2xs")
    };
    icons::paint(ui.painter(), center, arrow, glyph, ink);
    let spec = t.text(role);
    let text = egui::RichText::new(format!("{}", seconds.round() as i64))
        .font(egui::FontId::proportional(spec.size))
        .variation("wght", 600.0);
    let galley = egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::FontId::proportional(spec.size),
    );
    ui.painter()
        .galley(center - galley.size() / 2.0 + vec2(0.0, 1.0), galley, ink);
}

/// What stands where the position would on a live stream: a dot in `destructive` and its label in
/// `text.badge` (`text.label` at `Lg`), 20 tall (28 at `Lg`).
#[must_use = "add it with `ui.add(...)`"]
pub struct LiveMark {
    label: String,
    large: bool,
}

impl LiveMark {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            large: false,
        }
    }

    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }
}

impl Widget for LiveMark {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (role, height) = if self.large {
            (TextRole::Label, 28.0)
        } else {
            (TextRole::Badge, 20.0)
        };
        let galley = egui::WidgetText::from(role.rich(&t, &self.label)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            role.font(&t),
        );
        let width = 8.0 + 6.0 + galley.size().x;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width().max(width), height), Sense::hover());
        let left = rect.center().x - width / 2.0;
        ui.painter()
            .circle_filled(pos2(left + 4.0, rect.center().y), 4.0, t.color("destructive"));
        let ink = ui.visuals().text_color();
        ui.painter()
            .galley(pos2(left + 14.0, rect.center().y - galley.size().y / 2.0), galley, ink);
        response
    }
}

/// The glyph for a level: loud, quiet or silent.
pub fn volume_icon(level: f32) -> Icon {
    if level <= 0.0 {
        Icon::VolumeX
    } else if level < 0.5 {
        Icon::Volume1
    } else {
        Icon::Volume2
    }
}

/// The volume control's words.
#[derive(Clone, Debug)]
pub struct VolumeWords {
    pub mute: String,
    pub unmute: String,
    pub volume: String,
}

impl Default for VolumeWords {
    fn default() -> Self {
        Self {
            mute: "Mute".into(),
            unmute: "Unmute".into(),
            volume: "Volume".into(),
        }
    }
}

/// What the volume control was asked this frame.
#[derive(Clone, Debug, Default)]
pub struct VolumeResponse {
    /// Mute asked on or off.
    pub muted: Option<bool>,
    /// A level from 0 to 1, as the slider is dragged or stepped.
    pub volume: Option<f32>,
}

/// Mute and the volume slider: a ghost `icon-sm` button whose glyph says loud, quiet or silent in the
/// muted ink, and a 96 wide media slider from 0 to 100, hidden where `compact` says there is no room.
/// Moving the slider off zero while muted asks to unmute.
#[must_use = "show it with `.show(ui)`"]
pub struct VolumeControl {
    id_salt: egui::Id,
    level: f32,
    muted: bool,
    mutable: bool,
    tone: MediaTone,
    compact: bool,
    words: VolumeWords,
}

impl VolumeControl {
    /// `level` from 0 to 1, as heard: zero while muted.
    pub fn new(id_salt: impl egui::AsIdSalt, level: f32, muted: bool) -> Self {
        Self {
            id_salt: egui::Id::unique(id_salt),
            level,
            muted,
            mutable: true,
            tone: MediaTone::Surface,
            compact: false,
            words: VolumeWords::default(),
        }
    }

    /// Whether the mute button works; without it the button is disabled, as without `onMutedChange`.
    pub fn mutable(mut self, mutable: bool) -> Self {
        self.mutable = mutable;
        self
    }

    pub fn tone(mut self, tone: MediaTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn words(mut self, words: VolumeWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> VolumeResponse {
        let t = tokens(ui.ctx());
        let mut out = VolumeResponse::default();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let (side, radius) = square(&t, ui, TransportSize::Sm);
            let label = if self.muted {
                &self.words.unmute
            } else {
                &self.words.mute
            };
            let (rect, response, hot) = ghost(ui, side, radius, label, self.mutable, Some(self.muted), self.tone);
            let ink = match self.tone {
                MediaTone::Overlay => t.color("terminal-foreground"),
                MediaTone::Surface if hot => t.color("foreground"),
                MediaTone::Surface => t.color("muted-foreground"),
            };
            icons::paint(
                ui.painter(),
                rect.center(),
                volume_icon(self.level),
                16.0,
                dim(ink, self.mutable),
            );
            if response.on_hover_text(label.clone()).clicked() {
                out.muted = Some(!self.muted);
            }
            if !self.compact {
                let slider = MediaSlider::new(
                    self.id_salt.with("level"),
                    (self.level * 100.0).round() as f64,
                    100.0,
                    &self.words.volume,
                )
                .value_text(|v| format!("{}%", v.round() as i64))
                .tone(self.tone)
                .width(96.0)
                .show(ui);
                if let Some(v) = slider.changed {
                    out.volume = Some((v / 100.0) as f32);
                    if self.muted && v > 0.0 && self.mutable {
                        out.muted = Some(false);
                    }
                }
                if let Some(v) = slider.scrub {
                    out.volume = Some((v / 100.0) as f32);
                }
            }
        });
        out
    }
}

/// What the position was asked this frame.
#[derive(Clone, Debug, Default)]
pub struct PositionResponse {
    /// The time chosen, once a drag is let go or at once for a key.
    pub seek: Option<f64>,
    /// The time under the pointer while dragged.
    pub scrub: Option<f64>,
    pub scrub_ended: bool,
}

/// The seek slider between the elapsed and the remaining time, both in the code face (`help` size, 44
/// wide; `label` size and 56 wide at `large`). Given the track's `waveform`, the slider is drawn as
/// that waveform with the played part filled, its bars widening once it is 64 tall.
#[must_use = "show it with `.show(ui)`"]
pub struct Position<'a> {
    id_salt: egui::Id,
    at: f64,
    duration: f64,
    label: String,
    buffered: &'a [MediaRange],
    tone: MediaTone,
    waveform: Option<&'a [f32]>,
    height: f32,
    large: bool,
}

impl<'a> Position<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, at: f64, duration: f64, label: impl Into<String>) -> Self {
        Self {
            id_salt: egui::Id::unique(id_salt),
            at,
            duration,
            label: label.into(),
            buffered: &[],
            tone: MediaTone::Surface,
            waveform: None,
            height: 40.0,
            large: false,
        }
    }

    pub fn buffered(mut self, ranges: &'a [MediaRange]) -> Self {
        self.buffered = ranges;
        self
    }

    pub fn tone(mut self, tone: MediaTone) -> Self {
        self.tone = tone;
        self
    }

    /// The track's levels from start to end, each from 0 to 1.
    pub fn waveform(mut self, levels: &'a [f32]) -> Self {
        self.waveform = Some(levels);
        self
    }

    /// The waveform's height; 40 by default.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// The times written larger, for a screen of its own.
    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    pub fn show(self, ui: &mut Ui) -> PositionResponse {
        let t = tokens(ui.ctx());
        let (spec, room, gap) = if self.large {
            (t.text("sm"), 56.0, 16.0)
        } else {
            (t.text("xs"), 44.0, 8.0)
        };
        let font = mono_font(ui.ctx(), mono_size(&t, spec.size), 400.0);
        let (ink, rest_ink) = match self.tone {
            MediaTone::Surface => (t.color("muted-foreground"), t.color("muted-foreground")),
            MediaTone::Overlay => (t.color("terminal-foreground"), t.color("terminal-muted")),
        };
        let duration = self.duration;
        let value_text = move |v: f64| format!("{} / {}", format_time(v), format_time(duration));
        let mut out = PositionResponse::default();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            let row = self.height.max(16.0);
            let (left, _) = ui.allocate_exact_size(vec2(room, row), Sense::hover());
            let elapsed = ui.painter().layout_no_wrap(format_time(self.at), font.clone(), ink);
            ui.painter().galley(
                pos2(
                    left.right() - elapsed.size().x,
                    left.center().y - elapsed.size().y / 2.0,
                ),
                elapsed,
                ink,
            );
            let middle = (ui.available_width() - room - gap).max(0.0);
            ui.allocate_ui_with_layout(
                vec2(middle, row),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| match self.waveform {
                    None => {
                        let r = MediaSlider::new(self.id_salt, self.at, duration, &self.label)
                            .value_text(value_text)
                            .buffered(self.buffered)
                            .step(SEEK_STEP)
                            .large_step(SEEK_LARGE_STEP)
                            .tone(self.tone)
                            .width(middle)
                            .show(ui);
                        out.seek = r.changed;
                        out.scrub = r.scrub;
                        out.scrub_ended = r.scrub_ended;
                    }
                    Some(levels) => {
                        let broad = self.height >= 64.0;
                        let r = AudioScrubber::new(self.id_salt, levels, self.at, duration)
                            .label(&self.label)
                            .value_text(value_text)
                            .step(SEEK_STEP)
                            .large_step(SEEK_LARGE_STEP)
                            .bars(if broad { 4.0 } else { 3.0 }, 2.0, if broad { 2.0 } else { 1.0 })
                            .height(self.height)
                            .show(ui);
                        out.seek = r.seek;
                        out.scrub = r.scrub;
                        out.scrub_ended = r.scrub_ended;
                    }
                },
            );
            let (right, _) = ui.allocate_exact_size(vec2(room, row), Sense::hover());
            let remaining = ui
                .painter()
                .layout_no_wrap(format_remaining(self.at, duration), font, rest_ink);
            ui.painter().galley(
                pos2(right.left(), right.center().y - remaining.size().y / 2.0),
                remaining,
                rest_ink,
            );
        });
        out
    }
}

/// The playback speed as a ghost button that says it (`1.25x`, in the code face) and opens a menu of
/// `rates`, the current one checked; `show` gives the rate chosen this frame.
#[must_use = "show it with `.show(ui)`"]
pub struct RateMenu<'a> {
    rate: f64,
    rates: &'a [f64],
    label: String,
    tone: MediaTone,
    size: TransportSize,
}

impl<'a> RateMenu<'a> {
    /// `label` names the menu, such as "Playback speed".
    pub fn new(rate: f64, label: impl Into<String>) -> Self {
        Self {
            rate,
            rates: &MEDIA_RATES,
            label: label.into(),
            tone: MediaTone::Surface,
            size: TransportSize::Sm,
        }
    }

    pub fn rates(mut self, rates: &'a [f64]) -> Self {
        self.rates = rates;
        self
    }

    pub fn tone(mut self, tone: MediaTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn size(mut self, size: TransportSize) -> Self {
        self.size = size;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<f64> {
        let t = tokens(ui.ctx());
        let (side, radius) = square(
            &t,
            ui,
            if self.size == TransportSize::Sm {
                TransportSize::Sm
            } else {
                TransportSize::Md
            },
        );
        let spec = t.text("xs");
        let font = mono_font(ui.ctx(), mono_size(&t, spec.size), 400.0);
        let written = format_rate(self.rate);
        let galley = ui.painter().layout_no_wrap(written.clone(), font, Color32::PLACEHOLDER);
        let width = (galley.size().x + 12.0).max(32.0);
        let label = format!("{}: {written}", self.label);
        let sense = Sense::click();
        let (rect, response) = ui.allocate_exact_size(vec2(width, side), sense);
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &label));
        let hot = response.hovered();
        if hot {
            let ground = match self.tone {
                MediaTone::Surface => t.color("hover"),
                MediaTone::Overlay => t.color_at("terminal-foreground", 0.15),
            };
            ui.painter().rect_filled(rect, radius, ground);
        }
        focus_ring(ui.painter(), &t, rect, radius, &response);
        let ink = match self.tone {
            MediaTone::Overlay => t.color("terminal-foreground"),
            MediaTone::Surface if hot => t.color("foreground"),
            MediaTone::Surface => t.color("muted-foreground"),
        };
        ui.painter().galley(rect.center() - galley.size() / 2.0, galley, ink);
        let items: Vec<MenuItem> = self
            .rates
            .iter()
            .map(|r| {
                let item = MenuItem::action(format_rate(*r)).code(true);
                if (*r - self.rate).abs() < 1e-9 {
                    item.icon(Icon::Check)
                } else {
                    item
                }
            })
            .collect();
        let chosen = DropdownMenu::new(&items).end(true).width(96.0).show(&response);
        chosen.map(|i| self.rates[i])
    }
}
