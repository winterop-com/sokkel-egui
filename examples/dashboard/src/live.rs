//! The simulated source the widgets read, as the web's `live.ts`: every tick it takes a reading of
//! the figures, starts and finishes runs of Tidevann's jobs, writes the services' logs, says what
//! happened, and now and then a site slows down and comes back. The last ninety days of uptime
//! come from a fixed seed, so it reads the same every time.

use sokkel_egui::{Day, LogLevel, LogLine, UptimeDay};

use crate::model::{JOBS, Metric, PEOPLE, PROJECTS, SERVICES, STEPS};

/// A deterministic random source, the web's `seeded` (mulberry32), so the page opens the same way.
#[derive(Clone, Debug)]
pub struct Random {
    state: u32,
}

impl Random {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// A number from 0 up to 1.
    pub fn roll(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let s = self.state;
        let mut t = (s ^ (s >> 15)).wrapping_mul(1 | s);
        t = (t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t))) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }

    pub fn pick<T: Copy>(&mut self, list: &[T]) -> T {
        list[((self.roll() * list.len() as f64) as usize).min(list.len() - 1)]
    }

    pub fn between(&mut self, low: f64, high: f64) -> f64 {
        low + self.roll() * (high - low)
    }
}

/// A run's state, of sokkel-ui's run states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Succeeded,
    Failed,
    Running,
    Queued,
    Cancelled,
}

impl RunState {
    pub fn word(self) -> &'static str {
        match self {
            RunState::Succeeded => "succeeded",
            RunState::Failed => "failed",
            RunState::Running => "running",
            RunState::Queued => "queued",
            RunState::Cancelled => "cancelled",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub id: String,
    pub job: &'static str,
    pub project: &'static str,
    pub by: &'static str,
    pub state: RunState,
    pub step: &'static str,
    pub started: f64,
    pub ended: Option<f64>,
}

/// How a happening reads: its tone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Critical,
    Warning,
    Info,
    Good,
    Quiet,
}

/// What a happening is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HappeningKind {
    Run,
    Deploy,
    Site,
    Comment,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Happening {
    pub id: String,
    pub at: f64,
    pub text: String,
    pub tone: Tone,
    pub kind: HappeningKind,
}

/// A site's health.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Operational,
    Degraded,
    Outage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Site {
    pub id: &'static str,
    pub city: &'static str,
    pub country: &'static str,
    /// Longitude and latitude.
    pub at: [f64; 2],
    pub state: Health,
    /// Round trip now, in milliseconds.
    pub latency: f64,
}

/// A reading of every metric, in `Metric::ALL`'s order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub at: f64,
    pub values: [f64; 7],
}

impl Reading {
    pub fn of(&self, metric: Metric) -> f64 {
        self.values[metric.index()]
    }
}

/// How many readings the charts keep.
pub const KEEP: usize = 60;

/// Where Tidevann's sites are, and their resting round trip.
pub const SITES: [(&str, &str, &str, [f64; 2], f64); 11] = [
    ("oslo", "Oslo", "Norway", [10.75, 59.91], 18.0),
    ("dublin", "Dublin", "Ireland", [-6.26, 53.35], 24.0),
    ("frankfurt", "Frankfurt", "Germany", [8.68, 50.11], 16.0),
    ("virginia", "Ashburn", "United States", [-77.49, 39.04], 92.0),
    ("oregon", "Portland", "United States", [-122.68, 45.52], 140.0),
    ("saopaulo", "Sao Paulo", "Brazil", [-46.63, -23.55], 190.0),
    ("mumbai", "Mumbai", "India", [72.88, 19.08], 150.0),
    ("singapore", "Singapore", "Singapore", [103.82, 1.35], 170.0),
    ("tokyo", "Tokyo", "Japan", [139.69, 35.69], 210.0),
    ("sydney", "Sydney", "Australia", [151.21, -33.87], 260.0),
    ("capetown", "Cape Town", "South Africa", [18.42, -33.92], 180.0),
];

fn base_latency(id: &str) -> f64 {
    SITES.iter().find(|s| s.0 == id).map_or(100.0, |s| s.4)
}

