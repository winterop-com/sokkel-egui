//! The app's frame, as the web's `ModelsApp.tsx` has it: the rail with its counts, the trail over
//! the page, the runner's menu, the status line, and the palette's commands.

use serde::Serialize;

use crate::data::format::{bytes, counted, uptime};
use crate::data::model::{Conversation, Download, LoadState, Loaded, Model, Prefs, Runner, RunnerStatus};

/// The pages the app has, in the rail's order.
pub const PAGES: [&str; 7] = ["library", "disk", "discover", "downloads", "chat", "compare", "runner"];

/// What each page is called.
pub fn label(page: &str) -> &'static str {
    match page {
        "library" => "Models",
        "disk" => "Disk usage",
        "discover" => "Discover",
        "downloads" => "Downloads",
        "chat" => "Chat",
        "compare" => "Compare",
        "runner" => "Runner",
        "settings" => "Settings",
        _ => "",
    }
}

/// One entry of the rail: its page, label, address, whether it is shown, and its count.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RailEntry {
    pub id: &'static str,
    pub label: &'static str,
    pub href: String,
    pub active: bool,
    pub count: Option<usize>,
    pub keywords: Vec<&'static str>,
}

/// The rail's groups: their id, title and entries.
pub fn rail(
    page: &str,
    downloads: &[Download],
    loaded: &[Loaded],
) -> Vec<(&'static str, &'static str, Vec<RailEntry>)> {
    let pulling = downloads.iter().filter(|one| one.state.going()).count();
    let ready = loaded.iter().filter(|one| one.state == LoadState::Ready).count();
    let entry = |id: &'static str, count: usize, keywords: &[&'static str]| RailEntry {
        id,
        label: label(id),
        href: format!("#/{id}"),
        active: page == id,
        count: (count > 0).then_some(count),
        keywords: keywords.to_vec(),
    };
    vec![
        (
            "library",
            "Library",
            vec![
                entry("library", 0, &["models", "library"]),
                entry("disk", 0, &["storage", "space"]),
            ],
        ),
        (
            "get",
            "Get models",
            vec![
                entry("discover", 0, &["search", "pull", "hub"]),
                entry("downloads", pulling, &["pulls"]),
            ],
        ),
        (
            "use",
            "Use",
            vec![
                entry("chat", 0, &["conversations", "ask"]),
                entry("compare", 0, &["side by side"]),
            ],
        ),
        (
            "run",
            "Run",
            vec![entry("runner", ready, &["memory", "loaded", "server"])],
        ),
        (
            "system",
            "System",
            vec![entry("settings", 0, &["preferences", "root", "theme"])],
        ),
    ]
}

/// The trail over a route: each crumb's label, its address, and whether it is in the code face.
pub fn trail(route: &[&str], models: &[Model], conversations: &[Conversation]) -> Vec<(String, Option<String>, bool)> {
    let first = route.first().copied().unwrap_or("library");
    let rest = &route[route.len().min(1)..];
    if !PAGES.contains(&first) {
        return vec![("Not found".to_owned(), None, false)];
    }
    if rest.is_empty() {
        return vec![(label(first).to_owned(), None, false)];
    }
    let second = rest.join("/");
    let named = match first {
        "library" => models.iter().find(|one| one.id == second).map(|one| one.name.clone()),
        "chat" => conversations
            .iter()
            .find(|one| one.id == second)
            .map(|one| one.title.clone()),
        "compare" => Some(if second == "new" { "New" } else { "Comparison" }.to_owned()),
        _ => Some(second.clone()),
    };
    vec![
        (label(first).to_owned(), Some(format!("#/{first}")), false),
        (named.unwrap_or(second), None, first == "library"),
    ]
}

/// The runner's menu: its state, what it says of the runner, and its words for a state between.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ServerMenu {
    pub state: &'static str,
    pub facts: Vec<String>,
    pub checking: &'static str,
    pub toggle: &'static str,
    pub toggle_on: bool,
}

