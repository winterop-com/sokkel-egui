//! The settings over the board, as the web's: the demo's trouble started and calmed, the status
//! page's title and history set, a subscriber removed.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::Appearance;
use status::StatusApp;

fn settings() -> Harness<'static, StatusApp> {
    let mut app = StatusApp::on(Appearance::default());
    app.go("settings");
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
fn the_demo_starts_trouble_and_calms_it() {
    let mut harness = settings();
    harness.get_by_label("Simulation").click();
    harness.run_steps(3);
    harness.get_by_label("Error spike").click();
    harness.run_steps(2);
    harness.get_by_label("Start").click();
    harness.run_steps(2);
    let scenarios = harness.state().live_world.scenarios.clone();
    assert_eq!(scenarios.len(), 2, "a spike on api and trouble in Singapore");
    assert_eq!(scenarios[0].service, Some(1));
    harness.get_by_label("Calm down").click();
    harness.run_steps(2);
    assert!(harness.state().live_world.scenarios.is_empty());
}

#[test]
fn the_status_page_takes_its_title_and_history() {
    let mut harness = settings();
    // the section, not the rail's place of the same name
    harness
        .get_all_by_label("Status page")
        .last()
        .expect("the section")
        .click();
    harness.run_steps(3);
    harness.get_by_label("14 days").click();
    harness.run_steps(2);
    assert_eq!(harness.state().options.history_days, 14);
    harness
        .get_all_by_label("Show the last 90 days")
        .next()
        .expect("the switch")
        .click();
    harness.run_steps(2);
    assert!(!harness.state().options.show_uptime);
    harness.get_by_label("Open").click();
    harness.run_steps(4);
    assert_eq!(harness.state().place, "public");
    assert!(!harness.state().settings);
}

#[test]
fn a_subscriber_is_removed() {
    let mut harness = settings();
    harness.get_by_label("Subscribers").click();
    harness.run_steps(3);
    harness
        .query_all_by_label("Actions")
        .next()
        .expect("a subscriber's menu")
        .click();
    harness.run_steps(2);
    harness.get_by_label("Remove").click();
    harness.run_steps(3);
    assert_eq!(harness.state().subscribe.subscribers.len(), 1);
}
