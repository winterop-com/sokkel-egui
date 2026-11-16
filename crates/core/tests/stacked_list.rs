//! A list stacked as labeled rows: the menu only on a row with actions, each row as tall as its
//! words, and with no rows its empty words, as the web's listed `DataTable`.

use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::{
    Appearance, Cell, Column, DataTable, MenuItem, StackLayout, StackPlace, TableState, TableVariant, TableWords,
    install,
};

struct List {
    rows: Vec<Vec<Cell>>,
    state: TableState,
    height: f32,
}

fn list(rows: Vec<Vec<Cell>>) -> Harness<'static, List> {
    let mut harness = Harness::builder().with_size([360.0, 400.0]).build_ui_state(
        |ui, list: &mut List| {
            install(ui.ctx(), &Appearance::default());
            let columns = [
                Column::new("Version", 104.0),
                Column::new("What it changed", 128.0).fill().stack(StackPlace::Primary),
                Column::new("By", 104.0).stack(StackPlace::Secondary),
            ];
            let top = ui.cursor().top();
            DataTable::new("deploys", &columns, &list.rows, &mut list.state)
                .variant(TableVariant::List)
                .stack_layout(StackLayout::Labeled)
                .stack_below(420.0)
                .actions(|row| {
                    if row == 0 {
                        vec![MenuItem::action("Roll back to v2.41.1")]
                    } else {
                        Vec::new()
                    }
                })
                .words(TableWords {
                    empty: "No deploys yet.".into(),
                    ..Default::default()
                })
                .footer(false)
                .show(ui);
            list.height = ui.cursor().top() - top;
        },
        List {
            rows,
            state: TableState::default(),
            height: 0.0,
        },
    );
    harness.run_steps(3);
    harness
}

fn deploy(version: &str, note: &str) -> Vec<Cell> {
    vec![Cell::Mono(version.into()), Cell::from(note), Cell::from("grace")]
}

#[test]
fn only_a_row_with_actions_has_a_menu() {
    let harness = list(vec![
        deploy("v2.41.2", "Retries on a failed upload step"),
        deploy("v2.41.1", "A fix for a slow validate step"),
    ]);
    assert_eq!(harness.query_all_by_label("Actions").count(), 1);
}

#[test]
fn a_listed_row_is_as_tall_as_its_words() {
    let one_line = list(vec![deploy("v2.41.2", "Smaller exports")]).state().height;
    let two_lines = list(vec![deploy(
        "v2.41.2",
        "Retries on a failed upload step, then on a slow validate step, then gives up",
    )])
    .state()
    .height;
    // a one-line value keeps no room for a second line, which a longer one takes
    assert!((two_lines - one_line - 22.0).abs() < 1.0, "{one_line} then {two_lines}");
}

#[test]
fn with_no_rows_it_says_its_empty_words() {
    let harness = list(Vec::new());
    assert!(harness.query_by_label("No deploys yet.").is_some());
}
