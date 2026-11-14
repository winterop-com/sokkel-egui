//! The services in a person's own order: a card dragged by its grip moves, and Reset order puts
//! the board's order back.

use eframe::egui::{self, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_ui_egui::Appearance;
use status::StatusApp;

fn services() -> Harness<'static, StatusApp> {
    let mut app = StatusApp::on(Appearance::default());
    app.go("services");
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut StatusApp| {
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
fn a_card_dragged_by_its_grip_moves_and_reset_puts_it_back() {
    let mut harness = services();
    let grip = harness.get_all_by_label("Move").next().unwrap().rect().center();
    harness.hover_at(grip);
    harness.run_steps(2);
    harness.drag_at(grip);
    harness.run_steps(2);
    // down past the second card
    for step in 1..=10 {
        harness.hover_at(grip + vec2(0.0, 20.0 * step as f32));
        harness.run_steps(2);
    }
    harness.drop_at(grip + vec2(0.0, 200.0));
    harness.run_steps(2);
    assert_ne!(harness.state().service_order, status::data::board_order());
    harness.get_by_label("Reset order").click();
    harness.run_steps(2);
    assert_eq!(harness.state().service_order, status::data::board_order());
}
