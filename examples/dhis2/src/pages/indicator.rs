//! One indicator, as the web's `pages/Indicator.tsx` edits it: its name, type and two expressions
//! edited and checked as typed, and its value worked out for a unit and a month, over the year and
//! for the units under it; saved, discarded or deleted.

use serde::Serialize;

use crate::backend::{
    Backend, Event, INDICATOR_TYPES, Indicator, IndicatorType, IndicatorValue, Then, Toast, indicator_value,
};
use crate::check::{CheckContext, Checked, check};
use crate::expression::Language;
use crate::js::number_to_string;
use crate::metadata::{children_of, latest_period, level_name, org_unit, period_label, periods, root};
use crate::ui::{Verdict, figure, verdict};

/// What the editor changes of an indicator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    pub name: String,
    pub description: String,
    pub kind: IndicatorType,
    pub numerator: String,
    pub denominator: String,
}

impl Draft {
    pub fn of(saved: &Indicator) -> Self {
        Draft {
            name: saved.name.clone(),
            description: saved.description.clone(),
            kind: saved.kind,
            numerator: saved.numerator.clone(),
            denominator: saved.denominator.clone(),
        }
    }

    /// How many fields differ from the saved indicator's.
    pub fn changed(&self, saved: &Indicator) -> usize {
        [
            self.name != saved.name,
            self.description != saved.description,
            self.kind != saved.kind,
            self.numerator != saved.numerator,
            self.denominator != saved.denominator,
        ]
        .into_iter()
        .filter(|one| *one)
        .count()
    }
}

/// What the page says when the uid names no indicator.
pub const EMPTY: &str = "No indicator has this uid. It may have been deleted.";

/// A unit under the one tried, with the indicator's parts and value there.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Below {
    pub id: String,
    pub name: String,
    pub numerator: Option<f64>,
    pub denominator: Option<f64>,
    pub value: Option<f64>,
}

/// A tile of the try-it section: its label, value, note and whether it stands out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Stat {
    pub label: String,
    pub value: String,
    pub note: Option<String>,
    pub emphasis: bool,
}

/// The editor: the indicator as saved and as drafted, and where and when it is tried.
#[derive(Clone, Debug)]
pub struct Editor {
    pub saved: Indicator,
    pub draft: Draft,
    pub busy: bool,
    pub deleting: bool,
    pub unit: String,
    pub period: String,
}

/// The two expressions checked, the state they leave the indicator in and whether it runs.
pub struct Checks {
    pub numerator: Checked,
    pub denominator: Checked,
    pub state: Verdict,
}

impl Checks {
    pub fn runnable(&self) -> bool {
        self.numerator.node.is_some() && self.denominator.node.is_some() && self.state != Verdict::Invalid
    }
}

impl Editor {
    pub fn new(saved: &Indicator) -> Self {
        Editor {
            saved: saved.clone(),
            draft: Draft::of(saved),
            busy: false,
            deleting: false,
            unit: root().to_owned(),
            period: latest_period().to_owned(),
        }
    }

    /// The draft's expressions checked against the version, with the indicators the server holds.
    pub fn checks(&self, version: &str, ids: &[String]) -> Checks {
        let context = CheckContext {
            version: version.to_owned(),
            indicators: ids.to_vec(),
            ..CheckContext::default()
        };
        let numerator = check(&self.draft.numerator, Language::Expression, &context);
        let denominator = check(&self.draft.denominator, Language::Expression, &context);
        let state = verdict(&[&numerator, &denominator]);
        Checks {
            numerator,
            denominator,
            state,
        }
    }

    fn value_at(&self, checks: &Checks, unit: &str, period: &str) -> Option<IndicatorValue> {
        if !checks.runnable() {
            return None;
        }
        let (Some(top), Some(bottom)) = (&checks.numerator.node, &checks.denominator.node) else {
            return None;
        };
        Some(indicator_value(top, bottom, self.draft.kind, unit, period))
    }

    /// The indicator's parts and value for the unit and month tried; none when it cannot run.
    pub fn now(&self, checks: &Checks) -> Option<IndicatorValue> {
        self.value_at(checks, &self.unit, &self.period)
    }

    /// Its value for the unit in each month the server holds, by the month's label.
    pub fn year(&self, checks: &Checks) -> Vec<(String, Option<f64>)> {
        periods()
            .iter()
            .map(|one| {
                (
                    period_label(one),
                    self.value_at(checks, &self.unit, one).and_then(|got| got.value),
                )
            })
            .collect()
    }

