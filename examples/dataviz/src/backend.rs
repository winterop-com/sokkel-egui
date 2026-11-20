//! The visualizer's server, as the web's `backend.ts` keeps it on the shared fake backend
//! (`tidevann-shared/backend`): datasets, the charts saved from them, boards of charts and the
//! preferences, each a collection seeded the first time; and every change the pages and dialogs
//! make to them, with what follows once it is answered.
//!
//! Each collection is the web's `collection`: a change is made and kept at once, its answer comes
//! `LATENCY` milliseconds later, and every change is told to those listening that much later with
//! the rows as they were when it was made. A page reads a collection's rows as they were last told
//! or listed (`Collection::seen`), none until the first answer, so what a page shows lags a change
//! by the same moment the web's does. The web's timers are kept as the web sets them, and
//! `Backend::advance` moves the clock. Where the changes stand (`saving`) follows the web's
//! `savingNow`. The collections, the clock and `Saving` are the shared backend's
//! (`tidevann_shared::backend`).

use sokkel_egui::Filter;

use crate::model::{
    Board, BoardItem, Column, ColumnType, DataRow, Dataset, DatasetSource, PanelSpan, Preferences, SavedChart,
    ViewSpec, counted, fresh_id, iso_time,
};
use crate::parse::{ParsedTable, normalize, rows_of, to_csv};
use crate::seed::{seed_boards, seed_charts, seed_datasets};
use crate::share::{SharedChart, shared_dataset};
use sokkel_egui_charts::data::Cell;
use tidevann_shared::backend;
pub use tidevann_shared::backend::{Clock, LATENCY, Row, Saving, saving_note};

/// Changes a row by a change, which takes its type from the row's.
fn apply<T, F: FnOnce(&mut T)>(row: &mut T, change: F) {
    change(row);
}

/// Adds a row at the front of a collection, as the backend's `create` does.
macro_rules! create {
    ($self:ident, $field:ident, $row:expr, $then:expr $(,)?) => {{
        let row = $row;
        let then = $then;
        let mut next = vec![row];
        next.extend($self.$field.rows.iter().cloned());
        Backend::save(&mut $self.clock, $self.sealed, &mut $self.$field, next);
        $self.answered(None, then);
    }};
}

/// Changes the row under an id, as the backend's `update` does: there being none is refused, and
/// nothing is kept; whether it was there.
macro_rules! update {
    ($self:ident, $field:ident, $id:expr, $change:expr, $then:expr $(,)?) => {{
        let id: &str = $id;
        let then = $then;
        match $self.$field.get(id).cloned() {
            None => {
                let failure = format!("No {} with the id {id}.", $self.$field.name.key());
                $self.answered(Some(failure), Vec::new());
                false
            }
            Some(mut row) => {
                apply(&mut row, $change);
                let next = $self
                    .$field
                    .rows
                    .iter()
                    .map(|one| if one.id() == id { row.clone() } else { one.clone() })
                    .collect();
                Backend::save(&mut $self.clock, $self.sealed, &mut $self.$field, next);
                $self.answered(None, then);
                true
            }
        }
    }};
}

/// Takes the row under an id out of a collection, as the backend's `remove` does.
macro_rules! remove {
    ($self:ident, $field:ident, $id:expr, $then:expr $(,)?) => {{
        let id: &str = $id;
        let then = $then;
        let next = $self.$field.rows.iter().filter(|row| row.id() != id).cloned().collect();
        Backend::save(&mut $self.clock, $self.sealed, &mut $self.$field, next);
        $self.answered(None, then);
    }};
}

macro_rules! row {
    ($($kind:ty),*) => {
        $(impl Row for $kind {
            fn id(&self) -> &str {
                &self.id
            }
        })*
    };
}

row!(Dataset, SavedChart, Board, Preferences);

/// The collections, by the name the backend keeps each under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Name {
    Datasets,
    Charts,
    Boards,
    Preferences,
}

impl Name {
    pub fn key(self) -> &'static str {
        match self {
            Name::Datasets => "datasets",
            Name::Charts => "charts",
            Name::Boards => "boards",
            Name::Preferences => "preferences",
        }
    }
}

impl backend::Name for Name {
    fn key(self) -> &'static str {
        Name::key(self)
    }
}

