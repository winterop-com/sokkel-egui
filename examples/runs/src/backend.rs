//! The console's server, as the web's `backend.ts` fakes it in the page: the jobs, runs, saved views
//! and preferences, the calls that change them, and an engine that moves runs along and starts them
//! on their schedules. Every call is at once and takes the moment it is made at; rows are kept as
//! the web's collections keep them, the newest first.

use std::sync::LazyLock;

use sokkel_egui::{Filter, FilterOperator, ListViewState};

use crate::model::{By, DAY, Job, JobStep, Person, Prefs, Project, Run, RunView, Schedule, Status, Trigger};
use crate::schedule::fires_between;
use crate::seed::{HISTORY, JOBS, RunOrder, Seeded, history, plan_run, scheduled_id, seeded, settle};
use crate::timeline::{is_live, run_at};

/// Runs started from the console go quickly, a second or two a step, so they settle while watched:
/// seconds a step takes, about, and the share that fail.
pub const QUICK: (f64, f64) = (1.6, 0.25);

/// How often the engine looks, in milliseconds.
pub const TICK: i64 = 1000;

/// The scheduler fires a schedule at most this many times in one look.
const DUE: usize = 4;

/// The saved views the console starts with.
pub fn default_views() -> Vec<RunView> {
    vec![
        RunView {
            id: "all".to_owned(),
            name: "All runs".to_owned(),
            state: ListViewState::default(),
            locked: true,
        },
        RunView {
            id: "failed".to_owned(),
            name: "Failures".to_owned(),
            state: ListViewState {
                filters: vec![Filter::new("state", FilterOperator::OneOf, ["failed"])],
                sort: Some(("started".to_owned(), false)),
                ..ListViewState::default()
            },
            locked: false,
        },
        RunView {
            id: "mine".to_owned(),
            name: "Started by me".to_owned(),
            state: ListViewState {
                filters: vec![Filter::new("by", FilterOperator::OneOf, ["ada"])],
                hidden: vec!["by".to_owned()],
                ..ListViewState::default()
            },
            locked: false,
        },
    ]
}

/// The steps a new job starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Starter {
    Pipeline,
    Sync,
    Report,
}

/// A starter of a new job: its steps and the label it is offered with.
#[derive(Clone, Debug, PartialEq)]
pub struct StarterOption {
    pub value: Starter,
    pub label: &'static str,
    pub steps: Vec<JobStep>,
}

pub static STARTERS: LazyLock<Vec<StarterOption>> = LazyLock::new(|| {
    vec![
        StarterOption {
            value: Starter::Pipeline,
            label: "Fetch, transform, validate, upload and notify",
            steps: vec![
                JobStep::new("fetch", "http", &[]),
                JobStep::new("transform", "sql", &["fetch"]),
                JobStep::new("validate", "schema", &["transform"]),
                JobStep::new("upload", "storage", &["validate"]),
                JobStep::new("notify", "webhook", &["upload"]),
            ],
        },
        StarterOption {
            value: Starter::Sync,
            label: "Fetch, validate and upload",
            steps: vec![
                JobStep::new("fetch", "http", &[]),
                JobStep::new("validate", "schema", &["fetch"]),
                JobStep::new("upload", "sql", &["validate"]),
            ],
        },
        StarterOption {
            value: Starter::Report,
            label: "Fetch, transform and notify",
            steps: vec![
                JobStep::new("fetch", "http", &[]),
                JobStep::new("transform", "python", &["fetch"]),
                JobStep::new("notify", "email", &["transform"]),
            ],
        },
    ]
});

/// What a new job is made from: its name and code, project and owner, and its steps from a starter.
#[derive(Clone, Debug, PartialEq)]
pub struct NewJob {
    pub id: String,
    pub name: String,
    pub description: String,
    pub project: Project,
    pub owner: Person,
    pub starter: Starter,
    pub schedule: Option<Schedule>,
}

/// What the engine tells the console: a run that settled.
#[derive(Clone, Debug, PartialEq)]
pub struct Settled {
    pub run: Run,
    pub job: String,
}

