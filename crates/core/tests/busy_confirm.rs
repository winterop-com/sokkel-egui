//! A busy confirm holds its dialog, and after `busy-release` (5 s) lets it go: Cancel says
//! "Stop waiting" and Escape closes, as the dialog spec's released state says (F23).

use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::{ConfirmAnswer, ConfirmDialog, install};

#[test]
fn a_busy_confirm_holds_then_lets_go() {
    let mut harness = Harness::builder().with_size([800.0, 600.0]).build_ui_state(
        |ui, answer: &mut Option<ConfirmAnswer>| {
            let ctx = ui.ctx().clone();
            let ctx = &ctx;
            install(ctx, &Default::default());
            let shown = ConfirmDialog::new("export", "Export every run?", "Export")
                .busy(Some("Exporting".into()))
                .show(ctx);
            if shown.is_some() {
                *answer = shown;
            }
        },
        None,
    );
    // the default step is a quarter second: two seconds in, Cancel is held and Escape does nothing
    for _ in 0..8 {
        harness.step();
    }
    assert!(harness.query_by_label("Stop waiting").is_none());
    harness.key_press(egui::Key::Escape);
    harness.step();
    assert_eq!(*harness.state(), None);
    // past five seconds the way out appears and lets the work go
    for _ in 0..16 {
        harness.step();
    }
    harness.get_by_label("Stop waiting").click();
    harness.step();
    harness.step();
    assert_eq!(*harness.state(), Some(ConfirmAnswer::StopWaiting));
}
