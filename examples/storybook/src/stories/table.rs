use eframe::egui::Ui;
use sokkel_ui_egui::{Cell, ChipTone, Column, DataTable, TableState, TableVariant};

use super::{State, section};

const JOBS: [&str; 5] = [
    "nightly-export",
    "sync-inventory",
    "rebuild-index",
    "send-digest",
    "import-orders",
];
const STATES: [(&str, ChipTone); 6] = [
    ("succeeded", ChipTone::Good),
    ("failed", ChipTone::Critical),
    ("running", ChipTone::Info),
    ("queued", ChipTone::Neutral),
    ("cancelled", ChipTone::Warning),
    ("skipped", ChipTone::Outline),
];
const PEOPLE: [&str; 4] = ["ada", "grace", "linus", "margaret"];

/// Forty runs, the same every time.
fn runs() -> Vec<Vec<Cell>> {
    (0..40)
        .map(|i: usize| {
            let (state, tone) = STATES[(i * 7 + i / 3) % STATES.len()];
            vec![
                Cell::Mono(format!("run_{:04}", 4120 - i)),
                Cell::Text(JOBS[(i * 3) % JOBS.len()].into()),
                Cell::Chip(state.into(), tone),
                Cell::Text(PEOPLE[i % PEOPLE.len()].into()),
                Cell::Text(format!("{}m {:02}s", (i * 13) % 9 + 1, (i * 37) % 60)),
                Cell::Text(format!("{}", ((i * 7919) % 20000) + 120)),
            ]
        })
        .collect()
}

fn columns() -> Vec<Column> {
    vec![
        Column::new("Run", 120.0),
        Column::new("Job", 180.0),
        Column::new("State", 130.0),
        Column::new("By", 110.0),
        Column::new("Took", 110.0).numeric(),
        Column::new("Rows", 120.0).numeric(),
    ]
}

pub fn data_table(state: &mut State, ui: &mut Ui) {
    let runs = runs();
    section(ui, "Runs, sorted and selectable");
    DataTable::new("runs", &columns(), &runs, &mut state.runs)
        .selectable(true)
        .height(320.0)
        .show(ui);
    section(ui, "A list");
    let jobs: Vec<Vec<Cell>> = JOBS
        .iter()
        .enumerate()
        .map(|(i, job)| {
            vec![
                Cell::Text((*job).into()),
                Cell::Text(
                    [
                        "every day at 02:00",
                        "every hour",
                        "on demand",
                        "every day at 07:00",
                        "every 15 minutes",
                    ][i]
                        .into(),
                ),
            ]
        })
        .collect();
    DataTable::new(
        "jobs",
        &[Column::new("Job", 220.0), Column::new("Schedule", 200.0)],
        &jobs,
        &mut state.jobs,
    )
    .variant(TableVariant::List)
    .show(ui);
    section(ui, "Empty");
    let mut empty = TableState::default();
    DataTable::new("empty", &columns(), &[], &mut empty).show(ui);
}
