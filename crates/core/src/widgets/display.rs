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

/// A short value as a chip, 20 tall, in the chip corner: a state, a version, a count, a tag,
/// after sokkel-ui's `Badge`. Its variant says how loud it is; a kind draws it in a kind color.
/// Drawn as a link (`href` or `link`) it takes focus, washes on hover and answers a click.
#[must_use = "add it with `ui.add(...)`"]
pub struct Badge {
    label: String,
    variant: BadgeVariant,
    kind: Option<&'static str>,
    icon: Option<Icon>,
    icon_after: bool,
    href: Option<String>,
    link: bool,
}

impl Badge {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: BadgeVariant::Default,
            kind: None,
            icon: None,
            icon_after: false,
            href: None,
            link: false,
        }
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// One of the six kind colors (`blue`, `green`, `jade`, `pink`, `teal`, `violet`), laid over
    /// the variant: its wash at 12%, its edge at 30%, its ink. [`crate::kind_of`] picks one
    /// steadily from a name, so the same value always wears the same color.
    pub fn kind(mut self, kind: &'static str) -> Self {
        self.kind = Some(kind);
        self
    }

    /// A 12px Lucide icon before the words.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self.icon_after = false;
        self
    }

    /// A 12px Lucide icon after the words.
    pub fn icon_after(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self.icon_after = true;
        self
    }

    /// Drawn as a link to `href`: it takes focus, washes on hover, and opens the address on a
    /// click.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self.link = true;
        self
    }

    /// Drawn as a link whose click the app answers (the response's `clicked`), as sokkel-ui's
    /// `render` draws it as another element.
    pub fn link(mut self, link: bool) -> Self {
        self.link = link;
        self
    }
}

impl BadgeVariant {
    fn key(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Secondary => "secondary",
            Self::Destructive => "destructive",
            Self::Outline => "outline",
            Self::Ghost => "ghost",
            Self::Link => "link",
        }
    }
}