/// A note the app shows for a moment.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Toast {
    /// `success`, or `error` for a failure.
    #[serde(rename = "type")]
    pub kind: Option<&'static str>,
    pub title: String,
    pub description: Option<String>,
    /// The words of its one button, such as Undo or Open, when it has one.
    pub action: Option<String>,
}

impl Toast {
    pub fn success(title: impl Into<String>) -> Self {
        Toast {
            kind: Some("success"),
            title: title.into(),
            description: None,
            action: None,
        }
    }

    pub fn with_action(mut self, action: &str) -> Self {
        self.action = Some(action.to_owned());
        self
    }
}

/// What a toast's button does: undo a chart taken off a board, or open the board one was added
/// to.
#[derive(Clone, Debug, PartialEq)]
pub enum ToastAction {
    /// Puts a board's items back as they were.
    Restore {
        board: String,
        items: Vec<BoardItem>,
    },
    Open(String),
}

/// What follows a change once it is answered.
#[derive(Clone, Debug, PartialEq)]
pub enum Then {
    Toast(Toast, Option<ToastAction>),
    /// Goes to a route, as the web's `go` sets the hash.
    Go(String),
    /// Replaces the address, as `window.location.replace` does.
    Replace(String),
    /// Loads the page again, after the demo data is reset.
    Reload,
    /// The second step of keeping a shared chart: its chart, on the dataset kept.
    KeepChart {
        name: String,
        dataset: String,
        spec: ViewSpec,
    },
}

#[derive(Clone, Debug)]
pub enum Job {
    /// A collection's change told to those listening, with the rows kept under the timer's seq.
    Tell(Name),
    /// A collection's list answered, with the rows kept under the timer's seq.
    Listed(Name),
    /// A change answered: kept or refused, and what follows it when kept.
    Answer { failure: Option<String>, then: Vec<Then> },
}

/// A collection of the backend, under its name.
pub type Collection<T> = backend::Collection<T, Name>;

/// A change to the preferences: the fields given, the rest kept.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrefsPatch {
    pub palette: Option<String>,
    pub numbers: Option<crate::model::NumberStyle>,
    pub decimals: Option<crate::model::Decimals>,
    pub aggregate: Option<crate::model::Aggregate>,
    pub suggestions: Option<bool>,
    pub board_order: Option<Vec<String>>,
}

impl PrefsPatch {
    fn apply(&self, prefs: &mut Preferences) {
        if let Some(palette) = &self.palette {
            prefs.palette = palette.clone();
        }
        if let Some(numbers) = self.numbers {
            prefs.numbers = numbers;
        }
        if let Some(decimals) = self.decimals {
            prefs.decimals = decimals;
        }
        if let Some(aggregate) = self.aggregate {
            prefs.aggregate = aggregate;
        }
        if let Some(suggestions) = self.suggestions {
            prefs.suggestions = suggestions;
        }
        if let Some(order) = &self.board_order {
            prefs.board_order = Some(order.clone());
        }
    }
}

/// A file the app handed the reader to save.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Download {
    pub name: String,
    pub text: String,
}

/// Where an import came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportSource {
    File,
    Paste,
}

/// The visualizer's backend on the web's clock, and what the app said and where it went.
#[derive(Clone, Debug)]
pub struct Backend {
    pub clock: Clock<Job>,
    /// How many ids `fresh_id` made, whose count ends each.
    pub made: u64,
    pub datasets: Collection<Dataset>,
    pub charts: Collection<SavedChart>,
    pub boards: Collection<Board>,
    pub preferences: Collection<Preferences>,
    /// Where the changes stand, and how many are on their way.
    pub saving: Saving,
    pending: usize,
    /// Set once the demo data is reset, so nothing written as the page reloads is kept.
    sealed: bool,
    /// The notes shown since they were last taken, with what each one's button does.
    pub toasts: Vec<(Toast, Option<ToastAction>)>,
    /// The places gone to since they were last taken: a route as `go` sets it, `replace #/...` as
    /// `location.replace` does, `reload`.
    pub went: Vec<String>,
    /// The files handed out to save.
    pub downloads: Vec<Download>,
}

