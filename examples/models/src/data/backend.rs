//! The app's server, faked in the page, as the web's `data/backend.ts` keeps it on the shared fake
//! backend (`tidevann-shared/backend`): every collection the Models app keeps, each change made and
//! kept at once and answered `LATENCY` milliseconds later, as a server would. Every change is told
//! to those listening that much later too, with the rows as they were when it was made; what the
//! engines read (`seen`, the web's `mirror`) is the rows as last told.
//!
//! `Backend` is the whole page: the collections, the clock and the microtasks they run on
//! (`runtime`), and the engines' own state, which `engine::*` drive. Carried in shape from the
//! dhis2 example's backend (`examples/dhis2/src/backend.rs`), its timers here a `Runtime` of
//! promises, since the engines chain answers as the web's `async` functions do: so its collections
//! are its own, and only `LATENCY` and `Row` are the shared backend's (`tidevann_shared::backend`).

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use sokkel_egui::{Filter, FilterOperator, ListViewState};

use super::model::{
    Comparison, Conversation, Download, Level, Loaded, LogEvent, LogSource, Model, Prefs, Runner, default_prefs,
};
use super::seed::{Ids, seed_comparisons, seed_conversations, seed_events, seed_loaded, seed_models, seed_runner};
use crate::engine::stream::{AnswerRun, Live, Writer};
use crate::js::base36;
use crate::runtime::{Host, Promise, Runtime, Value};

pub use tidevann_shared::backend::{LATENCY, Row};

macro_rules! row {
    ($($kind:ty),*) => {
        $(impl Row for $kind {
            fn id(&self) -> &str {
                &self.id
            }
        })*
    };
}

row!(
    Model,
    Conversation,
    Download,
    Runner,
    Loaded,
    LogEvent,
    Comparison,
    Prefs,
    LibraryView
);

/// A view of the library: its filters, sort and columns under a name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibraryView {
    pub id: String,
    pub name: String,
    pub state: ListViewState,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub locked: Option<bool>,
}

/// The views the library starts with.
pub fn seed_views() -> Vec<LibraryView> {
    vec![
        LibraryView {
            id: "all".to_owned(),
            name: "All models".to_owned(),
            state: ListViewState::default(),
            locked: Some(true),
        },
        LibraryView {
            id: "small".to_owned(),
            name: "Fits a laptop".to_owned(),
            state: ListViewState {
                filters: vec![Filter::new("size", FilterOperator::AtMost, ["6"])],
                sort: Some(("size".to_owned(), true)),
                ..ListViewState::default()
            },
            locked: None,
        },
        LibraryView {
            id: "code".to_owned(),
            name: "For code".to_owned(),
            state: ListViewState {
                filters: vec![Filter::new("capabilities", FilterOperator::OneOf, ["code"])],
                hidden: vec!["context".to_owned()],
                ..ListViewState::default()
            },
            locked: None,
        },
    ]
}

/// One collection: its rows as the backend keeps them, and as they were last told (`seen`).
#[derive(Clone, Debug)]
pub struct Collection<T: Row> {
    pub name: &'static str,
    /// The rows as kept, every change made at once.
    pub rows: Vec<T>,
    /// The rows as last told or listed, none until the first answer: what the engines read.
    pub seen: Vec<T>,
}

impl<T: Row> Collection<T> {
    fn new(name: &'static str, rows: Vec<T>) -> Self {
        Collection {
            name,
            rows,
            seen: Vec::new(),
        }
    }

    /// The row under `id` as kept.
    pub fn get(&self, id: &str) -> Option<&T> {
        self.rows.iter().find(|row| row.id() == id)
    }

    /// The row under `id` as last told.
    pub fn seen(&self, id: &str) -> Option<&T> {
        self.seen.iter().find(|row| row.id() == id)
    }
}

/// Picks one collection out of the backend.
pub type Pick<T> = fn(&mut Backend) -> &mut Collection<T>;

/// The page: its collections, its clock, and the engines' state.
pub struct Backend {
    pub runtime: Runtime<Backend>,
    pub models: Collection<Model>,
    pub conversations: Collection<Conversation>,
    pub downloads: Collection<Download>,
    pub runner: Collection<Runner>,
    pub loaded: Collection<Loaded>,
    pub events: Collection<LogEvent>,
    pub comparisons: Collection<Comparison>,
    pub prefs: Collection<Prefs>,
    pub views: Collection<LibraryView>,
    /// The backend's count of rows made, which names a row made without an id.
    pub made: i64,
    /// The log's own count, which names each line.
    pub logged: i64,
    /// The ids the seed and the chat make for messages and files.
    pub ids: Ids,
    /// The routes gone to, as `go` sets them.
    pub went: Vec<String>,
    /// Pulls that landed in the library, as the app hears them.
    pub pulled: Vec<Download>,

    // --- The stream engine ---
    /// Every answer being written, by the key its owner gave it, in the order they began.
    pub live: Vec<(String, Live)>,
    pub(crate) stopping: HashSet<String>,
    /// The answer to end at once when each key still waiting for its model is stopped.
    pub(crate) waiting: HashMap<String, u64>,
    pub(crate) runs: HashMap<u64, AnswerRun>,
    pub(crate) writers: HashMap<u64, Writer>,
    pub(crate) answers_made: u64,

    // --- The chat engine ---
    pub(crate) queues: HashMap<String, Promise>,

    // --- The runner engine ---
    pub(crate) busy: HashMap<String, i64>,
    pub(crate) loads: HashMap<String, Promise>,
    pub(crate) reserving: Option<Promise>,

    // --- The download engine ---
    pub(crate) queueing: HashSet<String>,
    pub(crate) ticking: bool,
}