fn log_texts(service: &str) -> &'static [&'static str] {
    match service {
        "api" => &[
            "GET /v1/runs 200",
            "POST /v1/runs 201",
            "GET /v1/jobs 200",
            "GET /v1/projects/harbor 200",
            "PATCH /v1/schedules 204",
        ],
        "worker" => &[
            "picked up run",
            "step finished",
            "wrote 1,204 rows",
            "uploaded the archive",
            "heartbeat",
        ],
        _ => &[
            "tick",
            "queued run",
            "next fire computed",
            "lease renewed",
            "schedule loaded",
        ],
    }
}

/// Runs started per job, per project and per person since the page opened, with what the history held.
#[derive(Clone, Debug, PartialEq)]
pub struct Counts {
    pub job: [u32; 5],
    pub project: [u32; 3],
    pub person: [u32; 4],
}

/// The live world, as the web's `LiveState`.
#[derive(Clone, Debug)]
pub struct Live {
    pub at: f64,
    pub readings: Vec<Reading>,
    pub runs: Vec<Run>,
    pub happenings: Vec<Happening>,
    /// Each service's log, in `SERVICES`' order.
    pub logs: [Vec<LogLine>; 3],
    pub sites: Vec<Site>,
    /// Each service's last ninety days, in `SERVICES`' order.
    pub uptime: [Vec<UptimeDay>; 3],
    pub counts: Counts,
    random: Random,
    run_number: u32,
    happening_number: u32,
}

/// The board's clock at the start: Wednesday 18 November 2026 at 09:00, in milliseconds.
pub fn start() -> f64 {
    f64::from(Day::from_ymd(2026, 11, 18).0) * 86_400_000.0 + 9.0 * 3_600_000.0
}

/// The time of day of `at`, `14:02:11`, or with `twelve` `2:02:11 PM`; `seconds` false leaves them off.
pub fn clock_of(at: f64, twelve: bool, seconds: bool) -> String {
    let s = (at / 1000.0).floor() as i64;
    let (h, m, sec) = (
        s.div_euclid(3600).rem_euclid(24),
        s.div_euclid(60).rem_euclid(60),
        s.rem_euclid(60),
    );
    let tail = if seconds { format!(":{sec:02}") } else { String::new() };
    if twelve {
        let hour = if h % 12 == 0 { 12 } else { h % 12 };
        let half = if h < 12 { "AM" } else { "PM" };
        format!("{hour}:{m:02}{tail} {half}")
    } else {
        format!("{h:02}:{m:02}{tail}")
    }
}

/// The day `at` falls on.
pub fn day_of(at: f64) -> Day {
    Day((at / 86_400_000.0).floor() as i32)
}

impl Live {
    fn new_run(&mut self, at: f64, state: RunState) -> Run {
        self.run_number += 1;
        let job = self.random.pick(&JOBS);
        let project = self.random.pick(&PROJECTS);
        let by = self.random.pick(&PEOPLE);
        Run {
            id: format!("run-{}", self.run_number),
            job,
            project,
            by,
            state,
            step: "fetch",
            started: at,
            ended: None,
        }
    }

    fn happening(&mut self, at: f64, text: String, tone: Tone, kind: HappeningKind) -> Happening {
        self.happening_number += 1;
        Happening {
            id: format!("h-{}", self.happening_number),
            at,
            text,
            tone,
            kind,
        }
    }

    fn log_line(&mut self, service: &'static str, at: f64) -> LogLine {
        let roll = self.random.roll();
        let level = if roll < 0.04 {
            LogLevel::Error
        } else if roll < 0.12 {
            LogLevel::Warning
        } else if roll < 0.3 {
            LogLevel::Debug
        } else {
            LogLevel::Info
        };
        let text = match level {
            LogLevel::Error => format!(
                "{} failed at {}: connection reset",
                self.random.pick(&JOBS),
                self.random.pick(&STEPS)
            ),
            LogLevel::Warning => {
                let step = self.random.pick(&STEPS);
                let job = self.random.pick(&JOBS);
                format!("slow {step} for {job}, {}s", self.random.between(2.0, 9.0).round())
            }
            _ => self.random.pick(log_texts(service)).to_owned(),
        };
        let project = self.random.pick(&PROJECTS);
        let ms = self.random.between(4.0, 180.0).round();
        LogLine {
            at: Some(clock_of(at, false, true)),
            level: Some(level),
            source: Some(service.into()),
            text,
            fields: vec![("project".into(), project.into()), ("ms".into(), format!("{ms}"))],
        }
    }

