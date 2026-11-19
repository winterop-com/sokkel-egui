//! One validation rule, as the web's `pages/ValidationRule.tsx` edits it: its sides and operator
//! edited and checked as typed, and run over the sites under a unit for a run of months, its
//! violations listed; saved, discarded or deleted.

use serde::Serialize;

use crate::backend::{
    Backend, Event, Importance, MissingValueStrategy, OPERATORS, Operator, STRATEGIES, Then, Toast, ValidationRule,
    Violation, run_rule, unit_name,
};
use crate::check::{CheckContext, Checked, check};
use crate::expression::Language;
use crate::metadata::{org_unit, path_of, period_label, periods, root, sites_under};
use crate::ui::{Verdict, counted, figure, verdict};

/// What the editor changes of a rule: everything but its uid and code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    pub name: String,
    pub instruction: String,
    pub importance: Importance,
    pub left: String,
    pub operator: Operator,
    pub right: String,
    pub left_strategy: MissingValueStrategy,
    pub right_strategy: MissingValueStrategy,
}

impl Draft {
    pub fn of(saved: &ValidationRule) -> Self {
        Draft {
            name: saved.name.clone(),
            instruction: saved.instruction.clone(),
            importance: saved.importance,
            left: saved.left.clone(),
            operator: saved.operator,
            right: saved.right.clone(),
            left_strategy: saved.left_strategy,
            right_strategy: saved.right_strategy,
        }
    }

    /// How many fields differ from the saved rule's.
    pub fn changed(&self, saved: &ValidationRule) -> usize {
        [
            self.name != saved.name,
            self.instruction != saved.instruction,
            self.importance != saved.importance,
            self.left != saved.left,
            self.operator != saved.operator,
            self.right != saved.right,
            self.left_strategy != saved.left_strategy,
            self.right_strategy != saved.right_strategy,
        ]
        .into_iter()
        .filter(|one| *one)
        .count()
    }
}

/// What the page says when the uid names no rule.
pub const EMPTY: &str = "No validation rule has this uid. It may have been deleted.";

/// A run of the rule: what it found, where and over which months, and the operator it compared with.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Run {
    pub violations: Vec<Violation>,
    pub tried: usize,
    pub skipped: usize,
    #[serde(rename = "where")]
    pub place: String,
    pub months: String,
    pub operator: Operator,
}

/// A violation as the run's table writes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ViolationRow {
    pub id: String,
    pub site: String,
    /// The units between the country and the site.
    pub path: String,
    pub month: String,
    pub left: String,
    pub sign: String,
    pub right: String,
}

/// A tile of the run: its label, value, note and whether it stands out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Stat {
    pub label: &'static str,
    pub value: usize,
    pub note: String,
    pub emphasis: bool,
}

/// The editor: the rule as saved and as drafted, where and over which months it is run, and its
/// last run.
#[derive(Clone, Debug)]
pub struct Editor {
    pub saved: ValidationRule,
    pub draft: Draft,
    pub busy: bool,
    pub deleting: bool,
    pub unit: String,
    pub from: String,
    pub to: String,
    pub run: Option<Run>,
}

/// The two sides checked and the state they leave the rule in.
pub struct Checks {
    pub left: Checked,
    pub right: Checked,
    pub state: Verdict,
}

impl Checks {
    pub fn runnable(&self) -> bool {
        self.left.node.is_some() && self.right.node.is_some() && self.state != Verdict::Invalid
    }
}

impl Editor {
    pub fn new(saved: &ValidationRule) -> Self {
        Editor {
            saved: saved.clone(),
            draft: Draft::of(saved),
            busy: false,
            deleting: false,
            unit: root().to_owned(),
            from: periods()[0].clone(),
            to: periods()[periods().len() - 1].clone(),
            run: None,
        }
    }

    /// The draft's sides checked against the version, with the indicators the server holds.
    pub fn checks(&self, version: &str, ids: &[String]) -> Checks {
        let context = CheckContext {
            version: version.to_owned(),
            indicators: ids.to_vec(),
            ..CheckContext::default()
        };
        let left = check(&self.draft.left, Language::Expression, &context);
        let right = check(&self.draft.right, Language::Expression, &context);
        let state = verdict(&[&left, &right]);
        Checks { left, right, state }
    }

    /// The months from `from` to `to`, either way round.
    pub fn months(&self) -> Vec<String> {
        let (low, high) = if self.from < self.to {
            (&self.from, &self.to)
        } else {
            (&self.to, &self.from)
        };
        periods()
            .iter()
            .filter(|one| *one >= low && *one <= high)
            .cloned()
            .collect()
    }

