//! The screens parity/metrics.json lists, drawn as the web's are shot: the demo's 09:00:05, the
//! last 15 minutes. `bun scripts/parity.ts board metrics` puts them beside the web's.

use eframe::egui;
use egui_kittest::{Harness, SnapshotResults};
use metrics::MetricsApp;
use sokkel_ui_egui::Appearance;

fn render(place: &str, height: f32) -> Harness<'static, MetricsApp> {
    let mut app = MetricsApp::on(Appearance::default());
    app.live = false;
    app.place = place.into();
    let mut harness = Harness::builder().with_size([1440.0, height]).build_ui_state(
        |ui, app: &mut MetricsApp| {
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
fn every_screen() {
    let mut results = SnapshotResults::new();
    for (name, place) in [
        ("metrics_services", "services"),
        ("metrics_workers", "workers"),
        ("metrics_jobs", "jobs"),
    ] {
        let mut harness = render(place, 1100.0);
        results.add(harness.try_snapshot(name));
    }
}
