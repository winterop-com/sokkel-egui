//! The simulated metrics source, as the web's `source.ts`: Tidevann's services, hosts, projects and
//! jobs, and every metric as a pure function of time, so any range can be read at once. Every
//! number is the web's own: the hash keeps JavaScript's 32-bit integer rules.

/// The services, projects and jobs.
pub const SERVICES: [&str; 3] = ["api", "worker", "scheduler"];
pub const PROJECTS: [&str; 3] = ["harbor", "juniper", "larkspur"];
pub const JOBS: [&str; 5] = [
    "nightly-export",
    "sync-inventory",
    "rebuild-index",
    "import-orders",
    "send-digest",
];

/// The project a job belongs to.
pub fn job_project(job: &str) -> &'static str {
    match job {
        "nightly-export" | "sync-inventory" => "harbor",
        "rebuild-index" | "import-orders" => "juniper",
        _ => "larkspur",
    }
}

/// One host: its id, the service it runs and where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Host {
    pub id: &'static str,
    pub service: &'static str,
    pub region: &'static str,
}

pub const HOSTS: [Host; 7] = [
    Host {
        id: "api-1",
        service: "api",
        region: "Northern Europe",
    },
    Host {
        id: "api-2",
        service: "api",
        region: "Western Europe",
    },
    Host {
        id: "api-3",
        service: "api",
        region: "North America",
    },
    Host {
        id: "worker-1",
        service: "worker",
        region: "Northern Europe",
    },
    Host {
        id: "worker-2",
        service: "worker",
        region: "Northern Europe",
    },
    Host {
        id: "worker-3",
        service: "worker",
        region: "North America",
    },
    Host {
        id: "scheduler-1",
        service: "scheduler",
        region: "Northern Europe",
    },
];

/// A host's full name.
pub fn host_name(id: &str) -> String {
    format!("{id}.example.org")
}

/// The labels a series can carry, in the web's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Label {
    Service,
    Host,
    Project,
    Job,
}

pub const LABELS: [Label; 4] = [Label::Service, Label::Host, Label::Project, Label::Job];

impl Label {
    pub fn key(self) -> &'static str {
        match self {
            Label::Service => "service",
            Label::Host => "host",
            Label::Project => "project",
            Label::Job => "job",
        }
    }
}

/// One series of a metric: its labels, `None` for a label it does not carry.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SeriesLabels {
    pub service: Option<&'static str>,
    pub host: Option<&'static str>,
    pub project: Option<&'static str>,
    pub job: Option<&'static str>,
}

impl SeriesLabels {
    pub fn get(&self, label: Label) -> Option<&'static str> {
        match label {
            Label::Service => self.service,
            Label::Host => self.host,
            Label::Project => self.project,
            Label::Job => self.job,
        }
    }

    /// The labels' values in the web's object order, which its seed sums over.
    fn values(&self) -> impl Iterator<Item = &'static str> {
        [self.service, self.host, self.project, self.job].into_iter().flatten()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetricId {
    Requests,
    Errors,
    LatencyP50,
    LatencyP95,
    LatencyP99,
    Cpu,
    Memory,
    QueueDepth,
    JobDuration,
}

/// How the series of a group combine into one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Aggregation {
    Sum,
    Avg,
    Max,
    Min,
}

/// Whether a metric's rise is good news or bad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rising {
    Good,
    Bad,
}

/// A metric: what it is called and says, its unit and decimals, how its series combine, and its
/// series.
pub struct Metric {
    pub id: MetricId,
    pub label: &'static str,
    pub description: &'static str,
    pub unit: &'static str,
    pub decimals: usize,
    /// The ways its series combine; the first is the natural one.
    pub aggregations: &'static [Aggregation],
    pub rising: Option<Rising>,
}

/// A number from 0 to 1, the same for the same inputs, as the web's `hash` on 32-bit integers.
pub fn hash(parts: &[f64]) -> f64 {
    let mut h: i32 = 2_166_136_261_u32 as i32;
    for part in parts {
        // `Math.floor(part) | 0`: ToInt32 of the floor, which wraps modulo 2^32 however large
        h ^= to_int32(part.floor());
        h = h.wrapping_mul(16_777_619);
        h ^= ((h as u32) >> 13) as i32;
        h = h.wrapping_mul(0x5bd1_e995_u32 as i32);
        h ^= ((h as u32) >> 15) as i32;
    }
    f64::from(h as u32) / 4_294_967_295.0
}

/// JavaScript's ToInt32: a whole number wrapped modulo 2^32 into the signed range, exact for the
/// large seeds a long label folds into.
fn to_int32(x: f64) -> i32 {
    if !x.is_finite() {
        return 0;
    }
    (x.rem_euclid(4_294_967_296.0) as u32) as i32
}

