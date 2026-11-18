//! The pieces several pages draw a run with, as the web's `parts.tsx`: its time, its columns, its
//! menu and its steps.

use sokkel_egui::{Cell, ChipTone, Column, Icon, MenuItem, TimelineEvent, relative_time_short};

use crate::app::LiveRun;
use crate::model::{Status, Times, say_by, stamp, took};

/// The columns' ids, for the saved views.
pub const COLUMN_IDS: [&str; 8] = ["id", "job", "project", "state", "step", "by", "started", "duration"];

/// A state as its chip, in the state's colors.
pub fn state_cell(state: Status) -> Cell {
    Cell::Chip(state.word().into(), ChipTone::Status(state.word()))
}

/// A moment as the person prefers to read it: how long ago, or on the UTC clock.
pub fn when(at: i64, now: i64, times: Times) -> String {
    match times {
        Times::Clock => stamp(at),
        Times::Relative => relative_time_short((at - now) as f64),
    }
}

/// The columns of a listing of runs; without `job`, the job's own columns are left out, on its page.
pub fn run_columns(job: bool) -> Vec<Column> {
    // the web's widths (8.5rem, 10rem and up, 6.5rem, 7.5rem, 8rem, 7.5rem, 7.5rem, 6rem) less the
    // 12 between columns, which the web's cells hold inside
    let mut columns = vec![Column::new("Run", 124.0).sortable(true)];
    if job {
        columns.push(Column::new("Job", 148.0).fill().sortable(true));
        columns.push(Column::new("Project", 92.0).sortable(true));
    }
    columns.extend([
        Column::new("State", 108.0).sortable(true),
        Column::new("Step", 116.0).sortable(true),
        Column::new("Started by", 108.0).sortable(true),
        Column::new("Started", 108.0).sortable(true),
        Column::new("Duration", 84.0).numeric().sortable(true),
    ]);
    columns
}

/// A run's cells, in the columns' order.
pub fn run_cells(run: &LiveRun, now: i64, times: Times, job: bool) -> Vec<Cell> {
    let mut cells = vec![Cell::Mono(run.run.id.clone())];
    if job {
        cells.push(Cell::Mono(run.run.job.clone()));
        cells.push(Cell::Text(run.run.project.word().into()));
    }
    cells.extend([
        state_cell(run.at.state),
        Cell::MutedCode(run.at.step.clone().unwrap_or_else(|| "-".into())),
        Cell::Muted(say_by(run.run.by, run.run.trigger)),
        Cell::Number {
            value: run.run.created as f64,
            text: when(run.run.created, now, times),
        },
        Cell::Number {
            value: run.at.duration as f64,
            text: if run.at.state == Status::Skipped {
                "-".into()
            } else {
                took(run.at.duration as f64)
            },
        },
    ]);
    cells
}

/// What a run's menu offers, from its row or its page: Open, Open page (with `page`), Retry, Copy
/// run ID, and Cancel run.
pub fn run_menu(live: bool, page: bool) -> Vec<MenuItem> {
    let mut items = vec![MenuItem::action("Open").icon(Icon::PanelRightOpen)];
    if page {
        items.push(MenuItem::action("Open page").icon(Icon::ExternalLink));
    }
    items.extend([
        MenuItem::action("Retry").icon(Icon::RotateCcw).disabled(live),
        MenuItem::action("Copy run ID").icon(Icon::Copy),
        MenuItem::Separator,
        MenuItem::action("Cancel run")
            .icon(Icon::X)
            .destructive()
            .disabled(!live),
    ]);
    items
}

/// A run's steps as a timeline: each step's state, how long it took, and when it started.
pub fn step_events(t: &sokkel_egui::Tokens, run: &LiveRun) -> Vec<TimelineEvent> {
    run.at
        .steps
        .iter()
        .map(|step| TimelineEvent {
            title: step.code.clone(),
            detail: Some(match step.status {
                Status::Failed => run.run.error.clone().unwrap_or_default(),
                Status::Running => format!("{}% done", crate::model::js_round(step.progress.unwrap_or(0.0) * 100.0)),
                _ => step.kind.clone(),
            }),
            at: step
                .duration_ms
                .map_or_else(|| step.status.word().to_owned(), |ms| took(ms as f64)),
            tone: Some(t.color(&format!("status-{}", step.status.word()))),
            ..Default::default()
        })
        .collect()
}
