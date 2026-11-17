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
    /// 6.
    Xs,
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
            Gap::Xs => 6.0,
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
            ui.allocate_ui_with_layout(vec2(room - side - gap, f32::INFINITY), down, |ui| {
                ui.set_width(room - side - gap);
                main(ui);
            });
            ui.allocate_ui_with_layout(vec2(side, f32::INFINITY), down, |ui| {
                ui.set_width(side);
                aside(ui);
            });
        });
    }
}

/// Two panes side by side with a rule between them that drags to share the room, after sokkel-ui's
/// `SplitPane`: the first holding `default_share` of the width, kept between `min_share` and
/// `max_share`, the rule (`border-strong`) dragged or double-clicked back to the default. Both
/// panes fill the height they are given. Narrower than `narrow_below` (640) the panes stack, the
/// first over the second, each half the height.
pub struct SplitPane {
    id: egui::Id,
    default_share: f32,
    min_share: f32,
    max_share: f32,
    narrow_below: f32,
    scroll: bool,
}

impl SplitPane {
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id: egui::Id::new(("sokkel-split-pane", id_salt)),
            default_share: 0.5,
            min_share: 0.2,
            max_share: 0.8,
            narrow_below: 640.0,
            scroll: false,
        }
    }

    /// Each pane scrolls what runs past its height, as the web's panes do around a block of their own.
    pub fn scroll(mut self, scroll: bool) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn default_share(mut self, share: f32) -> Self {
        self.default_share = share;
        self
    }

    pub fn min_share(mut self, share: f32) -> Self {
        self.min_share = share;
        self
    }

    pub fn max_share(mut self, share: f32) -> Self {
        self.max_share = share;
        self
    }

    pub fn narrow_below(mut self, width: f32) -> Self {
        self.narrow_below = width;
        self
    }

    pub fn show(self, ui: &mut Ui, first: impl FnOnce(&mut Ui), second: impl FnOnce(&mut Ui)) {
        let t = crate::tokens(ui.ctx());
        let room = ui.available_rect_before_wrap();
        let rule = t.color("border-strong");
        let pane = |ui: &mut Ui, rect: egui::Rect, salt: &str, add: Box<dyn FnOnce(&mut Ui) + '_>| {
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt((self.id, salt))
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.set_clip_rect(rect.intersect(ui.clip_rect()));
            if self.scroll {
                egui::ScrollArea::vertical()
                    .id_salt((self.id, salt, "scroll"))
                    .auto_shrink([false, false])
                    .show(&mut child, |ui| add(ui));
            } else {
                add(&mut child);
            }
        };
        if room.width() < self.narrow_below {
            let half = room.height() / 2.0;
            let top = egui::Rect::from_min_size(room.min, vec2(room.width(), half));
            let bottom = egui::Rect::from_min_max(egui::pos2(room.left(), top.bottom() + 1.0), room.max);
            pane(ui, top, "first", Box::new(first));
            ui.painter()
                .hline(room.x_range(), top.bottom() + 0.5, egui::Stroke::new(1.0, rule));
            pane(ui, bottom, "second", Box::new(second));
            ui.allocate_rect(room, egui::Sense::hover());
            return;
        }
        let clamp = |share: f32| share.clamp(self.min_share, self.max_share);
        let mut share = clamp(ui.data(|d| d.get_temp::<f32>(self.id)).unwrap_or(self.default_share));
        // the rule's handle: 8 wide over the rule, dragged to share the room
        let at = room.left() + (room.width() * share).round();
        let handle = egui::Rect::from_center_size(egui::pos2(at, room.center().y), vec2(8.0, room.height()));
        let response = ui
            .interact(handle, self.id.with("handle"), egui::Sense::click_and_drag())
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Resize"));
        if response.dragged()
            && let Some(pos) = response.interact_pointer_pos()
        {
            share = clamp((pos.x - room.left()) / room.width());
        }
        if response.double_clicked() {
            share = clamp(self.default_share);
        }
        ui.data_mut(|d| d.insert_temp(self.id, share));
        let at = room.left() + (room.width() * share).round();
        let left = egui::Rect::from_min_max(room.min, egui::pos2(at, room.bottom()));
        let right = egui::Rect::from_min_max(egui::pos2(at + 1.0, room.top()), room.max);
        pane(ui, left, "first", Box::new(first));
        pane(ui, right, "second", Box::new(second));
        let ink = if response.hovered() || response.dragged() {
            t.color("ring")
        } else {
            rule
        };
        ui.painter()
            .vline(at + 0.5, room.y_range(), egui::Stroke::new(1.0, ink));
        ui.allocate_rect(room, egui::Sense::hover());
    }
}