impl Backend {
    /// The backend as the app opens at `now`: every collection from its seed, listed once as the
    /// visualizer's frame lists them.
    pub fn new(now: i64) -> Self {
        let mut backend = Backend {
            clock: Clock::new(now),
            made: 0,
            datasets: Collection::new(Name::Datasets, seed_datasets()),
            charts: Collection::new(Name::Charts, seed_charts()),
            boards: Collection::new(Name::Boards, seed_boards()),
            preferences: Collection::new(Name::Preferences, vec![Preferences::default()]),
            saving: Saving::Saved,
            pending: 0,
            sealed: false,
            toasts: Vec::new(),
            went: Vec::new(),
            downloads: Vec::new(),
        };
        for name in [Name::Datasets, Name::Charts, Name::Boards, Name::Preferences] {
            backend.list(name);
        }
        backend
    }

    pub fn now(&self) -> i64 {
        self.clock.now
    }

    /// Now, as an ISO time.
    pub fn now_iso(&self) -> String {
        iso_time(self.clock.now)
    }

    /// A fresh id with a prefix, as the web's `freshId` makes it now.
    pub fn fresh_id(&mut self, prefix: &str) -> String {
        fresh_id(prefix, self.clock.now, &mut self.made)
    }

    /// The preferences as the pages read them: the first row the backend told, or the defaults.
    pub fn prefs(&self) -> Preferences {
        self.preferences.seen().first().cloned().unwrap_or_default()
    }

    /// Whether every collection has answered, so the pages are drawn.
    pub fn loaded(&self) -> bool {
        self.datasets.seen.is_some() && self.charts.seen.is_some() && self.boards.seen.is_some()
    }

    /// Fires every timer due by `to`, each at its time, in the order they were set.
    pub fn advance(&mut self, to: i64) {
        while let Some((seq, job)) = self.clock.take_due(to) {
            self.fire(seq, job);
        }
        self.clock.reach(to);
    }

    /// The time of the next timer due, if any.
    pub fn next_due(&self) -> Option<i64> {
        self.clock.next_due()
    }

    fn fire(&mut self, seq: u64, job: Job) {
        match job {
            Job::Tell(name) | Job::Listed(name) => match name {
                Name::Datasets => self.datasets.heard(seq),
                Name::Charts => self.charts.heard(seq),
                Name::Boards => self.boards.heard(seq),
                Name::Preferences => self.preferences.heard(seq),
            },
            Job::Answer { failure, then } => {
                self.saving.settle(&mut self.pending, failure.as_deref());
                if failure.is_none() {
                    for one in then {
                        self.follow(one);
                    }
                }
            }
        }
    }

    fn follow(&mut self, then: Then) {
        match then {
            Then::Toast(toast, action) => self.toasts.push((toast, action)),
            Then::Go(path) => self.went.push(path),
            Then::Replace(path) => self.went.push(format!("replace {path}")),
            Then::Reload => self.went.push("reload".to_owned()),
            Then::KeepChart { name, dataset, spec } => {
                let id = self.fresh_id("ch");
                let chart = SavedChart {
                    id: id.clone(),
                    name: name.clone(),
                    dataset_id: dataset,
                    spec,
                    updated_at: self.now_iso(),
                };
                self.create_chart(
                    chart,
                    vec![
                        Then::Toast(Toast::success(format!("{name} and its data kept here")), None),
                        Then::Go(format!("/charts/{id}")),
                    ],
                );
            }
        }
    }

    /// Goes to a route, as the web's `go` sets the hash.
    pub fn go(&mut self, path: impl Into<String>) {
        self.went.push(path.into());
    }

    /// Hands a file to the reader to save.
    pub fn download(&mut self, name: impl Into<String>, text: impl Into<String>) {
        self.downloads.push(Download {
            name: name.into(),
            text: text.into(),
        });
    }

    // --- The collections' calls ------------------------------------------------------------------

    fn list(&mut self, name: Name) {
        let clock = &mut self.clock;
        match name {
            Name::Datasets => self.datasets.tell(clock, Job::Listed),
            Name::Charts => self.charts.tell(clock, Job::Listed),
            Name::Boards => self.boards.tell(clock, Job::Listed),
            Name::Preferences => self.preferences.tell(clock, Job::Listed),
        }
    }

    /// Keeps a collection's rows and tells those listening, unless the data was reset.
    fn save<T: Row>(clock: &mut Clock<Job>, sealed: bool, collection: &mut Collection<T>, next: Vec<T>) {
        if !sealed {
            collection.save(clock, next, Job::Tell);
        }
    }

