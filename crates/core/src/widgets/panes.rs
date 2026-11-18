//! Panes beside a page: a column to move about in, after sokkel-ui's `SubNav`, and a panel of the
//! chosen thing's facts, after its `Inspector`.

use egui::{Rect, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// A column at a page's start to move about in, after sokkel-ui's `SubNav`: the sidebar's ground,
/// a strong rule at its end, 8 across and 12 down, a row at its top (such as a search) and what it
/// lists under it, 12 apart. Its end edge drags it wider or narrower.
pub struct SubNav<'a> {
    id: egui::Id,
    size: &'a mut f32,
    min: f32,
    max: f32,
}

impl<'a> SubNav<'a> {
    /// `size` is its width, which a drag of its edge changes.
    pub fn new(id: impl egui::AsIdSalt, size: &'a mut f32) -> Self {
        Self {
            id: egui::Id::unique(id),
            size,
            min: 192.0,
            max: 400.0,
        }
    }

    /// Draws the column down `ui`'s whole height, `top` over `body`.
    pub fn show(self, ui: &mut Ui, top: impl FnOnce(&mut Ui), body: impl FnOnce(&mut Ui)) {
        let t = tokens(ui.ctx());
        let width = self.size.clamp(self.min, self.max);
        let room = ui.available_rect_before_wrap();
        let rect = Rect::from_min_size(room.min, vec2(width, room.height()));
        ui.allocate_rect(rect, Sense::hover());
        ui.painter().rect_filled(rect, 0.0, t.color("sidebar"));
        ui.painter().vline(
            rect.right() - 0.5,
            rect.y_range(),
            Stroke::new(1.0, t.color("border-strong")),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(vec2(8.0, 12.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.spacing_mut().item_spacing.y = 12.0;
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(4, 0))
            .show(&mut child, |ui| {
                ui.set_width(width - 16.0 - 8.0);
                top(ui);
            });
        egui::ScrollArea::vertical()
            .id_salt(self.id.with("list"))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.set_width(width - 16.0);
                body(ui);
            });
        let edge = Rect::from_min_max(
            pos2(rect.right() - 4.0, rect.top()),
            pos2(rect.right() + 4.0, rect.bottom()),
        );
        let grip = ui.interact(edge, self.id.with("edge"), Sense::drag());
        grip.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Slider, true, "Resize the column"));
        if grip.hovered() || grip.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if grip.dragged() {
            *self.size = (width + grip.drag_delta().x).clamp(self.min, self.max);
        }
    }
}

/// The words an inspector says.
#[derive(Clone, Debug)]
pub struct InspectorWords {
    pub copy: String,
    pub copied: String,
}

impl Default for InspectorWords {
    fn default() -> Self {
        Self {
            copy: "Copy".into(),
            copied: "Copied".into(),
        }
    }
}

/// A panel of the chosen thing's facts, after sokkel-ui's `Inspector`: on the card ground in a
/// strong edge, its head with the title, the code in the code face with a button that copies it,
/// and a line of status under them; its body scrolling, its sections the inspector gap apart.
pub struct Inspector {
    id: egui::Id,
    title: String,
    code: Option<String>,
    status: Option<String>,
    width: f32,
    words: InspectorWords,
}

impl Inspector {
    pub fn new(id: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self {
            id: egui::Id::unique(id),
            title: title.into(),
            code: None,
            status: None,
            width: 360.0,
            words: InspectorWords::default(),
        }
    }

    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Small muted words under the title, such as the thing's kind.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Draws the panel `height` tall.
    pub fn show(self, ui: &mut Ui, height: f32, body: impl FnOnce(&mut Ui)) {
        let t = tokens(ui.ctx());
        let pad = t.density("inspector-pad");
        let (rect, _) = ui.allocate_exact_size(vec2(self.width, height), Sense::hover());
        let radius = t.radius("surface");
        ui.painter().rect_filled(rect, radius, t.color("card"));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.spacing_mut().item_spacing.y = 0.0;
        egui::Frame::new()
            .inner_margin(egui::Margin::same(pad as i8))
            .show(&mut child, |ui| {
                ui.set_width(self.width - 2.0 * pad);
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.add(
                    egui::Label::new(
                        TextRole::DetailTitle
                            .rich(&t, &self.title)
                            .color(t.color("card-foreground")),
                    )
                    .wrap(),
                );
                if let Some(code) = &self.code {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
                        ui.label(egui::RichText::new(code).font(mono).color(t.color("muted-foreground")));
                        let copied_id = self.id.with("copied");
                        let now = ui.input(|i| i.time);
                        let copied = ui
                            .data(|d| d.get_temp::<f64>(copied_id))
                            .is_some_and(|at| now - at < 1.5);
                        let (slot, hit) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
                        let words = if copied { &self.words.copied } else { &self.words.copy };
                        hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, words));
                        if hit.hovered() {
                            ui.painter().rect_filled(slot, t.radius("md"), t.color("hover"));
                        }
                        icons::paint(
                            ui.painter(),
                            slot.center(),
                            if copied { Icon::Check } else { Icon::Copy },
                            12.0,
                            t.color("muted-foreground"),
                        );
                        if hit.clicked() {
                            ui.ctx().copy_text(code.clone());
                            ui.data_mut(|d| d.insert_temp(copied_id, now));
                        }
                    });
                }
                if let Some(status) = &self.status {
                    ui.add_space(10.0);
                    ui.label(TextRole::Help.rich(&t, status).color(t.color("muted-foreground")));
                }
            });
        let rule = child.cursor().top();
        child
            .painter()
            .hline(rect.x_range(), rule, Stroke::new(1.0, t.color("border")));
        egui::ScrollArea::vertical()
            .id_salt(self.id.with("body"))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::same(pad as i8))
                    .show(ui, |ui| {
                        ui.set_width(self.width - 2.0 * pad);
                        ui.spacing_mut().item_spacing.y = t.density("inspector-gap");
                        body(ui);
                    });
            });
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, t.color("border-strong")),
            egui::StrokeKind::Inside,
        );
    }
}

/// One part of an inspector's body, after sokkel-ui's `InspectorSection`: its title small and
/// semibold, and what it holds 12 under it.
pub fn inspector_section(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    let t = tokens(ui.ctx());
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 12.0;
        let galley = WidgetText::from(TextRole::Help.rich(&t, title).variation("wght", 600.0)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            ui.available_width(),
            TextRole::Help.font(&t),
        );
        ui.label(galley);
        body(ui);
    });
}
