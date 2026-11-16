//! The Regions page: a region's chip narrows the sites table to its sites, All brings every one back.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::Appearance;
use status::StatusApp;

#[test]
fn a_region_chip_narrows_the_sites() {
    let mut app = StatusApp::on(Appearance::default());
    app.go("regions");
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut StatusApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    assert!(harness.query_by_label("11 rows").is_some());
    harness.get_by_label("Europe").click();
    harness.run_steps(4);
    assert_eq!(harness.state().region, "Europe");
    assert!(harness.query_by_label("3 rows").is_some());
    harness.get_by_label("All").click();
    harness.run_steps(4);
    assert!(harness.query_by_label("11 rows").is_some());
}

#[test]
fn a_site_chosen_opens_its_card_and_the_incident_covering_it() {
    let mut app = StatusApp::on(Appearance::default());
    app.go("regions");
    app.sites_map.selected = Some("frankfurt".into());
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut StatusApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness.snapshot("regions_card");
    harness.get_by_label("Open INC-41").click();
    harness.run_steps(4);
    assert_eq!(harness.state().incident.as_deref(), Some("incident-41"));
}
