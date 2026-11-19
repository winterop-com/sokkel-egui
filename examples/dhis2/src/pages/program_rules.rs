//! The program's rules, as the web's `pages/ProgramRules.tsx` keeps them: listed in the order they
//! run and dragged into another order to number their priorities again, the chosen one's
//! condition and actions edited, and the program's form beside them, reacting to every rule as it
//! is filled.

use serde::Serialize;

use crate::backend::{Backend, Event, Then, Toast};
use crate::check::{CheckContext, Checked, check};
use crate::expression::{Language, Value, text};
use crate::js::{base36, trim};
use crate::metadata::uid;
use crate::program::{
    ACTION_TYPES, ActionType, EVALUATED_VARIABLES, FieldType, FormValues, ProgramRule, RuleAction, RuleRun, fields,
    new_action, ordered, program, run_rules, variables,
};
use crate::ui::counted;

/// What the editor changes of a rule: everything but its uid.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub name: String,
    pub description: String,
    pub priority: f64,
    pub condition: String,
    pub actions: Vec<RuleAction>,
}

impl Draft {
    pub fn of(rule: &ProgramRule) -> Self {
        Draft {
            name: rule.name.clone(),
            description: rule.description.clone(),
            priority: rule.priority,
            condition: rule.condition.clone(),
            actions: rule.actions.clone(),
        }
    }

    /// The draft as a rule under `id`.
    pub fn rule(&self, id: &str) -> ProgramRule {
        ProgramRule {
            id: id.to_owned(),
            name: self.name.clone(),
            description: self.description.clone(),
            priority: self.priority,
            condition: self.condition.clone(),
            actions: self.actions.clone(),
        }
    }

    /// How many fields differ from the saved rule's.
    pub fn changed(&self, saved: &ProgramRule) -> usize {
        [
            self.name != saved.name,
            self.description != saved.description,
            self.priority != saved.priority,
            self.condition != saved.condition,
            self.actions != saved.actions,
        ]
        .into_iter()
        .filter(|one| *one)
        .count()
    }
}

/// What the page says when the program has no rules.
pub const EMPTY: &str = "The program has no rules. Add one with New.";

/// What a rule check reads: the program's rule variables and the `V{}` the form gives.
pub fn context(version: &str) -> CheckContext {
    CheckContext {
        version: version.to_owned(),
        variables: variables(),
        evaluated_variables: EVALUATED_VARIABLES.iter().map(|one| (*one).to_owned()).collect(),
        indicators: Vec::new(),
    }
}

/// The page: the rule being edited, when it is, and the editor's own state.
#[derive(Clone, Debug, Default)]
pub struct ProgramRules {
    /// The draft of a rule, by its uid; none until a field is changed.
    pub draft: Option<(String, Draft)>,
    /// The editor's: whether a save is on its way, and whether the delete question is open.
    pub busy: bool,
    pub deleting: bool,
    /// The form the rules run on.
    pub form: TryForm,
}

/// A rule of the list beside the editor: its uid, name and priority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub meta: String,
}

impl ProgramRules {
    pub fn new() -> Self {
        ProgramRules {
            form: TryForm::new(),
            ..ProgramRules::default()
        }
    }

    /// The rule shown: the one the route names, or the first to run.
    pub fn saved(&self, rows: &[ProgramRule], selected: Option<&str>) -> Option<ProgramRule> {
        let ordered = ordered(rows);
        ordered
            .iter()
            .find(|one| Some(one.id.as_str()) == selected)
            .or(ordered.first())
            .cloned()
    }

    /// The rule shown as it is being edited: its draft, or the rule as saved.
    pub fn editing(&self, saved: &ProgramRule) -> Draft {
        match &self.draft {
            Some((id, draft)) if *id == saved.id => draft.clone(),
            _ => Draft::of(saved),
        }
    }

    /// Every rule in the order they run, the one edited as drafted.
    pub fn rules(&self, rows: &[ProgramRule], selected: Option<&str>) -> Vec<ProgramRule> {
        let saved = self.saved(rows, selected);
        ordered(rows)
            .into_iter()
            .map(|one| match &saved {
                Some(saved) if saved.id == one.id => self.editing(saved).rule(&one.id),
                _ => one,
            })
            .collect()
    }

    /// The list beside the editor.
    pub fn nodes(rows: &[ProgramRule]) -> Vec<Node> {
        ordered(rows)
            .iter()
            .map(|one| Node {
                id: one.id.clone(),
                label: one.name.clone(),
                meta: crate::js::number_to_string(one.priority),
            })
            .collect()
    }

    /// Changes the draft of the rule shown.
    pub fn set(&mut self, saved: &ProgramRule, change: impl FnOnce(&mut Draft)) {
        let mut draft = self.editing(saved);
        change(&mut draft);
        self.draft = Some((saved.id.clone(), draft));
    }