/// The console's data and the engine that moves it along.
#[derive(Clone, Debug)]
pub struct Backend {
    pub jobs: Vec<Job>,
    pub runs: Vec<Run>,
    pub views: Vec<RunView>,
    /// The person's preferences; the web keeps them as a collection of one row.
    pub prefs: Vec<Prefs>,
    /// The generator runs started from the console and by the engine are planned with.
    pub random: Seeded,
    /// When the engine last looked for schedules come due.
    pub looked: i64,
}

impl Backend {
    /// The data as the console starts with it at `now`: Tidevann's jobs, their runs over `HISTORY`
    /// before `now`, the saved views and the default preferences.
    pub fn seeded(now: i64) -> Self {
        Self {
            jobs: JOBS.clone(),
            runs: history(&JOBS, now - HISTORY, now),
            views: default_views(),
            prefs: vec![Prefs::default()],
            random: seeded(now),
            looked: now,
        }
    }

    /// The data as it was kept, the engine's generator seeded at `now`, as the page seeds it when
    /// it loads.
    pub fn kept(jobs: Vec<Job>, runs: Vec<Run>, views: Vec<RunView>, prefs: Vec<Prefs>, now: i64) -> Self {
        Self {
            jobs,
            runs,
            views,
            prefs,
            random: seeded(now),
            looked: now,
        }
    }

    pub fn job(&self, id: &str) -> Option<&Job> {
        self.jobs.iter().find(|job| job.id == id)
    }

    pub fn run(&self, id: &str) -> Option<&Run> {
        self.runs.iter().find(|run| run.id == id)
    }

    /// Changes the run under `id` and gives it back as it now is; none when there is no such run.
    fn update_run(&mut self, id: &str, change: impl FnOnce(&mut Run)) -> Option<Run> {
        let run = self.runs.iter_mut().find(|run| run.id == id)?;
        change(run);
        Some(run.clone())
    }

    /// Starts the engine at `now`: the runs the schedules fired while the console was closed are
    /// filled in from the history, up to a day back, and the scheduler looks from `now` on.
    pub fn start_engine(&mut self, now: i64) {
        self.looked = now;
        let newest = self
            .runs
            .iter()
            .filter(|run| run.by == By::Schedule)
            .map(|run| run.created)
            .fold(0, i64::max);
        let from = newest.max(now - DAY);
        let missed = history(&self.jobs, from, now);
        let added: Vec<Run> = missed
            .into_iter()
            .filter(|run| run.by == By::Schedule && !self.runs.iter().any(|one| one.id == run.id))
            .collect();
        self.runs.splice(0..0, added);
    }

    /// Starts a run of `job` at `now`, queued for a moment before its first step.
    pub fn start_run(&mut self, job: &Job, by: Person, retry_of: Option<&Run>, now: i64) -> Run {
        let order = RunOrder {
            job,
            at: now,
            by: By::Person(by),
            trigger: if retry_of.is_none() {
                Trigger::Manual
            } else {
                Trigger::Retry
            },
            retry_of: retry_of.map(|run| run.id.clone()),
            pace: Some(QUICK.0),
            risk: Some(QUICK.1),
        };
        let run = plan_run(&order, &mut self.random, None);
        self.runs.insert(0, run.clone());
        run
    }

    /// Stops a run that is queued or running: the step running is cancelled and those after it
    /// skipped. None when there is no such run.
    pub fn cancel_run(&mut self, id: &str, by: Person, now: i64) -> Option<Run> {
        self.update_run(id, |run| {
            run.cancelled_at = Some(now);
            run.cancelled_by = Some(by);
            run.state = Status::Cancelled;
            run.ended = Some(now);
        })
    }

    /// Runs the job of a run again, by `by`, with the same inputs.
    pub fn retry_run(&mut self, run: &Run, job: &Job, by: Person, now: i64) -> Run {
        self.start_run(job, by, Some(run), now)
    }

    /// Changes the job under `id` and gives it back as it now is; none when there is no such job.
    pub fn save_job(&mut self, id: &str, change: impl FnOnce(&mut Job)) -> Option<Job> {
        let job = self.jobs.iter_mut().find(|job| job.id == id)?;
        change(job);
        Some(job.clone())
    }

