//! A run of a pipeline on the simulated backend, as the web's `simulate.ts`: each step in turn,
//! after the steps it needs, its outcome decided by its settings so the same pipeline always ends
//! the same way, and the run kept in the runs collection as it goes, with its log.
//!
//! The web moves a run with timers and reads the clock as they fire. Here a `Going` is moved by
//! the backend's `tick`, each step at the moment its timer would fire, so a run's log reads the
//! same at any moment. The run as the web's `simulate.ts` holds it (`current`) and the row the
//! store keeps are two, as on the web: a change is made to both, and a cancel to the row alone.

use std::collections::HashMap;

use serde_json::{Map, Value};
use url::Url;

use crate::js::{iso, length, locale_number, number_to_string, round, slice, trim};
use crate::model::{Failure, Level, LogLine, ME, PROJECTS, Project, Run, RunState, RunStepRecord, StepState, WorkFile};
use crate::pipeline::{Pipeline, StepInfo, js_string, prototype_text, read_pipeline, references, step_order};

/// How long the store takes to answer, in milliseconds: a run's first step starts this long after
/// the run is made.
pub const LATENCY: i64 = 150;

/// A number from a text, the same every time.
pub fn hash(text: &str) -> u32 {
    let mut value: u32 = 2_166_136_261;
    for c in text.chars() {
        let unit = c.encode_utf16(&mut [0; 2])[0];
        value = (value ^ u32::from(unit)).wrapping_mul(16_777_619);
    }
    value
}

/// How many rows each check refuses.
fn refused_by(check: &str) -> i64 {
    match check {
        "email" => 31,
        "positive" => 6,
        "unique" => 2,
        _ => 0,
    }
}

/// What a step handed on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Done {
    pub rows: Option<i64>,
    pub rejected: Option<i64>,
    pub output: Option<String>,
}

/// What one try of a step did.
#[derive(Clone, Debug, Default, PartialEq)]
struct Outcome {
    ok: bool,
    rows: Option<i64>,
    rejected: Option<i64>,
    output: Option<String>,
    lines: Vec<(Level, String)>,
    reason: Option<String>,
}

fn as_string(value: Option<&Value>, fallback: &str) -> String {
    value.and_then(Value::as_str).unwrap_or(fallback).to_owned()
}

fn grouped(count: i64) -> String {
    locale_number(count as f64)
}

/// A `${...}` reference's value as a run fills it in.
struct Filler<'a> {
    params: &'a Map<String, Value>,
    number: i64,
    date: String,
    started: String,
    results: &'a HashMap<String, Done>,
}

impl Filler<'_> {
    fn fill(&self, value: &str) -> String {
        let mut out = String::new();
        let mut at = 0;
        for (from, to, inside) in references(value) {
            out.push_str(&value[at..from]);
            out.push_str(&self.reference(inside));
            at = to;
        }
        out.push_str(&value[at..]);
        out
    }

    fn reference(&self, reference: &str) -> String {
        let mut parts = trim(reference).split('.');
        let root = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("");
        let field = parts.next().unwrap_or("");
        match root {
            "params" => match self.params.get(name) {
                Some(Value::Null) => String::new(),
                Some(value) => js_string(value),
                None => prototype_text(name).unwrap_or_default(),
            },
            "run" => match name {
                "id" => self.number.to_string(),
                "date" => self.date.clone(),
                "started" => self.started.clone(),
                "by" => ME.to_owned(),
                _ => prototype_text(name).unwrap_or_default(),
            },
            "steps" => {
                let Some(done) = self.results.get(name) else {
                    return String::new();
                };
                match field {
                    "rows" => done.rows.map(|rows| rows.to_string()).unwrap_or_default(),
                    "rejected" => done.rejected.map(|rows| rows.to_string()).unwrap_or_default(),
                    "output" => done.output.clone().unwrap_or_default(),
                    _ => prototype_text(field).unwrap_or_default(),
                }
            }
            _ => String::new(),
        }
    }
}

