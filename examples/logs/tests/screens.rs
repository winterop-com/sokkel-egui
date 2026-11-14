//! The log viewer's screens and what its controls do: a level narrows the lines, words search
//! them, a line pressed opens its fields beside the stream.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use logs::{LogsApp, stream::Level};
use sokkel_ui_egui::{Appearance, Scheme};

fn render(scheme: Scheme, setup: impl FnOnce(&mut LogsApp)) -> Harness<'static, LogsApp> {
    let mut app = LogsApp::on(Appearance {
        scheme,
        ..Default::default()
    });
    setup(&mut app);
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut LogsApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(6);
    harness
}

#[test]
fn the_stream() {
    let mut harness = render(Scheme::Light, |_| {});
    harness.snapshot("logs");
}

#[test]
fn the_stream_in_dark_with_a_line_open() {
    let mut harness = render(Scheme::Dark, |app| {
        app.level = 3;
        let first = app.shown().first().map(|l| l.id);
        app.opened = first;
    });
    harness.snapshot("logs_error_open");
}

#[test]
fn a_level_and_words_narrow_the_lines() {
    let mut app = LogsApp::on(Appearance::default());
    let all = app.shown().len();
    app.level = 3;
    assert!(app.shown().iter().all(|l| l.level == Level::Error));
    assert!(app.shown().len() < all);
    app.level = 0;
    app.search = "nightly-export".into();
    assert!(app.shown().iter().all(|l| l.message.contains("nightly-export")));
    app.search.clear();
    app.place = "worker".into();
    assert!(app.shown().iter().all(|l| l.service == "worker"));
}

#[test]
fn a_reading_adds_lines_newest_first() {
    let mut app = LogsApp::on(Appearance::default());
    let before = app.stream.lines.len();
    app.tick();
    assert!(app.stream.lines.len() > before);
    assert_eq!(app.shown()[0].at, app.at);
}

#[test]
fn a_service_chosen_in_the_rail_narrows_the_stream() {
    let mut harness = render(Scheme::Light, |_| {});
    harness.get_by_label("worker").click();
    harness.run_steps(3);
    assert_eq!(harness.state().place, "worker");
}
