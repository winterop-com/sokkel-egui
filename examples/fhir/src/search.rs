//! FHIR search on the simulated server, as the web's `data/search.ts` reads it: `Type?param=value&...`
//! as the specification reads it. Each search parameter is defined, as FHIR defines it, by a
//! FHIRPath expression that picks the values it matches from a resource; the parameter types
//! string, token, date, reference, quantity and uri are matched with their modifiers (`:exact`,
//! `:contains`, `:not`, `:text`, `:missing`) and prefixes (`eq`, `ne`, `gt`, `lt`, `ge`, `le`,
//! `sa`, `eb`); a comma is an or, a repeated parameter an and. `_count`, `_offset`, `_sort` and
//! `_summary=count` shape the result, a searchset Bundle.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Value, json};

use crate::fhirpath::{evaluate, parse, to_json};
use crate::js::{fold, locale_compare, number_of_text, number_to_string, trim};
use crate::model::{Issue, Severity, js_string, key_of, type_of};
use crate::server::{SERVER_BASE, Stored};
use crate::types::TYPE_NAMES;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    String,
    Token,
    Date,
    Reference,
    Quantity,
    Uri,
}

impl ParamType {
    pub fn key(self) -> &'static str {
        match self {
            ParamType::String => "string",
            ParamType::Token => "token",
            ParamType::Date => "date",
            ParamType::Reference => "reference",
            ParamType::Quantity => "quantity",
            ParamType::Uri => "uri",
        }
    }
}

/// A search parameter: its name, its type, the FHIRPath expression that picks its values, and what
/// it matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SearchParam {
    pub name: &'static str,
    #[serde(rename = "type")]
    pub kind: ParamType,
    pub expression: &'static str,
    pub doc: &'static str,
}

const fn param(name: &'static str, kind: ParamType, expression: &'static str, doc: &'static str) -> SearchParam {
    SearchParam {
        name,
        kind,
        expression,
        doc,
    }
}

use ParamType::{Date, Quantity, Reference, String as Text, Token, Uri};

const COMMON: [SearchParam; 2] = [
    param("_id", Token, "Resource.id", "The logical id of the resource."),
    param(
        "_lastUpdated",
        Date,
        "Resource.meta.lastUpdated",
        "When the resource last changed.",
    ),
];

