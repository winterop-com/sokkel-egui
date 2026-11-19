//! What the demo starts with, as the web's `data/seed.ts` makes it: a library of fifteen models,
//! four conversations, a comparison and a quiet runner.
//!
//! The web's `newId` counts the ids it makes in the module; here the count is the backend's
//! (`Ids`), handed in, so the seed and the app go on from the same number.

use super::catalog::{CatalogEntry, Variant, catalog, files_of, model_id, quant_note};
use super::format::{bytes, context, params};
use super::model::{
    AssistantMessage, Capability, ColumnState, CompareColumn, Comparison, Conversation, Level, LoadState, Loaded,
    LogEvent, LogSource, Message, Model, ReplyState, Runner, RunnerStatus, Sampling, Source, Usage, UserMessage,
    Version, default_sampling,
};
use crate::engine::generate::{Ask, plan, tokens};
use crate::js::{base36, fixed, round};

pub const DAY: i64 = 86_400_000;
pub const HOUR: i64 = 3_600_000;
pub const MINUTE: i64 = 60_000;

/// The ids `newId` makes: a prefix, the moment in base 36 and a count in base 36.
#[derive(Clone, Debug, Default)]
pub struct Ids {
    pub made: i64,
}

impl Ids {
    pub fn next(&mut self, prefix: &str, now: i64) -> String {
        self.made += 1;
        format!("{prefix}-{}-{}", base36(now), base36(self.made))
    }
}

/// A model's card, as its publisher would write it.
pub fn card_of(entry: &CatalogEntry, variant: &Variant) -> String {
    let tags: Vec<String> = entry.tags.iter().map(|tag| format!("- {tag}")).collect();
    format!(
        "# {name}

{summary}

| | |
| --- | --- |
| Parameters | {params} |
| Context | {context} tokens |
| Rounding | `{quant}`: {said} |
| Format | `{format}` |
| Size | {size} |
| Published by | {publisher} |

## Use

Load it in the runner, then ask it over the runner's API:

```bash
curl http://runner.example.org:8400/v1/chat \\
  -d '{{\"model\": \"{name}\", \"messages\": [{{\"role\": \"user\", \"content\": \"Hello\"}}]}}'
```

## Good at

{tags}

> [!NOTE]
> Answers can be wrong. Check anything that matters before acting on it.
",
        name = entry.name,
        summary = entry.summary,
        params = params(entry.params),
        context = context(entry.context),
        quant = variant.quant,
        said = quant_note(&variant.quant).unwrap_or("as published"),
        format = variant.format.key(),
        size = bytes(variant.size),
        publisher = entry.publisher,
        tags = tags.join("\n"),
    )
}

/// A library model made from a catalog entry's variant, as a pull or a copy lands it.
pub fn model_of(entry: &CatalogEntry, variant: &Variant, source: Source, added: i64) -> Model {
    let defaults = Sampling {
        temperature: if entry.capabilities.contains(&Capability::Code) {
            0.2
        } else if entry.capabilities.contains(&Capability::Reasoning) {
            0.6
        } else {
            0.7
        },
        max_tokens: if entry.params < 2.0 { 512.0 } else { 1024.0 },
        ..default_sampling()
    };
    let files = files_of(entry, variant);
    Model {
        id: model_id(&entry.name, &variant.quant),
        name: entry.name.clone(),
        publisher: entry.publisher.clone(),
        family: entry.family.clone(),
        params: entry.params,
        quant: variant.quant.clone(),
        format: variant.format,
        size: files.iter().map(|file| file.size).sum(),
        context: entry.context,
        source,
        added,
        used: None,
        capabilities: entry.capabilities.clone(),
        tags: entry.tags.clone(),
        files,
        defaults,
        card: card_of(entry, variant),
    }
}

/// The models kept as the demo opens: the catalog entry, its rounding, days since it was added and
/// hours since it last answered.
const KEPT: [(&str, &str, f64, Option<f64>); 15] = [
    ("ada/tide-7b-instruct", "Q4_K_M", 41.0, Some(0.2)),
    ("ada/tide-14b-instruct", "Q4_K_M", 33.0, Some(26.0)),
    ("runner/kelp-3b", "Q4_K_M", 8.0, Some(3.0)),
    ("grace/kelp-1b", "Q8_0", 29.0, None),
    ("desktop/brine-coder-14b", "4bit", 18.0, Some(1.5)),
    ("linus/brine-coder-14b", "BF16", 12.0, Some(70.0)),
    ("linus/brine-coder-3b", "Q5_K_M", 25.0, Some(30.0)),
    ("margaret/reef-8b", "Q4_K_M", 15.0, Some(5.0)),
    ("margaret/reef-32b", "Q3_K_S", 21.0, Some(200.0)),
    ("desktop/drift-vision-11b", "Q4_K_M", 18.0, Some(50.0)),
    ("linus/shoal-0.5b", "Q8_0", 60.0, None),
    ("ada/eddy-22b", "4bit", 45.0, Some(400.0)),
    ("margaret/skerry-9b", "Q5_K_M", 70.0, Some(100.0)),
    ("runner/cove-embed-0.3b", "F16", 40.0, None),
    ("desktop/pebble-2b", "4bit", 90.0, Some(900.0)),
];

