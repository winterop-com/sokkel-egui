//! A row of chips with one chosen, after sokkel-ui's `ChipBar`.

use egui::{Ui, vec2};

use crate::{Button, ButtonSize, ButtonVariant};

/// A row of chips with one chosen, such as the regions over a map: each a small button, the chosen
/// one in the primary look and the rest secondary, 8 apart. When they run past the row's room the
/// row scrolls sideways with no scroll bar.
pub struct ChipBar<'a> {
    id_salt: &'a str,
    chips: &'a [(&'a str, &'a str)],
}

impl<'a> ChipBar<'a> {
    /// `chips` are each a value and its label.
    pub fn new(id_salt: &'a str, chips: &'a [(&'a str, &'a str)]) -> Self {
        Self { id_salt, chips }
    }

    /// Draws the row; returns true when a chip was chosen, `value` then holding it.
    pub fn show(self, ui: &mut Ui, value: &mut String) -> bool {
        let mut changed = false;
        egui::ScrollArea::horizontal()
            .id_salt(("sokkel-chip-bar", self.id_salt))
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
                    for (chip, label) in self.chips {
                        let chosen = value == chip;
                        let button = Button::new(*label).size(ButtonSize::Sm).variant(if chosen {
                            ButtonVariant::Default
                        } else {
                            ButtonVariant::Secondary
                        });
                        if ui.add(button).clicked() && !chosen {
                            *value = (*chip).to_owned();
                            changed = true;
                        }
                    }
                });
            });
        changed
    }
}
