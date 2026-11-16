//! The incidents as a person looks through them: every incident shown, then "/" reaching the
//! search and a word narrowing the rows.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::Appearance;
use status::StatusApp;

fn incidents() -> Harness<'static, StatusApp> {
    let mut app = StatusApp::on(Appearance::default());
    app.go("incidents");
    app.shown = 2;
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
fn slash_reaches_the_search_and_a_word_narrows_the_rows() {
    let mut harness = incidents();
    assert!(harness.query_by_label("9 rows").is_some());
    harness.event(egui::Event::Text("/".into()));
    harness.run_steps(2);
    harness.event(egui::Event::Text("search".into()));
    harness.run_steps(3);
    assert_eq!(harness.state().incident_search, "search");
    // "Slow searches in Asia" and "Search results out of date"
    assert!(harness.query_by_label("2 rows").is_some());
}

#[test]
fn an_incident_resolves_reopens_and_goes_to_the_person_on_call() {
    let mut app = status::StatusApp::on(sokkel_egui::Appearance::default());
    app.resolve_or_reopen("incident-41");
    let inc = app.incidents.iter().find(|i| i.id == "incident-41").unwrap();
    assert_eq!(inc.state, status::data::State::Resolved);
    assert!(inc.resolved.is_some());
    assert_eq!(inc.updates[0].message, "This incident is resolved.");
    app.resolve_or_reopen("incident-41");
    let inc = app.incidents.iter().find(|i| i.id == "incident-41").unwrap();
    assert_eq!(inc.state, status::data::State::Investigating);
    assert!(inc.resolved.is_none());
    app.on_call = "linus".into();
    app.assign_to_on_call("incident-41");
    assert_eq!(
        app.incidents.iter().find(|i| i.id == "incident-41").unwrap().assignee,
        "linus"
    );
}

#[test]
fn an_update_posted_moves_the_incident_on() {
    use eframe::egui;
    use egui_kittest::{Harness, kittest::Queryable};
    let mut app = status::StatusApp::on(sokkel_egui::Appearance::default());
    app.go("incidents/incident-41");
    assert_eq!(app.post_state, 3, "the form starts on the state after monitoring");
    app.post_state = 2;
    let mut harness = Harness::builder().with_size([1280.0, 900.0]).build_ui_state(
        |ui, app: &mut status::StatusApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(3);
    harness.state_mut().post_message = "Exports are caught up.".into();
    harness.run_steps(2);
    harness.get_by_label("Post").click();
    harness.run_steps(3);
    let app = harness.state();
    let incident = app.incidents.iter().find(|i| i.id == "incident-41").unwrap();
    assert_eq!(incident.updates.len(), 4);
    assert_eq!(incident.updates[0].message, "Exports are caught up.");
    assert!(app.post_message.is_empty());
}
