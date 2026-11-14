//! The dashboards as they open, every kind of widget among them, live from the simulated source.

use dashboard::DashboardApp;
use eframe::egui;
use egui_kittest::Harness;
use sokkel_ui_egui::{Appearance, Scheme};

fn render(scheme: Scheme, setup: impl FnOnce(&mut DashboardApp)) -> Harness<'static, DashboardApp> {
    let mut app = DashboardApp::on(Appearance {
        scheme,
        ..Default::default()
    });
    app.prefs.live = false;
    setup(&mut app);
    let mut harness = Harness::builder().with_size([1440.0, 1500.0]).build_ui_state(
        |ui, app: &mut DashboardApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(8);
    harness
}

#[test]
fn the_overview() {
    let mut harness = render(Scheme::Light, |_| {});
    harness.snapshot("dashboard_overview");
}

#[test]
fn operations_in_dark() {
    let mut harness = render(Scheme::Dark, |app| app.open("operations"));
    harness.snapshot("dashboard_operations");
}

#[test]
fn the_team() {
    let mut harness = render(Scheme::Light, |app| app.open("team"));
    harness.snapshot("dashboard_team");
}
