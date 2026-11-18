//! The page for an address none of an app's pages answers, after sokkel-ui's `NotFound`.

use egui::Ui;

use super::{Button, ButtonSize, ButtonVariant, Empty};

/// What a not-found page says.
#[derive(Clone, Debug)]
pub struct NotFoundWords {
    pub title: String,
    /// The line under the title: `There is no page at {address}.`
    pub detail: String,
    /// The way back to the app's first page.
    pub home: String,
}

impl Default for NotFoundWords {
    fn default() -> Self {
        Self {
            title: "Nothing at this address".into(),
            detail: "There is no page at {address}.".into(),
            home: "Go home".into(),
        }
    }
}

/// The page an app draws for an address none of its pages answers, in place of its first page as if
/// nothing were wrong: an empty state saying what the address was, and an outline button back to the
/// first page. `show` is true on the frame the button is pressed; the app goes home.
pub struct NotFound {
    address: String,
    words: NotFoundWords,
}

impl NotFound {
    /// `address` is the address that leads nowhere, as the reader sees it, such as `#/jobs/gone`.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            words: NotFoundWords::default(),
        }
    }

    pub fn words(mut self, words: NotFoundWords) -> Self {
        self.words = words;
        self
    }

    /// Draw it; true when the way home is pressed.
    pub fn show(self, ui: &mut Ui) -> bool {
        Empty::new(self.words.title)
            .description(self.words.detail.replace("{address}", &self.address))
            .show(ui, |ui| {
                ui.add(
                    Button::new(self.words.home)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm),
                )
                .clicked()
            })
    }
}
