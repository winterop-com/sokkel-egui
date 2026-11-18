//! The media parts: the transport under cut's preview as the web shot draws it
//! (`parity/cut/web/cut_editor.png`), the other transport pieces, the waveform and its scrubber, a
//! slider field and the not-found page; and the sliders answering the pointer and the keys.

use std::sync::Arc;

use egui::{Align, Key, Layout, RichText, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{
    Appearance, AudioScrubber, Icon, LiveMark, MediaSlider, MediaTone, NotFound, PlayButton, PlayWords, Position,
    RateMenu, Scheme, SliderField, TransportButton, TransportGlyph, TransportSize, VolumeControl, VolumeWords,
    Waveform, format_remaining, format_time, format_timecode, install, mono_font, mono_size, static_levels, tokens,
};

fn light() -> Appearance {
    Appearance {
        scheme: Scheme::Light,
        ..Appearance::default()
    }
}

fn surf_peaks() -> Arc<[f32]> {
    let peaks: Vec<f32> = serde_json::from_str(include_str!("fixtures/surf_peaks.json")).unwrap();
    peaks.into()
}

/// The transport under the preview in cut's editor: the scrub bar, then the timecode, the buttons
/// to the start, a frame back, play, a frame on and to the end, and the level, compact.
#[test]
fn the_transport_under_the_preview() {
    // a desktop's window, the transport in the preview's 605
    let mut harness = Harness::builder().with_size([800.0, 72.0 + 16.0]).build_ui(|ui| {
        install(ui.ctx(), &light());
        let t = tokens(ui.ctx());
        let length = 26.0;
        ui.set_max_width(605.0);
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let _ = MediaSlider::new("playhead", 0.0, length, "Playhead")
                    .value_text(move |v| format!("{} of {}", format_timecode(v, 30.0), format_timecode(length, 30.0)))
                    .step(1.0 / 30.0)
                    .large_step(1.0)
                    .show(ui);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    let spec = t.text("sm");
                    let font = mono_font(ui.ctx(), mono_size(&t, spec.size), 400.0);
                    let mut job = egui::text::LayoutJob::default();
                    RichText::new(format_timecode(0.0, 30.0))
                        .font(font.clone())
                        .color(t.color("foreground"))
                        .append_to(&mut job, ui.style(), egui::FontSelection::Default, Align::Center);
                    RichText::new(format!(" / {}", format_timecode(length, 30.0)))
                        .font(font)
                        .color(t.color("muted-foreground"))
                        .append_to(&mut job, ui.style(), egui::FontSelection::Default, Align::Center);
                    ui.label(job);
                    let volume = 28.0;
                    let middle = ui.available_width() - volume - 12.0;
                    ui.allocate_ui_with_layout(vec2(middle, 32.0), Layout::left_to_right(Align::Center), |ui| {
                        ui.set_min_width(middle);
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let buttons = 4.0 * 28.0 + 32.0 + 4.0 * 4.0;
                        ui.add_space(((middle - buttons) / 2.0).max(0.0));
                        ui.add(TransportButton::new(Icon::ChevronFirst, "To the start"));
                        ui.add(TransportButton::new(Icon::StepBack, "A frame back"));
                        ui.add(PlayButton::new(false).size(TransportSize::Md));
                        ui.add(TransportButton::new(Icon::StepForward, "A frame on"));
                        ui.add(TransportButton::new(Icon::ChevronLast, "To the end"));
                    });
                    let _ = VolumeControl::new("level", 1.0, false)
                        .compact(true)
                        .words(VolumeWords {
                            volume: "Level".into(),
                            ..VolumeWords::default()
                        })
                        .show(ui);
                });
            });
    });
    harness.run_steps(3);
    harness.snapshot("media_transport");
}

