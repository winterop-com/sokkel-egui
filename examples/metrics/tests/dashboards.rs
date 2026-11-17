//! The Dashboards page: the list, a new dashboard made as a copy, and one deleted from the list.

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
fn the_dashboards_are_listed() {
    let mut harness = harness();
    // the rail's first entry, over the dashboards' own
    harness.get_all_by_label("Dashboards").next().unwrap().click();
    harness.run_steps(3);
    assert_eq!(harness.state().place, "dashboards");
    harness.snapshot("metrics_dashboards");
}

#[test]
fn a_new_dashboard_starts_as_a_copy() {
    let mut harness = harness();
    harness.state_mut().place = "dashboards".into();
    harness.run_steps(3);
    harness.get_by_label("New").click();
    harness.run_steps(3);
    {
        let form = harness.state_mut().details.as_mut().unwrap();
        form.title = "Front door".into();
        form.from = 2;
    }
    harness.get_by_label("Create").click();
    harness.run_steps(3);
    let app = harness.state();
    assert_eq!(app.dashboards.len(), 4);
    let made = &app.dashboards[3];
    assert_eq!(made.title, "Front door");
    assert_eq!(made.panels.len(), app.dashboards[1].panels.len());
    assert_ne!(made.panels[0].id, app.dashboards[1].panels[0].id);
    assert_eq!(app.place, made.id);
}

#[test]
fn a_dashboard_is_deleted_from_the_list() {
    let mut harness = harness();
    harness.state_mut().place = "dashboards".into();
    harness.run_steps(3);
    harness.get_all_by_label("Actions").nth(1).unwrap().click();
    harness.run_steps(2);
    harness.get_by_label("Delete").click();
    harness.run_steps(3);
    harness.get_by_label("Delete").click();
    harness.run_steps(3);
    let app = harness.state();
    assert_eq!(app.dashboards.len(), 2);
    assert!(app.dashboards.iter().all(|b| b.id != "workers"));
    assert_eq!(app.place, "dashboards");
}