    /// A change made: followed by `saving` until it is answered, with what follows it.
    fn answered(&mut self, failure: Option<String>, then: Vec<Then>) {
        self.saving.follow(&mut self.pending);
        self.clock.after(LATENCY, Job::Answer { failure, then });
    }

    fn create_chart(&mut self, chart: SavedChart, then: Vec<Then>) {
        create!(self, charts, chart, then);
    }

    // --- Datasets --------------------------------------------------------------------------------

    /// Keeps a table read in the import dialog as a new dataset and opens it.
    pub fn import(
        &mut self,
        name: &str,
        source: ImportSource,
        file: Option<&str>,
        columns: &[Column],
        table: &ParsedTable,
    ) -> String {
        let id = self.fresh_id("ds");
        let description = match (source, file) {
            (ImportSource::File, Some(file)) => format!("Imported from {file}."),
            _ => "Pasted in.".to_owned(),
        };
        let rows = rows_of(table, columns, &id);
        let name = crate::js::trim(name).to_owned();
        let count = rows.len();
        let dataset = Dataset {
            id: id.clone(),
            name: name.clone(),
            description,
            source: match source {
                ImportSource::File => DatasetSource::File,
                ImportSource::Paste => DatasetSource::Paste,
            },
            columns: columns.to_vec(),
            rows,
            updated_at: self.now_iso(),
        };
        create!(
            self,
            datasets,
            dataset,
            vec![
                Then::Toast(
                    Toast::success(format!("{name} imported, {}", counted(count, "row"))),
                    None
                ),
                Then::Go(format!("/datasets/{id}")),
            ],
        );
        id
    }

    /// Names a dataset anew, from its title.
    pub fn rename_dataset(&mut self, id: &str, value: &str) {
        let (name, at) = (crate::js::trim(value).to_owned(), self.now_iso());
        update!(
            self,
            datasets,
            id,
            |row| {
                row.name = name;
                row.updated_at = at;
            },
            Vec::new(),
        );
    }

    /// Describes a dataset anew, from under its title.
    pub fn describe_dataset(&mut self, id: &str, value: &str) {
        let (description, at) = (crate::js::trim(value).to_owned(), self.now_iso());
        update!(
            self,
            datasets,
            id,
            |row| {
                row.description = description;
                row.updated_at = at;
            },
            Vec::new(),
        );
    }

    /// A copy of a dataset as the page shows it, opened once it is made.
    pub fn duplicate_dataset(&mut self, dataset: &Dataset) {
        let id = self.fresh_id("ds");
        let copy = Dataset {
            id: id.clone(),
            name: format!("{} copy", dataset.name),
            updated_at: self.now_iso(),
            ..dataset.clone()
        };
        let toast = Toast::success(format!("{} made", copy.name));
        create!(
            self,
            datasets,
            copy,
            vec![Then::Toast(toast, None), Then::Go(format!("/datasets/{id}"))],
        );
    }

    /// Deletes a dataset and the charts drawn from it, as the page knows them.
    pub fn delete_dataset(&mut self, dataset: &Dataset, charts: &[SavedChart]) {
        let drawn: Vec<String> = charts
            .iter()
            .filter(|chart| chart.dataset_id == dataset.id)
            .map(|chart| chart.id.clone())
            .collect();
        let then = vec![
            Then::Toast(Toast::success(format!("{} deleted", dataset.name)), None),
            Then::Go("/datasets".to_owned()),
        ];
        remove!(
            self,
            datasets,
            &dataset.id,
            if drawn.is_empty() { then.clone() } else { Vec::new() }
        );
        let last = drawn.len();
        for (at, chart) in drawn.iter().enumerate() {
            remove!(
                self,
                charts,
                chart,
                if at + 1 == last { then.clone() } else { Vec::new() }
            );
        }
    }

    /// A change to a dataset made to it as the store holds it now, its time set.
    fn write_dataset(&mut self, id: &str, change: impl FnOnce(&mut Dataset, &mut u64, i64), then: Vec<Then>) -> bool {
        let (now, at) = (self.clock.now, self.now_iso());
        let mut made = self.made;
        let done = update!(
            self,
            datasets,
            id,
            |row| {
                change(row, &mut made, now);
                row.updated_at = at;
            },
            then,
        );
        self.made = made;
        done
    }

