//! The shapes the workbench reads, as the web's `data/model.ts` keeps them: a FHIR resource as its
//! JSON, the server's words about one (an issue, a refusal, an OperationOutcome), and the texts a
//! concept, a name, a quantity and a reference are read as.

use serde::Serialize;
use serde_json::{Value, json};

use crate::js::{base64_decode, base64_encode, number_to_string};

/// A FHIR resource as the server holds it: its JSON object, `resourceType` and `id` first.
pub type Resource = Value;

/// One part of an element's path from the resource: a name, or a place in a list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Part {
    Name(String),
    Index(usize),
}

impl From<&str> for Part {
    fn from(name: &str) -> Self {
        Part::Name(name.to_owned())
    }
}

impl From<usize> for Part {
    fn from(index: usize) -> Self {
        Part::Index(index)
    }
}

/// How bad an issue is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Information,
}

impl Severity {
    pub fn key(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Information => "information",
        }
    }
}

/// One issue the server found, as a FHIR OperationOutcome's issue says it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub severity: Severity,
    pub code: &'static str,
    /// What it is about, as the element's path from the resource, such as `name`, 0, `family`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<Part>>,
    pub message: String,
}

impl Issue {
    pub fn new(severity: Severity, code: &'static str, path: Option<Vec<Part>>, message: impl Into<String>) -> Self {
        Issue {
            severity,
            code,
            path,
            message: message.into(),
        }
    }

    pub fn error(code: &'static str, path: Option<Vec<Part>>, message: impl Into<String>) -> Self {
        Issue::new(Severity::Error, code, path, message)
    }
}

/// A refusal from the server, carrying the OperationOutcome's issues.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Refused {
    pub status: u16,
    pub issues: Vec<Issue>,
}

impl Refused {
    pub fn new(status: u16, issues: Vec<Issue>) -> Self {
        Refused { status, issues }
    }

    /// What the refusal says: its first error's message, else its first issue's.
    pub fn message(&self) -> String {
        self.issues
            .iter()
            .find(|issue| issue.severity == Severity::Error)
            .or(self.issues.first())
            .map_or_else(
                || format!("Refused with {}", self.status),
                |issue| issue.message.clone(),
            )
    }
}

/// An issue's path as FHIRPath writes it: `Patient.name[0].family`.
pub fn issue_path(kind: &str, path: Option<&[Part]>) -> String {
    let mut text = kind.to_owned();
    for part in path.unwrap_or_default() {
        match part {
            Part::Index(index) => text.push_str(&format!("[{index}]")),
            Part::Name(name) => {
                text.push('.');
                text.push_str(name);
            }
        }
    }
    text
}

/// An OperationOutcome resource from issues, as the server answers a refusal.
pub fn outcome(kind: &str, issues: &[Issue]) -> Resource {
    json!({
        "resourceType": "OperationOutcome",
        "id": "outcome",
        "issue": issues.iter().map(|issue| {
            let mut one = json!({ "severity": issue.severity.key(), "code": issue.code, "diagnostics": issue.message });
            if let Some(path) = &issue.path {
                one["expression"] = json!([issue_path(kind, Some(path))]);
            }
            one
        }).collect::<Vec<_>>(),
    })
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_object().and_then(|fields| fields.get(key))
}

/// A value as `String(value)` writes it, for the plain values JSON holds.
pub fn js_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(figure) => number_to_string(figure.as_f64().unwrap_or(0.0)),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => "null".to_owned(),
        Value::Array(items) => items
            .iter()
            .map(|one| if one.is_null() { String::new() } else { js_string(one) })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// `a ?? b` over JSON: the first that is there and not null.
fn present(value: Option<&Value>) -> Option<&Value> {
    value.filter(|one| !one.is_null())
}

