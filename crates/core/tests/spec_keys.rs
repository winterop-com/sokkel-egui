//! The keyboard the parts' specs give them: a choice list is one Tab stop whose arrows move and
//! choose; an issue count opens on its first issue, its arrows move, Escape returns to its face;
//! a server menu opens on ArrowDown and Escape returns to its face; a sortable grid's arrows move
//! the focused card, the focus following it, and Escape puts a picked-up card back; a number cell
//! refuses what is not a number and saves its digits; a prompt submits on Enter, refuses an empty
//! value, holds while busy and starts over at each opening; a chip's face and remove button are
//! Tab stops whose Enter, Space and clicks stop at the chip, and a menu chip opens on ArrowDown.

use egui::{Event, Key, PointerButton, Pos2};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{
    Cell, CellEdit, Choice, ChoiceList, Column, DataTable, Editor, IssueCount, IssueEntry, ItemCard, PromptAnswer,
    PromptDialog, ServerMenu, ServerState, SortableGrid, TableState, install,
};

fn choices() -> Harness<'static, String> {
    let mut harness = Harness::builder().with_size([400.0, 300.0]).build_ui_state(
        |ui, value: &mut String| {
            install(ui.ctx(), &Default::default());
            let choices = [
                Choice {
                    value: "all",
                    label: "Every job",
                    description: None,
                },
                Choice {
                    value: "failed",
                    label: "Failed runs",
                    description: Some("Only the runs that failed."),
                },
                Choice {
                    value: "none",
                    label: "No email",
                    description: None,
                },
            ];
            ChoiceList::new("Notify about", &choices).show(ui, value);
        },
        "failed".to_owned(),
    );
    harness.run_steps(3);
    harness
}

#[test]
fn a_choice_list_is_one_tab_stop_at_the_chosen_choice() {
    let mut harness = choices();
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert!(harness.get_by_label("Failed runs").is_focused());
    // the arrows move and choose, wrapping
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    assert_eq!(harness.state(), "none");
    assert!(harness.get_by_label("No email").is_focused());
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    assert_eq!(harness.state(), "all");
    harness.key_press(Key::ArrowUp);
    harness.run_steps(2);
    assert_eq!(harness.state(), "none");
    // Enter does not choose; Tab leaves the list
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert!(!harness.get_by_label("Every job").is_focused());
    assert!(!harness.get_by_label("Failed runs").is_focused());
}

#[test]
fn an_issue_count_opens_on_its_first_issue_and_escape_returns_to_it() {
    let mut harness = Harness::builder().with_size([500.0, 400.0]).build_ui_state(
        |ui, chosen: &mut Option<usize>| {
            install(ui.ctx(), &Default::default());
            ui.add_space(250.0);
            let list = [
                IssueEntry {
                    label: "Job name".into(),
                    text: "A job named nightly-export already exists.".into(),
                    warning: false,
                },
                IssueEntry {
                    label: "Description".into(),
                    text: "Names over 24 characters are cut short.".into(),
                    warning: true,
                },
            ];
            if let Some(at) = IssueCount::new("issues", &list).show(ui) {
                *chosen = Some(at);
            }
        },
        None,
    );
    harness.run_steps(3);
    let face = "1 error, 1 warning";
    harness.get_by_label(face);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert!(harness.get_by_label(face).is_focused());
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert!(
        harness
            .get_by_label("Job name: A job named nightly-export already exists.")
            .is_focused()
    );
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    assert!(
        harness
            .get_by_label("Description: Names over 24 characters are cut short.")
            .is_focused()
    );
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert!(harness.get_by_label(face).is_focused());
    assert!(
        harness
            .query_by_label("Description: Names over 24 characters are cut short.")
            .is_none()
    );
    // open again and choose with Enter
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert_eq!(*harness.state(), Some(0));
}

#[test]
fn a_server_menu_opens_on_arrow_down_and_escape_returns_to_its_face() {
    let mut harness = Harness::builder().with_size([600.0, 400.0]).build_ui(|ui| {
        install(ui.ctx(), &Default::default());
        ServerMenu::new("Tidevann records", ServerState::Up)
            .version(Some("2.4.0"))
            .check(true)
            .show(ui);
    });
    harness.run_steps(3);
    let face = "Server: Tidevann records 2.4.0, Connected";
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert!(harness.get_by_label(face).is_focused());
    harness.key_press(Key::ArrowDown);
    harness.run_steps(3);
    harness.get_by_label("Check again");
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert!(harness.query_by_label("Check again").is_none());
    assert!(harness.get_by_label(face).is_focused());
}

