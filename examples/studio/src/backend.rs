//! The workspace on the simulated backend, as the web's `backend.ts` keeps it in the shared
//! collection store and `state.tsx` changes it: the files and the documents they are edited in, the
//! runs started here, the values tried against schemas, where the boxes of each pipeline's graph
//! were moved to, and the reader's settings with their tabs.
//!
//! Every call is at once and takes the moment it is made at. Rows are kept as the store keeps them:
//! a new one first, a changed one in its place, and a new row's id `<collection>-<now in base 36>-
//! <count>`, the count running over every collection. A run moves on as `tick` comes to the moments
//! its timers would fire: its first step `LATENCY` after it is made, as the web's first step waits
//! for the store's answer.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::js::base36;
use crate::model::{FileKind, Prefs, Project, Run, RunState, WorkFile, file_order, kind_of};
use crate::seed::{SAMPLES, seed_files, template};
use crate::simulate::{Due, Going, LATENCY, cancelled, start_run};

/// Where the latest save stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveState {
    Idle,
    Saving,
    Saved { at: i64 },
    Failed { reason: String },
}

/// A question the studio is asking in a dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    New {
        project: Project,
        kind: FileKind,
    },
    Rename {
        id: String,
    },
    Delete {
        id: String,
    },
    /// Tabs with changes not saved, about to close.
    Close {
        ids: Vec<String>,
    },
}

/// What a change says to the reader as a toast: its title and the line under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub description: String,
}

/// Where a tab leads: a file's page, or a table's rows.
pub fn tab_href(id: &str) -> String {
    match id.strip_prefix("table:") {
        Some(table) => format!("/table/{table}"),
        None => format!("/file/{id}"),
    }
}

/// Where the boxes of a pipeline's graph were moved to, by step.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// The pipeline file's id.
    pub id: String,
    pub positions: BTreeMap<String, (f64, f64)>,
}

/// What `tick` has yet to do: a run's first step after the store answers, or a step's timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wait {
    Begin,
    Step(Due),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Timer {
    at: i64,
    /// The order timers were set in, which settles two due at once.
    seq: u64,
    run: String,
    wait: Wait,
}

/// The studio's data and the documents open on it.
#[derive(Clone, Debug)]
pub struct Backend {
    /// The files as saved, as the store keeps them.
    pub files: Vec<WorkFile>,
    /// The runs as the store keeps them: the newest made first.
    pub runs: Vec<Run>,
    /// The value tried against each schema, by the schema file's id.
    pub samples: Vec<(String, String)>,
    pub layouts: Vec<Layout>,
    /// The reader's settings; the web keeps them as a collection of one row.
    pub prefs: Prefs,
    /// Where the latest save stands.
    pub saving: SaveState,
    /// The question a dialog is asking.
    pub asking: Option<Asking>,
    /// The documents files are edited in, by file id, made from the saved text the first time and
    /// kept until the file's tab closes.
    docs: Vec<(String, String)>,
    /// How many rows were made, for the next one's id.
    made: u64,
    seq: u64,
    timers: Vec<Timer>,
    going: HashMap<String, Going>,
    /// The runs started since the backend was made; any other run still going was left so.
    begun: HashSet<String>,
}

impl Backend {
    /// The workspace as a new reader starts with it.
    pub fn seeded() -> Self {
        Self::kept(
            seed_files(),
            Vec::new(),
            SAMPLES
                .iter()
                .map(|(id, text)| ((*id).to_owned(), (*text).to_owned()))
                .collect(),
            Prefs::default(),
        )
    }

    /// The workspace as it was kept.
    pub fn kept(files: Vec<WorkFile>, runs: Vec<Run>, samples: Vec<(String, String)>, prefs: Prefs) -> Self {
        Self {
            files,
            runs,
            samples,
            layouts: Vec::new(),
            prefs,
            saving: SaveState::Idle,
            asking: None,
            docs: Vec::new(),
            made: 0,
            seq: 0,
            timers: Vec::new(),
            going: HashMap::new(),
            begun: HashSet::new(),
        }
    }

