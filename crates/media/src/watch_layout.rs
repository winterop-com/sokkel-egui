//! A watch page's layout, after sokkel-ui-media's `WatchLayout`.

use egui::{Align, Layout, Rect, Ui, UiBuilder, pos2, vec2};

/// How wide the layout must be for the aside to stand beside the player, as the web's
/// `@4xl/watch` (56rem).
const BESIDE: f32 = 896.0;
/// The gap between the columns and between the player and what is under it.
const GAP_X: f32 = 24.0;
const GAP_Y: f32 = 20.0;

type Part<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A watch page, after sokkel-ui-media's `WatchLayout`: the player, what stands under it (the
/// title, the channel, the description and the comments), and an aside beside both, such as what
/// plays next. 896 wide or more, the aside stands at the end, between 256 and 352 wide, and runs
/// down beside the player and what is under it, 24 from them; in `theater` the player spans both
/// columns, no taller than `theater_height` lets it be, and the aside starts under it. Narrower,
/// the three stack 20 apart, the aside last.
#[must_use = "show it with `.show(ui)`"]
pub struct WatchLayout<'a> {
    player: Part<'a>,
    body: Option<Part<'a>>,
    aside: Option<Part<'a>>,
    theater: bool,
    theater_height: Option<f32>,
}

impl<'a> WatchLayout<'a> {
    pub fn new(player: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self {
            player: Box::new(player),
            body: None,
            aside: None,
            theater: false,
            theater_height: None,
        }
    }

    /// What stands under the player.
    pub fn body(mut self, body: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.body = Some(Box::new(body));
        self
    }

    /// What stands beside the player and what is under it.
    pub fn aside(mut self, aside: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.aside = Some(Box::new(aside));
        self
    }

    /// The player across both columns, as tall as `height` lets it be at sixteen by nine.
    pub fn theater(mut self, theater: bool, height: f32) -> Self {
        self.theater = theater;
        self.theater_height = Some(height);
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let width = ui.available_width();
        // the room each column is given down the page: as tall as the page shows, growing past it
        let tall = ui.available_height().max(1.0);
        let top = ui.cursor().top();
        let left = ui.cursor().left();
        let beside = self.aside.is_some() && width >= BESIDE;
        if !beside {
            ui.spacing_mut().item_spacing.y = GAP_Y;
            let theater = self.theater_height.filter(|_| self.theater);
            column(ui, width, theater, self.player);
            if let Some(body) = self.body {
                body(ui);
            }
            if let Some(aside) = self.aside {
                aside(ui);
            }
            return;
        }
        // the aside: between 256 and 352 wide, the rest the player's
        let aside_width = (width - GAP_X - 256.0).clamp(256.0, 352.0);
        let main_width = width - GAP_X - aside_width;
        let child = |ui: &mut Ui, rect: Rect, part: Part<'a>, max: Option<f32>| -> f32 {
            let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)));
            column(&mut child, rect.width(), max, part);
            child.min_rect().bottom()
        };
        let (player_bottom, body_left, aside_top) = if self.theater {
            let max = self.theater_height;
            let bottom = child(
                ui,
                Rect::from_min_size(pos2(left, top), vec2(width, tall)),
                self.player,
                max,
            );
            (bottom, left, bottom + GAP_Y)
        } else {
            let bottom = child(
                ui,
                Rect::from_min_size(pos2(left, top), vec2(main_width, tall)),
                self.player,
                None,
            );
            (bottom, left, top)
        };
        let mut bottom = player_bottom;
        if let Some(body) = self.body {
            let b = child(
                ui,
                Rect::from_min_size(pos2(body_left, player_bottom + GAP_Y), vec2(main_width, tall)),
                body,
                None,
            );
            bottom = bottom.max(b);
        }
        if let Some(aside) = self.aside {
            let b = child(
                ui,
                Rect::from_min_size(pos2(left + main_width + GAP_X, aside_top), vec2(aside_width, tall)),
                aside,
                None,
            );
            bottom = bottom.max(b);
        }
        ui.allocate_rect(
            Rect::from_min_max(pos2(left, top), pos2(left + width, bottom)),
            egui::Sense::hover(),
        );
    }
}

/// One part in a column `width` wide; the player held to `max` tall at sixteen by nine, centered.
fn column(ui: &mut Ui, width: f32, max: Option<f32>, part: Part<'_>) {
    match max {
        Some(height) if width * 9.0 / 16.0 > height => {
            let narrow = height * 16.0 / 9.0;
            ui.horizontal(|ui| {
                ui.add_space((width - narrow) / 2.0);
                ui.allocate_ui_with_layout(vec2(narrow, height), Layout::top_down(Align::Min), |ui| {
                    ui.set_width(narrow);
                    part(ui);
                });
            });
        }
        _ => {
            ui.set_width(width);
            part(ui);
        }
    }
}
