//! The screens parity/records.json lists, drawn as the web's are shot: the app opened at
//! 2026-11-18 09:00 UTC, the server's answer in. `bun scripts/parity.ts board records` puts them
//! beside the web's.

use eframe::egui;
use egui_kittest::{Harness, SnapshotResults};
use records::RecordsApp;
use records::app::DEMO_START;
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, RecordsApp> {
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
fn every_screen() {
    let mut results = SnapshotResults::new();
    let screens: &[(&str, &str)] = &[
        ("records_overview", ""),
        ("records_forms", "forms"),
        ("records_form", "forms/FRM-MONTH"),
        ("records_form_site", "forms/FRM-SITE"),
        ("records_form_layout", "forms/FRM-MONTH/layout"),
        ("records_form_layout_site", "forms/FRM-SITE/layout"),
    ];
    for &(name, path) in screens {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
}
