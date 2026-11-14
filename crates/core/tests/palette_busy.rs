//! The palette while a command runs (F23 as refined) and while the app's own search is out
//! (0.28.1's `searching`): held, then let go after `busy-release`; Searching after `wait-shown`.

use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::{CommandGroup, CommandPalette, MenuItem, PaletteOutcome, install};

struct Held {
    groups: Vec<CommandGroup>,
    query: String,
    highlight: usize,
    running: Option<(usize, usize)>,
    searching: bool,
    last: PaletteOutcome,
}

fn harness(running: Option<(usize, usize)>, searching: bool, query: &str) -> Harness<'static, Held> {
    let held = Held {
        groups: vec![CommandGroup::new(
            "Runs",
            [
                MenuItem::action("Rerun nightly-export"),
                MenuItem::action("Rerun sync-inventory"),
            ],
        )],
        query: query.into(),
        highlight: 0,
        running,
        searching,
        last: PaletteOutcome::default(),
    };
    Harness::builder()
        .with_size([800.0, 600.0])
        .with_step_dt(0.05)
        .build_ui_state(
            |ui, held: &mut Held| {
                install(ui.ctx(), &Default::default());
                let outcome = CommandPalette::new("palette", &held.groups, &mut held.query, &mut held.highlight)
                    .running(held.running)
                    .searching(held.searching)
                    .show(ui.ctx());
                if outcome != PaletteOutcome::default() {
                    held.last = outcome;
                }
            },
            held,
        )
}

#[test]
fn a_running_command_holds_the_palette_then_lets_go() {
    let mut harness = harness(Some((0, 0)), false, "");
    harness.run_steps(4);
    assert!(harness.query_by_label("Running").is_some());
    harness.key_press(egui::Key::Escape);
    harness.step();
    assert!(!harness.state().last.close, "held before busy-release");
    assert!(harness.query_by_label("Stop waiting").is_none());
    // a twentieth of a second a step: past five seconds the way out appears
    for _ in 0..105 {
        harness.step();
    }
    harness.get_by_label("Stop waiting").click();
    harness.run_steps(2);
    assert!(harness.state().last.stop_waiting);
    assert!(harness.state().last.close);
}

#[test]
fn a_search_out_says_so_only_after_its_wait() {
    let mut harness = harness(None, true, "nothing like it");
    harness.run_steps(2);
    assert!(
        harness.query_by_label("Searching").is_none(),
        "an answer sooner draws no wait"
    );
    harness.run_steps(6);
    assert!(harness.query_by_label("Searching").is_some());
}