pub fn seed_models(now: i64) -> Vec<Model> {
    KEPT.iter()
        .map(|(id, quant, days_ago, used_hours_ago)| {
            let entry = catalog()
                .iter()
                .find(|one| one.id == *id)
                .expect("the kept models are in the catalog");
            let variant = entry
                .variants
                .iter()
                .find(|one| one.quant == *quant)
                .expect("the kept roundings are in the catalog");
            let added = (now as f64 - days_ago * DAY as f64 - 7.0 * HOUR as f64) as i64;
            let mut model = model_of(entry, variant, entry.source, added);
            model.used = used_hours_ago.map(|hours| now as f64 - hours * HOUR as f64);
            model
        })
        .collect()
}

/// The memory a model takes once loaded: its weights and a working context.
pub fn memory_of(size: f64, context: f64, params: f64) -> f64 {
    round(size * 1.06 + context.min(8192.0) * params * 2_000.0 + 220_000_000.0)
}

/// The memory a library model takes once loaded.
pub fn model_memory(model: &Model) -> f64 {
    memory_of(model.size, model.context, model.params)
}

/// The models the runner holds as the demo opens: one in use a moment ago, one a little before.
pub fn seed_loaded(library: &[Model], now: i64) -> Vec<Loaded> {
    [
        ("tide-7b-instruct-q4_k_m", 5 * HOUR, 2 * MINUTE, 14),
        ("brine-coder-14b-4bit", 25 * MINUTE, 4 * MINUTE, 3),
    ]
    .iter()
    .filter_map(|(id, loaded_ago, used_ago, answers)| {
        let model = library.iter().find(|one| one.id == *id)?;
        Some(Loaded {
            id: (*id).to_owned(),
            model: model.name.clone(),
            state: LoadState::Ready,
            progress: 1.0,
            memory: model_memory(model),
            context: model.context.min(8192.0),
            loaded_at: now - loaded_ago,
            used_at: now - used_ago,
            answers: *answers,
        })
    })
    .collect()
}

/// A finished answer as the simulated model writes it, for the seed's conversations, and the tokens
/// it took with its question.
fn answered(
    ids: &mut Ids,
    now: i64,
    model: &Model,
    prompt: &str,
    settings: &Sampling,
    history: f64,
    time: i64,
) -> (AssistantMessage, f64) {
    let planned = plan(&Ask {
        prompt: prompt.to_owned(),
        model: model.clone(),
        settings: settings.clone(),
        attachments: Vec::new(),
        history,
        attempt: 0,
    });
    let output = tokens(&planned.text) + tokens(&planned.reasoning);
    let usage = Usage {
        input: planned.input,
        output,
        first_token: planned.prefill,
        rate: fixed(planned.rate, 1),
        latency: round(planned.prefill + (output / planned.rate) * 1000.0),
        note: planned.note.clone(),
    };
    let message = AssistantMessage {
        id: ids.next("m", now),
        versions: vec![Version {
            text: planned.text,
            reasoning: Some(planned.reasoning),
            model: model.id.clone(),
            state: ReplyState::Done,
            usage: Some(usage),
            time,
            problem: None,
            feedback: None,
        }],
        at: 0,
    };
    (message, planned.input + output)
}

#[allow(clippy::too_many_arguments)]
fn conversation(
    ids: &mut Ids,
    models: &[Model],
    id: &str,
    model_name: &str,
    prompts: &[&str],
    ended_ago: i64,
    now: i64,
    pinned: bool,
) -> Conversation {
    let model = models.iter().find(|one| one.id == model_name).unwrap_or(&models[0]);
    let settings = model.defaults.clone();
    let mut messages = Vec::new();
    let mut history = 0.0;
    for (index, prompt) in prompts.iter().enumerate() {
        let time = now - ended_ago - (prompts.len() - index) as i64 * 3 * MINUTE;
        messages.push(Message::User(UserMessage {
            id: ids.next("m", now),
            text: (*prompt).to_owned(),
            time,
            attachments: Vec::new(),
            reactions: None,
        }));
        let (message, used) = answered(ids, now, model, prompt, &settings, history, time + 20_000);
        history += used;
        messages.push(Message::Assistant(message));
    }
    let title = match id {
        "c-parse" => "Amounts with a decimal comma",
        "c-cron" => "Weekday cron for send-digest",
        "c-week" => "This week's runs",
        "c-rate" => "Why a bigger model writes slower",
        _ => prompts[0],
    };
    Conversation {
        id: id.to_owned(),
        title: title.to_owned(),
        model: model.id.clone(),
        time: now - ended_ago,
        pinned,
        messages,
        settings,
    }
}