    /// A cell's value saved as the text its column keeps.
    pub fn save_cell(&mut self, dataset: &str, row: &str, column: &Column, text: &str) {
        let value = normalize(text, column.kind);
        self.write_dataset(
            dataset,
            |current, _, _| {
                for one in current.rows.iter_mut().filter(|one| one.id == row) {
                    one.set(&column.id, value.clone());
                }
            },
            Vec::new(),
        );
    }

    /// A blank row added at the top.
    pub fn add_row(&mut self, dataset: &str) {
        self.write_dataset(
            dataset,
            |current, made, now| {
                let mut row = DataRow::new(fresh_id(&format!("{}-r", current.id), now, made));
                for column in &current.columns {
                    row.set(&column.id, "");
                }
                current.rows.insert(0, row);
            },
            vec![Then::Toast(Toast::success("A blank row added at the top"), None)],
        );
    }

    /// The rows under `ids` deleted.
    pub fn remove_rows(&mut self, dataset: &str, ids: &[String]) {
        let mut unique: Vec<&String> = Vec::new();
        for id in ids {
            if !unique.contains(&id) {
                unique.push(id);
            }
        }
        let count = unique.len();
        self.write_dataset(
            dataset,
            |current, _, _| current.rows.retain(|row| !ids.contains(&row.id)),
            vec![Then::Toast(
                Toast::success(format!("{} deleted", counted(count, "row"))),
                None,
            )],
        );
    }

    /// A copy of a row as the page shows it, after the row as the store holds it.
    pub fn duplicate_row(&mut self, dataset: &str, row: &DataRow) {
        self.write_dataset(
            dataset,
            |current, made, now| {
                let at = current
                    .rows
                    .iter()
                    .position(|one| one.id == row.id)
                    .map_or(0, |at| at + 1);
                let copy = DataRow {
                    id: fresh_id(&format!("{}-r", current.id), now, made),
                    values: row.values.clone(),
                };
                current.rows.insert(at, copy);
            },
            vec![Then::Toast(Toast::success("Row duplicated"), None)],
        );
    }

    /// A column renamed or retyped in its place; a new type rewrites the values it can read.
    pub fn set_column(&mut self, dataset: &str, at: usize, name: Option<&str>, kind: Option<ColumnType>) {
        self.write_dataset(
            dataset,
            |current, _, _| {
                let Some(column) = current.columns.get_mut(at) else {
                    return;
                };
                if let Some(name) = name {
                    column.name = name.to_owned();
                }
                if let Some(kind) = kind {
                    column.kind = kind;
                }
                let after = column.clone();
                if kind.is_some() {
                    for row in &mut current.rows {
                        let value = normalize(row.text(&after.id), after.kind);
                        row.set(&after.id, value);
                    }
                }
            },
            Vec::new(),
        );
    }

    /// A text column added after the rest, named `New column` or the first such name free.
    pub fn add_column(&mut self, dataset: &Dataset) {
        let used: Vec<&str> = dataset.columns.iter().map(|column| column.id.as_str()).collect();
        let mut at = dataset.columns.len() + 1;
        while used.contains(&format!("c{at}").as_str()) {
            at += 1;
        }
        let id = format!("c{at}");
        let names: Vec<String> = dataset
            .columns
            .iter()
            .map(|column| column.name.to_lowercase())
            .collect();
        let mut name = "New column".to_owned();
        let mut count = 2;
        while names.contains(&name.to_lowercase()) {
            name = format!("New column {count}");
            count += 1;
        }
        let toast = Toast::success(format!("{name} added; press its name to rename it"));
        self.write_dataset(
            &dataset.id,
            |current, _, _| {
                for row in &mut current.rows {
                    row.set(&id, "");
                }
                current.columns.push(Column {
                    id: id.clone(),
                    name: name.clone(),
                    kind: ColumnType::Text,
                });
            },
            vec![Then::Toast(toast, None)],
        );
    }

