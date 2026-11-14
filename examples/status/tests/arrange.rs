//! The overview arranged: Edit puts the widgets over the board and the tools in each panel's head,
//! a widget pressed adds its panel, Done keeps the board as arranged and Cancel throws it away.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::{Appearance, DashboardLayoutItem, PanelSpan};
use status::{StatusApp, default_layout};

fn render(setup: impl FnOnce(&mut StatusApp)) -> Harness<'static, StatusApp> {
    let mut app = StatusApp::on(Appearance::default());
    app.tick();
    app.tick();
    setup(&mut app);
    let mut harness = Harness::builder().with_size([1280.0, 2400.0]).build_ui_state(
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

fn ids(layout: &[DashboardLayoutItem]) -> Vec<&str> {
    layout.iter().map(|i| i.id.as_str()).collect()
}

#[test]
fn every_panel_arranged() {
    // the whole board laid out, as a reader might keep it, in edit mode
    let mut harness = render(|app| {
        app.overview_layout = vec![
            DashboardLayoutItem::new("latency", PanelSpan::Half),
            DashboardLayoutItem::new("requests", PanelSpan::Half),
            DashboardLayoutItem::new("on-call", PanelSpan::Third),
            DashboardLayoutItem::new("maintenance", PanelSpan::Third),
            DashboardLayoutItem::new("deploys", PanelSpan::Third),
        ];
        let kept = app.overview_layout.clone();
        app.overview_edit.edit(&kept);
    });
    harness.snapshot("overview_arranged");
}

#[test]
fn a_widget_added_and_kept() {
    let mut harness = render(|_| {});
    harness.get_by_label("Edit").click();
    harness.run_steps(3);
    assert!(harness.query_by_label("Report incident").is_none());
    harness.get_by_label("Add Latency").click();
    harness.run_steps(3);
    harness.get_by_label("Remove Sites").click();
    harness.run_steps(3);
    harness.get_by_label("Done").click();
    harness.run_steps(3);
    assert_eq!(ids(&harness.state().overview_layout), ["open", "uptime", "latency"]);
}

#[test]
fn cancel_leaves_the_board_as_it_was() {
    let mut harness = render(|_| {});
    harness.get_by_label("Edit").click();
    harness.run_steps(3);
    harness.get_by_label("Remove Uptime").click();
    harness.run_steps(3);
    harness.get_by_label("Cancel").click();
    harness.run_steps(3);
    assert_eq!(harness.state().overview_layout, default_layout());
    assert!(harness.query_by_label("Report incident").is_some());
}