impl Host for Backend {
    fn runtime(&mut self) -> &mut Runtime<Self> {
        &mut self.runtime
    }
}

impl Backend {
    /// The page as it opens at `now`: every collection from its seed and listed by the engines,
    /// and the engines started, as the web's `App` starts them.
    pub fn new(now: i64) -> Self {
        let mut ids = Ids::default();
        let library = seed_models(now);
        let loaded = seed_loaded(&library, now);
        let conversations = seed_conversations(&mut ids, &library, now);
        let comparisons = seed_comparisons(&mut ids, &library, now);
        let mut backend = Backend {
            runtime: Runtime::new(now),
            models: Collection::new("library", library),
            conversations: Collection::new("conversations", conversations),
            downloads: Collection::new("downloads", Vec::new()),
            runner: Collection::new("runner", seed_runner(now)),
            loaded: Collection::new("loaded", loaded),
            events: Collection::new("events", seed_events(now)),
            comparisons: Collection::new("comparisons", comparisons),
            prefs: Collection::new("prefs", vec![default_prefs()]),
            views: Collection::new("views", seed_views()),
            made: 0,
            logged: 0,
            ids,
            went: Vec::new(),
            pulled: Vec::new(),
            live: Vec::new(),
            stopping: HashSet::new(),
            waiting: HashMap::new(),
            runs: HashMap::new(),
            writers: HashMap::new(),
            answers_made: 0,
            queues: HashMap::new(),
            busy: HashMap::new(),
            loads: HashMap::new(),
            reserving: None,
            queueing: HashSet::new(),
            ticking: false,
        };
        // The engines' mirrors list what they read as their modules load.
        backend.list(|b| &mut b.runner);
        backend.list(|b| &mut b.loaded);
        backend.list(|b| &mut b.models);
        backend.list(|b| &mut b.prefs);
        backend.list(|b| &mut b.conversations);
        backend.list(|b| &mut b.models);
        backend.list(|b| &mut b.comparisons);
        backend.list(|b| &mut b.downloads);
        backend.list(|b| &mut b.models);
        backend.list(|b| &mut b.prefs);
        backend.start_runner_engine();
        backend.start_download_engine();
        backend
    }

    /// A collection's rows answered after the delay, which its mirror then reads.
    pub fn list<T: Row + 'static>(&mut self, pick: Pick<T>) -> Promise {
        let rows = pick(self).rows.clone();
        let answer = self.later(Ok(Rc::new(rows.clone())));
        self.on(answer, move |backend, _| pick(backend).seen = rows);
        answer
    }

    /// An answer after the delay.
    fn later(&mut self, outcome: Result<Value, String>) -> Promise {
        let promise = self.pending();
        self.set_timeout(LATENCY, move |backend| backend.settle(promise, outcome));
        promise
    }

    /// Keeps a collection's rows and tells them to those listening after the delay.
    fn save<T: Row + 'static>(&mut self, pick: Pick<T>, rows: Vec<T>) {
        pick(self).rows = rows.clone();
        self.set_timeout(LATENCY, move |backend| pick(backend).seen = rows);
    }

    /// Adds a row under `id`, or under an id the backend makes from the collection's name.
    pub fn create<T: Row + 'static>(
        &mut self,
        pick: Pick<T>,
        make: impl FnOnce(String) -> T,
        id: Option<String>,
    ) -> Promise {
        self.made += 1;
        let name = pick(self).name;
        let now = self.now();
        let id = id.unwrap_or_else(|| format!("{name}-{}-{}", base36(now), self.made));
        let row = make(id);
        let mut rows = vec![row.clone()];
        rows.extend(pick(self).rows.iter().cloned());
        self.save(pick, rows);
        self.later(Ok(Rc::new(row)))
    }

    /// Adds several rows at once, each under its own id, as one change.
    pub fn insert<T: Row + 'static>(&mut self, pick: Pick<T>, added: Vec<T>) -> Promise {
        let mut rows = added.clone();
        rows.extend(pick(self).rows.iter().cloned());
        self.save(pick, rows);
        self.later(Ok(Rc::new(added)))
    }

    /// Changes the row under `id`; there being none is refused.
    pub fn update<T: Row + 'static>(&mut self, pick: Pick<T>, id: &str, change: impl FnOnce(&mut T)) -> Promise {
        let name = pick(self).name;
        let Some(mut row) = pick(self).get(id).cloned() else {
            return self.later(Err(format!("No {name} with the id {id}.")));
        };
        change(&mut row);
        let rows = pick(self)
            .rows
            .iter()
            .map(|one| if one.id() == id { row.clone() } else { one.clone() })
            .collect();
        self.save(pick, rows);
        self.later(Ok(Rc::new(row)))
    }

    pub fn remove<T: Row + 'static>(&mut self, pick: Pick<T>, id: &str) -> Promise {
        let rows = pick(self).rows.iter().filter(|one| one.id() != id).cloned().collect();
        self.save(pick, rows);
        self.later(Ok(crate::runtime::unit()))
    }

    /// Writes a line to the runner's log.
    pub fn log(&mut self, level: Level, source: LogSource, text: impl Into<String>) {
        self.logged += 1;
        let now = self.now();
        let text = text.into();
        let id = format!("e-{}-{}", base36(now), self.logged);
        self.create(
            |b| &mut b.events,
            move |id| LogEvent {
                id,
                time: now,
                level,
                source,
                text,
            },
            Some(id),
        );
    }

    /// Goes to a route, as the web's `go` sets the hash.
    pub fn go(&mut self, path: impl Into<String>) {
        self.went.push(path.into());
    }

    /// The preferences as the engines read them.
    pub fn prefs_now(&self) -> Prefs {
        self.prefs.seen.first().cloned().unwrap_or_else(default_prefs)
    }
}