/// The search parameters each type takes, beside the common ones.
fn by_type(kind: &str) -> Vec<SearchParam> {
    match kind {
        "Patient" => vec![
            param(
                "name",
                Text,
                "Patient.name",
                "Any part of a name: given, family, prefix or text.",
            ),
            param("family", Text, "Patient.name.family", "The family name."),
            param("given", Text, "Patient.name.given", "A given name."),
            param("gender", Token, "Patient.gender", "male, female, other or unknown."),
            param("birthdate", Date, "Patient.birthDate", "The date of birth."),
            param("address-city", Text, "Patient.address.city", "The city of an address."),
            param(
                "identifier",
                Token,
                "Patient.identifier",
                "A patient number, as system|value.",
            ),
            param("active", Token, "Patient.active", "Whether the record is in use."),
            param(
                "general-practitioner",
                Reference,
                "Patient.generalPractitioner",
                "The patient\u{2019}s own practitioner.",
            ),
            param(
                "organization",
                Reference,
                "Patient.managingOrganization",
                "The clinic that keeps the record.",
            ),
        ],
        "Practitioner" => vec![
            param("name", Text, "Practitioner.name", "Any part of a name."),
            param("family", Text, "Practitioner.name.family", "The family name."),
            param(
                "gender",
                Token,
                "Practitioner.gender",
                "male, female, other or unknown.",
            ),
            param(
                "identifier",
                Token,
                "Practitioner.identifier",
                "A staff number, as system|value.",
            ),
            param("active", Token, "Practitioner.active", "Whether the record is in use."),
        ],
        "Organization" => vec![
            param("name", Text, "Organization.name", "The name of the organization."),
            param(
                "address-city",
                Text,
                "Organization.address.city",
                "The city of an address.",
            ),
            param("active", Token, "Organization.active", "Whether the record is in use."),
        ],
        "Encounter" => vec![
            param("status", Token, "Encounter.status", "Where the encounter stands."),
            param(
                "class",
                Token,
                "Encounter.class",
                "Ambulatory, inpatient, virtual and the like.",
            ),
            param("date", Date, "Encounter.period.start", "When it started."),
            param("subject", Reference, "Encounter.subject", "The patient."),
            param("patient", Reference, "Encounter.subject", "The patient."),
            param(
                "practitioner",
                Reference,
                "Encounter.participant.individual",
                "A practitioner who took part.",
            ),
            param(
                "service-provider",
                Reference,
                "Encounter.serviceProvider",
                "The clinic that gave the care.",
            ),
        ],
        "Observation" => vec![
            param("code", Token, "Observation.code", "What was observed, as system|code."),
            param(
                "category",
                Token,
                "Observation.category",
                "The kind, such as vital-signs.",
            ),
            param("status", Token, "Observation.status", "Where the result stands."),
            param("subject", Reference, "Observation.subject", "Who it is about."),
            param("patient", Reference, "Observation.subject", "The patient it is about."),
            param(
                "encounter",
                Reference,
                "Observation.encounter",
                "The encounter it was made in.",
            ),
            param("date", Date, "Observation.effective", "When it was observed."),
            param(
                "value-quantity",
                Quantity,
                "Observation.value.ofType(Quantity)",
                "The value, such as gt140.",
            ),
            param("performer", Reference, "Observation.performer", "Who is responsible."),
        ],
        "Condition" => vec![
            param("code", Token, "Condition.code", "The condition, as system|code."),
            param(
                "clinical-status",
                Token,
                "Condition.clinicalStatus",
                "active, recurrence, remission or resolved.",
            ),
            param("subject", Reference, "Condition.subject", "Who has the condition."),
            param("patient", Reference, "Condition.subject", "The patient who has it."),
            param(
                "onset-date",
                Date,
                "Condition.onset.ofType(dateTime)",
                "When it started.",
            ),
            param(
                "recorded-date",
                Date,
                "Condition.recordedDate",
                "When it was first recorded.",
            ),
        ],
        "Library" => vec![
            param("name", Text, "Library.name", "The name for machines."),
            param("title", Text, "Library.title", "The name for people."),
            param("version", Token, "Library.version", "The business version."),
            param("status", Token, "Library.status", "draft, active or retired."),
            param("url", Uri, "Library.url", "The canonical address."),
            param("description", Text, "Library.description", "Words in the description."),
        ],
        _ => Vec::new(),
    }
}

/// The result parameters every search takes.
pub const RESULT_PARAMS: [SearchParam; 4] = [
    param("_count", Token, "", "How many to return in one page."),
    param("_offset", Token, "", "How many matches to skip, for the next page."),
    param("_sort", Token, "", "Parameters to sort by, a leading - for descending."),
    param("_summary", Token, "", "count returns only how many match."),
];

/// The search parameters a type takes, its own first.
pub fn params_of(kind: &str) -> Vec<SearchParam> {
    let mut out = by_type(kind);
    out.extend(COMMON);
    out
}

/// The values a parameter's expression picks from a resource.
fn values_of(param: &SearchParam, resource: &Value) -> Vec<Value> {
    let expression = match param.expression.strip_prefix("Resource.") {
        Some(rest) => format!("{}.{rest}", type_of(resource)),
        None => param.expression.to_owned(),
    };
    let Ok(expr) = parse(&expression) else {
        return Vec::new();
    };
    evaluate(&expr, resource, &BTreeMap::new(), 0)
        .map(|items| to_json(&items))
        .unwrap_or_default()
}

/// The strings a string parameter matches against in a value: a name's or an address's parts, or
/// the text.
fn strings_of(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) => vec![text.clone()],
        Value::Object(fields) => fields
            .iter()
            .filter(|(name, _)| {
                [
                    "family",
                    "given",
                    "text",
                    "prefix",
                    "suffix",
                    "line",
                    "city",
                    "postalCode",
                    "country",
                ]
                .contains(&name.as_str())
            })
            .flat_map(|(_, part)| match part {
                Value::Array(items) => items.clone(),
                other => vec![other.clone()],
            })
            .filter_map(|part| part.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

/// A system and code pair a token parameter matches against, with the text it is shown as.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TokenValue {
    system: Option<String>,
    code: String,
    text: Option<String>,
}

