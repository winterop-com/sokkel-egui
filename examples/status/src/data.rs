//! Tidevann's services as the status board knows them, from sokkel-ui's status example
//! (`examples/status/web/src/data.ts`).

/// The people who answer, and who are signed in.
pub const PEOPLE: [&str; 4] = ["ada", "grace", "linus", "margaret"];

/// One of Tidevann's services: what it is, where it answers and what it needs.
#[derive(Clone, Copy, Debug)]
pub struct Service {
    pub id: &'static str,
    pub description: &'static str,
    pub host: &'static str,
    pub group: &'static str,
    /// Whether it holds data rather than answers requests.
    pub store: bool,
    /// What it calls on to answer.
    pub depends_on: &'static [&'static str],
    /// Its 95th percentile latency on a good day, in milliseconds.
    pub latency: f64,
    /// Its share of failed requests on a good day, in percent.
    pub errors: f64,
    /// The requests it answers on a good day, per second.
    pub load: f64,
    /// The version running before any deploy from the board.
    pub version: &'static str,
}

/// The board's own order of its services, as the web's `SERVICE_IDS`: the order the overview's
/// tiles and the services table stand in until the person drags them.
pub const SERVICE_IDS: [&str; 8] = [
    "web",
    "api",
    "worker",
    "scheduler",
    "queue",
    "storage",
    "search",
    "mail",
];

/// The board's own order as indices into `SERVICES`.
pub fn board_order() -> Vec<usize> {
    SERVICE_IDS
        .iter()
        .map(|id| SERVICES.iter().position(|s| s.id == *id).expect("a service"))
        .collect()
}

pub const SERVICES: [Service; 8] = [
    Service {
        id: "web",
        description: "The app people sign in to",
        host: "tidevann.example.org",
        group: "Product",
        store: false,
        depends_on: &["api"],
        latency: 140.0,
        errors: 0.08,
        load: 820.0,
        version: "v4.12.2",
    },
    Service {
        id: "api",
        description: "The public API and the app behind it",
        host: "api.example.org",
        group: "Product",
        store: false,
        depends_on: &["queue", "storage", "search"],
        latency: 180.0,
        errors: 0.12,
        load: 1240.0,
        version: "v2.41.2",
    },
    Service {
        id: "worker",
        description: "Runs the jobs: nightly-export, sync-inventory, rebuild-index",
        host: "worker.example.org",
        group: "Processing",
        store: false,
        depends_on: &["queue", "storage"],
        latency: 420.0,
        errors: 0.2,
        load: 160.0,
        version: "v1.18.3",
    },
    Service {
        id: "scheduler",
        description: "Starts the jobs on their schedules",
        host: "scheduler.example.org",
        group: "Processing",
        store: false,
        depends_on: &["queue"],
        latency: 60.0,
        errors: 0.02,
        load: 40.0,
        version: "v0.9.7",
    },
    Service {
        id: "mail",
        description: "Sends send-digest and every notice",
        host: "mail.example.org",
        group: "Processing",
        store: false,
        depends_on: &["queue"],
        latency: 310.0,
        errors: 0.15,
        load: 55.0,
        version: "v1.4.2",
    },
    Service {
        id: "queue",
        description: "Carries work between the services",
        host: "queue.example.org",
        group: "Data",
        store: true,
        depends_on: &[],
        latency: 12.0,
        errors: 0.01,
        load: 2600.0,
        version: "v3.2.4",
    },
    Service {
        id: "storage",
        description: "Keeps the files, the exports and the backups",
        host: "storage.example.org",
        group: "Data",
        store: true,
        depends_on: &[],
        latency: 45.0,
        errors: 0.03,
        load: 1900.0,
        version: "v5.0.4",
    },
    Service {
        id: "search",
        description: "Answers searches from the index rebuild-index writes",
        host: "search.example.org",
        group: "Data",
        store: true,
        depends_on: &["storage"],
        latency: 95.0,
        errors: 0.05,
        load: 430.0,
        version: "v2.7.3",
    },
];

