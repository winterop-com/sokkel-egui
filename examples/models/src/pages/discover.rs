//! The discover page, as the web's `pages/Discover.tsx` has it: what the library holds of each
//! entry, each entry's row, and its variants with the memory each takes.

use serde::Serialize;

use crate::data::catalog::{CatalogEntry, QUANTS, catalog, model_id};
use crate::data::format::{GB, bytes, context, counted, counted_as, params};
use crate::data::model::{Download, SOURCE_IDS};
use crate::data::seed::memory_of;
use crate::js::locale;

/// What the library holds of an entry: this very variant, another of the same model, or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Held {
    Kept,
    Other,
    None,
}

pub fn held(entry: &CatalogEntry, models: &[crate::data::model::Model]) -> Held {
    if entry.variants.iter().any(|variant| {
        models
            .iter()
            .any(|model| model.id == model_id(&entry.name, &variant.quant))
    }) {
        return Held::Kept;
    }
    if models.iter().any(|model| model.name == entry.name) {
        Held::Other
    } else {
        Held::None
    }
}

/// What the library column says of an entry.
pub fn held_said(held: Held) -> &'static str {
    match held {
        Held::Kept => "ready|in library",
        Held::Other => "pending|other copy",
        Held::None => "-",
    }
}

/// The page's count and description.
pub fn header(downloads: &[Download]) -> (String, String, String) {
    let pulling = downloads.iter().filter(|one| one.state.going()).count();
    (
        format!(
            "{} in {} sources",
            counted(catalog().len() as f64, "model"),
            SOURCE_IDS.len()
        ),
        "Search a public hub and the runners on this machine, then pull a model into the library.".to_owned(),
        if pulling > 0 {
            format!("Downloads ({pulling})")
        } else {
            "Downloads".to_owned()
        },
    )
}

/// An entry's row: name, publisher, source, parameters, pulls and what the library holds.
pub fn row(entry: &CatalogEntry, models: &[crate::data::model::Model]) -> Vec<String> {
    vec![
        entry.name.clone(),
        entry.publisher.clone(),
        entry.source.label().to_owned(),
        params(entry.params),
        if entry.downloads == 0.0 {
            "-".to_owned()
        } else {
            locale(entry.downloads)
        },
        held_said(held(entry, models)).to_owned(),
    ]
}

/// What each variant's memory column says against the memory the runner may fill.
pub fn variant_memory(entry: &CatalogEntry, memory: f64) -> Vec<String> {
    entry
        .variants
        .iter()
        .map(|variant| {
            let need = memory_of(variant.size, entry.context, entry.params);
            if need <= memory * GB {
                bytes(need)
            } else {
                "failed|too large".to_owned()
            }
        })
        .collect()
}

/// The sheet's about lines: its size and pulls, its source, and what its roundings mean.
pub fn about(entry: &CatalogEntry) -> [String; 3] {
    let pulls = if entry.downloads > 0.0 {
        format!(
            " Pulled {} times last month, {} likes.",
            locale(entry.downloads),
            locale(entry.likes)
        )
    } else {
        String::new()
    };
    let roundings: Vec<String> = QUANTS
        .iter()
        .filter(|(quant, _)| entry.variants.iter().any(|variant| variant.quant == *quant))
        .map(|(quant, said)| format!("{quant}: {said}."))
        .collect();
    [
        format!(
            "{} parameters, {} tokens of context.{pulls} Changed {} ago.",
            params(entry.params),
            context(entry.context),
            counted_as(entry.updated, "day", "days")
        ),
        format!("{}.", entry.source.detail()),
        roundings.join(" "),
    ]
}
