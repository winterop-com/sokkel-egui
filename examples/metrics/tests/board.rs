//! A dashboard's own actions: arranging its panels, its details, a copy of it, and deleting it.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use metrics::MetricsApp;
use sokkel_egui::Appearance;

fn harness() -> Harness<'static, MetricsApp> {
    let mut app = MetricsApp::on(Appearance::default());
    app.live = false;
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut MetricsApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(3);
    harness
}

#[test]
fn arranging_holds_a_draft_until_done() {
    let mut harness = harness();
    harness.get_by_label("Edit").click();
    harness.run_steps(3);
    assert!(harness.state().edit.editing());
    harness.snapshot("metrics_arranging");
    // a panel from the palette lands at the end of the draft, and Cancel throws it away
    let before = harness.state().dashboards[0].panels.len();
    harness.get_by_label("Add Duration by job").click();
    harness.run_steps(3);
    assert_eq!(harness.state().dashboards[0].panels.len(), before);
    harness.get_by_label("Cancel").click();
    harness.run_steps(3);
    assert!(!harness.state().edit.editing());
    assert_eq!(harness.state().dashboards[0].panels.len(), before);
    // and Done keeps it
    harness.get_by_label("Edit").click();
    harness.run_steps(3);
    harness.get_by_label("Add Duration by job").click();
    harness.run_steps(3);
    harness.get_by_label("Done").click();
    harness.run_steps(3);
    assert_eq!(harness.state().dashboards[0].panels.len(), before + 1);
}

#[test]
fn the_details_change_and_a_copy_is_made() {
    let mut harness = harness();
    harness.get_by_label("More").click();
    harness.run_steps(2);
    harness.get_by_label("Edit details").click();
    harness.run_steps(3);
    assert!(harness.state().details.is_some());
    harness.state_mut().details.as_mut().unwrap().title = "Front door".into();
    harness.get_by_label("Save").click();
    harness.run_steps(3);
    assert_eq!(harness.state().dashboards[0].title, "Front door");
    harness.get_by_label("More").click();
    harness.run_steps(2);
    harness.get_by_label("Duplicate dashboard").click();
    harness.run_steps(3);
    let app = harness.state();
    assert_eq!(app.dashboards.len(), 4);
    assert_eq!(app.dashboards[3].title, "Front door copy");
    assert_eq!(app.place, app.dashboards[3].id);
}

#[test]
fn deleting_a_dashboard_asks_first() {
    let mut harness = harness();
    harness.get_by_label("More").click();
    harness.run_steps(2);
    harness.get_by_label("Delete dashboard").click();
    harness.run_steps(3);
    assert!(harness.query_by_label("Delete Services overview?").is_some());
    harness.get_by_label("Delete").click();
    harness.run_steps(3);
    let app = harness.state();
    assert_eq!(app.dashboards.len(), 2);
    assert_eq!(app.place, "workers");
}
