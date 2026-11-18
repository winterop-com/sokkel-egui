//! What the fake server keeps, as the web's `collections.ts` keeps it in the shared store
//! (`examples/shared/backend/store.ts`): the projects with their edits, and the person's settings;
//! and what a person does to the projects themselves, as `actions.ts` does it: make, rename,
//! duplicate, delete, and keep an edit as the editor saves it.
//!
//! Every call is at once and takes the moment it is made at. Rows are kept as the store keeps them:
//! a new one first, a changed one in its place, and a new row's id `projects-<now in base 36>-
//! <count>`, the count running over every row the store makes. What the web says in a toast comes
//! back as a `Notice`.

use crate::js::base36;
use crate::model::{ClipIds, Edit, Prefs, Project, copy_edit, default_tracks, length_of, length_text};
use crate::seed::seed_projects;

/// How the web's toast looks: its kind, which sets its color and icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Plain,
    Success,
    Warning,
    Error,
}

/// What a change says to the person as a toast: its title, the line under it, and the button on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub title: String,
    pub description: Option<String>,
    /// The words on its button, which opens what was made.
    pub action: Option<String>,
}

impl Notice {
    fn new(kind: NoticeKind, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            description: None,
            action: None,
        }
    }

    fn failed(title: &str, reason: String) -> Self {
        Self {
            description: Some(reason),
            ..Self::new(NoticeKind::Error, title)
        }
    }
}

/// The projects and the settings on the simulated backend.
#[derive(Clone, Debug)]
pub struct Backend {
    /// The projects as the store keeps them: the newest made first.
    pub projects: Vec<Project>,
    pub prefs: Prefs,
    /// How many rows the store has made, for the next one's id.
    made: u64,
}

/// The projects, the one edited last first.
pub fn by_updated(all: &[Project]) -> Vec<&Project> {
    let mut sorted: Vec<&Project> = all.iter().collect();
    sorted.sort_by_key(|project| std::cmp::Reverse(project.updated));
    sorted
}

impl Backend {
    /// The projects as a new visitor starts with them, made at `now`.
    pub fn seeded(now: i64) -> Self {
        Self::kept(seed_projects(now), Prefs::default())
    }

    /// The projects and settings as they were kept.
    pub fn kept(projects: Vec<Project>, prefs: Prefs) -> Self {
        Self {
            projects,
            prefs,
            made: 0,
        }
    }

    pub fn project(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|project| project.id == id)
    }

    /// The projects, the one edited last first.
    pub fn by_updated(&self) -> Vec<&Project> {
        by_updated(&self.projects)
    }

    /// Adds a project as the store does: first, under an id it makes.
    fn create(&mut self, mut project: Project, now: i64) -> &Project {
        self.made += 1;
        project.id = format!("projects-{}-{}", base36(now), self.made);
        self.projects.insert(0, project);
        &self.projects[0]
    }

    /// Changes the project under `id`, or says there is none.
    fn update(&mut self, id: &str, change: impl FnOnce(&mut Project)) -> Result<&Project, String> {
        let Some(project) = self.projects.iter_mut().find(|project| project.id == id) else {
            return Err(format!("No projects with the id {id}."));
        };
        change(project);
        Ok(project)
    }

    /// Makes an empty project; its id, which the editor opens, and what it says.
    pub fn create_project(&mut self, name: &str, now: i64) -> (String, Notice) {
        let made = self.create(
            Project {
                id: String::new(),
                name: name.to_owned(),
                owner: "ada".to_owned(),
                created: now,
                updated: now,
                edit: Edit {
                    tracks: default_tracks(),
                    clips: Vec::new(),
                },
            },
            now,
        );
        (
            made.id.clone(),
            Notice::new(NoticeKind::Success, format!("{name} made")),
        )
    }

    /// Renames a project; what it says when it cannot.
    pub fn rename_project(&mut self, id: &str, name: &str, now: i64) -> Result<(), Notice> {
        self.update(id, |project| {
            project.name = name.to_owned();
            project.updated = now;
        })
        .map(|_| ())
        .map_err(|reason| Notice::failed("Could not rename the project", reason))
    }

    /// A copy of a project with every clip under new ids, listed first as the one edited last; its
    /// id and what it says, with a button that opens it.
    pub fn duplicate_project(&mut self, id: &str, now: i64, ids: &mut ClipIds) -> Result<(String, Notice), Notice> {
        let Some(project) = self.project(id) else {
            return Err(Notice::failed(
                "Could not duplicate the project",
                format!("No projects with the id {id}."),
            ));
        };
        let copy = Project {
            id: String::new(),
            name: format!("{} copy", project.name),
            owner: "ada".to_owned(),
            created: now,
            updated: now,
            edit: copy_edit(&project.edit, ids),
        };
        let made = self.create(copy, now);
        let notice = Notice {
            action: Some("Open".to_owned()),
            ..Notice::new(NoticeKind::Success, format!("{} made", made.name))
        };
        Ok((made.id.clone(), notice))
    }

    /// Deletes a project, and says so.
    pub fn delete_project(&mut self, id: &str) -> Notice {
        let name = self.project(id).map(|project| project.name.clone());
        self.projects.retain(|project| project.id != id);
        Notice::new(NoticeKind::Plain, format!("{} deleted", name.unwrap_or_default()))
    }

    /// Keeps an edit's tracks and clips, as the editor saves them while a person works.
    pub fn save_edit(&mut self, id: &str, edit: &Edit, now: i64) -> Result<(), Notice> {
        self.update(id, |project| {
            project.edit = edit.clone();
            project.updated = now;
        })
        .map(|_| ())
        .map_err(|reason| Notice::failed("Could not save the edit", reason))
    }

    /// Changes the settings.
    pub fn update_prefs(&mut self, change: impl FnOnce(&mut Prefs)) {
        change(&mut self.prefs);
    }
}

/// How long the editor waits after the last change before it saves the edit, in milliseconds.
pub const SAVE_AFTER: i64 = 600;

/// How many projects the rail lists under Recent.
pub const RECENT: usize = 5;

/// Why a name cannot be a project's, if it cannot: none, or one another project has. `renaming` is
/// the name of the project being renamed, which may keep it.
pub fn name_problem(name: &str, taken: &[&str], renaming: Option<&str>) -> Option<&'static str> {
    let trimmed = crate::js::trim(name);
    let clash = taken
        .iter()
        .any(|one| one.to_lowercase() == trimmed.to_lowercase() && Some(*one) != renaming);
    if trimmed.is_empty() {
        Some("A project needs a name.")
    } else if clash {
        Some("Another project has this name.")
    } else {
        None
    }
}

/// The question the delete dialog asks.
pub fn delete_question(project: &Project) -> String {
    format!("Delete {}?", project.name)
}

/// The line under it.
pub fn delete_detail(project: &Project) -> String {
    format!(
        "Its {} clips go with it; the media in the bin stays. This cannot be undone.",
        project.edit.clips.len()
    )
}

/// A count with its noun, `1 project` or `3 projects`.
pub fn counted(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// What a project's card says under its name, before when it was edited: `7 clips`.
pub fn clips_count(project: &Project) -> String {
    counted(project.edit.clips.len(), "clip", "clips")
}

/// The badge on a project's card: its length, `0:26`.
pub fn project_length(project: &Project) -> String {
    length_text(length_of(&project.edit))
}
