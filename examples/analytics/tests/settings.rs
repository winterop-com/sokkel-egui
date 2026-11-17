//! Settings: the live stream's section with its note, and the demo's data put back after asking.

use analytics::AnalyticsApp;
use analytics::app::DEMO_START;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::Appearance;

fn harness() -> Harness<'static, AnalyticsApp> {
    let mut app = AnalyticsApp::at(DEMO_START, Appearance::default());
    app.go("settings");
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut AnalyticsApp| {
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
fn the_live_section_says_how_many_changes_are_held() {
    let mut harness = harness();
    harness.get_by_label("Live data").click();
    harness.run_steps(3);
    harness.snapshot("analytics_settings_live");
}

#[test]
fn the_demo_data_go_back_after_asking() {
    let mut harness = harness();
    harness.state_mut().prefs.workspace = "Harbor".into();
    harness.state_mut().goals.goals.clear();
    harness.get_by_label("Demo data").click();
    harness.run_steps(3);
    harness.get_by_label("Reset").click();
    harness.run_steps(3);
    assert!(harness.state().resetting);
    // the question's Reset, in front of the dialog
    harness.get_all_by_label("Reset").last().unwrap().click();
    harness.run_steps(3);
    assert_eq!(harness.state().prefs.workspace, "Tidevann");
    assert_eq!(harness.state().goals.goals.len(), 3);
    assert!(!harness.state().resetting);
}