    /// The last ninety days of a service, a few bad days among them.
    fn uptime_of(&mut self, now: f64) -> Vec<UptimeDay> {
        let today = day_of(now);
        (0..90)
            .rev()
            .map(|back| {
                let roll = self.random.roll();
                let uptime = if roll < 0.03 {
                    self.random.between(0.9, 0.97)
                } else if roll < 0.1 {
                    self.random.between(0.985, 0.999)
                } else {
                    1.0
                };
                UptimeDay {
                    date: today.plus(-back),
                    uptime: Some(uptime),
                    status: None,
                    note: None,
                }
            })
            .collect()
    }

    fn next_values(&mut self, before: Option<[f64; 7]>, degraded: f64) -> [f64; 7] {
        let before = before.unwrap_or([42.0, 1.0, 98.0, 12.0, 140.0, 48.0, 61.0]);
        let r = &mut self.random;
        let runs = (before[0] + r.between(-4.0, 4.0)).clamp(18.0, 80.0);
        let failures = (before[1] + r.between(-0.8, 0.7) + degraded * 0.6).clamp(0.0, 9.0);
        let queue = (before[3] + r.between(-3.0, 3.0) + degraded).clamp(0.0, 60.0).round();
        let latency = (before[4] + r.between(-12.0, 12.0) + degraded * 15.0)
            .clamp(70.0, 420.0)
            .round();
        let cpu = (before[5] + r.between(-5.0, 5.0) + degraded * 2.0)
            .clamp(12.0, 98.0)
            .round();
        let memory = (before[6] + r.between(-2.0, 2.0)).clamp(40.0, 92.0).round();
        let (runs, failures) = (runs.round(), failures.round());
        [
            runs,
            failures,
            (100.0 - failures / runs.max(1.0) * 100.0).clamp(80.0, 100.0),
            queue,
            latency,
            cpu,
            memory,
        ]
    }

    /// The world as the page opens on it.
    pub fn begin() -> Self {
        let now = start();
        let every = 3000.0;
        let mut live = Live {
            at: now,
            readings: Vec::new(),
            runs: Vec::new(),
            happenings: Vec::new(),
            logs: [Vec::new(), Vec::new(), Vec::new()],
            sites: SITES
                .iter()
                .map(|(id, city, country, at, latency)| Site {
                    id,
                    city,
                    country,
                    at: *at,
                    state: Health::Operational,
                    latency: *latency,
                })
                .collect(),
            uptime: [Vec::new(), Vec::new(), Vec::new()],
            counts: Counts {
                job: [0; 5],
                project: [0; 3],
                person: [0; 4],
            },
            random: Random::new(20_261_113),
            run_number: 4200,
            happening_number: 0,
        };
        let mut values = None;
        for index in (0..KEEP).rev() {
            let next = live.next_values(values, 0.0);
            values = Some(next);
            live.readings.push(Reading {
                at: now - index as f64 * every,
                values: next,
            });
        }
        for _ in 0..240 {
            let (j, p, q) = (
                live.random.pick(&[0, 1, 2, 3, 4]),
                live.random.pick(&[0, 1, 2]),
                live.random.pick(&[0, 1, 2, 3]),
            );
            live.counts.job[j] += 1;
            live.counts.project[p] += 1;
            live.counts.person[q] += 1;
        }
        for index in (1..=14).rev() {
            let at = now - f64::from(index) * 95_000.0;
            let run = live.new_run(at, RunState::Queued);
            let failed = live.random.roll() < 0.15;
            let step = if failed { live.random.pick(&STEPS) } else { "notify" };
            let done = Run {
                state: if failed { RunState::Failed } else { RunState::Succeeded },
                step,
                ended: Some(at + 60_000.0),
                ..run
            };
            let text = if failed {
                format!("{} failed at {} in {}", done.job, done.step, done.project)
            } else {
                format!("{} succeeded in {}", done.job, done.project)
            };
            let tone = if failed { Tone::Critical } else { Tone::Good };
            let h = live.happening(at + 60_000.0, text, tone, HappeningKind::Run);
            live.runs.insert(0, done);
            live.happenings.insert(0, h);
        }
        let running = Run {
            step: "transform",
            ..live.new_run(now - 40_000.0, RunState::Running)
        };
        live.runs.insert(0, running);
        let queued = live.new_run(now - 5000.0, RunState::Queued);
        live.runs.insert(0, queued);
        let deploy = live.happening(
            now - 600_000.0,
            "grace deployed worker 2.14.0".into(),
            Tone::Info,
            HappeningKind::Deploy,
        );
        live.happenings.insert(0, deploy);
        live.happenings.sort_by(|a, b| b.at.total_cmp(&a.at));
        for (s, service) in SERVICES.iter().enumerate() {
            let lines: Vec<LogLine> = (0..40)
                .map(|index| live.log_line(service, now - f64::from(40 - index) * 1500.0))
                .collect();
            live.logs[s] = lines;
        }
        for s in 0..3 {
            live.uptime[s] = live.uptime_of(now);
        }
        live
    }

