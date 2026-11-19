//! The compare page, as the web's `pages/Compare.tsx` has it: the draft of models, prompt and
//! settings, the comparison shown with its earlier ones, and each column's answer, from what is
//! being written while it streams, else from what was kept.

use serde::Serialize;

use crate::data::format::{bytes, clock, day};
use crate::data::model::{ColumnState, Comparison, Model, Usage};
use crate::engine::chat::compare_key;
use crate::engine::stream::{Live, Phase, usage_of, writing};
use crate::js::{locale, to_fixed};

/// What the next comparison will ask: its models in order, its prompt and its settings.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub models: Vec<String>,
    pub prompt: String,
    pub temperature: f64,
    pub max_tokens: f64,
}

impl Default for Draft {
    fn default() -> Self {
        Draft {
            models: vec![
                "kelp-3b-q4_k_m".to_owned(),
                "tide-7b-instruct-q4_k_m".to_owned(),
                "reef-8b-q4_k_m".to_owned(),
            ],
            prompt: String::new(),
            temperature: 0.7,
            max_tokens: 512.0,
        }
    }
}

/// One column's answer as it shows.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Answer {
    pub model: String,
    pub detail: String,
    pub state: ColumnState,
    pub reasoning: Option<String>,
    pub content: String,
    pub usage: Option<Usage>,
    pub problem: Option<String>,
    pub removable: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ComparePage {
    pub shown: Option<String>,
    pub count: Option<String>,
    /// The earlier comparisons, newest first, each by its id and when.
    pub earlier: Vec<(String, String)>,
    /// Each chosen model's key, and what its handle says.
    pub keys: Vec<(String, String)>,
    pub can_add: bool,
    pub can_remove: bool,
    pub placeholder: String,
    pub disabled: bool,
    pub ready: bool,
    pub running: bool,
    pub asked: Option<String>,
    pub columns: usize,
    pub answers: Vec<Answer>,
}

/// The comparison an address shows: none for `new`, the newest without one, else the one named.
pub fn shown<'a>(comparisons: &'a [Comparison], id: Option<&str>) -> Option<&'a Comparison> {
    match id {
        Some("new") => None,
        None => newest(comparisons).into_iter().next(),
        Some(id) => comparisons.iter().find(|one| one.id == id),
    }
}

fn newest(comparisons: &[Comparison]) -> Vec<&Comparison> {
    let mut sorted: Vec<&Comparison> = comparisons.iter().collect();
    sorted.sort_by_key(|one| std::cmp::Reverse(one.time));
    sorted
}

/// Each chosen model's key: its id, with how many times it was chosen before when it is chosen
/// twice.
pub fn keys(models: &[String]) -> Vec<String> {
    models
        .iter()
        .enumerate()
        .map(|(index, model)| {
            let before = models[..index].iter().filter(|one| *one == model).count();
            if before == 0 {
                model.clone()
            } else {
                format!("{model}:{before}")
            }
        })
        .collect()
}

pub fn compare_page(
    comparisons: &[Comparison],
    models: &[Model],
    live: &[(String, Live)],
    draft: &Draft,
    id: Option<&str>,
    now: i64,
) -> ComparePage {
    let shown = shown(comparisons, id);
    let running = shown.is_some_and(|one| writing(live, &format!("cmp:{}:", one.id)));
    let picked: Vec<&String> = draft
        .models
        .iter()
        .filter(|model| models.iter().any(|one| one.id == **model))
        .collect();
    let ready = !crate::js::trim(&draft.prompt).is_empty() && picked.len() >= 2;
    let free = models.iter().find(|model| !draft.models.contains(&model.id));
    let keys_now = keys(&draft.models);
    let name_of = |model: &String| match models.iter().find(|one| one.id == *model) {
        Some(found) => format!("{} {}", found.name, found.quant),
        None => model.clone(),
    };
    let answers = shown.map_or_else(Vec::new, |comparison| {
        comparison
            .columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let key = compare_key(&comparison.id, index);
                let current = live.iter().find(|(one, _)| *one == key).map(|(_, live)| live);
                let model = models.iter().find(|one| one.id == column.model);
                let state = match current.map(|live| live.phase) {
                    None => column.state,
                    Some(Phase::Loading | Phase::Waiting) => ColumnState::Waiting,
                    Some(Phase::Streaming) => ColumnState::Streaming,
                    Some(Phase::Done) => ColumnState::Done,
                    Some(Phase::Stopped) => ColumnState::Stopped,
                    Some(Phase::Failed) => ColumnState::Failed,
                };
                let usage = match current {
                    Some(live) if live.first.is_some() => Some(usage_of(live, now)),
                    _ => column.usage.clone(),
                };
                Answer {
                    model: model.map_or_else(|| column.model.clone(), |model| model.name.clone()),
                    detail: match model {
                        None => "not in the library".to_owned(),
                        Some(model) => format!(
                            "{} · {}{}",
                            model.quant,
                            bytes(model.size),
                            if current.is_some_and(|live| live.phase == Phase::Loading) {
                                " · loading"
                            } else {
                                ""
                            }
                        ),
                    },
                    state,
                    reasoning: current.map_or_else(|| column.reasoning.clone(), |live| Some(live.reasoning.clone())),
                    content: current.map_or_else(|| column.text.clone(), |live| live.text.clone()),
                    usage,
                    problem: current
                        .and_then(|live| live.problem.clone())
                        .or_else(|| column.problem.clone()),
                    removable: comparison.columns.len() > 1 && !running,
                }
            })
            .collect()
    });
    ComparePage {
        shown: shown.map(|one| one.id.clone()),
        count: shown.map(|one| format!("{} models", one.columns.len())),
        earlier: newest(comparisons)
            .iter()
            .map(|one| (one.id.clone(), format!("{} {}", day(one.time), clock(one.time))))
            .collect(),
        keys: keys_now
            .iter()
            .zip(&draft.models)
            .map(|(key, model)| (key.clone(), name_of(model)))
            .collect(),
        can_add: draft.models.len() < 4 && free.is_some(),
        can_remove: draft.models.len() > 2,
        placeholder: if picked.len() < 2 {
            "Choose two models first".to_owned()
        } else {
            format!("Prompt for the {} models", picked.len())
        },
        disabled: picked.len() < 2,
        ready,
        running,
        asked: shown.map(|one| {
            format!(
                "{} {}, temperature {}, at most {} tokens",
                day(one.time),
                clock(one.time),
                to_fixed(one.temperature, 2),
                locale(one.max_tokens)
            )
        }),
        columns: shown.map_or(2, |one| match one.columns.len() {
            4.. => 4,
            3 => 3,
            _ => 2,
        }),
        answers,
    }
}

/// The draft with a model added: the first not chosen yet.
pub fn add_model(draft: &Draft, models: &[Model]) -> Draft {
    let mut next = draft.clone();
    if let Some(free) = models.iter().find(|model| !draft.models.contains(&model.id)) {
        next.models.push(free.id.clone());
    }
    next
}
