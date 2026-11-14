//! An arranged dashboard: a panel dragged by its grip lands where it is let go, its size menu and
//! remove change the layout, the arrow keys move it from its grip, a widget pressed adds a panel at
//! the end; and edit mode's Cancel throws the edits away, Done keeps them.

use egui::{Event, Key, PointerButton, Pos2};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::{
    DashboardArrange, DashboardEdit, DashboardEditActions, DashboardLayoutItem, DashboardPanel, DashboardWidget,
    PanelSpan, arranged_index, install,
};

struct State {
    layout: Vec<DashboardLayoutItem>,
    edit: DashboardEdit,
    added: Option<(String, usize)>,
}

fn layout() -> Vec<DashboardLayoutItem> {
    ["runs", "failed", "queued"]
        .into_iter()
        .map(|id| DashboardLayoutItem::new(id, PanelSpan::Third))
        .collect()
}

fn harness(arranging: bool) -> Harness<'static, State> {
    let mut state = State {
        layout: layout(),
        edit: DashboardEdit::default(),
        added: None,
    };
    if arranging {
        let kept = state.layout.clone();
        state.edit.edit(&kept);
    }
    let mut harness = Harness::builder().with_size([1000.0, 400.0]).build_ui_state(
        |ui, state: &mut State| {
            install(ui.ctx(), &Default::default());
            ui.horizontal(|ui| {
                DashboardEditActions::new(&mut state.edit, &mut state.layout).show(ui);
            });
            let editing = state.edit.editing();
            let palette = [DashboardWidget::new("note", "Note")];
            let label = |item: &DashboardLayoutItem| item.id.clone();
            let layout = state.edit.layout(&mut state.layout);
            let shown = DashboardArrange::new("test", layout, &label)
                .arranging(editing)
                .palette(&palette)
                .show(ui, |ui, item| {
                    DashboardPanel::new(&item.id).show(ui, |ui| {
                        ui.label("12");
                    });
                });
            if shown.added.is_some() {
                state.added = shown.added;
            }
        },
        state,
    );
    harness.run_steps(4);
    harness
}

fn ids(layout: &[DashboardLayoutItem]) -> Vec<&str> {
    layout.iter().map(|i| i.id.as_str()).collect()
}

fn drag(harness: &mut Harness<'static, State>, from: Pos2, to: Pos2) {
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
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    harness.run_steps(3);
}

#[test]
fn where_a_thing_lands() {
    let ids = ["a", "b", "c"];
    assert_eq!(arranged_index(&ids, Some("a"), Some("c"), true), Some(2));
    assert_eq!(arranged_index(&ids, Some("c"), Some("a"), false), Some(0));
    // dropped back where it was
    assert_eq!(arranged_index(&ids, Some("b"), Some("a"), true), None);
    assert_eq!(arranged_index(&ids, None, None, true), Some(3));
}

#[test]
fn a_panel_dragged_by_its_grip_lands_where_it_is_let_go() {
    let mut harness = harness(true);
    let grip = harness.get_by_label("Move runs").rect().center();
    let queued = harness.get_by_label("Move queued").rect();
    // past the middle of queued's panel, near the grid's right edge
    let to = Pos2::new(queued.right() - 4.0, queued.center().y + 40.0);
    drag(&mut harness, grip, to);
    let state = harness.state();
    assert_eq!(
        ids(state.edit.clone().layout(&mut state.layout.clone())),
        ["failed", "queued", "runs"]
    );
}

#[test]
fn the_size_menu_and_remove_change_the_layout() {
    let mut harness = harness(true);
    harness.get_by_label("Size of failed").click();
    harness.run_steps(3);
    harness.get_by_label("Half").click();
    harness.run_steps(3);
    harness.get_by_label("Remove queued").click();
    harness.run_steps(3);
    let mut edit = harness.state().edit.clone();
    let mut kept = harness.state().layout.clone();
    let layout = edit.layout(&mut kept).clone();
    assert_eq!(ids(&layout), ["runs", "failed"]);
    assert_eq!(layout[1].span, PanelSpan::Half);
}

#[test]
fn the_arrow_keys_move_a_panel_from_its_grip() {
    let mut harness = harness(true);
    harness.get_by_label("Move runs").focus();
    harness.run_steps(2);
    harness.key_press(Key::ArrowRight);
    harness.run_steps(3);
    // the focus follows the grip, so a second press moves it on
    harness.key_press(Key::ArrowRight);
    harness.run_steps(3);
    let mut edit = harness.state().edit.clone();
    let mut kept = harness.state().layout.clone();
    assert_eq!(ids(edit.layout(&mut kept)), ["failed", "queued", "runs"]);
}

#[test]
fn a_widget_pressed_is_added_at_the_end() {
    let mut harness = harness(true);
    harness.get_by_label("Add Note").click();
    harness.run_steps(2);
    assert_eq!(harness.state().added, Some(("note".into(), 3)));
}

#[test]
fn cancel_throws_the_edits_away_and_done_keeps_them() {
    let mut harness = harness(false);
    assert!(harness.query_by_label("Move runs").is_none());
    harness.get_by_label("Edit").click();
    harness.run_steps(3);
    harness.get_by_label("Remove runs").click();
    harness.run_steps(3);
    harness.get_by_label("Cancel").click();
    harness.run_steps(3);
    assert_eq!(ids(&harness.state().layout), ["runs", "failed", "queued"]);
    harness.get_by_label("Edit").click();
    harness.run_steps(3);
    harness.get_by_label("Remove runs").click();
    harness.run_steps(3);
    harness.get_by_label("Done").click();
    harness.run_steps(3);
    assert!(!harness.state().edit.editing());
    assert_eq!(ids(&harness.state().layout), ["failed", "queued"]);
}

#[test]
fn a_guarded_cancel_with_edits_only_asks() {
    let mut harness = Harness::builder().with_size([800.0, 200.0]).build_ui_state(
        |ui, state: &mut (DashboardEdit, Vec<DashboardLayoutItem>, bool)| {
            install(ui.ctx(), &Default::default());
            ui.horizontal(|ui| {
                let outcome = DashboardEditActions::new(&mut state.0, &mut state.1)
                    .guard_cancel(true)
                    .show(ui);
                if outcome.cancel_asked {
                    state.2 = true;
                }
            });
        },
        (DashboardEdit::default(), layout(), false),
    );
    harness.run_steps(2);
    harness.get_by_label("Edit").click();
    harness.run_steps(2);
    // no edits yet: Cancel cancels
    harness.get_by_label("Cancel").click();
    harness.run_steps(2);
    assert!(!harness.state().0.editing());
    harness.get_by_label("Edit").click();
    harness.run_steps(2);
    let (edit, kept, _) = harness.state_mut();
    edit.layout(kept).pop();
    harness.get_by_label("Cancel").click();
    harness.run_steps(2);
    assert!(harness.state().2);
    assert!(harness.state().0.editing());
}
