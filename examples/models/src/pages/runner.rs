//! The runner page, as the web's `pages/Runner.tsx` has it: whether it runs, the models it holds in
//! memory and what they take, its health, and its log.

use serde::Serialize;

use super::disk::{Stat, stat};
use crate::data::disk::{usage_of, valid_root};
use crate::data::format::{GB, bytes, clock_seconds, counted, uptime};
use crate::data::model::{Download, Level, LoadState, Loaded, LogEvent, LogSource, Model, Prefs, Runner, RunnerStatus};
use crate::engine::runner::MACHINE_MEMORY;
use crate::js::{locale, number_to_string, round};

/// One health check: its name, status, detail and hint.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Check {
    pub name: &'static str,
    pub status: &'static str,
    pub detail: String,
    pub hint: Option<&'static str>,
}

/// One line of the log as the log view shows it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Line {
    pub at: String,
    pub level: &'static str,
    pub source: &'static str,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunnerPage {
    pub code: String,
    /// The header's button: `stop`, or the start button's words and whether it is on.
    pub button: (String, bool),
    pub stats: Vec<Stat>,
    pub memory_meta: String,
    pub memory_label: String,
    pub memory_value: f64,
    /// Each loaded model's row: model, state, memory, context, answers, last used, unloads in.
    pub loaded: Vec<Vec<String>>,
    pub empty: &'static str,
    pub checks: Vec<Check>,
    pub lines: Vec<Line>,
}

/// What the state column says of a loaded model.
pub fn load_chip(one: &Loaded) -> String {
    match one.state {
        LoadState::Ready => "ready|".to_owned(),
        LoadState::Loading => format!("loading|loading {}%", number_to_string(round(one.progress * 100.0))),
    }
}

/// The log's lines: the source chosen and the words searched for, oldest first, the last 400.
pub fn lines(events: &[LogEvent], source: Option<LogSource>, query: &str) -> Vec<Line> {
    let words = crate::js::trim(query).to_lowercase();
    let mut sorted: Vec<&LogEvent> = events.iter().collect();
    sorted.sort_by_key(|event| event.time);
    let kept: Vec<&LogEvent> = sorted
        .into_iter()
        .filter(|event| {
            source.is_none_or(|source| event.source == source)
                && (words.is_empty() || event.text.to_lowercase().contains(&words))
        })
        .collect();
    let skip = kept.len().saturating_sub(400);
    kept[skip..]
        .iter()
        .map(|event| Line {
            at: clock_seconds(event.time),
            level: match event.level {
                Level::Info => "info",
                Level::Warn => "warning",
                Level::Error => "error",
            },
            source: event.source.key(),
            text: event.text.clone(),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn runner_page(
    runner: Option<&Runner>,
    loaded: &[Loaded],
    models: &[Model],
    prefs: &Prefs,
    events: &[LogEvent],
    downloads: &[Download],
    now: i64,
    source: Option<LogSource>,
    query: &str,
) -> RunnerPage {
    let status = runner.map_or(RunnerStatus::Down, |one| one.status);
    let used = loaded.iter().fold(0.0, |sum, one| sum + one.memory);
    let limit = prefs.memory * GB;
    let disk = usage_of(&prefs.root, models, downloads);
    let since = runner.map_or(now, |one| one.since);
    let up_for = uptime((now - since) as f64);
    let keep = prefs.keep_alive;
    let checks = vec![
        Check {
            name: "Runner",
            status: match status {
                RunnerStatus::Up => "ok",
                RunnerStatus::Down => "fail",
                _ => "warn",
            },
            detail: if status == RunnerStatus::Up {
                format!("Answering on runner.example.org:8400 for {up_for}")
            } else {
                format!("The runner is {}", status.key())
            },
            hint: (status == RunnerStatus::Down).then_some("Start it with the button above."),
        },
        Check {
            name: "Memory",
            status: if used > limit * 0.9 { "warn" } else { "ok" },
            detail: format!(
                "{} of the {} the runner may use ({} in this machine)",
                bytes(used),
                bytes(limit),
                bytes(MACHINE_MEMORY)
            ),
            hint: (used > limit * 0.9).then_some("Unload a model, or raise the limit in Settings."),
        },
        Check {
            name: "Library root",
            status: if valid_root(&prefs.root) { "ok" } else { "fail" },
            detail: format!("{} on {}", prefs.root, disk.disk.name),
            hint: (!valid_root(&prefs.root)).then_some("Set a folder path in Settings."),
        },
        Check {
            name: "Disk room",
            status: if disk.free < 50.0 * GB { "warn" } else { "ok" },
            detail: format!("{} free", bytes(disk.free)),
            hint: (disk.free < 50.0 * GB).then_some("Delete models kept twice on the Disk usage page."),
        },
        Check {
            name: "Keep-alive",
            status: "ok",
            detail: if keep == 0.0 {
                "Models stay loaded until unloaded".to_owned()
            } else {
                format!("Idle models unload after {} min", number_to_string(keep))
            },
            hint: None,
        },
    ];
    let said: Vec<String> = loaded
        .iter()
        .map(|one| format!("{} {}", one.model, bytes(one.memory)))
        .collect();
    RunnerPage {
        code: format!(
            "runner.example.org:8400 · {}",
            runner.map_or(String::new(), |one| one.version.clone())
        ),
        button: match status {
            RunnerStatus::Up => ("Stop".to_owned(), true),
            RunnerStatus::Starting => ("Starting".to_owned(), false),
            RunnerStatus::Stopping => ("Stopping".to_owned(), false),
            RunnerStatus::Down => ("Start".to_owned(), true),
        },
        stats: vec![
            stat(
                "State",
                format!("{}|", status.key()),
                matches!(status, RunnerStatus::Up | RunnerStatus::Down).then(|| format!("for {up_for}")),
                None,
            ),
            stat(
                "Loaded",
                loaded
                    .iter()
                    .filter(|one| one.state == LoadState::Ready)
                    .count()
                    .to_string(),
                Some(format!("{} on disk", counted(models.len() as f64, "model"))),
                None,
            ),
            stat(
                "Memory",
                bytes(used),
                Some(format!("of {}", bytes(limit))),
                Some(used <= limit * 0.9),
            ),
            stat(
                "Keep-alive",
                if keep == 0.0 {
                    "off".to_owned()
                } else {
                    format!("{} min", number_to_string(keep))
                },
                Some("idle models unload after it".to_owned()),
                None,
            ),
        ],
        memory_meta: format!("{} of {}", bytes(used), bytes(limit)),
        memory_label: if said.is_empty() {
            "Nothing loaded".to_owned()
        } else {
            said.join(", ")
        },
        memory_value: (round((used / limit) * 1000.0) / 10.0).min(100.0),
        loaded: loaded
            .iter()
            .map(|one| {
                vec![
                    one.model.clone(),
                    load_chip(one),
                    bytes(one.memory),
                    locale(one.context),
                    one.answers.to_string(),
                    // When it was last used, which the page says relative to the moment it is drawn.
                    String::new(),
                    if keep == 0.0 || one.state != LoadState::Ready {
                        "-".to_owned()
                    } else {
                        uptime(((one.used_at as f64 + keep * 60_000.0 - now as f64).max(0.0)) + 60_000.0)
                    },
                ]
            })
            .collect(),
        empty: if status == RunnerStatus::Up {
            "Nothing is loaded. A chat loads its model, or load one here."
        } else {
            "The runner is stopped."
        },
        checks,
        lines: lines(events, source, query),
    }
}