fn sortable() -> Harness<'static, Vec<&'static str>> {
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, order: &mut Vec<&'static str>| {
            install(ui.ctx(), &Default::default());
            let shown = order.clone();
            let out = SortableGrid::new("jobs", shown.len(), "Jobs")
                .item_label(|index| shown[index].to_owned())
                .show(ui, |ui, index, width, height| {
                    ItemCard::new(shown[index]).show(ui, width, height)
                });
            if let Some((from, to)) = out.moved {
                let taken = order.remove(from);
                order.insert(to, taken);
            }
        },
        vec!["nightly-export", "sync-inventory", "send-digest"],
    );
    harness.run_steps(3);
    harness
}

#[test]
fn a_sortable_grids_arrows_move_the_focused_card_and_escape_puts_it_back() {
    let mut harness = sortable();
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert!(harness.get_by_label("Move nightly-export").is_focused());
    harness.key_press(Key::ArrowRight);
    harness.run_steps(3);
    assert_eq!(*harness.state(), ["sync-inventory", "nightly-export", "send-digest"]);
    assert!(harness.get_by_label("Move nightly-export").is_focused());
    // picked up, moved on, then put back by Escape
    harness.key_press(Key::Space);
    harness.run_steps(2);
    harness.key_press(Key::ArrowRight);
    harness.run_steps(3);
    assert_eq!(*harness.state(), ["sync-inventory", "send-digest", "nightly-export"]);
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert_eq!(*harness.state(), ["sync-inventory", "nightly-export", "send-digest"]);
    assert!(harness.get_by_label("Move nightly-export").is_focused());
    harness.get_by_label("nightly-export moved to 2 of 3");
}

fn drag(harness: &mut Harness<'static, Vec<&'static str>>, from: Pos2, to: Pos2, escape: bool) {
    harness.event(Event::PointerMoved(from));
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    harness.step();
    for k in 1..=8 {
        harness.event(Event::PointerMoved(from + (to - from) * (k as f32 / 8.0)));
        harness.step();
    }
    if escape {
        harness.key_press(Key::Escape);
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    harness.run_steps(3);
}

#[test]
fn a_sortable_card_dragged_lands_where_it_is_let_go_and_escape_puts_it_back() {
    let mut harness = sortable();
    let first = harness.get_by_label("Move nightly-export").rect().center();
    let last = harness.get_by_label("Move send-digest").rect().center();
    let (from, to): (Pos2, Pos2) = (first, last);
    drag(&mut harness, from, to, true);
    assert_eq!(*harness.state(), ["nightly-export", "sync-inventory", "send-digest"]);
    drag(&mut harness, from, to, false);
    assert_eq!(*harness.state(), ["sync-inventory", "send-digest", "nightly-export"]);
}

fn counts(typed: &'static str) -> Option<CellEdit> {
    let mut state = TableState::default();
    state.focus_cell(0, 1);
    let mut harness = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        move |ui, edit: &mut Option<CellEdit>| {
            install(ui.ctx(), &Default::default());
            let columns = [
                Column::new("Job", 160.0).fill(),
                Column::new("Runs", 96.0).numeric().editable(Editor::Number),
            ];
            let rows = vec![vec![Cell::Text("nightly-export".into()), Cell::Text("1,284".into())]];
            let out = DataTable::new("counts", &columns, &rows, &mut state)
                .footer(false)
                .show(ui);
            if out.edited.is_some() {
                *edit = out.edited;
            }
        },
        None,
    );
    harness.run_steps(3);
    harness.event(Event::Text(typed.into()));
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    if harness.state().is_none() {
        harness.get_by_label("Enter a number");
    }
    harness.state().clone()
}

#[test]
fn a_number_cell_refuses_words_and_saves_digits() {
    assert_eq!(counts("seven"), None);
    assert_eq!(
        counts("1,300"),
        Some(CellEdit {
            row: 0,
            column: 1,
            value: "1300".into()
        })
    );
}

#[derive(Default)]
struct Asked {
    open: bool,
    busy: bool,
    answers: Vec<PromptAnswer>,
}