impl Widget for Badge {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let role = TextRole::Badge;
        let galley = WidgetText::from(role.rich(&t, &self.label)).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            role.font(&t),
        );
        let icon_room = if self.icon.is_some() { 12.0 + 4.0 } else { 0.0 };
        let sense = if self.link { Sense::click() } else { Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + icon_room + 12.0, 20.0), sense);
        let role_kind = if self.link { egui::Role::Link } else { egui::Role::Label };
        response.widget_info(|| egui::WidgetInfo::labeled(role_kind, true, &self.label));
        let hovered = response.hovered();
        if self.link && hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if self.link
            && response.clicked()
            && let Some(href) = &self.href
        {
            ui.ctx().open_url(egui::OpenUrl::new_tab(href));
        }
        if ui.is_rect_visible(rect) {
            // the variant's paint, or the kind's laid over it; ghost and link answer hover
            let variant = match self.kind {
                Some(kind) => format!("kind-{kind}"),
                None => self.variant.key().to_owned(),
            };
            let state = if hovered && t.paint(&format!("badge.{variant}.hover.container")).is_some() {
                "hover"
            } else {
                "rest"
            };
            let key = format!("badge.{variant}.{state}.container");
            let paint = t.paint(&key).unwrap_or_default();
            let none = Color32::TRANSPARENT;
            let mut fill = paint.ground.unwrap_or(none);
            let border = paint.edge.unwrap_or(none);
            let mut ink = paint.ink.unwrap_or_else(|| t.color("foreground"));
            // drawn as a link, the filled variants and outline wash on hover too (`[a]:hover`)
            if self.link && hovered && self.kind.is_none() {
                match self.variant {
                    BadgeVariant::Default => fill = t.color_at("primary", 0.8),
                    BadgeVariant::Secondary => fill = t.color_at("secondary", 0.8),
                    BadgeVariant::Destructive => fill = t.color_at("destructive", 0.2),
                    BadgeVariant::Outline => {
                        fill = t.color("hover");
                        ink = t.color("muted-foreground");
                    }
                    BadgeVariant::Ghost | BadgeVariant::Link => {}
                }
            }
            let painter = ui.painter();
            painter.rect(
                rect,
                t.radius("chip"),
                fill,
                Stroke::new(1.0, border),
                StrokeKind::Inside,
            );
            let mut x = rect.left() + 6.0;
            let paint_icon = |x: f32| {
                if let Some(icon) = self.icon {
                    icons::paint(painter, pos2(x + 6.0, rect.center().y), icon, 12.0, ink);
                }
            };
            if !self.icon_after {
                paint_icon(x);
                x += icon_room;
            }
            let text_rect = egui::Rect::from_min_size(pos2(x, rect.center().y - galley.size().y / 2.0), galley.size());
            painter.galley(text_rect.min, galley, ink);
            if self.icon_after {
                paint_icon(text_rect.right() + 4.0);
            }
            if hovered && self.variant == BadgeVariant::Link && self.kind.is_none() {
                let y = text_rect.bottom() - 2.0;
                painter.hline(text_rect.x_range(), y, Stroke::new(1.0, ink));
            }
            if self.link {
                super::focus_ring(painter, &t, rect, t.radius("chip"), &response);
            }
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
        self.show_with(ui, |_| {})
    }

    /// The banner with `actions` at its end, 8 apart and level with its words' middle, as the
    /// web's `actions` (a Check again).
    pub fn show_with(self, ui: &mut egui::Ui, actions: impl FnOnce(&mut egui::Ui)) -> egui::Response {
        let t = crate::tokens(ui.ctx());
        let (key, icon) = match self.tone {
            BannerTone::Critical => ("critical", crate::Icon::CircleAlert),
            BannerTone::Warning => ("warning", crate::Icon::TriangleAlert),
            BannerTone::Info => ("info", crate::Icon::Info),
            BannerTone::Good => ("good", crate::Icon::CircleCheck),
        };
        let ink = t.color(&format!("{key}-ink"));
        let width = ui.available_width();
        let top = ui.cursor().top();
        let left = ui.cursor().left();
        let ground = ui.painter().add(egui::Shape::Noop);
        let rule = ui.painter().add(egui::Shape::Noop);
        // the actions at the end first, so the words wrap in what they leave
        let mut end = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(
                    egui::pos2(left, top + 6.0),
                    egui::vec2(width - 12.0, 28.0),
                ))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        end.spacing_mut().item_spacing.x = 8.0;
        actions(&mut end);
        let used = end.min_rect();
        let taken = if used.width() > 0.0 { used.width() + 8.0 } else { 0.0 };
        let tall_end = if used.width() > 0.0 { used.height() } else { 0.0 };
        let words = egui::WidgetText::from(crate::TextRole::Body.rich(&t, &self.text)).into_galley(
            ui,
            Some(egui::TextWrapMode::Wrap),
            (width - 24.0 - 24.0 - taken).max(1.0),
            crate::TextRole::Body.font(&t),
        );
        let inner = words.size().y.max(20.0).max(tall_end);
        let height = inner + 12.0;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        // what it says, for a screen reader
        let said = self.text.clone();
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &said));
        let painter = ui.painter();
        painter.set(
            ground,
            egui::Shape::rect_filled(rect, egui::CornerRadius::ZERO, t.color_at(key, 0.12)),
        );
        painter.set(
            rule,
            egui::Shape::hline(
                rect.x_range(),
                rect.bottom() - 0.5,
                egui::Stroke::new(1.0, t.color_at(key, 0.35)),
            ),
        );
        let middle = rect.top() + 6.0 + inner / 2.0;
        crate::icons::paint(painter, egui::pos2(rect.left() + 12.0 + 8.0, middle), icon, 16.0, ink);
        painter.galley(
            egui::pos2(rect.left() + 12.0 + 24.0, middle - words.size().y / 2.0),
            words,
            ink,
        );
        response
    }
}

/// Words that go somewhere, after sokkel-ui's `TextLink`: in the link's ink, underlined under the
/// pointer, pressed as a link.
#[must_use = "add it with `ui.add(...)`"]
pub struct TextLink {
    label: String,
    role: TextRole,
}

impl TextLink {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            role: TextRole::Body,
        }
    }

    /// Its text in another role, such as the help's beside a help line.
    pub fn role(mut self, role: TextRole) -> Self {
        self.role = role;
        self
    }
}

