//! Figures at one width (`tabular_job`): numbers of as many digits are as wide whatever their
//! digits, in the sans face and in the code face.

use egui::{FontFamily, FontId, TextFormat};
use egui_kittest::Harness;
use sokkel_egui::{Appearance, install, tabular_job};

#[test]
fn numbers_of_as_many_digits_are_as_wide() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui.ctx(), &Appearance::default());
    });
    harness.run_steps(2);
    let ctx = harness.ctx.clone();
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let width = |text: &str| {
            let format = TextFormat {
                font_id: FontId::new(15.0, family.clone()),
                ..Default::default()
            };
            let job = tabular_job(&ctx, text, format);
            ctx.fonts_mut(|f| f.layout_job(job)).size().x
        };
        let ones = width("1,111");
        for other in ["8,888", "4,070", "2,349"] {
            assert!(
                (width(other) - ones).abs() < 0.01,
                "{family:?}: {other} is {} wide, 1,111 is {ones}",
                width(other)
            );
        }
    }
}