fn prompt() -> Harness<'static, Asked> {
    let mut harness = Harness::builder().with_size([800.0, 600.0]).build_ui_state(
        |ui, asked: &mut Asked| {
            install(ui.ctx(), &Default::default());
            if asked.open
                && let Some(answer) = PromptDialog::new("name", "New project", "Name")
                    .default_value("")
                    .busy(asked.busy)
                    .busy_release(60_000.0)
                    .show(ui.ctx())
            {
                if !matches!(answer, PromptAnswer::Submit(_)) {
                    asked.open = false;
                }
                asked.answers.push(answer);
            }
        },
        Asked {
            open: true,
            ..Default::default()
        },
    );
    harness.run_steps(3);
    harness
}

#[test]
fn a_prompt_submits_on_enter_and_refuses_an_empty_value() {
    let mut harness = prompt();
    // empty: Enter submits nothing
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert!(harness.state().answers.is_empty());
    harness.event(Event::Text("orchard".into()));
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().answers, [PromptAnswer::Submit("orchard".into())]);
    // busy: Escape waits
    harness.state_mut().busy = true;
    harness.run_steps(2);
    harness.key_press(Key::Escape);
    harness.run_steps(2);
    assert!(harness.state().open);
    harness.state_mut().busy = false;
    harness.run_steps(2);
    harness.key_press(Key::Escape);
    harness.run_steps(2);
    assert!(!harness.state().open);
    assert_eq!(harness.state().answers.last(), Some(&PromptAnswer::Cancel));
    // opened again, it starts over
    harness.state_mut().open = true;
    harness.run_steps(3);
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert_eq!(harness.state().answers.len(), 2);
}

#[derive(Default)]
struct ChipPresses {
    row: usize,
    row_keys: usize,
    pressed: usize,
    removed: usize,
    chosen: Option<usize>,
}

fn chips() -> Harness<'static, ChipPresses> {
    let mut harness = Harness::builder().with_size([400.0, 300.0]).build_ui_state(
        |ui, presses: &mut ChipPresses| {
            install(ui.ctx(), &Default::default());
            // a row under the chips, pressed by a click and by Enter or Space
            let row = ui.interact(ui.max_rect(), ui.scope_id().with("row"), egui::Sense::CLICK);
            ui.horizontal(|ui| {
                ui.add(sokkel_egui::Chip::new("nightly"));
                let chip = sokkel_egui::Chip::new("harbor")
                    .button()
                    .select_label("Filter by harbor")
                    .removable(true)
                    .show(ui);
                presses.pressed += usize::from(chip.pressed);
                presses.removed += usize::from(chip.removed);
                let menu = sokkel_egui::Chip::new("juniper")
                    .menu(
                        [sokkel_egui::MenuItem::action("Filter by juniper")],
                        sokkel_egui::ChipMenuAlign::Start,
                    )
                    .show(ui);
                if menu.chosen.is_some() {
                    presses.chosen = menu.chosen;
                }
            });
            presses.row += usize::from(row.clicked());
            presses.row_keys += usize::from(ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space)));
        },
        ChipPresses::default(),
    );
    harness.run_steps(3);
    harness
}

#[test]
fn a_chips_face_and_remove_button_are_tab_stops_whose_presses_stop_at_the_chip() {
    let mut harness = chips();
    // a plain chip takes no focus: the first stop is the button's face, then its remove button
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert!(harness.get_by_label("Filter by harbor").is_focused());
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    harness.key_press(Key::Space);
    harness.run_steps(2);
    assert_eq!(harness.state().pressed, 2);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert!(harness.get_by_label("Remove").is_focused());
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().removed, 1);
    assert_eq!(harness.state().row_keys, 0, "Enter and Space stop at the chip");
    // a click on the face presses the chip, not the row under it
    harness.get_by_label("Filter by harbor").click();
    harness.run_steps(2);
    assert_eq!(harness.state().pressed, 3);
    assert_eq!(harness.state().row, 0);
}

#[test]
fn a_menu_chip_opens_on_arrow_down() {
    let mut harness = chips();
    for _ in 0..3 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
    }
    assert!(harness.get_by_label("juniper").is_focused());
    assert!(harness.query_by_label("Filter by juniper").is_none());
    harness.key_press(Key::ArrowDown);
    harness.run_steps(3);
    harness.get_by_label("Filter by juniper").click();
    harness.run_steps(3);
    assert_eq!(harness.state().chosen, Some(0));
    assert_eq!(harness.state().row, 0);
}
