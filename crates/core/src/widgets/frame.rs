//! The app frame every sokkel app has: [`Shell`] lays out a [`Rail`] of the app's places, a
//! [`TopBar`] with the trail, the page, and a [`StatusLine`] as one bar across the bottom edge.

use egui::{
    Align, CornerRadius, Layout, Rect, Response, Sense, Stroke, TextWrapMode, Ui, UiBuilder, Widget, WidgetText, pos2,
    vec2,
};

use crate::{Icon, TextRole, icons, tokens};

/// One place in the rail: an id the app routes by, a label, a Lucide icon and an optional count.
#[derive(Clone, Debug)]
pub struct RailEntry {
    pub id: String,
    pub label: String,
    pub icon: Icon,
    pub count: Option<u32>,
}

impl RailEntry {
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Icon) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            count: None,
        }
    }

    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }
}

/// A run of places under an optional title.
#[derive(Clone, Debug)]
pub struct RailGroup {
    pub title: Option<String>,
    pub entries: Vec<RailEntry>,
}

impl RailGroup {
    pub fn new(title: Option<&str>, entries: impl IntoIterator<Item = RailEntry>) -> Self {
        Self {
            title: title.map(Into::into),
            entries: entries.into_iter().collect(),
        }
    }
}

/// The rail of an app's places, filling the height it is given: the app's mark and name, then
/// its groups. Collapsed, it shows the icons alone, each naming its place on hover.
#[must_use = "add it with `ui.add(...)`"]
pub struct Rail<'a> {
    name: String,
    groups: &'a [RailGroup],
    active: &'a mut String,
    collapsed: bool,
}

impl<'a> Rail<'a> {
    pub fn new(name: impl Into<String>, groups: &'a [RailGroup], active: &'a mut String) -> Self {
        Self {
            name: name.into(),
            groups,
            active,
            collapsed: false,
        }
    }

    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// The rail's width: 240 open, 57 collapsed (3.5rem and its edge).
    pub fn width(collapsed: bool) -> f32 {
        if collapsed { 57.0 } else { 240.0 }
    }
}

