//! A row of media, a title over a description and actions, after sokkel-ui's `Item` primitive.

use egui::{Align, Layout, Response, Sense, Stroke, TextWrapMode, Ui, UiBuilder, WidgetText, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// How roomy an item is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ItemSize {
    /// 12 across and 10 over and under, its parts 10 apart.
    #[default]
    Default,
    /// 10 across and 8 over and under, its parts 8 apart, the description small.
    Xs,
}

/// How an item stands out from what is around it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ItemVariant {
    #[default]
    Plain,
    /// Inside an edge in the border's color.
    Outline,
    /// On the muted ground at half strength.
    Muted,
}

/// A description the caller draws, such as a line of markdown with words marked.
pub type ItemDescription<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// One item, as sokkel-ui's `Item`: an icon in the muted ink, a title at medium weight over a
/// description in the muted ink (two lines at most), and actions at its end; the large radius and
/// a 1 edge, transparent unless `Outline`. A pressable item is a link: it takes the hover ground,
/// and its press comes back as the response's click. With a description the icon stands at the
/// top, 2 down.
pub struct Item<'a> {
    title: String,
    icon: Option<Icon>,
    description: Option<String>,
    description_ui: Option<ItemDescription<'a>>,
    size: ItemSize,
    variant: ItemVariant,
    pressable: bool,
}

impl<'a> Item<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            icon: None,
            description: None,
            description_ui: None,
            size: ItemSize::Default,
            variant: ItemVariant::Plain,
            pressable: false,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// A description the caller draws in place of `description`.
    pub fn description_with(mut self, description: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.description_ui = Some(Box::new(description));
        self
    }

    pub fn size(mut self, size: ItemSize) -> Self {
        self.size = size;
        self
    }

    pub fn variant(mut self, variant: ItemVariant) -> Self {
        self.variant = variant;
        self
    }

    /// A link: the hover ground, and a press.
    pub fn pressable(mut self, pressable: bool) -> Self {
        self.pressable = pressable;
        self
    }

    pub fn show(self, ui: &mut Ui, actions: impl FnOnce(&mut Ui)) -> Response {
        let t = tokens(ui.ctx());
        let (pad_x, pad_y, gap) = match self.size {
            ItemSize::Default => (12.0, 10.0, 10.0),
            ItemSize::Xs => (10.0, 8.0, 8.0),
        };
        let edge = 1.0;
        let width = ui.available_width();
        let ground = ui.painter().add(egui::Shape::Noop);
        let described = self.description.is_some() || self.description_ui.is_some();
        let sense = if self.pressable { Sense::click() } else { Sense::hover() };
        let inner = ui.scope_builder(UiBuilder::new().sense(sense), |ui| {
            ui.set_width(width);
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric((pad_x + edge) as i8, (pad_y + edge) as i8))
                .show(ui, |ui| {
                    ui.set_width(width - 2.0 * (pad_x + edge));
                    let align = if described { Align::Min } else { Align::Center };
                    ui.with_layout(Layout::left_to_right(align), |ui| {
                        ui.spacing_mut().item_spacing = vec2(gap, 0.0);
                        if let Some(icon) = self.icon {
                            let (slot, _) = ui.allocate_exact_size(
                                vec2(16.0, if described { 20.0 + 2.0 } else { 16.0 }),
                                Sense::hover(),
                            );
                            let center = if described {
                                egui::pos2(slot.center().x, slot.top() + 2.0 + 10.0)
                            } else {
                                slot.center()
                            };
                            icons::paint(ui.painter(), center, icon, 16.0, t.color("muted-foreground"));
                        }
                        // the actions first, from the end, so the words take the room they leave
                        let room = ui.available_rect_before_wrap();
                        let mut tail = ui.new_child(
                            UiBuilder::new()
                                .max_rect(room)
                                .layout(Layout::right_to_left(Align::Center)),
                        );
                        tail.spacing_mut().item_spacing.x = 8.0;
                        actions(&mut tail);
                        let taken = tail.min_rect().width();
                        let words = if taken > 1.0 {
                            room.width() - taken - gap
                        } else {
                            room.width()
                        };
                        ui.allocate_ui_with_layout(vec2(words, 0.0), Layout::top_down(Align::Min), |ui| {
                            ui.set_width(words);
                            ui.spacing_mut().item_spacing.y = if self.size == ItemSize::Xs { 0.0 } else { 4.0 };
                            let title = TextRole::Body
                                .rich(&t, &self.title)
                                .variation("wght", 500.0)
                                .line_height(Some((TextRole::Body.font(&t).size * 1.375).round()));
                            let galley = WidgetText::from(title).into_galley(
                                ui,
                                Some(TextWrapMode::Truncate),
                                words,
                                TextRole::Body.font(&t),
                            );
                            let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                            ui.painter().galley(rect.min, galley, t.color("foreground"));
                            if let Some(describe) = self.description_ui {
                                describe(ui);
                            } else if let Some(description) = &self.description {
                                let role = if self.size == ItemSize::Xs {
                                    TextRole::Help
                                } else {
                                    TextRole::Body
                                };
                                let mut job = egui::text::LayoutJob::default();
                                role.rich(&t, description).append_to(
                                    &mut job,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    Align::Min,
                                );
                                job.wrap = egui::text::TextWrapping {
                                    max_width: words,
                                    max_rows: 2,
                                    break_anywhere: false,
                                    overflow_character: Some('…'),
                                };
                                let galley = ui.painter().layout_job(job);
                                let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                                ui.painter().galley(rect.min, galley, t.color("muted-foreground"));
                            }
                        });
                    });
                });
        });
        let response = inner.response;
        let rect = response.rect;
        let radius = t.radius("lg");
        let fill = match self.variant {
            ItemVariant::Muted => t.color_at("muted", 0.5),
            _ if self.pressable && response.hovered() => t.color("hover"),
            _ => egui::Color32::TRANSPARENT,
        };
        let stroke = match self.variant {
            ItemVariant::Outline => Stroke::new(edge, t.color("border")),
            _ => Stroke::NONE,
        };
        ui.painter().set(ground, egui::Shape::rect_filled(rect, radius, fill));
        if stroke != Stroke::NONE {
            ui.painter().rect_stroke(rect, radius, stroke, egui::StrokeKind::Inside);
        }
        if self.pressable {
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, &self.title));
        }
        response
    }
}

/// Items set against each other, as sokkel-ui's `ItemGroup` with `flush`, for a list whose items
/// need no room between them.
pub fn item_group<R>(ui: &mut Ui, items: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        items(ui)
    })
    .inner
}
