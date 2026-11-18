//! Tidevann's jobs, and the runs they would have had over a span of time, as the web's `seed.ts`:
//! each schedule's fires and a few runs started by hand, drawn from a seeded generator so the same
//! span gives the same runs.

use std::cell::Cell;
use std::sync::LazyLock;

use serde_json::{Map, Value, json};

use crate::model::{
    By, Cadence, DAY, HOUR, Job, JobStep, PEOPLE, Person, Project, Run, RunStepPlan, Schedule, Status, Trigger, base36,
    day_of, js_round,
};
use crate::schedule::{FIRES, fires_between};
use crate::timeline::run_at;

pub static JOBS: LazyLock<Vec<Job>> = LazyLock::new(|| {
    vec![
        Job {
            id: "nightly-export".to_owned(),
            name: "Nightly export".to_owned(),
            description: "Exports the day's orders from harbor to the archive, then tells the team it is there."
                .to_owned(),
            project: Project::Harbor,
            owner: Person::Ada,
            steps: vec![
                JobStep::new("fetch-orders", "http", &[]),
                JobStep::new("fetch-inventory", "http", &[]),
                JobStep::new("transform", "sql", &["fetch-orders", "fetch-inventory"]),
                JobStep::new("validate", "schema", &["transform"]),
                JobStep::new("upload", "storage", &["validate"]),
                JobStep::new("notify", "webhook", &["upload"]),
            ],
            schedule: Some(Schedule::new(Cadence::Daily, "02:00", "2026-09-01")),
            paused: false,
            retries: 2,
            timeout: Some(60),
            pace: 45.0,
            risk: 0.14,
        },
        Job {
            id: "sync-inventory".to_owned(),
            name: "Inventory sync".to_owned(),
            description: "Brings larkspur's stock levels in from the warehouse api every hour.".to_owned(),
            project: Project::Larkspur,
            owner: Person::Linus,
            steps: vec![
                JobStep::new("fetch", "http", &[]),
                JobStep::new("transform", "python", &["fetch"]),
                JobStep::new("validate", "schema", &["transform"]),
                JobStep::new("upload", "sql", &["validate"]),
            ],
            schedule: Some(Schedule::new(Cadence::Hourly, "00:15", "2026-09-01")),
            paused: false,
            retries: 3,
            timeout: Some(15),
            pace: 16.0,
            risk: 0.06,
        },
        Job {
            id: "rebuild-index".to_owned(),
            name: "Search index rebuild".to_owned(),
            description: "Rebuilds the search index of harbor from scratch, then swaps it in once it checks out."
                .to_owned(),
            project: Project::Harbor,
            owner: Person::Grace,
            steps: vec![
                JobStep::new("fetch", "http", &[]),
                JobStep::new("transform", "python", &["fetch"]),
                JobStep::new("validate", "schema", &["transform"]),
                JobStep::new("upload", "storage", &["validate"]),
                JobStep::new("notify", "webhook", &["upload"]),
            ],
            schedule: Some(Schedule::new(Cadence::Weekly, "03:30", "2026-09-06")),
            paused: false,
            retries: 1,
            timeout: Some(120),
            pace: 260.0,
            risk: 0.1,
        },
        Job {
            id: "send-digest".to_owned(),
            name: "Morning digest".to_owned(),
            description:
                "Writes yesterday's runs into one email and sends it to larkspur's team before the day starts."
                    .to_owned(),
            project: Project::Larkspur,
            owner: Person::Margaret,
            steps: vec![
                JobStep::new("fetch", "http", &[]),
                JobStep::new("transform", "python", &["fetch"]),
                JobStep::new("notify", "email", &["transform"]),
            ],
            schedule: Some(Schedule::new(Cadence::Weekdays, "07:00", "2026-09-01")),
            paused: false,
            retries: 2,
            timeout: Some(10),
            pace: 5.0,
            risk: 0.05,
        },
        Job {
            id: "import-orders".to_owned(),
            name: "Order import".to_owned(),
            description:
                "Picks up juniper's new orders every quarter of an hour and loads them for the morning's shipping."
                    .to_owned(),
            project: Project::Juniper,
            owner: Person::Grace,
            steps: vec![
                JobStep::new("fetch", "http", &[]),
                JobStep::new("validate", "schema", &["fetch"]),
                JobStep::new("transform", "sql", &["validate"]),
                JobStep::new("upload", "sql", &["transform"]),
            ],
            schedule: Some(Schedule::new(Cadence::Quarter, "00:00", "2026-09-01")),
            paused: false,
            retries: 3,
            timeout: Some(10),
            pace: 7.0,
            risk: 0.11,
        },
    ]
});