impl Widget for Rail<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, t.color("sidebar"));
        painter.vline(
            rect.right() - 0.5,
            rect.y_range(),
            Stroke::new(1.0, t.color("border-strong")),
        );

        // the head: the app's mark, and its name when open
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), t.length("spacing-shell-top")));
        let mark = Rect::from_center_size(pos2(head.left() + 12.0 + 14.0, head.center().y), vec2(28.0, 28.0));
        let mark = if self.collapsed {
            Rect::from_center_size(head.center(), mark.size())
        } else {
            mark
        };
        painter.rect_filled(mark, t.radius("md"), t.color("primary"));
        let initial: String = self.name.chars().take(1).collect();
        painter.text(
            mark.center(),
            egui::Align2::CENTER_CENTER,
            initial,
            TextRole::Head.font(&t),
            t.color("primary-foreground"),
        );
        if !self.collapsed {
            let name = galley(ui, TextRole::SectionTitle, &self.name, head.width() - 60.0);
            painter.galley(
                pos2(mark.right() + 8.0, head.center().y - name.size().y / 2.0),
                name,
                t.color("sidebar-foreground"),
            );
        }

        let row = t.density("row");
        let mut y = head.bottom();
        for group in self.groups {
            if let Some(title) = &group.title {
                y += 16.0;
                if self.collapsed {
                    painter.hline(
                        rect.left() + 16.0..=rect.right() - 16.0,
                        y + 4.0,
                        Stroke::new(1.0, t.color("sidebar-border")),
                    );
                    y += 10.0;
                } else {
                    // upper case, tracked, at 500, in faint
                    let spec = t.text("help");
                    let rich = TextRole::Help
                        .rich(&t, title.to_uppercase())
                        .variation("wght", 500.0)
                        .extra_letter_spacing(spec.size * 0.025);
                    let label = WidgetText::from(rich).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        rect.width() - 32.0,
                        TextRole::Help.font(&t),
                    );
                    painter.galley(pos2(rect.left() + 16.0, y), label, t.color("faint"));
                    y += 26.0;
                }
            } else {
                y += 8.0;
            }
            for entry in &group.entries {
                let bounds = if self.collapsed {
                    Rect::from_center_size(pos2(rect.center().x, y + row / 2.0), vec2(36.0, row))
                } else {
                    Rect::from_min_max(pos2(rect.left() + 8.0, y), pos2(rect.right() - 9.0, y + row))
                };
                let hit = ui.interact(bounds, response.id.with(&entry.id), Sense::click());
                let active = *self.active == entry.id;
                hit.widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, &entry.label)
                });
                if hit.clicked() {
                    *self.active = entry.id.clone();
                }
                let hit = if self.collapsed {
                    hit.on_hover_text(&entry.label)
                } else {
                    hit
                };
                if active {
                    let corners = if self.collapsed {
                        t.radius("md")
                    } else {
                        CornerRadius {
                            nw: 2,
                            sw: 2,
                            ..t.radius("md")
                        }
                    };
                    painter.rect_filled(bounds, corners, t.color("sidebar-accent"));
                    if !self.collapsed {
                        let edge = Rect::from_min_size(bounds.min, vec2(2.0, bounds.height()));
                        painter.rect_filled(
                            edge,
                            CornerRadius {
                                nw: 2,
                                sw: 2,
                                ne: 0,
                                se: 0,
                            },
                            t.color("sidebar-primary"),
                        );
                    }
                } else if hit.hovered() {
                    painter.rect_filled(bounds, t.radius("md"), t.color("hover"));
                }
                let ink = if active {
                    t.color("sidebar-accent-foreground")
                } else if hit.hovered() {
                    t.color("foreground")
                } else {
                    t.color("muted-foreground")
                };
                if self.collapsed {
                    icons::paint(&painter, bounds.center(), entry.icon, 16.0, ink);
                } else {
                    icons::paint(
                        &painter,
                        pos2(bounds.left() + 10.0 + 8.0, bounds.center().y),
                        entry.icon,
                        16.0,
                        ink,
                    );
                    let role = if active { TextRole::Action } else { TextRole::Body };
                    let label = galley(ui, role, &entry.label, bounds.width() - 80.0);
                    painter.galley(
                        pos2(bounds.left() + 38.0, bounds.center().y - label.size().y / 2.0),
                        label,
                        ink,
                    );
                    if let Some(count) = entry.count {
                        let figure = galley(ui, TextRole::Badge, &count.to_string(), 60.0);
                        let at = pos2(
                            bounds.right() - 12.0 - figure.size().x,
                            bounds.center().y - figure.size().y / 2.0,
                        );
                        painter.galley(at, figure, t.color("foreground"));
                    }
                }
                y += row + 4.0;
            }
        }
        response
    }
}

fn galley(ui: &Ui, role: TextRole, text: &str, width: f32) -> std::sync::Arc<egui::Galley> {
    let t = tokens(ui.ctx());
    WidgetText::from(role.rich(&t, text)).into_galley(ui, Some(TextWrapMode::Truncate), width.max(1.0), role.font(&t))
}

/// The bar over the page: the trail, ancestors muted and the current place in the action weight,
/// with the bar's own actions at its end.
pub struct TopBar<'a> {
    trail: &'a [&'a str],
}

impl<'a> TopBar<'a> {
    pub fn new(trail: &'a [&'a str]) -> Self {
        Self { trail }
    }

    pub fn show(self, ui: &mut Ui, actions: impl FnOnce(&mut Ui)) -> Response {
        let t = tokens(ui.ctx());
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::ZERO, t.color("sidebar"));
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, t.color("border-strong")),
        );
        let inner = rect.shrink2(vec2(16.0, 0.0));
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let last = self.trail.len().saturating_sub(1);
                for (i, crumb) in self.trail.iter().enumerate() {
                    if i > 0 {
                        let (chevron, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                        icons::paint(
                            ui.painter(),
                            chevron.center(),
                            Icon::ChevronRight,
                            12.0,
                            t.color("faint"),
                        );
                    }
                    let (role, ink) = if i == last {
                        (TextRole::Action, t.color("foreground"))
                    } else {
                        (TextRole::Body, t.color("muted-foreground"))
                    };
                    ui.label(role.rich(&t, *crumb).color(ink));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), actions);
            },
        );
        response
    }
}