pub fn server_menu(runner: Option<&Runner>, loaded: &[Loaded], now: i64) -> ServerMenu {
    let status = runner.map_or(RunnerStatus::Down, |one| one.status);
    let ready = loaded.iter().filter(|one| one.state == LoadState::Ready).count();
    let since = runner.map_or(now, |one| one.since);
    ServerMenu {
        state: match status {
            RunnerStatus::Up => "up",
            RunnerStatus::Down => "down",
            _ => "checking",
        },
        facts: vec![
            "runner.example.org:8400".to_owned(),
            match status {
                RunnerStatus::Up => format!(
                    "Up for {}, {} loaded",
                    uptime((now - since) as f64),
                    counted(ready as f64, "model")
                ),
                RunnerStatus::Down => "Stopped; chats cannot be answered until it starts".to_owned(),
                RunnerStatus::Starting => "Starting".to_owned(),
                RunnerStatus::Stopping => "Stopping".to_owned(),
            },
        ],
        checking: if status == RunnerStatus::Stopping {
            "Stopping"
        } else {
            "Starting"
        },
        toggle: if status == RunnerStatus::Up {
            "Stop the runner"
        } else {
            "Start the runner"
        },
        toggle_on: matches!(status, RunnerStatus::Up | RunnerStatus::Down),
    }
}

/// What the probe logs: the level and the line.
pub fn probe(runner: Option<&Runner>) -> (crate::data::model::Level, &'static str) {
    if runner.is_some_and(|one| one.status == RunnerStatus::Up) {
        (crate::data::model::Level::Info, "probe: the runner answered in 3 ms")
    } else {
        (
            crate::data::model::Level::Warn,
            "probe: no answer from runner.example.org:8400",
        )
    }
}

/// The status line: whether it pulses, its tone, its note, and its two items.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StatusLine {
    pub live: bool,
    pub warn: bool,
    pub note: String,
    pub loaded: String,
    pub on_disk: String,
}

pub fn status_line(
    runner: Option<&Runner>,
    downloads: &[Download],
    loaded: &[Loaded],
    models: &[Model],
    now: i64,
) -> StatusLine {
    let status = runner.map_or(RunnerStatus::Down, |one| one.status);
    let pulling: Vec<&Download> = downloads.iter().filter(|one| one.state.going()).collect();
    let ready: Vec<&Loaded> = loaded.iter().filter(|one| one.state == LoadState::Ready).collect();
    let since = runner.map_or(now, |one| one.since);
    StatusLine {
        live: pulling
            .iter()
            .any(|one| one.state == crate::data::model::DownloadState::Running),
        warn: status == RunnerStatus::Down,
        note: if status == RunnerStatus::Down {
            "The runner is stopped".to_owned()
        } else if !pulling.is_empty() {
            format!(
                "Pulling {}",
                pulling
                    .iter()
                    .map(|one| one.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            format!("Runner up for {}", uptime((now - since) as f64))
        },
        loaded: if ready.is_empty() {
            "none".to_owned()
        } else {
            ready
                .iter()
                .map(|one| one.model.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        },
        on_disk: format!(
            "{} · {}",
            models.len(),
            bytes(models.iter().fold(0.0, |sum, model| sum + model.size))
        ),
    }
}

/// The model a new chat starts with: the default, else the first in the library.
pub fn default_model<'a>(models: &'a [Model], prefs: &Prefs) -> Option<&'a Model> {
    models
        .iter()
        .find(|one| one.id == prefs.default_model)
        .or_else(|| models.first())
}

/// The palette's own commands: each id, label and hint.
pub fn actions(
    runner: Option<&Runner>,
    downloads: &[Download],
    models: &[Model],
    prefs: &Prefs,
) -> Vec<(&'static str, &'static str, Option<String>)> {
    let up = runner.is_some_and(|one| one.status == RunnerStatus::Up);
    let mut list = vec![
        (
            "new-chat",
            "New chat",
            default_model(models, prefs).map(|one| one.name.clone()),
        ),
        ("pull", "Pull a model", None),
        ("compare", "Compare models", None),
        if up {
            ("runner-stop", "Stop the runner", None)
        } else {
            ("runner-start", "Start the runner", None)
        },
    ];
    if downloads.iter().any(|one| one.state.ended()) {
        list.push(("clear-pulls", "Clear ended downloads", None));
    }
    list
}

/// The palette's conversations, the latest first: each id, title and the model's name.
pub fn palette_conversations(conversations: &[Conversation], models: &[Model]) -> Vec<(String, String, String)> {
    let mut sorted: Vec<&Conversation> = conversations.iter().collect();
    sorted.sort_by_key(|one| std::cmp::Reverse(one.time));
    sorted
        .iter()
        .map(|one| {
            (
                format!("conversation-{}", one.id),
                one.title.clone(),
                models
                    .iter()
                    .find(|model| model.id == one.model)
                    .map_or_else(|| one.model.clone(), |model| model.name.clone()),
            )
        })
        .collect()
}