/// A place the services answer from, with where it is and its round trip on a good day.
#[derive(Clone, Copy, Debug)]
pub struct Site {
    pub id: &'static str,
    pub city: &'static str,
    pub country: &'static str,
    pub region: &'static str,
    /// Longitude and latitude.
    pub at: [f64; 2],
    /// Its round trip on a good day, in milliseconds.
    pub latency: f64,
}

pub const SITES: [Site; 11] = [
    Site {
        id: "oslo",
        city: "Oslo",
        country: "Norway",
        region: "Europe",
        at: [10.75, 59.91],
        latency: 18.0,
    },
    Site {
        id: "frankfurt",
        city: "Frankfurt",
        country: "Germany",
        region: "Europe",
        at: [8.68, 50.11],
        latency: 14.0,
    },
    Site {
        id: "dublin",
        city: "Dublin",
        country: "Ireland",
        region: "Europe",
        at: [-6.26, 53.35],
        latency: 21.0,
    },
    Site {
        id: "new-york",
        city: "New York",
        country: "United States",
        region: "North America",
        at: [-74.01, 40.71],
        latency: 24.0,
    },
    Site {
        id: "toronto",
        city: "Toronto",
        country: "Canada",
        region: "North America",
        at: [-79.38, 43.65],
        latency: 27.0,
    },
    Site {
        id: "sao-paulo",
        city: "São Paulo",
        country: "Brazil",
        region: "South America",
        at: [-46.63, -23.55],
        latency: 38.0,
    },
    Site {
        id: "johannesburg",
        city: "Johannesburg",
        country: "South Africa",
        region: "Africa",
        at: [28.05, -26.2],
        latency: 44.0,
    },
    Site {
        id: "mumbai",
        city: "Mumbai",
        country: "India",
        region: "Asia",
        at: [72.88, 19.08],
        latency: 35.0,
    },
    Site {
        id: "singapore",
        city: "Singapore",
        country: "Singapore",
        region: "Asia",
        at: [103.82, 1.35],
        latency: 29.0,
    },
    Site {
        id: "tokyo",
        city: "Tokyo",
        country: "Japan",
        region: "Asia",
        at: [139.69, 35.69],
        latency: 26.0,
    },
    Site {
        id: "sydney",
        city: "Sydney",
        country: "Australia",
        region: "Oceania",
        at: [151.21, -33.87],
        latency: 33.0,
    },
];

/// How bad an incident is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Minor,
    Major,
    Critical,
}

/// Where an incident stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Investigating,
    Identified,
    Monitoring,
    Resolved,
}

/// One of the incidents the demo starts with: resolved, days before the board's day.
pub struct Past {
    pub title: &'static str,
    pub severity: Severity,
    pub services: &'static [&'static str],
    pub sites: &'static [&'static str],
    pub days_ago: u32,
    /// How long it lasted.
    pub minutes: u32,
    pub assignee: &'static str,
    /// Its updates, oldest first, spread over how long it lasted.
    pub steps: &'static [(State, &'static str)],
}

