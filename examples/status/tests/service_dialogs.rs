//! A service's dialogs: scheduling a window for work and putting out a new version, their refusals
//! in the web's words, and a deploy that runs and then succeeds.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::{Appearance, Day, Moment};
use status::{
    StatusApp,
    dialogs::{DeployForm, MaintenanceForm},
};

#[test]
fn a_window_needs_its_work_and_an_end_after_its_start() {
    let today = Day::parse("2026-11-16").unwrap();
    let mut form = MaintenanceForm::new("search", today);
    assert_eq!(form.why(), Some("Say what the work is."));
    form.title = "Moving to larger disks".into();
    assert_eq!(form.why(), None);
    form.ends = Some(Moment::new(today.plus(1), 1, 0));
    assert_eq!(form.why(), Some("It has to end after it starts."));
    form.ends = Some(Moment::new(today.plus(1), 4, 0));
    let window = form.make(today, "ada", 3).unwrap();
    assert_eq!(window.starts, 86_400_000.0 + 2.0 * 3_600_000.0);
    assert_eq!(window.ends, 86_400_000.0 + 4.0 * 3_600_000.0);
}

#[test]
fn a_version_is_written_as_one_and_is_new() {
    let mut form = DeployForm::new("api", "v2.41.2");
    assert_eq!(form.version, "v2.41.3");
    assert_eq!(form.why(), None);
    form.version = "2.41.3".into();
    assert_eq!(form.why().as_deref(), Some("Write the version as v1.2.3."));
    form.version = "v2.41.2".into();
    assert_eq!(form.why().as_deref(), Some("v2.41.2 is running already."));
}

#[test]
fn a_deploy_runs_then_is_the_version_running() {
    let mut app = StatusApp::on(Appearance::default());
    assert_eq!(app.running("api"), "v2.41.2");
    let form = DeployForm::new("api", "v2.41.2");
    let at = (9.0 * 3600.0 + 4.0) * 1000.0;
    app.deploys.push(form.make(at, "ada", 0));
    assert_eq!(app.running("api"), "v2.41.2", "a running deploy is not yet what runs");
    for _ in 0..6 {
        app.tick();
    }
    assert_eq!(app.running("api"), "v2.41.3");
}

#[test]
fn the_maintenance_dialog_with_its_picker_open() {
    let mut app = StatusApp::on(Appearance::default());
    app.tick();
    app.tick();
    app.go("services/api");
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut StatusApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness.get_by_label("Schedule maintenance").click();
    harness.run_steps(10);
    assert!(harness.state().maintenance_form.is_some());
    harness
        .get_all_by_label("Choose a date and time")
        .next()
        .unwrap()
        .click();
    harness.run_steps(6);
    harness.snapshot("maintenance_picker");
}