    /// Changes one of the shown rule's actions.
    pub fn set_action(&mut self, saved: &ProgramRule, key: &str, change: impl FnOnce(&mut RuleAction)) {
        self.set(saved, |draft| {
            if let Some(one) = draft.actions.iter_mut().find(|one| one.key == key) {
                change(one);
            }
        });
    }

    /// Adds an action to the shown rule.
    pub fn add_action(&mut self, saved: &ProgramRule, backend: &mut Backend) {
        let made = new_action(ActionType::Showwarning, &mut backend.actions_made, backend.clock.now);
        self.set(saved, |draft| draft.actions.push(made));
    }

    /// Takes an action off the shown rule.
    pub fn remove_action(&mut self, saved: &ProgramRule, key: &str) {
        self.set(saved, |draft| draft.actions.retain(|one| one.key != key));
    }

    /// Moves one of the shown rule's actions.
    pub fn move_action(&mut self, saved: &ProgramRule, from: usize, to: usize) {
        self.set(saved, |draft| sokkel_egui::move_item(&mut draft.actions, from, to));
    }

    /// Moves a rule to another place in the run order, numbering every rule's priority from 1 in
    /// the new order.
    pub fn reorder(&mut self, rows: &[ProgramRule], backend: &mut Backend, id: &str, index: usize) {
        let ordered = ordered(rows);
        let Some(from) = ordered.iter().position(|one| one.id == id) else {
            return;
        };
        let mut next = ordered.clone();
        sokkel_egui::move_item(&mut next, from, index);
        let place = |id: &str| next.iter().position(|one| one.id == id).map(|at| (at + 1) as f64);
        if let Some((drafted, draft)) = &mut self.draft
            && let Some(priority) = place(drafted)
        {
            draft.priority = priority;
        }
        let changed: Vec<(String, f64)> = next
            .iter()
            .enumerate()
            .filter(|(at, one)| one.priority != (at + 1) as f64)
            .map(|(at, one)| (one.id.clone(), (at + 1) as f64))
            .collect();
        for (id, priority) in changed {
            backend.update_program_rule(
                &id,
                move |row| row.priority = priority,
                vec![Then::Refused("Not moved".to_owned())],
            );
        }
    }

    /// A new rule, numbered after those there are and run after them, made and opened.
    pub fn create(&mut self, rows: &[ProgramRule], backend: &mut Backend) {
        let ordered = ordered(rows);
        let made = (ordered.len() + 1).to_string();
        let action = new_action(ActionType::Showwarning, &mut backend.actions_made, backend.clock.now);
        let id = uid(&format!("prr:new:{made}:{}", base36(backend.now())));
        let row = ProgramRule {
            id: id.clone(),
            name: format!("New rule {made}"),
            description: String::new(),
            priority: ordered.last().map_or(0.0, |one| one.priority) + 1.0,
            condition: "d2:hasValue(#{water source})".to_owned(),
            actions: vec![action],
        };
        backend.create_program_rule(
            row,
            vec![
                Then::Go(format!("/program-rules/{id}")),
                Then::Refused("Not created".to_owned()),
            ],
        );
    }

    /// The shown rule's condition checked.
    pub fn condition(&self, saved: &ProgramRule, version: &str) -> Checked {
        check(&self.editing(saved).condition, Language::Rule, &context(version))
    }

    /// How many fields the draft changed.
    pub fn changed(&self, saved: &ProgramRule) -> usize {
        self.editing(saved).changed(saved)
    }

    /// Lets the draft go.
    pub fn discard(&mut self) {
        self.draft = None;
    }

    /// Keeps the shown rule's draft.
    pub fn save(&mut self, saved: &ProgramRule, backend: &mut Backend) {
        self.busy = true;
        let draft = self.editing(saved);
        let name = draft.name.clone();
        backend.update_program_rule(
            &saved.id,
            move |row| {
                row.name = draft.name;
                row.description = draft.description;
                row.priority = draft.priority;
                row.condition = draft.condition;
                row.actions = draft.actions;
            },
            vec![
                Then::Event(Event::Settled),
                Then::Event(Event::Discard),
                Then::Toast(Toast::success("Saved", name)),
                Then::Refused("Not saved".to_owned()),
            ],
        );
    }

    /// Hears an answer: a save settled is no longer busy, and one kept lets its draft go.
    pub fn hear(&mut self, event: &Event) {
        match event {
            Event::Settled => self.busy = false,
            Event::Discard => self.discard(),
        }
    }