pub const PAST: [Past; 8] = [
    Past {
        title: "Elevated error rates on the api",
        severity: Severity::Major,
        services: &["api", "web"],
        sites: &[],
        days_ago: 3,
        minutes: 42,
        assignee: "grace",
        steps: &[
            (
                State::Investigating,
                "The api is answering one request in twelve with an error. We are looking into it.",
            ),
            (
                State::Identified,
                "The errors follow the deploy of api v2.41.1. We are rolling it back.",
            ),
            (
                State::Monitoring,
                "The rollback is out and the errors have stopped. We are watching the api.",
            ),
            (
                State::Resolved,
                "The api has answered normally for half an hour. This incident is resolved.",
            ),
        ],
    },
    Past {
        title: "Slow searches in Asia",
        severity: Severity::Minor,
        services: &["search"],
        sites: &["singapore", "tokyo"],
        days_ago: 9,
        minutes: 95,
        assignee: "linus",
        steps: &[
            (
                State::Investigating,
                "Searches from Singapore and Tokyo take several seconds.",
            ),
            (
                State::Identified,
                "rebuild-index ran long and left the index half written in Asia.",
            ),
            (State::Resolved, "The index is whole again and searches are fast."),
        ],
    },
    Past {
        title: "Jobs queued behind a stuck worker",
        severity: Severity::Major,
        services: &["worker", "scheduler"],
        sites: &[],
        days_ago: 17,
        minutes: 130,
        assignee: "ada",
        steps: &[
            (
                State::Investigating,
                "nightly-export and sync-inventory have not started. We are looking into the worker.",
            ),
            (
                State::Identified,
                "One worker held the queue after a failed upload step.",
            ),
            (
                State::Monitoring,
                "The worker is restarted and the queued runs are going through.",
            ),
            (
                State::Resolved,
                "Every queued run has finished. This incident is resolved.",
            ),
        ],
    },
    Past {
        title: "Digests sent late",
        severity: Severity::Minor,
        services: &["mail"],
        sites: &[],
        days_ago: 26,
        minutes: 210,
        assignee: "margaret",
        steps: &[
            (State::Investigating, "send-digest is running two hours late."),
            (State::Resolved, "The digests are out and the next run is on time."),
        ],
    },
    Past {
        title: "Storage unreachable from São Paulo",
        severity: Severity::Critical,
        services: &["storage", "api", "worker"],
        sites: &["sao-paulo"],
        days_ago: 38,
        minutes: 64,
        assignee: "grace",
        steps: &[
            (State::Investigating, "Uploads and exports fail from São Paulo."),
            (
                State::Identified,
                "A network path into São Paulo is down. Traffic is moving to New York.",
            ),
            (
                State::Monitoring,
                "Traffic goes through New York and uploads work again.",
            ),
            (
                State::Resolved,
                "The path is back and São Paulo answers on its own. This incident is resolved.",
            ),
        ],
    },
    Past {
        title: "The app slow to sign in",
        severity: Severity::Minor,
        services: &["web"],
        sites: &[],
        days_ago: 51,
        minutes: 35,
        assignee: "linus",
        steps: &[
            (State::Investigating, "Signing in takes ten seconds or more."),
            (
                State::Resolved,
                "A slow session store is replaced. Signing in is quick again.",
            ),
        ],
    },
    Past {
        title: "Queue refusing new work",
        severity: Severity::Critical,
        services: &["queue", "worker", "scheduler", "mail"],
        sites: &[],
        days_ago: 66,
        minutes: 48,
        assignee: "ada",
        steps: &[
            (
                State::Investigating,
                "The queue refuses new work, so no job starts and no mail goes out.",
            ),
            (
                State::Identified,
                "The queue ran out of disk after a burst of import-orders runs.",
            ),
            (State::Monitoring, "The disk is grown and the queue takes work again."),
            (
                State::Resolved,
                "Every service is caught up. This incident is resolved.",
            ),
        ],
    },
    Past {
        title: "Search results out of date",
        severity: Severity::Minor,
        services: &["search"],
        sites: &[],
        days_ago: 80,
        minutes: 300,
        assignee: "margaret",
        steps: &[
            (State::Investigating, "Searches miss what was added today."),
            (State::Resolved, "rebuild-index has caught up and results are current."),
        ],
    },
];

/// One update on an incident, newest first in its incident.
#[derive(Clone, Debug)]
pub struct Update {
    /// When it was posted, in milliseconds of the board's clock.
    pub at: f64,
    pub state: State,
    pub message: String,
    pub by: &'static str,
}

/// Something that went or is going wrong, as the board keeps it.
#[derive(Clone, Debug)]
pub struct Incident {
    pub id: String,
    /// Its code, such as `INC-41`.
    pub code: String,
    pub title: String,
    pub severity: Severity,
    pub state: State,
    pub services: Vec<&'static str>,
    pub sites: Vec<&'static str>,
    pub assignee: &'static str,
    /// When it opened, in milliseconds of the board's clock.
    pub opened: f64,
    /// When it was resolved, or `None` while it is open.
    pub resolved: Option<f64>,
    pub updates: Vec<Update>,
}

const MINUTE: f64 = 60_000.0;
const DAY: f64 = 24.0 * 60.0 * MINUTE;

