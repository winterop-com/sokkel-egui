//! Responses: a row opens its receipt in a sheet beside the list, and Send on there sends it on.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use records::RecordsApp;
use records::app::DEMO_START;
use records::backend::ReceiptState;
use sokkel_egui::Appearance;

#[test]
fn a_response_opens_in_the_sheet_and_is_sent_on() {
    let mut app = RecordsApp::at(DEMO_START, Appearance::default());
    app.go("responses");
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut RecordsApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    let code = "7f3d438ab4b07185d993c7ecf06aad0e";
    harness.state_mut().responses.opened = Some(code.into());
    harness.run_steps(4);
    harness.snapshot("records_response_sheet");
    harness.get_all_by_label("Send on").last().unwrap().click();
    harness.run_steps(4);
    let state = harness.state().backend.receipt_of(code).map(|r| r.state);
    assert_eq!(state, Some(ReceiptState::Forwarded));
}
