//! The tables a query reads, as the web's `tables.ts`: the jobs, three weeks of their runs and the
//! steps of each, made the same every time from a fixed seed, with the runs started in the studio
//! added as they happen.

use std::sync::OnceLock;

use crate::js::{iso_instant as iso, round, utc};
use crate::model::Run;
use crate::sql::{Column, ColumnType, Table, Value};

/// The same sequence of numbers from 0 to 1 every time.
struct Seeded {
    state: u64,
}

impl Seeded {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> f64 {
        self.state = (self.state * 1_664_525 + 1_013_904_223) % 4_294_967_296;
        self.state as f64 / 4_294_967_296.0
    }
}

/// A scheduled pipeline: its name, project, owner, cron line and steps.
pub struct Job {
    pub name: &'static str,
    pub project: &'static str,
    pub owner: &'static str,
    pub schedule: &'static str,
    pub steps: &'static [&'static str],
}

/// The scheduled pipelines.
pub const JOBS: [Job; 5] = [
    Job {
        name: "nightly-export",
        project: "harbor",
        owner: "ada",
        schedule: "0 2 * * *",
        steps: &["fetch", "transform", "validate", "upload", "notify"],
    },
    Job {
        name: "import-orders",
        project: "harbor",
        owner: "grace",
        schedule: "*/30 * * * *",
        steps: &["fetch", "validate", "upload"],
    },
    Job {
        name: "sync-inventory",
        project: "juniper",
        owner: "linus",
        schedule: "0 * * * *",
        steps: &["fetch", "transform", "upload"],
    },
    Job {
        name: "rebuild-index",
        project: "juniper",
        owner: "margaret",
        schedule: "0 4 * * 0",
        steps: &["fetch", "transform", "upload", "notify"],
    },
    Job {
        name: "send-digest",
        project: "larkspur",
        owner: "ada",
        schedule: "0 8 * * 1-5",
        steps: &["fetch", "transform", "notify"],
    },
];

/// About how long each step takes, in milliseconds.
fn slow(step: &str) -> f64 {
    match step {
        "fetch" => 1800.0,
        "transform" => 900.0,
        "validate" => 600.0,
        "upload" => 1400.0,
        "notify" => 200.0,
        _ => 500.0,
    }
}

/// A moment as the tables write it, in UTC: 2026-11-02 02:00:14.
pub fn table_time(at: i64) -> String {
    iso(at)[..19].replacen('T', " ", 1)
}

/// The runs and their steps, as rows of the runs and steps tables.
struct History {
    runs: Vec<Vec<Value>>,
    steps: Vec<Vec<Value>>,
}

fn history() -> History {
    let mut random = Seeded::new(4181);
    let mut runs = Vec::new();
    let mut steps = Vec::new();
    let base = utc(2026, 9, 20, 0, 0, 0);
    let people = ["ada", "grace", "linus", "margaret", "scheduler"];
    for index in 0..160_i64 {
        let job = &JOBS[(random.next() * JOBS.len() as f64).floor() as usize];
        let started =
            base + ((index as f64 / 160.0) * 21.0 * 86_400_000.0 + random.next() * 3_600_000.0).floor() as i64;
        let roll = random.next();
        let fail_at: i64 = if roll < 0.16 {
            (random.next() * job.steps.len() as f64).floor() as i64
        } else {
            -1
        };
        let cancelled = fail_at == -1 && roll > 0.97;
        let mut duration = 0.0;
        let id = 4000 + index;
        for (at, step) in job.steps.iter().enumerate() {
            let at = at as i64;
            let ran = fail_at == -1 || at <= fail_at;
            let took = round(slow(step) * (0.5 + random.next()));
            let state = if cancelled && at == job.steps.len() as i64 - 1 {
                "cancelled"
            } else if !ran {
                "skipped"
            } else if at == fail_at {
                "failed"
            } else {
                "succeeded"
            };
            if ran {
                duration += took;
            }
            let rows = if ran && state != "failed" {
                Value::Num(round(200.0 + random.next() * 4800.0))
            } else {
                Value::Null
            };
            steps.push(vec![
                Value::Num(id as f64),
                Value::text(step),
                Value::text(step),
                Value::text(state),
                if ran { Value::Num(took) } else { Value::Null },
                rows,
            ]);
        }
        let state = if fail_at >= 0 {
            "failed"
        } else if cancelled {
            "cancelled"
        } else {
            "succeeded"
        };
        let triggered_by = if random.next() < 0.75 {
            "scheduler"
        } else {
            people[(random.next() * 4.0).floor() as usize]
        };
        runs.push(vec![
            Value::Num(id as f64),
            Value::text(job.name),
            Value::text(job.project),
            Value::text(state),
            Value::Str(table_time(started)),
            Value::Num(duration),
            Value::text(triggered_by),
        ]);
    }
    History { runs, steps }
}