    /// Its parts and value for each unit under the one tried, in the month tried.
    pub fn below(&self, checks: &Checks) -> Vec<Below> {
        children_of(&self.unit)
            .into_iter()
            .map(|child| {
                let got = self
                    .value_at(checks, &child.id, &self.period)
                    .unwrap_or(IndicatorValue {
                        numerator: None,
                        denominator: None,
                        value: None,
                    });
                Below {
                    id: child.id.clone(),
                    name: child.name.clone(),
                    numerator: got.numerator,
                    denominator: got.denominator,
                    value: got.value,
                }
            })
            .collect()
    }

    /// The first column's header of the units under the one tried: their level.
    pub fn below_header(&self) -> &'static str {
        match org_unit(&self.unit) {
            Some(here) if here.level < 4 => level_name(here.level + 1),
            _ => "Unit",
        }
    }

    /// The unit the draft's type writes after a value.
    pub fn unit_sign(&self) -> &'static str {
        self.draft.kind.unit()
    }

    /// What the status line says: the value for the unit and month, or that it does not run.
    pub fn foot(&self, checks: &Checks) -> String {
        if checks.state == Verdict::Invalid {
            return "The expressions do not run as written".to_owned();
        }
        let here = org_unit(&self.unit).map_or("", |one| one.name.as_str());
        format!(
            "{here}, {}: {}",
            period_label(&self.period),
            figure(self.now(checks).and_then(|got| got.value), self.unit_sign())
        )
    }

    /// The three tiles: the numerator, the denominator and the value with what it is times.
    pub fn stats(&self, checks: &Checks) -> Vec<Stat> {
        let now = self.now(checks);
        let note = if checks.state == Verdict::Invalid {
            "Cannot run as written".to_owned()
        } else if now.is_some_and(|got| got.value.is_none()) {
            "Missing data or a zero denominator".to_owned()
        } else {
            format!("\u{d7} {}", number_to_string(self.draft.kind.factor()))
        };
        vec![
            Stat {
                label: "Numerator".to_owned(),
                value: figure(now.and_then(|got| got.numerator), ""),
                note: None,
                emphasis: false,
            },
            Stat {
                label: "Denominator".to_owned(),
                value: figure(now.and_then(|got| got.denominator), ""),
                note: None,
                emphasis: false,
            },
            Stat {
                label: self.draft.kind.label().to_owned(),
                value: figure(now.and_then(|got| got.value), self.unit_sign()),
                note: Some(note),
                emphasis: checks.state == Verdict::Invalid,
            },
        ]
    }

    /// The chart's label.
    pub fn chart_label(&self) -> String {
        format!(
            "{} by month for {}",
            self.draft.name,
            org_unit(&self.unit).map_or("", |one| one.name.as_str())
        )
    }

    /// How many fields the draft changed.
    pub fn changed(&self) -> usize {
        self.draft.changed(&self.saved)
    }

    /// Puts the draft back as saved.
    pub fn discard(&mut self) {
        self.draft = Draft::of(&self.saved);
    }

    /// Keeps the draft.
    pub fn save(&mut self, backend: &mut Backend) {
        self.busy = true;
        let draft = self.draft.clone();
        let name = draft.name.clone();
        backend.update_indicator(
            &self.saved.id,
            move |row| {
                row.name = draft.name;
                row.description = draft.description;
                row.kind = draft.kind;
                row.numerator = draft.numerator;
                row.denominator = draft.denominator;
            },
            vec![
                Then::Event(Event::Settled),
                Then::Toast(Toast::success("Saved", name)),
                Then::Refused("Not saved".to_owned()),
            ],
        );
    }

    /// Hears an answer: a save settled is no longer busy.
    pub fn hear(&mut self, event: &Event) {
        if *event == Event::Settled {
            self.busy = false;
        }
    }

    /// Deletes the indicator and goes back to the list.
    pub fn remove(&self, backend: &mut Backend) {
        backend.remove_indicator(
            &self.saved.id,
            vec![
                Then::Toast(Toast::note(format!("{} deleted", self.saved.name))),
                Then::Go("/indicators".to_owned()),
                Then::Refused("Not deleted".to_owned()),
            ],
        );
    }

    /// The question before the indicator is deleted, and its detail.
    pub fn delete_question(&self) -> (String, &'static str) {
        (
            format!("Delete {}?", self.saved.name),
            "The indicator goes from the server. Nothing else reads it here.",
        )
    }
}

/// The type field's options: each type with the factor it multiplies by.
pub fn type_options() -> Vec<(&'static str, String)> {
    INDICATOR_TYPES
        .iter()
        .map(|one| {
            (
                one.key(),
                format!("{} (\u{d7}{})", one.label(), number_to_string(one.factor())),
            )
        })
        .collect()
}
