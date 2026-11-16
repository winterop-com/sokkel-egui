//! The command palette, from the top bar's Search or the shortcut, as the web's.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::Appearance;
use status::StatusApp;

fn render() -> Harness<'static, StatusApp> {
    let mut harness = Harness::builder().with_size([1280.0, 900.0]).build_ui_state(
        |ui, app: &mut StatusApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        StatusApp::on(Appearance::default()),
    );
    harness.run_steps(4);
    harness
}

fn run(harness: &mut Harness<'static, StatusApp>, words: &str) {
    harness.event(egui::Event::Text(words.into()));
    harness.run_steps(2);
    harness.key_press(egui::Key::Enter);
    harness.run_steps(2);
}

#[test]
fn search_opens_the_palette_and_a_service_is_gone_to() {
    let mut harness = render();
    harness.get_by_label("Search").click();
    harness.run_steps(3);
    assert!(harness.state().palette.is_some());
    harness.snapshot("palette");
    run(&mut harness, "scheduler");
    assert_eq!(harness.state().place, "services");
    assert_eq!(harness.state().service.as_deref(), Some("scheduler"));
    assert!(harness.state().palette.is_none());
}

#[test]
fn the_shortcut_pauses_the_readings_and_reports_an_incident() {
    let mut harness = render();
    let was = harness.state().live;
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    harness.run_steps(2);
    run(&mut harness, if was { "pause live" } else { "resume live" });
    assert_ne!(harness.state().live, was);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    harness.run_steps(2);
    run(&mut harness, "report an incident");
    assert!(harness.state().new_incident.is_some());
}

#[test]
fn an_incident_is_found_by_its_code() {
    let mut harness = render();
    let (id, code) = {
        let first = &harness.state().incidents[0];
        (first.id.clone(), first.code.clone())
    };
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    harness.run_steps(2);
    run(&mut harness, &code);
    assert_eq!(harness.state().place, "incidents");
    assert_eq!(harness.state().incident.as_deref(), Some(id.as_str()));
}

#[test]
fn hand_over_gives_the_pager_and_the_open_incident() {
    let mut harness = render();
    let from = harness.state().on_call.clone();
    let held: Vec<String> = harness
        .state()
        .incidents
        .iter()
        .filter(|i| i.state != status::data::State::Resolved && i.assignee == from)
        .map(|i| i.id.clone())
        .collect();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    harness.run_steps(2);
    run(&mut harness, "hand over");
    assert!(harness.state().hand_over.is_some());
    harness.state_mut().hand_over.as_mut().unwrap().note = "Watch the worker.".into();
    harness.run_steps(2);
    harness.snapshot("hand_over");
    harness.get_by_label("Hand over").click();
    harness.run_steps(3);
    let app = harness.state();
    assert!(app.hand_over.is_none());
    assert_ne!(app.on_call, from);
    assert_eq!(app.on_call_note, "Watch the worker.");
    assert_eq!(app.handovers[0].from, from);
    for id in held {
        let incident = app.incidents.iter().find(|i| i.id == id).unwrap();
        assert_eq!(incident.assignee, app.on_call);
    }
}