/// A generator of numbers from 0 to 1, the same for the same seed: the web's linear congruential
/// one on 32 bits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seeded {
    state: u32,
}

impl Seeded {
    /// The next number, from 0 up to 1.
    pub fn draw(&mut self) -> f64 {
        self.state = self.state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(self.state) / 4_294_967_296.0
    }
}

/// A generator seeded with `seed`, read as `seed >>> 0` reads a whole number.
pub fn seeded(seed: i64) -> Seeded {
    Seeded { state: seed as u32 }
}

/// The value at `at`, from 0 up to 1, along the values; none of none.
pub fn pick<V>(values: &[V], at: f64) -> Option<&V> {
    if values.is_empty() {
        return None;
    }
    values.get((at * values.len() as f64).floor() as usize % values.len())
}

/// What a step says when it fails, by the word its code starts with.
pub fn failure_of(step: &str, job: &Job) -> String {
    match step.split('-').next().unwrap_or_default() {
        "fetch" => format!(
            "GET https://api.example.org/{}/{} answered 503 after {} tries",
            job.project.word(),
            job.id,
            job.retries + 1
        ),
        "transform" => "column sku is missing from 12 rows, first at row 48".to_owned(),
        "validate" => "amount must be a number (37 rows), first at row 112".to_owned(),
        "upload" => "storage.example.org refused the upload: the bucket is over its quota".to_owned(),
        _ => "smtp.example.org did not answer within 30s".to_owned(),
    }
}

/// What a run of a job is given.
pub fn inputs_of(job: &Job, at: i64) -> Map<String, Value> {
    let day = day_of(at - DAY);
    let project = job.project.word();
    let value = match job.id.as_str() {
        "nightly-export" => json!({
            "day": day,
            "project": project,
            "format": "csv",
            "target": format!("https://storage.example.org/{project}/exports/"),
        }),
        "sync-inventory" => json!({
            "project": project,
            "source": format!("https://api.example.org/{project}/inventory"),
            "dry_run": false,
        }),
        "rebuild-index" => json!({ "project": project, "shards": 6, "full": true }),
        "send-digest" => json!({ "project": project, "day": day, "to": format!("{project}-team@example.org") }),
        _ => json!({
            "project": project,
            "source": format!("https://api.example.org/{project}/orders"),
            "batch": 500,
        }),
    };
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

/// A run as it is asked for.
#[derive(Clone, Debug)]
pub struct RunOrder<'a> {
    pub job: &'a Job,
    pub at: i64,
    pub by: By,
    pub trigger: Trigger,
    pub retry_of: Option<String>,
    /// Seconds a step takes, about; the job's own pace when none.
    pub pace: Option<f64>,
    /// The share of runs that fail; the job's own when none.
    pub risk: Option<f64>,
}

impl<'a> RunOrder<'a> {
    pub fn new(job: &'a Job, at: i64, by: By, trigger: Trigger) -> Self {
        Self {
            job,
            at,
            by,
            trigger,
            retry_of: None,
            pace: None,
            risk: None,
        }
    }
}

thread_local! {
    /// How many runs were planned, as the web's module counts them: a run made without an id is
    /// named with it.
    static MADE: Cell<u64> = const { Cell::new(0) };
}

