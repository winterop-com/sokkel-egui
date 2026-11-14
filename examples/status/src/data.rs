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