    /// A column deleted, its values from every row.
    pub fn drop_column(&mut self, dataset: &str, column: &Column) {
        self.write_dataset(
            dataset,
            |current, _, _| {
                for row in &mut current.rows {
                    row.drop_key(&column.id);
                }
                current.columns.retain(|one| one.id != column.id);
            },
            vec![Then::Toast(Toast::success(format!("{} deleted", column.name)), None)],
        );
    }

    // --- Charts ----------------------------------------------------------------------------------

    /// Saves the builder's draft: a new chart, or the one edited, or a copy of it as a new one.
    pub fn save_chart(&mut self, chart: Option<&SavedChart>, name: &str, dataset: &str, spec: &ViewSpec, as_new: bool) {
        let name = crate::js::trim(name).to_owned();
        let at = self.now_iso();
        let toast = Then::Toast(Toast::success(format!("{name} saved")), None);
        match chart {
            Some(chart) if !as_new => {
                let (dataset, spec) = (dataset.to_owned(), spec.clone());
                update!(
                    self,
                    charts,
                    &chart.id,
                    |row| {
                        row.name = name;
                        row.dataset_id = dataset;
                        row.spec = spec;
                        row.updated_at = at;
                    },
                    vec![toast],
                );
            }
            _ => {
                let id = self.fresh_id("ch");
                let saved = SavedChart {
                    id: id.clone(),
                    name,
                    dataset_id: dataset.to_owned(),
                    spec: spec.clone(),
                    updated_at: at,
                };
                self.create_chart(saved, vec![toast, Then::Replace(format!("#/charts/{id}"))]);
            }
        }
    }

    /// Places a saved chart on a board, half wide, from the builder.
    pub fn add_chart_to(&mut self, chart: &SavedChart, board: &Board) {
        let mut items = board.items.clone();
        items.push(BoardItem {
            id: self.fresh_id("it"),
            chart_id: chart.id.clone(),
            span: PanelSpan::Half,
        });
        let at = self.now_iso();
        let toast = Toast::success(format!("{} added to {}", chart.name, board.name)).with_action("Open");
        update!(
            self,
            boards,
            &board.id,
            |row| {
                row.items = items;
                row.updated_at = at;
            },
            vec![Then::Toast(
                toast,
                Some(ToastAction::Open(format!("/boards/{}", board.id)))
            )],
        );
    }

    /// A copy of a saved chart.
    pub fn duplicate_chart(&mut self, chart: &SavedChart) {
        let copy = SavedChart {
            id: self.fresh_id("ch"),
            name: format!("{} copy", chart.name),
            dataset_id: chart.dataset_id.clone(),
            spec: chart.spec.clone(),
            updated_at: self.now_iso(),
        };
        let toast = Toast::success(format!("{} made", copy.name));
        self.create_chart(copy, vec![Then::Toast(toast, None)]);
    }

    /// Deletes a saved chart, and takes it off the boards the page knows it is on.
    pub fn delete_chart(&mut self, chart: &SavedChart, boards: &[Board]) {
        let on: Vec<&Board> = boards
            .iter()
            .filter(|board| board.items.iter().any(|item| item.chart_id == chart.id))
            .collect();
        let then = vec![Then::Toast(Toast::success(format!("{} deleted", chart.name)), None)];
        remove!(
            self,
            charts,
            &chart.id,
            if on.is_empty() { then.clone() } else { Vec::new() }
        );
        let at = self.now_iso();
        let last = on.len();
        for (index, board) in on.into_iter().enumerate() {
            let items: Vec<BoardItem> = board
                .items
                .iter()
                .filter(|item| item.chart_id != chart.id)
                .cloned()
                .collect();
            let at = at.clone();
            let ok = update!(
                self,
                boards,
                &board.id,
                |row| {
                    row.items = items;
                    row.updated_at = at;
                },
                if index + 1 == last { then.clone() } else { Vec::new() },
            );
            if !ok {
                break;
            }
        }
    }

    // --- Boards ----------------------------------------------------------------------------------

    /// Makes a board, with a chart on it to start where one is given; `open` goes to it once made.
    pub fn create_board(&mut self, name: &str, description: &str, adding: Option<&SavedChart>, open: bool) -> String {
        let items = match adding {
            Some(chart) => vec![BoardItem {
                id: self.fresh_id("it"),
                chart_id: chart.id.clone(),
                span: PanelSpan::Half,
            }],
            None => Vec::new(),
        };
        let id = self.fresh_id("bd");
        let name = crate::js::trim(name).to_owned();
        let board = Board {
            id: id.clone(),
            name: name.clone(),
            description: crate::js::trim(description).to_owned(),
            items,
            filters: Vec::new(),
            updated_at: self.now_iso(),
        };
        let mut then = vec![Then::Toast(Toast::success(format!("{name} made")), None)];
        if open {
            then.push(Then::Go(format!("/boards/{id}")));
        }
        create!(self, boards, board, then);
        id
    }

