//! The screens parity/tasks.json lists, drawn as the web example's are shot: 1280 by 800, on
//! 2026-11-16, fresh data. `make parity` puts them beside the web's.

use eframe::egui;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotResults};
use sokkel_ui_egui::{Appearance, Day, Scheme};
use tasks::{TasksApp, View};

fn render(scheme: Scheme, setup: impl FnOnce(&mut TasksApp)) -> Harness<'static, TasksApp> {
    render_at([1280.0, 800.0], scheme, setup)
}

fn render_at(size: [f32; 2], scheme: Scheme, setup: impl FnOnce(&mut TasksApp)) -> Harness<'static, TasksApp> {
    let mut app = TasksApp::on(
        Day::parse("2026-11-16").unwrap(),
        Appearance {
            scheme,
            ..Default::default()
        },
    );
    setup(&mut app);
    let mut harness = Harness::builder().with_size(size).build_ui_state(
        |ui, app: &mut TasksApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness
}

/// A screen: its name in parity/tasks.json, its scheme (System is light here, as the web's default), and how the app gets there.
type Screen = (&'static str, Scheme, fn(&mut TasksApp));

#[test]
fn every_screen() {
    let mut results = SnapshotResults::new();
    let screens: [Screen; 14] = [
        ("inbox", Scheme::System, |_| {}),
        ("today", Scheme::System, |app| app.go("today")),
        ("today_empty", Scheme::System, |app| {
            app.go("today");
            let today = app.today;
            for task in &mut app.tasks {
                if task.due.is_some_and(|due| due <= today.0) {
                    task.state = tasks::TaskState::Done;
                }
            }
        }),
        ("upcoming", Scheme::System, |app| app.go("upcoming")),
        ("project", Scheme::System, |app| app.go("harbor")),
        ("edit_title", Scheme::System, |app| app.table.edit(0, 1)),
        ("edit_due", Scheme::System, |app| app.table.edit(0, 2)),
        ("edit_assignee", Scheme::System, |app| app.table.edit(1, 3)),
        ("board", Scheme::System, |app| app.prefs.view = View::Board),
        ("settings", Scheme::System, |app| app.go("settings")),
        ("settings_appearance", Scheme::System, |app| {
            app.settings = Some("appearance".into())
        }),
        ("settings_demo", Scheme::System, |app| {
            app.settings = Some("demo".into())
        }),
        ("settings_reset", Scheme::System, |app| {
            app.settings = Some("demo".into());
            app.resetting = true;
        }),
        ("inbox_dark", Scheme::Dark, |_| {}),
    ];
    for (name, scheme, setup) in screens {
        let mut harness = render(scheme, setup);
        results.add(harness.try_snapshot(name));
    }
    // the date editor with its calendar open, by its button as a person would
    let mut harness = render(Scheme::System, |app| app.table.edit(0, 2));
    harness.get_by_label("Choose a date").click();
    harness.run_steps(3);
    harness.remove_cursor();
    harness.run_steps(2);
    results.add(harness.try_snapshot("edit_due_calendar"));

    // a card's menu on the board, opened from its button as the pointer rests on the card
    let mut harness = render(Scheme::System, |app| app.prefs.view = View::Board);
    let button = harness.get_all_by_label("More actions").next().unwrap().rect();
    harness.hover_at(button.center());
    harness.run_steps(2);
    harness.get_all_by_label("More actions").next().unwrap().click();
    harness.run_steps(3);
    results.add(harness.try_snapshot("board_menu"));

    // the command palette, opened by its keys
    let mut harness = render(Scheme::System, |_| {});
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    harness.run_steps(3);
    results.add(harness.try_snapshot("palette"));

    // the account menu, opened from the rail's foot
    let mut harness = render(Scheme::System, |_| {});
    harness.get_by_label("Account").click();
    harness.run_steps(3);
    harness.remove_cursor();
    harness.run_steps(2);
    results.add(harness.try_snapshot("account"));

    // a phone, 390 by 844: the rail a drawer behind the top bar's menu button
    let phone = [390.0, 844.0];
    let mut harness = render_at(phone, Scheme::System, |_| {});
    results.add(harness.try_snapshot("phone"));
    let mut harness = render_at(phone, Scheme::System, |_| {});
    harness.get_by_label("Open the navigation").click();
    harness.run_steps(3);
    harness.remove_cursor();
    harness.run_steps(2);
    results.add(harness.try_snapshot("phone_drawer"));

    // a stacked row's title pressed: its editor in the title's place
    let mut harness = render_at(phone, Scheme::System, |_| {});
    harness.get_by_label("Rerun nightly-export for last week").click();
    harness.run_steps(3);
    harness.remove_cursor();
    harness.run_steps(2);
    results.add(harness.try_snapshot("phone_edit_title"));
}
