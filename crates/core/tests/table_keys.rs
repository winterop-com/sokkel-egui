//! A table sorted by keys from its head row: Up from the first row reaches the head, Enter sorts
//! the column the keys stand on, Right moves to the next, and Down goes back to the rows.

use egui::Key;
use egui_kittest::Harness;
use sokkel_egui::{Appearance, Cell, Column, DataTable, Editor, TableState, install};

struct Table {
    columns: Vec<Column>,
    rows: Vec<Vec<Cell>>,
    state: TableState,
}

fn table() -> Harness<'static, Table> {
    let table = Table {
        columns: vec![
            Column::new("Job", 160.0).editable(Editor::Text),
            Column::new("Owner", 120.0).editable(Editor::Text),
        ],
        rows: [
            ("sync-inventory", "grace"),
            ("nightly-export", "ada"),
            ("rebuild-index", "linus"),
        ]
        .iter()
        .map(|(job, owner)| vec![Cell::from(*job), Cell::from(*owner)])
        .collect(),
        state: TableState::default(),
    };
    let mut harness = Harness::builder().with_size([480.0, 300.0]).build_ui_state(
        |ui, table: &mut Table| {
            install(ui.ctx(), &Appearance::default());
            DataTable::new("jobs", &table.columns, &table.rows, &mut table.state).show(ui);
        },
        table,
    );
    harness.run_steps(3);
    harness
}

#[test]
fn up_from_the_first_row_reaches_the_head_where_enter_sorts() {
    let mut harness = table();
    // the first row's job cell made active: its editor opened, then left with Escape
    harness.state_mut().state.edit(0, 0);
    harness.run_steps(2);
    harness.key_press(Key::Escape);
    harness.run_steps(2);
    harness.key_press(Key::ArrowUp);
    harness.run_steps(1);
    harness.key_press(Key::Enter);
    harness.run_steps(1);
    assert_eq!(harness.state().state.sort, Some((0, true)));
    harness.key_press(Key::Enter);
    harness.run_steps(1);
    assert_eq!(harness.state().state.sort, Some((0, false)));
    // Right to the owner's head, which sorts in its turn
    harness.key_press(Key::ArrowRight);
    harness.run_steps(1);
    harness.key_press(Key::Enter);
    harness.run_steps(1);
    assert_eq!(harness.state().state.sort, Some((1, true)));
    // Down back to the rows: the first row shown, in the owner's column (ada's)
    harness.key_press(Key::ArrowDown);
    harness.run_steps(1);
    assert_eq!(harness.state().state.active, Some((1, 1)));
}
