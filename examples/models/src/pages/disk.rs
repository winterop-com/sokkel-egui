//! The disk usage page, as the web's `pages/Disk.tsx` has it: the whole disk, the models by source
//! and by format, model by model, and the models kept more than once with what keeping only the
//! smallest frees.

use serde::Serialize;

use crate::data::disk::{path_of, usage_of};
use crate::data::format::{GB, bytes, counted};
use crate::data::model::{Download, Model, Source};
use crate::js::fixed;

/// Gigabytes to one place, as the charts read them.
pub fn gb(value: f64) -> f64 {
    fixed(value / GB, 1)
}

/// A tile: its label, value, note, and whether it reads as good.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Stat {
    pub label: String,
    pub value: String,
    pub note: Option<String>,
    pub good: Option<bool>,
}

pub fn stat(label: &str, value: String, note: Option<String>, good: Option<bool>) -> Stat {
    Stat {
        label: label.to_owned(),
        value,
        note,
        good,
    }
}

/// One bar of the whole disk: the disk, the part and its gigabytes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Part {
    pub disk: String,
    pub part: &'static str,
    pub size: f64,
}

/// One model as the charts read it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Piece {
    pub source: &'static str,
    pub format: &'static str,
    pub name: String,
    pub size: f64,
}

/// A model kept in more than one format or rounding: its copies smallest first, and what keeping
/// only the smallest frees.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Copies {
    pub name: String,
    pub kept: Vec<String>,
    pub said: String,
    pub reclaim: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DiskPage {
    pub description: String,
    pub stats: Vec<Stat>,
    pub subtitle: String,
    pub whole: Vec<Part>,
    pub rows: Vec<Piece>,
    pub copies: Vec<Copies>,
    pub copies_meta: Option<String>,
    /// Where models are kept, said under the page.
    pub path: String,
}

pub fn disk_page(root: &str, models: &[Model], downloads: &[Download]) -> DiskPage {
    let usage = usage_of(root, models, downloads);
    let name = usage.disk.name.clone();
    let part = |part, size| Part {
        disk: name.clone(),
        part,
        size: gb(size),
    };
    let whole = vec![
        part("Models", usage.models),
        part("Downloads under way", usage.partial),
        part("Other files", usage.disk.other),
        part("Free", usage.free),
    ];
    let rows = models
        .iter()
        .map(|model| Piece {
            source: model.source.label(),
            format: model.format.key(),
            name: format!("{} {}", model.name, model.quant),
            size: gb(model.size),
        })
        .collect();
    let mut names: Vec<(String, Vec<&Model>)> = Vec::new();
    for model in models {
        match names.iter_mut().find(|(name, _)| *name == model.name) {
            Some((_, kept)) => kept.push(model),
            None => names.push((model.name.clone(), vec![model])),
        }
    }
    let copies: Vec<(Copies, f64)> = names
        .into_iter()
        .filter(|(_, kept)| kept.len() > 1)
        .map(|(name, mut kept)| {
            kept.sort_by(|a, b| a.size.partial_cmp(&b.size).unwrap_or(std::cmp::Ordering::Equal));
            let reclaim = kept[1..].iter().fold(0.0, |sum, model| sum + model.size);
            let said: Vec<String> = kept
                .iter()
                .map(|model| format!("{} {} ({})", model.quant, model.format.key(), bytes(model.size)))
                .collect();
            (
                Copies {
                    name,
                    kept: kept.iter().map(|model| model.id.clone()).collect(),
                    said: said.join(", "),
                    reclaim: bytes(reclaim),
                },
                reclaim,
            )
        })
        .collect();
    let freed = copies.iter().fold(0.0, |sum, (_, reclaim)| sum + reclaim);
    let pulling = downloads.iter().filter(|one| one.state.going()).count();
    DiskPage {
        description: format!("The library's root, {root}, is on {}.", usage.disk.name),
        stats: vec![
            stat(
                "Models",
                bytes(usage.models),
                Some(counted(models.len() as f64, "model")),
                None,
            ),
            stat(
                "Free",
                bytes(usage.free),
                Some(format!("of {}", bytes(usage.disk.capacity))),
                Some(usage.free > 100.0 * GB),
            ),
            stat(
                "Other files",
                bytes(usage.disk.other),
                Some("Not the library's".to_owned()),
                None,
            ),
            stat(
                "Downloads under way",
                bytes(usage.partial),
                Some(counted(pulling as f64, "pull")),
                None,
            ),
        ],
        subtitle: usage.disk.name.clone(),
        whole,
        rows,
        copies_meta: (!copies.is_empty()).then(|| format!("{} to free", bytes(freed))),
        copies: copies.into_iter().map(|(one, _)| one).collect(),
        path: format!(
            "Models are kept under {}.",
            path_of(root, Source::Hub, "<publisher>", "<name>", "<rounding>")
        ),
    }
}
