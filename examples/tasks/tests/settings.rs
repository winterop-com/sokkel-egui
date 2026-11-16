//! The settings dialog as a person uses it: it opens with the search taking the keys, and Escape
//! closes it.

use eframe::egui::{self, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::{Appearance, Day};
use tasks::TasksApp;

fn settings() -> Harness<'static, TasksApp> {
    let mut app = TasksApp::on(Day::parse("2026-11-16").unwrap(), Appearance::default());
    app.go("settings");
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut TasksApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness
}

#[test]
fn typing_searches_and_escape_closes() {
    let mut harness = settings();
    assert!(harness.query_all_by_label("Open on").next().is_some());
    harness.event(egui::Event::Text("density".into()));
    harness.run_steps(2);
    // the search narrowed the sections to Appearance, which holds Density
    assert!(harness.query_all_by_label("Open on").next().is_none());
    assert!(harness.state().settings.is_some());
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert!(harness.state().settings.is_none());
}
