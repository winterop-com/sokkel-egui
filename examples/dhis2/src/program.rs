//! The one program on the simulated server, as the web's `data/program.ts` keeps it: an event
//! program for inspecting water points, its form's fields with the rule variables that read them,
//! and the engine that runs its program rules on the form as it is filled: hiding fields,
//! assigning values, and showing warnings, errors and text.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::check::{CheckContext, check};
use crate::expression::{ItemKind, Language, Value, evaluate, text};
use crate::js::{base36, locale_compare, number_to_string, round, trim};
use crate::metadata::{TODAY, constant, constants, uid};

pub struct Program {
    pub id: String,
    pub name: &'static str,
    pub stage: &'static str,
}

pub fn program() -> &'static Program {
    static PROGRAM: OnceLock<Program> = OnceLock::new();
    PROGRAM.get_or_init(|| Program {
        id: uid("pr:WATER_POINT"),
        name: "Water point inspection",
        stage: "Inspection",
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Option,
    Boolean,
    Number,
    Text,
    Date,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldOption {
    pub value: &'static str,
    pub label: &'static str,
}

/// A field of the program's form.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProgramField {
    /// The program rule variable that reads the field, as a rule writes it in `#{}`.
    pub variable: &'static str,
    pub label: &'static str,
    #[serde(rename = "type")]
    pub kind: FieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<FieldOption>>,
    /// Whether the field takes what a rule assigns rather than what is typed.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub assigned: bool,
}

const fn field(variable: &'static str, label: &'static str, kind: FieldType) -> ProgramField {
    ProgramField {
        variable,
        label,
        kind,
        options: None,
        assigned: false,
    }
}

pub fn fields() -> &'static [ProgramField] {
    static FIELDS: OnceLock<Vec<ProgramField>> = OnceLock::new();
    FIELDS.get_or_init(|| {
        let option = |value, label| FieldOption { value, label };
        vec![
            ProgramField {
                options: Some(vec![
                    option("well", "Well"),
                    option("tap", "Tap"),
                    option("river", "River"),
                    option("rain tank", "Rain tank"),
                ]),
                ..field("water source", "Water source", FieldType::Option)
            },
            field("treated", "Water treated", FieldType::Boolean),
            field("reason not treated", "Reason not treated", FieldType::Text),
            field("samples taken", "Samples taken", FieldType::Number),
            field("samples failed", "Samples failed", FieldType::Number),
            ProgramField {
                assigned: true,
                ..field("risk score", "Risk score", FieldType::Number)
            },
            field("follow-up date", "Follow-up date", FieldType::Date),
        ]
    })
}

/// The rule variables, one for each field.
pub fn variables() -> Vec<String> {
    fields().iter().map(|one| one.variable.to_owned()).collect()
}

