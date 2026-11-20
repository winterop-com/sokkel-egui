//! A table whose rows are a tree, its keys as the web's: Right opens the active branch and does
//! nothing more, Left closes it or moves to its parent, Down walks the rows as drawn (children
//! sorted among themselves under their parent), Home and End reach the ends. A branch whose
//! children are not given asks for them once while it waits, and one refused offers Try again.
//! Selecting a branch selects every row under it, and pages count the top-level rows. A press or
//! Space with Shift selects the range from the row selected last, a press with Cmd or Ctrl one row.

use std::collections::BTreeSet;

use egui::Key;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{Appearance, Cell, Column, DataTable, TableState, install};

struct Tree {
    columns: Vec<Column>,
    rows: Vec<Vec<Cell>>,
    parents: Vec<Option<usize>>,
    /// The rows that say they have children not given yet.
    lazy: BTreeSet<usize>,
    state: TableState,
    /// Every branch the table asked children for, in order.
    asked: Vec<usize>,
    selectable: bool,
    /// Rows to a page, for a paged tree.
    paging: Option<usize>,
}

/// harbor (0) with its jobs sync-inventory (2) and nightly-export (3), whose run is run_4120 (4);
/// juniper (1) with no children given, but some to come.
fn tree() -> Harness<'static, Tree> {
    let names = ["harbor", "juniper", "sync-inventory", "nightly-export", "run_4120"];
    let tree = Tree {
        columns: vec![Column::new("Name", 240.0).sortable(true)],
        rows: names.iter().map(|name| vec![Cell::from(*name)]).collect(),
        parents: vec![None, None, Some(0), Some(0), Some(3)],
        lazy: [1].into_iter().collect(),
        state: TableState::sorted(0, true),
        asked: Vec::new(),
        selectable: false,
        paging: None,
    };
    let mut harness = Harness::builder().with_size([480.0, 360.0]).build_ui_state(
        |ui, tree: &mut Tree| {
            install(ui.ctx(), &Appearance::default());
            let lazy = tree.lazy.clone();
            let mut table = DataTable::new("jobs", &tree.columns, &tree.rows, &mut tree.state)
                .tree(tree.parents.clone())
                .lazy(move |row| lazy.contains(&row))
                .selectable(tree.selectable);
            if let Some(size) = tree.paging {
                table = table.paging(&[size]);
            }
            let out = table.show(ui);
            tree.asked.extend(out.load_children);
        },
        tree,
    );
    harness.run_steps(3);
    harness
}

fn key(harness: &mut Harness<'static, Tree>, key: Key) {
    harness.key_press(key);
    harness.run_steps(1);
}

fn active(harness: &Harness<'static, Tree>) -> Option<usize> {
    harness.state().state.active.map(|(row, _)| row)
}

#[test]
fn right_opens_and_left_closes_or_goes_to_the_parent() {
    let mut harness = tree();
    harness.state_mut().state.focus_row(0);
    harness.run_steps(1);
    // Right opens harbor, the keys staying on it; Right again on the open branch does nothing
    key(&mut harness, Key::ArrowRight);
    assert!(harness.state().state.expanded.contains(&0));
    assert_eq!(active(&harness), Some(0));
    key(&mut harness, Key::ArrowRight);
    assert_eq!(active(&harness), Some(0));
    // Down to its first child as sorted (nightly-export before sync-inventory), which Right opens
    key(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness), Some(3));
    key(&mut harness, Key::ArrowRight);
    assert!(harness.state().state.expanded.contains(&3));
    assert_eq!(active(&harness), Some(3));
    // Down to its run, a leaf: Right does nothing
    key(&mut harness, Key::ArrowDown);
    key(&mut harness, Key::ArrowRight);
    assert_eq!(active(&harness), Some(4));
    // Left on a leaf moves to its parent, Left on an open branch closes it, then to its parent
    key(&mut harness, Key::ArrowLeft);
    assert_eq!(active(&harness), Some(3));
    key(&mut harness, Key::ArrowLeft);
    assert!(!harness.state().state.expanded.contains(&3));
    assert_eq!(active(&harness), Some(3));
    key(&mut harness, Key::ArrowLeft);
    assert_eq!(active(&harness), Some(0));
    key(&mut harness, Key::ArrowLeft);
    assert!(!harness.state().state.expanded.contains(&0));
    // Left on a top-level row, closed, stays
    key(&mut harness, Key::ArrowLeft);
    assert_eq!(active(&harness), Some(0));
}

#[test]
fn home_end_and_the_page_keys_reach_along_the_rows_as_drawn() {
    let mut harness = tree();
    harness.state_mut().state.expanded.extend([0, 3]);
    harness.state_mut().state.focus_row(3);
    harness.run_steps(1);
    key(&mut harness, Key::End);
    assert_eq!(active(&harness), Some(1));
    key(&mut harness, Key::Home);
    assert_eq!(active(&harness), Some(0));
    // a view's worth: the table's 360 hold more than these five rows, so to the last
    key(&mut harness, Key::PageDown);
    assert_eq!(active(&harness), Some(1));
    key(&mut harness, Key::PageUp);
    assert_eq!(active(&harness), Some(0));
}

#[test]
fn selecting_a_branch_takes_every_row_under_it() {
    let mut harness = tree();
    harness.state_mut().selectable = true;
    harness.state_mut().state.focus_row(0);
    harness.run_steps(1);
    // closed, harbor still takes its jobs and nightly-export's run
    key(&mut harness, Key::Space);
    let chosen: Vec<usize> = harness.state().state.selected.iter().copied().collect();
    assert_eq!(chosen, [0, 2, 3, 4]);
    key(&mut harness, Key::Space);
    assert!(harness.state().state.selected.is_empty());
}