/// What one try of a step does, from its settings and what the steps before it handed on; none
/// where the web's step throws (a rule that is null), and the run stalls.
fn attempt(step: &StepInfo, filler: &Filler, before: &[&Done], project: Project) -> Option<Outcome> {
    let config = &step.config;
    let given = before.iter().fold(None, |rows, one| one.rows.or(rows)).unwrap_or(0);
    let line = |level, text: String| (level, text);
    Some(match step.kind.as_str() {
        "fetch" => {
            let url = filler.fill(&as_string(config.get("url"), ""));
            let Ok(parsed) = Url::parse(&url) else {
                return Some(Outcome {
                    lines: vec![line(Level::Error, format!("{url} is not an address"))],
                    reason: Some(format!("{url} is not an address.")),
                    ..Outcome::default()
                });
            };
            let host = parsed.host_str().unwrap_or("").to_owned();
            if host != "example.org" && !host.ends_with(".example.org") {
                return Some(Outcome {
                    lines: vec![
                        line(Level::Info, format!("GET {url}")),
                        line(Level::Error, format!("could not resolve {host}")),
                    ],
                    reason: Some(format!("Could not resolve {host}.")),
                    ..Outcome::default()
                });
            }
            let rows = 400 + i64::from(hash(&url) % 2400);
            Outcome {
                ok: true,
                rows: Some(rows),
                output: Some(url.clone()),
                lines: vec![
                    line(Level::Info, format!("GET {url}")),
                    line(
                        Level::Info,
                        format!(
                            "200 OK, {}, {} rows",
                            as_string(config.get("format"), "json"),
                            grouped(rows)
                        ),
                    ),
                ],
                ..Outcome::default()
            }
        }
        "transform" => {
            let script = as_string(config.get("script"), "");
            let language = as_string(config.get("language"), "jq");
            if script.contains("error(") {
                return Some(Outcome {
                    lines: vec![line(
                        Level::Error,
                        format!("{language}: the script raised an error on row 1"),
                    )],
                    reason: Some("The script raised an error.".to_owned()),
                    ..Outcome::default()
                });
            }
            let rows = round(given as f64 * 0.97) as i64;
            let shown = if length(&script) > 60 {
                format!("{}...", slice(&script, 0.0, Some(57.0)))
            } else {
                script
            };
            Outcome {
                ok: true,
                rows: Some(rows),
                lines: vec![
                    line(Level::Info, format!("{language}: {shown}")),
                    line(
                        Level::Info,
                        format!("{} rows in, {} out", grouped(given), grouped(rows)),
                    ),
                ],
                ..Outcome::default()
            }
        }
        "validate" => {
            let rules: Vec<Value> = match config.get("rules") {
                Some(Value::Array(rules)) => rules.clone(),
                _ => Vec::new(),
            };
            let mut lines = Vec::new();
            let mut rejected = 0;
            for rule in &rules {
                if rule.is_null() {
                    return None;
                }
                let check = as_string(rule.get("check"), "");
                let refused = given.min(refused_by(&check));
                rejected += refused;
                let said = if refused == 0 {
                    "every row passes".to_owned()
                } else {
                    format!("{refused} rows refused")
                };
                lines.push(line(
                    if refused > 0 { Level::Warn } else { Level::Info },
                    format!("{check} check on {}: {said}", as_string(rule.get("field"), "")),
                ));
            }
            let on_invalid = as_string(config.get("on_invalid"), "fail");
            let allowed = config.get("max_rejected").and_then(Value::as_f64).unwrap_or(0.0);
            if on_invalid == "fail" && rejected as f64 > allowed {
                let reason = format!(
                    "{rejected} rows refused, more than max_rejected ({}).",
                    number_to_string(allowed)
                );
                lines.push(line(Level::Error, reason.clone()));
                return Some(Outcome {
                    rejected: Some(rejected),
                    lines,
                    reason: Some(reason),
                    ..Outcome::default()
                });
            }
            let kept = if on_invalid == "skip" { given - rejected } else { given };
            lines.push(line(
                Level::Info,
                if rejected == 0 {
                    format!("{} rows valid", grouped(given))
                } else if on_invalid == "skip" {
                    format!("{rejected} rows left out, {} kept", grouped(kept))
                } else {
                    format!("{rejected} rows passed with a warning")
                },
            ));
            Outcome {
                ok: true,
                rows: Some(kept),
                rejected: Some(rejected),
                lines,
                ..Outcome::default()
            }
        }
        "upload" => {
            let bucket = filler.fill(&as_string(config.get("bucket"), ""));
            let path = filler.fill(&as_string(config.get("path"), ""));
            let project = project.name();
            if !bucket.starts_with(&format!("{project}-")) {
                return Some(Outcome {
                    lines: vec![
                        line(Level::Info, format!("PUT {bucket}/{path}")),
                        line(Level::Error, format!("403: {project} cannot write to {bucket}")),
                    ],
                    reason: Some(format!("{project} cannot write to the bucket {bucket}.")),
                    ..Outcome::default()
                });
            }
            Outcome {
                ok: true,
                rows: Some(given),
                output: Some(format!("{bucket}/{path}")),
                lines: vec![
                    line(Level::Info, format!("PUT {bucket}/{path}")),
                    line(Level::Info, format!("wrote {} rows", grouped(given))),
                ],
                ..Outcome::default()
            }
        }
        _ => Outcome {
            ok: true,
            rows: Some(given),
            ..Outcome::default()
        },
    })
}

