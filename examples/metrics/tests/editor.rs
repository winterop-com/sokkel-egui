//! The panel editor: a new panel added to the dashboard, a panel edited from its menu, and the
//! question before leaving with changes.

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
fn a_new_panel_is_added_to_the_dashboard() {
    let mut harness = harness();
    let before = harness.state().dashboards[0].panels.len();
    harness.get_by_label("Add panel").click();
    harness.run_steps(4);
    assert!(harness.state().editor.is_some());
    harness.snapshot("metrics_editor");
    harness.get_by_label("Add to dashboard").click();
    harness.run_steps(3);
    let app = harness.state();
    assert!(app.editor.is_none());
    assert_eq!(app.dashboards[0].panels.len(), before + 1);
    assert_eq!(app.dashboards[0].panels[before].title, "New panel");
}

#[test]
fn a_panel_is_edited_from_its_menu() {
    let mut harness = harness();
    let first = harness.state().dashboards[0].panels[0].clone();
    harness.get_all_by_label("Panel actions").next().unwrap().click();
    harness.run_steps(2);
    // the menu's Edit, after the head's
    harness.get_all_by_label("Edit").last().unwrap().click();
    harness.run_steps(4);
    assert_eq!(
        harness.state().editor.as_ref().map(|e| e.draft.id.clone()),
        Some(first.id.clone())
    );
    // nothing changed yet: Save waits, and leaving asks nothing
    harness.get_by_label("Gauge").click();
    harness.run_steps(3);
    harness.snapshot("metrics_editor_gauge");
    harness.get_by_label("Save").click();
    harness.run_steps(3);
    let app = harness.state();
    assert!(app.editor.is_none());
    assert_eq!(app.dashboards[0].panels[0].kind, metrics::model::PanelKind::Gauge);
    assert_eq!(app.dashboards[0].panels[0].id, first.id);
}

#[test]
fn leaving_with_changes_asks_first() {
    let mut harness = harness();
    harness.get_by_label("Add panel").click();
    harness.run_steps(4);
    harness.get_by_label("Discard").click();
    harness.run_steps(3);
    assert!(harness.query_by_label("Leave without saving?").is_some());
    harness.get_by_label("Stay").click();
    harness.run_steps(3);
    assert!(harness.state().editor.is_some());
    harness.get_by_label("Discard").click();
    harness.run_steps(3);
    harness.get_by_label("Leave").click();
    harness.run_steps(3);
    assert!(harness.state().editor.is_none());
}

#[test]
fn an_empty_dashboard_offers_a_panel() {
    let mut harness = harness();
    harness.state_mut().dashboards[0].panels.clear();
    harness.run_steps(3);
    // the head's Add panel and the empty board's
    assert_eq!(harness.get_all_by_label("Add panel").count(), 2);
    harness.get_all_by_label("Add panel").last().unwrap().click();
    harness.run_steps(3);
    assert!(harness.state().editor.is_some());
}
