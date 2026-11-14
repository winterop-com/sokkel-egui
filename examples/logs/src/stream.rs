//! Tidevann's services writing their logs, simulated: a seeded generator, so a run reads the same
//! every time, a few lines a reading, mostly info, now and then a warning or an error.

/// How bad a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn word(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

/// One line a service wrote.
#[derive(Clone, Debug)]
pub struct Line {
    pub id: u64,
    /// Seconds of the day.
    pub at: u32,
    pub service: &'static str,
    pub host: String,
    pub level: Level,
    pub message: String,
    /// The request or run the line is about, where it has one.
    pub trace: Option<String>,
}

impl Line {
    /// The time it was written, `HH:MM:SS`.
    pub fn clock(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.at / 3600, self.at / 60 % 60, self.at % 60)
    }
}

pub const SERVICES: [&str; 8] = [
    "web",
    "api",
    "worker",
    "scheduler",
    "queue",
    "storage",
    "search",
    "mail",
];
const JOBS: [&str; 5] = [
    "nightly-export",
    "sync-inventory",
    "rebuild-index",
    "send-digest",
    "import-orders",
];
const STEPS: [&str; 5] = ["fetch", "transform", "validate", "upload", "notify"];
const PEOPLE: [&str; 4] = ["ada", "grace", "linus", "margaret"];
const PATHS: [&str; 5] = [
    "/v1/runs",
    "/v1/jobs",
    "/v1/projects/harbor",
    "/v1/schedules",
    "/v1/runs/stream",
];

/// The stream: the lines so far, newest last, and the generator that writes the next.
pub struct Stream {
    pub lines: Vec<Line>,
    seed: u64,
    next_id: u64,
}

/// How many lines the viewer keeps.
pub const KEEP: usize = 2000;

impl Stream {
    /// A stream that has run for `readings` readings up to `at` (seconds of the day), two seconds apart.
    pub fn new(at: u32, readings: u32) -> Self {
        let mut stream = Self {
            lines: Vec::new(),
            seed: 20261116,
            next_id: 1,
        };
        for i in 0..readings {
            stream.reading(at - (readings - 1 - i) * 2);
        }
        stream
    }

    /// A number in 0..1, the same run after run (splitmix64).
    fn random(&mut self) -> f64 {
        self.seed = self.seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }

    fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[((self.random() * of.len() as f64) as usize).min(of.len() - 1)]
    }

    /// The lines one reading writes, at `at`: one to four, across the services.
    pub fn reading(&mut self, at: u32) {
        let count = 1 + (self.random() * 4.0) as usize;
        for _ in 0..count {
            let line = self.line(at);
            self.lines.push(line);
        }
        if self.lines.len() > KEEP {
            let over = self.lines.len() - KEEP;
            self.lines.drain(..over);
        }
    }

    fn line(&mut self, at: u32) -> Line {
        let service = self.pick(&SERVICES);
        let roll = self.random();
        let level = if roll < 0.04 {
            Level::Error
        } else if roll < 0.14 {
            Level::Warn
        } else {
            Level::Info
        };
        let job = self.pick(&JOBS);
        let step = self.pick(&STEPS);
        let person = self.pick(&PEOPLE);
        let path = self.pick(&PATHS);
        let ms = 8 + (self.random() * 240.0) as u32;
        let run = 4000 + (self.random() * 900.0) as u32;
        let (message, trace) = match (service, level) {
            ("api" | "web", Level::Info) => (format!("GET {path} 200 {ms}ms"), Some(format!("req-{run:x}"))),
            ("api" | "web", Level::Warn) => (
                format!("GET {path} 429 rate limited for {person}"),
                Some(format!("req-{run:x}")),
            ),
            ("api" | "web", Level::Error) => (
                format!("GET {path} 503 upstream storage timed out after 30s"),
                Some(format!("req-{run:x}")),
            ),
            ("worker", Level::Info) => (
                format!("{job} run {run}: {step} finished in {:.1}s", f64::from(ms) / 40.0),
                Some(format!("run-{run}")),
            ),
            ("worker", Level::Warn) => (
                format!("{job} run {run}: {step} retried, attempt 2 of 3"),
                Some(format!("run-{run}")),
            ),
            ("worker", Level::Error) => (
                format!("{job} run {run}: {step} failed: the source closed the connection"),
                Some(format!("run-{run}")),
            ),
            ("scheduler", Level::Info) => (
                format!(
                    "queued {job} for {:02}:{:02}",
                    (at / 3600 + 1) % 24,
                    (ms / 4) % 60 / 15 * 15
                ),
                None,
            ),
            ("scheduler", _) => (
                format!("{job} is running late: the last run has not finished"),
                Some(format!("run-{run}")),
            ),
            ("queue", Level::Info) => (format!("{} messages waiting, {} consumers", ms / 6, 4), None),
            ("queue", _) => (format!("{} messages waiting, oldest {}s", ms * 3, ms / 3), None),
            ("storage", Level::Info) => (format!("wrote {} MB for {job}", ms / 3 + 1), Some(format!("run-{run}"))),
            ("storage", Level::Warn) => ("disk in Frankfurt is 88% full".into(), None),
            ("storage", Level::Error) => (
                "write to Frankfurt failed: no space left on device".into(),
                Some(format!("run-{run}")),
            ),
            ("search", Level::Info) => (format!("indexed {} documents in {ms}ms", ms * 12), None),
            ("search", _) => (
                format!("rebuild-index is {}% behind", ms / 10 + 5),
                Some(format!("run-{run}")),
            ),
            ("mail", Level::Info) => (
                format!("send-digest sent {} emails", ms * 4),
                Some(format!("run-{run}")),
            ),
            ("mail", _) => (
                format!("send-digest bounced for {person}@example.org"),
                Some(format!("run-{run}")),
            ),
            _ => ("ok".into(), None),
        };
        let host = format!("{service}-{}.example.org", 1 + (self.random() * 3.0) as u32);
        let id = self.next_id;
        self.next_id += 1;
        Line {
            id,
            at,
            service,
            host,
            level,
            message,
            trace,
        }
    }
}