    /// A board's name and description changed in its dialog.
    pub fn save_board_details(&mut self, id: &str, name: &str, description: &str) {
        let (name, description, at) = (
            crate::js::trim(name).to_owned(),
            crate::js::trim(description).to_owned(),
            self.now_iso(),
        );
        update!(
            self,
            boards,
            id,
            |row| {
                row.name = name;
                row.description = description;
                row.updated_at = at;
            },
            vec![Then::Toast(Toast::success("Board saved"), None)],
        );
    }

    /// A saved chart placed on a board at the width picked, from the board's dialog; `board` holds
    /// the items the page shows.
    pub fn add_to_board(&mut self, board: &Board, chosen: &str, span: PanelSpan, charts: &[SavedChart]) {
        let mut items = board.items.clone();
        items.push(BoardItem {
            id: self.fresh_id("it"),
            chart_id: chosen.to_owned(),
            span,
        });
        let at = self.now_iso();
        let named = charts
            .iter()
            .find(|chart| chart.id == chosen)
            .map_or("The chart", |chart| chart.name.as_str());
        let toast = Toast::success(format!("{named} added to {}", board.name));
        update!(
            self,
            boards,
            &board.id,
            |row| {
                row.items = items;
                row.updated_at = at;
            },
            vec![Then::Toast(toast, None)],
        );
    }

    /// Deletes a board; its charts stay saved.
    pub fn delete_board(&mut self, board: &Board) {
        remove!(
            self,
            boards,
            &board.id,
            vec![
                Then::Toast(Toast::success(format!("{} deleted", board.name)), None),
                Then::Go("/boards".to_owned()),
            ],
        );
    }

    /// The boards moved into a new order, kept in the preferences; the order shown after.
    pub fn reorder_boards(&mut self, shown: &[Board], from: usize, to: usize, prefs: &str) -> Vec<Board> {
        let mut next = shown.to_vec();
        if from != to && from < next.len() && to < next.len() {
            let moved = next.remove(from);
            next.insert(to, moved);
        }
        let order: Vec<String> = next.iter().map(|board| board.id.clone()).collect();
        update!(
            self,
            preferences,
            prefs,
            |row| row.board_order = Some(order),
            Vec::new(),
        );
        next
    }

    /// Names a board anew, from its title.
    pub fn rename_board(&mut self, id: &str, value: &str) {
        let (name, at) = (crate::js::trim(value).to_owned(), self.now_iso());
        update!(
            self,
            boards,
            id,
            |row| {
                row.name = name;
                row.updated_at = at;
            },
            Vec::new(),
        );
    }

    /// A copy of a board with the items and filters the page shows, opened once made.
    pub fn duplicate_board(&mut self, board: &Board, items: &[BoardItem], filters: &[Filter]) {
        let items: Vec<BoardItem> = items
            .iter()
            .map(|item| BoardItem {
                id: self.fresh_id("it"),
                ..item.clone()
            })
            .collect();
        let id = self.fresh_id("bd");
        let copy = Board {
            id: id.clone(),
            name: format!("{} copy", board.name),
            description: board.description.clone(),
            items,
            filters: filters.to_vec(),
            updated_at: self.now_iso(),
        };
        let toast = Toast::success(format!("{} made", copy.name));
        create!(
            self,
            boards,
            copy,
            vec![Then::Toast(toast, None), Then::Go(format!("/boards/{id}"))],
        );
    }

    /// A board's filters kept with it.
    pub fn set_board_filters(&mut self, id: &str, filters: &[Filter]) {
        let filters = filters.to_vec();
        update!(self, boards, id, |row| row.filters = filters, Vec::new());
    }

    /// A board's items kept: an edit done, or a chart taken off or put back.
    pub fn save_board_items(&mut self, id: &str, items: &[BoardItem]) {
        self.save_board_items_then(id, items, Vec::new());
    }

