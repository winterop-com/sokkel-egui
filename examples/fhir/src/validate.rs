//! The server's check of a resource before it keeps it, as the web's `data/validate.ts` makes it:
//! the elements its type knows, each holding what it should, the required ones there, codes from
//! their value sets, and references that point at a resource of an allowed type that the server
//! holds; and the name and version a CQL library declares.

use serde_json::{Map, Value};

use crate::js::{number_to_string, quote};
use crate::model::{Issue, Part, Severity, from_base64, js_string};
use crate::types::{ElementDef, datatype, is_primitive, matches_pattern, type_def};

fn at(path: &[Part], more: impl Into<Part>) -> Vec<Part> {
    let mut out = path.to_vec();
    out.push(more.into());
    out
}

fn check_primitive(kind: &str, value: &Value, path: &[Part], element: &ElementDef, issues: &mut Vec<Issue>) {
    let err = |code: &'static str, message: String| Issue::error(code, Some(path.to_vec()), message);
    if kind == "boolean" {
        if !value.is_boolean() {
            issues.push(err("value", format!("{} must be true or false.", element.name)));
        }
        return;
    }
    if kind == "decimal" || kind == "integer" {
        let whole = value.as_f64().is_some_and(|figure| figure.fract() == 0.0);
        if !value.is_number() || (kind == "integer" && !whole) {
            issues.push(err(
                "value",
                format!(
                    "{} must be {}.",
                    element.name,
                    if kind == "integer" {
                        "a whole number"
                    } else {
                        "a number"
                    }
                ),
            ));
        }
        return;
    }
    let Some(text) = value.as_str() else {
        issues.push(err("value", format!("{} must be a string.", element.name)));
        return;
    };
    if text.is_empty() {
        issues.push(err(
            "value",
            format!("{} is empty; leave it out instead.", element.name),
        ));
        return;
    }
    if !matches_pattern(kind, text) {
        issues.push(err("value", format!("\"{text}\" is not a valid {kind}.")));
    }
    if let Some(codes) = element.codes
        && !codes.contains(&text)
    {
        issues.push(err(
            "code-invalid",
            format!(
                "\"{text}\" is not in the value set of {}: {}.",
                element.name,
                codes.join(", ")
            ),
        ));
    }
}