/// How the status line's note is inked: quiet in the foot's own ink, live, a warning, an error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusTone {
    #[default]
    Quiet,
    Live,
    Warn,
    Error,
}

/// The foot strip across the window's bottom edge: a note at the start with a dot for its tone,
/// and short facts at the end.
#[must_use = "add it with `ui.add(...)`"]
pub struct StatusLine {
    note: String,
    tone: StatusTone,
    facts: Vec<String>,
}

impl StatusLine {
    pub fn new(note: impl Into<String>) -> Self {
        Self {
            note: note.into(),
            tone: StatusTone::Quiet,
            facts: Vec::new(),
        }
    }

    pub fn tone(mut self, tone: StatusTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn fact(mut self, fact: impl Into<String>) -> Self {
        self.facts.push(fact.into());
        self
    }
}

impl Widget for StatusLine {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, Sense::hover());
        let ground = t.color_or("page-foot-ground", t.color("background"));
        let line = t.color_or("page-foot-line", t.color("border"));
        let quiet = t.color_or("page-foot-ink", t.color("muted-foreground"));
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, ground);
        painter.hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, line));
        let ink = match self.tone {
            StatusTone::Quiet => quiet,
            StatusTone::Live => t.color("info"),
            StatusTone::Warn => t.color("warning"),
            StatusTone::Error => t.color("destructive"),
        };
        let mut x = rect.left() + 16.0;
        if self.tone != StatusTone::Quiet {
            painter.circle_filled(pos2(x + 4.0, rect.center().y), 4.0, ink);
            x += 16.0;
        }
        let note = galley(ui, TextRole::Help, &self.note, rect.width() / 2.0);
        painter.galley(pos2(x, rect.center().y - note.size().y / 2.0), note, ink);
        let mut right = rect.right() - 16.0;
        for fact in self.facts.iter().rev() {
            let text = galley(ui, TextRole::Help, fact, 240.0);
            right -= text.size().x;
            painter.galley(pos2(right, rect.center().y - text.size().y / 2.0), text, quiet);
            right -= 16.0;
        }
        response
    }
}

/// Which part of the frame [`Shell::show`] is asking for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellPart {
    Rail,
    Top,
    Page,
    Foot,
}

/// The frame: the rail at the start, the top bar over the page, and the status line as one bar
/// across the bottom, within the rect it is given.
pub struct Shell {
    collapsed: bool,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    pub fn new() -> Self {
        Self { collapsed: false }
    }

    /// Lay the frame out for a collapsed rail.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Fill `rect` with the frame, asking `part` to draw each piece in its place.
    pub fn show(self, ui: &mut Ui, rect: Rect, mut part: impl FnMut(ShellPart, &mut Ui)) {
        let t = tokens(ui.ctx());
        let foot = t.length("spacing-shell-foot");
        let top = t.length("spacing-shell-top");
        let rail_width = Rail::width(self.collapsed);
        let body = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - foot));
        let rail = Rect::from_min_max(body.min, pos2(body.left() + rail_width, body.bottom()));
        let top_bar = Rect::from_min_max(pos2(rail.right(), body.top()), pos2(body.right(), body.top() + top));
        let page = Rect::from_min_max(pos2(rail.right(), top_bar.bottom()), body.max);
        let foot_strip = Rect::from_min_max(pos2(rect.left(), body.bottom()), rect.max);
        ui.painter()
            .rect_filled(page, CornerRadius::ZERO, t.color("background"));
        for (which, area) in [
            (ShellPart::Rail, rail),
            (ShellPart::Top, top_bar),
            (ShellPart::Page, page),
            (ShellPart::Foot, foot_strip),
        ] {
            ui.scope_builder(
                UiBuilder::new().max_rect(area).layout(Layout::top_down(Align::Min)),
                |ui| {
                    ui.set_clip_rect(area.intersect(ui.clip_rect()));
                    part(which, ui);
                },
            );
        }
        ui.painter().rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(1.0, t.color("border-strong")),
            egui::StrokeKind::Inside,
        );
        ui.allocate_rect(rect, Sense::hover());
    }
}