/// A change to a run, made to the run as simulated and to its stored row alike.
#[derive(Clone, Debug, Default)]
pub struct Patch {
    pub state: Option<RunState>,
    pub finished: Option<i64>,
    pub steps: Option<Vec<RunStepRecord>>,
    pub log: Option<Vec<LogLine>>,
    pub failed: Option<Failure>,
}

impl Patch {
    /// The patch made to a run, as `{ ...run, ...patch }`.
    pub fn apply(&self, run: &mut Run) {
        if let Some(state) = self.state {
            run.state = state;
        }
        if let Some(finished) = self.finished {
            run.finished = Some(finished);
        }
        if let Some(steps) = &self.steps {
            run.steps = steps.clone();
        }
        if let Some(log) = &self.log {
            run.log = log.clone();
        }
        if let Some(failed) = &self.failed {
            run.failed = Some(failed.clone());
        }
    }
}

/// A run under way: what it read from its pipeline, what its steps handed on, and the run as it
/// stands here.
#[derive(Clone, Debug)]
pub struct Going {
    pipeline: Pipeline,
    order: Vec<StepInfo>,
    project: Project,
    number: i64,
    started: i64,
    results: HashMap<String, Done>,
    current: Run,
    /// Whether a step's timer was set and the run has not finished: the web's `timers` holds it.
    pub timer: bool,
}

/// What a run asks for next: a step's timer to fire at `due`, the step began at `begun`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Due {
    pub at: i64,
    pub index: usize,
    pub begun: i64,
}

/// A run as first stored, and its `Going`, from a pipeline's text (the editor's, saved or not);
/// refused when the pipeline does not read or its steps loop.
pub fn start_run(file: &WorkFile, text: &str, number: i64, draft: bool, now: i64) -> Result<(Run, Going), String> {
    let pipeline = read_pipeline(text);
    let order = step_order(&pipeline.steps);
    let (false, Some(order)) = (pipeline.broken, order) else {
        return Err("The pipeline does not read as YAML, or its steps loop.".to_owned());
    };
    let project = PROJECTS
        .into_iter()
        .find(|one| Some(one.name()) == pipeline.project.as_deref())
        .unwrap_or(file.project);
    let name = if pipeline.name.is_empty() {
        file.name.clone()
    } else {
        pipeline.name.clone()
    };
    let run = Run {
        id: String::new(),
        number,
        file: file.id.clone(),
        pipeline: name.clone(),
        project,
        state: RunState::Running,
        started: now,
        finished: None,
        by: ME.to_owned(),
        steps: order
            .iter()
            .map(|step| RunStepRecord {
                id: step.id.clone(),
                kind: step.kind.clone(),
                needs: step.needs.clone(),
                state: StepState::Queued,
                duration_ms: None,
                rows: None,
            })
            .collect(),
        log: vec![LogLine {
            at: 0,
            step: None,
            level: Level::Info,
            text: format!(
                "run {number} of {name} started by {ME}{}",
                if draft { ", from unsaved changes" } else { "" }
            ),
        }],
        failed: None,
        draft,
    };
    let going = Going {
        pipeline,
        order,
        project,
        number,
        started: now,
        results: HashMap::new(),
        current: run.clone(),
        timer: false,
    };
    Ok((run, going))
}

