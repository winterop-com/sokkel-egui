//! Tidevann's workspace, simulated: jobs, their schedules and forty runs, the same every time.

use sokkel_ui_egui::{Cell, ChipTone};

pub const JOBS: [(&str, &str, &str, &str); 5] = [
    ("nightly-export", "harbor", "ada", "every day at 02:00"),
    ("sync-inventory", "juniper", "grace", "every hour"),
    ("rebuild-index", "larkspur", "linus", "on demand"),
    ("send-digest", "harbor", "margaret", "every day at 07:00"),
    ("import-orders", "juniper", "ada", "every 15 minutes"),
];

pub const STATES: [(&str, ChipTone); 6] = [
    ("succeeded", ChipTone::Good),
    ("failed", ChipTone::Critical),
    ("running", ChipTone::Info),
    ("queued", ChipTone::Neutral),
    ("cancelled", ChipTone::Warning),
    ("skipped", ChipTone::Outline),
];

/// A run: its id, its job, its state, who started it, how long it took, the rows it moved.
pub struct Run {
    pub id: String,
    pub job: &'static str,
    pub state: usize,
    pub by: &'static str,
    pub took: String,
    pub rows: u32,
}

pub fn runs() -> Vec<Run> {
    (0..40)
        .map(|i: usize| Run {
            id: format!("run_{:04}", 4120 - i),
            job: JOBS[(i * 3) % JOBS.len()].0,
            state: (i * 7 + i / 3) % STATES.len(),
            by: JOBS[i % JOBS.len()].2,
            took: format!("{}m {:02}s", (i * 13) % 9 + 1, (i * 37) % 60),
            rows: ((i * 7919) % 20000) as u32 + 120,
        })
        .collect()
}

pub fn run_cells(runs: &[Run]) -> Vec<Vec<Cell>> {
    runs.iter()
        .map(|run| {
            let (state, tone) = STATES[run.state];
            vec![
                Cell::Mono(run.id.clone()),
                Cell::Text(run.job.into()),
                Cell::Chip(state.into(), tone),
                Cell::Text(run.by.into()),
                Cell::Text(run.took.clone()),
                Cell::Text(run.rows.to_string()),
            ]
        })
        .collect()
}

/// Runs per day over the last fortnight, succeeded and failed.
pub fn per_day() -> (Vec<f64>, Vec<f64>) {
    let succeeded = (0..14)
        .map(|d| 80.0 + ((d * 37) % 23) as f64 + d as f64 * 2.0)
        .collect();
    let failed = (0..14).map(|d| 3.0 + ((d * 11) % 7) as f64).collect();
    (succeeded, failed)
}
