//! The board as a person uses it: a card dragged to another column, and moved or deleted from its
//! menu.

use eframe::egui::{self, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::{Appearance, Day};
use tasks::{TaskState, TasksApp, View};

fn board() -> Harness<'static, TasksApp> {
    let mut app = TasksApp::on(Day::parse("2026-11-16").unwrap(), Appearance::default());
    app.prefs.view = View::Board;
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

fn state_of(harness: &Harness<'_, TasksApp>, title: &str) -> Option<TaskState> {
    harness
        .state()
        .tasks
        .iter()
        .find(|task| task.title == title)
        .map(|task| task.state)
}

const FIRST: &str = "Rerun nightly-export for last week";

#[test]
fn a_card_dragged_to_another_column_takes_its_state() {
    let mut harness = board();
    // the first card of To do, held away from its menu button
    let button = harness.get_all_by_label("More actions").next().unwrap().rect();
    let start = button.left_center() - vec2(80.0, 0.0);
    harness.hover_at(start);
    harness.run();
    harness.drag_at(start);
    harness.run();
    // over Doing's column, under its cards
    for step in 1..=10 {
        harness.hover_at(start + vec2(30.0 * step as f32, 30.0 * step as f32));
        harness.run();
    }
    harness.drop_at(start + vec2(300.0, 300.0));
    harness.run();
    assert_eq!(state_of(&harness, FIRST), Some(TaskState::Doing));
}

#[test]
fn a_card_moves_and_goes_from_its_menu() {
    let mut harness = board();
    let button = harness.get_all_by_label("More actions").next().unwrap().rect();
    harness.hover_at(button.center());
    harness.run();
    harness.get_all_by_label("More actions").next().unwrap().click();
    harness.run();
    harness.get_by_label("Done").click();
    harness.run();
    assert_eq!(state_of(&harness, FIRST), Some(TaskState::Done));

    let before = harness.state().tasks.len();
    let button = harness.get_all_by_label("More actions").next().unwrap().rect();
    harness.hover_at(button.center());
    harness.run();
    harness.get_all_by_label("More actions").next().unwrap().click();
    harness.run();
    harness.get_by_label("Delete").click();
    harness.run();
    assert_eq!(harness.state().tasks.len(), before - 1);
}
