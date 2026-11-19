//! The simulated FHIR server, as the web's `data/server.ts` keeps it on the shared fake backend
//! (`tidevann-shared/backend`): the resources it holds and every version of each, and the searches
//! the person kept. It reads, creates, updates and deletes as a FHIR server's REST interface does:
//! checking a resource before it keeps it, refusing an update made against an older version, giving
//! each change a new version, and refusing to delete what other resources point at.
//!
//! Each collection is the web's `collection`: a call does its work at once and answers `LATENCY`
//! milliseconds later, and every change is told to those listening that much later with the rows as
//! they were when it was made. The server's calls await several of those in turn, so each runs here
//! as stages on the clock, each stage what the web does between two awaits; its answer is handed to
//! the page that asked (`Backend::take_settled`). A page reads a collection's rows as they were last
//! told or listed (`Collection::seen`). The clock and the collections are carried from the dhis2
//! example's backend (`examples/dhis2/src/backend.rs`).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::js::{iso_instant, stringify};
use crate::model::{Issue, Refused, Resource, Severity, key_of, type_of, version_of};
use crate::seed::seed;
use crate::validate::validate;

pub const SERVER_NAME: &str = "harbor";
pub const SERVER_BASE: &str = "https://fhir.example.org/r4";
pub const SERVER_ADDRESS: &str = "fhir.example.org/r4";
pub const SERVER_VERSION: &str = "FHIR 4.0.1";

/// The person using the workbench, who every change is saved as.
pub const ME: &str = "ada";

/// How long each call to the backend takes, in milliseconds.
pub const LATENCY: i64 = 150;

/// A resource as the server holds it, under its key `Type/id`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stored {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub resource: Resource,
}

/// What a version did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Change {
    Create,
    Update,
    Delete,
}

impl Change {
    pub fn key(self) -> &'static str {
        match self {
            Change::Create => "create",
            Change::Update => "update",
            Change::Delete => "delete",
        }
    }
}

/// One version of a resource, under `Type/id/_history/n`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub key: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "versionId")]
    pub version_id: i64,
    pub at: String,
    pub by: String,
    pub change: Change,
    /// The resource as that version left it; none for a delete.
    pub resource: Option<Resource>,
}

/// A search the person kept, run again from Saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSearch {
    pub id: String,
    pub label: String,
    pub query: String,
}

/// The searches a person kept, in the order they put them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSearches {
    pub id: String,
    pub searches: Vec<SavedSearch>,
}

/// A row of a collection, kept under its id.
pub trait Row: Clone {
    fn id(&self) -> &str;
}

