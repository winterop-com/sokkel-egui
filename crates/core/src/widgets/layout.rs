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
