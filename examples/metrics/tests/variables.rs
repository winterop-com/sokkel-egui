//! The variables bar: a service chosen lets go of a host of another service, as the web's.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use metrics::MetricsApp;
use sokkel_egui::Appearance;

#[test]
fn choosing_a_service_lets_go_of_a_host_of_another() {
    let mut app = MetricsApp::on(Appearance::default());
    app.live = false;
    app.variables.host = "api-1".into();
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut MetricsApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(3);
    harness.get_by_label("service").click();
    harness.run_steps(2);
    harness.get_by_label("worker").click();
    harness.run_steps(3);
    assert_eq!(harness.state().variables.service, "worker");
    assert_eq!(harness.state().variables.host, "all");
}