impl Going {
    /// The run takes the id the store gave it.
    pub fn stored_as(&mut self, id: &str) {
        self.current.id = id.to_owned();
    }

    fn filler(&self) -> Filler<'_> {
        Filler {
            params: &self.pipeline.params,
            number: self.number,
            date: iso(self.started)[..10].to_owned(),
            started: iso(self.started),
            results: &self.results,
        }
    }

    fn save(&mut self, patch: Patch, stored: &mut Run) {
        patch.apply(&mut self.current);
        patch.apply(stored);
    }

    fn log(&self, step: Option<&str>, level: Level, text: String, now: i64) -> Vec<LogLine> {
        let mut log = self.current.log.clone();
        log.push(LogLine {
            at: now - self.started,
            step: step.map(str::to_owned),
            level,
            text,
        });
        log
    }

    fn steps_with(&self, id: &str, change: impl Fn(&RunStepRecord) -> RunStepRecord) -> Vec<RunStepRecord> {
        self.current
            .steps
            .iter()
            .map(|one| if one.id == id { change(one) } else { one.clone() })
            .collect()
    }

    /// Moves on from step `index` at `now`: skips the steps whose needs did not succeed, starts the
    /// next one, or ends the run. Answers with the timer the started step sets.
    pub fn next(&mut self, mut index: usize, now: i64, stored: &mut Run) -> Option<Due> {
        loop {
            let Some(step) = self.order.get(index).cloned() else {
                let failed = self
                    .current
                    .steps
                    .iter()
                    .find(|one| one.state == StepState::Failed)
                    .cloned();
                self.timer = false;
                let (level, text) = match &failed {
                    None => (Level::Info, "run succeeded".to_owned()),
                    Some(one) => (Level::Error, format!("run failed at {}", one.id)),
                };
                let log = self.log(None, level, text, now);
                self.save(
                    Patch {
                        state: Some(if failed.is_none() {
                            RunState::Succeeded
                        } else {
                            RunState::Failed
                        }),
                        finished: Some(now),
                        log: Some(log),
                        ..Patch::default()
                    },
                    stored,
                );
                return None;
            };
            let blocked = step.kind != "notify"
                && step.needs.iter().any(|need| {
                    self.current
                        .steps
                        .iter()
                        .find(|one| &one.id == need)
                        .is_none_or(|one| one.state != StepState::Succeeded)
                });
            if blocked {
                let steps = self.steps_with(&step.id, |one| RunStepRecord {
                    state: StepState::Skipped,
                    ..one.clone()
                });
                let log = self.log(
                    Some(&step.id),
                    Level::Info,
                    "skipped: a step it needs did not succeed".to_owned(),
                    now,
                );
                self.save(
                    Patch {
                        steps: Some(steps),
                        log: Some(log),
                        ..Patch::default()
                    },
                    stored,
                );
                index += 1;
                continue;
            }
            let steps = self.steps_with(&step.id, |one| RunStepRecord {
                state: StepState::Running,
                ..one.clone()
            });
            let tries = if step.retries > 0.0 {
                format!(", up to {} tries", number_to_string(step.retries + 1.0))
            } else {
                String::new()
            };
            let log = self.log(Some(&step.id), Level::Info, format!("started{tries}"), now);
            self.save(
                Patch {
                    steps: Some(steps),
                    log: Some(log),
                    ..Patch::default()
                },
                stored,
            );
            let took = 500 + i64::from(hash(&format!("{}{}", step.id, step.kind)) % 900);
            self.timer = true;
            return Some(Due {
                at: now + took,
                index,
                begun: now,
            });
        }
    }

    /// Step `index`'s timer fires at `now`: the step is tried (again, up to its retries), its outcome
    /// kept, and the run moves on. Answers with the next timer; none when the run ended or stalled.
    pub fn fire(&mut self, due: Due, now: i64, stored: &mut Run) -> Option<Due> {
        let step = self.order.get(due.index)?.clone();
        let at = now - self.started;
        let mut lines = self.current.log.clone();
        let line = |level: Level, text: String| LogLine {
            at,
            step: Some(step.id.clone()),
            level,
            text,
        };
        let outcome = if step.kind == "notify" {
            let failed_so_far = self.current.steps.iter().any(|one| one.state == StepState::Failed);
            let state = if failed_so_far { "failed" } else { "succeeded" };
            let on: Vec<Value> = match step.config.get("on") {
                Some(Value::Array(on)) => on.clone(),
                _ => vec![Value::String("failed".to_owned())],
            };
            let channel = as_string(step.config.get("channel"), "");
            let filler = self.filler();
            let message = filler.fill(&as_string(
                step.config.get("message"),
                &format!("{} {state}", self.pipeline.name),
            ));
            let said = if on.iter().any(|one| one.as_str() == Some(state)) {
                format!("posted to {channel}: {message}")
            } else {
                let on: Vec<String> = on
                    .iter()
                    .map(|one| if one.is_null() { String::new() } else { js_string(one) })
                    .collect();
                format!("nothing to post: the run {state}, and on is {}", on.join(", "))
            };
            Outcome {
                ok: true,
                lines: vec![(Level::Info, said)],
                ..Outcome::default()
            }
        } else {
            let before: Vec<&Done> = step.needs.iter().filter_map(|need| self.results.get(need)).collect();
            let filler = self.filler();
            let mut outcome = attempt(&step, &filler, &before, self.project)?;
            let mut tried = 1.0;
            while !outcome.ok && tried <= step.retries {
                for (level, text) in &outcome.lines {
                    lines.push(line(*level, text.clone()));
                }
                lines.push(line(
                    Level::Warn,
                    format!("try {} failed; trying again", number_to_string(tried)),
                ));
                outcome = attempt(&step, &filler, &before, self.project)?;
                tried += 1.0;
            }
            outcome
        };
        for (level, text) in &outcome.lines {
            lines.push(line(*level, text.clone()));
        }
        let duration = now - due.begun;
        self.results.insert(
            step.id.clone(),
            Done {
                rows: outcome.rows,
                rejected: outcome.rejected,
                output: outcome.output.clone(),
            },
        );
        self.current.log = lines;
        let steps = self.steps_with(&step.id, |one| RunStepRecord {
            state: if outcome.ok {
                StepState::Succeeded
            } else {
                StepState::Failed
            },
            duration_ms: Some(duration),
            rows: outcome.rows.or(one.rows),
            ..one.clone()
        });
        let (level, text) = if outcome.ok {
            (Level::Info, "succeeded")
        } else {
            (Level::Error, "failed")
        };
        let log = self.log(Some(&step.id), level, text.to_owned(), now);
        let failed = (!outcome.ok && self.current.failed.is_none()).then(|| Failure {
            step: step.id.clone(),
            reason: outcome.reason.clone().unwrap_or_else(|| "It failed.".to_owned()),
        });
        self.save(
            Patch {
                steps: Some(steps),
                log: Some(log),
                failed,
                ..Patch::default()
            },
            stored,
        );
        self.next(due.index + 1, now, stored)
    }
}

/// The change a cancel makes to a stored run at `now`: the step running and those ahead are
/// cancelled, and the log says who did it.
pub fn cancelled(run: &Run, now: i64) -> Patch {
    let mut log = run.log.clone();
    log.push(LogLine {
        at: now - run.started,
        step: None,
        level: Level::Warn,
        text: format!("cancelled by {ME}"),
    });
    Patch {
        state: Some(RunState::Cancelled),
        finished: Some(now),
        steps: Some(
            run.steps
                .iter()
                .map(|step| match step.state {
                    StepState::Running | StepState::Queued => RunStepRecord {
                        state: StepState::Cancelled,
                        ..step.clone()
                    },
                    _ => step.clone(),
                })
                .collect(),
        ),
        log: Some(log),
        failed: None,
    }
}
