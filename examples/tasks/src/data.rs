//! The tasks example's data, ported from sokkel-ui's `examples/tasks/web/src/data.ts`: the people,
//! the projects, the states, the seed and the preferences, kept by the app as the web example's
//! simulated backend keeps them.

use serde::{Deserialize, Serialize};
use sokkel_egui::Day;

pub const PEOPLE: [&str; 4] = ["ada", "grace", "linus", "margaret"];
pub const PROJECTS: [&str; 3] = ["harbor", "juniper", "larkspur"];

/// Where a task stands: the board's columns, in their order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Todo,
    Doing,
    Done,
}

pub const STATES: [TaskState; 3] = [TaskState::Todo, TaskState::Doing, TaskState::Done];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    /// The day it is due, as days since 1970-01-01, or none.
    pub due: Option<i32>,
    pub assignee: String,
    pub project: String,
    pub state: TaskState,
    /// Its place in the person's own order, lowest first; a list in that order and each board
    /// column follow it.
    pub rank: f64,
}

impl Task {
    pub fn due(&self) -> Option<Day> {
        self.due.map(Day)
    }

    pub fn done(&self) -> bool {
        self.state == TaskState::Done
    }
}

const SEED: [(&str, Option<i32>, &str, &str, TaskState); 14] = [
    (
        "Rerun nightly-export for last week",
        Some(-1),
        "ada",
        "harbor",
        TaskState::Todo,
    ),
    (
        "Review the failed sync-inventory runs",
        Some(0),
        "grace",
        "harbor",
        TaskState::Doing,
    ),
    (
        "Add a validate step to import-orders",
        Some(0),
        "linus",
        "juniper",
        TaskState::Todo,
    ),
    (
        "Draft the wording of send-digest",
        Some(0),
        "margaret",
        "larkspur",
        TaskState::Todo,
    ),
    (
        "Speed up rebuild-index on the worker",
        Some(2),
        "linus",
        "juniper",
        TaskState::Doing,
    ),
    (
        "Move notify to the new scheduler",
        Some(3),
        "ada",
        "larkspur",
        TaskState::Todo,
    ),
    (
        "Write the runbook for the api service",
        Some(5),
        "grace",
        "harbor",
        TaskState::Todo,
    ),
    (
        "Retry import-orders from the upload step",
        Some(1),
        "margaret",
        "juniper",
        TaskState::Todo,
    ),
    (
        "Archive the cancelled runs from last month",
        Some(9),
        "ada",
        "harbor",
        TaskState::Todo,
    ),
    (
        "Pick a time for the weekly send-digest",
        None,
        "margaret",
        "larkspur",
        TaskState::Todo,
    ),
    (
        "Split transform into two steps",
        None,
        "linus",
        "juniper",
        TaskState::Todo,
    ),
    (
        "Clean up skipped runs in larkspur",
        Some(-3),
        "grace",
        "larkspur",
        TaskState::Done,
    ),
    (
        "Set up alerts for failed uploads",
        Some(-2),
        "ada",
        "harbor",
        TaskState::Done,
    ),
    (
        "Rename the fetch step in sync-inventory",
        Some(-1),
        "linus",
        "juniper",
        TaskState::Done,
    ),
];

/// The seed, its days counted from `today`.
pub fn seed(today: Day) -> Vec<Task> {
    SEED.iter()
        .enumerate()
        .map(|(index, (title, offset, assignee, project, state))| Task {
            id: format!("task-{}", index + 1),
            title: (*title).into(),
            due: offset.map(|days| today.plus(days).0),
            assignee: (*assignee).into(),
            project: (*project).into(),
            state: *state,
            rank: (index + 1) as f64,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    List,
    Board,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Order {
    Due,
    Mine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeekStart {
    Monday,
    Sunday,
}

/// What the person chose in Settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prefs {
    /// The list the app opens on.
    pub start_list: String,
    /// Whether lists show the tasks already done.
    pub show_done: bool,
    pub view: View,
    pub order: Order,
    pub week_start: WeekStart,
    /// Who a task added from the quick add is given to.
    pub assignee: String,
    /// The project a task added outside a project's list goes into.
    pub project: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            start_list: "inbox".into(),
            show_done: false,
            view: View::List,
            order: Order::Mine,
            week_start: WeekStart::Monday,
            assignee: "ada".into(),
            project: "harbor".into(),
        }
    }
}
