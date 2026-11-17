//! The search palette: opened from the top bar, its commands going to a place or a panel.

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
fn the_palette_goes_to_a_panel() {
    let mut harness = harness();
    harness.get_by_label("Search").click();
    harness.run_steps(4);
    assert!(harness.state().palette.is_some());
    harness.snapshot("metrics_palette");
    // a panel of another dashboard, found by what is typed: its board shown, the panel in its window
    harness.state_mut().palette = Some(("queue by project".into(), 0));
    harness.run_steps(3);
    harness.get_by_label("Queue by project").click();
    harness.run_steps(4);
    let app = harness.state();
    assert!(app.palette.is_none());
    assert_eq!(app.place, "jobs");
    let viewing = app.panels.viewing.clone().unwrap();
    let board = app.dashboards.iter().find(|b| b.id == "jobs").unwrap();
    assert_eq!(
        board.panels.iter().find(|p| p.id == viewing).unwrap().title,
        "Queue by project"
    );
}

#[test]
fn the_palette_goes_to_a_page() {
    let mut harness = harness();
    harness.get_by_label("Search").click();
    harness.run_steps(4);
    harness.get_all_by_label("Alerts").last().unwrap().click();
    harness.run_steps(3);
    assert_eq!(harness.state().place, "alerts");
}
