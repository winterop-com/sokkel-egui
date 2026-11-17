//! Alerts: the rules read at 09:00:05 as the web's are, what fired, a rule made, turned off and
//! deleted, the resolved alerts cleared, and evaluating turned off in Settings.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use metrics::MetricsApp;
use metrics::alerting::{RuleState, written};
use sokkel_egui::Appearance;

fn harness() -> Harness<'static, MetricsApp> {
    let mut app = MetricsApp::on(Appearance::default());
    app.live = false;
    app.place = "alerts".into();
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut MetricsApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(3);
    harness
}

#[test]
fn the_rules_read_as_the_web_s() {
    let mut harness = harness();
    let app = harness.state();
    let now: Vec<String> = app
        .alerting
        .rules
        .iter()
        .map(|r| written(r.metric, app.alerting.readings[&r.id].value))
        .collect();
    assert_eq!(now, ["146 ms", "0.42%", "49", "75%", "332 req/s"]);
    assert_eq!(app.alerting.readings["rule-requests-low"].state, RuleState::Paused);
    assert_eq!(app.alerting.events.len(), 69);
    harness.snapshot("metrics_alerts");
    harness.get_by_label("Fired").click();
    harness.run_steps(3);
    harness.snapshot("metrics_alerts_fired");
}

#[test]
fn a_rule_is_made_turned_off_and_deleted() {
    let mut harness = harness();
    harness.get_by_label("New rule").click();
    harness.run_steps(4);
    harness.snapshot("metrics_rule_dialog");
    harness.state_mut().alerts.rule.as_mut().unwrap().draft.name = "api slow".into();
    harness.run_steps(2);
    harness.get_by_label("Add rule").click();
    harness.run_steps(3);
    assert_eq!(harness.state().alerting.rules.len(), 6);
    harness.get_by_label("Turn api slow off").click();
    harness.run_steps(3);
    let app = harness.state();
    let made = app.alerting.rules.last().unwrap();
    assert!(!made.enabled);
    assert_eq!(app.alerting.readings[&made.id].state, RuleState::Paused);
    harness.get_all_by_label("Actions").last().unwrap().click();
    harness.run_steps(2);
    harness.get_by_label("Delete").click();
    harness.run_steps(3);
    harness.get_by_label("Delete").click();
    harness.run_steps(3);
    assert_eq!(harness.state().alerting.rules.len(), 5);
}

#[test]
fn the_resolved_alerts_are_cleared() {
    let mut harness = harness();
    harness.get_by_label("Fired").click();
    harness.run_steps(3);
    harness.get_by_label("Clear resolved").click();
    harness.run_steps(3);
    harness.get_by_label("Clear").click();
    harness.run_steps(3);
    assert!(harness.state().alerting.events.iter().all(|e| e.resolved_at.is_none()));
}

#[test]
fn evaluating_off_pauses_every_rule() {
    let mut harness = harness();
    harness.state_mut().settings = Some("alerts".into());
    harness.run_steps(3);
    // the switch, before the row's title
    harness.get_all_by_label("Evaluate rules").next().unwrap().click();
    harness.run_steps(3);
    let app = harness.state();
    assert!(!app.prefs.evaluate);
    assert!(app.alerting.readings.values().all(|r| r.state == RuleState::Paused));
}
