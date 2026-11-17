//! The screens parity/analytics.json lists, drawn as the web's are shot: the app opened at
//! 2026-11-18 09:00 UTC and run for five seconds of its own time, the live stream's first events
//! in. `bun scripts/parity.ts board analytics` puts them beside the web's.

use analytics::AnalyticsApp;
use analytics::app::DEMO_START;
use eframe::egui;
use egui_kittest::{Harness, SnapshotResults};
use sokkel_egui::Appearance;

pub fn render(path: &str, height: f32) -> Harness<'static, AnalyticsApp> {
    let mut app = AnalyticsApp::at(DEMO_START, Appearance::default());
    app.go(path);
    app.run_for(5000);
    let mut harness = Harness::builder().with_size([1440.0, height]).build_ui_state(
        |ui, app: &mut AnalyticsApp| {
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
    for (name, path, height) in [
        ("analytics_overview", "", 1100.0),
        ("analytics_revenue", "revenue", 1100.0),
        ("analytics_invoices", "revenue/invoices", 900.0),
        ("analytics_customers", "customers", 900.0),
        ("analytics_customer", "customers/t20606-12", 1100.0),
        ("analytics_funnels", "funnels", 900.0),
        ("analytics_funnel", "funnels/signup-to-paid", 900.0),
    ] {
        let mut harness = render(path, height);
        results.add(harness.try_snapshot(name));
    }
}
