//! What a schema says is wrong with a form's value, after sokkel-ui's `schema/validate.ts`,
//! `schema/issues.ts` and `schema/issue-words.ts`: the core's schema checker's findings, each at
//! its path with a fix when one is plain, and said in plain English.

use serde_json::{Value, json};

use super::super::code_editor::schema::{self as checker, IssueKind, literal};
use super::read::{field_schema, get, schema_at, text, types, value_at};

/// How bad an issue is: only `Critical` stops a submit; a `Warning` is allowed but worth a look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueTone {
    Critical,
    Warning,
}

/// What is wrong with a value, by rule; the form says it in words.
#[derive(Clone, Debug, PartialEq)]
pub enum SchemaIssueKind {
    Required,
    Type {
        expected: String,
    },
    Enum {
        values: Vec<Value>,
    },
    Const {
        value: Value,
    },
    Minimum {
        limit: f64,
        exclusive: bool,
    },
    Maximum {
        limit: f64,
        exclusive: bool,
    },
    MinLength {
        limit: u64,
    },
    MaxLength {
        limit: u64,
    },
    Pattern {
        pattern: String,
    },
    Format {
        format: String,
    },
    MinItems {
        limit: u64,
    },
    MaxItems {
        limit: u64,
    },
    UniqueItems,
    Deprecated,
    /// The object has a property the schema does not allow; the issue sits at the object.
    Additional {
        property: String,
    },
    /// A rule without words of its own, said as the checker says it.
    Other {
        message: String,
    },
}

/// One thing a schema says about a value: where, how bad, which rule, and a value that satisfies
/// it when one is plain.
#[derive(Clone, Debug, PartialEq)]
pub struct SchemaIssue {
    pub kind: SchemaIssueKind,
    pub path: Vec<String>,
    pub tone: IssueTone,
    pub fix: Option<Value>,
}

/// An issue in words: the message, and what to do when the message alone does not say it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueSaid {
    pub text: String,
    pub detail: Option<String>,
}

impl IssueSaid {
    fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            detail: None,
        }
    }

    fn with(text: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            detail: Some(detail.into()),
        }
    }
}

fn number(n: f64) -> String {
    super::super::write_number(n)
}

/// An issue in plain English, as sokkel-ui's `sayIssue`; `schema` is the field's own.
pub fn say_issue(issue: &SchemaIssue, schema: &Value) -> IssueSaid {
    let plural = |count: u64, unit: &str| format!("{count} {unit}{}", if count == 1 { "" } else { "s" });
    match &issue.kind {
        SchemaIssueKind::Required => IssueSaid::new(if get(schema, "enum").is_some() {
            "Choose a value"
        } else {
            "Enter a value"
        }),
        SchemaIssueKind::Type { expected } => IssueSaid::new(match expected.as_str() {
            "integer" => "Enter a whole number".to_owned(),
            "number" => "Enter a number".to_owned(),
            other => format!("Enter a value of type {other}"),
        }),
        SchemaIssueKind::Enum { values } => IssueSaid::with(
            "Choose one of the values",
            values.iter().map(literal).collect::<Vec<_>>().join(", "),
        ),
        SchemaIssueKind::Const { value } => IssueSaid::new(format!("Must be {}", literal(value))),
        SchemaIssueKind::Minimum { limit, exclusive } => IssueSaid::new(if *exclusive {
            format!("Must be more than {}", number(*limit))
        } else {
            format!("Must be at least {}", number(*limit))
        }),
        SchemaIssueKind::Maximum { limit, exclusive } => IssueSaid::new(if *exclusive {
            format!("Must be less than {}", number(*limit))
        } else {
            format!("Must be at most {}", number(*limit))
        }),
        SchemaIssueKind::MinLength { limit } => IssueSaid::new(format!("Use at least {}", plural(*limit, "character"))),
        SchemaIssueKind::MaxLength { limit } => IssueSaid::new(format!("Use at most {}", plural(*limit, "character"))),
        SchemaIssueKind::Pattern { pattern } => IssueSaid::with(
            "Not in the expected form",
            text(schema, "description")
                .map(str::to_owned)
                .unwrap_or_else(|| format!("It must match {pattern}.")),
        ),
        SchemaIssueKind::Format { format } => match format.as_str() {
            "email" => IssueSaid::with("Enter an email address", "As in ada@example.org."),
            "uri" | "url" => IssueSaid::with("Enter a full address", "Starting with https://."),
            "date" => IssueSaid::with("Enter a date", "As in 2026-11-05."),
            "date-time" => IssueSaid::with("Enter a date and time", "As in 2026-11-05T09:30:00Z."),
            "time" => IssueSaid::with("Enter a time", "As in 09:30:00."),
            "uuid" => IssueSaid::with("Enter a UUID", "As in 0b6f4c1e-2f6a-4c9e-9d3b-6f1d2e8a7c55."),
            "hostname" => IssueSaid::with("Enter a host name", "As in hooks.example.org."),
            "ipv4" => IssueSaid::with("Enter an IPv4 address", "As in 192.0.2.10."),
            other => IssueSaid::new(format!("Enter a value in the {other} format")),
        },
        SchemaIssueKind::MinItems { limit } => IssueSaid::new(format!("Add at least {}", plural(*limit, "item"))),
        SchemaIssueKind::MaxItems { limit } => IssueSaid::new(format!("Keep at most {}", plural(*limit, "item"))),
        SchemaIssueKind::UniqueItems => IssueSaid::new("Each value only once"),
        SchemaIssueKind::Deprecated => {
            IssueSaid::with("Deprecated", "Still accepted, and may be removed in a later version.")
        }
        SchemaIssueKind::Additional { property } => IssueSaid::new(format!("No field {property} is allowed here")),
        SchemaIssueKind::Other { message } => {
            let mut chars = message.chars();
            IssueSaid::new(
                chars
                    .next()
                    .map(|c| c.to_uppercase().chain(chars).collect::<String>())
                    .unwrap_or_default(),
            )
        }
    }
}

