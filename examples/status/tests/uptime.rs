//! A service's uptime: the last ninety days from the incident history, and a day pressed listing
//! what happened on it.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{Appearance, overall_uptime};
use status::StatusApp;

fn render(route: &str) -> Harness<'static, StatusApp> {
    let mut app = StatusApp::on(Appearance::default());
    app.tick();
    app.tick();
    app.go(route);
    let mut harness = Harness::builder().with_size([1280.0, 1100.0]).build_ui_state(
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
fn the_days_with_incidents_lose_uptime_and_take_their_color() {
    let app = StatusApp::on(Appearance::default());
    let uptime = app.uptime();
    let api = &uptime[0];
    assert_eq!(api.len(), 90);
    assert_eq!(api.last().map(|d| d.date.to_string()), Some("2026-11-16".into()));
    // INC-41 is open on today, and an incident three days before touched api and web
    let three = &api[86];
    assert!(three.uptime.is_some_and(|u| u < 1.0));
    assert!(three.status.is_some() && three.note.is_some());
    assert!(overall_uptime(api).is_some_and(|o| o > 0.98 && o < 1.0));
}

#[test]
fn a_day_pressed_lists_its_incidents() {
    let mut harness = render("services/api/2026-11-13");
    assert!(harness.query_by_label_contains("Press a day").is_none());
    harness.snapshot("service_day");
    harness.state_mut().go("services/api/2026-11-12");
    harness.run_steps(2);
    assert!(
        harness
            .query_by_label_contains("Nothing happened to api on Nov 12, 2026.")
            .is_some()
    );
}