impl Widget for TextLink {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let t = tokens(ui.ctx());
        let galley = egui::WidgetText::from(self.role.rich(&t, &self.label)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            self.role.font(&t),
        );
        let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, &self.label));
        let ink = t.color_or("link", t.color("primary-ink"));
        let baseline = galley
            .rows
            .first()
            .map_or(rect.bottom(), |r| rect.top() + r.max_y() - 2.0);
        ui.painter().galley(rect.min, galley, ink);
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            ui.painter()
                .hline(rect.x_range(), baseline + 1.0, egui::Stroke::new(1.0, ink));
        }
        super::focus_ring(ui.painter(), &t, rect.expand(2.0), t.radius("mini"), &response);
        response
    }
}

/// A heading that opens what is under it, after sokkel-ui's `Unfoldable`: the whole heading row
/// is the control, its chevron turning, washing under the pointer as a menu row does. The row
/// reaches 4 past the column on each side, padded 4, so its chevron stands at the column's start
/// and its heading and what it holds line up 22 in. A summary under a shut heading says what is
/// folded away; facts can stand at the row's end; [`Unfoldable::show_static`] draws a row with
/// nothing to unfold, lined up with the rest. Held open while forced.
pub struct Unfoldable<'a> {
    id: egui::Id,
    heading: String,
    open_first: bool,
    open: Option<&'a mut bool>,
    aside: Option<super::section::Tools<'a>>,
    summary: Option<String>,
    wrap_summary: bool,
    dot: Option<egui::Color32>,
    ink: Option<egui::Color32>,
    forced: bool,
    quiet: bool,
}

/// The summary's and the content's start: the chevron's 14 and the row's 8.
const UNFOLD_INDENT: f32 = 22.0;

impl<'a> Unfoldable<'a> {
    pub fn new(id: impl egui::AsIdSalt, heading: impl Into<String>) -> Self {
        Self {
            id: egui::Id::unique(id),
            heading: heading.into(),
            open_first: false,
            open: None,
            aside: None,
            summary: None,
            wrap_summary: false,
            dot: None,
            ink: None,
            forced: false,
            quiet: false,
        }
    }

    /// The heading small, muted and at the body's weight, as a fold for details under a part
    /// (a problem's technical details).
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// A dot of 10 in `color` before the heading, 8 apart, as a heading led by sokkel-ui's `Dot`.
    pub fn dot(mut self, color: egui::Color32) -> Self {
        self.dot = Some(color);
        self
    }

    /// The heading's ink: the ink of the surface it stands on (`popover-foreground` in a popup,
    /// `sidebar-foreground` on the rail). Without it the heading takes the ui's text color.
    pub fn ink(mut self, ink: egui::Color32) -> Self {
        self.ink = Some(ink);
        self
    }

    /// Held open whatever the reader chose, as a filter does while it is typed: the row is static
    /// meanwhile, its chevron turned down in faint, and the reader's own choice returns after.
    pub fn forced(mut self, forced: bool) -> Self {
        self.forced = forced;
        self
    }

    /// Open the first time it is drawn.
    pub fn default_open(mut self, open: bool) -> Self {
        self.open_first = open;
        self
    }

    /// Open as the app holds it: a press on the row turns `open`, as the web's `open` with
    /// `onOpenChange`; without it the part holds its own.
    pub fn open(mut self, open: &'a mut bool) -> Self {
        self.open = Some(open);
        self
    }

    /// A line under the heading while it is shut, saying in short what the folded part holds.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Let a long summary wrap onto more lines instead of being cut to one.
    pub fn wrap_summary(mut self, wrap: bool) -> Self {
        self.wrap_summary = wrap;
        self
    }

    /// Facts at the row's end, outside the button's name, such as a count or a state's chip.
    pub fn aside(mut self, aside: impl FnOnce(&mut egui::Ui) + 'a) -> Self {
        self.aside = Some(Box::new(aside));
        self
    }

    /// The row alone, with nothing to unfold: no chevron and no button, its heading where an
    /// unfoldable row's stands, so a list of mixed rows lines up.
    pub fn show_static(self, ui: &mut egui::Ui) {
        self.draw(ui, None::<fn(&mut egui::Ui)>);
    }

    pub fn show(self, ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
        self.draw(ui, Some(content));
    }

