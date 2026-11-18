//! The strip across the top of a pane, after sokkel-ui's `PaneHeader`.

use egui::{Align, CornerRadius, Layout, Response, Sense, Stroke, TextWrapMode, Ui, WidgetText, vec2};

use crate::{TextRole, tokens};

/// Where a pane's head stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PaneHeaderVariant {
    /// On the pane's own ground, over a rule in the border's color.
    #[default]
    Plain,
    /// The head of a boxed section, on the section heads' ground over their rule.
    Panel,
}

/// A title drawn by the caller, such as a name renamed in place.
pub type PaneTitle<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The strip across the top of a pane, with a rule under it, as sokkel-ui's `PaneHeader`: at least
/// `panel-head` tall, `panel-pad` at its sides, the pane's title in the heading face and the section
/// heads' ink (4 over and under it), a quieter fact beside it, and actions 4 apart at its end. A
/// pane of tools alone leaves the title out.
pub struct PaneHeader<'a> {
    title: Option<String>,
    title_ui: Option<PaneTitle<'a>>,
    meta: Option<String>,
    variant: PaneHeaderVariant,
}

impl Default for PaneHeader<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> PaneHeader<'a> {
    pub fn new() -> Self {
        Self {
            title: None,
            title_ui: None,
            meta: None,
            variant: PaneHeaderVariant::Plain,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// A title the caller draws, such as an `InlineEdit` at the heading's weight
    /// (`section_title_weight`), in place of `title`.
    pub fn title_with(mut self, title: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.title_ui = Some(Box::new(title));
        self
    }

    /// A count or a short fact beside the title, quieter than it.
    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    pub fn variant(mut self, variant: PaneHeaderVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn show(self, ui: &mut Ui, actions: impl FnOnce(&mut Ui)) -> Response {
        let t = tokens(ui.ctx());
        let pad = t.density("panel-pad");
        let head = t.density("panel-head");
        let width = ui.available_width();
        let ground = ui.painter().add(egui::Shape::Noop);
        let spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let inner = ui.allocate_ui_with_layout(vec2(width, head), Layout::left_to_right(Align::Center), |ui| {
            ui.set_min_size(vec2(width, head));
            ui.add_space(pad);
            let titled = self.title.is_some() || self.title_ui.is_some();
            if titled {
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if let Some(title) = self.title_ui {
                            title(ui);
                        } else if let Some(title) = &self.title {
                            let role = TextRole::SectionTitle;
                            let ink = t.color_or("section-head-ink", t.color("foreground"));
                            let galley = WidgetText::from(role.rich(&t, title)).into_galley(
                                ui,
                                Some(TextWrapMode::Truncate),
                                ui.available_width(),
                                role.font(&t),
                            );
                            let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                            ui.painter().galley(rect.min, galley, ink);
                        }
                        if let Some(meta) = &self.meta {
                            ui.label(TextRole::Help.rich(&t, meta).color(t.color("muted-foreground")));
                        }
                    });
                    ui.add_space(4.0);
                });
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(pad);
                ui.spacing_mut().item_spacing.x = 4.0;
                actions(ui);
            });
        });
        ui.spacing_mut().item_spacing = spacing;
        let rect = inner.response.rect;
        let (fill, line) = match self.variant {
            PaneHeaderVariant::Plain => (None, t.color("border")),
            PaneHeaderVariant::Panel => (
                Some(t.color_or("section-head-ground", t.color("muted"))),
                t.color_or("section-head-line", t.color("border")),
            ),
        };
        if let Some(fill) = fill {
            ui.painter()
                .set(ground, egui::Shape::rect_filled(rect, CornerRadius::ZERO, fill));
        }
        ui.painter()
            .hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, line));
        inner.response
    }
}

/// The weight a pane's title is set at, for a title the caller draws to match it.
pub fn section_title_weight(ui: &Ui) -> f32 {
    tokens(ui.ctx()).text(TextRole::SectionTitle.key()).weight
}