/// The incidents the demo starts with, at `now`: INC-41 open 48 minutes, and the past ones
/// resolved, days before, as the web's `seedIncidents`.
pub fn seed_incidents(now: f64) -> Vec<Incident> {
    let open = Incident {
        id: "incident-41".into(),
        code: "INC-41".into(),
        title: "Slow exports from storage".into(),
        severity: Severity::Minor,
        state: State::Monitoring,
        services: vec!["storage", "worker"],
        sites: vec!["frankfurt"],
        assignee: "grace",
        opened: now - 48.0 * MINUTE,
        resolved: None,
        updates: [
            (
                9.0,
                State::Monitoring,
                "Exports are back to their usual speed. We are watching nightly-export finish.",
            ),
            (
                31.0,
                State::Identified,
                "A full disk in Frankfurt slows writes to storage. We are moving exports to Oslo.",
            ),
            (
                48.0,
                State::Investigating,
                "nightly-export is writing to storage at a third of its usual speed.",
            ),
        ]
        .into_iter()
        .map(|(ago, state, message)| Update {
            at: now - ago * MINUTE,
            state,
            message: message.into(),
            by: "grace",
        })
        .collect(),
    };
    let past = PAST.iter().enumerate().map(|(index, one)| {
        let opened = now - one.days_ago as f64 * DAY - 3.0 * 60.0 * MINUTE;
        let resolved = opened + one.minutes as f64 * MINUTE;
        let span = (resolved - opened) / (one.steps.len().max(2) - 1) as f64;
        Incident {
            id: format!("incident-{}", 40 - index),
            code: format!("INC-{}", 40 - index),
            title: one.title.into(),
            severity: one.severity,
            state: State::Resolved,
            services: one.services.to_vec(),
            sites: one.sites.to_vec(),
            assignee: one.assignee,
            opened,
            resolved: Some(resolved),
            updates: one
                .steps
                .iter()
                .enumerate()
                .rev()
                .map(|(step, (state, message))| Update {
                    at: opened + span * step as f64,
                    state: *state,
                    message: (*message).into(),
                    by: one.assignee,
                })
                .collect(),
        }
    });
    std::iter::once(open).chain(past).collect()
}

/// What the person on call now was told at the handover.
pub const ON_CALL_NOTE: &str = "INC-41 is in monitoring; nightly-export should finish by morning.";

/// A week on call, Monday to Monday.
#[derive(Clone, Debug)]
pub struct Shift {
    pub id: String,
    pub starts: sokkel_ui_egui::Day,
    /// The Monday after it.
    pub ends: sokkel_ui_egui::Day,
    pub person: &'static str,
}

/// The rotation the demo starts with: eight weeks from two before the one `today` is in, the
/// people in turn, as the web's `seedShifts`.
pub fn seed_shifts(today: sokkel_ui_egui::Day) -> Vec<Shift> {
    let monday = today.plus(-(today.weekday() as i32));
    (0..8)
        .map(|index| {
            let starts = monday.plus((index - 2) * 7);
            Shift {
                id: format!("shift-{}", index + 1),
                starts,
                ends: starts.plus(7),
                person: PEOPLE[(index as usize + 3) % PEOPLE.len()],
            }
        })
        .collect()
}

/// The pager changing hands: who gave it to whom, when, and what they said.
#[derive(Clone, Debug)]
pub struct Handover {
    pub id: String,
    /// When, in milliseconds of the board's clock.
    pub at: f64,
    pub from: &'static str,
    pub to: &'static str,
    pub note: String,
}

/// The handovers the demo starts with: this Monday at nine, and the one before.
pub fn seed_handovers(monday_nine: f64) -> Vec<Handover> {
    vec![
        Handover {
            id: "handover-1".into(),
            at: monday_nine,
            from: "ada",
            to: "grace",
            note: "Quiet week. The search index move is planned for the weekend.".into(),
        },
        Handover {
            id: "handover-2".into(),
            at: monday_nine - 7.0 * DAY,
            from: "margaret",
            to: "ada",
            note: "Watch the worker after the upload fix.".into(),
        },
    ]
}
