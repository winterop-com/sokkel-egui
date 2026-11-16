//! A service's Dependencies: every service drawn with what it calls on, the one shown marked, and a
//! press on another opening that one, as the web's `ServiceGraph`.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::Appearance;
use status::StatusApp;

#[test]
fn a_press_on_a_service_in_the_graph_opens_it() {
    let mut app = StatusApp::on(Appearance::default());
    app.tick();
    app.go("services/api");
    let mut harness = Harness::builder().with_size([1280.0, 1500.0]).build_ui_state(
        |ui, app: &mut StatusApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness.get_by_label("worker").click();
    harness.run_steps(2);
    assert_eq!(harness.state().service.as_deref(), Some("worker"));
}
