//! What the board does on its own, as the web's: an incident opened for trouble, the on-call
//! responder moving it along until it is resolved, notices for what changes unless the settings
//! say never, and the maps kept across a reset of the demo.

use sokkel_egui::Appearance;
use status::data::State;
use status::live::ScenarioKind;
use status::{Notify, StatusApp};

fn now(app: &StatusApp) -> f64 {
    f64::from(app.start + app.elapsed) * 1000.0
}

#[test]
fn trouble_opens_an_incident_that_the_responder_sees_through() {
    let mut app = StatusApp::on(Appearance::default());
    let api = status::data::SERVICES.iter().position(|s| s.id == "api").unwrap();
    let at = now(&app);
    app.live_world.trigger(ScenarioKind::Spike, Some(api), at);
    for _ in 0..6 {
        app.tick();
    }
    let incident = app.incidents.first().expect("an incident").clone();
    assert_eq!(incident.title, "Elevated error rates on api");
    assert_eq!(incident.assignee, "grace", "given to whoever is on call");
    assert!(incident.cause.is_some());
    assert!(
        app.notices
            .iter()
            .any(|n| n.title == format!("{} opened", incident.code)),
        "and told"
    );
    // the responder finds the cause after a while
    for _ in 0..8 {
        app.tick();
    }
    assert_ne!(app.incidents[0].state, State::Investigating);
    // calmed, the service recovers and the incident is watched, then resolved
    app.live_world.calm_down();
    for _ in 0..20 {
        app.tick();
    }
    assert_eq!(app.incidents[0].state, State::Resolved);
    assert!(
        app.notices
            .iter()
            .any(|n| n.title == format!("{} resolved", incident.code))
    );
}

#[test]
fn never_keeps_the_notices_quiet() {
    let mut app = StatusApp::on(Appearance::default());
    app.options.notify = Notify::Off;
    let at = now(&app);
    app.live_world.trigger(ScenarioKind::Spike, Some(1), at);
    for _ in 0..6 {
        app.tick();
    }
    assert!(app.incidents.len() > 9, "the incident opens all the same");
    assert!(app.notices.is_empty());
}

#[test]
fn a_reset_keeps_the_maps_and_the_settings() {
    let mut app = StatusApp::on(Appearance::default());
    app.sites_map.basemap = Some(sokkel_egui_maps::Basemap::Satellite);
    app.options.history_days = 14;
    app.incidents.clear();
    app.reset_demo();
    assert_eq!(app.sites_map.basemap, Some(sokkel_egui_maps::Basemap::Satellite));
    assert_eq!(app.options.history_days, 14);
    assert!(!app.incidents.is_empty(), "the incidents back as the demo started");
}