fn tokens_of(value: &Value) -> Vec<TokenValue> {
    match value {
        Value::String(_) | Value::Bool(_) | Value::Number(_) => vec![TokenValue {
            system: None,
            code: js_string(value),
            text: None,
        }],
        Value::Object(fields) => {
            if let Some(Value::Array(coding)) = fields.get("coding") {
                let text = fields.get("text").and_then(Value::as_str).map(str::to_owned);
                return coding
                    .iter()
                    .flat_map(|one| {
                        tokens_of(one).into_iter().map(|token| TokenValue {
                            text: text.clone().or(token.text.clone()),
                            ..token
                        })
                    })
                    .collect();
            }
            let code = fields
                .get("code")
                .filter(|one| !one.is_null())
                .or_else(|| fields.get("value"));
            let Some(Value::String(code)) = code else {
                return Vec::new();
            };
            vec![TokenValue {
                system: fields.get("system").and_then(Value::as_str).map(str::to_owned),
                code: code.clone(),
                text: fields.get("display").and_then(Value::as_str).map(str::to_owned),
            }]
        }
        _ => Vec::new(),
    }
}

const PREFIXES: [&str; 8] = ["eq", "ne", "gt", "lt", "ge", "le", "sa", "eb"];

/// A value's prefix and the rest: `eq` when it has none.
fn prefixed(text: &str) -> (&'static str, String) {
    let units: Vec<u16> = text.encode_utf16().collect();
    let head = String::from_utf16_lossy(&units[..units.len().min(2)]);
    let rest = String::from_utf16_lossy(&units[units.len().min(2)..]);
    if let Some(prefix) = PREFIXES.iter().find(|one| **one == head)
        && rest.starts_with(|c: char| c.is_ascii_digit() || c == '-')
    {
        return (prefix, rest);
    }
    ("eq", text.to_owned())
}

fn compare(order: Ordering, prefix: &str) -> bool {
    match prefix {
        "eq" => order == Ordering::Equal,
        "ne" => order != Ordering::Equal,
        "gt" | "sa" => order == Ordering::Greater,
        "lt" | "eb" => order == Ordering::Less,
        "ge" => order != Ordering::Less,
        _ => order != Ordering::Greater,
    }
}

/// `^\d{4}(-\d{2}(-\d{2}(T\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:\d{2})?)?)?)?$`.
fn is_search_date(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |from: usize, n: usize| (from..from + n).all(|k| b.get(k).is_some_and(u8::is_ascii_digit));
    if !digits(0, 4) {
        return false;
    }
    let mut at = 4;
    if at == b.len() {
        return true;
    }
    if !(b[at] == b'-' && digits(at + 1, 2)) {
        return false;
    }
    at += 3;
    if at == b.len() {
        return true;
    }
    if !(b[at] == b'-' && digits(at + 1, 2)) {
        return false;
    }
    at += 3;
    if at == b.len() {
        return true;
    }
    if !(b[at] == b'T' && digits(at + 1, 2) && b.get(at + 3) == Some(&b':') && digits(at + 4, 2)) {
        return false;
    }
    at += 6;
    if b.get(at) == Some(&b':') && digits(at + 1, 2) {
        at += 3;
        if b.get(at) == Some(&b'.') && digits(at + 1, 1) {
            at += 1;
            while b.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
        }
    }
    if at == b.len() {
        return true;
    }
    match b[at] {
        b'Z' => at + 1 == b.len(),
        b'+' | b'-' => digits(at + 1, 2) && b.get(at + 3) == Some(&b':') && digits(at + 4, 2) && at + 6 == b.len(),
        _ => false,
    }
}