    /// A new row's id in a collection.
    fn new_id(&mut self, collection: &str, now: i64) -> String {
        self.made += 1;
        format!("{collection}-{}-{}", base36(now), self.made)
    }

    fn set_timer(&mut self, at: i64, run: &str, wait: Wait) {
        self.seq += 1;
        self.timers.push(Timer {
            at,
            seq: self.seq,
            run: run.to_owned(),
            wait,
        });
    }

    /// A saved file.
    pub fn file(&self, id: &str) -> Option<&WorkFile> {
        self.files.iter().find(|file| file.id == id)
    }

    /// The runs, the latest first by number, as the studio lists them.
    pub fn runs_newest(&self) -> Vec<&Run> {
        let mut runs: Vec<&Run> = self.runs.iter().collect();
        runs.sort_by_key(|run| std::cmp::Reverse(run.number));
        runs
    }

    /// The sample tried against a schema.
    pub fn sample(&self, id: &str) -> Option<&str> {
        self.samples
            .iter()
            .find(|(one, _)| one == id)
            .map(|(_, text)| text.as_str())
    }

    /// Keeps the sample tried against a schema.
    pub fn set_sample(&mut self, id: &str, text: &str) {
        match self.samples.iter_mut().find(|(one, _)| one == id) {
            Some((_, kept)) => *kept = text.to_owned(),
            None => self.samples.insert(0, (id.to_owned(), text.to_owned())),
        }
    }

    /// Keeps where a pipeline's boxes were moved to.
    pub fn set_layout(&mut self, id: &str, positions: BTreeMap<String, (f64, f64)>) {
        match self.layouts.iter_mut().find(|one| one.id == id) {
            Some(layout) => layout.positions = positions,
            None => self.layouts.insert(
                0,
                Layout {
                    id: id.to_owned(),
                    positions,
                },
            ),
        }
    }

    /// Changes the settings.
    pub fn set_prefs(&mut self, change: impl FnOnce(&mut Prefs)) {
        change(&mut self.prefs);
    }

    /// The text of a file's document: made from its saved text the first time.
    pub fn doc_for(&mut self, id: &str) -> Option<&str> {
        if !self.docs.iter().any(|(one, _)| one == id) {
            let text = self.file(id)?.text.clone();
            self.docs.push((id.to_owned(), text));
        }
        self.docs
            .iter()
            .find(|(one, _)| one == id)
            .map(|(_, text)| text.as_str())
    }

    /// The text of a file's document if one is open.
    pub fn doc(&self, id: &str) -> Option<&str> {
        self.docs
            .iter()
            .find(|(one, _)| one == id)
            .map(|(_, text)| text.as_str())
    }

    /// Types into a file's document.
    pub fn edit(&mut self, id: &str, text: &str) {
        if self.doc_for(id).is_none() {
            return;
        }
        if let Some((_, kept)) = self.docs.iter_mut().find(|(one, _)| one == id) {
            *kept = text.to_owned();
        }
    }

