//! Where a run stands at a moment, worked out from its plan, as the web's `timeline.ts`: when each
//! step starts and ends, which one fails, and whether it was cancelled. The console draws every run
//! through this, and the engine keeps the stored state in step with it.

use std::collections::{HashMap, HashSet};

use crate::model::{Run, RunStepPlan, Status};

/// One step of a run at a moment, as sokkel-ui's `RunStep` with when it starts and ends.
#[derive(Clone, Debug, PartialEq)]
pub struct StepAt {
    pub code: String,
    pub kind: String,
    pub needs: Vec<String>,
    pub status: Status,
    /// How far it has gone, 0 to 1, while it runs or where it was cancelled.
    pub progress: Option<f64>,
    /// How long it ran, in milliseconds; none while it waits or once skipped.
    pub duration_ms: Option<i64>,
    pub start: i64,
    pub end: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunAt {
    pub state: Status,
    pub steps: Vec<StepAt>,
    /// The step running now, or the one it failed or stopped at, or its last.
    pub step: Option<String>,
    /// When its first step started or will start.
    pub started: i64,
    pub ended: Option<i64>,
    /// How long it has run, in milliseconds.
    pub duration: i64,
}

/// The steps' indexes in an order where every step comes after the steps it needs; a step whose
/// needs never come (a loop) goes first among those left.
fn ordered(steps: &[RunStepPlan]) -> Vec<usize> {
    let mut done: HashSet<&str> = HashSet::new();
    let mut out = Vec::new();
    let mut left: Vec<usize> = (0..steps.len()).collect();
    while !left.is_empty() {
        let ready: Vec<usize> = left
            .iter()
            .copied()
            .filter(|&index| {
                steps[index]
                    .needs
                    .iter()
                    .all(|need| done.contains(need.as_str()) || !steps.iter().any(|one| &one.code == need))
            })
            .collect();
        let next = if ready.is_empty() { vec![left[0]] } else { ready };
        for &index in &next {
            done.insert(steps[index].code.as_str());
            out.push(index);
        }
        left.retain(|index| !next.contains(index));
    }
    out
}

#[derive(Clone, Copy)]
struct Timed {
    start: i64,
    end: i64,
    outcome: Status,
}

/// When each step starts and ends and how it ends, were nothing cancelled.
fn plan(run: &Run) -> Vec<Timed> {
    let base = run.created + run.wait;
    let mut timed: HashMap<&str, Timed> = HashMap::new();
    for index in ordered(&run.steps) {
        let step = &run.steps[index];
        let before: Vec<Timed> = step
            .needs
            .iter()
            .filter_map(|need| timed.get(need.as_str()).copied())
            .collect();
        let start = before.iter().map(|one| one.end).fold(base, i64::max);
        let blocked = before
            .iter()
            .any(|one| matches!(one.outcome, Status::Failed | Status::Skipped));
        let entry = if blocked {
            Timed {
                start,
                end: start,
                outcome: Status::Skipped,
            }
        } else {
            let failed = run.fail_at.as_deref() == Some(step.code.as_str());
            Timed {
                start,
                end: start + step.ms,
                outcome: if failed { Status::Failed } else { Status::Succeeded },
            }
        };
        timed.insert(step.code.as_str(), entry);
    }
    run.steps
        .iter()
        .map(|step| {
            timed.get(step.code.as_str()).copied().unwrap_or(Timed {
                start: base,
                end: base,
                outcome: Status::Skipped,
            })
        })
        .collect()
}

/// Where the run stands at `now`.
pub fn run_at(run: &Run, now: i64) -> RunAt {
    let base = run.created + run.wait;
    if run.skipped.is_some() {
        return RunAt {
            state: Status::Skipped,
            steps: run
                .steps
                .iter()
                .map(|step| StepAt {
                    code: step.code.clone(),
                    kind: step.kind.clone(),
                    needs: step.needs.clone(),
                    status: Status::Skipped,
                    progress: None,
                    duration_ms: None,
                    start: run.created,
                    end: run.created,
                })
                .collect(),
            step: None,
            started: run.created,
            ended: Some(run.created),
            duration: 0,
        };
    }
    let cancelled_at = run.cancelled_at.filter(|at| *at <= now);
    let cancelled = cancelled_at.is_some();
    let at = cancelled_at.unwrap_or(now);
    let timed = plan(run);
    let steps: Vec<StepAt> = run
        .steps
        .iter()
        .zip(&timed)
        .map(|(step, one)| {
            let Timed { start, end, outcome } = *one;
            let mut progress = None;
            let status = if at < start || (outcome == Status::Skipped && at < end) {
                if cancelled { Status::Skipped } else { Status::Queued }
            } else if at < end {
                progress = Some((at - start) as f64 / (end - start).max(1) as f64);
                if cancelled { Status::Cancelled } else { Status::Running }
            } else {
                outcome
            };
            let ran = (!matches!(status, Status::Queued | Status::Skipped)).then(|| at.min(end) - start);
            StepAt {
                code: step.code.clone(),
                kind: step.kind.clone(),
                needs: step.needs.clone(),
                status,
                progress,
                duration_ms: ran,
                start,
                end,
            }
        })
        .collect();
    let last = timed.iter().map(|one| one.end).fold(base, i64::max);
    let state = if cancelled {
        Status::Cancelled
    } else if now < base {
        Status::Queued
    } else if now < last {
        Status::Running
    } else if run.fail_at.is_some() {
        Status::Failed
    } else {
        Status::Succeeded
    };
    let ended = if cancelled {
        Some(at)
    } else if now >= last {
        Some(last)
    } else {
        None
    };
    let running = steps
        .iter()
        .find(|step| matches!(step.status, Status::Running | Status::Cancelled));
    let failed = steps.iter().find(|step| step.status == Status::Failed);
    let step = running
        .or(failed)
        .map(|step| step.code.clone())
        .or_else(|| (state != Status::Queued).then(|| steps.last().map(|step| step.code.clone()))?);
    let duration = if state == Status::Queued {
        0
    } else {
        (ended.unwrap_or(at) - base).max(0)
    };
    RunAt {
        state,
        steps,
        step,
        started: base,
        ended,
        duration,
    }
}

/// Whether a run has yet to settle.
pub fn is_live(state: Status) -> bool {
    matches!(state, Status::Running | Status::Queued)
}
