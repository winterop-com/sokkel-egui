//! Goals: a goal set through the dialog is read live beside the others.

use analytics::AnalyticsApp;
use analytics::app::DEMO_START;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::Appearance;

#[test]
fn a_goal_set_in_the_dialog_joins_the_others() {
    let mut app = AnalyticsApp::at(DEMO_START, Appearance::default());
    app.go("goals");
    let mut harness = Harness::builder().with_size([1440.0, 1100.0]).build_ui_state(
        |ui, app: &mut AnalyticsApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness.get_by_label("New").click();
    harness.run_steps(4);
    harness.snapshot("analytics_goal_dialog");
    // nothing named yet: the dialog stays and says why
    harness.get_by_label("Set goal").click();
    harness.run_steps(3);
    assert!(harness.state().goals.editing.is_some());
    let draft = harness.state_mut().goals.editing.as_mut().unwrap();
    draft.name = "Revenue this quarter".into();
    draft.metric = 1;
    draft.target = 1_800_000.0;
    harness.run_steps(2);
    harness.get_by_label("Set goal").click();
    harness.run_steps(3);
    assert!(harness.state().goals.editing.is_none());
    let goals = &harness.state().goals.goals;
    assert_eq!(goals.len(), 4);
    assert_eq!(goals[3].name, "Revenue this quarter");
}
