//! The preview, as the web's `editor/Preview.tsx`: the edit's picture at the playhead, painted
//! straight from the scenes, the scrub bar under it, and the transport: to the start, a frame back,
//! play, a frame on, to the end, the timecode and the level.

use eframe::egui::{self, Align, Layout, Rect, RichText, Ui, vec2};
use sokkel_egui::{
    Chip, Icon, MediaSlider, PlayButton, PlayWords, TransportButton, TransportSize, VolumeControl, VolumeWords,
    mono_font, mono_size, tokens,
};

use crate::editor_page::EditorView;
use crate::model::{FPS, length_of, timecode};
use crate::playback::rate_label;
use crate::render::draw_edit_frame;

/// The transport's height under the picture: the scrub bar and the row of buttons, 8 over and under.
const TRANSPORT: f32 = 72.0;

pub fn preview(ui: &mut Ui, view: &mut EditorView, now: i64) {
    let t = tokens(ui.ctx());
    let ms = now as f64;
    let room = ui.max_rect();
    let length = length_of(view.edit());
    let at = view.playback.playhead.min(length);

    // the picture, as large as fits at 16 by 9 on the terminal's ground, 8 in from its edges
    let stage = Rect::from_min_max(room.min, egui::pos2(room.right(), room.bottom() - TRANSPORT));
    ui.painter().rect_filled(stage, 0.0, t.color("terminal"));
    let inner = stage.shrink(8.0);
    let scale = (inner.width() / 16.0).min(inner.height() / 9.0).max(0.0);
    let picture = Rect::from_center_size(inner.center(), vec2(16.0 * scale, 9.0 * scale));
    if picture.width() > 0.0 {
        draw_edit_frame(ui.painter(), picture, view.edit(), at);
    }
    ui.painter().hline(
        room.x_range(),
        stage.bottom() + 0.5,
        egui::Stroke::new(1.0, t.color("border")),
    );

    let bar = Rect::from_min_max(egui::pos2(room.left(), stage.bottom() + 1.0), room.max);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(bar)
            .layout(Layout::top_down(Align::Min)),
    );
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(&mut child, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let slider = MediaSlider::new("cut-playhead", at, length.max(1.0 / FPS), "Playhead")
                .value_text(move |v| format!("{} of {}", timecode(v), timecode(length)))
                .step(1.0 / FPS)
                .large_step(1.0)
                .show(ui);
            if let Some(to) = slider.changed.or(slider.scrub) {
                view.playback.seek(to, ms);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                let font = mono_font(ui.ctx(), mono_size(&t, t.text("sm").size), 400.0);
                let mut job = egui::text::LayoutJob::default();
                RichText::new(timecode(at))
                    .font(font.clone())
                    .color(t.color("foreground"))
                    .append_to(&mut job, ui.style(), egui::FontSelection::Default, Align::Center);
                RichText::new(format!(" / {}", timecode(length)))
                    .font(font)
                    .color(t.color("muted-foreground"))
                    .append_to(&mut job, ui.style(), egui::FontSelection::Default, Align::Center);
                ui.label(job);
                let rate = rate_label(view.playback.rate);
                let volume = 28.0;
                let chip = if rate.is_some() { 40.0 + 12.0 } else { 0.0 };
                let middle = (ui.available_width() - volume - 12.0 - chip).max(0.0);
                ui.allocate_ui_with_layout(vec2(middle, 32.0), Layout::left_to_right(Align::Center), |ui| {
                    ui.set_min_width(middle);
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let buttons = 4.0 * 28.0 + 32.0 + 4.0 * 4.0;
                    ui.add_space(((middle - buttons) / 2.0).max(0.0));
                    if ui
                        .add(TransportButton::new(Icon::ChevronFirst, "To the start"))
                        .clicked()
                    {
                        view.playback.seek(0.0, ms);
                    }
                    if ui.add(TransportButton::new(Icon::StepBack, "A frame back")).clicked() {
                        view.playback.step(-1.0, ms);
                    }
                    let play = PlayButton::new(view.playback.playing())
                        .size(TransportSize::Md)
                        .words(PlayWords {
                            play: "Play".into(),
                            pause: "Pause".into(),
                        });
                    if ui.add(play).clicked() {
                        view.playback.toggle(ms);
                    }
                    if ui.add(TransportButton::new(Icon::StepForward, "A frame on")).clicked() {
                        view.playback.step(1.0, ms);
                    }
                    if ui.add(TransportButton::new(Icon::ChevronLast, "To the end")).clicked() {
                        view.playback.seek(length, ms);
                    }
                });
                if let Some(rate) = rate {
                    ui.add(Chip::new(rate));
                }
                let level = VolumeControl::new("cut-level", if view.muted { 0.0 } else { view.level }, view.muted)
                    .compact(true)
                    .words(VolumeWords {
                        mute: "Mute".into(),
                        unmute: "Unmute".into(),
                        volume: "Level".into(),
                    })
                    .show(ui);
                if let Some(muted) = level.muted {
                    view.muted = muted;
                }
                if let Some(volume) = level.volume {
                    view.level = volume;
                }
            });
        });
}