/// Whether one value matches one of a parameter's alternatives.
fn match_one(param: &SearchParam, modifier: Option<&str>, wanted: &str, value: &Value) -> bool {
    match param.kind {
        ParamType::String => {
            let asked = fold(wanted);
            strings_of(value).iter().any(|text| {
                let have = fold(text);
                match modifier {
                    Some("exact") => text == wanted,
                    Some("contains") => have.contains(&asked),
                    _ => have.starts_with(&asked),
                }
            })
        }
        ParamType::Uri => value.as_str() == Some(wanted),
        ParamType::Token => {
            let (system, code) = match wanted.find('|') {
                Some(bar) => (Some(&wanted[..bar]), &wanted[bar + 1..]),
                None => (None, wanted),
            };
            tokens_of(value).iter().any(|token| {
                if modifier == Some("text") {
                    return fold(token.text.as_deref().unwrap_or(&token.code)).contains(&fold(wanted));
                }
                let system_ok = match system {
                    None => true,
                    Some("") => token.system.is_none(),
                    Some(system) => token.system.as_deref() == Some(system),
                };
                system_ok && (code.is_empty() || token.code == code)
            })
        }
        ParamType::Reference => {
            let Some(target) = value.get("reference").and_then(Value::as_str) else {
                return false;
            };
            if wanted.contains('/') {
                target == wanted
            } else {
                target.ends_with(&format!("/{wanted}"))
            }
        }
        ParamType::Date => {
            let Some(text) = value.as_str() else {
                return false;
            };
            let (prefix, rest) = prefixed(wanted);
            let have = crate::js::slice_units(text, rest.encode_utf16().count());
            compare(locale_compare(&have, &rest, false), prefix)
        }
        ParamType::Quantity => {
            let Some(amount) = value.get("value").and_then(Value::as_f64) else {
                return false;
            };
            let parts: Vec<&str> = wanted.split('|').collect();
            let (prefix, rest) = prefixed(parts[0]);
            if let Some(unit) = parts.get(2).filter(|unit| !unit.is_empty()) {
                let code = value.get("code").and_then(Value::as_str);
                let shown = value.get("unit").and_then(Value::as_str);
                if code != Some(*unit) && shown != Some(*unit) {
                    return false;
                }
            }
            let difference = amount - number_of_text(&rest);
            let order = if difference > 0.0 {
                Ordering::Greater
            } else if difference < 0.0 {
                Ordering::Less
            } else if difference == 0.0 {
                Ordering::Equal
            } else {
                return compare_nan(prefix);
            };
            compare(order, prefix)
        }
    }
}

/// How a prefix reads a comparison with a number that is not one: `Math.sign(NaN)` is NaN, which
/// no order is but `ne` holds for.
fn compare_nan(prefix: &str) -> bool {
    prefix == "ne"
}

/// Why one alternative of a parameter cannot be read, or none when it can.
fn bad_value(param: &SearchParam, wanted: &str) -> Option<String> {
    if wanted.is_empty() {
        return Some(format!("{} is empty.", param.name));
    }
    if param.kind == ParamType::Date && !is_search_date(&prefixed(wanted).1) {
        return Some(format!(
            "\"{wanted}\" is not a date; write it as 2026-03-01, with a prefix such as ge if wanted."
        ));
    }
    if param.kind == ParamType::Quantity
        && !number_of_text(&prefixed(wanted.split('|').next().unwrap_or("")).1).is_finite()
    {
        return Some(format!(
            "\"{wanted}\" is not a quantity; write it as a number, with a prefix such as gt and |system|unit if wanted."
        ));
    }
    None
}

/// A search's answer: the matches on the page asked for, as the server read the query.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SearchResult {
    #[serde(rename = "type")]
    pub kind: String,
    /// The query as the server read it, its parameters in the order given.
    pub query: String,
    pub total: usize,
    pub matches: Vec<Stored>,
    /// What the server could not read and so left out.
    pub issues: Vec<Issue>,
    pub bundle: Value,
    pub offset: f64,
    pub count: f64,
}

/// A query's type and its parameters, from `Patient?gender=female`, `/Patient?...` or a whole
/// address.
pub fn read_query(text: &str) -> (String, Vec<(String, String)>) {
    let trimmed = trim(text).replacen(SERVER_BASE, "", 1);
    let trimmed = trimmed.trim_start_matches('/');
    let (kind, search) = match trimmed.find('?') {
        Some(at) => (&trimmed[..at], &trimmed[at + 1..]),
        None => (trimmed, ""),
    };
    (trim(kind).to_owned(), crate::js::search_params(search))
}

/// The value a row sorts by for a parameter: a quantity's figure, else its first text or code.
#[derive(Clone, Debug, PartialEq)]
enum SortValue {
    Number(f64),
    Text(String),
}

