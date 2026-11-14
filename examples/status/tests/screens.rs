//! The screens parity/status.json lists, drawn as the web example's are shot: 1280 by 800, at the
//! demo's start, after the same two readings. `bun scripts/parity.ts board status` puts them beside
//! the web's.

use eframe::egui;
use egui_kittest::{Harness, SnapshotResults};
use sokkel_ui_egui::Appearance;
use status::StatusApp;

fn render(route: &str) -> Harness<'static, StatusApp> {
    let mut app = StatusApp::on(Appearance::default());
    // the web's shot runs 5 s of its clock: two readings, the last at 09:00:04
    app.readings = 2;
    app.go(route);
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut StatusApp| {
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
    for (name, route) in [
        ("overview", "overview"),
        ("incidents", "incidents"),
        ("services", "services"),
        ("service", "services/api"),
        ("regions", "regions"),
        ("on_call", "on-call"),
    ] {
        let mut harness = render(route);
        results.add(harness.try_snapshot(name));
    }
}
