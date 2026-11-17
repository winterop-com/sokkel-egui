//! What the app keeps between runs: the dashboards, the rules and what they fired, the preferences
//! and the place come back as they were left, as the web's backend keeps them in the browser.

use std::collections::HashMap;

use metrics::MetricsApp;
use metrics::alerting::Condition;
use metrics::kept::{KEY, Kept};
use sokkel_egui::{Appearance, PanelSpan, Threshold};

/// eframe's storage held in memory, as the window's file holds it.
#[derive(Default)]
struct Memory(HashMap<String, String>);

impl eframe::Storage for Memory {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.into(), value);
    }
    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }
    fn flush(&mut self) {}
}

/// `app` written into eframe's storage and read back into a fresh app, as a closed window and a
/// new run do.
fn reopened(app: &MetricsApp) -> MetricsApp {
    let mut storage = Memory::default();
    eframe::set_value(&mut storage, KEY, &app.kept());
    let kept: Kept = eframe::get_value(&storage, KEY).unwrap();
    let mut again = MetricsApp::on(Appearance::default());
    again.restore(kept);
    again
}

#[test]
fn what_was_changed_comes_back() {
    let mut app = MetricsApp::on(Appearance::default());
    let panel = &mut app.dashboards[0].panels[0];
    panel.title = "Requests by host".into();
    panel.color = "chart-4";
    panel.span = PanelSpan::Full;
    panel.thresholds = vec![Threshold::base("good"), Threshold::from(80.0, "critical")];
    app.dashboards.remove(2);
    app.alerting.rules[0].threshold = 250.0;
    app.alerting.rules[0].condition = Condition::Below;
    app.prefs.range = "1h";
    app.prefs.refresh = 30;
    app.place = "alerts".into();
    app.at += 600.0;

    let again = reopened(&app);
    // the events are read again at the clock the app left, so some may have fired or resolved
    assert_eq!(again.dashboards, app.dashboards);
    assert_eq!(again.alerting.rules, app.alerting.rules);
    assert_eq!(again.prefs, app.prefs);
    assert!(again.alerting.events.len() >= app.alerting.events.len());
    assert_eq!(again.dashboards.len(), 2);
    assert_eq!(again.place, "alerts");
    assert_eq!(again.range, metrics::query::Range::Preset("1h"));
    assert_eq!(again.refresh, 30);
    assert_eq!(again.at, app.at);
}

#[test]
fn ids_count_on_from_the_last_run() {
    let mut app = MetricsApp::on(Appearance::default());
    let first = app.alerting.next_id("rule");
    let mut again = reopened(&app);
    assert_ne!(again.alerting.next_id("rule"), first);
}

#[test]
fn a_name_the_app_no_longer_knows_falls_back() {
    let app = MetricsApp::on(Appearance::default());
    let mut storage = Memory::default();
    eframe::set_value(&mut storage, KEY, &app.kept());
    let text = storage.0[KEY]
        .replacen("\"chart-2\"", "\"chart-99\"", 1)
        .replace("\"15m\"", "\"2w\"");
    assert!(text.contains("chart-99") && text.contains("2w"));
    storage.0.insert(KEY.into(), text);
    let kept: Kept = eframe::get_value(&storage, KEY).unwrap();
    let panel = kept
        .dashboards
        .iter()
        .flat_map(|b| &b.panels)
        .find(|p| p.color == "chart-2");
    assert!(panel.is_some());
    assert_eq!(kept.prefs.range, "15m");
}

#[test]
fn a_place_since_deleted_opens_the_first_dashboard() {
    let mut app = MetricsApp::on(Appearance::default());
    app.place = "gone".into();
    assert_eq!(reopened(&app).place, app.dashboards[0].id);
}