/// A run of a job as it is asked for: its plan, its outcome and what it is given.
pub fn plan_run(order: &RunOrder<'_>, random: &mut Seeded, id: Option<String>) -> Run {
    let job = order.job;
    let at = order.at;
    let pace = order.pace.unwrap_or(job.pace);
    let steps: Vec<RunStepPlan> = job
        .steps
        .iter()
        .map(|step| RunStepPlan {
            code: step.code.clone(),
            kind: step.kind.clone(),
            needs: step.needs.clone(),
            ms: js_round(pace * 1000.0 * (0.45 + random.draw() * 1.2)) as i64,
        })
        .collect();
    let fails = random.draw() < order.risk.unwrap_or(job.risk) && !steps.is_empty();
    let fail_at = if fails {
        pick(&steps, random.draw()).map(|step| step.code.clone())
    } else {
        None
    };
    let made = MADE.with(|made| {
        made.set(made.get() + 1);
        made.get()
    });
    let id = id.unwrap_or_else(|| format!("run_{}{}", base36(at.div_euclid(1000)), base36(made as i64)));
    let wait = js_round(if order.pace.is_none() {
        400.0 + random.draw() * 3600.0
    } else {
        1200.0 + random.draw() * 800.0
    }) as i64;
    let error = fail_at.as_deref().map(|step| failure_of(step, job));
    let roll = random.draw();
    Run {
        id,
        job: job.id.clone(),
        project: job.project,
        by: order.by,
        trigger: order.trigger,
        retry_of: order.retry_of.clone(),
        created: at,
        wait,
        steps,
        fail_at,
        error,
        skipped: None,
        cancelled_at: None,
        cancelled_by: None,
        records: js_round(roll * roll * 250_000.0) as i64,
        inputs: inputs_of(job, at),
        state: Status::Queued,
        ended: None,
    }
}

/// The id of the run a schedule starts at a fire, the same wherever it is made, so a fire is run
/// once.
pub fn scheduled_id(job: &str, fire: i64) -> String {
    let mark = job
        .encode_utf16()
        .fold(7_i64, |sum, unit| (sum * 31 + i64::from(unit)) % 1296);
    format!("run_{}{:0>2}", base36(fire.div_euclid(1000)), base36(mark))
}

/// Stamps the state a run stands at by `now`.
pub fn settle(run: Run, now: i64) -> Run {
    let at = run_at(&run, now);
    Run {
        state: at.state,
        ended: at.ended,
        ..run
    }
}

/// The runs Tidevann's jobs had after `from`, up to `now`: one for each fire of each schedule, a
/// few of them skipped or cancelled, and about one an hour started by hand. Those not done by
/// `now` are still going.
pub fn history(jobs: &[Job], from: i64, now: i64) -> Vec<Run> {
    let mut random = seeded(from.div_euclid(60_000));
    let mut runs = Vec::new();
    for job in jobs {
        let Some(schedule) = job.schedule.as_ref().filter(|_| !job.paused) else {
            continue;
        };
        for at in fires_between(schedule, from, now, FIRES) {
            let order = RunOrder::new(job, at, By::Schedule, Trigger::Schedule);
            let mut run = plan_run(&order, &mut random, Some(scheduled_id(&job.id, at)));
            let roll = random.draw();
            if roll < 0.015 {
                run.skipped = Some("No worker slot was free within ten minutes.".to_owned());
            } else if roll < 0.03 {
                let span: i64 = run.steps.iter().map(|step| step.ms).sum();
                run.cancelled_at = Some(at + run.wait + js_round(span as f64 * random.draw() * 0.8) as i64);
                run.cancelled_by = pick(&PEOPLE, random.draw()).copied();
            }
            runs.push(settle(run, now));
        }
    }
    let hours = (now - from).div_euclid(HOUR);
    for hour in 0..hours {
        if random.draw() < 0.45 {
            continue;
        }
        let Some(job) = pick(jobs, random.draw()) else {
            continue;
        };
        let at = from + hour * HOUR + js_round(random.draw() * HOUR as f64) as i64;
        if at > now {
            continue;
        }
        let Some(by) = pick(&PEOPLE, random.draw()).copied() else {
            continue;
        };
        let order = RunOrder::new(job, at, By::Person(by), Trigger::Manual);
        let id = format!("run_{}m", base36(at.div_euclid(1000)));
        runs.push(settle(plan_run(&order, &mut random, Some(id)), now));
    }
    runs.sort_by_key(|run| std::cmp::Reverse(run.created));
    runs
}