fn kept() -> &'static History {
    static HISTORY: OnceLock<History> = OnceLock::new();
    HISTORY.get_or_init(history)
}

fn column(name: &str, kind: ColumnType, description: &str) -> Column {
    Column {
        name: name.to_owned(),
        kind,
        description: description.to_owned(),
    }
}

/// The tables as they stand, the studio's own runs among them, each numbered 5000 and its number.
pub fn tables_with(studio_runs: &[Run]) -> Vec<Table> {
    let mut ours: Vec<&Run> = studio_runs.iter().collect();
    ours.sort_by_key(|run| run.number);
    let history = kept();
    let mut run_rows = history.runs.clone();
    run_rows.extend(ours.iter().map(|run| {
        vec![
            Value::Num((5000 + run.number) as f64),
            Value::Str(run.pipeline.clone()),
            Value::text(run.project.name()),
            Value::text(run.state.name()),
            Value::Str(table_time(run.started)),
            run.finished
                .map_or(Value::Null, |finished| Value::Num((finished - run.started) as f64)),
            Value::Str(run.by.clone()),
        ]
    }));
    let mut step_rows = history.steps.clone();
    step_rows.extend(ours.iter().flat_map(|run| {
        run.steps.iter().map(|step| {
            vec![
                Value::Num((5000 + run.number) as f64),
                Value::Str(step.id.clone()),
                Value::Str(step.kind.clone()),
                Value::text(step.state.name()),
                step.duration_ms.map_or(Value::Null, |ms| Value::Num(ms as f64)),
                step.rows.map_or(Value::Null, |rows| Value::Num(rows as f64)),
            ]
        })
    }));
    vec![
        Table {
            name: "runs".to_owned(),
            description: "Every run of every pipeline, the scheduled ones and those started here.".to_owned(),
            columns: vec![
                column("id", ColumnType::Integer, "The run's number."),
                column("pipeline", ColumnType::Text, "The pipeline it ran."),
                column("project", ColumnType::Text, "harbor, juniper or larkspur."),
                column("state", ColumnType::Text, "running, succeeded, failed or cancelled."),
                column(
                    "started_at",
                    ColumnType::Text,
                    "When it started, in UTC: 2026-11-02 02:00:14.",
                ),
                column(
                    "duration_ms",
                    ColumnType::Integer,
                    "How long it took; null while it runs.",
                ),
                column(
                    "triggered_by",
                    ColumnType::Text,
                    "The scheduler, or the person who started it.",
                ),
            ],
            rows: run_rows,
        },
        Table {
            name: "steps".to_owned(),
            description: "Each step of each run, as far as it got.".to_owned(),
            columns: vec![
                column("run_id", ColumnType::Integer, "The run, as runs.id."),
                column("step", ColumnType::Text, "The step's id in its pipeline."),
                column(
                    "kind",
                    ColumnType::Text,
                    "fetch, transform, validate, upload or notify.",
                ),
                column(
                    "state",
                    ColumnType::Text,
                    "succeeded, failed, skipped, cancelled or running.",
                ),
                column(
                    "duration_ms",
                    ColumnType::Integer,
                    "How long it ran; null when it did not.",
                ),
                column("rows", ColumnType::Integer, "How many rows it gave."),
            ],
            rows: step_rows,
        },
        Table {
            name: "jobs".to_owned(),
            description: "The scheduled pipelines and who looks after each.".to_owned(),
            columns: vec![
                column("name", ColumnType::Text, "The pipeline's name, as runs.pipeline."),
                column("project", ColumnType::Text, "harbor, juniper or larkspur."),
                column("owner", ColumnType::Text, "Who looks after it."),
                column("schedule", ColumnType::Text, "When it runs, as a cron line."),
            ],
            rows: JOBS
                .iter()
                .map(|job| {
                    vec![
                        Value::text(job.name),
                        Value::text(job.project),
                        Value::text(job.owner),
                        Value::text(job.schedule),
                    ]
                })
                .collect(),
        },
    ]
}
