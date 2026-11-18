//! What the studio keeps, as the web's `model.ts`: the workspace's files, the runs of its pipelines
//! and the reader's settings, and the words names, durations, times and counts are said in.
//!
//! The web reads times on the local clock. Here the local zone is a fixed offset from UTC:
//! `set_local_offset` sets it, UTC until then.

use std::cmp::Ordering;
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

use serde::{Deserialize, Serialize};

use crate::js::{DAY, HOUR, MINUTE, locale_compare, locale_number, number_to_string, round, to_fixed, trim};

/// How far the local zone is ahead of UTC, in milliseconds: UTC until the app sets it.
static OFFSET: AtomicI64 = AtomicI64::new(0);

/// Sets the local zone, `ms` milliseconds ahead of UTC, as the app starts.
pub fn set_local_offset(ms: i64) {
    OFFSET.store(ms, AtomicOrdering::Relaxed);
}

/// How far the local zone is ahead of UTC, in milliseconds.
pub fn local_offset() -> i64 {
    OFFSET.load(AtomicOrdering::Relaxed)
}

/// A project of the Tidevann workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Project {
    Harbor,
    Juniper,
    Larkspur,
}

/// The projects, in the order the rail lists them.
pub const PROJECTS: [Project; 3] = [Project::Harbor, Project::Juniper, Project::Larkspur];

impl Project {
    /// The project's name: harbor.
    pub fn name(self) -> &'static str {
        match self {
            Self::Harbor => "harbor",
            Self::Juniper => "juniper",
            Self::Larkspur => "larkspur",
        }
    }

    /// The project a name is, if it is one.
    pub fn named(name: &str) -> Option<Self> {
        PROJECTS.into_iter().find(|one| one.name() == name)
    }
}

/// What a file holds: a pipeline is YAML, a schema is a JSON Schema document, a query is SQL.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Pipeline,
    Schema,
    Query,
}

/// How a kind of file is named and said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindInfo {
    pub kind: FileKind,
    pub label: &'static str,
    pub plural: &'static str,
    pub extension: &'static str,
}

/// The kinds, in the order the rail groups them.
pub const KINDS: [KindInfo; 3] = [
    KindInfo {
        kind: FileKind::Pipeline,
        label: "Pipeline",
        plural: "Pipelines",
        extension: ".yaml",
    },
    KindInfo {
        kind: FileKind::Schema,
        label: "Schema",
        plural: "Schemas",
        extension: ".schema.json",
    },
    KindInfo {
        kind: FileKind::Query,
        label: "Query",
        plural: "Queries",
        extension: ".sql",
    },
];

/// How a kind of file is named and said.
pub fn kind_of(kind: FileKind) -> KindInfo {
    KINDS.into_iter().find(|one| one.kind == kind).unwrap_or(KINDS[0])
}

/// One file of the workspace, as last saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkFile {
    pub id: String,
    pub project: Project,
    pub kind: FileKind,
    /// The file's name with its extension: `nightly-export.yaml`.
    pub name: String,
    pub text: String,
    pub updated: i64,
    /// Its place among the files of its project and kind, as the reader put them; files without one
    /// follow, by name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<i64>,
}

/// The order of the files in one of a project's groups: by the reader's ranks, then by name.
pub fn file_order(a: &WorkFile, b: &WorkFile) -> Ordering {
    match (a.rank, b.rank) {
        (Some(x), Some(y)) if x != y => x.cmp(&y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        _ => locale_compare(&a.name, &b.name),
    }
}

/// Where a step of a run stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

impl StepState {
    /// The state as the tables write it: succeeded.
    pub fn name(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Where a run stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunState {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl RunState {
    /// The state as the tables write it: succeeded.
    pub fn name(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// One step of a run, as far as it got.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStepRecord {
    pub id: String,
    pub kind: String,
    pub needs: Vec<String>,
    pub state: StepState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<i64>,
}

/// How loud a line of a run's log is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// A line of a run's log: when it was written, from the start of the run, by which step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLine {
    pub at: i64,
    pub step: Option<String>,
    pub level: Level,
    pub text: String,
}

/// The step a run failed at, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    pub step: String,
    pub reason: String,
}

/// A run of a pipeline started in the studio.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    /// Counted up across the workspace: run 12.
    pub number: i64,
    pub file: String,
    pub pipeline: String,
    pub project: Project,
    pub state: RunState,
    pub started: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished: Option<i64>,
    pub by: String,
    pub steps: Vec<RunStepRecord>,
    pub log: Vec<LogLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<Failure>,
    /// Run from changes not saved yet.
    pub draft: bool,
}

/// The editor's text size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextSize {
    Sm,
    Md,
    Lg,
}

/// How a pipeline is edited: as text, on its graph, or both side by side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineView {
    Text,
    Graph,
    Split,
}

