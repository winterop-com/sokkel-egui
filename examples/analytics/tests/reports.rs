//! Reports: a report with changes not saved asks before the page is left, and Leave lets them go.

use analytics::AnalyticsApp;
use analytics::app::DEMO_START;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::Appearance;

fn harness() -> Harness<'static, AnalyticsApp> {
    let mut app = AnalyticsApp::at(DEMO_START, Appearance::default());
    app.go("reports/monthly-board");
    let mut harness = Harness::builder().with_size([1440.0, 1100.0]).build_ui_state(
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
fn a_changed_report_asks_before_the_page_is_left() {
    let mut harness = harness();
    harness.state_mut().reports.draft.as_mut().unwrap().name = "Board report".into();
    harness.run_steps(2);
    harness.get_by_label("Overview").click();
    harness.run_steps(3);
    assert_eq!(harness.state().page(), "reports");
    assert!(harness.state().leaving.is_some());
    harness.get_by_label("Leave").click();
    harness.run_steps(3);
    assert_eq!(harness.state().page(), "overview");
}

#[test]
fn an_unchanged_report_is_left_at_once() {
    let mut harness = harness();
    harness.get_by_label("Overview").click();
    harness.run_steps(3);
    assert_eq!(harness.state().page(), "overview");
}