/// The rest of the parts, each as an app would place it.
#[test]
fn the_media_parts() {
    let peaks = surf_peaks();
    let mut volume = 80.0;
    let mut speed = 1.25;
    let mut harness = Harness::builder().with_size([800.0, 760.0]).build_ui(move |ui| {
        install(ui.ctx(), &light());
        let t = tokens(ui.ctx());
        ui.set_max_width(640.0);
        ui.spacing_mut().item_spacing.y = 16.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.add(TransportButton::new(
                TransportGlyph::Skip {
                    seconds: 15.0,
                    back: true,
                },
                "Back 15 seconds",
            ));
            ui.add(PlayButton::new(true).words(PlayWords::default()));
            ui.add(TransportButton::new(
                TransportGlyph::Skip {
                    seconds: 30.0,
                    back: false,
                },
                "On 30 seconds",
            ));
            ui.add(TransportButton::new(Icon::Shuffle, "Shuffle").pressed(true));
            ui.add(TransportButton::new(Icon::Repeat, "Repeat").pressed(false));
            let _ = RateMenu::new(speed, "Playback speed").show(ui);
            ui.add(PlayButton::new(false).size(TransportSize::Lg));
            ui.add(LiveMark::new("Live"));
        });
        let _ = VolumeControl::new("volume", 0.4, false).show(ui);
        let _ = Position::new("position", 75.0, 222.0, "Seek").show(ui);
        let _ = Position::new("waveform", 9.0, 30.0, "Seek").waveform(&peaks).show(ui);
        egui::Frame::new()
            .fill(t.color("terminal"))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                let _ = Position::new("over", 75.0, 222.0, "Seek")
                    .tone(MediaTone::Overlay)
                    .show(ui);
            });
        ui.add(
            Waveform::new(&static_levels(48, 42))
                .height(Some(48.0))
                .color(t.color("primary")),
        );
        let _ = AudioScrubber::new("scrubber", &peaks, 12.0, 30.0).height(40.0).show(ui);
        ui.add(
            SliderField::new("Volume", &mut volume, 100.0)
                .unit("%")
                .help("How loud the clip plays."),
        );
        ui.add(
            SliderField::new("Speed", &mut speed, 2.0)
                .step(0.25)
                .format(|v| format!("{v:.2}"))
                .inline(true),
        );
        let _ = NotFound::new("#/projects/gone").show(ui);
    });
    harness.run_steps(3);
    harness.snapshot("media_parts");
}

#[test]
fn a_media_slider_reports_a_drag_once_it_is_let_go() {
    #[derive(Default)]
    struct State {
        value: f64,
        scrubbed: Vec<f64>,
        changed: Vec<f64>,
    }
    let mut harness = Harness::builder().with_size([216.0, 32.0]).build_ui_state(
        |ui, state: &mut State| {
            install(ui.ctx(), &light());
            let r = MediaSlider::new("s", state.value, 100.0, "Level").step(5.0).show(ui);
            if let Some(v) = r.scrub {
                state.scrubbed.push(v);
            }
            if let Some(v) = r.changed {
                state.changed.push(v);
                state.value = v;
            }
        },
        State::default(),
    );
    harness.run_steps(2);
    // the slider runs 8 to 208, its thumb's middle 6 in from either end
    let at = |v: f32| pos2(8.0 + 6.0 + (200.0 - 12.0) * v, 8.0 + 8.0);
    harness.hover_at(at(0.25));
    harness.drag_at(at(0.25));
    harness.run_steps(1);
    harness.hover_at(at(0.5));
    harness.run_steps(1);
    assert!(harness.state().changed.is_empty());
    harness.drop_at(at(0.5));
    harness.run_steps(2);
    assert!(!harness.state().scrubbed.is_empty());
    assert_eq!(harness.state().changed.len(), 1);
    assert!((harness.state().changed[0] - 50.0).abs() < 1.0);
    // a key is told at once
    harness.get_by_label("Level").focus();
    harness.run_steps(1);
    harness.key_press(Key::ArrowRight);
    harness.run_steps(1);
    assert!((harness.state().value - 55.0).abs() < 1.0);
    harness.key_press(Key::Home);
    harness.run_steps(1);
    assert_eq!(harness.state().value, 0.0);
}

#[test]
fn the_clocks_read_as_sokkel_ui_s() {
    assert_eq!(format_time(f64::NAN), "0:00");
    assert_eq!(format_time(75.9), "1:15");
    assert_eq!(format_time(3725.0), "1:02:05");
    assert_eq!(format_remaining(75.0, 222.0), "\u{2212}2:27");
}
