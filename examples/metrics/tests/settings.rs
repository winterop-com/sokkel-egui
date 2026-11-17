//! Settings: opened from the rail over the page, the defaults a board follows until the reader
//! picks their own, and putting the demo's data back.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use metrics::MetricsApp;
use metrics::app::Zone;
use metrics::query::Range;
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
fn settings_open_over_the_page() {
    let mut harness = harness();
    harness.state_mut().place = "dashboards".into();
    harness.run_steps(2);
    harness.get_by_label("Settings").click();
    harness.run_steps(4);
    let app = harness.state();
    assert_eq!(app.settings.as_deref(), Some("dashboards"));
    assert_eq!(app.place, "dashboards");
    harness.snapshot("metrics_settings");
}

#[test]
fn a_default_reaches_a_board_that_has_not_picked_its_own() {
    let mut harness = harness();
    harness.state_mut().settings = Some("dashboards".into());
    harness.run_steps(3);
    harness.get_by_label("10s").click();
    harness.get_by_label("UTC").click();
    harness.run_steps(3);
    let app = harness.state();
    assert_eq!(app.prefs.refresh, 10);
    assert_eq!(app.refresh, 10);
    assert_eq!(app.prefs.zone, Zone::Utc);
    // a range the reader picked stays theirs
    harness.state_mut().range = Range::Preset("1h");
    harness.state_mut().range_chosen = true;
    harness.state_mut().prefs.range = "5m";
    harness.run_steps(2);
    assert_eq!(harness.state().range, Range::Preset("1h"));
}

#[test]
fn the_demo_data_goes_back() {
    let mut harness = harness();
    harness.state_mut().dashboards.pop();
    harness.state_mut().settings = Some("data".into());
    harness.run_steps(3);
    harness.get_by_label("Reset").click();
    harness.run_steps(3);
    assert!(harness.query_by_label("Reset the demo data?").is_some());
    harness.get_all_by_label("Reset").last().unwrap().click();
    harness.run_steps(3);
    assert_eq!(harness.state().dashboards.len(), 3);
}
