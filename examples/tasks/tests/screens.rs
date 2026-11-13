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

/// A screen: its name in parity/tasks.json, its scheme, and how the app gets there.
type Screen = (&'static str, Scheme, fn(&mut TasksApp));

#[test]
fn every_screen() {
    let mut results = SnapshotResults::new();
    let screens: [Screen; 7] = [
        ("inbox", Scheme::Light, |_| {}),
        ("today", Scheme::Light, |app| app.go("today")),
        ("upcoming", Scheme::Light, |app| app.go("upcoming")),
        ("project", Scheme::Light, |app| app.go("harbor")),
        ("board", Scheme::Light, |app| app.prefs.view = View::Board),
        ("settings", Scheme::Light, |app| app.go("settings")),
        ("inbox_dark", Scheme::Dark, |_| {}),
    ];
    for (name, scheme, setup) in screens {
        let mut harness = render(scheme, setup);
        results.add(harness.try_snapshot(name));
    }
}