/// The `V{}` variables the form gives a rule: the inspection's date and today.
pub const EVALUATED_VARIABLES: [&str; 2] = ["event_date", "current_date"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ActionType {
    Hidefield,
    Showwarning,
    Showerror,
    Assign,
    Displaytext,
}

/// What an action shows and asks for: its label, and whether it names a field, takes data and
/// takes a message.
pub struct ActionShape {
    pub label: &'static str,
    pub field: bool,
    pub data: bool,
    pub content: bool,
}

pub const ACTION_TYPES: [ActionType; 5] = [
    ActionType::Hidefield,
    ActionType::Showwarning,
    ActionType::Showerror,
    ActionType::Assign,
    ActionType::Displaytext,
];

impl ActionType {
    pub fn key(self) -> &'static str {
        match self {
            ActionType::Hidefield => "HIDEFIELD",
            ActionType::Showwarning => "SHOWWARNING",
            ActionType::Showerror => "SHOWERROR",
            ActionType::Assign => "ASSIGN",
            ActionType::Displaytext => "DISPLAYTEXT",
        }
    }

    pub fn shape(self) -> ActionShape {
        let (label, field, data, content) = match self {
            ActionType::Hidefield => ("Hide field", true, false, false),
            ActionType::Showwarning => ("Show warning", true, true, true),
            ActionType::Showerror => ("Show error", true, true, true),
            ActionType::Assign => ("Assign value", true, true, false),
            ActionType::Displaytext => ("Display text", false, true, true),
        };
        ActionShape {
            label,
            field,
            data,
            content,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleAction {
    pub key: String,
    #[serde(rename = "type")]
    pub kind: ActionType,
    /// The field the action is about, by its variable.
    pub field: String,
    pub content: String,
    /// An expression whose value is assigned, or written after the content.
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgramRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub priority: f64,
    pub condition: String,
    pub actions: Vec<RuleAction>,
}

/// A new action, under a key made from how many were made before and the time, as the web's
/// `newAction` makes it.
pub fn new_action(kind: ActionType, made: &mut u64, now: i64) -> RuleAction {
    *made += 1;
    RuleAction {
        key: format!("action-{made}-{}", base36(now)),
        kind,
        field: fields()[0].variable.to_owned(),
        content: String::new(),
        data: String::new(),
    }
}

fn action(kind: ActionType, field: &str, content: &str, data: &str) -> RuleAction {
    RuleAction {
        key: uid(&format!("act:{}:{field}:{content}:{data}", kind.key())),
        kind,
        field: field.to_owned(),
        content: content.to_owned(),
        data: data.to_owned(),
    }
}

fn rule(
    code: &str,
    name: &str,
    description: &str,
    priority: f64,
    condition: &str,
    actions: Vec<RuleAction>,
) -> ProgramRule {
    ProgramRule {
        id: uid(&format!("prr:{code}")),
        name: name.to_owned(),
        description: description.to_owned(),
        priority,
        condition: condition.to_owned(),
        actions,
    }
}

/// The program's rules the demo starts with.
pub fn rule_seed() -> Vec<ProgramRule> {
    let samples = &constants()[1].id;
    vec![
        rule(
            "REASON",
            "Reason only when untreated",
            "Asks why the water was not treated, once it is known that it was not.",
            1.0,
            "!d2:hasValue(#{treated}) || #{treated}",
            vec![action(ActionType::Hidefield, "reason not treated", "", "")],
        ),
        rule(
            "RISK",
            "Risk score",
            "Scores the water point from its source, its treatment and the samples that failed.",
            2.0,
            "true",
            vec![action(
                ActionType::Assign,
                "risk score",
                "",
                "if(#{water source} == 'river', 3, if(#{water source} == 'well', 2, 1))\n  + if(d2:hasValue(#{treated}) && !#{treated}, 2, 0)\n  + d2:zing(#{samples failed})",
            )],
        ),
        rule(
            "FAILED",
            "Failed within taken",
            "No more samples can fail than were taken.",
            3.0,
            "d2:hasValue(#{samples failed}) && #{samples failed} > #{samples taken}",
            vec![action(
                ActionType::Showerror,
                "samples failed",
                "More samples failed than were taken.",
                "",
            )],
        ),
        rule(
            "RIVER",
            "Untreated river water",
            "Warns when river water is drunk untreated.",
            4.0,
            "#{water source} == 'river'\n  && d2:hasValue(#{treated}) && !#{treated}",
            vec![action(
                ActionType::Showwarning,
                "treated",
                "River water should be treated before it is drunk.",
                "",
            )],
        ),
        rule(
            "FOLLOW_UP",
            "Follow-up when at risk",
            "Asks for a follow-up date only for a water point whose risk score is 4 or more.",
            5.0,
            "#{risk score} < 4",
            vec![action(ActionType::Hidefield, "follow-up date", "", "")],
        ),
        rule(
            "FOLLOW_UP_SOON",
            "Follow-up within a month",
            "Warns when the follow-up is set more than 30 days after the inspection.",
            6.0,
            "d2:hasValue(#{follow-up date})\n  && d2:daysBetween(V{event_date}, #{follow-up date}) > 30",
            vec![action(
                ActionType::Showwarning,
                "follow-up date",
                "The follow-up is more than 30 days after the inspection: ",
                "d2:concatenate(d2:daysBetween(V{event_date}, #{follow-up date}), ' days')",
            )],
        ),
        rule(
            "SAMPLES",
            "Samples short of target",
            "Says when fewer samples were taken than each site should take.",
            7.0,
            &format!("d2:hasValue(#{{samples taken}}) && #{{samples taken}} < C{{{samples}}}"),
            vec![action(
                ActionType::Displaytext,
                "",
                "Samples asked of each site: ",
                &format!("C{{{samples}}}"),
            )],
        ),
    ]
}

/// What an action did on the form.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Effect {
    pub rule: String,
    #[serde(rename = "type")]
    pub kind: ActionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub text: String,
}

/// A rule that could not run, with why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Broken {
    pub rule: String,
    pub message: String,
}

/// What the rules did on the form: the fields hidden, the values assigned, every effect, and the
/// rules that could not run.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RuleRun {
    pub hidden: BTreeSet<String>,
    /// The fields hidden, in the order they were first hidden.
    pub hidden_order: Vec<String>,
    /// The values assigned, by field, in the order each was first assigned.
    pub assigned: Vec<(String, Value)>,
    pub effects: Vec<Effect>,
    pub broken: Vec<Broken>,
}

/// The form's values by variable, in the order each was first set.
pub type FormValues = Vec<(String, Value)>;

fn get<'a>(values: &'a FormValues, key: &str) -> Option<&'a Value> {
    values.iter().find(|(one, _)| one == key).map(|(_, value)| value)
}

