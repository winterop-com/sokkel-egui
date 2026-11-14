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