pub fn seed_conversations(ids: &mut Ids, models: &[Model], now: i64) -> Vec<Conversation> {
    vec![
        conversation(
            ids,
            models,
            "c-parse",
            "brine-coder-14b-4bit",
            &[
                "Write a Python function to parse amounts that may use a decimal comma, like 12,50.",
                "Now a jq query for the failed runs in the API answer.",
            ],
            25 * MINUTE,
            now,
            false,
        ),
        conversation(
            ids,
            models,
            "c-cron",
            "tide-7b-instruct-q4_k_m",
            &["What cron schedule runs send-digest every weekday at 06:30?"],
            3 * HOUR,
            now,
            true,
        ),
        conversation(
            ids,
            models,
            "c-week",
            "tide-14b-instruct-q4_k_m",
            &["Summarize this week for the harbor project: failures, timeouts and anything slow."],
            26 * HOUR,
            now,
            false,
        ),
        conversation(
            ids,
            models,
            "c-rate",
            "reef-8b-q4_k_m",
            &["Explain why a bigger model writes slower than a small one."],
            6 * DAY,
            now,
            false,
        ),
    ]
}

pub fn seed_comparisons(ids: &mut Ids, models: &[Model], now: i64) -> Vec<Comparison> {
    let prompt = "Explain why nightly-export failed last night, in under a hundred words.";
    let picked = ["kelp-3b-q4_k_m", "tide-7b-instruct-q4_k_m", "reef-8b-q4_k_m"];
    let columns = picked
        .iter()
        .map(|id| {
            let model = models.iter().find(|one| one.id == *id).unwrap_or(&models[0]);
            let settings = Sampling {
                temperature: 0.7,
                max_tokens: 512.0,
                ..default_sampling()
            };
            let (message, _) = answered(ids, now, model, prompt, &settings, 0.0, now - 2 * HOUR);
            let version = &message.versions[0];
            CompareColumn {
                model: (*id).to_owned(),
                state: ColumnState::Done,
                text: version.text.clone(),
                reasoning: Some(version.reasoning.clone().unwrap_or_default()),
                usage: version.usage.clone(),
                problem: None,
            }
        })
        .collect();
    vec![Comparison {
        id: "cmp-seed".to_owned(),
        prompt: prompt.to_owned(),
        time: now - 2 * HOUR,
        temperature: 0.7,
        max_tokens: 512.0,
        columns,
    }]
}

pub fn seed_runner(now: i64) -> Vec<Runner> {
    vec![Runner {
        id: "runner".to_owned(),
        status: RunnerStatus::Up,
        since: now - 5 * HOUR - 12 * MINUTE,
        version: "0.9.4".to_owned(),
    }]
}

pub fn seed_events(now: i64) -> Vec<LogEvent> {
    let lines: [(i64, Level, LogSource, String); 12] = [
        (
            5 * HOUR + 12 * MINUTE,
            Level::Info,
            LogSource::Runner,
            "runner 0.9.4 listening on runner.example.org:8400".to_owned(),
        ),
        (
            5 * HOUR + 11 * MINUTE,
            Level::Info,
            LogSource::Library,
            "library opened at /Volumes/models: 15 models".to_owned(),
        ),
        (
            5 * HOUR,
            Level::Info,
            LogSource::Loader,
            "loaded tide-7b-instruct Q4_K_M in 2.1 s (4.9 GB)".to_owned(),
        ),
        (
            3 * HOUR,
            Level::Info,
            LogSource::Chat,
            "tide-7b-instruct answered 74 tokens at 57.2 tok/s".to_owned(),
        ),
        (
            2 * HOUR,
            Level::Info,
            LogSource::Loader,
            "loaded kelp-3b Q4_K_M in 1.0 s (2.0 GB)".to_owned(),
        ),
        (
            2 * HOUR,
            Level::Info,
            LogSource::Loader,
            "loaded reef-8b Q4_K_M in 2.4 s (5.4 GB)".to_owned(),
        ),
        (
            2 * HOUR - MINUTE,
            Level::Info,
            LogSource::Chat,
            "compared kelp-3b, tide-7b-instruct and reef-8b on one prompt".to_owned(),
        ),
        (
            100 * MINUTE,
            Level::Info,
            LogSource::Loader,
            "unloaded kelp-3b after 10 min idle".to_owned(),
        ),
        (
            100 * MINUTE,
            Level::Info,
            LogSource::Loader,
            "unloaded reef-8b after 10 min idle".to_owned(),
        ),
        (
            90 * MINUTE,
            Level::Warn,
            LogSource::Pull,
            format!("{} answered slowly; retried a part of eddy-22b", Source::Hub.label()),
        ),
        (
            25 * MINUTE,
            Level::Info,
            LogSource::Loader,
            "loaded brine-coder-14b 4bit in 3.6 s (9.1 GB)".to_owned(),
        ),
        (
            24 * MINUTE,
            Level::Info,
            LogSource::Chat,
            "brine-coder-14b answered 212 tokens at 33.8 tok/s".to_owned(),
        ),
    ];
    let mut events: Vec<LogEvent> = lines
        .into_iter()
        .enumerate()
        .map(|(index, (ago, level, source, text))| LogEvent {
            id: format!("e-seed-{index}"),
            time: now - ago,
            level,
            source,
            text,
        })
        .collect();
    events.reverse();
    events
}