    fn draw(mut self, ui: &mut egui::Ui, content: Option<impl FnOnce(&mut egui::Ui)>) {
        let t = tokens(ui.ctx());
        let held = self.open.as_deref().copied();
        let mut chosen: bool = held.unwrap_or_else(|| ui.data(|d| d.get_temp(self.id)).unwrap_or(self.open_first));
        let unfolds = content.is_some();
        let forced = self.forced && unfolds;
        let control = unfolds && !forced;
        let mut open = unfolds && (chosen || forced);
        let width = ui.available_width();
        let ink = self.ink.unwrap_or_else(|| ui.visuals().text_color());
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            // text.sm's line padded 4 above and below, a finger tall on a phone while it unfolds
            let sm = t.text("sm");
            let mut line = sm.line_height + 8.0;
            if control && super::is_phone(ui.ctx()) {
                line = line.max(t.length("spacing-finger"));
            }
            let (column, _) = ui.allocate_exact_size(vec2(width, line), Sense::hover());
            // the row reaches 4 past the column on each side
            let row = column.expand2(vec2(4.0, 0.0));
            let mut aside_width = 0.0;
            if let Some(aside) = self.aside.take() {
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(row.shrink2(vec2(4.0, 0.0)))
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                child.spacing_mut().item_spacing.x = 8.0;
                child.style_mut().override_text_style = None;
                aside(&mut child);
                aside_width = child.min_rect().width() + 8.0;
            }
            let face = egui::Rect::from_min_max(row.min, pos2(row.right() - aside_width, row.bottom()));
            let sense = if control { Sense::click() } else { Sense::hover() };
            let hit = ui.interact(face, self.id.with("row"), sense);
            if control {
                hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, open, &self.heading));
                if hit.hovered() {
                    ui.painter().rect_filled(face, t.radius("md"), t.color("hover"));
                }
            } else {
                hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &self.heading));
            }
            if unfolds {
                icons::paint(
                    ui.painter(),
                    pos2(face.left() + 4.0 + 7.0, face.center().y),
                    if open { Icon::ChevronDown } else { Icon::ChevronRight },
                    14.0,
                    t.color(if forced { "faint" } else { "muted-foreground" }),
                );
            }
            let mut start = face.left() + 4.0 + UNFOLD_INDENT;
            if let Some(color) = self.dot {
                ui.painter()
                    .circle_filled(pos2(start + 5.0, face.center().y), 5.0, color);
                start += 10.0 + 8.0;
            }
            let (heading, ink) = if self.quiet {
                (TextRole::Help.rich(&t, &self.heading), t.color("muted-foreground"))
            } else {
                (
                    egui::RichText::new(&self.heading)
                        .size(sm.size)
                        .line_height(Some(sm.line_height))
                        .variation("wght", t.weight(crate::Weight::Medium)),
                    ink,
                )
            };
            let galley = WidgetText::from(heading).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (face.right() - start - 4.0).max(1.0),
                egui::FontId::proportional(sm.size),
            );
            ui.painter()
                .galley(pos2(start, face.center().y - galley.size().y / 2.0), galley, ink);
            if control && hit.has_focus() {
                // the ring inside the row's edge, flush with it
                ui.painter().rect_stroke(
                    face,
                    t.radius("md"),
                    Stroke::new(t.length("focus-ring-width"), t.color("ring")),
                    StrokeKind::Inside,
                );
            }
            if control && hit.clicked() {
                chosen = !chosen;
                open = chosen;
            }
            if !open && let Some(summary) = &self.summary {
                // 6 up into the column's gap, under the heading's start
                ui.add_space(-6.0);
                let wrap = if self.wrap_summary {
                    TextWrapMode::Wrap
                } else {
                    TextWrapMode::Truncate
                };
                let galley = WidgetText::from(TextRole::Help.rich(&t, summary)).into_galley(
                    ui,
                    Some(wrap),
                    (width - UNFOLD_INDENT).max(1.0),
                    TextRole::Help.font(&t),
                );
                let (rect, _) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
                ui.painter().galley(
                    pos2(rect.left() + UNFOLD_INDENT, rect.top()),
                    galley,
                    t.color("muted-foreground"),
                );
            }
            if open && let Some(content) = content {
                ui.horizontal(|ui| {
                    ui.add_space(UNFOLD_INDENT);
                    ui.vertical(|ui| {
                        ui.set_width(width - UNFOLD_INDENT);
                        content(ui);
                    });
                });
            }
        });
        match self.open {
            Some(held) => {
                *held = chosen;
            }
            None => ui.data_mut(|d| {
                d.insert_temp(self.id, chosen);
            }),
        }
    }
}
