//! Toasts: one added shows until its time runs out, the close button takes it away, and one
//! without a timeout stays.

use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::{Toast, Toaster, install, toasts};

fn harness() -> Harness<'static> {
    Harness::builder().with_size([800.0, 500.0]).build_ui(|ui| {
        install(ui.ctx(), &Default::default());
        Toaster::new().show(ui.ctx());
    })
}

#[test]
fn a_toast_goes_when_its_time_is_up() {
    let mut harness = harness();
    harness.run();
    Toast::success("Overview made").timeout(Some(0.5)).show(&harness.ctx);
    Toast::new("Stays").timeout(None).show(&harness.ctx);
    harness.run_steps(2);
    assert_eq!(toasts(&harness.ctx).len(), 2);
    // a step is a sixtieth of a second
    harness.run_steps(60);
    let left: Vec<String> = toasts(&harness.ctx).into_iter().map(|t| t.title).collect();
    assert_eq!(left, ["Stays"]);
}

#[test]
fn the_close_button_closes_it() {
    let mut harness = harness();
    harness.run();
    Toast::new("Overview deleted").timeout(None).show(&harness.ctx);
    harness.run_steps(2);
    harness.get_by_label("Close toast").click();
    harness.run_steps(2);
    assert!(toasts(&harness.ctx).is_empty());
}

/// Where the toast stands, by its close button.
fn close_at(harness: &Harness<'static>) -> egui::Rect {
    harness.get_by_label("Close toast").rect()
}

#[test]
fn a_new_toast_rises_into_place() {
    let mut harness = Harness::builder()
        .with_size([800.0, 500.0])
        .with_step_dt(1.0 / 60.0)
        .build_ui(|ui| {
            install(ui.ctx(), &Default::default());
            Toaster::new().show(ui.ctx());
        });
    harness.run();
    Toast::new("Overview made").timeout(None).show(&harness.ctx);
    harness.run_steps(3);
    let rising = close_at(&harness).center().y;
    harness.run_steps(60);
    let placed = close_at(&harness).center().y;
    assert!(rising > placed + 20.0, "it rises from below: {rising} then {placed}");
}

fn swipe(harness: &mut Harness<'static>, by: egui::Vec2) {
    let from = close_at(harness).center() - egui::vec2(120.0, 0.0);
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    for step in 1..=10 {
        harness.hover_at(from + by * step as f32 / 10.0);
        harness.run();
    }
    harness.drop_at(from + by);
    harness.run();
}

#[test]
fn a_toast_swiped_right_past_forty_goes_and_one_let_go_sooner_springs_back() {
    let mut harness = harness();
    harness.run();
    Toast::new("Overview deleted").timeout(None).show(&harness.ctx);
    harness.run_steps(4);
    let placed = close_at(&harness);
    swipe(&mut harness, egui::vec2(20.0, 0.0));
    harness.run_steps(4);
    assert_eq!(toasts(&harness.ctx).len(), 1, "short of the swipe it stays");
    assert!(
        (close_at(&harness).center().x - placed.center().x).abs() < 1.0,
        "and springs back"
    );
    swipe(&mut harness, egui::vec2(80.0, 0.0));
    assert!(toasts(&harness.ctx).is_empty(), "past the swipe it goes");
}

#[test]
fn a_toast_swiped_down_goes() {
    let mut harness = harness();
    harness.run();
    Toast::new("Overview deleted").timeout(None).show(&harness.ctx);
    harness.run_steps(4);
    swipe(&mut harness, egui::vec2(0.0, 60.0));
    assert!(toasts(&harness.ctx).is_empty());
}

#[test]
fn a_toast_s_action_says_which_and_closes_it() {
    let mut harness = harness();
    harness.run();
    let id = Toast::new("INC-42 opened")
        .description("Elevated error rates on api")
        .action("Open")
        .timeout(None)
        .show(&harness.ctx);
    harness.run_steps(4);
    harness.snapshot("toast_action");
    harness.get_by_label("Open").click();
    harness.run_steps(2);
    assert_eq!(sokkel_ui_egui::toast_acted(&harness.ctx), vec![id]);
    assert!(toasts(&harness.ctx).is_empty());
    assert!(sokkel_ui_egui::toast_acted(&harness.ctx).is_empty(), "taken once");
}
