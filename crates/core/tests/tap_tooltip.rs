//! A shut button's reason on a tap by touch, held for `tip-hold` (0.28.1's `tap`), and not on a
//! mouse press.

use egui::{Event, PointerButton, Pos2, TouchDeviceId, TouchId, TouchPhase};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::{Button, install};

const REASON: &str = "The run is still going.";

fn harness() -> Harness<'static> {
    Harness::builder()
        .with_size([400.0, 200.0])
        .with_step_dt(0.25)
        .build_ui(|ui| {
            install(ui.ctx(), &Default::default());
            ui.add_space(40.0);
            ui.add(Button::new("Rerun").reason(REASON));
        })
}

fn press(harness: &mut Harness<'static>, at: Pos2, touch: bool) {
    for (phase, pressed) in [(TouchPhase::Start, true), (TouchPhase::End, false)] {
        if touch {
            harness.event(Event::Touch {
                device_id: TouchDeviceId(1),
                id: TouchId(1),
                phase,
                pos: at,
                force: None,
            });
        }
        harness.event(Event::PointerMoved(at));
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        });
        harness.step();
    }
    // the pointer goes away, as a finger lifted leaves it, so no hover keeps the tooltip
    harness.event(Event::PointerGone);
    harness.step();
}

#[test]
fn a_tap_holds_the_reason_a_mouse_press_does_not() {
    let mut harness = harness();
    harness.run_steps(2);
    let at = harness.get_by_label("Rerun").rect().center();
    press(&mut harness, at, false);
    assert!(harness.query_by_label(REASON).is_none(), "a mouse press shows nothing");
    press(&mut harness, at, true);
    harness.run_steps(4);
    assert!(harness.query_by_label(REASON).is_some(), "held a second after the tap");
    harness.run_steps(16);
    assert!(harness.query_by_label(REASON).is_none(), "let go after tip-hold");
}
