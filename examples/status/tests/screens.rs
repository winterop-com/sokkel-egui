//! The screens parity/status.json lists, drawn as the web example's are shot: 1280 by 800 (a whole page taller), at the
//! demo's start, after the same two readings. `bun scripts/parity.ts board status` puts them beside
//! the web's.

use eframe::egui;
use egui_kittest::{Harness, SnapshotResults};
use sokkel_ui_egui::Appearance;
use status::StatusApp;

fn render(route: &str, height: f32) -> Harness<'static, StatusApp> {
    let mut app = StatusApp::on(Appearance::default());
    // the web's shot runs 5 s of its clock: two readings, the last at 09:00:04
    app.tick();
    app.tick();
    app.go(route);
    let mut harness = Harness::builder().with_size([1280.0, height]).build_ui_state(
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
    for (name, route, height) in [
        ("overview", "overview", 800.0),
        ("incidents", "incidents", 800.0),
        ("incident", "incidents/incident-41", 800.0),
        ("services", "services", 800.0),
        ("service", "services/api", 800.0),
        ("service_whole", "services/api", 1500.0),
        ("regions", "regions", 800.0),
        ("on_call", "on-call", 800.0),
        ("public", "public", 800.0),
    ] {
        let mut harness = render(route, height);
        results.add(harness.try_snapshot(name));
    }
}