/// Smooth noise from -1 to 1 along `x`, eased between a fixed value at each whole step.
fn noise(seed: f64, x: f64) -> f64 {
    let at = x.floor();
    let fraction = x - at;
    let ease = (1.0 - (fraction * std::f64::consts::PI).cos()) / 2.0;
    let from = hash(&[seed, at]) * 2.0 - 1.0;
    let to = hash(&[seed, at + 1.0]) * 2.0 - 1.0;
    from + (to - from) * ease
}

/// The day's swell, from -1 at night to 1 in the afternoon.
fn daily(second: f64) -> f64 {
    (((second % 86_400.0) / 86_400.0 - 0.3) * std::f64::consts::PI * 2.0).sin()
}

/// Every ten minutes an incident may come, a minute or two long, rising and falling over ten seconds.
const INCIDENT_EVERY: f64 = 600.0;

/// How strong the incident is at a second, from 0 to 1, for a seed that picks its timetable.
pub fn incident(second: f64, seed: f64) -> f64 {
    let window = (second / INCIDENT_EVERY).floor();
    let mut strongest: f64 = 0.0;
    for one in [window - 1.0, window] {
        if hash(&[seed, one, 7.0]) > 0.6 {
            continue;
        }
        let start = one * INCIDENT_EVERY + hash(&[seed, one, 11.0]) * 420.0;
        let length = 60.0 + hash(&[seed, one, 13.0]) * 90.0;
        let into = second - start;
        if !(0.0..=length).contains(&into) {
            continue;
        }
        strongest = strongest.max((into / 10.0).min((length - into) / 10.0).min(1.0));
    }
    strongest
}

/// A series' seed: each label's text folded as the web's `seedOf`, summed.
fn seed_of(labels: &SeriesLabels) -> f64 {
    labels
        .values()
        .map(|text| text.chars().fold(7.0, |one, c| one * 31.0 + f64::from(c as u32)))
        .sum()
}

/// A level that swells with the day and wanders, with a quick flicker on top.
fn level(base: f64, labels: &SeriesLabels, second: f64, swell: f64, wander: f64, flicker: f64) -> f64 {
    let seed = seed_of(labels);
    base * (1.0
        + swell * daily(second)
        + wander * noise(seed, second / 300.0)
        + flicker * noise(seed + 1.0, second / 4.0))
}

fn host_of(id: Option<&str>) -> Option<&'static Host> {
    HOSTS.iter().find(|h| Some(h.id) == id)
}

fn project_share(project: Option<&str>) -> f64 {
    match project {
        Some("harbor") => 0.5,
        Some("juniper") => 0.3,
        Some("larkspur") => 0.2,
        _ => 0.3,
    }
}

fn job_seconds(job: Option<&str>) -> f64 {
    match job {
        Some("nightly-export") => 42.0,
        Some("sync-inventory") => 18.0,
        Some("rebuild-index") => 95.0,
        Some("import-orders") => 27.0,
        Some("send-digest") => 6.0,
        _ => 20.0,
    }
}

fn service_of(labels: &SeriesLabels) -> Option<&'static str> {
    host_of(labels.host).map(|h| h.service)
}

fn latency(spread: f64, labels: &SeriesLabels, second: f64) -> f64 {
    let service = service_of(labels);
    let base = match service {
        Some("api") => 45.0,
        Some("worker") => 20.0,
        _ => 10.0,
    };
    let lift = if service == Some("api") {
        1.0 + 1.7 * incident(second, 1.0)
    } else {
        1.0
    };
    level(base * spread, labels, second, 0.15, 0.1, 0.06) * lift
}

