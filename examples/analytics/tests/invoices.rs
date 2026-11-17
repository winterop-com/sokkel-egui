//! Revenue's invoices: a saved view picked from the menu sets the listing to its filters and sort,
//! the filter bar says what it keeps, and changing the listing marks the view changed.

use analytics::AnalyticsApp;
use analytics::app::DEMO_START;
use analytics::model::InvoiceStatus;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::Appearance;

fn harness() -> Harness<'static, AnalyticsApp> {
    let mut app = AnalyticsApp::at(DEMO_START, Appearance::default());
    app.go("revenue/invoices");
    app.run_for(5000);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
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
fn a_saved_view_sets_the_listing() {
    let mut harness = harness();
    harness.get_by_label("All invoices").click();
    harness.run_steps(3);
    harness.get_by_label("Failed payments").click();
    harness.run_steps(4);
    let app = harness.state();
    assert_eq!(app.invoices.chosen, "failed");
    assert_eq!(app.invoices.filters.len(), 1);
    assert_eq!(app.tables["invoices"].sort, Some((5, false)));
    let failed = app
        .world
        .invoices
        .iter()
        .filter(|i| i.status == InvoiceStatus::Failed)
        .count();
    assert!(failed > 0);
    harness.snapshot("analytics_invoices_failed");
}
