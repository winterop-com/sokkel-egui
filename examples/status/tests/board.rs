//! The overview's board under the tiles: the sites on a map, what is open, and every service's
//! last ninety days.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::Appearance;
use status::StatusApp;

fn render() -> Harness<'static, StatusApp> {
    let mut app = StatusApp::on(Appearance::default());
    app.tick();
    app.tick();
    let mut harness = Harness::builder().with_size([1280.0, 1900.0]).build_ui_state(
        |ui, app: &mut StatusApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(6);
    harness
}

#[test]
fn the_board_stands_under_the_tiles() {
    let mut harness = render();
    harness.snapshot("overview_board");
}

#[test]
fn all_opens_the_incidents() {
    let mut harness = render();
    harness.get_by_label("All").click();
    harness.run_steps(3);
    assert_eq!(harness.state().place, "incidents");
}

#[test]
fn a_tile_opens_its_service() {
    let mut harness = render();
    // the tile is the first "api" on the page; the uptime list names it again below
    harness.get_all_by_label("api").next().expect("the api tile").click();
    harness.run_steps(3);
    assert_eq!(harness.state().place, "services");
    assert_eq!(harness.state().service.as_deref(), Some("api"));
}

#[test]
fn the_status_page_opens_from_the_rail_and_leads_back() {
    let mut harness = render();
    harness.get_by_label("Status page").click();
    harness.run_steps(3);
    assert_eq!(harness.state().place, "public");
    assert!(
        harness
            .query_by_label("All systems operational, as of 09:00:04")
            .is_some()
    );
    harness.get_by_label("Open the board").click();
    harness.run_steps(3);
    assert_eq!(harness.state().place, "overview");
}