    fn save_board_items_then(&mut self, id: &str, items: &[BoardItem], then: Vec<Then>) {
        let (items, at) = (items.to_vec(), self.now_iso());
        update!(
            self,
            boards,
            id,
            |row| {
                row.items = items;
                row.updated_at = at;
            },
            then,
        );
    }

    /// A chart taken off a board outside Edit: kept at once, with an Undo that puts the items back
    /// as the page showed them.
    pub fn take_off(&mut self, board: &Board, items: &[BoardItem], item: &str, chart: Option<&str>) {
        let next: Vec<BoardItem> = items.iter().filter(|one| one.id != item).cloned().collect();
        let toast =
            Toast::success(format!("{} taken off {}", chart.unwrap_or("The chart"), board.name)).with_action("Undo");
        let undo = ToastAction::Restore {
            board: board.id.clone(),
            items: items.to_vec(),
        };
        self.save_board_items_then(&board.id, &next, vec![Then::Toast(toast, Some(undo))]);
    }

    /// Presses a toast's button.
    pub fn press(&mut self, action: &ToastAction) {
        match action {
            ToastAction::Restore { board, items } => self.save_board_items(board, items),
            ToastAction::Open(path) => self.go(path.clone()),
        }
    }

    /// A chart's figures handed out as CSV, under a file name made from its title.
    pub fn download_csv(&mut self, file: &str, names: &[String], rows: &[Vec<Cell>]) {
        self.download(format!("{file}.csv"), to_csv(names, rows));
    }

    // --- Preferences and the demo's data ---------------------------------------------------------

    /// Changes the preferences.
    pub fn set_prefs(&mut self, prefs: &str, patch: PrefsPatch) {
        update!(self, preferences, prefs, |row| patch.apply(row), Vec::new());
    }

    /// Puts a settings section back as it first was: `charts` or `numbers`.
    pub fn reset_section(&mut self, prefs: &str, section: &str) {
        let start = Preferences::default();
        let patch = match section {
            "charts" => PrefsPatch {
                palette: Some(start.palette),
                aggregate: Some(start.aggregate),
                suggestions: Some(start.suggestions),
                ..PrefsPatch::default()
            },
            "numbers" => PrefsPatch {
                numbers: Some(start.numbers),
                decimals: Some(start.decimals),
                ..PrefsPatch::default()
            },
            _ => return,
        };
        self.set_prefs(prefs, patch);
    }

    /// Puts the demo data back as it started, keeps nothing written after, and loads the page
    /// again.
    pub fn reset_all(&mut self) {
        self.datasets.rows = seed_datasets();
        self.charts.rows = seed_charts();
        self.boards.rows = seed_boards();
        self.preferences.rows = vec![Preferences::default()];
        self.sealed = true;
        self.clock.after(
            LATENCY,
            Job::Answer {
                failure: None,
                then: vec![Then::Reload],
            },
        );
        self.pending += 1;
    }

    /// The rows kept on an earlier visit in place of the seed's, listed as the seed's would have
    /// been: eframe's storage keeps what the browser's would.
    pub fn restore(
        &mut self,
        datasets: Vec<Dataset>,
        charts: Vec<SavedChart>,
        boards: Vec<Board>,
        preferences: Vec<Preferences>,
    ) {
        fn put<T: Row>(collection: &mut Collection<T>, rows: Vec<T>) {
            for (_, listed) in &mut collection.pending {
                *listed = rows.clone();
            }
            collection.rows = rows;
        }
        put(&mut self.datasets, datasets);
        put(&mut self.charts, charts);
        put(&mut self.boards, boards);
        put(&mut self.preferences, preferences);
    }

    // --- A shared chart --------------------------------------------------------------------------

    /// Keeps a chart opened from a link, and its data, here; opens it once both are kept.
    pub fn keep_shared(&mut self, shared: &SharedChart) {
        let id = self.fresh_id("ds");
        let mut dataset = shared_dataset(shared, &id, &self.now_iso());
        dataset.name = shared.dataset.name.clone();
        create!(
            self,
            datasets,
            dataset,
            vec![Then::KeepChart {
                name: shared.name.clone(),
                dataset: id,
                spec: shared.spec.clone(),
            }],
        );
    }
}
