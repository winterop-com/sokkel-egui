//! Laying parts out in a page's room, after sokkel-ui's `Layout` parts.

use egui::{Ui, vec2};

/// Columns of equal width, as many as `count` while each keeps at least `min` (the window's room
/// otherwise), `gap` apart across and down, after sokkel-ui's `Columns`.
pub struct Columns {
    count: usize,
    min: f32,
    gap: f32,
}

impl Columns {
    /// `count` columns, each at least 224 wide (`md`), 24 apart (`xl`), as the web's defaults.
    pub fn new(count: usize) -> Self {
        Self {
            count: count.max(1),
            min: 224.0,
            gap: 24.0,
        }
    }

    /// The least width of a column: `xs` 120, `sm` 160, `md` 224, `lg` 288, `xl` 448.
    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    /// The room between columns and between rows: `sm` 8, `md` 12, `lg` 16, `xl` 24.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Lay `items` items out, `cell(ui, index, width)` drawing each in its column.
    pub fn show(self, ui: &mut Ui, items: usize, mut cell: impl FnMut(&mut Ui, usize, f32)) {
        let room = ui.available_width();
        let fits = ((room + self.gap) / (self.min.min(room) + self.gap)).floor().max(1.0) as usize;
        let columns = self.count.min(fits).max(1);
        let width = (room - self.gap * (columns - 1) as f32) / columns as f32;
        let gap = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing.y = self.gap;
        for start in (0..items).step_by(columns) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = self.gap;
                for index in start..(start + columns).min(items) {
                    ui.allocate_ui(vec2(width, 0.0), |ui| {
                        ui.set_width(width);
                        cell(ui, index, width);
                    });
                }
            });
        }
        ui.spacing_mut().item_spacing = gap;
    }
}

/// The room between the parts of a `Stack`, as sokkel-ui's `gap`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gap {
    /// The density's page gap, between a page's parts.
    Page,
    /// The density's section gap, between the parts of one section.
    Section,
    /// 8.
    Sm,
    /// 12.
    Md,
    /// 16.
    Lg,
    /// 24.
    Xl,
}

impl Gap {
    pub fn length(self, tokens: &crate::Tokens) -> f32 {
        match self {
            Gap::Page => tokens.density("page-gap"),
            Gap::Section => tokens.density("section-gap"),
            Gap::Sm => 8.0,
            Gap::Md => 12.0,
            Gap::Lg => 16.0,
            Gap::Xl => 24.0,
        }
    }
}

/// Parts one under another, `gap` apart, after sokkel-ui's `Stack`.
pub struct Stack {
    gap: Gap,
}

impl Stack {
    pub fn new(gap: Gap) -> Self {
        Self { gap }
    }

    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
        let gap = self.gap.length(&crate::tokens(ui.ctx()));
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = gap;
            content(ui)
        })
        .inner
    }
}

/// How wide a `Split`'s aside stands: `sm` 256, `md` 320, `lg` 384.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsideWidth {
    Sm,
    Md,
    Lg,
}

/// The page's main part beside an aside at its end, after sokkel-ui's `Split`: side by side, 24
/// apart, from 768 of room; one under the other below it.
pub struct Split {
    aside: AsideWidth,
}

impl Split {
    pub fn new(aside: AsideWidth) -> Self {
        Self { aside }
    }

    pub fn show(self, ui: &mut Ui, main: impl FnOnce(&mut Ui), aside: impl FnOnce(&mut Ui)) {
        let room = ui.available_width();
        let side = match self.aside {
            AsideWidth::Sm => 256.0,
            AsideWidth::Md => 320.0,
            AsideWidth::Lg => 384.0,
        };
        let gap = 24.0;
        if room < 768.0 {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = gap;
                main(ui);
                aside(ui);
            });
            return;
        }
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            // each side lays its parts one under another, as wide as it is given
            let down = egui::Layout::top_down(egui::Align::Min);
            ui.allocate_ui_with_layout(vec2(room - side - gap, ui.available_height()), down, |ui| {
                ui.set_width(room - side - gap);
                main(ui);
            });
            ui.allocate_ui_with_layout(vec2(side, ui.available_height()), down, |ui| {
                ui.set_width(side);
                aside(ui);
            });
        });
    }
}
