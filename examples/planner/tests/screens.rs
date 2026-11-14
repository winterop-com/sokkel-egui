//! The planner's screens and what its controls do: a job unticked leaves the week, a run pressed
//! opens beside it, a run ahead can be skipped, the week moves back and on.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use planner::{PlannerApp, RunState};
use sokkel_ui_egui::{Appearance, Scheme};

fn render(scheme: Scheme, setup: impl FnOnce(&mut PlannerApp)) -> Harness<'static, PlannerApp> {
    let mut app = PlannerApp::on(Appearance {
        scheme,
        ..Default::default()
    });
    setup(&mut app);
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut PlannerApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(6);
    harness
}

fn first_ahead(app: &PlannerApp) -> String {
    let now = app.now;
    app.runs
        .iter()
        .find(|r| r.start >= now)
        .map(|r| r.id.clone())
        .expect("a run ahead")
}

#[test]
fn the_week() {
    let mut harness = render(Scheme::Light, |_| {});
    harness.snapshot("planner");
}

#[test]
fn the_week_in_dark_with_a_run_open() {
    let mut harness = render(Scheme::Dark, |app| {
        app.opened = Some(first_ahead(app));
    });
    harness.snapshot("planner_run_open");
}

#[test]
fn runs_before_now_have_gone_and_the_rest_are_scheduled() {
    let app = PlannerApp::on(Appearance::default());
    for run in &app.runs {
        if run.start >= app.now {
            assert_eq!(run.state, RunState::Scheduled);
        } else {
            assert_ne!(run.state, RunState::Scheduled);
        }
    }
    assert!(app.runs.iter().any(|r| r.state == RunState::Failed));
}

#[test]
fn a_job_unticked_leaves_the_week() {
    let mut harness = render(Scheme::Light, |_| {});
    let all = harness.state().events().1.len();
    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "sync-inventory")
        .click();
    harness.run_steps(3);
    assert!(harness.state().hidden[1]);
    let left = harness.state().events().1;
    assert!(left.len() < all);
    assert!(left.iter().all(|e| e.title != "sync-inventory"));
}

#[test]
fn a_run_ahead_can_be_skipped() {
    let mut harness = render(Scheme::Light, |app| {
        app.opened = Some(first_ahead(app));
    });
    harness.get_by_label("Skip this run").click();
    harness.run_steps(3);
    let app = harness.state();
    let id = app.opened.clone().expect("a run open");
    let run = app.runs.iter().find(|r| r.id == id).expect("the run");
    assert_eq!(run.state, RunState::Skipped);
}

#[test]
fn the_week_moves_back_and_on() {
    let mut harness = render(Scheme::Light, |_| {});
    let monday = harness.state().monday;
    harness.get_by_label("The next week").click();
    harness.run_steps(3);
    assert_eq!(harness.state().monday, monday.plus(7));
    assert!(harness.state().runs.iter().all(|r| r.state == RunState::Scheduled));
    harness.get_by_label("This week").click();
    harness.run_steps(3);
    assert_eq!(harness.state().monday, monday);
}
