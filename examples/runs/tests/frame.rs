//! The frame: the rail narrowed to its icons and widened again, and the frame in a desktop window
//! whose title bar is hidden (the rail's head leaving room for the window's buttons on macOS).

use egui_kittest::{Harness, kittest::Queryable};
use runs::RunsApp;
use sokkel_egui::Appearance;

fn render(setup: impl FnOnce(&mut RunsApp)) -> Harness<'static, RunsApp> {
    let mut app = RunsApp::with(Appearance::default());
    setup(&mut app);
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut RunsApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
                .show(ui, |ui| app.show(ui))
                .inner
        },
        app,
    );
    harness.run_steps(3);
    harness
}

#[test]
fn the_rail_collapses_and_expands() {
    let mut harness = render(|_| {});
    harness.get_by_label("Collapse the navigation").click();
    harness.run_steps(3);
    assert!(harness.state().rail_collapsed);
    harness.snapshot("runs_rail_collapsed");
    harness.get_by_label("Expand the navigation").click();
    harness.run_steps(3);
    assert!(!harness.state().rail_collapsed);
}

#[cfg(target_os = "macos")]
#[test]
fn a_desktop_window_leaves_room_for_its_buttons() {
    let mut harness = render(|app| app.desktop = true);
    harness.snapshot("runs_desktop");
}

#[cfg(target_os = "macos")]
#[test]
fn a_collapsed_rail_keeps_its_head_under_the_buttons() {
    let mut harness = render(|app| {
        app.desktop = true;
        app.rail_collapsed = true;
    });
    harness.snapshot("runs_desktop_collapsed");
}
