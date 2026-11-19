//! What every page shares, as the web's `ui.tsx` keeps it: a figure as the pages write it, a count
//! with its noun, a finding as the field's issue mark lists it, the words offered as an expression
//! is typed, what a uid under the pointer stands for, an expression's verdict, and an expression
//! read back with the names its items stand for.

use serde::Serialize;

use crate::check::{Checked, Tone, item_name};
use crate::expression::{Language, collapse_spaces, describe, evaluated, without_block_comments};
use crate::js::{intl_number, trim};
use crate::metadata::{REPORTING_METRICS, constants, data_element, data_elements, data_set, option_combo};
use crate::versions::names_of;

/// A number as `Intl.NumberFormat('en-US', { maximumFractionDigits: 1 })` writes it.
pub fn number(value: f64) -> String {
    intl_number(value, 0, 1, true, false)
}

/// A value as the pages write it, a dash for none.
pub fn figure(value: Option<f64>, unit: &str) -> String {
    match value {
        None => "\u{2013}".to_owned(),
        Some(value) => format!("{}{unit}", number(value)),
    }
}

/// A count with its noun, the plural `many` or the noun with an s.
pub fn count(value: f64, one: &str, many: Option<&str>) -> String {
    let noun = if value == 1.0 {
        one.to_owned()
    } else {
        many.map_or_else(|| format!("{one}s"), str::to_owned)
    };
    format!("{} {noun}", number(value))
}

/// A count of things with its noun.
pub fn counted(value: usize, one: &str) -> String {
    count(value as f64, one, None)
}

/// A finding as the field's issue mark lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// `critical` or `warning`.
    pub tone: &'static str,
    pub text: String,
}

/// The line an offset falls on, counted from 1.
fn line_of(source: &str, offset: usize) -> usize {
    let units: Vec<u16> = source.encode_utf16().take(offset).collect();
    1 + units.iter().filter(|unit| **unit == u16::from(b'\n')).count()
}

/// The findings as the field's issue mark lists them, each with its line when the source has more
/// than one.
pub fn issues_of(source: &str, checked: &Checked) -> Vec<Issue> {
    checked
        .findings
        .iter()
        .map(|finding| Issue {
            tone: if finding.tone == Tone::Error {
                "critical"
            } else {
                "warning"
            },
            text: if source.contains('\n') {
                format!("Line {}: {}", line_of(source, finding.from), finding.message)
            } else {
                finding.message.clone()
            },
        })
        .collect()
}

/// A word offered as an expression is typed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Completion {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub kind: &'static str,
    pub apply: String,
}

/// The words a function is offered with: its name and whether the workbench runs it.
pub fn function_word(name: &str) -> Completion {
    let label = name.strip_prefix('.').unwrap_or(name);
    let bare = label.strip_prefix("d2:").unwrap_or(label);
    Completion {
        label: bare.to_owned(),
        detail: Some(
            if evaluated(name).is_none() {
                "function, not evaluated here"
            } else {
                "function"
            }
            .to_owned(),
        ),
        kind: "function",
        apply: format!("{bare}("),
    }
}

/// The leftmost item opened and not closed at the end of `before`: its kind and what is typed in it.
fn open_item(before: &str) -> Option<(&str, &str)> {
    let bytes = before.as_bytes();
    for (at, _) in before.char_indices() {
        let (kind, open) = if before[at..].starts_with("OUG{") {
            ("OUG", 4)
        } else if matches!(bytes[at], b'#' | b'A' | b'C' | b'R' | b'V') && bytes.get(at + 1) == Some(&b'{') {
            (&before[at..at + 1], 2)
        } else {
            continue;
        };
        let inside = &before[at + open..];
        if !inside.contains('}') {
            return Some((kind, inside));
        }
    }
    None
}

/// Whether `before` ends with a `)` or a `}`, maybe spaces, and a dot.
fn after_dot(before: &str) -> bool {
    let Some(rest) = before.strip_suffix('.') else {
        return false;
    };
    let rest = rest.trim_end_matches(crate::js::is_space);
    rest.ends_with(')') || rest.ends_with('}')
}