impl Row for Stored {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Row for Version {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Row for SavedSearches {
    fn id(&self) -> &str {
        &self.id
    }
}

/// The resources the server starts with.
pub fn resources_seed() -> Vec<Stored> {
    seed()
        .into_iter()
        .map(|one| Stored {
            id: key_of(&one.resource),
            kind: type_of(&one.resource),
            resource: one.resource,
        })
        .collect()
}

/// Every version the server starts with: each resource's earlier ones, then itself.
pub fn history_seed() -> Vec<Version> {
    seed()
        .into_iter()
        .flat_map(|one| {
            let key = key_of(&one.resource);
            let kind = type_of(&one.resource);
            let last = crate::model::last_updated(&one.resource).unwrap_or_default();
            let mut versions = one.earlier;
            versions.push((last, one.resource));
            versions
                .into_iter()
                .enumerate()
                .map(move |(index, (at, resource))| Version {
                    id: format!("{key}/_history/{}", index + 1),
                    key: key.clone(),
                    kind: kind.clone(),
                    version_id: index as i64 + 1,
                    at,
                    by: if index == 0 { "grace" } else { "linus" }.to_owned(),
                    change: if index == 0 { Change::Create } else { Change::Update },
                    resource: Some(resource),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The searches the person starts with.
pub fn saved_seed() -> Vec<SavedSearches> {
    let one = |id: &str, label: &str, query: &str| SavedSearch {
        id: id.to_owned(),
        label: label.to_owned(),
        query: query.to_owned(),
    };
    vec![SavedSearches {
        id: ME.to_owned(),
        searches: vec![
            one(
                "saved-1",
                "High systolic pressure",
                "Observation?code=http://loinc.org|8480-6&value-quantity=ge150&_sort=-date",
            ),
            one(
                "saved-2",
                "Active hypertension",
                "Condition?code=38341003&clinical-status=active",
            ),
            one("saved-3", "Draft libraries", "Library?status=draft"),
        ],
    }]
}

/// Every resource that points at `key`, by its key.
pub fn pointing_at(all: &[Stored], key: &str) -> Vec<String> {
    let needle = format!("\"reference\":\"{key}\"");
    all.iter()
        .filter(|row| row.id != key && stringify(&row.resource).contains(&needle))
        .map(|row| row.id.clone())
        .collect()
}

/// The text an editor holds, read as JSON, or a refusal saying where it does not read.
pub fn parse(text: &str) -> Result<Value, Refused> {
    crate::js::parse_json(text).map_err(|why| {
        Refused::new(
            400,
            vec![Issue::error("structure", None, format!("The text is not JSON: {why}"))],
        )
    })
}

/// A failure of a call: the server's refusal, or the backend's own error.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Failure {
    Refused(Refused),
    Error(String),
}

impl Failure {
    /// What the failure says, as an error's message.
    pub fn message(&self) -> String {
        match self {
            Failure::Refused(refused) => refused.message(),
            Failure::Error(message) => message.clone(),
        }
    }
}

/// What a call answered.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// A resource kept (or the version it had, for an update that changed nothing), and the
    /// warnings it carried.
    Kept {
        resource: Resource,
        warnings: Vec<Issue>,
    },
    /// A delete or the saved searches kept.
    Done,
    Failed(Failure),
}

/// A call's answer, under the ticket it was made with.
#[derive(Clone, Debug, PartialEq)]
pub struct Settled {
    pub ticket: u64,
    pub outcome: Outcome,
}

/// Which collection a timer tells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Name {
    Resources,
    History,
    Saved,
}

/// A call to the server on its way, at the stage it waits to run next.
#[derive(Clone, Debug)]
enum Call {
    Update {
        kind: String,
        id: String,
        resource: Value,
        read_at: f64,
        all: Vec<Stored>,
        stage: u8,
        kept: Option<(Resource, Vec<Issue>)>,
    },
    Create {
        kind: String,
        resource: Value,
        all: Vec<Stored>,
        stage: u8,
        id: String,
        with_id: Value,
        issues: Vec<Issue>,
        past: Vec<Version>,
        kept: Option<Resource>,
    },
    Remove {
        key: String,
        all: Vec<Stored>,
        stage: u8,
        kind: String,
        version_id: i64,
    },
    KeepSearches {
        searches: Vec<SavedSearch>,
        rows: Vec<SavedSearches>,
        stage: u8,
    },
}

#[derive(Clone, Debug)]
enum Job {
    /// A collection's change told, or its list answered, with the rows kept under the timer's seq.
    Heard(Name),
    /// A call's next stage.
    Stage(u64, Box<Call>),
}

#[derive(Clone, Debug)]
struct Timer {
    at: i64,
    seq: u64,
    job: Job,
}

/// The web's clock and its timers.
#[derive(Clone, Debug, Default)]
pub struct Clock {
    pub now: i64,
    seq: u64,
    timers: Vec<Timer>,
}

impl Clock {
    fn after(&mut self, ms: i64, job: Job) -> u64 {
        self.seq += 1;
        self.timers.push(Timer {
            at: self.now + ms,
            seq: self.seq,
            job,
        });
        self.seq
    }
}

/// One collection: its rows as the backend keeps them, and as the pages last heard them.
#[derive(Clone, Debug)]
pub struct Collection<T: Row> {
    name: Name,
    /// The rows as kept, every change made at once.
    pub rows: Vec<T>,
    /// The rows as last told or listed, none until the first answer.
    pub seen: Option<Vec<T>>,
    /// Whether a change was kept since the seed, as the browser keeps the rows once one is.
    pub written: bool,
    pending: Vec<(u64, Vec<T>)>,
}

impl<T: Row> Collection<T> {
    fn new(name: Name, rows: Vec<T>) -> Self {
        Collection {
            name,
            rows,
            seen: None,
            written: false,
            pending: Vec::new(),
        }
    }

    /// The rows a page shows: none until the first answer.
    pub fn seen(&self) -> &[T] {
        self.seen.as_deref().unwrap_or_default()
    }

    /// Whether a page has heard the rows yet.
    pub fn loaded(&self) -> bool {
        self.seen.is_some()
    }

    fn heard(&mut self, seq: u64) {
        if let Some(at) = self.pending.iter().position(|(one, _)| *one == seq) {
            self.seen = Some(self.pending.remove(at).1);
        }
    }

    fn list(&mut self, clock: &mut Clock) {
        let seq = clock.after(LATENCY, Job::Heard(self.name));
        self.pending.push((seq, self.rows.clone()));
    }

    fn save(&mut self, clock: &mut Clock, next: Vec<T>) {
        self.rows = next;
        self.written = true;
        let seq = clock.after(LATENCY, Job::Heard(self.name));
        self.pending.push((seq, self.rows.clone()));
    }

    fn prepend(&mut self, clock: &mut Clock, row: T) {
        let mut next = vec![row];
        next.extend(self.rows.iter().cloned());
        self.save(clock, next);
    }

    fn replace(&mut self, clock: &mut Clock, id: &str, change: impl FnOnce(&mut T)) -> Result<(), String> {
        let Some(mut row) = self.rows.iter().find(|one| one.id() == id).cloned() else {
            return Err(format!("No {} with the id {id}.", self.label()));
        };
        change(&mut row);
        let next = self
            .rows
            .iter()
            .map(|one| if one.id() == id { row.clone() } else { one.clone() })
            .collect();
        self.save(clock, next);
        Ok(())
    }

    fn remove(&mut self, clock: &mut Clock, id: &str) {
        let next = self.rows.iter().filter(|one| one.id() != id).cloned().collect();
        self.save(clock, next);
    }

    fn label(&self) -> &'static str {
        match self.name {
            Name::Resources => "resources",
            Name::History => "history",
            Name::Saved => "saved-searches",
        }
    }
}

/// The server on the web's clock.
#[derive(Clone, Debug)]
pub struct Backend {
    pub clock: Clock,
    pub resources: Collection<Stored>,
    pub history: Collection<Version>,
    pub saved: Collection<SavedSearches>,
    tickets: u64,
    settled: Vec<Settled>,
}

/// The id prefix the server makes ids of a type with.
fn prefix_of(kind: &str) -> String {
    match kind {
        "Patient" => "pat",
        "Practitioner" => "prac",
        "Organization" => "org",
        "Encounter" => "enc",
        "Observation" => "obs",
        "Condition" => "cond",
        "Library" => "lib",
        _ => return kind.to_lowercase(),
    }
    .to_owned()
}

/// Makes an id for a new resource of a type that no resource has.
fn fresh_id(all: &[Stored], kind: &str) -> String {
    let prefix = prefix_of(kind);
    let mut n = all.iter().filter(|row| row.kind == kind).count() + 1;
    while all.iter().any(|row| row.id == format!("{kind}/{prefix}-{n:03}")) {
        n += 1;
    }
    format!("{prefix}-{n:03}")
}

/// A resource without its `meta`, as `JSON.stringify({ ...resource, meta: undefined })` writes it.
fn without_meta(resource: &Value) -> String {
    let mut copy = resource.clone();
    if let Some(fields) = copy.as_object_mut() {
        fields.shift_remove("meta");
    }
    stringify(&copy)
}

/// `{ ...resource, meta: { ...resource.meta, versionId, lastUpdated } }`.
fn with_meta(resource: &Value, version_id: i64, at: &str) -> Value {
    let mut copy = resource.clone();
    if let Some(fields) = copy.as_object_mut() {
        let mut meta = match fields.get("meta") {
            Some(Value::Object(old)) => old.clone(),
            _ => Map::new(),
        };
        meta.insert("versionId".to_owned(), json!(version_id.to_string()));
        meta.insert("lastUpdated".to_owned(), json!(at));
        fields.insert("meta".to_owned(), Value::Object(meta));
    }
    copy
}

fn refuse(issues: &[Issue]) -> Option<Failure> {
    issues
        .iter()
        .any(|issue| issue.severity == Severity::Error)
        .then(|| Failure::Refused(Refused::new(422, issues.to_vec())))
}

impl Backend {
    /// The server as the app opens at `now`: every collection from its seed.
    pub fn new(now: i64) -> Self {
        Backend {
            clock: Clock {
                now,
                ..Clock::default()
            },
            resources: Collection::new(Name::Resources, resources_seed()),
            history: Collection::new(Name::History, history_seed()),
            saved: Collection::new(Name::Saved, saved_seed()),
            tickets: 0,
            settled: Vec::new(),
        }
    }

    /// The rows a visit before kept, in place of the seed, as the browser keeps a collection once a
    /// change is kept; every collection listed again.
    pub fn restore(&mut self, resources: Vec<Stored>, history: Vec<Version>, saved: Vec<SavedSearches>) {
        self.resources.rows = resources;
        self.resources.written = true;
        self.history.rows = history;
        self.history.written = true;
        self.saved.rows = saved;
        self.saved.written = true;
        self.resources.pending.clear();
        self.history.pending.clear();
        self.saved.pending.clear();
        self.list_all();
    }

    /// Every collection listed, as the pages that read them do when they are drawn.
    pub fn list_all(&mut self) {
        self.resources.list(&mut self.clock);
        self.history.list(&mut self.clock);
        self.saved.list(&mut self.clock);
    }

    pub fn now(&self) -> i64 {
        self.clock.now
    }

    /// The time of the next timer due, if any.
    pub fn next_due(&self) -> Option<i64> {
        self.clock.timers.iter().map(|timer| timer.at).min()
    }

    /// Fires the next timer due by `to`, at its time; false when none is.
    pub fn fire_next(&mut self, to: i64) -> bool {
        let next = self
            .clock
            .timers
            .iter()
            .enumerate()
            .filter(|(_, timer)| timer.at <= to)
            .min_by_key(|(_, timer)| (timer.at, timer.seq))
            .map(|(index, _)| index);
        let Some(index) = next else { return false };
        let timer = self.clock.timers.remove(index);
        self.clock.now = timer.at;
        match timer.job {
            Job::Heard(Name::Resources) => self.resources.heard(timer.seq),
            Job::Heard(Name::History) => self.history.heard(timer.seq),
            Job::Heard(Name::Saved) => self.saved.heard(timer.seq),
            Job::Stage(ticket, call) => self.stage(ticket, *call),
        }
        true
    }

    /// The calls answered since they were last taken.
    pub fn take_settled(&mut self) -> Vec<Settled> {
        std::mem::take(&mut self.settled)
    }

    fn ticket(&mut self) -> u64 {
        self.tickets += 1;
        self.tickets
    }

    fn next(&mut self, ticket: u64, call: Call) {
        self.clock.after(LATENCY, Job::Stage(ticket, Box::new(call)));
    }

    fn settle(&mut self, ticket: u64, outcome: Outcome) {
        self.settled.push(Settled { ticket, outcome });
    }

    /// Updates `Type/id` with a resource, as `PUT [base]/Type/id` with `If-Match` the version it
    /// was read at: refused with the issues found when it does not check, and with a conflict when
    /// someone saved a newer version since. Answers the version kept and the warnings it carried.
    pub fn update(&mut self, kind: &str, id: &str, resource: Value, read_at: f64) -> u64 {
        let ticket = self.ticket();
        let all = self.resources.rows.clone();
        self.next(
            ticket,
            Call::Update {
                kind: kind.to_owned(),
                id: id.to_owned(),
                resource,
                read_at,
                all,
                stage: 1,
                kept: None,
            },
        );
        ticket
    }

    /// Creates a resource, as `POST [base]/Type`: under the id it names when that is free, else
    /// under one the server makes.
    pub fn create(&mut self, kind: &str, resource: Value) -> u64 {
        let ticket = self.ticket();
        let all = self.resources.rows.clone();
        self.next(
            ticket,
            Call::Create {
                kind: kind.to_owned(),
                resource,
                all,
                stage: 1,
                id: String::new(),
                with_id: Value::Null,
                issues: Vec::new(),
                past: Vec::new(),
                kept: None,
            },
        );
        ticket
    }

    /// Deletes `Type/id`, as `DELETE [base]/Type/id`, refused while other resources point at it.
    pub fn remove(&mut self, key: &str) -> u64 {
        let ticket = self.ticket();
        let all = self.resources.rows.clone();
        self.next(
            ticket,
            Call::Remove {
                key: key.to_owned(),
                all,
                stage: 1,
                kind: String::new(),
                version_id: 0,
            },
        );
        ticket
    }

    /// Keeps the person's saved searches, in the order given.
    pub fn keep_searches(&mut self, searches: Vec<SavedSearch>) -> u64 {
        let ticket = self.ticket();
        let rows = self.saved.rows.clone();
        self.next(
            ticket,
            Call::KeepSearches {
                searches,
                rows,
                stage: 1,
            },
        );
        ticket
    }

    /// `keep`: the resource with its new version and the instant, written to the resources and,
    /// a moment later, the history.
    fn keep_resource(
        &mut self,
        all: &[Stored],
        kind: &str,
        resource: &Value,
        version_id: i64,
    ) -> Result<Value, Failure> {
        let at = iso_instant(self.clock.now);
        let kept = with_meta(resource, version_id, &at);
        let key = key_of(&kept);
        if all.iter().any(|row| row.id == key) {
            let fresh = kept.clone();
            self.resources
                .replace(&mut self.clock, &key, |row| row.resource = fresh)
                .map_err(Failure::Error)?;
        } else {
            self.resources.prepend(
                &mut self.clock,
                Stored {
                    id: key,
                    kind: kind.to_owned(),
                    resource: kept.clone(),
                },
            );
        }
        Ok(kept)
    }

    fn record(&mut self, kind: &str, resource: &Value, change: Change, version_id: i64) {
        let key = key_of(resource);
        let at = crate::model::last_updated(resource).unwrap_or_default();
        self.history.prepend(
            &mut self.clock,
            Version {
                id: format!("{key}/_history/{version_id}"),
                key,
                kind: kind.to_owned(),
                version_id,
                at,
                by: ME.to_owned(),
                change,
                resource: Some(resource.clone()),
            },
        );
    }

    fn stage(&mut self, ticket: u64, call: Call) {
        match call {
            Call::Update {
                kind,
                id,
                resource,
                read_at,
                all,
                stage,
                kept,
            } => match stage {
                1 => {
                    let key = format!("{kind}/{id}");
                    let Some(current) = all.iter().find(|row| row.id == key).cloned() else {
                        let refused = Refused::new(
                            404,
                            vec![Issue::error("not-found", None, format!("The server holds no {key}."))],
                        );
                        return self.settle(ticket, Outcome::Failed(Failure::Refused(refused)));
                    };
                    let issues = validate(&resource, &kind, Some(&id), &|target| {
                        all.iter().any(|row| row.id == target)
                    });
                    if let Some(failure) = refuse(&issues) {
                        return self.settle(ticket, Outcome::Failed(failure));
                    }
                    let now = version_of(&current.resource);
                    if now != read_at {
                        let refused = Refused::new(
                            412,
                            vec![Issue::error(
                                "conflict",
                                None,
                                format!(
                                    "{key} was saved as version {} since this was read at version {}. Reload it to see the change.",
                                    crate::js::number_to_string(now),
                                    crate::js::number_to_string(read_at)
                                ),
                            )],
                        );
                        return self.settle(ticket, Outcome::Failed(Failure::Refused(refused)));
                    }
                    if without_meta(&resource) == without_meta(&current.resource) {
                        return self.settle(
                            ticket,
                            Outcome::Kept {
                                resource: current.resource,
                                warnings: vec![Issue::new(
                                    Severity::Information,
                                    "informational",
                                    None,
                                    "Nothing changed, so no new version was made.",
                                )],
                            },
                        );
                    }
                    match self.keep_resource(&all, &kind, &resource, now as i64 + 1) {
                        Err(failure) => self.settle(ticket, Outcome::Failed(failure)),
                        Ok(stored) => self.next(
                            ticket,
                            Call::Update {
                                kind,
                                id,
                                resource,
                                read_at,
                                all,
                                stage: 2,
                                kept: Some((stored, issues)),
                            },
                        ),
                    }
                }
                2 => {
                    let (stored, issues) = kept.unwrap_or_default();
                    let version_id = version_of(&stored) as i64;
                    self.record(&kind, &stored, Change::Update, version_id);
                    self.next(
                        ticket,
                        Call::Update {
                            kind,
                            id,
                            resource,
                            read_at,
                            all,
                            stage: 3,
                            kept: Some((stored, issues)),
                        },
                    );
                }
                _ => {
                    let (resource, warnings) = kept.unwrap_or_default();
                    self.settle(ticket, Outcome::Kept { resource, warnings });
                }
            },
            Call::Create {
                kind,
                resource,
                all,
                stage,
                id,
                with_id,
                issues,
                past,
                kept,
            } => match stage {
                1 => {
                    let fields = resource.as_object().cloned().unwrap_or_default();
                    let named = fields
                        .get("id")
                        .and_then(Value::as_str)
                        .filter(|one| !one.is_empty())
                        .map(str::to_owned);
                    if let Some(named) = &named
                        && all.iter().any(|row| row.id == format!("{kind}/{named}"))
                    {
                        let refused = Refused::new(
                            409,
                            vec![Issue::error(
                                "duplicate",
                                Some(vec![crate::model::Part::from("id")]),
                                format!("{kind}/{named} is taken; leave id empty for the server to make one."),
                            )],
                        );
                        return self.settle(ticket, Outcome::Failed(Failure::Refused(refused)));
                    }
                    let id = named.unwrap_or_else(|| fresh_id(&all, &kind));
                    let mut with_id = fields;
                    with_id.insert("id".to_owned(), json!(id));
                    with_id.shift_remove("meta");
                    let with_id = Value::Object(with_id);
                    let issues = validate(&with_id, &kind, Some(&id), &|target| {
                        all.iter().any(|row| row.id == target)
                    });
                    if let Some(failure) = refuse(&issues) {
                        return self.settle(ticket, Outcome::Failed(failure));
                    }
                    let past: Vec<Version> = self
                        .history
                        .rows
                        .iter()
                        .filter(|version| version.key == format!("{kind}/{id}"))
                        .cloned()
                        .collect();
                    self.next(
                        ticket,
                        Call::Create {
                            kind,
                            resource,
                            all,
                            stage: 2,
                            id,
                            with_id,
                            issues,
                            past,
                            kept: None,
                        },
                    );
                }
                2 => {
                    let version_id = past.iter().map(|one| one.version_id).max().unwrap_or(0).max(0) + 1;
                    match self.keep_resource(&all, &kind, &with_id, version_id) {
                        Err(failure) => self.settle(ticket, Outcome::Failed(failure)),
                        Ok(stored) => self.next(
                            ticket,
                            Call::Create {
                                kind,
                                resource,
                                all,
                                stage: 3,
                                id,
                                with_id,
                                issues,
                                past,
                                kept: Some(stored),
                            },
                        ),
                    }
                }
                3 => {
                    let stored = kept.clone().unwrap_or_default();
                    let version_id = version_of(&stored) as i64;
                    self.record(&kind, &stored, Change::Create, version_id);
                    self.next(
                        ticket,
                        Call::Create {
                            kind,
                            resource,
                            all,
                            stage: 4,
                            id,
                            with_id,
                            issues,
                            past,
                            kept,
                        },
                    );
                }
                _ => self.settle(
                    ticket,
                    Outcome::Kept {
                        resource: kept.unwrap_or_default(),
                        warnings: issues,
                    },
                ),
            },
            Call::Remove {
                key,
                all,
                stage,
                kind,
                version_id,
            } => match stage {
                1 => {
                    let Some(current) = all.iter().find(|row| row.id == key).cloned() else {
                        let refused = Refused::new(
                            404,
                            vec![Issue::error("not-found", None, format!("The server holds no {key}."))],
                        );
                        return self.settle(ticket, Outcome::Failed(Failure::Refused(refused)));
                    };
                    let pointers = pointing_at(&all, &key);
                    if !pointers.is_empty() {
                        let refused = Refused::new(
                            409,
                            vec![Issue::error(
                                "conflict",
                                None,
                                format!(
                                    "{} resource{} point at {key}, such as {}. Delete or change them first.",
                                    pointers.len(),
                                    if pointers.len() == 1 { "" } else { "s" },
                                    pointers.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                                ),
                            )],
                        );
                        return self.settle(ticket, Outcome::Failed(Failure::Refused(refused)));
                    }
                    let version_id = version_of(&current.resource) as i64 + 1;
                    self.resources.remove(&mut self.clock, &key);
                    self.next(
                        ticket,
                        Call::Remove {
                            key,
                            all,
                            stage: 2,
                            kind: current.kind,
                            version_id,
                        },
                    );
                }
                2 => {
                    let at = iso_instant(self.clock.now);
                    self.history.prepend(
                        &mut self.clock,
                        Version {
                            id: format!("{key}/_history/{version_id}"),
                            key: key.clone(),
                            kind: kind.clone(),
                            version_id,
                            at,
                            by: ME.to_owned(),
                            change: Change::Delete,
                            resource: None,
                        },
                    );
                    self.next(
                        ticket,
                        Call::Remove {
                            key,
                            all,
                            stage: 3,
                            kind,
                            version_id,
                        },
                    );
                }
                _ => self.settle(ticket, Outcome::Done),
            },
            Call::KeepSearches { searches, rows, stage } => match stage {
                1 => {
                    if rows.iter().any(|row| row.id == ME) {
                        let next = searches.clone();
                        if let Err(why) = self.saved.replace(&mut self.clock, ME, |row| row.searches = next) {
                            return self.settle(ticket, Outcome::Failed(Failure::Error(why)));
                        }
                    } else {
                        self.saved.prepend(
                            &mut self.clock,
                            SavedSearches {
                                id: ME.to_owned(),
                                searches: searches.clone(),
                            },
                        );
                    }
                    self.next(
                        ticket,
                        Call::KeepSearches {
                            searches,
                            rows,
                            stage: 2,
                        },
                    );
                }
                _ => self.settle(ticket, Outcome::Done),
            },
        }
    }
}
