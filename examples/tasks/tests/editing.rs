//! Tasks edited in place, as a person edits them: a title typed and saved, an empty one refused, an
//! assignee chosen from the list, and an edit dropped with Escape.

use eframe::egui::{self, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_ui_egui::{Appearance, Day};
use tasks::TasksApp;

fn app(setup: impl FnOnce(&mut TasksApp)) -> Harness<'static, TasksApp> {
    let mut app = TasksApp::on(Day::parse("2026-11-16").unwrap(), Appearance::default());
    setup(&mut app);
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

fn titled(harness: &Harness<'_, TasksApp>, title: &str) -> bool {
    harness.state().tasks.iter().any(|task| task.title == title)
}

#[test]
fn a_typed_title_is_saved_with_enter() {
    let mut harness = app(|app| app.table.edit(0, 1));
    harness.event(egui::Event::Text("Rerun nightly-export again".into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(titled(&harness, "Rerun nightly-export again"));
    assert!(!titled(&harness, "Rerun nightly-export for last week"));
}

#[test]
fn an_empty_title_is_refused_and_escape_drops_it() {
    let mut harness = app(|app| app.table.edit(0, 1));
    harness.key_press(Key::Backspace);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    // refused: nothing saved, and the editor still open with why
    assert!(!harness.state().tasks.iter().any(|task| task.title.is_empty()));
    harness.key_press(Key::Escape);
    harness.run();
    assert!(titled(&harness, "Rerun nightly-export for last week"));
}

#[test]
fn an_assignee_is_chosen_from_the_list() {
    let mut harness = app(|app| app.table.edit(1, 3));
    harness.get_by_label("margaret").click();
    harness.run();
    let task = harness
        .state()
        .tasks
        .iter()
        .find(|task| task.title == "Review the failed sync-inventory runs")
        .unwrap();
    assert_eq!(task.assignee, "margaret");
}

#[test]
fn a_day_picked_from_the_calendar_is_saved() {
    let mut harness = app(|app| app.table.edit(0, 2));
    harness.get_by_label("Choose a date").click();
    harness.run();
    harness.get_by_label("2026-11-20").click();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let task = harness
        .state()
        .tasks
        .iter()
        .find(|task| task.title == "Rerun nightly-export for last week")
        .unwrap();
    assert_eq!(task.due, Some(Day::parse("2026-11-20").unwrap().0));
}

#[test]
fn a_character_typed_on_the_active_cell_opens_it_on_that_character() {
    let mut harness = app(|_| {});
    // the title cell made active by a press, then a key typed
    harness.state_mut().table.edit(0, 1);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    harness.event(egui::Event::Text("N".into()));
    harness.run();
    harness.event(egui::Event::Text("ew title".into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(titled(&harness, "New title"));
}

#[test]
fn the_calendar_moves_by_its_keys_and_enter_picks() {
    let mut harness = app(|app| app.table.edit(0, 2));
    harness.get_by_label("Choose a date").click();
    harness.run();
    // from Nov 15: a week on, then a day on
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let task = harness
        .state()
        .tasks
        .iter()
        .find(|task| task.title == "Rerun nightly-export for last week")
        .unwrap();
    assert_eq!(task.due, Some(Day::parse("2026-11-23").unwrap().0));
}
