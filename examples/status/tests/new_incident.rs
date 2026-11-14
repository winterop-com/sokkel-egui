//! Opening an incident by hand: the dialog's refusals in the web's words, and the incident it
//! makes, the next code, investigating, with its first update.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::Appearance;
use status::{StatusApp, data, new_incident::NewIncident};

#[test]
fn the_dialog_says_what_is_missing_then_opens_the_incident() {
    let mut form = NewIncident::new(Some("api"), "grace");
    assert_eq!(form.why(), Some("Say what is wrong first."));
    form.title = "Slow logins".into();
    assert_eq!(form.why(), None);
    form.services = vec![false; data::SERVICES.len()];
    assert_eq!(form.why(), Some("Tick a service or pick a site it touches."));
    form.place = 1;
    form.message.clear();
    assert_eq!(form.why(), Some("Write the first update."));
    form.message = "We are looking into it.".into();
    let existing = data::seed_incidents(0.0);
    let made = form.make(&existing, 1000.0, "ada");
    assert_eq!(made.code, "INC-42");
    assert_eq!(made.state, data::State::Investigating);
    assert_eq!(made.assignee, "grace");
    assert_eq!(made.sites, vec![data::SITES[0].id]);
    assert_eq!(made.updates[0].message, "We are looking into it.");
}

#[test]
fn report_incident_opens_the_dialog_over_the_board() {
    let mut app = StatusApp::on(Appearance::default());
    app.tick();
    app.tick();
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut StatusApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness.get_by_label("Report incident").click();
    harness.run_steps(4);
    assert!(harness.state().new_incident.is_some());
    harness.snapshot("new_incident");
    harness.get_by_label("Cancel").click();
    harness.run_steps(3);
    assert!(harness.state().new_incident.is_none());
}
