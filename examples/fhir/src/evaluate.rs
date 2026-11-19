//! FHIRPath as the FHIRPath page evaluates it, as the web's `data/evaluate.ts` keeps it: an
//! expression evaluated against a resource or a Bundle, `%resource` and `%context` bound to it,
//! `%loinc` and `%sct` to their systems; and the words the editor offers while one is typed.

use serde::Serialize;
use serde_json::Value;

use crate::fhirpath::{evaluate as run, parse, to_json, variables};

/// What an expression gave: nothing evaluated, its value, or why it does not evaluate, at the
/// line and column (from 0) ANTLR names.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Evaluation {
    Empty,
    Done {
        value: Vec<Value>,
        /// How long it took, in milliseconds.
        ms: f64,
    },
    Failed {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        line: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        column: Option<usize>,
    },
}

/// The text with each closed block comment replaced by what `by` makes of it; an unclosed one stays.
pub fn without_block_comments(text: &str, by: impl Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut at = 0;
    while let Some(open) = text[at..].find("/*").map(|found| found + at) {
        let Some(close) = text[open + 2..].find("*/").map(|found| found + open + 2) else {
            break;
        };
        out.push_str(&text[at..open]);
        out.push_str(&by(&text[open..close + 2]));
        at = close + 2;
    }
    out.push_str(&text[at..]);
    out
}

/// The text without its `//` comments, each to its line's end.
fn without_line_comments(text: &str) -> String {
    text.split('\n')
        .map(|line| line.find("//").map_or(line, |at| &line[..at]))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `line: N; column: M; message: ...` read from an error's words, its message what follows.
fn located(text: &str) -> Option<(usize, usize, String)> {
    let mut from = 0;
    while let Some(found) = text[from..].find("line:").map(|at| at + from) {
        from = found + 1;
        let rest = text[found + 5..].trim_start_matches(crate::js::is_space);
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        let rest = rest[digits.len()..].trim_start_matches(crate::js::is_space);
        let Some(rest) = rest.strip_prefix(';') else { continue };
        let rest = rest.trim_start_matches(crate::js::is_space);
        let Some(rest) = rest.strip_prefix("column:") else {
            continue;
        };
        let rest = rest.trim_start_matches(crate::js::is_space);
        let column: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if column.is_empty() {
            continue;
        }
        let rest = rest[column.len()..].trim_start_matches(crate::js::is_space);
        let Some(rest) = rest.strip_prefix(';') else { continue };
        let rest = rest.trim_start_matches(crate::js::is_space);
        let Some(rest) = rest.strip_prefix("message:") else {
            continue;
        };
        let message = crate::js::trim(rest.trim_start_matches(crate::js::is_space)).to_owned();
        return Some((digits.parse().ok()?, column.parse().ok()?, message));
    }
    None
}

/// Evaluates an expression against a resource or a Bundle at the clock's `now_ms`.
pub fn evaluate(expression: &str, subject: Option<&Value>, now_ms: i64) -> Evaluation {
    let code = without_line_comments(&without_block_comments(expression, |comment| {
        comment.chars().map(|c| if c == '\n' { '\n' } else { ' ' }).collect()
    }));
    let Some(subject) = subject else {
        return Evaluation::Empty;
    };
    if crate::js::trim(&code).is_empty() {
        return Evaluation::Empty;
    }
    let vars = variables(&[("loinc", "http://loinc.org"), ("sct", "http://snomed.info/sct")]);
    let result = parse(expression).and_then(|expr| run(&expr, subject, &vars, now_ms));
    match result {
        Ok(items) => Evaluation::Done {
            value: to_json(&items),
            ms: 0.0,
        },
        Err(text) => match located(&text) {
            Some((line, column, message)) => Evaluation::Failed {
                message,
                line: Some(line),
                column: Some(column),
            },
            None => Evaluation::Failed {
                message: text,
                line: None,
                column: None,
            },
        },
    }
}

/// A word the editor offers while an expression is typed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Completion {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    pub kind: &'static str,
}

fn function(label: &str, detail: &str, docs: &str) -> Completion {
    Completion {
        label: label.to_owned(),
        detail: Some(detail.to_owned()),
        docs: Some(docs.to_owned()),
        kind: "function",
    }
}

