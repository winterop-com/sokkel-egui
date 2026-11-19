//! One model's page, as the web's `pages/ModelPage.tsx` has it: its header, its details and where
//! it is kept, what it takes in use, its files largest first, and the defaults its chats start with.

use serde::Serialize;

use super::library::memory_chip;
use crate::data::catalog::quant_note;
use crate::data::disk::model_path;
use crate::data::format::{bytes, context, counted, day, params};
use crate::data::model::{Conversation, FileRole, Loaded, Model, ModelFile, SamplingPatch, default_sampling};
use crate::data::seed::model_memory;
use crate::js::{locale, round};

/// What a file's role says in the files' list.
pub fn role_said(role: FileRole) -> &'static str {
    match role {
        FileRole::Weights => "loads",
        FileRole::Projector => "projector",
        FileRole::Tokenizer => "tokenizer",
        FileRole::Config => "config",
        FileRole::Card => "card",
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelPage {
    pub title: String,
    pub code: String,
    pub description: String,
    /// The details, each a term and what it says.
    pub details: Vec<(String, String)>,
    /// What it takes in use.
    pub in_use: Vec<(String, String)>,
    /// The files' meta, and each file's name, role and size, largest first.
    pub files_meta: String,
    pub files: Vec<(String, String, String)>,
    /// The load button: `load` or `unload`.
    pub load: &'static str,
    /// The max tokens field's hint.
    pub max_hint: String,
}

pub fn model_page(model: &Model, loaded: &[Loaded], conversations: &[Conversation], root: &str) -> ModelPage {
    let held = loaded.iter().find(|one| one.id == model.id);
    let path = model_path(root, model);
    let talks = conversations.iter().filter(|one| one.model == model.id).count();
    let mut files: Vec<&ModelFile> = model.files.iter().collect();
    files.sort_by(|a, b| b.size.partial_cmp(&a.size).unwrap_or(std::cmp::Ordering::Equal));
    let said = |term: &str, value: String| (term.to_owned(), value);
    ModelPage {
        title: model.name.clone(),
        code: format!("{} · {} · {}", model.publisher, model.quant, model.format.key()),
        description: format!(
            "{} parameters · {} · {} context",
            params(model.params),
            bytes(model.size),
            context(model.context)
        ),
        details: vec![
            said("Publisher", model.publisher.clone()),
            said("Family", model.family.clone()),
            said("Parameters", params(model.params)),
            said(
                "Rounding",
                format!(
                    "{}: {}",
                    model.quant,
                    quant_note(&model.quant).unwrap_or("as published")
                ),
            ),
            said("Format", format!("{}: {}", model.format.key(), model.format.note())),
            said("Size", bytes(model.size)),
            said("Context", format!("{} tokens", locale(model.context))),
            said("Source", model.source.label().to_owned()),
            said("Added", day(model.added)),
            said("Path", path),
        ],
        in_use: vec![
            said("Runner", memory_chip(held)),
            said(
                "Memory",
                bytes(held.map_or_else(|| model_memory(model), |one| one.memory)),
            ),
            said(
                "Conversations",
                if talks == 0 {
                    "none".to_owned()
                } else {
                    counted(talks as f64, "conversation")
                },
            ),
            said(
                "Last used",
                model
                    .used
                    .map_or_else(|| "never".to_owned(), |used| day(round(used) as i64)),
            ),
        ],
        files_meta: format!("{} · {}", files.len(), bytes(model.size)),
        files: files
            .iter()
            .map(|file| {
                let role = if file.role == FileRole::Weights {
                    format!("ready|{}", role_said(file.role))
                } else {
                    role_said(file.role).to_owned()
                };
                (file.name.clone(), role, bytes(file.size))
            })
            .collect(),
        load: if held.is_none() { "load" } else { "unload" },
        max_hint: format!("up to {}", locale(model.context)),
    }
}

/// The change Reset to the app's defaults makes to a model's defaults.
pub fn reset_defaults() -> SamplingPatch {
    SamplingPatch::all(&default_sampling())
}

/// Whether a max tokens typed in the defaults is kept: a whole count within the context.
pub fn max_tokens_kept(value: Option<f64>, model: &Model) -> bool {
    value.is_some_and(|value| value >= 1.0 && value <= model.context)
}
