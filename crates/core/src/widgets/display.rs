//! Display primitives: Badge, Card, Alert, Separator and Skeleton, after sokkel-ui's `ui/` set.

use egui::{
    Color32, CornerRadius, Frame, InnerResponse, Margin, Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Widget,
    WidgetText, pos2, vec2,
};

use crate::{Icon, TextRole, icons, tokens};

/// How a badge looks, as sokkel-ui's `Badge` `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeVariant {
    #[default]
    Default,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

/// A short label, 20 tall, in the chip corner: a count, a version, a kind.
#[must_use = "add it with `ui.add(...)`"]
pub struct Badge {
    label: String,
    variant: BadgeVariant,
    icon: Option<Icon>,
}

impl Badge {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: BadgeVariant::Default,
            icon: None,
        }
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// A 12px Lucide icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl Widget for Badge {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let none = Color32::TRANSPARENT;
        let (fill, border, ink) = match self.variant {
            BadgeVariant::Default => (t.color("primary"), none, t.color("primary-foreground")),
            BadgeVariant::Secondary => (t.color("secondary"), none, t.color("secondary-foreground")),
            BadgeVariant::Destructive => (
                t.color_at("destructive", if t.dark { 0.2 } else { 0.1 }),
                none,
                t.color("destructive"),
            ),
            BadgeVariant::Outline => (none, t.color("border"), t.color("foreground")),
            BadgeVariant::Ghost => (none, none, t.color("foreground")),
            BadgeVariant::Link => (none, none, t.color("primary")),
        };
        let role = TextRole::Badge;
        let galley = WidgetText::from(role.rich(&t, &self.label)).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            role.font(&t),
        );
        let lead = if self.icon.is_some() { 12.0 + 4.0 } else { 0.0 };
        let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + lead + 12.0, 20.0), Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &self.label));
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            painter.rect(
                rect,
                t.radius("chip"),
                fill,
                Stroke::new(1.0, border),
                StrokeKind::Inside,
            );
            let mut x = rect.left() + 6.0;
            if let Some(icon) = self.icon {
                icons::paint(painter, pos2(x + 6.0, rect.center().y), icon, 12.0, ink);
                x += lead;
            }
            painter.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, ink);
        }
        response
    }
}

/// A surface for one subject: a title and a description over its content, in the card ground
/// with the strong edge and the surface corner. `bar` sets the title in a head bar instead.
#[must_use = "show it with `.show(ui, |ui| ...)`"]
pub struct Card {
    title: Option<String>,
    description: Option<String>,
    bar: bool,
    footer: Option<String>,
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

impl Card {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            bar: false,
            footer: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The title in a head bar: the section head's ground and rule, the compact head role.
    pub fn bar(mut self, bar: bool) -> Self {
        self.bar = bar;
        self
    }

    /// A line of muted text in the footer strip under the content.
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = tokens(ui.ctx());
        let radius = t.radius("surface");
        Frame::new()
            .fill(t.color("card"))
            .stroke(Stroke::new(1.0, t.color("border-strong")))
            .corner_radius(radius)
            .show(ui, |ui| {
                // a card stacks its parts whatever layout the row around it has
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| self.parts(ui, add))
                    .inner
            })
    }

    fn parts<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
        let t = tokens(ui.ctx());
        let spacing = 16.0;
        let radius = t.radius("surface");
        let ink = t.color_or("section-head-ink", t.color("card-foreground"));
        {
            {
                ui.spacing_mut().item_spacing.y = 0.0;
                let width = ui.available_width();
                if self.bar {
                    let ground = t.color_or("section-head-ground", t.color("card"));
                    let line = t.color_or("section-head-line", t.color("border"));
                    let height = t.density("panel-head");
                    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
                    let top = CornerRadius { sw: 0, se: 0, ..radius };
                    ui.painter().rect_filled(rect, top, ground);
                    ui.painter()
                        .hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, line));
                    if let Some(title) = &self.title {
                        let galley = WidgetText::from(TextRole::Head.rich(&t, title)).into_galley(
                            ui,
                            Some(TextWrapMode::Truncate),
                            rect.width() - 2.0 * spacing,
                            TextRole::Head.font(&t),
                        );
                        let pos = pos2(rect.left() + spacing, rect.center().y - galley.size().y / 2.0);
                        ui.painter().galley(pos, galley, ink);
                    }
                }
                let inner = Frame::new()
                    .inner_margin(Margin::same(spacing as i8))
                    .show(ui, |ui| {
                        ui.set_width(width - 2.0 * spacing);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            if !self.bar
                                && let Some(title) = &self.title
                            {
                                ui.label(TextRole::SectionTitle.rich(&t, title).color(ink));
                            }
                            if let Some(description) = &self.description {
                                ui.label(TextRole::Body.rich(&t, description).color(t.color("muted-foreground")));
                            }
                            if self.title.is_some() && !self.bar || self.description.is_some() {
                                ui.add_space(spacing - 4.0);
                            }
                            ui.spacing_mut().item_spacing.y = 8.0;
                            add(ui)
                        })
                        .inner
                    })
                    .inner;
                if let Some(footer) = &self.footer {
                    let (rect, _) = ui.allocate_exact_size(vec2(width, 20.0 + 2.0 * spacing), Sense::hover());
                    let bottom = CornerRadius { nw: 0, ne: 0, ..radius };
                    ui.painter().rect_filled(rect, bottom, t.color("panel-ground"));
                    ui.painter()
                        .hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, t.color("border")));
                    ui.painter().text(
                        pos2(rect.left() + spacing, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        footer,
                        TextRole::Help.font(&t),
                        t.color("muted-foreground"),
                    );
                }
                inner
            }
        }
    }
}

