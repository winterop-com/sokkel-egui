//! The studio's screens at the web's routes, at 1440 by 900 on the parity day:
//! `bun scripts/parity.ts board studio` puts them beside the web's.

use egui_kittest::{Harness, SnapshotResults};
use sokkel_egui::Appearance;
use studio::StudioApp;
use studio::app::DEMO_START;

pub fn render(path: &str) -> Harness<'static, StudioApp> {
    let mut app = StudioApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut StudioApp| {
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
    for (name, path) in [
        ("studio_pipeline", "file/harbor-nightly-export"),
        ("studio_schema", "file/harbor-order-schema"),
        ("studio_query", "file/juniper-slow-steps"),
        ("studio_table", "table/runs"),
        ("studio_runs", "runs"),
    ] {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
}

#[test]
fn settings_dialogs_and_palettes() {
    use studio::backend::Asking;
    use studio::model::{FileKind, Project};
    let mut results = SnapshotResults::new();
    let mut harness = render("settings");
    results.add(harness.try_snapshot("studio_settings"));
    let mut harness = render("file/harbor-nightly-export");
    harness.state_mut().backend.asking = Some(Asking::New {
        project: Project::Juniper,
        kind: FileKind::Query,
    });
    harness.run_steps(4);
    results.add(harness.try_snapshot("studio_new_file"));
    let mut harness = render("file/harbor-nightly-export");
    harness.state_mut().palette = Some(Default::default());
    harness.run_steps(4);
    results.add(harness.try_snapshot("studio_palette"));
}

#[test]
fn a_run_going() {
    use studio::app::DEMO_START;
    let mut results = SnapshotResults::new();
    let mut harness = render("file/harbor-nightly-export");
    let started = harness.state_mut().backend.run("harbor-nightly-export", DEMO_START);
    let id = match started {
        Some(Ok(run)) => run.id,
        other => panic!("the pipeline did not start: {other:?}"),
    };
    harness.state_mut().wait(2_500);
    harness.run_steps(4);
    results.add(harness.try_snapshot("studio_pipeline_running"));
    harness.state_mut().wait(30_000);
    harness.state_mut().go(&format!("runs/{id}"));
    harness.run_steps(4);
    results.add(harness.try_snapshot("studio_run"));
    harness.state_mut().go("runs");
    harness.run_steps(4);
    results.add(harness.try_snapshot("studio_runs_one"));
}