    /// Runs the draft over each site under the unit in each month, and says what it found; nothing
    /// when a side does not read.
    pub fn start(&mut self, checks: &Checks) -> Option<Toast> {
        let (Some(left), Some(right)) = (&checks.left.node, &checks.right.node) else {
            return None;
        };
        let sites: Vec<String> = sites_under(&self.unit)
            .into_iter()
            .map(|site| site.id.clone())
            .collect();
        let months = self.months();
        let result = run_rule(
            &self.saved.id,
            self.draft.operator,
            self.draft.left_strategy,
            self.draft.right_strategy,
            left,
            right,
            &sites,
            &months,
        );
        let toast = Toast::note(if result.violations.is_empty() {
            "The rule holds everywhere it was tried".to_owned()
        } else {
            format!("{} found", counted(result.violations.len(), "violation"))
        });
        let label = |at: Option<&String>| at.map_or_else(|| "Invalid Date".to_owned(), |one| period_label(one));
        self.run = Some(Run {
            violations: result.violations,
            tried: result.tried,
            skipped: result.skipped,
            place: org_unit(&self.unit).map_or_else(String::new, |one| one.name.clone()),
            months: format!("{} to {}", label(months.first()), label(months.last())),
            operator: self.draft.operator,
        });
        Some(toast)
    }

    /// What the status line says: the last run's violations of the pairs tried.
    pub fn foot(&self) -> String {
        match &self.run {
            None => "Not run yet".to_owned(),
            Some(run) => format!(
                "{} of {} tried",
                counted(run.violations.len(), "violation"),
                counted(run.tried, "pair")
            ),
        }
    }

    /// The run's three tiles.
    pub fn stats(&self) -> Vec<Stat> {
        let Some(run) = &self.run else {
            return Vec::new();
        };
        vec![
            Stat {
                label: "Violations",
                value: run.violations.len(),
                note: format!("{}, {}", run.place, run.months),
                emphasis: !run.violations.is_empty(),
            },
            Stat {
                label: "Tried",
                value: run.tried,
                note: "site and month pairs".to_owned(),
                emphasis: false,
            },
            Stat {
                label: "Skipped",
                value: run.skipped,
                note: "for missing values".to_owned(),
                emphasis: false,
            },
        ]
    }

    /// The run's violations as its table writes them.
    pub fn rows(&self) -> Vec<ViolationRow> {
        let Some(run) = &self.run else {
            return Vec::new();
        };
        run.violations
            .iter()
            .map(|one| {
                let path = path_of(&one.unit);
                let between = if path.len() > 2 {
                    &path[1..path.len() - 1]
                } else {
                    &[][..]
                };
                ViolationRow {
                    id: one.id.clone(),
                    site: unit_name(&one.unit),
                    path: between
                        .iter()
                        .map(|step| step.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" / "),
                    month: period_label(&one.period),
                    left: figure(one.left, ""),
                    sign: run.operator.sign().to_owned(),
                    right: figure(one.right, ""),
                }
            })
            .collect()
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
        backend.update_rule(
            &self.saved.id,
            move |row| {
                row.name = draft.name;
                row.instruction = draft.instruction;
                row.importance = draft.importance;
                row.left = draft.left;
                row.operator = draft.operator;
                row.right = draft.right;
                row.left_strategy = draft.left_strategy;
                row.right_strategy = draft.right_strategy;
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

    /// Deletes the rule and goes back to the list.
    pub fn remove(&self, backend: &mut Backend) {
        backend.remove_rule(
            &self.saved.id,
            vec![
                Then::Toast(Toast::note(format!("{} deleted", self.saved.name))),
                Then::Go("/validation-rules".to_owned()),
                Then::Refused("Not deleted".to_owned()),
            ],
        );
    }

    /// The question before the rule is deleted, and its detail.
    pub fn delete_question(&self) -> (String, &'static str) {
        (
            format!("Delete {}?", self.saved.name),
            "The rule goes from the server, and with it the violations it finds.",
        )
    }
}

/// The months offered for a run, by their labels.
pub fn period_options() -> Vec<(String, String)> {
    periods().iter().map(|one| (one.clone(), period_label(one))).collect()
}

/// The missing value strategies offered, by their labels.
pub fn strategy_options() -> Vec<(&'static str, &'static str)> {
    STRATEGIES.iter().map(|one| (one.key(), one.label())).collect()
}

/// The operators offered, each with its sign.
pub fn operator_options() -> Vec<(&'static str, String)> {
    OPERATORS
        .iter()
        .map(|one| (one.key(), format!("{} ({})", one.label(), one.sign())))
        .collect()
}