    /// The files whose document holds changes not saved.
    pub fn dirty(&self) -> HashSet<String> {
        self.docs
            .iter()
            .filter(|(id, text)| self.file(id).is_some_and(|file| file.text != *text))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Saves a file's document.
    pub fn save(&mut self, id: &str, now: i64) {
        let Some(text) = self.doc(id).map(str::to_owned) else {
            return;
        };
        if let Some(file) = self.files.iter_mut().find(|file| file.id == id) {
            file.text = text;
            file.updated = now;
            // The web says it saved once the store answers.
            self.saving = SaveState::Saved { at: now + LATENCY };
        } else {
            let reason = format!("No files with the id {id}.");
            self.saving = SaveState::Failed { reason };
        }
    }

    /// Saves every document holding changes.
    pub fn save_all(&mut self, now: i64) {
        let dirty = self.dirty();
        let ids: Vec<String> = self
            .docs
            .iter()
            .map(|(id, _)| id.clone())
            .filter(|id| dirty.contains(id))
            .collect();
        for id in ids {
            self.save(&id, now);
        }
    }

    /// Puts a file's document back to what was saved.
    pub fn revert(&mut self, id: &str) {
        let Some(saved) = self.file(id).map(|file| file.text.clone()) else {
            return;
        };
        if let Some((_, kept)) = self.docs.iter_mut().find(|(one, _)| one == id) {
            *kept = saved;
        }
    }

    /// Makes a tab the shown one, adding it to the strip, as its page opens.
    pub fn show(&mut self, id: &str) {
        if self.prefs.active.as_deref() == Some(id) && self.prefs.tabs.iter().any(|tab| tab == id) {
            return;
        }
        self.prefs.active = Some(id.to_owned());
        if !self.prefs.tabs.iter().any(|tab| tab == id) {
            self.prefs.tabs.push(id.to_owned());
        }
    }

    /// Closes tabs, asking first about those with changes not saved. Answers with where to go when
    /// the shown tab closed.
    pub fn close(&mut self, ids: &[String]) -> Option<String> {
        let dirty = self.dirty();
        if ids.iter().any(|id| dirty.contains(id)) {
            self.asking = Some(Asking::Close { ids: ids.to_vec() });
            return None;
        }
        self.close_now(ids)
    }

    /// Closes tabs as they are, dropping their changes. Answers with where to go when the shown tab
    /// closed: the tab in its place, or `/` when none is left.
    pub fn close_now(&mut self, ids: &[String]) -> Option<String> {
        let tabs: Vec<String> = self
            .prefs
            .tabs
            .iter()
            .filter(|tab| !ids.contains(tab))
            .cloned()
            .collect();
        self.docs.retain(|(id, _)| !ids.contains(id));
        match self.prefs.active.clone().filter(|active| ids.contains(active)) {
            Some(active) => {
                let at = self.prefs.tabs.iter().position(|tab| *tab == active);
                let next = match at {
                    Some(at) if !tabs.is_empty() => tabs.get(at.min(tabs.len() - 1)).cloned(),
                    _ => None,
                };
                self.prefs.tabs = tabs;
                self.prefs.active = next.clone();
                Some(next.map_or_else(|| "/".to_owned(), |next| tab_href(&next)))
            }
            None => {
                self.prefs.tabs = tabs;
                None
            }
        }
    }

    /// Makes a file of a kind in a project from its kind's template, named `name` and its
    /// extension.
    pub fn create_file(&mut self, project: Project, kind: FileKind, name: &str, now: i64) -> WorkFile {
        let file = WorkFile {
            id: self.new_id("files", now),
            project,
            kind,
            name: format!("{name}{}", kind_of(kind).extension),
            text: template(kind, name, project),
            updated: now,
            rank: None,
        };
        self.files.insert(0, file.clone());
        file
    }

    /// Renames a file, its extension kept.
    pub fn rename_file(&mut self, id: &str, name: &str) {
        if let Some(file) = self.files.iter_mut().find(|file| file.id == id) {
            file.name = format!("{name}{}", kind_of(file.kind).extension);
        }
    }

    /// Moves a file into a project, at `index` among the files of its kind there (counted without
    /// it), or at the end; every file there is ranked in the order that makes.
    pub fn move_file(&mut self, id: &str, project: Project, index: Option<usize>) {
        let Some(file) = self.file(id).cloned() else {
            return;
        };
        let mut group: Vec<WorkFile> = self
            .files
            .iter()
            .filter(|one| one.project == project && one.kind == file.kind && one.id != id)
            .cloned()
            .collect();
        group.sort_by(file_order);
        let at = index.unwrap_or(group.len()).min(group.len());
        group.insert(at, WorkFile { project, ..file });
        for (rank, one) in group.iter().enumerate() {
            let rank = rank as i64;
            if let Some(kept) = self.files.iter_mut().find(|kept| kept.id == one.id) {
                if one.id == id {
                    kept.project = project;
                    kept.rank = Some(rank);
                } else if one.rank != Some(rank) {
                    kept.rank = Some(rank);
                }
            }
        }
    }

    /// Deletes a file and closes its tab; answers with where to go when it was the shown one.
    pub fn delete_file(&mut self, id: &str) -> Option<String> {
        self.files.retain(|file| file.id != id);
        self.close_now(&[id.to_owned()])
    }

    /// Runs a pipeline from its document as it stands, and answers with the run as first stored;
    /// refused (none) when the file is not a pipeline, or with the toast the web shows.
    pub fn run(&mut self, id: &str, now: i64) -> Option<Result<Run, Notice>> {
        let file = self.file(id).filter(|file| file.kind == FileKind::Pipeline)?.clone();
        let text = self.doc_for(id)?.to_owned();
        let number = self.runs_newest().first().map_or(0, |run| run.number) + 1;
        Some(match start_run(&file, &text, number, text != file.text, now) {
            Ok((mut run, mut going)) => {
                run.id = self.new_id("runs", now);
                going.stored_as(&run.id);
                self.runs.insert(0, run.clone());
                self.going.insert(run.id.clone(), going);
                self.set_timer(now + LATENCY, &run.id, Wait::Begin);
                Ok(run)
            }
            Err(reason) => Err(Notice {
                title: "The run did not start".to_owned(),
                description: reason,
            }),
        })
    }

    /// Stops a run that is going at `now`: the step running and those ahead are cancelled. A run
    /// left going by a reload, whose steps nothing moves any more, is cancelled the same way.
    pub fn cancel(&mut self, id: &str, now: i64) {
        let Some(run) = self.runs.iter().find(|run| run.id == id) else {
            return;
        };
        let timer = self.going.get(id).is_some_and(|going| going.timer);
        // A run of this backend with no timer has just finished.
        if run.state != RunState::Running || (!timer && self.begun.contains(id)) {
            return;
        }
        self.timers
            .retain(|one| !(one.run == id && matches!(one.wait, Wait::Step(_))));
        if let Some(going) = self.going.get_mut(id) {
            going.timer = false;
        }
        let patch = cancelled(run, now);
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == id) {
            patch.apply(run);
        }
    }