/// The text a Coding or CodeableConcept is read as: its text, its first display, or its code.
pub fn concept_text(value: &Value) -> String {
    if !value.is_object() {
        return String::new();
    }
    let first = text_at(value, "coding").and_then(|coding| coding.get(0));
    let found = present(text_at(value, "text"))
        .or_else(|| first.and_then(|one| present(text_at(one, "display"))))
        .or_else(|| first.and_then(|one| present(text_at(one, "code"))))
        .or_else(|| present(text_at(value, "display")))
        .or_else(|| present(text_at(value, "code")));
    found.map(js_string).unwrap_or_default()
}

/// A HumanName list as one name: the first's given names and family.
pub fn human_name(value: &Value) -> String {
    let Some(name) = value.as_array().and_then(|names| names.first()) else {
        return String::new();
    };
    if let Some(text) = text_at(name, "text") {
        return js_string(text);
    }
    let list = |key: &str| -> Vec<String> {
        match present(text_at(name, key)) {
            Some(Value::Array(items)) => items.iter().map(js_string).collect(),
            Some(other) => vec![js_string(other)],
            None => Vec::new(),
        }
    };
    let mut parts = list("prefix");
    parts.extend(list("given"));
    parts.push(present(text_at(name, "family")).map(js_string).unwrap_or_default());
    parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A Quantity as text: `72 /min`, `81.4 kg`.
pub fn quantity_text(value: &Value) -> String {
    if !value.is_object() {
        return String::new();
    }
    let Some(amount) = text_at(value, "value") else {
        return String::new();
    };
    let unit = present(text_at(value, "unit"))
        .or_else(|| present(text_at(value, "code")))
        .map(js_string)
        .unwrap_or_default();
    crate::js::trim(&format!("{} {unit}", js_string(amount))).to_owned()
}

/// A reference's target as `Type/id`, or its display.
pub fn reference_text(value: &Value) -> String {
    if !value.is_object() {
        return String::new();
    }
    present(text_at(value, "display"))
        .or_else(|| present(text_at(value, "reference")))
        .map(js_string)
        .unwrap_or_default()
}

/// A field of a resource, as `resource.name` reads it.
pub fn field<'a>(resource: &'a Value, name: &str) -> Option<&'a Value> {
    text_at(resource, name)
}

/// A field of a resource as text, or none when it is not a text.
pub fn field_str<'a>(resource: &'a Value, name: &str) -> Option<&'a str> {
    text_at(resource, name).and_then(Value::as_str)
}

/// A resource's type.
pub fn type_of(resource: &Value) -> String {
    field(resource, "resourceType").map(js_string).unwrap_or_default()
}

/// A resource's id.
pub fn id_of(resource: &Value) -> String {
    field(resource, "id").map(js_string).unwrap_or_default()
}

/// A resource's key on the server: `Patient/pat-001`.
pub fn key_of(resource: &Value) -> String {
    format!("{}/{}", type_of(resource), id_of(resource))
}

/// A resource's version, as `Number(resource.meta?.versionId ?? '0')` reads it.
pub fn version_of(resource: &Value) -> f64 {
    let meta = field(resource, "meta").and_then(|meta| present(text_at(meta, "versionId")));
    match meta {
        None => 0.0,
        Some(Value::Number(figure)) => figure.as_f64().unwrap_or(f64::NAN),
        Some(Value::Bool(flag)) => f64::from(u8::from(*flag)),
        Some(other) => crate::js::number_of_text(&js_string(other)),
    }
}

/// A resource's `meta.versionId`, as it is written.
pub fn version_id(resource: &Value) -> Option<String> {
    field(resource, "meta")
        .and_then(|meta| present(text_at(meta, "versionId")))
        .map(js_string)
}

/// A resource's `meta.lastUpdated`.
pub fn last_updated(resource: &Value) -> Option<String> {
    field(resource, "meta")
        .and_then(|meta| present(text_at(meta, "lastUpdated")))
        .map(js_string)
}

/// Text as base64 in UTF-8, as an Attachment's `data` holds it.
pub fn to_base64(text: &str) -> String {
    base64_encode(text.as_bytes())
}

/// An Attachment's base64 `data` as text, none when it is not base64.
pub fn from_base64(data: &str) -> Option<String> {
    base64_decode(data).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}