/// The words offered as an expression is typed, by what is typed before the cursor: inside `#{`
/// the data elements by name (inserting their uid) or, after a data element and a dot, its
/// category option combos; inside `C{` and `R{` the constants and the data set; inside a rule's
/// `#{` the program's variables; elsewhere the functions the server's version knows.
pub fn completions(language: Language, version: &str, variables: &[String], before: &str) -> Vec<Completion> {
    let names = names_of(version);
    if let Some((kind, inside)) = open_item(before) {
        if language == Language::Rule && (kind == "#" || kind == "A") {
            return variables
                .iter()
                .map(|variable| Completion {
                    label: variable.clone(),
                    detail: Some("rule variable".to_owned()),
                    kind: "variable",
                    apply: format!("{variable}}}"),
                })
                .collect();
        }
        if language == Language::Rule && kind == "V" {
            return names
                .variables
                .iter()
                .map(|variable| Completion {
                    label: (*variable).to_owned(),
                    detail: Some("program variable".to_owned()),
                    kind: "variable",
                    apply: format!("{variable}}}"),
                })
                .collect();
        }
        if kind == "C" {
            return constants()
                .iter()
                .map(|one| Completion {
                    label: one.name.clone(),
                    detail: Some(one.id.clone()),
                    kind: "constant",
                    apply: format!("{}}}", one.id),
                })
                .collect();
        }
        if kind == "R" {
            return if inside.contains('.') {
                REPORTING_METRICS
                    .iter()
                    .map(|metric| Completion {
                        label: (*metric).to_owned(),
                        detail: None,
                        kind: "enum",
                        apply: format!("{metric}}}"),
                    })
                    .collect()
            } else {
                vec![Completion {
                    label: data_set().name.clone(),
                    detail: Some(data_set().id.clone()),
                    kind: "type",
                    apply: format!("{}.", data_set().id),
                }]
            };
        }
        if kind == "#" {
            let element = data_element(inside.split('.').next().unwrap_or_default());
            if let (true, Some(element)) = (inside.contains('.'), element) {
                return element
                    .category_combo
                    .combos
                    .iter()
                    .map(|combo| Completion {
                        label: combo.name.clone(),
                        detail: Some(combo.id.clone()),
                        kind: "enum",
                        apply: format!("{}}}", combo.id),
                    })
                    .collect();
            }
            return data_elements()
                .iter()
                .map(|one| Completion {
                    label: one.name.clone(),
                    detail: Some(one.id.clone()),
                    kind: "variable",
                    apply: one.id.clone(),
                })
                .collect();
        }
        return Vec::new();
    }
    if after_dot(before) {
        return names
            .functions
            .iter()
            .filter(|name| name.starts_with('.'))
            .map(|name| function_word(name))
            .collect();
    }
    if before.ends_with("d2:") {
        return names
            .functions
            .iter()
            .filter(|name| name.starts_with("d2:"))
            .map(|name| function_word(name))
            .collect();
    }
    let mut out: Vec<Completion> = names
        .functions
        .iter()
        .filter(|name| !name.starts_with('.') && !name.starts_with("d2:"))
        .map(|name| function_word(name))
        .collect();
    if language == Language::Rule {
        out.push(Completion {
            label: "d2:".to_owned(),
            detail: Some("program functions".to_owned()),
            kind: "keyword",
            apply: "d2:".to_owned(),
        });
    }
    out
}

/// What a uid under the pointer stands for, by its kind of metadata.
pub fn hover_word(word: &str) -> Option<String> {
    if let Some(element) = data_element(word) {
        return Some(format!(
            "{}: data element, by {}",
            element.name, element.category_combo.name
        ));
    }
    if let Some(combo) = option_combo(word) {
        return Some(format!("{}: category option combo", combo.name));
    }
    if let Some(found) = constants().iter().find(|one| one.id == word) {
        return Some(format!(
            "{}: constant, {}",
            found.name,
            crate::js::number_to_string(found.value)
        ));
    }
    if word == data_set().id {
        return Some(format!("{}: data set", data_set().name));
    }
    None
}

/// A mark an editor underlines a finding with, at least one unit wide.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Mark {
    pub from: usize,
    pub to: usize,
    pub tone: Tone,
    pub message: String,
}

/// The marks an expression field underlines its findings with.
pub fn marks_of(checked: &Checked) -> Vec<Mark> {
    checked
        .findings
        .iter()
        .map(|finding| Mark {
            from: finding.from,
            to: finding.to.max(finding.from + 1),
            tone: finding.tone,
            message: finding.message.clone(),
        })
        .collect()
}

/// What an expression field says under itself: the expression read with the names its items stand
/// for, for an aggregate expression that says anything.
pub fn field_help(language: Language, value: &str) -> Option<String> {
    let said = match language {
        Language::Expression => describe_items(value),
        Language::Rule => trim(&collapse_spaces(&without_block_comments(value))).to_owned(),
    };
    (!said.is_empty() && language != Language::Rule).then(|| format!("Reads as: {said}"))
}

/// How a set of checks went, as one state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Valid,
    Warnings,
    Invalid,
}

impl Verdict {
    pub fn key(self) -> &'static str {
        match self {
            Verdict::Valid => "valid",
            Verdict::Warnings => "warnings",
            Verdict::Invalid => "invalid",
        }
    }
}

/// How a set of checks went, as one state.
pub fn verdict(checks: &[&Checked]) -> Verdict {
    let findings: Vec<_> = checks.iter().flat_map(|one| one.findings.iter()).collect();
    if findings.iter().any(|one| one.tone == Tone::Error) {
        Verdict::Invalid
    } else if !findings.is_empty() {
        Verdict::Warnings
    } else {
        Verdict::Valid
    }
}

/// The color each state the pages draw as a chip takes: an expression's verdict and a rule's
/// importance.
pub const STATUSES: [(&str, &str); 6] = [
    ("valid", "good"),
    ("warnings", "warning"),
    ("invalid", "critical"),
    ("high", "critical"),
    ("medium", "warning"),
    ("low", "neutral"),
];

/// An aggregate expression with its items written as the names they stand for.
pub fn describe_items(source: &str) -> String {
    describe(source, |kind, body| {
        item_name(kind, trim(body)).map(|name| format!("{{{name}}}"))
    })
}
