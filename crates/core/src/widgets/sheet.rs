//! One thing open beside a list, after sokkel-ui's `DetailSheet`: a sheet from the window's end over
//! a shade, its head naming the thing, its body scrolling, its actions at its foot.

use egui::{Context, Id, Order, Rect, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// The words a detail sheet says.
#[derive(Clone, Debug)]
pub struct DetailSheetWords {
    pub close: String,
    pub resize: String,
}

impl Default for DetailSheetWords {
    fn default() -> Self {
        Self {
            close: "Close".into(),
            resize: "Resize the panel".into(),
        }
    }
}

type Part<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A sheet from the window's end over a shade: its head with the code over the title, a line of
/// chips and actions, and a close button; its body scrolling, its parts `section-gap` apart; a foot
/// of actions at the end. Its start edge drags it wider or narrower, between 384 and nine tenths of
/// the window. Escape or a press on the shade closes it.
pub struct DetailSheet<'a> {
    id: Id,
    title: String,
    code: Option<String>,
    description: Option<String>,
    size: &'a mut f32,
    line: Option<Part<'a>>,
    foot: Option<Part<'a>>,
    words: DetailSheetWords,
}

impl<'a> DetailSheet<'a> {
    /// `size` is its width, which a drag of its edge changes.
    pub fn new(id: impl egui::AsIdSalt, title: impl Into<String>, size: &'a mut f32) -> Self {
        Self {
            id: Id::unique(id),
            title: title.into(),
            code: None,
            description: None,
            size,
            line: None,
            foot: None,
            words: DetailSheetWords::default(),
        }
    }

    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The line of chips and actions under the title.
    pub fn line(mut self, line: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.line = Some(Box::new(line));
        self
    }

    /// The actions at the sheet's foot, laid from its end.
    pub fn foot(mut self, foot: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.foot = Some(Box::new(foot));
        self
    }

    pub fn words(mut self, words: DetailSheetWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the sheet over everything; true when it is asked to close.
    pub fn show(self, ctx: &Context, body: impl FnOnce(&mut Ui)) -> bool {
        let t = tokens(ctx);
        let screen = ctx.content_rect();
        let most = (screen.width() * 0.9).max(384.0);
        let width = self.size.clamp(384.0, most).min(screen.width());
        let mut close = false;
        // the shade over the page, a press on it closing the sheet
        egui::Area::new(self.id.with("shade"))
            .order(Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (shade, hit) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(shade, 0.0, t.color_at("foreground", 0.2));
                if hit.clicked() {
                    close = true;
                }
            });
        let rect = Rect::from_min_max(pos2(screen.right() - width, screen.top()), screen.max);
        egui::Area::new(self.id.with("sheet"))
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.set_min_size(rect.size());
                ui.set_max_size(rect.size());
                ui.painter().rect_filled(rect, 0.0, t.color("popover"));
                ui.painter()
                    .vline(rect.left(), rect.y_range(), Stroke::new(1.0, t.color("border")));
                // the edge to drag, 8 wide over the sheet's start
                let edge = Rect::from_min_max(
                    pos2(rect.left() - 4.0, rect.top()),
                    pos2(rect.left() + 4.0, rect.bottom()),
                );
                let grip = ui.interact(edge, self.id.with("edge"), Sense::drag());
                grip.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Slider, true, &self.words.resize));
                if grip.hovered() || grip.dragged() {
                    ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }
                if grip.dragged() {
                    *self.size = (width - grip.drag_delta().x).clamp(384.0, most);
                }
                ui.spacing_mut().item_spacing.y = 0.0;
                // the head: 16 across, 12 over and under, a rule under it
                let inner = rect.width() - 32.0 - 36.0;
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        ui.set_width(rect.width() - 32.0);
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width(inner);
                                ui.spacing_mut().item_spacing.y = 6.0;
                                if let Some(code) = &self.code {
                                    let mono = crate::mono_font(
                                        ui.ctx(),
                                        crate::mono_small_size(&t, TextRole::Help.font(&t).size),
                                        400.0,
                                    );
                                    let galley = WidgetText::from(egui::RichText::new(code).font(mono.clone()))
                                        .into_galley(ui, Some(TextWrapMode::Truncate), inner, mono);
                                    ui.label(galley);
                                }
                                ui.add(
                                    egui::Label::new(
                                        TextRole::DetailTitle
                                            .rich(&t, &self.title)
                                            .color(t.color("popover-foreground")),
                                    )
                                    .wrap(),
                                );
                                if let Some(description) = &self.description {
                                    ui.add(
                                        egui::Label::new(
                                            TextRole::Body.rich(&t, description).color(t.color("muted-foreground")),
                                        )
                                        .wrap(),
                                    );
                                }
                                if let Some(line) = self.line {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 6.0;
                                        line(ui);
                                    });
                                }
                            });
                            let (slot, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                            let hit = ui.interact(slot, self.id.with("close"), Sense::click());
                            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.close));
                            if hit.hovered() {
                                ui.painter().rect_filled(slot, t.radius("control"), t.color("hover"));
                            }
                            icons::paint(ui.painter(), slot.center(), Icon::X, 16.0, t.color("muted-foreground"));
                            super::focus_ring(ui.painter(), &t, slot, t.radius("control"), &hit);
                            if hit.clicked() {
                                close = true;
                            }
                        });
                    });
                let rule = ui.available_rect_before_wrap();
                ui.painter()
                    .hline(rect.x_range(), rule.top(), Stroke::new(1.0, t.color("border")));
                let foot_height = if self.foot.is_some() {
                    t.density("control") + 16.0 + 1.0
                } else {
                    0.0
                };
                let room = (rect.bottom() - rule.top() - foot_height).max(0.0);
                egui::ScrollArea::vertical()
                    .id_salt(self.id.with("body"))
                    .max_height(room)
                    .min_scrolled_height(room)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::new().inner_margin(16).show(ui, |ui| {
                            ui.set_width(rect.width() - 32.0);
                            ui.spacing_mut().item_spacing.y = t.density("section-gap");
                            body(ui);
                        });
                    });
                if let Some(foot) = self.foot {
                    let band = Rect::from_min_max(pos2(rect.left(), rect.bottom() - foot_height), rect.max);
                    ui.painter()
                        .hline(band.x_range(), band.top() + 0.5, Stroke::new(1.0, t.color("border")));
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(band.shrink2(vec2(16.0, 8.0)))
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                    );
                    child.spacing_mut().item_spacing.x = 8.0;
                    foot(&mut child);
                }
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        close
    }
}