pub const METRICS: [Metric; 9] = [
    Metric {
        id: MetricId::Requests,
        label: "Requests",
        description: "Requests each service answers per second.",
        unit: " req/s",
        decimals: 1,
        aggregations: &[Aggregation::Sum, Aggregation::Avg, Aggregation::Max, Aggregation::Min],
        rising: None,
    },
    Metric {
        id: MetricId::Errors,
        label: "Error rate",
        description: "Requests that ended in an error, of every request answered.",
        unit: "%",
        decimals: 2,
        aggregations: &[Aggregation::Avg, Aggregation::Max, Aggregation::Min],
        rising: Some(Rising::Bad),
    },
    Metric {
        id: MetricId::LatencyP50,
        label: "Latency p50",
        description: "The median time to answer a request.",
        unit: " ms",
        decimals: 0,
        aggregations: &[Aggregation::Avg, Aggregation::Max, Aggregation::Min],
        rising: Some(Rising::Bad),
    },
    Metric {
        id: MetricId::LatencyP95,
        label: "Latency p95",
        description: "The time 95 requests in 100 are answered within.",
        unit: " ms",
        decimals: 0,
        aggregations: &[Aggregation::Avg, Aggregation::Max, Aggregation::Min],
        rising: Some(Rising::Bad),
    },
    Metric {
        id: MetricId::LatencyP99,
        label: "Latency p99",
        description: "The time 99 requests in 100 are answered within.",
        unit: " ms",
        decimals: 0,
        aggregations: &[Aggregation::Avg, Aggregation::Max, Aggregation::Min],
        rising: Some(Rising::Bad),
    },
    Metric {
        id: MetricId::Cpu,
        label: "CPU",
        description: "The share of each host\u{2019}s processors in use.",
        unit: "%",
        decimals: 0,
        aggregations: &[Aggregation::Avg, Aggregation::Max, Aggregation::Min],
        rising: Some(Rising::Bad),
    },
    Metric {
        id: MetricId::Memory,
        label: "Memory",
        description: "Memory each host\u{2019}s processes hold.",
        unit: " GB",
        decimals: 2,
        aggregations: &[Aggregation::Sum, Aggregation::Avg, Aggregation::Max, Aggregation::Min],
        rising: Some(Rising::Bad),
    },
    Metric {
        id: MetricId::QueueDepth,
        label: "Queue depth",
        description: "Runs waiting for a worker slot.",
        unit: "",
        decimals: 0,
        aggregations: &[Aggregation::Sum, Aggregation::Avg, Aggregation::Max, Aggregation::Min],
        rising: Some(Rising::Bad),
    },
    Metric {
        id: MetricId::JobDuration,
        label: "Job duration",
        description: "How long each job\u{2019}s runs take, from start to finish.",
        unit: " s",
        decimals: 1,
        aggregations: &[Aggregation::Avg, Aggregation::Max, Aggregation::Min],
        rising: Some(Rising::Bad),
    },
];

/// A metric by its id.
pub fn metric_of(id: MetricId) -> &'static Metric {
    METRICS.iter().find(|m| m.id == id).unwrap_or(&METRICS[0])
}

/// The series a metric has, in the web's order.
pub fn series_of(id: MetricId) -> Vec<SeriesLabels> {
    let service_host_project = || {
        HOSTS
            .iter()
            .flat_map(|h| {
                PROJECTS.iter().map(move |p| SeriesLabels {
                    service: Some(h.service),
                    host: Some(h.id),
                    project: Some(p),
                    job: None,
                })
            })
            .collect::<Vec<_>>()
    };
    let service_host = || {
        HOSTS
            .iter()
            .map(|h| SeriesLabels {
                service: Some(h.service),
                host: Some(h.id),
                ..Default::default()
            })
            .collect::<Vec<_>>()
    };
    let workers = HOSTS.iter().filter(|h| h.service == "worker");
    match id {
        MetricId::Requests | MetricId::Errors => service_host_project(),
        MetricId::QueueDepth => workers
            .flat_map(|h| {
                PROJECTS.iter().map(move |p| SeriesLabels {
                    service: Some("worker"),
                    host: Some(h.id),
                    project: Some(p),
                    job: None,
                })
            })
            .collect(),
        MetricId::JobDuration => workers
            .flat_map(|h| {
                JOBS.iter().map(move |j| SeriesLabels {
                    service: Some("worker"),
                    host: Some(h.id),
                    project: Some(job_project(j)),
                    job: Some(j),
                })
            })
            .collect(),
        _ => service_host(),
    }
}

/// A metric's value for one series at a second, as the web's metric `value`.
pub fn value(id: MetricId, labels: &SeriesLabels, second: f64) -> f64 {
    let service = service_of(labels);
    match id {
        MetricId::Requests => {
            let share = project_share(labels.project);
            let rate = match (service, labels.host) {
                (Some("api"), Some("api-1")) => 120.0,
                (Some("api"), Some("api-2")) => 100.0,
                (Some("api"), Some("api-3")) => 80.0,
                (Some("api"), _) => 90.0,
                (Some("worker"), _) => 15.0,
                _ => 4.0,
            };
            level(rate * share, labels, second, 0.25, 0.12, 0.04).max(0.0)
        }
        MetricId::Errors => {
            let api = service == Some("api");
            let base = if api { 0.4 } else { 0.15 };
            let lift = if api {
                5.5 * incident(second, 1.0)
            } else {
                0.3 * incident(second, 2.0)
            };
            (level(base, labels, second, 0.1, 0.4, 0.3) + lift).max(0.0)
        }
        MetricId::LatencyP50 => latency(1.0, labels, second),
        MetricId::LatencyP95 => latency(3.1, labels, second),
        MetricId::LatencyP99 => latency(5.6, labels, second),
        MetricId::Cpu => {
            let (base, lift) = match service {
                Some("api") => (38.0, 24.0 * incident(second, 1.0)),
                Some("worker") => (62.0, 22.0 * incident(second, 2.0)),
                _ => (16.0, 0.0),
            };
            (level(base, labels, second, 0.2, 0.18, 0.08) + lift).clamp(1.0, 100.0)
        }
        MetricId::Memory => {
            let base = match service {
                Some("api") => 1.4,
                Some("worker") => 2.6,
                _ => 0.6,
            };
            // climbs until the collector frees it, every quarter of an hour
            let seed = seed_of(labels);
            let climb = ((second + seed * 37.0) % 900.0) / 900.0;
            base * (1.0 + 0.28 * climb + 0.04 * noise(seed, second / 60.0))
        }
        MetricId::QueueDepth => {
            let seed = seed_of(labels);
            let batch = ((((second + seed * 13.0) % 900.0) / 900.0 * std::f64::consts::PI * 2.0).sin())
                .max(0.0)
                .powi(6);
            let share = project_share(labels.project);
            (share * (14.0 + 10.0 * noise(seed, second / 90.0) + 40.0 * batch + 30.0 * incident(second, 2.0)))
                .max(0.0)
                .round()
        }
        MetricId::JobDuration => {
            level(job_seconds(labels.job), labels, second, 0.12, 0.2, 0.05) * (1.0 + 0.6 * incident(second, 2.0))
        }
    }
}

