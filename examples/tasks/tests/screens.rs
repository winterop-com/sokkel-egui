//! The screens parity/tasks.json lists, drawn as the web example's are shot: 1280 by 800, on
//! 2026-11-16, fresh data. `make parity` puts them beside the web's.

use egui_kittest::{Harness, SnapshotResults};
use sokkel_ui_egui::{Appearance, Day, Scheme};
use tasks::{TasksApp, View};

fn render(scheme: Scheme, setup: impl FnOnce(&mut TasksApp)) -> Harness<'static, TasksApp> {
    let mut app = TasksApp::on(
        Day::parse("2026-11-16").unwrap(),
        Appearance {
            scheme,
            ..Default::default()
        },
    );
    setup(&mut app);
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
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
    let screens: [Screen; 10] = [
        ("inbox", Scheme::System, |_| {}),
        ("today", Scheme::System, |app| app.go("today")),
        ("upcoming", Scheme::System, |app| app.go("upcoming")),
        ("project", Scheme::System, |app| app.go("harbor")),
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
}