/// Whether a string passes a pattern, as the core's checker reads one.
fn matches(pattern: &str, text: &str) -> bool {
    checker::validate(
        &json!({"type": "string", "pattern": pattern}),
        &Value::String(text.to_owned()),
    )
    .is_empty()
}

/// A value that matches the pattern, made from the given one by the usual repairs, when one does.
fn repair(value: &str, pattern: &str) -> Option<String> {
    let trimmed = value.trim();
    let mut slug = String::new();
    for c in trimmed.to_lowercase().chars() {
        let c = if c.is_whitespace() || c == '_' { '-' } else { c };
        if c.is_ascii_lowercase() || c.is_ascii_digit() || (c == '-' && !slug.ends_with('-')) {
            slug.push(c);
        }
    }
    let slug = slug.trim_matches('-').to_owned();
    let parts: Vec<&str> = slug.split('-').collect();
    let lead = parts
        .iter()
        .position(|p| p.starts_with(|c: char| c.is_ascii_lowercase()));
    let rotated = match lead {
        Some(lead) if lead > 0 => [&parts[lead..], &parts[..lead]].concat().join("-"),
        _ => slug.clone(),
    };
    let unled = slug.trim_start_matches(|c: char| !c.is_ascii_lowercase()).to_owned();
    [trimmed.to_owned(), trimmed.to_lowercase(), slug, rotated, unled]
        .into_iter()
        .find(|candidate| !candidate.is_empty() && candidate != value && matches(pattern, candidate))
}

