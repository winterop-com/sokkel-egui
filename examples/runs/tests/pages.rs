//! The screens parity/runs.json lists, drawn as the web's are shot: the console opened at
//! 2026-11-18 09:00 UTC. `bun scripts/parity.ts board runs` puts them beside the web's.

use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotResults};
use runs::RunsApp;
use runs::app::DEMO_START;
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, RunsApp> {
    let mut app = RunsApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut RunsApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
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
    let mut harness = render("runs");
    results.add(harness.try_snapshot("runs_runs"));
    // a run opened beside the list, on each of its panel's tabs
    let id = harness.state().backend.runs[1].id.clone();
    harness.state_mut().opened = Some(id);
    harness.run_steps(4);
    results.add(harness.try_snapshot("runs_run_panel"));
    harness.get_by_label("Steps").click();
    harness.run_steps(4);
    results.add(harness.try_snapshot("runs_run_panel_steps"));
    harness.get_by_label("Log").click();
    harness.run_steps(4);
    results.add(harness.try_snapshot("runs_run_panel_log"));
}