fn sort_value(param: Option<&SearchParam>, row: &Stored) -> SortValue {
    let Some(param) = param else {
        return SortValue::Text(String::new());
    };
    let first = values_of(param, &row.resource).into_iter().next();
    if param.kind == ParamType::Quantity
        && let Some(amount) = first.as_ref().and_then(|one| one.get("value")).and_then(Value::as_f64)
    {
        return SortValue::Number(amount);
    }
    SortValue::Text(match first {
        None => String::new(),
        Some(value @ (Value::Object(_) | Value::Array(_))) => strings_of(&value)
            .into_iter()
            .next()
            .or_else(|| tokens_of(&value).into_iter().next().map(|one| one.code))
            .unwrap_or_default(),
        Some(other) => js_string(&other),
    })
}

fn pairs_text(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// Runs a search against the server's resources: the matches, ordered and paged, as a searchset
/// Bundle.
pub fn search(all: &[Stored], text: &str, page_size: f64) -> SearchResult {
    let (kind, params) = read_query(text);
    let mut issues = Vec::new();
    if !TYPE_NAMES.contains(&kind.as_str()) {
        issues.push(Issue::new(
            Severity::Error,
            "not-supported",
            None,
            if kind.is_empty() {
                "Name a resource type first, as in Patient?name=ada.".to_owned()
            } else {
                format!(
                    "The server keeps no {kind} resources; it keeps {}.",
                    TYPE_NAMES.join(", ")
                )
            },
        ));
    }
    let known = params_of(&kind);
    let mut rows: Vec<Stored> = all.iter().filter(|row| row.kind == kind).cloned().collect();
    let mut count = page_size;
    let mut offset = 0.0_f64;
    let mut sort: Vec<String> = Vec::new();
    let mut summary = false;
    let mut used: Vec<(String, String)> = Vec::new();
    for (key, value) in &params {
        let mut split = key.split(':');
        let name = split.next().unwrap_or("");
        let modifier = split.next();
        match name {
            "_count" => {
                let n = number_of_text(value);
                if n.is_finite() && n.fract() == 0.0 && n >= 0.0 {
                    count = n.min(500.0);
                    used.push((key.clone(), value.clone()));
                } else {
                    issues.push(Issue::new(
                        Severity::Warning,
                        "value",
                        None,
                        format!("_count takes a whole number, not \"{value}\"; it was left out."),
                    ));
                }
                continue;
            }
            "_offset" => {
                let n = number_of_text(value);
                offset = if n.is_nan() { 0.0 } else { n.max(0.0) };
                if offset == 0.0 {
                    offset = 0.0;
                }
                used.push((key.clone(), value.clone()));
                continue;
            }
            "_sort" => {
                sort = value
                    .split(',')
                    .filter(|one| !one.is_empty())
                    .map(str::to_owned)
                    .collect();
                let unknown: Vec<String> = sort
                    .iter()
                    .filter(|one| {
                        let bare = one.strip_prefix('-').unwrap_or(one);
                        !known.iter().any(|param| param.name == bare)
                    })
                    .cloned()
                    .collect();
                for one in &unknown {
                    issues.push(Issue::new(
                        Severity::Warning,
                        "not-supported",
                        None,
                        format!(
                            "{kind} has no parameter {} to sort by; it was left out.",
                            one.strip_prefix('-').unwrap_or(one)
                        ),
                    ));
                }
                sort.retain(|one| !unknown.contains(one));
                used.push((key.clone(), value.clone()));
                continue;
            }
            "_summary" => {
                summary = value == "count";
                used.push((key.clone(), value.clone()));
                continue;
            }
            _ => {}
        }
        let Some(param) = known.iter().find(|one| one.name == name) else {
            issues.push(Issue::new(
                Severity::Warning,
                "not-supported",
                None,
                format!("{kind} has no search parameter {name}; it was left out."),
            ));
            continue;
        };
        if let Some(modifier) = modifier
            && !["missing", "exact", "contains", "not", "text"].contains(&modifier)
        {
            issues.push(Issue::new(
                Severity::Warning,
                "not-supported",
                None,
                format!("The modifier :{modifier} is not supported; {key} was left out."),
            ));
            continue;
        }
        if modifier == Some("missing") {
            let missing = value == "true";
            rows.retain(|row| values_of(param, &row.resource).is_empty() == missing);
            used.push((key.clone(), value.clone()));
            continue;
        }
        let alternatives: Vec<&str> = value.split(',').collect();
        if let Some(bad) = alternatives.iter().find_map(|one| bad_value(param, one)) {
            issues.push(Issue::new(
                Severity::Error,
                "value",
                None,
                format!("{bad} {key} was left out."),
            ));
            continue;
        }
        rows.retain(|row| {
            let values = values_of(param, &row.resource);
            let hit = alternatives
                .iter()
                .any(|wanted| values.iter().any(|one| match_one(param, modifier, wanted, one)));
            if modifier == Some("not") { !hit } else { hit }
        });
        used.push((key.clone(), value.clone()));
    }
    if !sort.is_empty() {
        let keys: Vec<(Option<&SearchParam>, bool)> = sort
            .iter()
            .map(|one| {
                let bare = one.strip_prefix('-').unwrap_or(one);
                (known.iter().find(|param| param.name == bare), one.starts_with('-'))
            })
            .collect();
        let mut keyed: Vec<(Vec<SortValue>, Stored)> = rows
            .into_iter()
            .map(|row| (keys.iter().map(|(param, _)| sort_value(*param, &row)).collect(), row))
            .collect();
        keyed.sort_by(|(left, _), (right, _)| {
            for ((one, other), (_, down)) in left.iter().zip(right).zip(&keys) {
                let order = match (one, other) {
                    (SortValue::Number(a), SortValue::Number(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                    _ => {
                        let text = |value: &SortValue| match value {
                            SortValue::Number(n) => number_to_string(*n),
                            SortValue::Text(text) => text.clone(),
                        };
                        locale_compare(&text(one), &text(other), true)
                    }
                };
                if order != Ordering::Equal {
                    return if *down { order.reverse() } else { order };
                }
            }
            Ordering::Equal
        });
        rows = keyed.into_iter().map(|(_, row)| row).collect();
    }
    let total = rows.len();
    let start = (offset.trunc() as usize).min(total);
    let end = ((offset + count).trunc().max(0.0) as usize).clamp(start, total);
    let page: Vec<Stored> = if summary { Vec::new() } else { rows[start..end].to_vec() };
    let query = if used.is_empty() {
        kind.clone()
    } else {
        format!("{kind}?{}", pairs_text(&used))
    };
    let link = |at: f64| {
        let mut shown: Vec<(String, String)> = used.iter().filter(|(key, _)| key != "_offset").cloned().collect();
        if at != 0.0 {
            shown.push(("_offset".to_owned(), number_to_string(at)));
        }
        if shown.is_empty() {
            format!("{SERVER_BASE}/{kind}")
        } else {
            format!("{SERVER_BASE}/{kind}?{}", pairs_text(&shown))
        }
    };
    let mut links = vec![json!({ "relation": "self", "url": link(offset) })];
    if !summary && count > 0.0 && offset + count < total as f64 {
        links.push(json!({ "relation": "next", "url": link(offset + count) }));
    }
    if !summary && offset > 0.0 {
        links.push(json!({ "relation": "previous", "url": link((offset - count).max(0.0)) }));
    }
    let mut entry: Vec<Value> = page
        .iter()
        .map(|row| {
            json!({
                "fullUrl": format!("{SERVER_BASE}/{}", key_of(&row.resource)),
                "resource": row.resource,
                "search": { "mode": "match" },
            })
        })
        .collect();
    if !issues.is_empty() {
        entry.push(json!({
            "resource": {
                "resourceType": "OperationOutcome",
                "issue": issues.iter().map(|issue| json!({ "severity": issue.severity.key(), "code": issue.code, "diagnostics": issue.message })).collect::<Vec<_>>(),
            },
            "search": { "mode": "outcome" },
        }));
    }
    let mut bundle = json!({
        "resourceType": "Bundle",
        "id": "search",
        "type": "searchset",
        "total": total,
        "link": links,
        "entry": entry,
    });
    if bundle["entry"].as_array().is_some_and(Vec::is_empty)
        && let Some(fields) = bundle.as_object_mut()
    {
        fields.shift_remove("entry");
    }
    SearchResult {
        kind,
        query,
        total,
        matches: page,
        issues,
        bundle,
        offset,
        count,
    }
}
