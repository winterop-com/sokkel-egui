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
                        let mono = crate::mono_font(
                            ui.ctx(),
                            crate::mono_small_size(&t, TextRole::Help.font(&t).size),
                            400.0,
                        );
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

/// A side panel's content, after sokkel-ui's `SidePanel`, filling the shell's panel column: its
/// head as tall as the top bar with a rule under it, holding its tabs (or the one tab's title), 48
/// clear at its end for the shell's toggle; under it the chosen tab's content, scrolling. When the
/// tabs' labels do not fit that room and every tab has an icon, each tab stands as its icon, its
/// label in a tooltip; tabs without icons keep their labels and scroll sideways.
pub struct SidePanel<'a> {
    id: egui::Id,
    tabs: &'a [&'a str],
    icons: &'a [Icon],
    active: &'a mut usize,
}

impl<'a> SidePanel<'a> {
    pub fn new(id: impl egui::AsIdSalt, tabs: &'a [&'a str], active: &'a mut usize) -> Self {
        Self {
            id: egui::Id::unique(id),
            tabs,
            icons: &[],
            active,
        }
    }

    /// Each tab's icon, in the tabs' order: the tabs stand as these when their labels do not fit.
    pub fn icons(mut self, icons: &'a [Icon]) -> Self {
        self.icons = icons;
        self
    }

    /// Draws the panel, `content` asked for the chosen tab by its index.
    pub fn show(self, ui: &mut Ui, content: impl FnOnce(usize, &mut Ui)) {
        let t = tokens(ui.ctx());
        let room = ui.available_rect_before_wrap();
        let top = t.length("spacing-shell-top");
        let head = Rect::from_min_size(room.min, vec2(room.width(), top));
        // the content stands right under the head's rule
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.allocate_rect(head, Sense::hover());
        ui.painter().hline(
            head.x_range(),
            head.bottom() - 0.5,
            Stroke::new(1.0, t.color("border-strong")),
        );
        if self.tabs.len() == 1 {
            let galley = WidgetText::from(TextRole::Body.rich(&t, self.tabs[0]).variation("wght", 500.0)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                head.width() - 64.0,
                TextRole::Body.font(&t),
            );
            ui.painter().galley(
                pos2(head.left() + 14.0, head.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        } else {
            // the tabs take the head's room before the toggle's 48, 8 in from the start
            let room_rect = Rect::from_min_max(
                pos2(head.left() + 8.0, head.top()),
                pos2(head.right() - 48.0, head.bottom()),
            );
            let label_galley = |ui: &Ui, label: &str, on: bool| {
                let rich = if on {
                    TextRole::Body.rich(&t, label).variation("wght", 500.0)
                } else {
                    TextRole::Body.rich(&t, label)
                };
                WidgetText::from(rich).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextRole::Body.font(&t),
                )
            };
            let labels_width = self
                .tabs
                .iter()
                .map(|label| label_galley(ui, label, false).size().x + 20.0)
                .sum::<f32>()
                + 4.0 * self.tabs.len().saturating_sub(1) as f32;
            let as_icons = labels_width > room_rect.width() && self.icons.len() == self.tabs.len();
            let mut strip = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(room_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            egui::ScrollArea::horizontal()
                .id_salt(self.id.with("tabs"))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .auto_shrink([false, true])
                .show(&mut strip, |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.set_min_height(room_rect.height());
                    for (index, label) in self.tabs.iter().enumerate() {
                        let on = *self.active == index;
                        let galley = (!as_icons).then(|| label_galley(ui, label, on));
                        let width = galley.as_ref().map_or(32.0, |g| g.size().x + 20.0);
                        let (tab, hit) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
                        hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, on, *label));
                        let ink = if on || hit.hovered() {
                            t.color("foreground")
                        } else {
                            t.color("muted-foreground")
                        };
                        if on {
                            ui.painter().rect_filled(tab, t.radius("md"), t.color("secondary"));
                        } else if hit.hovered() {
                            ui.painter().rect_filled(tab, t.radius("md"), t.color("hover"));
                        }
                        let hit = match galley {
                            Some(galley) => {
                                ui.painter().galley(
                                    pos2(tab.left() + 10.0, tab.center().y - galley.size().y / 2.0),
                                    galley,
                                    ink,
                                );
                                hit
                            }
                            None => {
                                icons::paint(ui.painter(), tab.center(), self.icons[index], 16.0, ink);
                                super::tooltip(hit, *label)
                            }
                        };
                        if hit.clicked() {
                            *self.active = index;
                        }
                    }
                });
        }
        let active = (*self.active).min(self.tabs.len().saturating_sub(1));
        egui::ScrollArea::vertical()
            .id_salt(self.id.with(("body", active)))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(room.width());
                ui.spacing_mut().item_spacing.y = 0.0;
                content(active, ui);
            });
    }
}

/// A titled block inside a side panel, after sokkel-ui's `PanelSection`: a quiet capital heading
/// over its content, 8 apart, 16 in and 12 over and under, a rule under it.
pub fn panel_section(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    panel_section_actions(ui, title, |_| {}, body);
}

/// A [`panel_section`] with controls at its heading's end, as the web's `PanelSection` `actions`:
/// a small button such as Clear.
pub fn panel_section_actions(ui: &mut Ui, title: &str, actions: impl FnOnce(&mut Ui), body: impl FnOnce(&mut Ui)) {
    let t = tokens(ui.ctx());
    let drawn = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 8.0;
            // the heading's row is 20 tall, its words in the middle
            let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
            let mut head = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(row)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            group_label(&mut head, title);
            head.with_layout(egui::Layout::right_to_left(egui::Align::Center), actions);
            body(ui);
        })
        .response
        .rect;
    ui.painter().hline(
        drawn.x_range(),
        drawn.bottom() - 0.5,
        Stroke::new(1.0, t.color("border")),
    );
}

/// A quiet capital heading, after sokkel-ui's `GroupLabel`: the small words at medium weight in
/// capitals, tracked a little, in the faint ink.
pub fn group_label(ui: &mut Ui, title: &str) {
    let t = tokens(ui.ctx());
    ui.add(egui::Label::new(group_label_text(&t, title)).truncate());
}

/// A group label's words as rich text (text.xs at 500, in capitals, tracked 0.025em, in faint),
/// for a part that lays the label out itself, as the rail does.
pub fn group_label_text(t: &crate::Tokens, title: &str) -> egui::RichText {
    let spec = t.text("help");
    TextRole::Help
        .rich(t, title.to_uppercase())
        .variation("wght", 500.0)
        .extra_letter_spacing(spec.size * 0.025)
        .color(t.color("faint"))
}

/// A group label laid out in `width`, cut to one line, for a part that paints it where it goes.
pub fn group_label_galley(ui: &Ui, title: &str, width: f32) -> std::sync::Arc<egui::Galley> {
    let t = tokens(ui.ctx());
    egui::WidgetText::from(group_label_text(&t, title)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        width,
        TextRole::Help.font(&t),
    )
}
