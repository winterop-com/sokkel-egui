//! The screens parity/runs.json lists, drawn as the web's are shot: the console opened at
//! 2026-11-18 09:00 UTC. `bun scripts/parity.ts board runs` puts them beside the web's.

use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotResults};
use runs::RunsApp;
use runs::app::DEMO_START;
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, RunsApp> {
    render_at(path, 900.0)
}

pub fn render_at(path: &str, height: f32) -> Harness<'static, RunsApp> {
    let mut app = RunsApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, height]).build_ui_state(
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
    let mut harness = render("");
    results.add(harness.try_snapshot("runs_overview"));
    let mut harness = render_at("", 1600.0);
    results.add(harness.try_snapshot("runs_overview_whole"));
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
    // back on Details, the table narrower than its columns scrolls sideways under the pointer,
    // its first and end columns pinned, a bar along its rows' foot
    harness.get_by_label("Details").click();
    harness.run_steps(4);
    harness.hover_at(eframe::egui::pos2(640.0, 500.0));
    harness.event(eframe::egui::Event::MouseWheel {
        unit: eframe::egui::MouseWheelUnit::Point,
        delta: eframe::egui::vec2(-1000.0, 0.0),
        phase: eframe::egui::TouchPhase::Move,
        source: eframe::egui::MouseWheelSource::Unknown,
        modifiers: eframe::egui::Modifiers::NONE,
    });
    harness.run_steps(30);
    results.add(harness.try_snapshot("runs_run_panel_scrolled"));

    let mut harness = render("jobs");
    results.add(harness.try_snapshot("runs_jobs"));
    harness.get_by_label("New").click();
    harness.run_steps(4);
    results.add(harness.try_snapshot("runs_new_job"));
    let mut harness = render("schedules");
    results.add(harness.try_snapshot("runs_schedules"));
    let mut harness = render("settings");
    results.add(harness.try_snapshot("runs_settings"));
    let mut harness = render("runs");
    harness.state_mut().palette = Some(("nig".into(), 0));
    harness.run_steps(4);
    results.add(harness.try_snapshot("runs_palette"));
    let mut harness = render("jobs/nightly-export");
    results.add(harness.try_snapshot("runs_job"));
    let mut harness = render("runs/run_toos50a9");
    results.add(harness.try_snapshot("runs_run"));
    let mut harness = render_at("runs/run_toos50a9", 1500.0);
    results.add(harness.try_snapshot("runs_run_whole"));
    let mut harness = render("");
    harness.state_mut().sign_out();
    harness.state_mut().appearance.motion = sokkel_egui::Motion::Off;
    harness.run_steps(4);
    results.add(harness.try_snapshot("runs_sign_in"));
}
