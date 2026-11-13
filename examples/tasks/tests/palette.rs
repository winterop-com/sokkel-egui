//! The command palette as a person uses it: the look changed from its Scheme group, and a place
//! reached from its Lists group.

use eframe::egui::{self, Key, Modifiers};
use egui_kittest::Harness;
use sokkel_ui_egui::{Appearance, Day, Scheme};
use tasks::TasksApp;

fn app() -> Harness<'static, TasksApp> {
    let app = TasksApp::on(Day::parse("2026-11-16").unwrap(), Appearance::default());
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
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

#[test]
fn the_scheme_changes_from_the_palette() {
    let mut harness = app();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run_steps(2);
    harness.event(egui::Event::Text("dark".into()));
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().appearance.scheme, Scheme::Dark);
}

#[test]
fn a_list_is_reached_from_the_palette() {
    let mut harness = app();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run_steps(2);
    harness.event(egui::Event::Text("juniper".into()));
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().list, "juniper");
}
