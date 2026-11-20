//! Selection as the web's `DataTable` draws and takes it: labeled cards and the labeled list carry
//! a selection box before the title, a chosen card is ringed in its paint and a chosen listed row
//! stands on its paint's ground; the grid's cards are ringed the same, their box at the top corner
//! while pointed at or while any row is chosen. A Shift range in a grouped table runs over the
//! web's `flat`, the groups in the order their rows were given (those `group_order` names first),
//! each group's rows as sorted, a folded group's left out; a paged table is not grouped.

use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{
    Appearance, Cell, Column, DataTable, StackLayout, StackPlace, TableState, TableVariant, Tokens, install,
};

struct Jobs {
    columns: Vec<Column>,
    rows: Vec<Vec<Cell>>,
    groups: Option<Vec<String>>,
    state: TableState,
    /// Draw the rows as labeled cards or a labeled list.
    stacked: Option<TableVariant>,
    /// Rows to a page.
    paging: Option<usize>,
    group_order: Vec<String>,
    /// Offer the grid's cards.
    cards: bool,
}

fn jobs(rows: &[(&str, &str)], groups: Option<&[&str]>, stacked: Option<TableVariant>) -> Harness<'static, Jobs> {
    let jobs = Jobs {
        columns: vec![
            Column::new("Job", 200.0).sortable(true),
            Column::new("Project", 120.0).stack(StackPlace::Line),
        ],
        rows: rows
            .iter()
            .map(|(job, project)| vec![Cell::from(*job), Cell::from(*project)])
            .collect(),
        groups: groups.map(|g| g.iter().map(|k| (*k).to_owned()).collect()),
        state: TableState::sorted(0, true),
        stacked,
        paging: None,
        group_order: Vec::new(),
        cards: false,
    };
    let mut harness = Harness::builder().with_size([480.0, 480.0]).build_ui_state(
        |ui, jobs: &mut Jobs| {
            install(ui.ctx(), &Appearance::default());
            let mut table = DataTable::new("jobs", &jobs.columns, &jobs.rows, &mut jobs.state)
                .selectable(true)
                .footer(false);
            if let Some(groups) = &jobs.groups {
                table = table.groups(groups.clone()).group_order(jobs.group_order.clone());
            }
            if let Some(size) = jobs.paging {
                table = table.paging(&[size]);
            }
            if jobs.cards {
                table = table.cards(160.0, 80.0, |ui, index, width| {
                    let card = ui.allocate_exact_size(egui::vec2(width, 80.0), egui::Sense::click()).1;
                    card.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("card {index}")));
                    card
                });
            }
            if let Some(variant) = jobs.stacked {
                table = table
                    .variant(variant)
                    .stack_layout(StackLayout::Labeled)
                    .stack_below(2000.0);
            }
            table.show(ui);
        },
        jobs,
    );
    harness.run_steps(3);
    harness
}

fn chosen(harness: &Harness<'static, Jobs>) -> Vec<usize> {
    harness.state().state.selected.iter().copied().collect()
}

fn press(harness: &mut Harness<'static, Jobs>, label: &str, modifiers: egui::Modifiers) {
    // a row reads as its cells' words
    harness.get_by_label_contains(label).click_modifiers(modifiers);
    harness.run_steps(2);
}

const JOBS: [(&str, &str); 4] = [
    ("nightly-export", "harbor"),
    ("archive-logs", "juniper"),
    ("sync-inventory", "harbor"),
    ("build-docs", "juniper"),
];

#[test]
fn a_grouped_range_takes_the_groups_in_the_order_their_rows_came() {
    let mut harness = jobs(&JOBS, Some(&["harbor", "juniper", "harbor", "juniper"]), None);
    // sorted by name, but harbor's rows came first: nightly-export, sync-inventory, then
    // juniper's archive-logs, build-docs, as the web's `groupRows`
    press(&mut harness, "sync-inventory", egui::Modifiers::COMMAND);
    press(&mut harness, "archive-logs", egui::Modifiers::SHIFT);
    assert_eq!(chosen(&harness), [1, 2]);
}

#[test]
fn a_grouped_range_leaves_out_a_folded_groups_rows() {
    let rows = [
        ("nightly-export", "harbor"),
        ("archive-logs", "juniper"),
        ("sync-inventory", "harbor"),
        ("build-docs", "juniper"),
        ("prune-runs", "larkspur"),
    ];
    let mut harness = jobs(
        &rows,
        Some(&["harbor", "juniper", "harbor", "juniper", "larkspur"]),
        None,
    );
    harness.state_mut().state.folded.insert("juniper".into());
    harness.run_steps(2);
    press(&mut harness, "nightly-export", egui::Modifiers::COMMAND);
    press(&mut harness, "prune-runs", egui::Modifiers::SHIFT);
    // juniper's rows are not drawn, so not in the web's `flat`: not in the range
    assert_eq!(chosen(&harness), [0, 2, 4]);
}

/// Every rect stroke drawn: its color and width.
fn strokes(harness: &Harness<'static, Jobs>) -> Vec<(egui::Color32, f32)> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) => Some((rect.stroke.color, rect.stroke.width)),
            _ => None,
        })
        .collect()
}

