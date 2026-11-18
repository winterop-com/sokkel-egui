//! The frame's own screens: the settings over the overview, the palette finding what is typed, and
//! the banner while the server does not answer.

use eframe::egui;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotResults};
use records::RecordsApp;
use records::app::DEMO_START;
use sokkel_egui::Appearance;

fn render(path: &str) -> Harness<'static, RecordsApp> {
    let mut app = RecordsApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut RecordsApp| {
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
fn settings_palette_and_banner() {
    let mut results = SnapshotResults::new();

    let mut harness = render("settings");
    results.add(harness.try_snapshot("records_settings"));
    harness.get_by_label("Demo server").click();
    harness.run_steps(4);
    results.add(harness.try_snapshot("records_settings_demo"));
    harness
        .get_all_by_label("The server answers")
        .next()
        .expect("the switch")
        .click();
    harness.run_steps(4);
    assert!(harness.state().backend.down, "the switch turns the server off");

    let mut harness = render("");
    harness.state_mut().palette = Some(("ho".into(), 0));
    harness.run_steps(4);
    results.add(harness.try_snapshot("records_palette"));

    let mut harness = render("");
    harness.state_mut().backend.down = true;
    harness.state_mut().check_server();
    harness.run_steps(4);
    results.add(harness.try_snapshot("records_down"));
}
