//! What the simulated server says about an expression before running it, as the web's
//! `data/check.ts` says it: whether it reads, whether every item names metadata it holds, and
//! whether the server's version knows every function and variable. Findings carry the span they
//! are about, for the editor to underline.

use serde::Serialize;

use crate::expression::{ItemKind, Language, Node, NodeKind, Span, evaluated, parse, walk, without_block_comments};
use crate::js::trim;
use crate::metadata::{REPORTING_METRICS, constant, data_element, data_sets, option_combo};
use crate::versions::{LATEST, added, names_of};

/// Whether a finding stops the expression from running, or only warns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Error,
    Warning,
}

/// Something wrong with an expression, over the span it is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub from: usize,
    pub to: usize,
    pub tone: Tone,
    pub message: String,
}

/// What a check reads besides the expression: the server's version, and a program's rule variables.
#[derive(Clone, Debug, Default)]
pub struct CheckContext {
    pub version: String,
    /// The program rule variables a rule may name in `#{}` and `A{}`.
    pub variables: Vec<String>,
    /// The variables the example evaluates in `V{}`.
    pub evaluated_variables: Vec<String>,
    /// The indicators an expression may name in `N{}`.
    pub indicators: Vec<String>,
}

/// An expression read and checked: its tree when it reads, and every finding.
#[derive(Clone, Debug, PartialEq)]
pub struct Checked {
    pub node: Option<Node>,
    pub findings: Vec<Finding>,
}

impl Checked {
    /// Whether any finding is an error.
    pub fn has_error(&self) -> bool {
        self.findings.iter().any(|one| one.tone == Tone::Error)
    }

    /// The first error's message.
    pub fn first_error(&self) -> Option<&str> {
        self.findings
            .iter()
            .find(|one| one.tone == Tone::Error)
            .map(|one| one.message.as_str())
    }
}

fn since(name: &str, version: &str) -> String {
    match added(name) {
        None => format!("this server ({version}) does not know it"),
        Some(came) => format!("it came in {came}; this server runs {version}"),
    }
}

/// The part of `body.split('.')` at `at`.
fn part(body: &str, at: usize) -> Option<&str> {
    body.split('.').nth(at)
}

/// The names an item stands for, as the editor's hover and the expression's description say them.
pub fn item_name(kind: ItemKind, body: &str) -> Option<String> {
    match kind {
        ItemKind::Hash => {
            let found = data_element(part(body, 0).unwrap_or_default())?;
            match part(body, 1) {
                None | Some("*") => Some(found.name.clone()),
                Some(combo) => option_combo(combo).map(|one| format!("{} {}", found.name, one.name)),
            }
        }
        ItemKind::C => constant(body).map(|one| one.name.clone()),
        ItemKind::R => {
            let set = part(body, 0).unwrap_or_default();
            let found = data_sets().into_iter().find(|one| one.id == set)?;
            let metric = part(body, 1).unwrap_or_default().to_lowercase().replace('_', " ");
            Some(format!("{} {metric}", found.name))
        }
        ItemKind::Days => Some("Days".to_owned()),
        _ => None,
    }
}

fn check_item(
    kind: ItemKind,
    raw: &str,
    span: Span,
    language: Language,
    context: &CheckContext,
    findings: &mut Vec<Finding>,
) {
    let mut say = |message: String, tone: Tone| {
        findings.push(Finding {
            from: span.from,
            to: span.to,
            tone,
            message,
        })
    };
    let body = trim(raw);
    if kind == ItemKind::Days {
        if language == Language::Rule {
            say(
                "[days] belongs to aggregate expressions, not to program rules.".to_owned(),
                Tone::Error,
            );
        }
        return;
    }
    if kind == ItemKind::C {
        if constant(body).is_none() {
            say(format!("No constant has the uid {body}."), Tone::Error);
        }
        return;
    }
    if language == Language::Rule {
        if kind == ItemKind::Hash || kind == ItemKind::A {
            if !context.variables.iter().any(|one| one == body) {
                say(
                    format!("The program has no rule variable named \"{body}\"."),
                    Tone::Error,
                );
            }
            return;
        }
        if kind == ItemKind::V {
            let names = names_of(&context.version).variables;
            if !names.contains(&body) {
                say(
                    if names_of(LATEST).variables.contains(&body) {
                        format!("V{{{body}}}: {}.", since(body, &context.version))
                    } else {
                        format!("No program variable is called {body}.")
                    },
                    Tone::Error,
                );
            } else if !context.evaluated_variables.iter().any(|one| one == body) {
                say(
                    format!("The server knows V{{{body}}}; this workbench does not try it on the form."),
                    Tone::Warning,
                );
            }
            return;
        }
        say(
            format!("{}{{}} has no meaning in a program rule.", kind.key()),
            Tone::Error,
        );
        return;
    }
    match kind {
        ItemKind::Hash => {
            let element = part(body, 0).unwrap_or_default();
            let Some(found) = data_element(element) else {
                say(format!("No data element has the uid {element}."), Tone::Error);
                return;
            };
            if let Some(combo) = part(body, 1).filter(|one| *one != "*") {
                match option_combo(combo) {
                    None => say(format!("No category option combo has the uid {combo}."), Tone::Error),
                    Some(one) if !found.category_combo.combos.iter().any(|own| own.id == combo) => say(
                        format!(
                            "{} is not one of the combinations {} is split by.",
                            one.name, found.name
                        ),
                        Tone::Error,
                    ),
                    Some(_) => {}
                }
            }
            if part(body, 2).is_some_and(|one| one != "*") {
                say(
                    "Attribute option combos are not split out on this server; write * or leave it off.".to_owned(),
                    Tone::Warning,
                );
            }
        }
        ItemKind::R => {
            let set = part(body, 0).unwrap_or_default();
            if !data_sets().iter().any(|one| one.id == set) {
                say(format!("No data set has the uid {set}."), Tone::Error);
            } else if !REPORTING_METRICS.contains(&part(body, 1).unwrap_or_default()) {
                say(
                    format!(
                        "A reporting rate asks for one of {} after its data set.",
                        REPORTING_METRICS.join(", ")
                    ),
                    Tone::Error,
                );
            }
        }
        ItemKind::N => {
            if !context.indicators.iter().any(|one| one == body) {
                say(format!("No indicator has the uid {body}."), Tone::Error);
            } else {
                say(
                    "The server knows this indicator; this workbench does not evaluate one indicator inside another."
                        .to_owned(),
                    Tone::Warning,
                );
            }
        }
        ItemKind::Oug => say(
            "Organization unit groups are not held on this server.".to_owned(),
            Tone::Error,
        ),
        ItemKind::I => say("This server holds no program indicators.".to_owned(), Tone::Error),
        _ => say(
            format!(
                "{}{{}} belongs to program expressions, not to aggregate ones.",
                kind.key()
            ),
            Tone::Error,
        ),
    }
}