/// The reader's settings and where they left the work area.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub id: String,
    /// The open tabs: file ids, or `table:<name>` for a table's rows.
    pub tabs: Vec<String>,
    pub active: Option<String>,
    pub wrap: bool,
    pub line_numbers: bool,
    pub size: TextSize,
    pub folding: bool,
    pub indent_guides: bool,
    pub bracket_colors: bool,
    /// The caret's line stays shaded while the focus is elsewhere.
    pub line_wash: bool,
    pub pipeline_view: PipelineView,
}

impl Default for Prefs {
    /// The web's `DEFAULT_PREFS`.
    fn default() -> Self {
        Self {
            id: "prefs".to_owned(),
            tabs: vec![
                "harbor-nightly-export".to_owned(),
                "harbor-order-schema".to_owned(),
                "juniper-slow-steps".to_owned(),
            ],
            active: Some("harbor-nightly-export".to_owned()),
            wrap: false,
            line_numbers: true,
            size: TextSize::Md,
            folding: true,
            indent_guides: true,
            bracket_colors: false,
            line_wash: false,
            pipeline_view: PipelineView::Split,
        }
    }
}

/// The person every edit and run is made as.
pub const ME: &str = "ada";

/// A file name's stem, without its kind's extension.
pub fn stem(name: &str, kind: FileKind) -> &str {
    name.strip_suffix(kind_of(kind).extension).unwrap_or(name)
}

/// Why a name cannot be a file's, or none when it can: empty, the wrong letters, or taken in the
/// project.
pub fn bad_name(name: &str, taken: &[&str]) -> Option<String> {
    let typed = trim(name);
    if typed.is_empty() {
        return Some("Give it a name.".to_owned());
    }
    if !is_slug(typed) {
        return Some("Lowercase letters, digits and dashes, starting with a letter.".to_owned());
    }
    if taken.contains(&typed) {
        return Some("A file of that kind in this project has that name.".to_owned());
    }
    None
}

/// Whether a text is a name as `^[a-z][a-z0-9-]*$` reads one.
pub fn is_slug(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A duration said briefly: 840 ms, 2.4 s, 1 min 12 s.
pub fn say_duration(ms: f64) -> String {
    if ms < 1000.0 {
        return format!("{} ms", number_to_string(round(ms)));
    }
    if ms < 60_000.0 {
        return format!("{} s", to_fixed(ms / 1000.0, 1));
    }
    let minutes = (ms / 60_000.0).floor();
    format!(
        "{} min {} s",
        number_to_string(minutes),
        number_to_string(round((ms % 60_000.0) / 1000.0))
    )
}

/// A time of day on the local clock: 14:02:09.
pub fn clock(at: i64) -> String {
    let within = (at + local_offset()).rem_euclid(DAY);
    format!(
        "{:02}:{:02}:{:02}",
        within / HOUR,
        within % HOUR / MINUTE,
        within % MINUTE / 1000
    )
}

/// A count with its noun: 1 row, 3 rows; `plural` is the noun and an s when none is given.
pub fn counted(count: f64, noun: &str, plural: Option<&str>) -> String {
    let word = if count == 1.0 {
        noun.to_owned()
    } else {
        plural.map_or_else(|| format!("{noun}s"), str::to_owned)
    };
    format!("{} {word}", locale_number(count))
}