    /// One tick of the world, `every` milliseconds after the last.
    pub fn tick(&mut self, every: f64) {
        let at = self.at + every;
        let troubled = self.sites.iter().filter(|s| s.state != Health::Operational).count();
        let last = self.readings.last().map(|r| r.values);
        let values = self.next_values(last, troubled as f64);
        self.readings.push(Reading { at, values });
        if self.readings.len() > KEEP {
            self.readings.remove(0);
        }
        // runs move on: the queued start, the running pass a step or finish, new ones are queued
        let mut said = Vec::new();
        let mut runs = std::mem::take(&mut self.runs);
        for run in &mut runs {
            if run.state == RunState::Queued && self.random.roll() < 0.6 {
                run.state = RunState::Running;
                continue;
            }
            if run.state != RunState::Running {
                continue;
            }
            let index = STEPS.iter().position(|s| *s == run.step).unwrap_or(0);
            if self.random.roll() < 0.08 + troubled as f64 * 0.05 {
                said.push((
                    format!("{} failed at {} in {}", run.job, run.step, run.project),
                    Tone::Critical,
                    HappeningKind::Run,
                ));
                run.state = RunState::Failed;
                run.ended = Some(at);
            } else if index == STEPS.len() - 1 {
                said.push((
                    format!("{} succeeded in {}", run.job, run.project),
                    Tone::Good,
                    HappeningKind::Run,
                ));
                run.state = RunState::Succeeded;
                run.ended = Some(at);
            } else if self.random.roll() < 0.55 {
                run.step = STEPS[index + 1];
            }
        }
        self.runs = runs;
        let started = if self.random.roll() < 0.5 {
            1
        } else if self.random.roll() < 0.2 {
            2
        } else {
            0
        };
        for _ in 0..started {
            let run = self.new_run(at, RunState::Queued);
            self.counts.job[JOBS.iter().position(|j| *j == run.job).unwrap_or(0)] += 1;
            self.counts.project[PROJECTS.iter().position(|p| *p == run.project).unwrap_or(0)] += 1;
            self.counts.person[PEOPLE.iter().position(|p| *p == run.by).unwrap_or(0)] += 1;
            self.runs.insert(0, run);
        }
        if self.random.roll() < 0.03 {
            let person = self.random.pick(&PEOPLE);
            let service = self.random.pick(&SERVICES);
            let minor = 14 + (self.random.roll() * 4.0) as u32;
            let patch = (self.random.roll() * 9.0) as u32;
            said.push((
                format!("{person} deployed {service} 2.{minor}.{patch}"),
                Tone::Info,
                HappeningKind::Deploy,
            ));
        }
        if self.random.roll() < 0.02 {
            let (person, job, project) = (
                self.random.pick(&PEOPLE),
                self.random.pick(&JOBS),
                self.random.pick(&PROJECTS),
            );
            said.push((
                format!("{person} commented on {job} in {project}"),
                Tone::Quiet,
                HappeningKind::Comment,
            ));
        }
        // now and then a site slows down or goes down for a while, and comes back
        let mut sites = std::mem::take(&mut self.sites);
        for site in &mut sites {
            let base = base_latency(site.id);
            if site.state == Health::Operational && self.random.roll() < 0.006 {
                let down = self.random.roll() < 0.25;
                site.state = if down { Health::Outage } else { Health::Degraded };
                site.latency = base * 3.0;
                said.push((
                    format!("{} is {}", site.city, if down { "down" } else { "slow" }),
                    if down { Tone::Critical } else { Tone::Warning },
                    HappeningKind::Site,
                ));
            } else if site.state != Health::Operational && self.random.roll() < 0.06 {
                site.state = Health::Operational;
                site.latency = base;
                said.push((
                    format!("{} is operational again", site.city),
                    Tone::Good,
                    HappeningKind::Site,
                ));
            } else {
                let factor = if site.state == Health::Operational { 1.0 } else { 3.0 };
                site.latency = (base * factor * self.random.between(0.9, 1.15)).round();
            }
        }
        self.sites = sites;
        // today's uptime of each service falls a little while a site is in trouble
        if troubled > 0 {
            for days in &mut self.uptime {
                if let Some(today) = days.last_mut() {
                    let was = today.uptime.unwrap_or(1.0);
                    today.uptime = Some((was - 0.0004 * troubled as f64).max(0.9));
                }
            }
        }
        for (s, service) in SERVICES.iter().enumerate() {
            let count = self.random.between(0.0, 3.0).floor() as usize;
            for _ in 0..count {
                let line = self.log_line(service, at);
                self.logs[s].push(line);
            }
            let over = self.logs[s].len().saturating_sub(200);
            self.logs[s].drain(..over);
        }
        for (text, tone, kind) in said {
            let h = self.happening(at, text, tone, kind);
            self.happenings.insert(0, h);
        }
        self.runs.truncate(40);
        self.happenings.truncate(60);
        self.at = at;
    }