/// The value with every empty string in a list read as unset, as the web's form checks an empty
/// item: the paths of those items.
fn empty_items(value: &Value, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match value {
        Value::Array(list) => {
            for (index, item) in list.iter().enumerate() {
                path.push(index.to_string());
                if item.is_null() || item.as_str() == Some("") {
                    out.push(path.clone());
                } else {
                    empty_items(item, path, out);
                }
                path.pop();
            }
        }
        Value::Object(map) => {
            for (key, child) in map {
                path.push(key.clone());
                empty_items(child, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

/// Everything a schema says is wrong with a form's value, each at its path, checked by the core's
/// JSON Schema 2020-12 checker; and, as warnings, the deprecated fields it sets. Empty strings are
/// read as absent, the way a form leaves a cleared field, and an empty item of a list is said to
/// be missing. Each issue carries a fix when one is plain from the value and the rule.
pub fn schema_issues(schema: &Value, value: &Value) -> Vec<SchemaIssue> {
    let found = checker::validate(schema, value);
    let mut empties = Vec::new();
    empty_items(value, &mut Vec::new(), &mut empties);
    // an item left unset (null) is empty only where its schema refuses null
    empties.retain(|path| {
        value_at(value, path).is_some_and(|v| !v.is_null()) || found.iter().any(|issue| issue.path == *path)
    });
    let mut out: Vec<SchemaIssue> = Vec::new();
    for issue in found {
        if empties.contains(&issue.path) {
            continue;
        }
        let at = value_at(value, &issue.path);
        let tone = if issue.warning {
            IssueTone::Warning
        } else {
            IssueTone::Critical
        };
        let (kind, fix) = match issue.kind {
            IssueKind::Required => (SchemaIssueKind::Required, None),
            IssueKind::Type(expected) => {
                if at.is_none() {
                    (SchemaIssueKind::Required, None)
                } else {
                    let read = match at {
                        Some(Value::Number(n)) => n.as_f64(),
                        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
                        _ => None,
                    };
                    let fix = (expected == "integer")
                        .then_some(read)
                        .flatten()
                        .filter(|n| n.is_finite())
                        .map(|n| Value::from(n.round() as i64));
                    (SchemaIssueKind::Type { expected }, fix)
                }
            }
            IssueKind::Enum(values) => (SchemaIssueKind::Enum { values }, None),
            IssueKind::Const(value) => (SchemaIssueKind::Const { value: value.clone() }, Some(value)),
            IssueKind::Minimum { limit, exclusive } => (
                SchemaIssueKind::Minimum { limit, exclusive },
                (!exclusive).then(|| super::read::number_value(limit)),
            ),
            IssueKind::Maximum { limit, exclusive } => (
                SchemaIssueKind::Maximum { limit, exclusive },
                (!exclusive).then(|| super::read::number_value(limit)),
            ),
            IssueKind::MinLength(limit) => (SchemaIssueKind::MinLength { limit }, None),
            IssueKind::MaxLength(limit) => (
                SchemaIssueKind::MaxLength { limit },
                at.and_then(Value::as_str)
                    .map(|s| Value::String(s.chars().take(limit as usize).collect())),
            ),
            IssueKind::Pattern(pattern) => {
                let fix = at
                    .and_then(Value::as_str)
                    .and_then(|s| repair(s, &pattern))
                    .map(Value::String);
                (SchemaIssueKind::Pattern { pattern }, fix)
            }
            IssueKind::Format(format) => {
                let written = at.and_then(Value::as_str).unwrap_or("").trim();
                // a bare host, such as example.org/hooks, is an address once it says https://
                let bare = !written.is_empty()
                    && !written.contains(char::is_whitespace)
                    && written
                        .split_once('.')
                        .is_some_and(|(head, rest)| !head.is_empty() && !head.contains(['/', ':']) && !rest.is_empty());
                let fix = (matches!(format.as_str(), "uri" | "url") && bare)
                    .then(|| Value::String(format!("https://{written}")));
                (SchemaIssueKind::Format { format }, fix)
            }
            IssueKind::MinItems(limit) => (SchemaIssueKind::MinItems { limit }, None),
            IssueKind::MaxItems(limit) => (
                SchemaIssueKind::MaxItems { limit },
                at.and_then(Value::as_array)
                    .map(|l| Value::Array(l.iter().take(limit as usize).cloned().collect())),
            ),
            IssueKind::UniqueItems => {
                let fix = at.and_then(Value::as_array).map(|list| {
                    let mut kept: Vec<Value> = Vec::new();
                    for item in list {
                        if !kept.iter().any(|k| super::read::same(k, item)) {
                            kept.push(item.clone());
                        }
                    }
                    Value::Array(kept)
                });
                (SchemaIssueKind::UniqueItems, fix)
            }
            IssueKind::Deprecated => (SchemaIssueKind::Deprecated, None),
            IssueKind::Additional(property) => (SchemaIssueKind::Additional { property }, None),
            IssueKind::Other(message) => (SchemaIssueKind::Other { message }, None),
        };
        out.push(SchemaIssue {
            kind,
            path: issue.path,
            tone,
            fix,
        });
    }
    // an empty item of a list whose items have a type is one with nothing in it
    for path in empties {
        let item = schema_at(schema, &path, schema);
        if !types(&item).0.is_empty() || get(&item, "enum").is_some() {
            out.push(SchemaIssue {
                kind: SchemaIssueKind::Required,
                path,
                tone: IssueTone::Critical,
                fix: None,
            });
        }
    }
    out
}

/// The issues a check found, without those about a value `deferred` leaves unchecked or anything
/// inside one: a field holding `${steps.fetch.rows}` where a number goes is not an error, as it is
/// filled in later.
pub fn without_deferred(
    issues: Vec<SchemaIssue>,
    document: &Value,
    deferred: Option<&dyn Fn(&Value) -> bool>,
) -> Vec<SchemaIssue> {
    let Some(deferred) = deferred else {
        return issues;
    };
    issues
        .into_iter()
        .filter(|issue| {
            !(0..=issue.path.len()).any(|depth| value_at(document, &issue.path[..depth]).is_some_and(deferred))
        })
        .collect()
}

/// The field schema an issue is said against.
pub(crate) fn issue_schema(schema: &Value, issue: &SchemaIssue) -> Value {
    field_schema(&schema_at(schema, &issue.path, schema), schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(schema: &Value, value: &Value) -> Vec<String> {
        schema_issues(schema, value)
            .iter()
            .map(|i| say_issue(i, &issue_schema(schema, i)).text)
            .collect()
    }

    /// A value holding a live reference: `${...}` not escaped as `$${...}`.
    fn live(value: &Value) -> bool {
        value.as_str().is_some_and(|s| {
            let b = s.as_bytes();
            (0..b.len().saturating_sub(1)).any(|i| b[i] == b'$' && b[i + 1] == b'{' && (i == 0 || b[i - 1] != b'$'))
        })
    }

    fn at(path: &[&str], message: &str) -> SchemaIssue {
        SchemaIssue {
            kind: SchemaIssueKind::Other {
                message: message.into(),
            },
            path: path.iter().map(|p| (*p).to_owned()).collect(),
            tone: IssueTone::Critical,
            fix: None,
        }
    }

    #[test]
    fn values_left_unchecked_drop_their_issues_and_keep_the_rest() {
        let issues = vec![
            at(&["retries"], "Expected an integer"),
            at(&["config", "rows", "0"], "Expected a number"),
            at(&["code"], "Must match ^[a-z]"),
        ];
        let document =
            json!({"retries": "${steps.fetch.retries}", "config": {"rows": "${steps.fetch.rows}"}, "code": "Nightly"});
        let kept = without_deferred(issues.clone(), &document, Some(&live));
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].path, vec!["code".to_owned()]);
        let escaped = without_deferred(issues.clone(), &json!({"retries": "$${not}"}), Some(&live));
        assert!(escaped.iter().any(|i| i.path[0] == "retries"));
        assert_eq!(without_deferred(issues, &json!({}), None).len(), 3);
    }

    #[test]
    fn issues_are_said_as_the_web_says_them() {
        let schema = json!({
            "type": "object",
            "required": ["code", "kind"],
            "properties": {
                "code": {"type": "string"},
                "kind": {"enum": ["fetch", "upload"]},
                "retries": {"type": "integer", "minimum": 0, "maximum": 5},
                "name": {"type": "string", "pattern": "^[a-z][a-z0-9-]*$", "description": "Lowercase with dashes."},
                "email": {"type": "string", "format": "email"},
                "url": {"type": "string", "format": "uri"},
                "tags": {"type": "array", "minItems": 2, "uniqueItems": true, "items": {"type": "string"}}
            }
        });
        let value = json!({
            "retries": 9,
            "name": "Nightly Export",
            "email": "ada",
            "url": "example.org/hooks",
            "tags": ["a", "a"]
        });
        let said = texts(&schema, &value);
        for text in [
            "Enter a value",
            "Choose a value",
            "Must be at most 5",
            "Not in the expected form",
            "Enter an email address",
            "Enter a full address",
            "Each value only once",
        ] {
            assert!(said.contains(&text.to_owned()), "{text} in {said:?}");
        }
        let issues = schema_issues(&schema, &value);
        let fix =
            |kind: fn(&SchemaIssueKind) -> bool| issues.iter().find(|i| kind(&i.kind)).and_then(|i| i.fix.clone());
        assert_eq!(fix(|k| matches!(k, SchemaIssueKind::Maximum { .. })), Some(json!(5)));
        assert_eq!(
            fix(|k| matches!(k, SchemaIssueKind::Pattern { .. })),
            Some(json!("nightly-export"))
        );
        assert_eq!(
            fix(|k| matches!(k, SchemaIssueKind::Format { format } if format == "uri")),
            Some(json!("https://example.org/hooks"))
        );
        assert_eq!(fix(|k| matches!(k, SchemaIssueKind::UniqueItems)), Some(json!(["a"])));
        let pattern = issues
            .iter()
            .find(|i| matches!(i.kind, SchemaIssueKind::Pattern { .. }))
            .unwrap();
        assert_eq!(
            say_issue(pattern, &issue_schema(&schema, pattern)).detail.as_deref(),
            Some("Lowercase with dashes.")
        );
    }

    #[test]
    fn an_empty_item_of_a_list_is_said_to_be_missing() {
        let schema = json!({"type": "object", "properties": {"tags": {"type": "array", "items": {"type": "string"}}}});
        let issues = schema_issues(&schema, &json!({"tags": ["nightly", ""]}));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, SchemaIssueKind::Required);
        assert_eq!(issues[0].path, vec!["tags".to_owned(), "1".to_owned()]);
    }

    #[test]
    fn a_choice_of_scalar_types_passes_what_it_reads() {
        let schema = json!({
            "type": "object",
            "properties": {"buffer": {"title": "Buffer", "anyOf": [{"type": "string", "pattern": "^\\d+KB$"}, {"type": "integer"}]}}
        });
        let allowed = vec!["string".to_owned(), "integer".to_owned()];
        for typed in ["16384", "16KB"] {
            let value = json!({"buffer": super::super::read::read_scalar(typed, &allowed)});
            assert!(schema_issues(&schema, &value).is_empty(), "{typed}");
        }
    }

    #[test]
    fn lists_and_maps_of_booleans_pass_the_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "flags": {"type": "array", "items": {"type": "boolean"}},
                "features": {"type": "object", "additionalProperties": {"type": "boolean"}}
            }
        });
        assert!(schema_issues(&schema, &json!({"flags": [true, false], "features": {"digest": true}})).is_empty());
    }
}