fn check_call(
    name: &str,
    span: Span,
    count: usize,
    language: Language,
    context: &CheckContext,
    findings: &mut Vec<Finding>,
) {
    let mut say = |message: String, tone: Tone| {
        findings.push(Finding {
            from: span.from,
            to: span.to,
            tone,
            message,
        })
    };
    if language == Language::Expression && name.starts_with("d2:") {
        say(
            format!("{name} is a program function; it belongs to program rules and program indicators."),
            Tone::Error,
        );
        return;
    }
    if !names_of(&context.version).functions.contains(&name) {
        say(
            if names_of(LATEST).functions.contains(&name) {
                format!("{name}: {}.", since(name, &context.version))
            } else {
                format!("No function is called {name}.")
            },
            Tone::Error,
        );
        return;
    }
    let Some((min, max)) = evaluated(name) else {
        say(
            format!("The server knows {name}; this workbench does not evaluate it."),
            Tone::Warning,
        );
        return;
    };
    let noun = if min == 1 { "argument" } else { "arguments" };
    if count < min || max.is_some_and(|max| count > max) {
        say(
            match max {
                Some(max) if max == min => format!("{name} takes {min} {noun}, not {count}."),
                None => format!("{name} takes at least {min} {noun}."),
                Some(max) => format!("{name} takes {min} to {max} arguments, not {count}."),
            },
            Tone::Error,
        );
    }
}

/// An expression read and checked: its tree when it reads, and every finding.
pub fn check(source: &str, language: Language, context: &CheckContext) -> Checked {
    if trim(&without_block_comments(source)).is_empty() {
        return Checked {
            node: None,
            findings: vec![Finding {
                from: 0,
                to: 0,
                tone: Tone::Error,
                message: "The expression is empty.".to_owned(),
            }],
        };
    }
    let node = match parse(source) {
        Ok(node) => node,
        Err(error) => {
            return Checked {
                node: None,
                findings: vec![Finding {
                    from: error.span.from,
                    to: error.span.to.max(error.span.from + 1),
                    tone: Tone::Error,
                    message: error.message,
                }],
            };
        }
    };
    let mut findings = Vec::new();
    walk(&node, &mut |one| match &one.kind {
        NodeKind::Item { kind, body } => check_item(*kind, body, one.span(), language, context, &mut findings),
        NodeKind::Call { name, args, name_span } => {
            check_call(name, *name_span, args.len(), language, context, &mut findings)
        }
        NodeKind::After {
            name,
            target,
            args,
            name_span,
        } => {
            check_call(name, *name_span, args.len(), language, context, &mut findings);
            if name == ".periodOffset" && !matches!(target.kind, NodeKind::Item { .. } | NodeKind::After { .. }) {
                findings.push(Finding {
                    from: name_span.from,
                    to: name_span.to,
                    tone: Tone::Error,
                    message: ".periodOffset is written after a data item.".to_owned(),
                });
            }
        }
        NodeKind::Name(name) => findings.push(Finding {
            from: one.from,
            to: one.to,
            tone: Tone::Error,
            message: format!("{name} is not a value, an item or a function call."),
        }),
        _ => {}
    });
    Checked {
        node: Some(node),
        findings,
    }
}