/// The labels a metric's series carry.
pub fn labels_of(id: MetricId) -> Vec<Label> {
    let series = series_of(id);
    LABELS
        .into_iter()
        .filter(|label| series.iter().any(|s| s.get(*label).is_some()))
        .collect()
}

/// The value meaning every value of a variable.
pub const ALL: &str = "all";

/// The variables a dashboard filters by: a value, or `all`.
#[derive(Clone, Debug, PartialEq)]
pub struct Variables {
    pub service: String,
    pub host: String,
    pub project: String,
}

impl Default for Variables {
    fn default() -> Self {
        Self {
            service: ALL.into(),
            host: ALL.into(),
            project: ALL.into(),
        }
    }
}

/// A filter on labels: a value wanted for each label it names.
pub type Filter = Vec<(Label, String)>;

/// Whether a series is kept by a filter: each label the series carries must match a filter that
/// names a value.
pub fn matches(series: &SeriesLabels, filter: &[(Label, String)]) -> bool {
    filter.iter().all(|(label, wanted)| {
        if wanted == ALL || wanted.is_empty() {
            return true;
        }
        series.get(*label).is_none_or(|has| has == wanted)
    })
}

/// Combines the values of a group.
pub fn combine(values: &[f64], aggregation: Aggregation) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(match aggregation {
        Aggregation::Sum => values.iter().sum(),
        Aggregation::Max => values.iter().copied().fold(f64::MIN, f64::max),
        Aggregation::Min => values.iter().copied().fold(f64::MAX, f64::min),
        Aggregation::Avg => values.iter().sum::<f64>() / values.len() as f64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values the web's `source.ts` gives, run by bun, so the egui board draws the same numbers.
    #[test]
    fn the_values_are_the_web_s() {
        let api1 = SeriesLabels {
            service: Some("api"),
            host: Some("api-1"),
            project: Some("harbor"),
            job: None,
        };
        let worker = SeriesLabels {
            service: Some("worker"),
            host: Some("worker-1"),
            project: Some("harbor"),
            job: Some("nightly-export"),
        };
        let queue = SeriesLabels {
            service: Some("worker"),
            host: Some("worker-2"),
            project: Some("juniper"),
            job: None,
        };
        let second = 1_795_338_000.0;
        let web = [
            (MetricId::Requests, api1, 62.62183372041973),
            (MetricId::Errors, api1, 0.32460421638318493),
            (MetricId::LatencyP50, api1, 45.43813325880026),
            (MetricId::LatencyP95, api1, 140.8582131022808),
            (MetricId::LatencyP99, api1, 254.45354624928146),
            (MetricId::Cpu, api1, 37.46613963318845),
            (MetricId::Memory, api1, 1.741656652303767),
            (MetricId::QueueDepth, queue, 7.0),
            (MetricId::JobDuration, worker, 36.85890665483058),
        ];
        for (id, labels, expected) in web {
            let got = value(id, &labels, second);
            assert!((got - expected).abs() < 1e-9, "{id:?}: {got}, the web {expected}");
        }
        // inside an incident, which lifts the api
        let during = 1_795_338_966.0;
        assert_eq!(incident(during, 1.0), 1.0);
        let api2 = SeriesLabels {
            service: Some("api"),
            host: Some("api-2"),
            ..Default::default()
        };
        assert!((value(MetricId::Errors, &api1, during) - 5.9201802152025715).abs() < 1e-9);
        assert!((value(MetricId::LatencyP95, &api2, during) - 371.7312912464971).abs() < 1e-9);
    }
}
