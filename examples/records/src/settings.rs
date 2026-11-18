//! The settings as a dialog over the page, as the web's `App.tsx` gives them: the look, whether a
//! form keeps the place, and the demo server: whether it answers, and its data put back.

use eframe::egui;
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, ConfirmAnswer, ConfirmDialog, SettingRow, SettingsDialog,
    SettingsSection, Switch, ThemeWords, setting_row, theme_settings_listed,
};

use crate::app::RecordsApp;

/// The themes the app carries, the only ones it offers.
pub const CARRIED: [&str; 7] = ["tidevann", "indigo", "paper", "contrast", "terminal", "steel", "ember"];

impl RecordsApp {
    /// The settings over the page while they are open, and the question before the demo's data
    /// goes back.
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        if let Some(mut active) = self.settings.take() {
            let sections = [
                SettingsSection::new("appearance", "Appearance", None),
                SettingsSection::new("forms", "Forms", None),
                SettingsSection::new("demo", "Demo server", None).reset(false),
            ];
            let mut answers = !self.backend.down;
            let (appearance, keep_place) = (&mut self.appearance, &mut self.keep_place);
            let mut reset = false;
            let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| match section {
                "appearance" => theme_settings_listed(ui, appearance, &CARRIED, true, &ThemeWords::default()),
                "forms" => setting_row(
                    ui,
                    "Keep the place chosen",
                    "The next form opens reporting from the place the last one was filed for.",
                    |ui| {
                        ui.add(Switch::new(keep_place, "Keep the place chosen"));
                    },
                ),
                _ => {
                    SettingRow::new(
                        "The server answers",
                        "Turn it off to see the app with a server that does not answer: the banner and the top bar say so, pages keep what they read before, and nothing can be filed, sent on, changed or asked of the API.",
                    )
                    .keywords(&["offline", "outage"])
                    .show(ui, |ui| {
                        ui.add(Switch::new(&mut answers, "The server answers"));
                    });
                    setting_row(
                        ui,
                        "Reset the demo data",
                        "Puts back the responses and households the demo starts with.",
                        |ui| {
                            let button = Button::new("Reset").variant(ButtonVariant::Outline).size(ButtonSize::Sm);
                            if ui.add(button).clicked() {
                                reset = true;
                            }
                        },
                    );
                }
            });
            if answers == self.backend.down {
                self.backend.down = !answers;
                self.check_server();
            }
            if outcome.reset {
                match active.as_str() {
                    "appearance" => self.appearance = Appearance::default(),
                    "forms" => self.keep_place = true,
                    _ => {}
                }
            }
            if reset {
                self.resetting = true;
            }
            if !outcome.close {
                self.settings = Some(active);
            }
        }
        if self.resetting {
            let question = ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                .detail("Everything changed since the demo started is put back, and the page loads again.")
                .destructive(true);
            match question.show(ctx) {
                Some(ConfirmAnswer::Confirm) => self.reset_demo(),
                Some(_) => self.resetting = false,
                None => {}
            }
        }
    }
}
