//! The frame on a phone, 390 wide: the rail a drawer behind the top bar's menu button, closed by a
//! place chosen in it or by Escape.

use eframe::egui::{self, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_ui_egui::{Appearance, Day};
use tasks::TasksApp;

fn phone() -> Harness<'static, TasksApp> {
    let app = TasksApp::on(Day::parse("2026-11-16").unwrap(), Appearance::default());
    let mut harness = Harness::builder().with_size([390.0, 844.0]).build_ui_state(
        |ui, app: &mut TasksApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness
}

fn drawer_open(harness: &Harness<'_, TasksApp>) -> bool {
    harness.query_all_by_label("Close the navigation").next().is_some()
}

#[test]
fn a_place_chosen_in_the_drawer_opens_and_closes_it() {
    let mut harness = phone();
    assert!(!drawer_open(&harness));
    harness.get_by_label("Open the navigation").click();
    harness.run_steps(3);
    assert!(drawer_open(&harness));
    harness.get_by_label("juniper").click();
    harness.run_steps(3);
    assert_eq!(harness.state().list, "juniper");
    assert!(!drawer_open(&harness));
}

#[test]
fn escape_closes_the_drawer() {
    let mut harness = phone();
    harness.get_by_label("Open the navigation").click();
    harness.run_steps(3);
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert!(!drawer_open(&harness));
}
