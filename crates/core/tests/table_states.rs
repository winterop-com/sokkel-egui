//! A table's own failed read: said in place of the rows with Try again, which the app hears; while
//! it reads again the placeholder rows stand over the failure.

use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{Appearance, Cell, Column, DataTable, ReadOutcome, TableState, install};

struct Failed {
    loading: bool,
    retried: usize,
    state: TableState,
}

fn failed(status: ReadOutcome) -> Harness<'static, Failed> {
    let mut harness = Harness::builder().with_size([520.0, 320.0]).build_ui_state(
        move |ui, failed: &mut Failed| {
            install(ui.ctx(), &Appearance::default());
            let columns = [Column::new("Run", 160.0), Column::new("Job", 200.0)];
            let rows: [Vec<Cell>; 0] = [];
            let out = DataTable::new("runs", &columns, &rows, &mut failed.state)
                .loading(failed.loading)
                .failed("The scheduler for harbor is restarting.", status)
                .retry(true)
                .show(ui);
            if out.retry {
                failed.retried += 1;
                failed.loading = true;
            }
        },
        Failed {
            loading: false,
            retried: 0,
            state: TableState::default(),
        },
    );
    harness.run_steps(3);
    harness
}

#[test]
fn try_again_is_heard_and_the_rows_load_over_the_failure() {
    let mut harness = failed(ReadOutcome::Status(503));
    harness.get_by_label("The server refused this read");
    harness.get_by_label("Try again").click();
    harness.run_steps(2);
    assert_eq!(harness.state().retried, 1);
    // reading again: the failure gives way to the placeholder rows
    assert!(harness.query_by_label("The server refused this read").is_none());
    assert!(harness.query_by_label("Try again").is_none());
}

#[test]
fn the_heading_says_what_the_server_answered() {
    let harness = failed(ReadOutcome::Status(404));
    harness.get_by_label("Nothing is there");
    let harness = failed(ReadOutcome::Unreachable);
    harness.get_by_label("The server is not answering");
}