    /// Deletes the shown rule and goes back to the program's first.
    pub fn remove(&self, saved: &ProgramRule, backend: &mut Backend) {
        backend.remove_program_rule(
            &saved.id,
            vec![
                Then::Toast(Toast::note(format!("{} deleted", saved.name))),
                Then::Go("/program-rules".to_owned()),
                Then::Refused("Not deleted".to_owned()),
            ],
        );
    }

    /// The question before a rule is deleted, and its detail.
    pub fn delete_question(saved: &ProgramRule) -> (String, &'static str) {
        (
            format!("Delete {}?", saved.name),
            "The rule goes from the program, and the form stops following it.",
        )
    }
}

/// An action's fields as the editor shows them.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionFields {
    /// Whether it asks for a field, a message and an expression.
    pub field: bool,
    pub content: bool,
    pub data: bool,
    /// The expression field's label.
    pub data_label: &'static str,
    /// The expression checked, nothing for an empty one an action may leave out.
    pub checked: Checked,
}

/// An action's fields as the editor shows them, its expression checked.
pub fn action_fields(action: &RuleAction, version: &str) -> ActionFields {
    let shape = action.kind.shape();
    ActionFields {
        field: shape.field,
        content: shape.content,
        data: shape.data,
        data_label: if action.kind == ActionType::Assign {
            "Value"
        } else {
            "Written after the message"
        },
        checked: if trim(&action.data).is_empty() && action.kind != ActionType::Assign {
            Checked {
                node: None,
                findings: Vec::new(),
            }
        } else {
            check(&action.data, Language::Rule, &context(version))
        },
    }
}

/// The action types offered, by their labels.
pub fn action_options() -> Vec<(&'static str, &'static str)> {
    ACTION_TYPES.iter().map(|one| (one.key(), one.shape().label)).collect()
}

/// The fields offered for an action, by their labels.
pub fn field_options() -> Vec<(&'static str, &'static str)> {
    fields().iter().map(|one| (one.variable, one.label)).collect()
}

/// The inspection's date the form starts with.
pub const EVENT_DATE: &str = "2026-11-02";

/// The values the form starts with.
pub fn start() -> FormValues {
    vec![
        ("water source".to_owned(), Value::Text("river".to_owned())),
        ("treated".to_owned(), Value::Bool(false)),
        ("samples taken".to_owned(), Value::Number(3.0)),
        ("samples failed".to_owned(), Value::Number(1.0)),
    ]
}

/// The program's form, filled in by hand, every rule run on it as it changes.
#[derive(Clone, Debug, Default)]
pub struct TryForm {
    pub values: FormValues,
    pub event_date: String,
}

/// A field of the form as it is shown: its widget's id, kind and value, and the warnings and errors
/// the rules show on it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FormField {
    pub variable: &'static str,
    pub id: String,
    pub label: &'static str,
    /// `computed` for an assigned field, else its type.
    pub widget: &'static str,
    pub value: serde_json::Value,
    pub issues: Vec<crate::ui::Issue>,
}

/// An effect as the form's table writes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectRow {
    pub id: String,
    pub rule: String,
    pub action: &'static str,
    pub field: Option<String>,
    pub text: Option<String>,
}

/// A rule that cannot run, as the form's table writes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BrokenRow {
    pub id: String,
    pub rule: String,
    pub why: String,
}

/// A field's widget id: its variable with every run of non-word characters a dash.
pub fn field_id(variable: &str) -> String {
    let mut out = String::from("try-");
    let mut gap = false;
    for c in variable.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
            gap = false;
        } else if !gap {
            out.push('-');
            gap = true;
        }
    }
    out
}

impl TryForm {
    pub fn new() -> Self {
        TryForm {
            values: start(),
            event_date: EVENT_DATE.to_owned(),
        }
    }

    /// The rules run on the form.
    pub fn run(&self, rules: &[ProgramRule], version: &str) -> RuleRun {
        run_rules(rules, &self.values, &self.event_date, version)
    }

    /// Sets a field's value.
    pub fn set(&mut self, variable: &str, value: Value) {
        match self.values.iter_mut().find(|(one, _)| one == variable) {
            Some(found) => found.1 = value,
            None => self.values.push((variable.to_owned(), value)),
        }
    }

    /// Sets the inspection's date; none puts the first one back.
    pub fn set_event_date(&mut self, value: Option<&str>) {
        self.event_date = value.unwrap_or(EVENT_DATE).to_owned();
    }

    /// Empties the form.
    pub fn clear(&mut self) {
        self.values.clear();
    }

    fn value(&self, variable: &str) -> Value {
        self.values
            .iter()
            .find(|(one, _)| one == variable)
            .map_or(Value::Null, |(_, value)| value.clone())
    }

