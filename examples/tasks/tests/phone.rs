//! The frame on a phone, 390 wide: the rail a drawer behind the top bar's menu button, closed by a
//! place chosen in it or by Escape.

use eframe::egui::{self, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::{Appearance, Day};
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
    harness.get_by_label("Upcoming").click();
    harness.run_steps(3);
    assert_eq!(harness.state().list, "upcoming");
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

#[test]
fn a_stacked_rows_box_marks_its_task_done() {
    let mut harness = phone();
    let before = harness.state().tasks.iter().filter(|task| task.done()).count();
    harness.get_all_by_label("Done").next().unwrap().click();
    harness.run_steps(3);
    let after = harness.state().tasks.iter().filter(|task| task.done()).count();
    assert_eq!(after, before + 1);
}

#[test]
fn a_stacked_title_is_edited_in_place() {
    let mut harness = phone();
    harness.get_by_label("Rerun nightly-export for last week").click();
    harness.run_steps(3);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    harness.event(egui::Event::Text("Rerun nightly-export again".into()));
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert!(
        harness
            .state()
            .tasks
            .iter()
            .any(|task| task.title == "Rerun nightly-export again")
    );
}

#[test]
fn a_stacked_rows_assignee_is_chosen_from_its_fact() {
    let mut harness = phone();
    // the second row's assignee, grace
    harness.get_all_by_label("grace").next().unwrap().click();
    harness.run_steps(3);
    // the list's option, drawn over the rows
    harness.get_all_by_label("margaret").last().unwrap().click();
    harness.run_steps(3);
    let task = harness
        .state()
        .tasks
        .iter()
        .find(|task| task.title == "Review the failed sync-inventory runs")
        .unwrap();
    assert_eq!(task.assignee, "margaret");
}

#[test]
fn a_long_stacked_list_draws_only_the_rows_in_view_while_one_is_edited() {
    let mut harness = phone();
    {
        let app = harness.state_mut();
        let first = app.tasks[0].clone();
        for n in 0..300 {
            let mut task = first.clone();
            task.id = format!("more-{n}");
            task.title = format!("Rerun nightly-export, part {n}");
            task.rank += 1000.0 + n as f64;
            app.tasks.push(task);
        }
        app.table.edit(0, 1);
    }
    harness.run_steps(4);
    let drawn = harness.query_all_by_label("Actions").count();
    assert!(drawn > 5 && drawn < 30, "{drawn} rows drawn");
}
