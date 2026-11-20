//! The entry grid is the table with editing: stacked below its width, each count is a field named
//! by its band and its own name; a press opens it, and Tab keeps what was typed and opens the next.

use egui::Key;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::{
    Appearance, Cell, CellEdit, Column, DataTable, Editor, StackLayout, TableState, TableVariant, install,
    sum_aggregate,
};

struct Grid {
    columns: Vec<Column>,
    rows: Vec<Vec<Cell>>,
    state: TableState,
    edits: Vec<CellEdit>,
}

fn grid() -> Harness<'static, Grid> {
    let count = |title: &str| {
        Column::new(title, 96.0)
            .numeric()
            .band("This week")
            .aggregate(sum_aggregate)
            .editable(Editor::Number)
    };
    let grid = Grid {
        columns: vec![Column::new("Job", 160.0).fill(), count("Succeeded"), count("Failed")],
        rows: vec![
            vec![Cell::from("nightly-export"), Cell::from("7"), Cell::from("")],
            vec![Cell::from("send-digest"), Cell::from("7"), Cell::from("0")],
        ],
        state: TableState::default(),
        edits: Vec::new(),
    };
    let mut harness = Harness::builder().with_size([400.0, 600.0]).build_ui_state(
        |ui, grid: &mut Grid| {
            install(ui.ctx(), &Appearance::default());
            let out = DataTable::new("counts", &grid.columns, &grid.rows, &mut grid.state)
                .variant(TableVariant::List)
                .groups(["harbor", "harbor"])
                .stack_layout(StackLayout::Labeled)
                .stack_below(560.0)
                .height(f32::INFINITY)
                .show(ui);
            grid.edits.extend(out.edited);
        },
        grid,
    );
    harness.run_steps(3);
    harness
}

#[test]
fn a_stacked_field_opens_on_a_press_and_tab_keeps_it_and_moves_on() {
    let mut harness = grid();
    harness
        .get_all_by_label("This week · Failed")
        .next()
        .expect("the first row's failed count, named by its band")
        .click();
    harness.run_steps(3);
    harness.get_by_role(egui::accesskit::Role::TextInput).type_text("4");
    harness.run_steps(2);
    harness.key_press(Key::Tab);
    harness.run_steps(3);
    assert_eq!(
        harness.state().edits,
        vec![CellEdit {
            row: 0,
            column: 2,
            value: "4".into(),
        }]
    );
    // the next editable cell, the second row's first count, is open
    assert!(harness.query_by_role(egui::accesskit::Role::TextInput).is_some());
}