/// How far back the first runs go.
pub const HISTORY: i64 = 48 * HOUR;

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::timeline::run_at;

    pub const NOW: i64 = 1_794_992_400_000;

    /// FNV-1a over the text's bytes, as the bun script hashes the same lines.
    pub fn fnv(text: &str) -> u32 {
        text.bytes().fold(0x811c_9dc5_u32, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
        })
    }

    fn or_empty<T: ToString>(value: Option<T>) -> String {
        value.map(|one| one.to_string()).unwrap_or_default()
    }

    /// A run on one line, as the bun script writes it.
    pub fn line(run: &Run, now: i64) -> String {
        let at = run_at(run, now);
        let ms: Vec<String> = run.steps.iter().map(|step| step.ms.to_string()).collect();
        [
            run.id.clone(),
            run.job.clone(),
            run.by.word().to_owned(),
            run.trigger.word().to_owned(),
            run.created.to_string(),
            run.wait.to_string(),
            ms.join(","),
            or_empty(run.fail_at.as_ref()),
            or_empty(run.skipped.as_ref()),
            or_empty(run.cancelled_at),
            or_empty(run.cancelled_by.map(Person::word)),
            run.records.to_string(),
            serde_json::to_string(&run.inputs).unwrap(),
            run.state.word().to_owned(),
            or_empty(run.ended),
            at.started.to_string(),
            at.duration.to_string(),
            or_empty(at.step),
        ]
        .join("|")
    }

    #[test]
    fn generates_as_the_web() {
        let mut random = seeded(42);
        let drawn = [random.draw(), random.draw(), random.draw()];
        assert_eq!(drawn, [0.2523451747838408, 0.08812504541128874, 0.5772811982315034]);
        let mut big = seeded(NOW);
        assert_eq!([big.draw(), big.draw()], [0.5323052329476923, 0.6039402303285897]);
    }

    #[test]
    fn names_and_gives_as_the_web() {
        let ids: Vec<String> = JOBS.iter().map(|job| scheduled_id(&job.id, NOW)).collect();
        assert_eq!(
            ids,
            [
                "run_toox00eb",
                "run_toox00a9",
                "run_toox00d5",
                "run_toox00c2",
                "run_toox00pi"
            ]
        );
        let said: Vec<String> = ["fetch-orders", "transform", "validate", "upload", "notify"]
            .iter()
            .map(|step| failure_of(step, &JOBS[0]))
            .collect();
        assert_eq!(
            said,
            [
                "GET https://api.example.org/harbor/nightly-export answered 503 after 3 tries",
                "column sku is missing from 12 rows, first at row 48",
                "amount must be a number (37 rows), first at row 112",
                "storage.example.org refused the upload: the bucket is over its quota",
                "smtp.example.org did not answer within 30s",
            ]
        );
        let inputs: Vec<String> = JOBS
            .iter()
            .map(|job| serde_json::to_string(&inputs_of(job, NOW)).unwrap())
            .collect();
        assert_eq!(
            inputs,
            [
                r#"{"day":"2026-11-17","project":"harbor","format":"csv","target":"https://storage.example.org/harbor/exports/"}"#,
                r#"{"project":"larkspur","source":"https://api.example.org/larkspur/inventory","dry_run":false}"#,
                r#"{"project":"harbor","shards":6,"full":true}"#,
                r#"{"project":"larkspur","day":"2026-11-17","to":"larkspur-team@example.org"}"#,
                r#"{"project":"juniper","source":"https://api.example.org/juniper/orders","batch":500}"#,
            ]
        );
    }

    #[test]
    fn history_is_the_webs() {
        let runs = history(&JOBS, NOW - HISTORY, NOW);
        assert_eq!(runs.len(), 271);
        let lines: Vec<String> = runs.iter().map(|run| line(run, NOW)).collect();
        assert_eq!(fnv(&lines.join("\n")), 65_000_764);
        let ends: Vec<(String, String, Status, i64, i64)> = runs[..5]
            .iter()
            .chain(&runs[runs.len() - 5..])
            .map(|run| {
                let at = run_at(run, NOW);
                (run.id.clone(), run.job.clone(), run.state, at.started, at.duration)
            })
            .collect();
        let want = [
            ("run_toox00pi", "import-orders", Status::Queued, 1_794_992_403_787, 0),
            (
                "run_toowb0pi",
                "import-orders",
                Status::Succeeded,
                1_794_991_500_935,
                31_337,
            ),
            (
                "run_toovm0pi",
                "import-orders",
                Status::Succeeded,
                1_794_990_602_592,
                27_893,
            ),
            (
                "run_tooux0a9",
                "sync-inventory",
                Status::Succeeded,
                1_794_989_702_753,
                46_544,
            ),
            (
                "run_tooux0pi",
                "import-orders",
                Status::Succeeded,
                1_794_989_702_223,
                37_529,
            ),
            (
                "run_tol9r0pi",
                "import-orders",
                Status::Succeeded,
                1_794_822_300_767,
                29_189,
            ),
            (
                "run_tol920pi",
                "import-orders",
                Status::Succeeded,
                1_794_821_402_077,
                26_400,
            ),
            (
                "run_tol8d0a9",
                "sync-inventory",
                Status::Succeeded,
                1_794_820_503_645,
                53_042,
            ),
            (
                "run_tol8d0pi",
                "import-orders",
                Status::Failed,
                1_794_820_500_905,
                9_256,
            ),
            (
                "run_tol80nm",
                "rebuild-index",
                Status::Succeeded,
                1_794_820_059_034,
                1_224_723,
            ),
        ];
        let want: Vec<(String, String, Status, i64, i64)> = want
            .iter()
            .map(|(id, job, state, started, duration)| {
                ((*id).to_owned(), (*job).to_owned(), *state, *started, *duration)
            })
            .collect();
        assert_eq!(ends, want);
        let count = |state: Status| runs.iter().filter(|run| run.state == state).count();
        assert_eq!(
            [
                count(Status::Queued),
                count(Status::Succeeded),
                count(Status::Failed),
                count(Status::Skipped)
            ],
            [1, 233, 33, 4]
        );
        assert_eq!(
            lines[0],
            r#"run_toox00pi|import-orders|schedule|schedule|1794992400000|3787|3240,6405,3917,4428|upload||||236396|{"project":"juniper","source":"https://api.example.org/juniper/orders","batch":500}|queued||1794992403787|0|"#
        );
        assert_eq!(
            lines[lines.len() - 1],
            r#"run_tol80nm|rebuild-index|linus|manual|1794820055435|3599|233026,256392,401612,195468,138225|||||36348|{"project":"harbor","shards":6,"full":true}|succeeded|1794821283757|1794820059034|1224723|notify"#
        );
    }

    #[test]
    fn history_cancels_as_the_web() {
        // A minute later the window holds a run cancelled by hand.
        let now = NOW + 60_000;
        let runs = history(&JOBS, now - HISTORY, now);
        assert_eq!(runs.len(), 275);
        let lines: Vec<String> = runs
            .iter()
            .map(|run| {
                [
                    run.id.clone(),
                    run.state.word().to_owned(),
                    run.created.to_string(),
                    or_empty(run.ended),
                    run.records.to_string(),
                    or_empty(run.cancelled_at),
                    or_empty(run.cancelled_by.map(Person::word)),
                ]
                .join("|")
            })
            .collect();
        assert_eq!(fnv(&lines.join("\n")), 1_406_879_058);
        let one = runs.iter().find(|run| run.cancelled_at.is_some()).unwrap();
        assert_eq!(one.id, "run_too5x0pi");
        assert_eq!(one.state, Status::Cancelled);
        assert_eq!(one.cancelled_at, Some(1_794_957_324_405));
        assert_eq!(one.cancelled_by, Some(Person::Linus));
        assert_eq!(one.ended, Some(1_794_957_324_405));
    }
}