/// `Type/id` as the server reads a reference: a type and an id of the id's characters.
fn reference_parts(reference: &str) -> Option<(&str, &str)> {
    let (kind, id) = reference.split_once('/')?;
    let mut letters = kind.chars();
    let first = letters.next()?;
    let kind_ok = first.is_ascii_uppercase() && kind.len() >= 2 && letters.all(|c| c.is_ascii_alphabetic());
    let id_ok = (1..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    (kind_ok && id_ok).then_some((kind, id))
}

fn check_reference(
    value: &Map<String, Value>,
    path: &[Part],
    element: &ElementDef,
    exists: &dyn Fn(&str) -> bool,
    issues: &mut Vec<Issue>,
) {
    let Some(reference) = value.get("reference").and_then(Value::as_str) else {
        return;
    };
    let Some((kind, _)) = reference_parts(reference) else {
        issues.push(Issue::error(
            "value",
            Some(at(path, "reference")),
            format!("\"{reference}\" is not a reference of the form Type/id."),
        ));
        return;
    };
    if let Some(targets) = element.targets
        && !targets.contains(&kind)
    {
        issues.push(Issue::error(
            "invalid",
            Some(at(path, "reference")),
            format!("{} may point at {}, not {kind}.", element.name, targets.join(" or ")),
        ));
        return;
    }
    if !exists(reference) {
        issues.push(Issue::error(
            "not-found",
            Some(at(path, "reference")),
            format!("The server holds no {reference}."),
        ));
    }
}

fn check_value(
    element: &ElementDef,
    value: &Value,
    path: &[Part],
    exists: &dyn Fn(&str) -> bool,
    issues: &mut Vec<Issue>,
) {
    if is_primitive(element.kind) {
        check_primitive(element.kind, value, path, element, issues);
        return;
    }
    let Some(fields) = value.as_object() else {
        issues.push(Issue::error(
            "structure",
            Some(path.to_vec()),
            format!("{} must be a {} object.", element.name, element.kind),
        ));
        return;
    };
    check_elements(
        &datatype(element.kind).unwrap_or_default(),
        fields,
        path,
        exists,
        issues,
    );
    if element.kind == "Reference" {
        check_reference(fields, path, element, exists, issues);
    }
}

fn check_elements(
    elements: &[ElementDef],
    value: &Map<String, Value>,
    path: &[Part],
    exists: &dyn Fn(&str) -> bool,
    issues: &mut Vec<Issue>,
) {
    for (name, child) in value {
        let Some(element) = elements.iter().find(|one| one.name == name) else {
            issues.push(Issue::error(
                "structure",
                Some(at(path, name.as_str())),
                format!("Unknown element \"{name}\"."),
            ));
            continue;
        };
        if element.array {
            let Some(items) = child.as_array() else {
                issues.push(Issue::error(
                    "structure",
                    Some(at(path, name.as_str())),
                    format!("{name} repeats, so it must be an array."),
                ));
                continue;
            };
            if items.is_empty() {
                issues.push(Issue::error(
                    "structure",
                    Some(at(path, name.as_str())),
                    format!("{name} is an empty array; leave it out instead."),
                ));
            }
            for (index, item) in items.iter().enumerate() {
                let mut inner = at(path, name.as_str());
                inner.push(Part::Index(index));
                check_value(element, item, &inner, exists, issues);
            }
        } else if child.is_array() {
            issues.push(Issue::error(
                "structure",
                Some(at(path, name.as_str())),
                format!("{name} does not repeat, so it must not be an array."),
            ));
        } else {
            check_value(element, child, &at(path, name.as_str()), exists, issues);
        }
    }
    for element in elements {
        if element.required && !value.contains_key(element.name) {
            issues.push(Issue::error(
                "required",
                Some(path.to_vec()),
                format!("{} is required.", element.name),
            ));
        }
    }
}

/// A value as `JSON.stringify` writes it in a message, `undefined` for one that is not there.
fn shown(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_owned(),
        Some(Value::String(text)) => quote(text),
        Some(Value::Number(figure)) => number_to_string(figure.as_f64().unwrap_or(0.0)),
        Some(other) => crate::js::stringify(other),
    }
}

/// Everything wrong with a resource the server is asked to keep as `kind`, under `id` when it is
/// being updated; `exists` says whether a reference's target is on the server.
pub fn validate(resource: &Value, kind: &str, id: Option<&str>, exists: &dyn Fn(&str) -> bool) -> Vec<Issue> {
    let Some(fields) = resource.as_object() else {
        return vec![Issue::error("structure", None, "A resource is a JSON object.")];
    };
    let Some(def) = type_def(kind) else {
        return vec![Issue::error(
            "not-supported",
            None,
            format!("The server keeps no {kind} resources."),
        )];
    };
    if fields.get("resourceType").and_then(Value::as_str) != Some(kind) {
        return vec![Issue::error(
            "invalid",
            Some(vec![Part::from("resourceType")]),
            format!(
                "resourceType is {}, but this is a {kind}.",
                shown(fields.get("resourceType"))
            ),
        )];
    }
    let mut issues = Vec::new();
    if let Some(id) = id
        && fields.get("id").and_then(Value::as_str) != Some(id)
    {
        issues.push(Issue::error(
            "invalid",
            Some(vec![Part::from("id")]),
            format!("id is {}, but the resource is {kind}/{id}.", shown(fields.get("id"))),
        ));
    }
    check_elements(&def.elements, fields, &[], exists, &mut issues);
    let values: Vec<&String> = fields.keys().filter(|name| name.starts_with("value")).collect();
    if values.len() > 1 {
        issues.push(Issue::error(
            "structure",
            Some(vec![Part::Name(values[1].clone())]),
            format!(
                "Only one value[x] is allowed, and this has {}.",
                values.iter().map(|one| one.as_str()).collect::<Vec<_>>().join(" and ")
            ),
        ));
    }
    if kind == "Observation" && !fields.contains_key("subject") {
        issues.push(Issue::new(
            Severity::Warning,
            "business-rule",
            None,
            "An observation without a subject is about no one.",
        ));
    }
    if kind == "Library" {
        issues.extend(library_issues(fields));
    }
    issues
}