fn put(values: &mut FormValues, key: &str, value: Value) {
    match values.iter_mut().find(|(one, _)| one == key) {
        Some(found) => found.1 = value,
        None => values.push((key.to_owned(), value)),
    }
}

/// The rules in the order they run: by priority, then by name.
pub fn ordered(rules: &[ProgramRule]) -> Vec<ProgramRule> {
    let mut ordered = rules.to_vec();
    ordered.sort_by(|a, b| {
        let by = a.priority - b.priority;
        if by != 0.0 && !by.is_nan() {
            by.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            locale_compare(&a.name, &b.name, false)
        }
    });
    ordered
}

/// The rules run on the form's values, assigned values taking the place of typed ones.
pub fn run_rules(rules: &[ProgramRule], values: &FormValues, event_date: &str, version: &str) -> RuleRun {
    let mut current = values.clone();
    let mut run = RuleRun::default();
    let context = CheckContext {
        version: version.to_owned(),
        variables: variables(),
        evaluated_variables: EVALUATED_VARIABLES.iter().map(|one| (*one).to_owned()).collect(),
        indicators: Vec::new(),
    };
    for rule in ordered(rules) {
        let condition = check(&rule.condition, Language::Rule, &context);
        let broken = condition.first_error().map(str::to_owned);
        let (Some(node), None) = (&condition.node, &broken) else {
            run.broken.push(Broken {
                rule: rule.id.clone(),
                message: broken.unwrap_or_else(|| "The condition cannot be read.".to_owned()),
            });
            continue;
        };
        let value = {
            let held = current.clone();
            let scope = move |kind: ItemKind, body: &str, _offset: f64| form_item(&held, event_date, kind, body);
            evaluate(node, &scope).value
        };
        let holds = match &value {
            Value::Bool(flag) => *flag,
            Value::Number(number) => *number != 0.0,
            Value::Text(text) => !text.is_empty() && text != "false",
            Value::Null => false,
        };
        if !holds {
            continue;
        }
        for one in &rule.actions {
            let shape = one.kind.shape();
            let mut data = Value::Null;
            if shape.data && !trim(&one.data).is_empty() {
                let checked = check(&one.data, Language::Rule, &context);
                let wrong = checked.first_error().map(str::to_owned);
                let (Some(node), None) = (&checked.node, &wrong) else {
                    run.broken.push(Broken {
                        rule: rule.id.clone(),
                        message: wrong.unwrap_or_else(|| "An action\u{2019}s expression cannot be read.".to_owned()),
                    });
                    continue;
                };
                let held = current.clone();
                let scope = move |kind: ItemKind, body: &str, _offset: f64| form_item(&held, event_date, kind, body);
                data = evaluate(node, &scope).value;
            }
            let said = format!(
                "{}{}",
                one.content,
                match &data {
                    Value::Null => String::new(),
                    Value::Number(number) => number_to_string(round(number * 100.0) / 100.0),
                    other => text(other),
                }
            );
            match one.kind {
                ActionType::Hidefield => {
                    // A hidden field's value is cleared, so the rules after it read it as empty.
                    put(&mut current, &one.field, Value::Null);
                    if run.hidden.insert(one.field.clone()) {
                        run.hidden_order.push(one.field.clone());
                    }
                    run.effects.push(Effect {
                        rule: rule.id.clone(),
                        kind: one.kind,
                        field: Some(one.field.clone()),
                        text: "Hidden".to_owned(),
                    });
                }
                ActionType::Assign => {
                    put(&mut current, &one.field, data.clone());
                    put(&mut run.assigned, &one.field, data.clone());
                    run.effects.push(Effect {
                        rule: rule.id.clone(),
                        kind: one.kind,
                        field: Some(one.field.clone()),
                        text: format!(
                            "Set to {}",
                            if data == Value::Null {
                                "nothing".to_owned()
                            } else {
                                text(&data)
                            }
                        ),
                    });
                }
                _ => run.effects.push(Effect {
                    rule: rule.id.clone(),
                    kind: one.kind,
                    field: shape.field.then(|| one.field.clone()),
                    text: said,
                }),
            }
        }
    }
    run
}

/// What an item reads on the form: a field by its rule variable, a constant, or the inspection's
/// date and today.
pub fn form_item(current: &FormValues, event_date: &str, kind: ItemKind, body: &str) -> Value {
    let body = trim(body);
    match kind {
        ItemKind::Hash | ItemKind::A => get(current, body).cloned().unwrap_or(Value::Null),
        ItemKind::C => constant(body).map_or(Value::Null, |one| Value::Number(one.value)),
        ItemKind::V => match body {
            "event_date" => Value::Text(event_date.to_owned()),
            "current_date" => Value::Text(TODAY.to_owned()),
            _ => Value::Null,
        },
        _ => Value::Null,
    }
}
