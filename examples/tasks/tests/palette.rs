//! The command palette as a person uses it: the look changed from its Scheme group, and a place
//! reached from its Lists group.

use eframe::egui::{self, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::{Appearance, Day, Scheme};
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

#[test]
fn ctrl_k_opens_the_palette_too_and_from_a_field() {
    let mut harness = app();
    harness.key_press_modifiers(Modifiers::CTRL, Key::K);
    harness.run_steps(2);
    assert!(open(&harness));
    harness.key_press(Key::Escape);
    harness.run_steps(2);
    assert!(!open(&harness));
    // a title being typed keeps none of it, and the palette opens over it
    harness.state_mut().table.edit(0, 1);
    harness.run_steps(2);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run_steps(2);
    assert!(open(&harness));
}

/// The palette is open: its Density group's Compact, which only the palette shows, is there.
fn open(harness: &Harness<'_, TasksApp>) -> bool {
    harness.query_by_label("Compact").is_some()
}

#[test]
fn mod_k_closes_an_open_palette() {
    let mut harness = app();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run_steps(2);
    assert!(open(&harness));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run_steps(2);
    assert!(!open(&harness));
}

#[test]
fn the_arrows_stop_at_the_ends_and_end_reaches_the_last() {
    let mut harness = app();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run_steps(2);
    // up from the first row stays on it, so two down from there is the third command, Today (were
    // it to wrap, up would go to the last and two down to the second)
    harness.key_press(Key::ArrowUp);
    harness.run_steps(1);
    harness.key_press(Key::ArrowDown);
    harness.run_steps(1);
    harness.key_press(Key::ArrowDown);
    harness.run_steps(1);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().list, "today");
    // End reaches the last command, Demo's Reset demo data, which asks to reset
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run_steps(2);
    harness.key_press(Key::End);
    harness.run_steps(1);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert!(harness.state().resetting);
}