/// A Library's CQL against its own elements: the name and version it declares must be the
/// library's.
fn library_issues(fields: &Map<String, Value>) -> Vec<Issue> {
    let content = fields
        .get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(index) = content
        .iter()
        .position(|one| one.get("contentType").and_then(Value::as_str) == Some("text/cql"))
    else {
        return Vec::new();
    };
    let Some(data) = content[index].get("data").filter(|one| !one.is_null()) else {
        return Vec::new();
    };
    let path = || Some(vec![Part::from("content"), Part::Index(index), Part::from("data")]);
    let Some(text) = from_base64(&js_string(data)) else {
        return vec![Issue::error("value", path(), "The CQL is not valid base64.")];
    };
    let Some(declared) = cql_header(&text) else {
        return vec![Issue::error(
            "invalid",
            path(),
            "The CQL does not start with a library declaration.",
        )];
    };
    let mut issues = Vec::new();
    if let Some(name) = fields.get("name")
        && name.as_str() != Some(declared.name.as_str())
    {
        issues.push(Issue::error(
            "invalid",
            Some(vec![Part::from("name")]),
            format!(
                "The CQL declares the library {}, not {}.",
                declared.name,
                js_string(name)
            ),
        ));
    }
    if let (Some(version), Some(said)) = (fields.get("version"), &declared.version)
        && version.as_str() != Some(said.as_str())
    {
        issues.push(Issue::error(
            "invalid",
            Some(vec![Part::from("version")]),
            format!("The CQL declares version {said}, not {}.", js_string(version)),
        ));
    }
    issues
}

/// The name and version a CQL library declares, and the line it does so on, from 1.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CqlHeader {
    pub name: String,
    pub version: Option<String>,
    pub line: usize,
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `library\s+("[^"]+"|[A-Za-z_]\w*)(?:\s+version\s+'([^']*)')?` at a line's start.
fn declaration(line: &str) -> Option<(String, Option<String>)> {
    let rest = line.strip_prefix("library")?;
    let trimmed = rest.trim_start_matches(crate::js::is_space);
    if trimmed.len() == rest.len() {
        return None;
    }
    let (name, after) = if let Some(quoted) = trimmed.strip_prefix('"') {
        let end = quoted.find('"')?;
        if end == 0 {
            return None;
        }
        (format!("\"{}\"", &quoted[..end]), &quoted[end + 1..])
    } else {
        let first = trimmed.chars().next()?;
        if !(first.is_ascii_alphabetic() || first == '_') {
            return None;
        }
        let end = trimmed.find(|c: char| !is_word(c)).unwrap_or(trimmed.len());
        (trimmed[..end].to_owned(), &trimmed[end..])
    };
    let version = (|| {
        let spaced = after.trim_start_matches(crate::js::is_space);
        if spaced.len() == after.len() {
            return None;
        }
        let rest = spaced.strip_prefix("version")?;
        let spaced = rest.trim_start_matches(crate::js::is_space);
        if spaced.len() == rest.len() {
            return None;
        }
        let quoted = spaced.strip_prefix('\'')?;
        let end = quoted.find('\'')?;
        Some(quoted[..end].to_owned())
    })();
    Some((name.replace('"', ""), version))
}

/// The name and version a CQL library declares on its first line of code.
pub fn cql_header(text: &str) -> Option<CqlHeader> {
    for (index, line) in text.split('\n').enumerate() {
        let trimmed = crate::js::trim(line);
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let (name, version) = declaration(trimmed)?;
        return Some(CqlHeader {
            name,
            version,
            line: index + 1,
        });
    }
    None
}