    /// Deletes a job: its runs going are cancelled, and its runs stay in the history.
    pub fn delete_job(&mut self, id: &str, by: Person, now: i64) {
        let going: Vec<String> = self
            .runs
            .iter()
            .filter(|run| run.job == id && is_live(run_at(run, now).state))
            .map(|run| run.id.clone())
            .collect();
        for run in going {
            self.cancel_run(&run, by, now);
        }
        self.jobs.retain(|job| job.id != id);
    }

    /// Makes a job from a starter; an error when a job has the code already.
    pub fn create_job(&mut self, fields: NewJob) -> Result<Job, String> {
        if self.jobs.iter().any(|job| job.id == fields.id) {
            return Err(format!("A job with the code {} is there already.", fields.id));
        }
        let steps = STARTERS
            .iter()
            .find(|one| one.value == fields.starter)
            .map(|one| one.steps.clone())
            .unwrap_or_default();
        let job = Job {
            id: fields.id,
            name: fields.name,
            description: fields.description,
            project: fields.project,
            owner: fields.owner,
            steps,
            schedule: fields.schedule,
            paused: false,
            retries: 2,
            timeout: Some(30),
            pace: 20.0,
            risk: 0.08,
        };
        self.jobs.insert(0, job.clone());
        Ok(job)
    }

