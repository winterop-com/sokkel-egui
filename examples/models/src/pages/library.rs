//! The library page's own logic, as the web's `pages/Library.tsx` has it: the fields its filters
//! read, the rows a search and the filters keep, what each column says of a model, and what a row's
//! menu offers. The matching is the library's own (`sokkel_egui::filter_rows`).

use serde::Serialize;
use sokkel_egui::{Filter, FilterField, FilterType, FilterValue, filter_rows};

use crate::data::format::{GB, bytes, context, counted, day, params};
use crate::data::model::{CAPABILITIES, FORMATS, LoadState, Loaded, Model, SOURCE_IDS};
use crate::js::{fixed, iso_day, round, trim, words};

/// The columns a saved view can order and hide.
pub const COLUMN_IDS: [&str; 9] = [
    "name", "quant", "format", "params", "size", "context", "source", "added", "memory",
];

/// One field the library filters on, as the bar offers it: its id, label, type, the values a set
/// offers with their labels, and its unit.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FieldSaid {
    pub id: &'static str,
    pub label: &'static str,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub options: Vec<(String, String)>,
    pub unit: Option<&'static str>,
}

fn sorted_unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    // The web sorts by UTF-16 units, which for these texts is their bytes' order.
    out.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    out
}

/// The fields the library's filter bar offers, read from the models on the disk.
pub fn fields_said(all: &[Model]) -> Vec<FieldSaid> {
    let quants = sorted_unique(all.iter().map(|model| model.quant.clone()));
    let tags = sorted_unique(all.iter().flat_map(|model| model.tags.clone()));
    let plain = |values: Vec<String>| values.into_iter().map(|value| (value.clone(), value)).collect();
    vec![
        FieldSaid {
            id: "format",
            label: "Format",
            kind: "set",
            options: plain(FORMATS.iter().map(|one| one.key().to_owned()).collect()),
            unit: None,
        },
        FieldSaid {
            id: "source",
            label: "Source",
            kind: "set",
            options: SOURCE_IDS
                .iter()
                .map(|one| (one.key().to_owned(), one.label().to_owned()))
                .collect(),
            unit: None,
        },
        FieldSaid {
            id: "capabilities",
            label: "Can do",
            kind: "set",
            options: plain(CAPABILITIES.iter().map(|one| one.key().to_owned()).collect()),
            unit: None,
        },
        FieldSaid {
            id: "quant",
            label: "Rounding",
            kind: "set",
            options: plain(quants),
            unit: None,
        },
        FieldSaid {
            id: "tags",
            label: "Tag",
            kind: "set",
            options: plain(tags),
            unit: None,
        },
        FieldSaid {
            id: "size",
            label: "Size",
            kind: "number",
            options: Vec::new(),
            unit: Some("GB"),
        },
        FieldSaid {
            id: "params",
            label: "Parameters",
            kind: "number",
            options: Vec::new(),
            unit: Some("B"),
        },
        FieldSaid {
            id: "context",
            label: "Context",
            kind: "number",
            options: Vec::new(),
            unit: Some("K tokens"),
        },
        FieldSaid {
            id: "added",
            label: "Added",
            kind: "date",
            options: Vec::new(),
            unit: None,
        },
        FieldSaid {
            id: "name",
            label: "Name",
            kind: "text",
            options: Vec::new(),
            unit: None,
        },
    ]
}

/// The fields as the library's filters read them.
pub fn fields_of<'a>(all: &[Model]) -> Vec<FilterField<'a, Model>> {
    fields_said(all)
        .into_iter()
        .map(|said| {
            let kind = match said.kind {
                "set" => FilterType::Set,
                "number" => FilterType::Number,
                "date" => FilterType::Date,
                _ => FilterType::Text,
            };
            let id = said.id;
            let value = move |model: &Model| -> FilterValue {
                match id {
                    "format" => FilterValue::Set(vec![model.format.key().to_owned()]),
                    "source" => FilterValue::Set(vec![model.source.key().to_owned()]),
                    "capabilities" => {
                        FilterValue::Set(model.capabilities.iter().map(|one| one.key().to_owned()).collect())
                    }
                    "quant" => FilterValue::Set(vec![model.quant.clone()]),
                    "tags" => FilterValue::Set(model.tags.clone()),
                    "size" => FilterValue::Number(fixed(model.size / GB, 2)),
                    "params" => FilterValue::Number(model.params),
                    "context" => FilterValue::Number(round(model.context / 1024.0)),
                    "added" => FilterValue::Day(iso_day(model.added.div_euclid(86_400_000))),
                    _ => FilterValue::Text(model.name.clone()),
                }
            };
            FilterField::new(said.id, said.label, kind, value).options(said.options)
        })
        .collect()
}

/// The models a search and the filters keep, in the library's order.
pub fn shown<'m>(models: &'m [Model], filters: &[Filter], query: &str) -> Vec<&'m Model> {
    let fields = fields_of(models);
    let kept = filter_rows(models, filters, &fields);
    let lowered = query.to_lowercase();
    let wanted = words(trim(&lowered));
    kept.into_iter()
        .filter(|model| {
            let mut said = vec![
                model.name.clone(),
                model.publisher.clone(),
                model.family.clone(),
                model.quant.clone(),
            ];
            said.extend(model.tags.iter().cloned());
            let said = said.join(" ").to_lowercase();
            wanted.iter().all(|word| said.contains(word))
        })
        .collect()
}

/// The page's count: the models and the bytes they take.
pub fn count(models: &[Model]) -> String {
    let total = models.iter().fold(0.0, |sum, model| sum + model.size);
    format!("{} · {}", counted(models.len() as f64, "model"), bytes(total))
}

/// What the runner holds of a model, as its chip says: in memory, loading with its share, or none.
pub fn memory_chip(held: Option<&Loaded>) -> String {
    match held {
        None => "-".to_owned(),
        Some(one) if one.state == LoadState::Ready => "ready|in memory".to_owned(),
        Some(one) => format!(
            "loading|loading {}%",
            crate::js::number_to_string(round(one.progress * 100.0))
        ),
    }
}

/// What each column says of a model, in `COLUMN_IDS`' order.
pub fn cells(model: &Model, loaded: &[Loaded]) -> Vec<String> {
    vec![
        model.name.clone(),
        model.quant.clone(),
        model.format.key().to_owned(),
        params(model.params),
        bytes(model.size),
        context(model.context),
        model.source.label().to_owned(),
        day(model.added),
        memory_chip(loaded.iter().find(|one| one.id == model.id)),
    ]
}

/// The size column's sum under the rows shown.
pub fn size_total(models: &[&Model]) -> String {
    bytes(models.iter().fold(0.0, |sum, model| sum + model.size))
}

/// What a row's menu offers: its id and label, chat with it, load or unload it, delete it.
pub fn model_actions(model: &Model, loaded: &[Loaded]) -> Vec<(&'static str, &'static str)> {
    let held = loaded.iter().any(|one| one.id == model.id);
    vec![
        ("chat", "Chat with it"),
        if held {
            ("unload", "Unload")
        } else {
            ("load", "Load in the runner")
        },
        ("line", ""),
        ("delete", "Delete"),
    ]
}