/// The FHIRPath functions the editor offers, each with its signature and what it does.
pub fn functions() -> Vec<Completion> {
    [
        ("where", "where(criteria)", "The items for which the criteria are true."),
        (
            "select",
            "select(projection)",
            "The projection evaluated on each item, flattened.",
        ),
        (
            "exists",
            "exists([criteria])",
            "True when the collection, or the items matching the criteria, are not empty.",
        ),
        ("empty", "empty()", "True when the collection is empty."),
        ("all", "all(criteria)", "True when the criteria hold for every item."),
        ("count", "count()", "The number of items."),
        ("first", "first()", "The first item."),
        ("last", "last()", "The last item."),
        ("tail", "tail()", "Every item but the first."),
        ("skip", "skip(num)", "Every item after the first num."),
        ("take", "take(num)", "The first num items."),
        ("distinct", "distinct()", "The items with duplicates removed."),
        (
            "ofType",
            "ofType(type)",
            "The items of the given type, such as ofType(Quantity).",
        ),
        ("is", "is(type)", "True when the single item is of the type."),
        ("as", "as(type)", "The single item if it is of the type, else empty."),
        (
            "iif",
            "iif(criterion, true-result [, otherwise-result])",
            "One result or the other by the criterion.",
        ),
        ("not", "not()", "The negation of a single Boolean."),
        (
            "contains",
            "contains(substring)",
            "True when the string holds the substring.",
        ),
        (
            "startsWith",
            "startsWith(prefix)",
            "True when the string starts with the prefix.",
        ),
        (
            "endsWith",
            "endsWith(suffix)",
            "True when the string ends with the suffix.",
        ),
        (
            "matches",
            "matches(regex)",
            "True when the string matches the regular expression.",
        ),
        (
            "replace",
            "replace(pattern, substitution)",
            "The string with each pattern replaced.",
        ),
        ("substring", "substring(start [, length])", "Part of the string."),
        ("upper", "upper()", "The string in upper case."),
        ("lower", "lower()", "The string in lower case."),
        ("length", "length()", "The number of characters in the string."),
        ("join", "join([separator])", "The strings joined into one."),
        ("split", "split(separator)", "The string split at each separator."),
        ("toString", "toString()", "The single item as a string."),
        ("toInteger", "toInteger()", "The single item as an integer."),
        ("toDecimal", "toDecimal()", "The single item as a decimal."),
        ("toDate", "toDate()", "The single item as a date."),
        ("sum", "sum()", "The sum of the numbers."),
        ("min", "min()", "The least of the items."),
        ("max", "max()", "The greatest of the items."),
        ("avg", "avg()", "The mean of the numbers."),
        ("round", "round([precision])", "The number rounded."),
        ("abs", "abs()", "The absolute value."),
        ("today", "today()", "The current date."),
        ("now", "now()", "The current date and time."),
        ("children", "children()", "Every child element of each item."),
        ("descendants", "descendants()", "Every element under each item."),
        (
            "repeat",
            "repeat(projection)",
            "The projection applied again and again, every result kept.",
        ),
        (
            "aggregate",
            "aggregate(aggregator [, init])",
            "A value built over the items, with $this and $total.",
        ),
        (
            "union",
            "union(other)",
            "The items of both collections, without duplicates.",
        ),
        (
            "combine",
            "combine(other)",
            "The items of both collections, duplicates kept.",
        ),
        ("intersect", "intersect(other)", "The items in both collections."),
        ("exclude", "exclude(other)", "The items not in the other collection."),
        (
            "hasValue",
            "hasValue()",
            "True when the single item is a primitive with a value.",
        ),
        ("extension", "extension(url)", "The extensions with the given address."),
        (
            "trace",
            "trace(name)",
            "The collection unchanged, logged under the name.",
        ),
    ]
    .into_iter()
    .map(|(label, detail, docs)| function(label, detail, docs))
    .collect()
}

/// The variables an expression can name.
pub fn variables_offered() -> Vec<Completion> {
    let one = |label: &str, detail: &str, docs: &str, kind: &'static str| Completion {
        label: label.to_owned(),
        detail: Some(detail.to_owned()),
        docs: Some(docs.to_owned()),
        kind,
    };
    vec![
        one(
            "%resource",
            "the subject",
            "The resource or Bundle the expression is evaluated against.",
            "variable",
        ),
        one(
            "%context",
            "the subject",
            "Where evaluation started: the same subject.",
            "variable",
        ),
        one(
            "%ucum",
            "'http://unitsofmeasure.org'",
            "The address of the units of measure.",
            "constant",
        ),
        one(
            "%sct",
            "'http://snomed.info/sct'",
            "The address of SNOMED CT.",
            "constant",
        ),
        one("%loinc", "'http://loinc.org'", "The address of LOINC.", "constant"),
    ]
}

const SUFFIXES: [&str; 6] = ["DateTime", "Quantity", "String", "CodeableConcept", "Boolean", "Period"];

/// Every element name in a resource, at any depth (to six, the first twenty of a list), offered as
/// a property after a dot: a choice element by its base name, sorted.
pub fn element_names(subject: &Value) -> Vec<Completion> {
    fn walk(value: &Value, depth: usize, names: &mut Vec<String>) {
        if depth > 6 {
            return;
        }
        match value {
            Value::Array(items) => {
                for item in items.iter().take(20) {
                    walk(item, depth + 1, names);
                }
            }
            Value::Object(fields) => {
                for (name, child) in fields {
                    let base = SUFFIXES
                        .iter()
                        .find(|suffix| name.ends_with(*suffix))
                        .map_or(name.as_str(), |suffix| {
                            if name == suffix {
                                name.as_str()
                            } else {
                                &name[..name.len() - suffix.len()]
                            }
                        });
                    if !names.iter().any(|one| one == base) {
                        names.push(base.to_owned());
                    }
                    walk(child, depth + 1, names);
                }
            }
            _ => {}
        }
    }
    let mut names = Vec::new();
    walk(subject, 0, &mut names);
    let mut names: Vec<String> = names.into_iter().filter(|name| !name.is_empty()).collect();
    names.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    names
        .into_iter()
        .map(|label| Completion {
            label,
            detail: None,
            docs: None,
            kind: "property",
        })
        .collect()
}

/// What a word in an expression is, for the editor's hover.
pub fn describe_word(word: &str) -> Option<String> {
    let all: Vec<Completion> = functions().into_iter().chain(variables_offered()).collect();
    let found = all
        .into_iter()
        .find(|one| one.label == word || one.label == format!("%{word}"))?;
    Some(format!(
        "{}: {}",
        found.detail.clone().unwrap_or(found.label.clone()),
        found.docs.unwrap_or_default()
    ))
}
