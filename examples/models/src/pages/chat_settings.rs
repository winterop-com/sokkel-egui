//! The chat's settings panel, as the web's `pages/ChatSettings.tsx` has it: the library's models
//! grouped by format for a select, what a model is, and the system prompts a person reaches for.

use serde::Serialize;

use super::library::memory_chip;
use crate::data::format::{bytes, context};
use crate::data::model::{FORMATS, Loaded, Model, SamplingPatch};
use crate::data::seed::model_memory;
use crate::engine::generate::rate_of;
use crate::js::{number_to_string, round};

/// The models of one format in a select: each id, name and rounding, under the format.
pub type FormatGroup = (&'static str, Vec<(String, String, String)>);

/// System prompts a person reaches for often.
pub const PRESETS: [&str; 3] = [
    "Answer briefly, in one short paragraph.",
    "You review code. Point at the line, say what is wrong and how to fix it.",
    "You are a helpful assistant. Answer plainly and say when you are not sure.",
];

/// A preset's button: its words up to the first comma or stop.
pub fn preset_label(preset: &str) -> &str {
    preset.split([',', '.']).next().unwrap_or(preset)
}

/// The library's models as a select offers them: each format with its models, an empty one left
/// out, each model by its id with its name and its rounding.
pub fn model_options(models: &[Model]) -> Vec<FormatGroup> {
    FORMATS
        .iter()
        .filter_map(|format| {
            let kept: Vec<(String, String, String)> = models
                .iter()
                .filter(|model| model.format == *format)
                .map(|model| (model.id.clone(), model.name.clone(), model.quant.clone()))
                .collect();
            (!kept.is_empty()).then_some((format.key(), kept))
        })
        .collect()
}

/// What a model is, in the panel: whether it is loaded, its size, the memory it takes, its context
/// and its pace.
pub fn model_facts(model: &Model, loaded: &[Loaded]) -> Vec<(&'static str, String)> {
    vec![
        ("Runner", memory_chip(loaded.iter().find(|one| one.id == model.id))),
        ("Size", bytes(model.size)),
        ("Memory", bytes(model_memory(model))),
        ("Context", format!("{} tokens", context(model.context))),
        (
            "Pace",
            format!("about {} tok/s", number_to_string(round(rate_of(model)))),
        ),
    ]
}

/// The button that puts a model's defaults back, and the change it makes.
pub fn back_to_defaults(model: &Model) -> (String, SamplingPatch) {
    (
        format!("Back to {}'s defaults", model.name),
        SamplingPatch::all(&model.defaults),
    )
}

/// Whether a max tokens typed in the panel is kept.
pub fn max_tokens_kept(value: Option<f64>) -> bool {
    value.is_some_and(|value| value >= 1.0)
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Panel {
    pub options: Vec<FormatGroup>,
    pub facts: Option<Vec<(&'static str, String)>>,
    pub max_hint: Option<String>,
    pub back: Option<String>,
}

pub fn panel(model: Option<&Model>, models: &[Model], loaded: &[Loaded]) -> Panel {
    Panel {
        options: model_options(models),
        facts: model.map(|model| model_facts(model, loaded)),
        max_hint: model.map(|model| format!("up to {}", crate::js::locale(model.context))),
        back: model.map(|model| back_to_defaults(model).0),
    }
}