    /// Whether a run's steps are still to move.
    pub fn moving(&self) -> bool {
        !self.timers.is_empty()
    }

    /// When the next timer fires, if one is set.
    pub fn next_due(&self) -> Option<i64> {
        self.timers.iter().map(|timer| timer.at).min()
    }

    /// Fires every timer due by `now`, each at its own moment, in the order they were set.
    pub fn tick(&mut self, now: i64) {
        while let Some(at) = self
            .timers
            .iter()
            .enumerate()
            .filter(|(_, timer)| timer.at <= now)
            .min_by_key(|(_, timer)| (timer.at, timer.seq))
            .map(|(at, _)| at)
        {
            let timer = self.timers.remove(at);
            let Some(mut going) = self.going.remove(&timer.run) else {
                continue;
            };
            let Some(stored) = self.runs.iter_mut().find(|run| run.id == timer.run) else {
                continue;
            };
            let due = match timer.wait {
                Wait::Begin => {
                    self.begun.insert(timer.run.clone());
                    going.next(0, timer.at, stored)
                }
                Wait::Step(due) => going.fire(due, timer.at, stored),
            };
            self.going.insert(timer.run.clone(), going);
            if let Some(due) = due {
                self.set_timer(due.at, &timer.run, Wait::Step(due));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StepState;

    const NOW: i64 = 1_794_992_400_000;

    #[test]
    fn files_are_made_named_moved_and_deleted_as_the_web_does() {
        let mut backend = Backend::seeded();
        let made = backend.create_file(Project::Juniper, FileKind::Query, "stock-alerts", NOW);
        assert_eq!(made.id, "files-mwlw4o00-1");
        assert_eq!(made.name, "stock-alerts.sql");
        assert_eq!(made.text, "-- stock-alerts\nSELECT *\nFROM runs\nLIMIT 20\n");
        assert_eq!(backend.files[0].id, made.id);
        backend.rename_file(&made.id, "low-stock");
        assert_eq!(
            backend.file(&made.id).map(|file| file.name.as_str()),
            Some("low-stock.sql")
        );

        backend.move_file("harbor-import-orders", Project::Juniper, Some(1));
        let ranked = |backend: &Backend, id: &str| backend.file(id).map(|file| (file.project, file.rank));
        assert_eq!(
            ranked(&backend, "juniper-rebuild-index"),
            Some((Project::Juniper, Some(0)))
        );
        assert_eq!(
            ranked(&backend, "harbor-import-orders"),
            Some((Project::Juniper, Some(1)))
        );
        assert_eq!(
            ranked(&backend, "juniper-sync-inventory"),
            Some((Project::Juniper, Some(2)))
        );
        backend.move_file("juniper-sync-inventory", Project::Juniper, None);
        assert_eq!(
            ranked(&backend, "juniper-sync-inventory"),
            Some((Project::Juniper, Some(2)))
        );

        backend.show("harbor-order-schema");
        assert_eq!(
            backend.delete_file("harbor-order-schema"),
            Some("/file/juniper-slow-steps".to_owned())
        );
        assert_eq!(backend.prefs.tabs, ["harbor-nightly-export", "juniper-slow-steps"]);
    }

    #[test]
    fn tabs_open_and_close_as_the_web_does() {
        let mut backend = Backend::seeded();
        backend.show("table:runs");
        assert_eq!(backend.prefs.active.as_deref(), Some("table:runs"));
        assert_eq!(backend.prefs.tabs.len(), 4);
        backend.edit("juniper-slow-steps", "SELECT 1 FROM jobs");
        assert!(backend.dirty().contains("juniper-slow-steps"));
        assert_eq!(backend.close(&["juniper-slow-steps".to_owned()]), None);
        assert_eq!(
            backend.asking,
            Some(Asking::Close {
                ids: vec!["juniper-slow-steps".to_owned()]
            })
        );
        assert_eq!(backend.close_now(&["juniper-slow-steps".to_owned()]), None);
        assert!(backend.dirty().is_empty());
        assert_eq!(
            backend.close_now(&["table:runs".to_owned()]),
            Some("/file/harbor-order-schema".to_owned())
        );
        let all = backend.prefs.tabs.clone();
        assert_eq!(backend.close_now(&all), Some("/".to_owned()));
        assert_eq!(backend.prefs.active, None);
    }

    #[test]
    fn a_document_saves_and_reverts() {
        let mut backend = Backend::seeded();
        backend.edit("larkspur-runs-by-owner", "SELECT * FROM jobs");
        backend.save_all(NOW);
        assert_eq!(backend.saving, SaveState::Saved { at: NOW + LATENCY });
        let file = backend.file("larkspur-runs-by-owner").expect("the file");
        assert_eq!((file.text.as_str(), file.updated), ("SELECT * FROM jobs", NOW));
        backend.edit("larkspur-runs-by-owner", "SELECT 2 FROM jobs");
        backend.revert("larkspur-runs-by-owner");
        assert_eq!(backend.doc("larkspur-runs-by-owner"), Some("SELECT * FROM jobs"));
    }

    #[test]
    fn a_run_moves_as_its_timers_fire() {
        let mut backend = Backend::seeded();
        let Some(Ok(run)) = backend.run("juniper-sync-inventory", NOW) else {
            panic!("the run starts");
        };
        assert_eq!((run.id.as_str(), run.number, run.draft), ("runs-mwlw4o00-1", 1, false));
        assert_eq!(backend.next_due(), Some(NOW + LATENCY));
        backend.tick(NOW + LATENCY);
        assert_eq!(backend.runs[0].steps[0].state, StepState::Running);
        backend.tick(NOW + 60_000);
        assert_eq!(backend.runs[0].state, RunState::Succeeded);
        assert!(!backend.moving());
        assert!(backend.run("juniper-slow-steps", NOW).is_none());
    }
}
