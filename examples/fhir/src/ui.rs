//! What every page shares, as the web's `ui.tsx` keeps it: the states resources and versions are in,
//! in the app's words and colors; an instant and a day as the pages write them; a count with its
//! noun; what a resource is called in a list and the state its chip shows; and the editors'
//! settings.

use serde::Serialize;
use serde_json::Value;

use crate::js::{date_utc, intl_date, intl_date_time, intl_number, number_of_text, parse_instant};
use crate::model::{concept_text, field, human_name, id_of, js_string, quantity_text, type_of};

/// A state's color and, where it differs from the state, its label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct StatusStyle {
    pub color: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<&'static str>,
}

/// The states resources and versions are in, in the app's words and colors, for every chip.
pub fn statuses() -> Vec<(&'static str, StatusStyle)> {
    let plain = |color| StatusStyle { color, label: None };
    let named = |color, label| StatusStyle {
        color,
        label: Some(label),
    };
    vec![
        ("active", plain("good")),
        ("draft", plain("info")),
        ("retired", plain("neutral")),
        ("final", plain("good")),
        ("amended", plain("info")),
        ("preliminary", plain("info")),
        ("registered", plain("neutral")),
        ("cancelled", plain("neutral")),
        ("entered-in-error", plain("critical")),
        ("finished", plain("good")),
        ("planned", plain("info")),
        ("in-progress", plain("warning")),
        ("resolved", plain("neutral")),
        ("remission", plain("info")),
        ("inactive", plain("neutral")),
        ("create", named("good", "created")),
        ("update", named("info", "updated")),
        ("delete", named("critical", "deleted")),
    ]
}

/// A number as `Intl.NumberFormat('en-US')` writes it.
pub fn number(value: f64) -> String {
    intl_number(value, 0, 3, true, false)
}

/// An ISO instant as a date and time, in the page's zone (UTC); the text itself when it is not one.
pub fn when(iso: Option<&str>) -> String {
    match iso {
        None | Some("") => String::new(),
        Some(text) => parse_instant(text).map_or_else(|| text.to_owned(), intl_date_time),
    }
}

/// A FHIR date (`2026`, `2026-03` or `2026-03-01`) or the day of a dateTime, as a date.
pub fn on_day(text: Option<&str>) -> String {
    let Some(text) = text.filter(|one| !one.is_empty()) else {
        return String::new();
    };
    let head = crate::js::slice_units(text, 10);
    let parts: Vec<f64> = head.split('-').map(number_of_text).collect();
    let year = parts.first().copied().unwrap_or(0.0);
    let month = parts.get(1).copied().unwrap_or(1.0);
    let day = parts.get(2).copied().unwrap_or(1.0);
    if !(year.is_finite() && month.is_finite() && day.is_finite()) {
        return "Invalid Date".to_owned();
    }
    let days = date_utc(year as i64, month as i64 - 1, day as i64);
    intl_date(days * 86_400_000)
}

/// A count with its noun: the plural `many`, or the noun with an s.
pub fn count(value: usize, one: &str, many: Option<&str>) -> String {
    let noun = if value == 1 {
        one.to_owned()
    } else {
        many.map_or_else(|| format!("{one}s"), str::to_owned)
    };
    format!("{} {noun}", number(value as f64))
}

/// What a resource is called in a list or a title: a person's name, an observation's code and
/// value.
pub fn title_of(resource: &Value) -> String {
    let id = id_of(resource);
    let or_id = |text: String| if text.is_empty() { id.clone() } else { text };
    let get = |name: &str| field(resource, name).cloned().unwrap_or(Value::Null);
    match type_of(resource).as_str() {
        "Patient" | "Practitioner" => or_id(human_name(&get("name"))),
        "Organization" => match field(resource, "name") {
            Some(Value::String(name)) if !name.is_empty() => name.clone(),
            _ => id,
        },
        "Observation" => {
            let mut value = quantity_text(&get("valueQuantity"));
            if value.is_empty() {
                value = match field(resource, "valueString") {
                    Some(Value::String(text)) => text.clone(),
                    _ => concept_text(&get("valueCodeableConcept")),
                };
            }
            let code = concept_text(&get("code"));
            if value.is_empty() {
                or_id(code)
            } else {
                format!("{code} {value}")
            }
        }
        "Condition" => or_id(concept_text(&get("code"))),
        "Encounter" => {
            let kind = match field(resource, "type") {
                Some(Value::Array(items)) => concept_text(items.first().unwrap_or(&Value::Null)),
                _ => String::new(),
            };
            let start = field(resource, "period").and_then(|period| period.get("start"));
            let head = if !kind.is_empty() {
                kind
            } else {
                let class = concept_text(&get("class"));
                if class.is_empty() {
                    "Encounter".to_owned()
                } else {
                    class
                }
            };
            let day = match start {
                None | Some(Value::Null) => String::new(),
                Some(start) => on_day(Some(&js_string(start))),
            };
            [head, day]
                .into_iter()
                .filter(|one| !one.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        }
        "Library" => match (field(resource, "title"), field(resource, "name")) {
            (Some(Value::String(title)), _) => title.clone(),
            (_, Some(Value::String(name))) => name.clone(),
            _ => id,
        },
        _ => id,
    }
}

/// A resource's state for its chip: its status, or a condition's clinical status, or a record's
/// active flag.
pub fn state_of(resource: &Value) -> Option<String> {
    if let Some(Value::String(status)) = field(resource, "status") {
        return Some(status.clone());
    }
    if let Some(clinical) = field(resource, "clinicalStatus") {
        let text = concept_text(clinical);
        return (!text.is_empty()).then_some(text);
    }
    if let Some(Value::Bool(active)) = field(resource, "active") {
        return Some(if *active { "active" } else { "inactive" }.to_owned());
    }
    None
}

/// How the editors are set up, as the settings choose.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Prefs {
    #[serde(rename = "lineNumbers")]
    pub line_numbers: bool,
    pub wrap: bool,
    /// `sm` or `md`.
    pub size: String,
    /// Evaluate FHIRPath as it is typed, or only on Mod-Enter and Evaluate.
    pub live: bool,
    /// How many matches a search page holds when the query gives no `_count`.
    #[serde(rename = "pageSize")]
    pub page_size: f64,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            line_numbers: true,
            wrap: false,
            size: "sm".to_owned(),
            live: true,
            page_size: 20.0,
        }
    }
}
