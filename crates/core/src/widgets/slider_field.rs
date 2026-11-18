//! A number chosen on a slider, after sokkel-ui's `primitives/slider` (`Slider`) and `SliderField`.

use egui::{Key, Response, Sense, TextWrapMode, Ui, Widget, WidgetText, pos2, vec2};

use super::media_slider::{SliderPaint, paint_slider};
use crate::{TextRole, mono_font, mono_size, tokens};

/// A horizontal slider over a number, after sokkel-ui's `primitives/slider`: a 4 tall round track on
/// `muted`, its range in `primary`, and a 12 round thumb on `on-shade` edged in `ring`, the `ring/50`
/// halo round it under the pointer and while dragged, the frame's focus outline from the keyboard.
/// The arrows step by `step`, Page Up and Page Down by ten steps, Home and End to either end.
#[must_use = "add it with `ui.add(...)`"]
pub struct Slider<'a> {
    value: &'a mut f64,
    min: f64,
    max: f64,
    step: f64,
    label: String,
    enabled: bool,
    width: Option<f32>,
}

impl<'a> Slider<'a> {
    /// `label` is the slider's accessible name.
    pub fn new(value: &'a mut f64, min: f64, max: f64, label: impl Into<String>) -> Self {
        Self {
            value,
            min,
            max,
            step: 1.0,
            label: label.into(),
            enabled: true,
            width: None,
        }
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// How wide it is drawn; the width the `ui` has left by default.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl Widget for Slider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let sense = if self.enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let (rect, mut response) = ui.allocate_exact_size(vec2(width, 12.0), sense);
        let span = (self.max - self.min).max(f64::EPSILON);
        let snap = |v: f64| {
            let stepped = if self.step > 0.0 {
                self.min + ((v - self.min) / self.step).round() * self.step
            } else {
                v
            };
            stepped.clamp(self.min, self.max)
        };
        let before = *self.value;
        if self.enabled {
            if let Some(p) = response.interact_pointer_pos()
                && (response.is_pointer_button_down_on() || response.dragged())
            {
                response.request_focus();
                let inner = (rect.width() - 12.0).max(1.0);
                let share = ((p.x - rect.left() - 6.0) / inner).clamp(0.0, 1.0) as f64;
                *self.value = snap(self.min + share * span);
            }
            if response.has_focus() {
                let now = *self.value;
                let step = self.step.max(f64::EPSILON);
                let next = ui.input(|i| {
                    if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::ArrowUp) {
                        Some(now + step)
                    } else if i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowDown) {
                        Some(now - step)
                    } else if i.key_pressed(Key::PageUp) {
                        Some(now + step * 10.0)
                    } else if i.key_pressed(Key::PageDown) {
                        Some(now - step * 10.0)
                    } else if i.key_pressed(Key::Home) {
                        Some(self.min)
                    } else if i.key_pressed(Key::End) {
                        Some(self.max)
                    } else {
                        None
                    }
                });
                if let Some(next) = next {
                    *self.value = snap(next);
                }
            }
        }
        if *self.value != before {
            response.mark_changed();
        }
        let value = *self.value;
        response.widget_info(|| {
            let mut info = egui::WidgetInfo::labeled(egui::Role::Slider, self.enabled, &self.label);
            info.value = Some(value);
            info
        });
        if ui.is_rect_visible(rect) {
            let rect = rect.expand2(vec2(0.0, 2.0));
            let lit = response.hovered() || response.dragged();
            paint_slider(
                ui,
                &t,
                rect,
                ((value - self.min) / span) as f32,
                SliderPaint {
                    track: t.color("muted"),
                    fill: t.color("primary"),
                    thumb_ground: t.color("on-shade"),
                    thumb_edge: t.color("ring"),
                    radius: egui::CornerRadius::same(255),
                    halo: lit && self.enabled,
                    focus_outline: true,
                    enabled: self.enabled,
                },
                |_, _| {},
                &response,
            );
        }
        response
    }
}

/// A number chosen on a slider with its value written out in figures, after sokkel-ui's
/// `SliderField`: the name in `text.label` and the value in the code face (`help` size, muted) on a
/// line above the slider, an optional help line under it; `inline` drops the name and writes the
/// value beside a 224 wide field instead.
#[must_use = "add it with `ui.add(...)`"]
pub struct SliderField<'a> {
    label: String,
    value: &'a mut f64,
    min: f64,
    max: f64,
    step: f64,
    unit: Option<String>,
    format: Option<Box<dyn Fn(f64) -> String + 'a>>,
    help: Option<String>,
    inline: bool,
}

impl<'a> SliderField<'a> {
    pub fn new(label: impl Into<String>, value: &'a mut f64, max: f64) -> Self {
        Self {
            label: label.into(),
            value,
            min: 0.0,
            max,
            step: 1.0,
            unit: None,
            format: None,
            help: None,
            inline: false,
        }
    }

    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Written after the value: `ms`, `px`, `%`.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// How the value is written, such as to one decimal; as a plain number by default.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'a) -> Self {
        self.format = Some(Box::new(format));
        self
    }

    /// A sentence under the slider.
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// The value beside the slider and no name, which a settings row already says.
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }
}

/// A number as JavaScript's `String(number)` writes it: no fraction when it is whole.
fn plain(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

impl Widget for SliderField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let formatted = match &self.format {
            Some(format) => format(*self.value),
            None => plain(*self.value),
        };
        let written = match &self.unit {
            Some(unit) => format!("{formatted} {unit}"),
            None => formatted,
        };
        let spec = t.text("help");
        let mono = mono_font(ui.ctx(), mono_size(&t, spec.size), spec.weight);
        let muted = t.color("muted-foreground");
        let slider = Slider::new(self.value, self.min, self.max, self.label.clone()).step(self.step);
        if self.inline {
            return ui
                .allocate_ui_with_layout(
                    vec2(224.0, 20.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        let r = ui.add(slider.width(224.0 - 64.0 - 12.0));
                        let galley = ui.painter().layout_no_wrap(written, mono, muted);
                        let (rect, _) = ui.allocate_exact_size(vec2(64.0, 20.0), Sense::hover());
                        ui.painter().galley(
                            pos2(rect.right() - galley.size().x, rect.center().y - galley.size().y / 2.0),
                            galley,
                            muted,
                        );
                        r
                    },
                )
                .inner;
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let width = ui.available_width();
            let label = WidgetText::from(TextRole::Label.rich(&t, &self.label)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                width,
                TextRole::Label.font(&t),
            );
            let value = ui.painter().layout_no_wrap(written, mono, muted);
            let line = label.size().y.max(value.size().y);
            let (head, _) = ui.allocate_exact_size(vec2(width, line), Sense::hover());
            ui.painter().galley(head.min, label, t.color("foreground"));
            ui.painter().galley(
                pos2(head.right() - value.size().x, head.bottom() - value.size().y),
                value,
                muted,
            );
            let r = ui.add(slider);
            if let Some(help) = &self.help {
                ui.label(TextRole::Help.rich(&t, help).color(muted));
            }
            r
        })
        .inner
    }
}