/// Every rect fill drawn wider than `least`.
fn grounds(harness: &Harness<'static, Jobs>, least: f32) -> Vec<egui::Color32> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) if rect.rect.width() > least => Some(rect.fill),
            _ => None,
        })
        .collect()
}

fn tokens() -> Tokens {
    Tokens::resolve(&Appearance::default(), false)
}

#[test]
fn a_labeled_card_has_a_selection_box_and_a_chosen_card_is_ringed() {
    let mut harness = jobs(&JOBS[..2], None, Some(TableVariant::Table));
    let ring = tokens()
        .paint("table.base.selected.card")
        .and_then(|p| p.ring)
        .expect("a chosen card's ring");
    assert_eq!(harness.query_all_by_label("Select row").count(), 2);
    assert!(!strokes(&harness).contains(&ring));
    // the box selects its row alone, Shift or not, as the web's stacked box
    harness
        .query_all_by_label("Select row")
        .nth(1)
        .expect("the second card's box")
        .click();
    harness.run_steps(2);
    // sorted by name: archive-logs, then nightly-export
    assert_eq!(chosen(&harness), [0]);
    assert!(strokes(&harness).contains(&ring), "no ring {ring:?} drawn");
    // a press on a card opens it, ringed in the opened paint
    harness.state_mut().state.selected.clear();
    harness.state_mut().state.opened = Some(0);
    harness.run_steps(2);
    let opened = tokens()
        .paint("table.base.opened.card")
        .and_then(|p| p.ring)
        .expect("an opened card's ring");
    assert!(strokes(&harness).contains(&opened));
    assert!(!strokes(&harness).contains(&ring));
}

#[test]
fn a_labeled_list_row_has_a_selection_box_and_a_chosen_row_its_ground() {
    let mut harness = jobs(&JOBS[..2], None, Some(TableVariant::List));
    let ground = tokens()
        .paint("table.base.selected.row")
        .and_then(|p| p.ground)
        .expect("a chosen row's ground");
    assert_eq!(harness.query_all_by_label("Select row").count(), 2);
    assert!(!grounds(&harness, 300.0).contains(&ground));
    harness
        .query_all_by_label("Select row")
        .next()
        .expect("the first row's box")
        .click();
    harness.run_steps(2);
    // sorted by name: archive-logs first
    assert_eq!(chosen(&harness), [1]);
    assert!(grounds(&harness, 300.0).contains(&ground));
}

#[test]
fn group_order_puts_the_groups_it_names_first() {
    let mut harness = jobs(&JOBS, Some(&["harbor", "juniper", "harbor", "juniper"]), None);
    harness.state_mut().group_order = vec!["juniper".into()];
    harness.run_steps(2);
    let juniper = harness.get_by_label("Hide group: juniper").rect().top();
    let harbor = harness.get_by_label("Hide group: harbor").rect().top();
    assert!(juniper < harbor, "juniper at {juniper}, harbor at {harbor}");
    // the range follows: archive-logs, build-docs, then nightly-export, sync-inventory
    press(&mut harness, "build-docs", egui::Modifiers::COMMAND);
    press(&mut harness, "nightly-export", egui::Modifiers::SHIFT);
    assert_eq!(chosen(&harness), [0, 3]);
}

#[test]
fn a_paged_table_is_not_grouped() {
    let mut harness = jobs(&JOBS, Some(&["harbor", "juniper", "harbor", "juniper"]), None);
    harness.state_mut().paging = Some(10);
    harness.run_steps(2);
    assert_eq!(harness.query_all_by_label_contains("group:").count(), 0);
    // the rows as sorted, one list: archive-logs, build-docs, nightly-export, sync-inventory
    press(&mut harness, "build-docs", egui::Modifiers::COMMAND);
    press(&mut harness, "nightly-export", egui::Modifiers::SHIFT);
    assert_eq!(chosen(&harness), [0, 3]);
}

#[test]
fn a_grid_card_has_its_box_while_pointed_at_or_any_row_is_chosen_and_a_chosen_card_is_ringed() {
    let mut harness = jobs(&JOBS[..2], None, None);
    harness.state_mut().cards = true;
    harness.state_mut().state.grid = true;
    harness.run_steps(3);
    let ring = tokens()
        .paint("table.base.selected.card")
        .and_then(|p| p.ring)
        .expect("a chosen card's ring");
    // no box until a card is pointed at
    assert_eq!(harness.query_all_by_label("Select row").count(), 0);
    harness.get_by_label("card 1").hover();
    harness.run_steps(2);
    assert_eq!(harness.query_all_by_label("Select row").count(), 1);
    harness.get_by_label("Select row").click();
    harness.run_steps(2);
    assert_eq!(chosen(&harness), [1]);
    assert!(strokes(&harness).contains(&ring), "no ring {ring:?} drawn");
    // with a row chosen every card shows its box
    harness.hover_at(egui::pos2(479.0, 479.0));
    harness.run_steps(2);
    assert_eq!(harness.query_all_by_label("Select row").count(), 2);
}
