//! The screens parity/records.json lists, drawn as the web's are shot: the app opened at
//! 2026-11-18 09:00 UTC, the server's answer in. `bun scripts/parity.ts board records` puts them
//! beside the web's.

use eframe::egui;
use egui_kittest::{Harness, SnapshotResults};
use records::RecordsApp;
use records::app::DEMO_START;
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, RecordsApp> {
    render_at(path, 900.0)
}

pub fn render_at(path: &str, height: f32) -> Harness<'static, RecordsApp> {
    let mut app = RecordsApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, height]).build_ui_state(
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
        ("records_responses", "responses"),
        ("records_households", "people"),
        ("records_places", "places"),
        ("records_places_region", "places/bong"),
        ("records_server", "server"),
        ("records_health", "health"),
    ];
    for &(name, path) in screens {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
    for (name, path) in [
        ("records_receipt", "responses/9832602a1b1ccacffb3c8507bd2d5fdf"),
        ("records_receipt_site", "responses/7f3d438ab4b07185d993c7ecf06aad0e"),
        ("records_household", "people/household-1"),
        ("records_health_all", "health"),
    ] {
        let mut harness = render_at(path, 1500.0);
        results.add(harness.try_snapshot(name));
    }
    // a kind opened into the search parameters it reads
    let mut harness = render_at("server", 1300.0);
    use egui_kittest::kittest::Queryable;
    harness
        .get_all_by_label("Expand")
        .nth(1)
        .expect("a kind's expand")
        .click();
    harness.run_steps(4);
    results.add(harness.try_snapshot("records_server_open"));
}
