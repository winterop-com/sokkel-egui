//! A dialog on a phone: a full-screen sheet with its footer at the window's foot, where a confirm
//! dialog stays a box sized to its words.

use egui_kittest::Harness;
use sokkel_ui_egui::{Appearance, Button, ButtonVariant, Dialog, TextField, install};

#[test]
fn a_dialog_on_a_phone_is_a_sheet() {
    let mut name = String::from("nightly-export");
    let mut harness = Harness::builder().with_size([390.0, 844.0]).build_ui(move |ui| {
        install(ui.ctx(), &Appearance::default());
        Dialog::new("rename", "Rename nightly-export")
            .description("Runs keep the name they ran under.")
            .show(
                ui.ctx(),
                |ui| {
                    ui.add(TextField::new("Name", &mut name));
                },
                |ui| {
                    ui.add(Button::new("Rename"));
                    ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                },
            );
    });
    harness.run_steps(3);
    harness.snapshot("dialog_sheet");
}
