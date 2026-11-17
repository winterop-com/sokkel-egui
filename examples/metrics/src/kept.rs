//! What the app keeps between runs, as the web's backend keeps its collections in the browser's
//! storage: the dashboards, the rules and what they fired, the preferences, the look, the place
//! shown and the source's clock. The fields that name a token or a preset are written as their
//! names and read back as the app's own.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sokkel_egui::{Appearance, PanelSpan, Threshold};

use crate::alerting::{AlertEvent, AlertRule};
use crate::app::Preferences;
use crate::model::Dashboard;

/// The key the app is kept under in eframe's storage.
pub const KEY: &str = "metrics";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Kept {
    pub appearance: Appearance,
    pub dashboards: Vec<Dashboard>,
    pub rules: Vec<AlertRule>,
    pub events: Vec<AlertEvent>,
    pub prefs: Preferences,
    pub place: String,
    /// The newest scrape, so the source runs on from where it stopped.
    pub at: f64,
    /// How many panels and copies, and rules and events, have been made, for their ids.
    pub made: u32,
    pub alerts_made: u32,
}

/// A token's or a preset's name, one of the app's own. An alias, so serde reads it through the
/// field's own `with` and does not ask to borrow it from the input.
pub type Name = &'static str;

/// `name` if it is one of `known`, else the first of them.
fn known_name(name: &str, known: &[&'static str]) -> &'static str {
    known.iter().find(|k| **k == name).copied().unwrap_or(known[0])
}

/// The editor's colors, the one a new panel takes first.
fn colors() -> Vec<&'static str> {
    let mut colors: Vec<&'static str> = crate::editor::COLORS.iter().map(|(c, _)| *c).collect();
    colors.swap(0, 1);
    colors
}

/// A panel's color token, read back as one of the editor's colors.
pub mod color {
    use super::*;

    pub fn serialize<S: Serializer>(color: &&'static str, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(color)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<&'static str, D::Error> {
        Ok(known_name(&String::deserialize(deserializer)?, &colors()))
    }
}

/// A range preset's id, read back as one of the presets.
pub mod preset {
    use super::*;

    pub fn serialize<S: Serializer>(id: &&'static str, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(id)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<&'static str, D::Error> {
        let mut ids = vec![crate::app::Preferences::default().range];
        ids.extend(crate::query::PRESETS.iter().map(|p| p.id));
        Ok(known_name(&String::deserialize(deserializer)?, &ids))
    }
}

/// A panel's thresholds, each a value and a color token.
pub mod thresholds {
    use super::*;

    pub fn serialize<S: Serializer>(steps: &[Threshold], serializer: S) -> Result<S::Ok, S::Error> {
        let steps: Vec<(Option<f64>, &str)> = steps.iter().map(|t| (t.value, t.color)).collect();
        steps.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Threshold>, D::Error> {
        let steps = Vec::<(Option<f64>, String)>::deserialize(deserializer)?;
        Ok(steps
            .into_iter()
            .map(|(value, color)| Threshold {
                value,
                color: known_name(&color, &colors()),
            })
            .collect())
    }
}

/// A panel's span, as the library names it.
#[derive(Serialize, Deserialize)]
#[serde(remote = "PanelSpan")]
pub enum SpanDef {
    Sixth,
    Quarter,
    Third,
    Half,
    TwoThirds,
    Full,
}

impl crate::app::MetricsApp {
    /// What the app keeps of itself.
    pub fn kept(&self) -> Kept {
        Kept {
            appearance: self.appearance.clone(),
            dashboards: self.dashboards.clone(),
            rules: self.alerting.rules.clone(),
            events: self.alerting.events.clone(),
            prefs: self.prefs.clone(),
            place: self.place.clone(),
            at: self.at,
            made: self.made,
            alerts_made: self.alerting.made(),
        }
    }

    /// The app as an earlier run left it: its dashboards, rules, preferences and place, the
    /// source's clock run on from where it stopped, and the range and refresh the preferences name.
    pub fn restore(&mut self, kept: Kept) {
        self.appearance = kept.appearance;
        self.dashboards = kept.dashboards;
        self.prefs = kept.prefs;
        self.at = kept.at;
        self.view_at = kept.at;
        self.made = kept.made;
        self.alerting = crate::alerting::Alerting::restored(
            kept.rules,
            kept.events,
            kept.alerts_made,
            self.at,
            self.prefs.evaluate,
            self.prefs.keep,
        );
        self.range = crate::query::Range::Preset(self.prefs.range);
        self.refresh = self.prefs.refresh;
        let known = [crate::app::DASHBOARDS, crate::app::ALERTS].contains(&kept.place.as_str())
            || self.dashboards.iter().any(|b| b.id == kept.place);
        self.place = if known {
            kept.place
        } else {
            self.dashboards
                .first()
                .map_or_else(|| crate::app::DASHBOARDS.into(), |b| b.id.clone())
        };
    }
}
