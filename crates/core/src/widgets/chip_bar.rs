//! A row of chips with one chosen, after sokkel-ui's `ChipBar`.

use egui::{Rect, Ui, pos2, vec2};

use crate::{Button, ButtonSize, ButtonVariant, Icon, IconButton, tokens};

/// The words a chip bar says: the buttons that scroll the row a page back and forward.
#[derive(Clone, Debug)]
pub struct ChipBarWords {
    pub back: String,
    pub forward: String,
}

impl Default for ChipBarWords {
    fn default() -> Self {
        Self {
            back: "Scroll back".into(),
            forward: "Scroll forward".into(),
        }
    }
}

/// A row of chips with one chosen, such as the regions over a map: each a small button, the chosen
/// one in the primary look and the rest secondary, 8 apart. When they run past the row's room the
/// row scrolls sideways with no scroll bar, and an arrow at each side that has more, on the page's
/// ground, scrolls it a page that way.
pub struct ChipBar<'a> {
    id_salt: &'a str,
    chips: &'a [(&'a str, &'a str)],
    words: ChipBarWords,
}

impl<'a> ChipBar<'a> {
    /// `chips` are each a value and its label.
    pub fn new(id_salt: &'a str, chips: &'a [(&'a str, &'a str)]) -> Self {
        Self {
            id_salt,
            chips,
            words: ChipBarWords::default(),
        }
    }

    pub fn words(mut self, words: ChipBarWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the row; returns true when a chip was chosen, `value` then holding it.
    pub fn show(self, ui: &mut Ui, value: &mut String) -> bool {
        let t = tokens(ui.ctx());
        let mut changed = false;
        let id = ui.make_persistent_id(("sokkel-chip-bar", self.id_salt));
        // a page asked for by an arrow last frame
        let paged: Option<f32> = ui.data_mut(|d| d.remove_temp(id.with("page")));
        // a row of chips tall, however tall the room it stands in
        let tall = t.density("control-sm") + 4.0;
        let mut area = egui::ScrollArea::horizontal()
            .id_salt(id)
            .max_height(tall)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
        if let Some(offset) = paged {
            area = area.horizontal_scroll_offset(offset);
        }
        let out = area.show(ui, |ui| {
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
        let row = out.inner_rect;
        let offset = out.state.clamped_offset().x;
        let end = (out.content_size.x - row.width()).max(0.0);
        let page = row.width() * 0.8;
        let side = t.density("control-sm");
        let arrow = |ui: &mut Ui, at: Rect, icon: Icon, label: &str, to: f32| {
            ui.painter().rect_filled(at, 0.0, t.color("background"));
            if ui.put(at, IconButton::new(icon, label)).clicked() {
                ui.data_mut(|d| d.insert_temp(id.with("page"), to.clamp(0.0, end)));
                ui.ctx().request_repaint();
            }
        };
        if offset > 1.0 {
            let at = Rect::from_min_size(pos2(row.left(), row.center().y - side / 2.0), vec2(side, side));
            arrow(ui, at, Icon::ChevronLeft, &self.words.back, offset - page);
        }
        if offset < end - 1.0 {
            let at = Rect::from_min_size(pos2(row.right() - side, row.center().y - side / 2.0), vec2(side, side));
            arrow(ui, at, Icon::ChevronRight, &self.words.forward, offset + page);
        }
        changed
    }
}