    /// One look of the engine at `now`, as the web's does every `TICK`: it stores the state each
    /// live run has reached and gives back those that settled, and it starts the runs whose
    /// schedules came due since it last looked, or skips one whose job still has a run going.
    pub fn tick(&mut self, now: i64) -> Vec<Settled> {
        // The web reads every collection once at the start of a look and works from those rows.
        let all = self.runs.clone();
        let known = self.jobs.clone();
        let scheduler = self.prefs.first().is_none_or(|mine| mine.scheduler);
        let mut settled = Vec::new();
        for run in &all {
            if !is_live(run.state) {
                continue;
            }
            let at = run_at(run, now);
            if at.state == run.state && at.ended == run.ended {
                continue;
            }
            let next = self.update_run(&run.id, |one| {
                one.state = at.state;
                one.ended = at.ended;
            });
            if let Some(next) = next.filter(|_| !is_live(at.state)) {
                settled.push(Settled {
                    run: next,
                    job: run.job.clone(),
                });
            }
        }
        if scheduler {
            let mut due = Vec::new();
            for job in &known {
                let Some(schedule) = job.schedule.as_ref().filter(|_| !job.paused) else {
                    continue;
                };
                for fire in fires_between(schedule, self.looked, now, DUE) {
                    let id = scheduled_id(&job.id, fire);
                    if all.iter().any(|one| one.id == id) {
                        continue;
                    }
                    let order = RunOrder::new(job, fire, By::Schedule, Trigger::Schedule);
                    let mut run = plan_run(&order, &mut self.random, Some(id));
                    if all.iter().any(|one| one.job == job.id && is_live(one.state)) {
                        run.skipped = Some("The run before was still going.".to_owned());
                    }
                    due.push(settle(run, now));
                }
            }
            self.runs.splice(0..0, due);
        }
        self.looked = now;
        settled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::tests::NOW;

    fn ids(runs: &[Run], count: usize) -> Vec<(String, String, Status, Option<String>, i64)> {
        runs[..count]
            .iter()
            .map(|run| {
                (
                    run.id.clone(),
                    run.job.clone(),
                    run.state,
                    run.skipped.clone(),
                    run.created,
                )
            })
            .collect()
    }

    fn row(id: &str, job: &str, state: Status, created: i64) -> (String, String, Status, Option<String>, i64) {
        (id.to_owned(), job.to_owned(), state, None, created)
    }

    /// The same calls, at the same moments, as the bun script makes on the web's backend.
    #[test]
    fn moves_along_as_the_webs() {
        let mut backend = Backend::seeded(NOW);
        assert_eq!(backend.runs.len(), 271);
        let views: Vec<&str> = backend.views.iter().map(|view| view.id.as_str()).collect();
        assert_eq!(views, ["all", "failed", "mine"]);
        backend.start_engine(NOW);
        assert_eq!(backend.runs.len(), 271);

        let sync = backend.jobs[1].clone();
        let started = backend.start_run(&sync, Person::Grace, None, NOW);
        let ms: Vec<i64> = started.steps.iter().map(|step| step.ms).collect();
        assert_eq!(
            (
                started.id.as_str(),
                started.state,
                started.wait,
                ms,
                started.fail_at.clone(),
                started.records
            ),
            (
                "run_toox007k",
                Status::Queued,
                1332,
                vec![1742, 1880, 2348, 1959],
                None,
                33_392
            )
        );

        let later = NOW + 10 * 60_000;
        let settled: Vec<(String, String, Status, Option<i64>)> = backend
            .tick(later)
            .into_iter()
            .map(|one| (one.run.id, one.job, one.run.state, one.run.ended))
            .collect();
        assert_eq!(
            settled,
            [
                (
                    "run_toox007k".to_owned(),
                    "sync-inventory".to_owned(),
                    Status::Succeeded,
                    Some(1_794_992_409_261)
                ),
                (
                    "run_toox00pi".to_owned(),
                    "import-orders".to_owned(),
                    Status::Failed,
                    Some(1_794_992_421_777)
                ),
            ]
        );
        assert_eq!(backend.runs.len(), 272);

        let again = backend.retry_run(&started, &sync, Person::Ada, later);
        let ms: Vec<i64> = again.steps.iter().map(|step| step.ms).collect();
        assert_eq!(
            (
                again.id.as_str(),
                again.trigger,
                again.retry_of.as_deref(),
                again.wait,
                ms
            ),
            (
                "run_tooxgo7l",
                Trigger::Retry,
                Some("run_toox007k"),
                1457,
                vec![1024, 1448, 1801, 2464]
            )
        );
        let cancelled = backend.cancel_run(&again.id, Person::Ada, later).unwrap();
        assert_eq!(
            (cancelled.state, cancelled.cancelled_at, cancelled.ended),
            (Status::Cancelled, Some(later), Some(later))
        );

        backend.delete_job("nightly-export", Person::Linus, later);
        let made = backend
            .create_job(NewJob {
                id: "weekly-report".to_owned(),
                name: "Weekly report".to_owned(),
                description: "A report.".to_owned(),
                project: Project::Juniper,
                owner: Person::Margaret,
                starter: Starter::Report,
                schedule: None,
            })
            .unwrap();
        let codes: Vec<&str> = made.steps.iter().map(|step| step.code.as_str()).collect();
        assert_eq!(codes, ["fetch", "transform", "notify"]);
        assert_eq!(
            (made.retries, made.timeout, made.pace, made.risk),
            (2, Some(30), 20.0, 0.08)
        );
        let jobs: Vec<&str> = backend.jobs.iter().map(|job| job.id.as_str()).collect();
        assert_eq!(
            jobs,
            [
                "weekly-report",
                "sync-inventory",
                "rebuild-index",
                "send-digest",
                "import-orders"
            ]
        );
        let twice = NewJob {
            id: "weekly-report".to_owned(),
            name: String::new(),
            description: String::new(),
            project: Project::Juniper,
            owner: Person::Margaret,
            starter: Starter::Sync,
            schedule: None,
        };
        assert_eq!(
            backend.create_job(twice),
            Err("A job with the code weekly-report is there already.".to_owned())
        );

        assert!(backend.tick(NOW + 70 * 60_000).is_empty());
        assert_eq!(backend.runs.len(), 278);
        assert_eq!(
            ids(&backend.runs, 6),
            [
                row("run_tooxp0a9", "sync-inventory", Status::Succeeded, 1_794_993_300_000),
                row("run_tooxp0pi", "import-orders", Status::Succeeded, 1_794_993_300_000),
                row("run_tooye0pi", "import-orders", Status::Succeeded, 1_794_994_200_000),
                row("run_tooz30pi", "import-orders", Status::Failed, 1_794_995_100_000),
                row("run_toozs0pi", "import-orders", Status::Succeeded, 1_794_996_000_000),
                row("run_tooxgo7l", "sync-inventory", Status::Cancelled, 1_794_993_000_000),
            ]
        );
    }
}