    /// The warnings and errors the rules show on a field.
    pub fn issues(run: &RuleRun, variable: &str) -> Vec<crate::ui::Issue> {
        run.effects
            .iter()
            .filter(|effect| {
                effect.field.as_deref() == Some(variable)
                    && matches!(effect.kind, ActionType::Showwarning | ActionType::Showerror)
            })
            .map(|effect| crate::ui::Issue {
                tone: if effect.kind == ActionType::Showerror {
                    "critical"
                } else {
                    "warning"
                },
                text: effect.text.clone(),
            })
            .collect()
    }

    /// The fields shown, those no rule hid, each as its widget shows it.
    pub fn shown(&self, run: &RuleRun) -> Vec<FormField> {
        fields()
            .iter()
            .filter(|field| !run.hidden.contains(field.variable))
            .map(|field| {
                let value = self.value(field.variable);
                let (widget, shown) = if field.assigned {
                    let assigned = run
                        .assigned
                        .iter()
                        .find(|(one, _)| one == field.variable)
                        .map(|(_, value)| value.clone())
                        .unwrap_or(Value::Null);
                    (
                        "computed",
                        if assigned == Value::Null {
                            serde_json::Value::Null
                        } else {
                            serde_json::Value::String(text(&assigned))
                        },
                    )
                } else {
                    match field.kind {
                        FieldType::Option => (
                            "option",
                            serde_json::Value::String(match &value {
                                Value::Text(one) => one.clone(),
                                _ => String::new(),
                            }),
                        ),
                        FieldType::Boolean => (
                            "boolean",
                            match value {
                                Value::Bool(flag) => serde_json::Value::Bool(flag),
                                _ => serde_json::Value::Null,
                            },
                        ),
                        FieldType::Number => (
                            "number",
                            match value {
                                Value::Number(number) => serde_json::json!(number),
                                _ => serde_json::Value::Null,
                            },
                        ),
                        FieldType::Date => (
                            "date",
                            match value {
                                Value::Text(one) => serde_json::Value::String(one),
                                _ => serde_json::Value::Null,
                            },
                        ),
                        FieldType::Text => (
                            "text",
                            serde_json::Value::String(match value {
                                Value::Text(one) => one,
                                _ => String::new(),
                            }),
                        ),
                    }
                };
                FormField {
                    variable: field.variable,
                    id: field_id(field.variable),
                    label: field.label,
                    widget,
                    value: shown,
                    issues: if field.assigned {
                        Vec::new()
                    } else {
                        Self::issues(run, field.variable)
                    },
                }
            })
            .collect()
    }

    /// What the status line says: how many rules act on the form, and how many cannot run.
    pub fn foot(run: &RuleRun) -> String {
        let mut fired: Vec<&str> = Vec::new();
        for effect in &run.effects {
            if !fired.contains(&effect.rule.as_str()) {
                fired.push(&effect.rule);
            }
        }
        let broken = if run.broken.is_empty() {
            String::new()
        } else {
            format!(", {} that cannot run", counted(run.broken.len(), "rule"))
        };
        format!("{} acting on the form{broken}", counted(fired.len(), "rule"))
    }

    /// The effects as the table writes them, each rule by its name among `rules`.
    pub fn effects(run: &RuleRun, rules: &[ProgramRule]) -> Vec<EffectRow> {
        let name = |id: &str| {
            rules
                .iter()
                .find(|one| one.id == id)
                .map_or_else(|| id.to_owned(), |one| one.name.clone())
        };
        run.effects
            .iter()
            .enumerate()
            .map(|(index, effect)| EffectRow {
                id: format!("{}-{index}", effect.rule),
                rule: name(&effect.rule),
                action: effect.kind.shape().label,
                field: effect.field.as_ref().and_then(|field| {
                    fields()
                        .iter()
                        .find(|one| one.variable == field)
                        .map(|one| one.label.to_owned())
                }),
                text: (effect.kind != ActionType::Hidefield).then(|| effect.text.clone()),
            })
            .collect()
    }

    /// The rules that cannot run, as the table writes them.
    pub fn broken(run: &RuleRun, rules: &[ProgramRule]) -> Vec<BrokenRow> {
        let name = |id: &str| {
            rules
                .iter()
                .find(|one| one.id == id)
                .map_or_else(|| id.to_owned(), |one| one.name.clone())
        };
        run.broken
            .iter()
            .enumerate()
            .map(|(index, one)| BrokenRow {
                id: format!("{}-{index}", one.rule),
                rule: name(&one.rule),
                why: one.message.clone(),
            })
            .collect()
    }
}

/// The form section's words: its meta and description.
pub fn form_words() -> (&'static str, String) {
    (
        program().stage,
        format!("{}, every rule run on it as it is filled.", program().name),
    )
}