/// A message that stays on the page: an icon, a title and a description, in the card ground.
#[must_use = "add it with `ui.add(...)`"]
pub struct Alert {
    title: String,
    description: Option<String>,
    icon: Option<Icon>,
    destructive: bool,
}

impl Alert {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            icon: None,
            destructive: false,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The destructive look: the title, icon and description in the destructive ink.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }
}

impl Widget for Alert {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (ink, muted) = if self.destructive {
            (t.color("destructive"), t.color_at("destructive", 0.9))
        } else {
            (t.color("card-foreground"), t.color("muted-foreground"))
        };
        Frame::new()
            .fill(t.color("card"))
            .stroke(Stroke::new(1.0, t.color("border")))
            .corner_radius(t.radius("surface"))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if let Some(icon) = self.icon {
                        let (rect, _) = ui.allocate_exact_size(vec2(16.0, 22.0), Sense::hover());
                        icons::paint(ui.painter(), pos2(rect.center().x, rect.top() + 12.0), icon, 16.0, ink);
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(TextRole::Action.rich(&t, &self.title).color(ink));
                        if let Some(description) = &self.description {
                            ui.label(TextRole::Body.rich(&t, description).color(muted));
                        }
                    });
                });
            })
            .response
    }
}

/// A 1px rule in the border color, across the row or, with `vertical`, down it.
#[must_use = "add it with `ui.add(...)`"]
pub struct Separator {
    vertical: bool,
}

impl Separator {
    pub fn horizontal() -> Self {
        Self { vertical: false }
    }

    pub fn vertical() -> Self {
        Self { vertical: true }
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let size = if self.vertical {
            vec2(1.0, ui.available_size_before_wrap().y.min(ui.spacing().interact_size.y))
        } else {
            vec2(ui.available_width(), 1.0)
        };
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::ZERO, t.color("border"));
        response
    }
}

/// A placeholder for content still loading: the muted ground, pulsing as `animate-pulse` does.
#[must_use = "add it with `ui.add(...)`"]
pub struct Skeleton {
    size: egui::Vec2,
    round: bool,
}

impl Skeleton {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            size: vec2(width, height),
            round: false,
        }
    }

    /// A circle, for an avatar still loading.
    pub fn round(mut self) -> Self {
        self.round = true;
        self
    }
}

impl Widget for Skeleton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());
        // opacity 1 to 0.5 and back over two seconds
        let phase = (ui.input(|i| i.time) as f32 * std::f32::consts::PI).cos() * 0.5 + 0.5;
        let alpha = 0.5 + 0.5 * phase;
        let radius = if self.round {
            CornerRadius::same(255)
        } else {
            t.radius("md")
        };
        ui.painter().rect_filled(rect, radius, t.color_at("muted", alpha));
        ui.ctx().request_repaint();
        response
    }
}

/// A banner's tone: what it says about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerTone {
    Critical,
    Warning,
    Info,
    Good,
}

/// A line across the page about how things stand, after sokkel-ui's `Banner`: its tone's icon and
/// words in the tone's ink, over the tone at 12% with a rule of it at 35% under it, 12 in and 6 over
/// and under.
pub struct Banner {
    tone: BannerTone,
    text: String,
}

impl Banner {
    pub fn new(tone: BannerTone, text: impl Into<String>) -> Self {
        Self {
            tone,
            text: text.into(),
        }
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let t = crate::tokens(ui.ctx());
        let (key, icon) = match self.tone {
            BannerTone::Critical => ("critical", crate::Icon::CircleAlert),
            BannerTone::Warning => ("warning", crate::Icon::TriangleAlert),
            BannerTone::Info => ("info", crate::Icon::Info),
            BannerTone::Good => ("good", crate::Icon::CircleCheck),
        };
        let ink = t.color(&format!("{key}-ink"));
        let width = ui.available_width();
        let words = egui::WidgetText::from(crate::TextRole::Body.rich(&t, &self.text)).into_galley(
            ui,
            Some(egui::TextWrapMode::Wrap),
            (width - 24.0 - 24.0).max(1.0),
            crate::TextRole::Body.font(&t),
        );
        let height = words.size().y.max(20.0) + 12.0;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        // what it says, for a screen reader
        let said = self.text.clone();
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &said));
        let painter = ui.painter();
        painter.rect_filled(rect, egui::CornerRadius::ZERO, t.color_at(key, 0.12));
        painter.hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            egui::Stroke::new(1.0, t.color_at(key, 0.35)),
        );
        crate::icons::paint(
            painter,
            egui::pos2(rect.left() + 12.0 + 8.0, rect.top() + 6.0 + 10.0),
            icon,
            16.0,
            ink,
        );
        painter.galley(egui::pos2(rect.left() + 12.0 + 24.0, rect.top() + 6.0), words, ink);
        response
    }
}