#[test]
fn pages_count_the_top_level_rows_with_their_open_rows() {
    let mut harness = tree();
    harness.state_mut().paging = Some(1);
    harness.state_mut().state.expanded.extend([0, 3]);
    harness.state_mut().state.focus_row(0);
    harness.run_steps(2);
    // the first page is harbor and every open row under it, not harbor alone
    key(&mut harness, Key::End);
    assert_eq!(active(&harness), Some(2));
}

#[test]
fn a_refused_branch_offers_try_again_which_asks_again() {
    let mut harness = tree();
    harness.state_mut().state.expanded.insert(1);
    harness.run_steps(2);
    assert_eq!(harness.state().asked, [1]);
    harness.state_mut().state.refuse_children(1);
    harness.run_steps(2);
    harness.get_by_label("The server refused this read");
    // refused, it is not asked again until Try again
    assert_eq!(harness.state().asked, [1]);
    harness.get_by_label("Try again").click();
    harness.run_steps(2);
    assert_eq!(harness.state().asked, [1, 1]);
    assert!(harness.query_by_label("Try again").is_none());
}

#[test]
fn down_walks_the_rows_as_drawn() {
    let mut harness = tree();
    harness.state_mut().state.expanded.extend([0, 3]);
    harness.state_mut().state.focus_row(0);
    harness.run_steps(1);
    let mut walked = vec![active(&harness).unwrap()];
    for _ in 0..5 {
        key(&mut harness, Key::ArrowDown);
        walked.push(active(&harness).unwrap());
    }
    // harbor, its jobs sorted with nightly-export's run under it, then juniper, where it stops
    assert_eq!(walked, [0, 3, 4, 2, 1, 1]);
    // children sort among themselves: descending, sync-inventory comes first under harbor
    harness.state_mut().state.sort = Some((0, false));
    harness.state_mut().state.focus_row(0);
    harness.run_steps(1);
    key(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness), Some(2));
}

#[test]
fn a_branch_with_children_to_come_asks_once_while_it_waits() {
    let mut harness = tree();
    assert!(harness.state().asked.is_empty(), "nothing asked while closed");
    harness.state_mut().state.focus_row(1);
    harness.run_steps(1);
    key(&mut harness, Key::ArrowRight);
    harness.run_steps(3);
    assert_eq!(harness.state().asked, [1], "asked once, not every frame");
    // the children come: rows whose parent is juniper, and it no longer waits
    {
        let tree = harness.state_mut();
        tree.rows.push(vec![Cell::from("rebuild-index")]);
        tree.parents.push(Some(1));
        tree.lazy.clear();
    }
    harness.run_steps(2);
    key(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness), Some(5), "the child that came is drawn under it");
    assert_eq!(harness.state().asked, [1]);
}

#[test]
fn a_branch_closed_while_it_waits_asks_again_when_opened() {
    let mut harness = tree();
    harness.state_mut().state.expanded.insert(1);
    harness.run_steps(2);
    assert_eq!(harness.state().asked, [1]);
    harness.state_mut().state.expanded.remove(&1);
    harness.run_steps(2);
    harness.state_mut().state.expanded.insert(1);
    harness.run_steps(2);
    assert_eq!(harness.state().asked, [1, 1]);
}

fn chosen(harness: &Harness<'static, Tree>) -> Vec<usize> {
    harness.state().state.selected.iter().copied().collect()
}

fn press(harness: &mut Harness<'static, Tree>, label: &str, modifiers: egui::Modifiers) {
    harness.get_by_label(label).click_modifiers(modifiers);
    harness.run_steps(2);
}

#[test]
fn a_press_with_shift_selects_the_range_and_with_cmd_or_ctrl_one_row() {
    let mut harness = tree();
    harness.state_mut().selectable = true;
    harness.state_mut().state.expanded.extend([0, 3]);
    harness.run_steps(2);
    // drawn: harbor, nightly-export, run_4120, sync-inventory, juniper
    press(&mut harness, "run_4120", egui::Modifiers::COMMAND);
    assert_eq!(chosen(&harness), [4]);
    // Shift: from the row pressed last to this one, as drawn
    press(&mut harness, "sync-inventory", egui::Modifiers::SHIFT);
    assert_eq!(chosen(&harness), [2, 4]);
    press(&mut harness, "harbor", egui::Modifiers::SHIFT);
    assert_eq!(chosen(&harness), [0, 2, 3, 4]);
    // Ctrl on a branch clears it with every row under it
    press(&mut harness, "nightly-export", egui::Modifiers::CTRL);
    assert_eq!(chosen(&harness), [0, 2]);
    // a range from a row that is off, onto one that is on, clears the range
    press(&mut harness, "harbor", egui::Modifiers::SHIFT);
    assert_eq!(chosen(&harness), [2]);
    // a plain press selects nothing
    press(&mut harness, "juniper", egui::Modifiers::NONE);
    assert_eq!(chosen(&harness), [2]);
}

#[test]
fn shift_space_selects_the_range_from_the_row_selected_last() {
    let mut harness = tree();
    harness.state_mut().selectable = true;
    harness.state_mut().state.expanded.extend([0, 3]);
    harness.state_mut().state.focus_row(4);
    harness.run_steps(2);
    key(&mut harness, Key::Space);
    assert_eq!(chosen(&harness), [4]);
    key(&mut harness, Key::ArrowDown);
    key(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness), Some(1));
    harness.key_press_modifiers(egui::Modifiers::SHIFT, Key::Space);
    harness.run_steps(1);
    assert_eq!(chosen(&harness), [1, 2, 4]);
}
