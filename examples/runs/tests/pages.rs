//! Every page of the app, in tidevann light, the overview in dark, and the runs page with a run open.

use egui_kittest::{Harness, SnapshotResults};
use runs::RunsApp;
use sokkel_ui_egui::{Appearance, Scheme};

fn render(scheme: Scheme, place: &str) -> Harness<'static, RunsApp> {
    let mut app = RunsApp::with(Appearance {
        scheme,
        ..Default::default()
    });
    app.go(place);
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut RunsApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
                .show(ui, |ui| app.show(ui))
                .inner
        },
        app,
    );
    harness.run_steps(3);
    harness
}

#[test]
fn every_page() {
    let mut results = SnapshotResults::new();
    for place in ["overview", "runs", "schedules", "jobs"] {
        let mut harness = render(Scheme::Light, place);
        results.add(harness.try_snapshot(format!("runs_{place}")));
    }
    let mut harness = render(Scheme::Dark, "overview");
    results.add(harness.try_snapshot("runs_overview_dark"));
    let mut harness = render(Scheme::Light, "settings");
    results.add(harness.try_snapshot("runs_settings"));
}