    /// A metric's latest reading and its readings so far.
    pub fn metric(&self, metric: Metric) -> (f64, Vec<f64>) {
        let values: Vec<f64> = self.readings.iter().map(|r| r.of(metric)).collect();
        (values.last().copied().unwrap_or(0.0), values)
    }
}

/// One fire of a job's schedule.
#[derive(Clone, Debug, PartialEq)]
pub struct Fire {
    pub id: String,
    pub job: &'static str,
    pub project: &'static str,
    pub at: f64,
    pub what: &'static str,
}

/// The next fires of the jobs' schedules after `at`, soonest first, as the web's `upcoming`:
/// each schedule's next fire first, then the ones after, so a frequent job does not fill the list.
pub fn upcoming(at: f64, count: usize) -> Vec<Fire> {
    const MIN: f64 = 60_000.0;
    let schedules: [(&str, &str, f64, f64, &str); 5] = [
        ("sync-inventory", "harbor", 15.0 * MIN, 2.0 * MIN, "every 15 minutes"),
        ("import-orders", "juniper", 60.0 * MIN, 10.0 * MIN, "hourly"),
        ("send-digest", "larkspur", 360.0 * MIN, 30.0 * MIN, "every 6 hours"),
        ("rebuild-index", "harbor", 720.0 * MIN, 180.0 * MIN, "twice a day"),
        (
            "nightly-export",
            "juniper",
            1440.0 * MIN,
            120.0 * MIN,
            "nightly at 02:00",
        ),
    ];
    let mut firsts = Vec::new();
    let mut later = Vec::new();
    for (job, project, every, offset, what) in schedules {
        let first = ((at - offset) / every).ceil() * every + offset;
        for step in 0..3 {
            let when = first + f64::from(step) * every;
            let fire = Fire {
                id: format!("{job}-{when}"),
                job,
                project,
                at: when,
                what,
            };
            if step == 0 {
                firsts.push(fire);
            } else {
                later.push(fire);
            }
        }
    }
    firsts.sort_by(|a, b| a.at.total_cmp(&b.at));
    later.sort_by(|a, b| a.at.total_cmp(&b.at));
    let mut fires: Vec<Fire> = firsts.into_iter().chain(later).take(count).collect();
    fires.sort_by(|a, b| a.at.total_cmp(&b.at));
    fires
}

/// Who is on call now and next: the rotation hands over every day at 09:00, through the people
/// in order, as the web's `onCall`.
pub fn on_call(at: f64) -> (&'static str, &'static str, f64, f64) {
    let day = 86_400_000.0;
    let handover = (at / day).floor() * day + 9.0 * 3_600_000.0;
    let since = if at >= handover { handover } else { handover - day };
    let index = ((since / day).floor() as usize) % PEOPLE.len();
    (PEOPLE[index], PEOPLE[(index + 1) % PEOPLE.len()], since, since + day)
}
