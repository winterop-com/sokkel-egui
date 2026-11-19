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
    /// A line under the label, small and muted: what the place holds ("Everything at a glance").
    pub description: Option<String>,
    /// The kinds of [`DragData`](crate::DragData) the entry's row takes when dropped on it, inside a
    /// rail with [`Rail::drops`]: a song onto a playlist.
    pub takes: Vec<String>,
    /// A color drawn as a dot in the icon's place, for a label, a tag or a channel: a color token's
    /// name, such as `kind-teal`.
    pub dot: Option<String>,
    /// The entries under this one, one level deep, as the web's entry `entries`: expanded, the
    /// row folds them under it behind a chevron at its end, open at first while one of them is the
    /// place shown; collapsed, the entry stands for them, marked while one of them is shown.
    pub entries: Vec<RailEntry>,
}

impl RailEntry {
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Icon) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            count: None,
            description: None,
            takes: Vec::new(),
            dot: None,
            entries: Vec::new(),
        }
    }

    /// The entries under this one, as the web's `entries`; each is chosen by its own id.
    pub fn entries(mut self, entries: impl IntoIterator<Item = RailEntry>) -> Self {
        self.entries = entries.into_iter().collect();
        self
    }

    /// A dot in the token's color in place of the icon, as the web's entry `dot`.
    pub fn dot(mut self, token: impl Into<String>) -> Self {
        self.dot = Some(token.into());
        self
    }

    /// Makes the entry's row a place a [`DragData`](crate::DragData) of this kind is dropped on,
    /// inside a rail with [`Rail::drops`], as the web's entry `drop` with its `accepts`; called again
    /// for each further kind it takes.
    pub fn takes(mut self, kind: impl Into<String>) -> Self {
        self.takes.push(kind.into());
        self
    }

    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// The line under the label; an expanded rail draws the entry two lines tall.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
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
    toggle: Option<&'a mut bool>,
    mark: Option<&'static str>,
    words: RailWords,
    body: Option<RailBody<'a>>,
    drops: Option<&'a mut Option<RailDrop>>,
}

/// A thing let go on a rail entry that takes it: what was dragged, and the entry's id.
#[derive(Clone, Debug, PartialEq)]
pub struct RailDrop {
    pub entry: String,
    pub data: crate::DragData,
}

/// What a rail holds in place of its groups, after sokkel-ui's rail `body`: a heading, a row such
/// as a search, and the content the rail scrolls under them.
pub struct RailBody<'a> {
    pub title: Option<String>,
    pub search: Option<ShellBanner<'a>>,
    pub content: Box<dyn FnOnce(&mut Ui) + 'a>,
    /// The places stand over the body, both shown at once with no row back to the places, as the
    /// web's `places: 'above'`, for an app with a few places.
    pub places_above: bool,
}

/// The words a rail says.
#[derive(Clone, Debug)]
pub struct RailWords {
    pub collapse: String,
    pub expand: String,
    /// The row over a body that goes back to the places.
    pub places: String,
}

impl Default for RailWords {
    fn default() -> Self {
        Self {
            collapse: "Collapse the navigation".into(),
            expand: "Expand the navigation".into(),
            places: "All places".into(),
        }
    }
}

impl<'a> Rail<'a> {
    pub fn new(name: impl Into<String>, groups: &'a [RailGroup], active: &'a mut String) -> Self {
        Self {
            name: name.into(),
            groups,
            active,
            collapsed: false,
            toggle: None,
            mark: None,
            words: RailWords::default(),
            body: None,
            drops: None,
        }
    }

    /// The rail takes drops, after sokkel-ui's `RailDrops`: while a [`DragData`](crate::DragData)
    /// is carried (egui's drag payload, `egui::DragAndDrop::set_payload`) whose kind an entry
    /// [`takes`](RailEntry::takes), the rail shows its places rather than its body, the entry's
    /// row is ringed while the thing is over it, and the thing let go there lands in `dropped`.
    /// A rail without it takes no drops.
    pub fn drops(mut self, dropped: &'a mut Option<RailDrop>) -> Self {
        self.drops = Some(dropped);
        self
    }

    /// The app's own content in place of the groups, as the web's `body`: a row at the top ("All
    /// places") shows the groups in its place, and a place chosen there brings the body back.
    /// Collapsed, the rail draws the groups' icons, a body having no icon form.
    pub fn body(mut self, body: RailBody<'a>) -> Self {
        self.body = Some(body);
        self
    }

    /// The reader narrows the rail to its icons and widens it again: a button at the head's end
    /// collapses it, and collapsed, its mark expands it, showing the open-panel icon under the
    /// pointer. `collapsed` holds the state; give the shell the same (`Shell::collapsed`).
    pub fn collapsible(mut self, collapsed: &'a mut bool) -> Self {
        self.collapsed = *collapsed;
        self.toggle = Some(collapsed);
        self
    }

    pub fn words(mut self, words: RailWords) -> Self {
        self.words = words;
        self
    }

    /// The app's mark on the tile at the rail's head, as SVG drawn in `currentColor`, the way the
    /// web takes a mark; without one the tile shows the name's initial.
    pub fn mark(mut self, svg: &'static str) -> Self {
        self.mark = Some(svg);
        self
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
    fn ui(mut self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        // the window's own buttons, where the frame stands under a hidden title bar
        let lights = desktop_lights(ui.ctx());
        // in the shell's drawer: expanded, without its edge, its head's control closing the drawer
        let in_drawer = drawer(ui.ctx()).drawing;
        let narrow = self.collapsed && !in_drawer;
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, t.color("sidebar"));
        if !in_drawer {
            // collapsed under the window's buttons, the edge is left undrawn through the head, so
            // the strip under the buttons reads as one title bar
            let top = if narrow && lights.is_some() {
                rect.top() + t.length("spacing-shell-top")
            } else {
                rect.top()
            };
            painter.vline(
                rect.right() - 0.5,
                top..=rect.bottom(),
                Stroke::new(1.0, t.color("border-strong")),
            );
        }

        // the head: the app's mark, and its name when open
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), t.length("spacing-shell-top")));
        // a 32 tile, 12 in from the edge, as sokkel-ui's `size-8` mark in its `px-3` head; after the
        // window's buttons where they stand in the head
        let start = match lights {
            Some(room) if !narrow && !in_drawer => room,
            _ => 12.0,
        };
        let mark = Rect::from_center_size(pos2(head.left() + start + 16.0, head.center().y), vec2(32.0, 32.0));
        // collapsed under the window's buttons, the head stays empty and the mark stands just below it
        let under = narrow && lights.is_some();
        let mark = if under {
            Rect::from_center_size(pos2(head.center().x, head.bottom() + 4.0 + 16.0), mark.size())
        } else if narrow {
            Rect::from_center_size(head.center(), mark.size())
        } else {
            mark
        };
        // collapsed and collapsible, the mark is the button that widens the rail again
        let expand = (narrow && self.toggle.is_some()).then(|| {
            let hit = ui.interact(mark, response.id.with("expand"), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.expand));
            hit
        });
        painter.rect_filled(mark, t.radius("md"), t.color("primary"));
        if let Some(hit) = &expand {
            super::focus_ring(&painter, &t, mark, t.radius("md"), hit);
        }
        if let Some(hit) = expand.as_ref().filter(|hit| hit.hovered()) {
            icons::paint(
                &painter,
                mark.center(),
                Icon::PanelLeftOpen,
                16.0,
                t.color("primary-foreground"),
            );
            if hit.clicked()
                && let Some(toggle) = self.toggle.as_deref_mut()
            {
                *toggle = false;
            }
        } else {
            paint_mark(ui, &painter, &t, mark, self.mark, &self.name);
        }
        // open and collapsible, a button at the head's end narrows the rail (in the drawer, the
        // drawer's close stands there instead)
        let collapse = (!narrow && !in_drawer && self.toggle.is_some())
            .then(|| Rect::from_center_size(pos2(head.right() - 12.0 - 16.0, head.center().y), vec2(32.0, 32.0)));
        if let Some(button) = collapse {
            let hit = ui.interact(button, response.id.with("collapse"), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.collapse));
            frame_control(&painter, &t, button, &hit, Icon::PanelLeftClose);
            if hit.clicked()
                && let Some(toggle) = self.toggle.as_deref_mut()
            {
                *toggle = true;
            }
        }
        if !narrow {
            // the name at the title size, semibold and tight, as `text-title font-semibold tracking-tight`
            let spec = t.text("detail-title");
            let rich = egui::RichText::new(&self.name)
                .size(spec.size)
                .line_height(Some(spec.line_height))
                .variation("wght", 600.0)
                .extra_letter_spacing(-0.025 * spec.size);
            let name = egui::WidgetText::from(rich).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (collapse.map_or(head.right() - 12.0, |b| b.left() - 8.0) - mark.right() - 8.0).max(1.0),
                TextRole::DetailTitle.font(&t),
            );
            painter.galley(
                pos2(mark.right() + 8.0, head.center().y - name.size().y / 2.0),
                name,
                t.color("sidebar-foreground"),
            );
        }

        if in_drawer {
            // the head's end: the control that closes the drawer, 32 square
            let close = Rect::from_center_size(pos2(head.right() - 12.0 - 16.0, head.center().y), vec2(32.0, 32.0));
            let hit = ui.interact(close, response.id.with("close"), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, ShellWords::default().close));
            frame_control(&painter, &t, close, &hit, Icon::X);
            if hit.clicked() {
                set_drawer(ui.ctx(), |d| d.open = false);
            }
        }
        // a finger tall on a phone
        let row = if is_phone(ui.ctx()) {
            t.length("spacing-finger")
        } else {
            t.density("row")
        };
        // the body in place of the groups while it shows
        let view_id = response.id.with("view");
        let above = self.body.as_ref().is_some_and(|b| b.places_above);
        // a thing an entry takes, carried: the rail shows its places to drop it on
        let carried = self
            .drops
            .as_ref()
            .and_then(|_| egui::DragAndDrop::payload::<crate::DragData>(ui.ctx()))
            .filter(|data| {
                self.groups
                    .iter()
                    .flat_map(|g| &g.entries)
                    .any(|e| e.takes.contains(&data.kind))
            });
        let showing_body = !narrow
            && !above
            && carried.is_none()
            && self.body.is_some()
            && ui.data(|d| d.get_temp::<bool>(view_id)).unwrap_or(true);
        if showing_body && let Some(body) = self.body.take() {
            let mut y = head.bottom();
            let back = Rect::from_min_max(pos2(rect.left() + 8.0, y), pos2(rect.right() - 9.0, y + row));
            let hit = ui.interact(back, response.id.with("places"), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.places));
            if hit.hovered() {
                painter.rect_filled(back, t.radius("md"), t.color("hover"));
            }
            super::focus_ring(&painter, &t, back, t.radius("md"), &hit);
            let ink = if hit.hovered() {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            icons::paint(
                &painter,
                pos2(back.left() + 12.0 + 8.0, back.center().y),
                Icon::ChevronLeft,
                16.0,
                ink,
            );
            let label = galley(ui, TextRole::Body, &self.words.places, back.width() - 52.0);
            painter.galley(
                pos2(back.left() + 40.0, back.center().y - label.size().y / 2.0),
                label,
                ink,
            );
            if hit.clicked() {
                ui.data_mut(|d| d.insert_temp(view_id, false));
            }
            y = back.bottom();
            draw_rail_body(ui, &t, rect, response.id, y, body);
            return response;
        }
        // a first group without a title keeps 4 over its first entry, so the entry's focus is whole
        let untitled_top = if self.groups.first().is_some_and(|g| g.title.is_none()) {
            4.0
        } else {
            0.0
        };
        // collapsed under the window's buttons, the mark's own line (`py-1`) stands over the groups
        let mut y = head.bottom() + untitled_top + if under { 40.0 } else { 0.0 };
        for (index, group) in self.groups.iter().enumerate() {
            // the groups stand one under the other with no gap: the 4 after a group's last entry
            // is its own gap, not one between groups
            if index > 0 {
                y -= 4.0;
            }
            if let Some(title) = &group.title {
                // a title stands 16 under the entries before (`pt-4`), its line 20 tall, then 6
                // (`pb-1.5`) and the group's 4 to its entries
                y += 16.0;
                if narrow {
                    // the title's room kept, its rule centered in the title's 20 line
                    painter.hline(
                        rect.left() + 16.0..=rect.right() - 16.0,
                        y + 10.0,
                        Stroke::new(1.0, t.color("border-strong")),
                    );
                    y += 30.0;
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
                    y += 30.0;
                }
            }
            // a parent's entries follow it while it is open; collapsed, the parent stands for them
            let mut drawn: Vec<(&RailEntry, Nested)> = Vec::new();
            for entry in &group.entries {
                if entry.entries.is_empty() {
                    drawn.push((entry, Nested::Plain));
                    continue;
                }
                let holds = entry.entries.iter().any(|child| child.id == *self.active);
                let open_id = response.id.with(("open", &entry.id));
                let open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(holds);
                drawn.push((entry, Nested::Parent { open, holds }));
                if open && !narrow {
                    drawn.extend(entry.entries.iter().map(|child| (child, Nested::Child)));
                }
            }
            for (entry, nested) in drawn {
                // a described entry: 6 over and under its 22 label line and its description's 20
                // tall lines, wrapped beside the count as the web's `text-pretty` wraps them
                let described = !narrow && entry.description.is_some();
                let count_room = entry.count.filter(|count| *count > 0).map_or(0.0, |count| {
                    let rich = TextRole::Badge.rich(&t, count.to_string()).variation("wght", 600.0);
                    WidgetText::from(rich)
                        .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextRole::Badge.font(&t))
                        .size()
                        .x
                        + 12.0
                });
                let words = entry
                    .description
                    .as_deref()
                    .filter(|_| described)
                    .map(|d| pretty_lines(ui, TextRole::Help, d, rect.width() - 17.0 - 52.0 - count_room));
                let lines = words.as_ref().map_or(1, Vec::len) as f32;
                let height = if described {
                    6.0 + 22.0 + 20.0 * lines + 6.0
                } else {
                    row
                };
                let bounds = if narrow {
                    Rect::from_center_size(pos2(rect.center().x, y + row / 2.0), vec2(36.0, row))
                } else {
                    Rect::from_min_max(pos2(rect.left() + 8.0, y), pos2(rect.right() - 9.0, y + height))
                };
                let line = if described {
                    bounds.top() + 6.0 + 11.0
                } else {
                    bounds.center().y
                };
                let hit = ui.interact(bounds, response.id.with(&entry.id), Sense::click());
                // a parent is marked while one of its entries is shown and it is folded (or the
                // rail collapsed), and set apart, unwashed, while it is open over it
                let (holding, folded_over) = match nested {
                    Nested::Parent { open, holds } => (holds && open && !narrow, holds && (!open || narrow)),
                    _ => (false, false),
                };
                let active = *self.active == entry.id || folded_over;
                // the chevron at a parent's end folds and unfolds its entries, drawn over the row
                let mut chevron_drawn = None;
                if let Nested::Parent { open, .. } = nested
                    && !narrow
                {
                    let chevron = Rect::from_center_size(pos2(bounds.right() - 4.0 - 14.0, line), vec2(28.0, 28.0));
                    let fold = ui.interact(chevron, response.id.with(("fold", &entry.id)), Sense::click());
                    fold.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, open, &entry.label));
                    chevron_drawn = Some((chevron, open, fold.hovered()));
                    if fold.clicked() {
                        ui.data_mut(|d| d.insert_temp(response.id.with(("open", &entry.id)), !open));
                    }
                }
                hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, &entry.label));
                if hit.clicked() {
                    *self.active = entry.id.clone();
                    // a place chosen in the drawer closes it, as a change of route does
                    set_drawer(ui.ctx(), |d| d.open = false);
                    // a place chosen while a body waits brings the body back
                    ui.data_mut(|d| d.insert_temp(view_id, true));
                }
                let hit = if narrow {
                    super::tooltip(hit, entry.label.clone())
                } else {
                    hit
                };
                // the start corners `radius-sm`, the end `radius-md`, in every state
                let (sm, md) = (t.radius("sm").nw, t.radius("md").nw);
                let row_corners = CornerRadius {
                    nw: sm,
                    sw: sm,
                    ne: md,
                    se: md,
                };
                if active {
                    let corners = if narrow { t.radius("md") } else { row_corners };
                    painter.rect_filled(bounds, corners, t.color("sidebar-accent"));
                    if !narrow {
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
                    let corners = if narrow { t.radius("md") } else { row_corners };
                    painter.rect_filled(bounds, corners, t.color("hover"));
                }
                // a thing the entry takes, over its row: ringed on the hover ground, and taken once let go
                if let Some(data) = carried.as_ref().filter(|data| entry.takes.contains(&data.kind))
                    && ui.rect_contains_pointer(bounds)
                {
                    let md = t.radius("md").nw;
                    let corners = CornerRadius {
                        nw: 0,
                        sw: 0,
                        ne: md,
                        se: md,
                    };
                    // the hover ground once, where the row has not drawn it under the pointer
                    let ground = if hit.hovered() || active {
                        egui::Color32::TRANSPARENT
                    } else {
                        t.color("hover")
                    };
                    painter.rect(
                        bounds,
                        corners,
                        ground,
                        Stroke::new(2.0, t.color("ring")),
                        egui::StrokeKind::Inside,
                    );
                    if ui.input(|i| i.pointer.any_released())
                        && let Some(dropped) = self.drops.as_deref_mut()
                    {
                        egui::DragAndDrop::clear_payload(ui.ctx());
                        *dropped = Some(RailDrop {
                            entry: entry.id.clone(),
                            data: (**data).clone(),
                        });
                    }
                }
                // the frame's one keyboard focus look, the button's
                super::focus_ring(&painter, &t, bounds, t.radius("md"), &hit);
                let ink = if active {
                    t.color("sidebar-accent-foreground")
                } else if hit.hovered() || holding {
                    t.color("foreground")
                } else {
                    t.color("muted-foreground")
                };
                // the entry's mark: its color dot (10 across), or its icon
                let mark = |at: egui::Pos2| match &entry.dot {
                    Some(token) => {
                        painter.circle_filled(at, 5.0, t.color(token));
                    }
                    None => icons::paint(&painter, at, entry.icon, 16.0, ink),
                };
                if narrow {
                    mark(bounds.center());
                    // the count over the icon's corner: a round critical badge 16 tall, ringed in the
                    // rail's ground, 4 over the tile and 6 past its end
                    if let Some(count) = entry.count.filter(|count| *count > 0) {
                        let spec = t.text("2xs");
                        let words = if count > 99 {
                            "99+".to_owned()
                        } else {
                            count.to_string()
                        };
                        let rich = egui::RichText::new(words)
                            .size(spec.size)
                            .line_height(Some(spec.line_height))
                            .variation("wght", 600.0);
                        let figure = WidgetText::from(rich).into_galley(
                            ui,
                            Some(TextWrapMode::Extend),
                            f32::INFINITY,
                            TextRole::Badge.font(&t),
                        );
                        let width = (figure.size().x + 8.0).max(16.0);
                        let badge = Rect::from_min_size(
                            pos2(bounds.right() + 6.0 - width, bounds.top() - 4.0),
                            vec2(width, 16.0),
                        );
                        painter.rect_filled(badge.expand(2.0), CornerRadius::same(10), t.color("sidebar"));
                        painter.rect_filled(badge, CornerRadius::same(8), t.color("critical"));
                        painter.galley(
                            badge.center() - figure.size() / 2.0,
                            figure,
                            t.color("critical-foreground"),
                        );
                    }
                } else {
                    // an entry under a parent has no mark: its label lines up under the parent's
                    if nested != Nested::Child {
                        mark(pos2(bounds.left() + 12.0 + 8.0, line));
                    }
                    let role = if active || holding {
                        TextRole::Action
                    } else {
                        TextRole::Body
                    };
                    let label = galley(ui, role, &entry.label, bounds.width() - 80.0);
                    painter.galley(pos2(bounds.left() + 40.0, line - label.size().y / 2.0), label, ink);
                    if let Some(lines) = &words {
                        let quiet = if active {
                            t.color("sidebar-accent-foreground").gamma_multiply(0.75)
                        } else {
                            t.color("muted-foreground")
                        };
                        for (index, line) in lines.iter().enumerate() {
                            let words = galley(ui, TextRole::Help, line, bounds.width() - 52.0);
                            let middle = bounds.top() + 6.0 + 22.0 + 20.0 * index as f32 + 10.0;
                            painter.galley(pos2(bounds.left() + 40.0, middle - words.size().y / 2.0), words, quiet);
                        }
                    }
                    if let Some(count) = entry
                        .count
                        .filter(|count| *count > 0 && !matches!(nested, Nested::Parent { .. }))
                    {
                        // the count at 600
                        let rich = TextRole::Badge.rich(&t, count.to_string()).variation("wght", 600.0);
                        let figure = WidgetText::from(rich).into_galley(
                            ui,
                            Some(TextWrapMode::Truncate),
                            60.0,
                            TextRole::Badge.font(&t),
                        );
                        let at = pos2(bounds.right() - 12.0 - figure.size().x, line - figure.size().y / 2.0);
                        painter.galley(at, figure, t.color("foreground"));
                    }
                }
                if let Some((chevron, open, hovered)) = chevron_drawn {
                    if hovered {
                        painter.rect_filled(chevron, t.radius("md"), t.color("hover"));
                    }
                    icons::paint(
                        &painter,
                        chevron.center(),
                        if open { Icon::ChevronDown } else { Icon::ChevronRight },
                        16.0,
                        t.color("muted-foreground"),
                    );
                }
                y += if narrow { row } else { height } + 4.0;
            }
        }
        // a body under the places, both shown at once
        if above
            && !narrow
            && let Some(body) = self.body.take()
        {
            draw_rail_body(ui, &t, rect, response.id, y, body);
        }
        response
    }
}

/// Where an entry stands among a parent's: on its own, a parent (open or folded, holding the place
/// shown or not), or one of an open parent's entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Nested {
    Plain,
    Parent { open: bool, holds: bool },
    Child,
}

/// A rail body's heading, its search and its content, scrolling, from `y` down the rail.
fn draw_rail_body(ui: &mut Ui, t: &crate::Tokens, rect: Rect, id: egui::Id, mut y: f32, body: RailBody<'_>) {
    let painter = ui.painter().clone();
    if let Some(title) = &body.title {
        // 16 over the heading's 20 line, 6 under it, as `pt-4 pb-1.5`
        y += 16.0;
        let spec = t.text("help");
        let rich = TextRole::Help
            .rich(t, title.to_uppercase())
            .variation("wght", 500.0)
            .extra_letter_spacing(spec.size * 0.025);
        let label = WidgetText::from(rich).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            rect.width() - 32.0,
            TextRole::Help.font(t),
        );
        painter.galley(pos2(rect.left() + 16.0, y), label, t.color("faint"));
        y += 20.0 + 6.0;
    }
    if let Some(search) = body.search {
        y += 4.0;
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    pos2(rect.left() + 8.0, y),
                    pos2(rect.right() - 9.0, y + t.density("control")),
                ))
                .layout(Layout::left_to_right(Align::Center)),
        );
        child.spacing_mut().item_spacing.x = 2.0;
        search(&mut child);
        // the row a control tall, whatever its buttons' own hit areas
        y += t.density("control");
    }
    y += 8.0;
    let area = Rect::from_min_max(
        pos2(rect.left() + 8.0, y),
        pos2(rect.right() - 9.0, rect.bottom() - 12.0),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(area)
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(area.intersect(ui.clip_rect()));
    egui::ScrollArea::vertical()
        .id_salt(id.with("body"))
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            ui.set_width(area.width());
            (body.content)(ui);
        });
}

/// The app's mark on its tile's ink: the SVG, drawn in `currentColor`, or the name's initial.
fn paint_mark(ui: &Ui, painter: &egui::Painter, t: &crate::Tokens, tile: Rect, svg: Option<&str>, name: &str) {
    match svg {
        Some(svg) => {
            // the mark's ink is `currentColor`: drawn white, then tinted with the tile's ink
            let white = svg.replace("currentColor", "#ffffff");
            let uri = format!("bytes://rail-mark-{}.svg", egui::Id::unique(svg).value());
            egui::Image::from_bytes(uri, white.into_bytes())
                .tint(t.color("primary-foreground"))
                .paint_at(ui, Rect::from_center_size(tile.center(), vec2(16.0, 16.0)));
        }
        None => {
            let initial: String = name.chars().take(1).collect();
            painter.text(
                tile.center(),
                egui::Align2::CENTER_CENTER,
                initial,
                TextRole::Head.font(t),
                t.color("primary-foreground"),
            );
        }
    }
}

/// A control of the frame's own, such as the rail head's or the top bar's menu button: an icon in
/// `muted-foreground`, under the pointer the `hover` ground and the `foreground` ink, and the
/// frame's keyboard focus, the button's.
fn frame_control(painter: &egui::Painter, t: &crate::Tokens, rect: Rect, hit: &Response, icon: Icon) {
    if hit.hovered() {
        painter.rect_filled(rect, t.radius("md"), t.color("hover"));
    }
    let ink = if hit.hovered() {
        t.color("foreground")
    } else {
        t.color("muted-foreground")
    };
    icons::paint(painter, rect.center(), icon, 16.0, ink);
    super::focus_ring(painter, t, rect, t.radius("md"), hit);
}

/// A top bar control on a phone: as `frame_control`, its focus outline inside it, since the bar is
/// barely taller than the control and an outline outside would be cut off at the window's top.
fn phone_control(painter: &egui::Painter, t: &crate::Tokens, rect: Rect, hit: &Response, icon: Icon) {
    if hit.hovered() {
        painter.rect_filled(rect, t.radius("md"), t.color("hover"));
    }
    let ink = if hit.hovered() {
        t.color("foreground")
    } else {
        t.color("muted-foreground")
    };
    icons::paint(painter, rect.center(), icon, 16.0, ink);
    super::focus_ring_inset(painter, t, rect, t.radius("md"), hit);
}

/// `text` cut into lines no wider than `width`, as CSS `text-wrap: pretty` cuts them: a word at a
/// time, then a last line of one word takes the word before it down, when that fits.
fn pretty_lines(ui: &Ui, role: TextRole, text: &str, width: f32) -> Vec<String> {
    let t = tokens(ui.ctx());
    let wide = |line: &str| {
        WidgetText::from(role.rich(&t, line))
            .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, role.font(&t))
            .size()
            .x
    };
    let mut lines: Vec<Vec<&str>> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if wide(&format!("{} {word}", line.join(" "))) <= width => line.push(word),
            _ => lines.push(vec![word]),
        }
    }
    let count = lines.len();
    if count >= 2 && lines[count - 1].len() == 1 && lines[count - 2].len() >= 2 {
        let moved = lines[count - 2].pop().unwrap_or_default();
        let candidate = format!("{moved} {}", lines[count - 1].join(" "));
        if wide(&candidate) <= width {
            lines[count - 1].insert(0, moved);
        } else {
            lines[count - 2].push(moved);
        }
    }
    lines.into_iter().map(|line| line.join(" ")).collect()
}

fn galley(ui: &Ui, role: TextRole, text: &str, width: f32) -> std::sync::Arc<egui::Galley> {
    let t = tokens(ui.ctx());
    WidgetText::from(role.rich(&t, text)).into_galley(ui, Some(TextWrapMode::Truncate), width.max(1.0), role.font(&t))
}

/// The bar over the page: the trail, ancestors muted and the current place in the action weight,
/// with the bar's own actions at its end.
pub struct TopBar<'a> {
    trail: &'a [&'a str],
    search: Option<(String, String)>,
    mark: Option<&'static str>,
    /// The crumbs drawn in the code face, by index.
    codes: Vec<usize>,
    /// What stands before the trail, such as a search field.
    start: Option<ShellBanner<'a>>,
    /// The crumbs before the last go somewhere when pressed.
    linked: bool,
}

/// The frame's top bar alone, for a screen without the shell, after sokkel-ui's `AppBar`: the
/// app's mark, the trail and the actions, in the bar the shell draws (`spacing-shell-top` tall, the
/// sidebar's ground, a strong rule under it), so the screen reads as the same app. With `desktop`
/// it is the window's title strip, as the shell's is.
pub struct AppBar<'a> {
    bar: TopBar<'a>,
    desktop: Option<DesktopWindow>,
}

impl<'a> AppBar<'a> {
    pub fn new(trail: &'a [&'a str]) -> Self {
        Self {
            bar: TopBar::new(trail),
            desktop: None,
        }
    }

    pub fn mark(mut self, svg: &'static str) -> Self {
        self.bar = self.bar.mark(svg);
        self
    }

    pub fn desktop(mut self, desktop: DesktopWindow) -> Self {
        self.desktop = Some(desktop);
        self
    }

    pub fn show(self, ui: &mut Ui, actions: impl FnOnce(&mut Ui)) -> TopBarResponse {
        let t = tokens(ui.ctx());
        let rect = Rect::from_min_size(
            ui.cursor().min,
            vec2(ui.available_width(), t.length("spacing-shell-top")),
        );
        let room = self
            .desktop
            .and_then(|d| d.traffic_lights)
            .map(|room| room.unwrap_or_else(|| t.length("spacing-traffic-lights")));
        ui.ctx().data_mut(|d| d.insert_temp(desktop_id(), room));
        if self.desktop.is_some() {
            let title = ui.interact(rect, desktop_id().with("app-bar"), Sense::click_and_drag());
            if title.double_clicked() {
                let zoomed = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!zoomed));
            } else if title.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        }
        let shown = ui
            .scope_builder(UiBuilder::new().max_rect(rect), |ui| self.bar.show(ui, actions))
            .inner;
        ui.advance_cursor_after_rect(rect);
        shown
    }
}

/// What the top bar was asked this frame.
pub struct TopBarResponse {
    pub response: Response,
    /// Its search button was pressed: open the palette.
    pub search: bool,
    /// A crumb before the last was pressed, with [`TopBar::linked`]: its index in the trail.
    pub crumb: Option<usize>,
}

impl<'a> TopBar<'a> {
    pub fn new(trail: &'a [&'a str]) -> Self {
        Self {
            trail,
            search: None,
            mark: None,
            start: None,
            codes: Vec::new(),
            linked: false,
        }
    }

    /// The crumbs before the last are links to their places, as the web's crumbs with an `href`:
    /// the pointer's hand and the foreground's ink over one, its press answered as
    /// [`TopBarResponse::crumb`].
    pub fn linked(mut self, linked: bool) -> Self {
        self.linked = linked;
        self
    }

    /// The crumb at `index` in the code face, as an addressable code is (the web's crumb
    /// `code: true`), such as an incident's `INC-41`.
    pub fn code(mut self, index: usize) -> Self {
        self.codes.push(index);
        self
    }

    /// Something before the trail, 12 before it, as the web's top bar `search`: a search field.
    pub fn start(mut self, start: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.start = Some(Box::new(start));
        self
    }

    /// The palette's button at the bar's end, a field-like button with its word and the keys that
    /// open the palette, as sokkel-ui's top bar draws it in a shell with a palette.
    pub fn search(mut self, word: impl Into<String>, keys: impl Into<String>) -> Self {
        self.search = Some((word.into(), keys.into()));
        self
    }

    /// The app's mark on a 28 tile before the trail, for a screen without the rail that carries it
    /// (an `AppBar`), as SVG drawn in `currentColor`.
    pub fn mark(mut self, svg: &'static str) -> Self {
        self.mark = Some(svg);
        self
    }

    pub fn show(self, ui: &mut Ui, actions: impl FnOnce(&mut Ui)) -> TopBarResponse {
        let t = tokens(ui.ctx());
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::ZERO, t.color("sidebar"));
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, t.color("border-strong")),
        );
        // where no rail stands between, or a collapsed one narrower than the window's buttons, the
        // bar leaves the rest of their room at its start
        let window = ui.ctx().content_rect().left();
        let lights = desktop_lights(ui.ctx()).map_or(0.0, |room| (room - (rect.left() - window)).max(0.0));
        let inner = Rect::from_min_max(
            pos2(rect.left() + 12.0 + lights, rect.top()),
            pos2(rect.right() - 12.0, rect.bottom()),
        );
        let mut searched = false;
        let mut crumb_pressed = None;
        // on a phone the last crumb alone, and the palette's button an icon
        let phone = is_phone(ui.ctx());
        let first = if phone { self.trail.len().saturating_sub(1) } else { 0 };
        let field = self.start.is_some();
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                if let Some(svg) = self.mark {
                    let (tile, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                    ui.painter().rect_filled(tile, t.radius("md"), t.color("primary"));
                    paint_mark(ui, ui.painter(), &t, tile, Some(svg), "");
                    ui.add_space(12.0 - ui.spacing().item_spacing.x);
                }
                if let Some(start) = self.start {
                    start(ui);
                    ui.add_space(12.0 - ui.spacing().item_spacing.x);
                }
                // crumbs and chevrons 4 apart
                ui.spacing_mut().item_spacing.x = 4.0;
                let last = self.trail.len().saturating_sub(1);
                for (i, crumb) in self.trail.iter().enumerate().skip(first) {
                    if i > first {
                        let (chevron, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                        icons::paint(
                            ui.painter(),
                            chevron.center(),
                            Icon::ChevronRight,
                            14.0,
                            t.color("faint"),
                        );
                    }
                    let (role, ink) = if i == last {
                        (TextRole::Action, t.color("foreground"))
                    } else {
                        (TextRole::Body, t.color("muted-foreground"))
                    };
                    let plain = role.rich(&t, *crumb);
                    let plain = if self.codes.contains(&i) {
                        let spec = t.text(role.key());
                        plain.font(crate::mono_font(ui.ctx(), crate::mono_size(&t, spec.size), spec.weight))
                    } else {
                        plain
                    };
                    let rich = plain.clone().color(ink);
                    if self.linked && i != last {
                        // drawn in its own ink, which the pointer over it changes, in the code face
                        // when it is a code
                        let galley = WidgetText::from(plain).into_galley(
                            ui,
                            Some(TextWrapMode::Extend),
                            f32::INFINITY,
                            role.font(&t),
                        );
                        let (place, hit) = ui.allocate_exact_size(galley.size(), Sense::click());
                        hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, *crumb));
                        let ink = if hit.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            t.color("foreground")
                        } else {
                            ink
                        };
                        ui.painter().galley(place.min, galley, ink);
                        if hit.clicked() {
                            crumb_pressed = Some(i);
                        }
                    } else {
                        ui.label(rich);
                    }
                }
                // from the end: the actions last, 6 apart, then the palette's button 12 before them
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let before = ui.cursor().right();
                    actions(ui);
                    if ui.cursor().right() < before {
                        ui.add_space(6.0);
                    }
                    if let Some((word, keys)) = &self.search {
                        searched = if phone {
                            search_icon(ui, word).clicked()
                        } else if field {
                            palette_keys_button(ui, word, keys).clicked()
                        } else {
                            search_button(ui, word, keys).clicked()
                        };
                    }
                });
            },
        );
        TopBarResponse {
            response,
            search: searched,
            crumb: crumb_pressed,
        }
    }
}

/// The palette's button on a phone: a magnifier on a finger square, with no edge or ground.
fn search_icon(ui: &mut Ui, word: &str) -> Response {
    let t = tokens(ui.ctx());
    let finger = t.length("spacing-finger");
    let (rect, response) = ui.allocate_exact_size(vec2(finger, finger), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word));
    phone_control(ui.painter(), &t, rect, &response, Icon::Search);
    response
}

/// The palette's button beside a search field at the bar's start, as the web's: a control tall,
/// 8 at each side, a field's ground and edge, and the keys alone in the code face, small.
fn palette_keys_button(ui: &mut Ui, word: &str, keys: &str) -> Response {
    let t = tokens(ui.ctx());
    let spec = t.text(TextRole::Help.key());
    let font = crate::mono_font(ui.ctx(), crate::mono_size(&t, spec.size), spec.weight);
    // the code face has no ⌘: it is drawn as the icon, smaller, as the web's fallback face draws it
    let glyph = font.size * 0.75;
    let commands = keys.matches('⌘').count();
    let rest = keys.replace('⌘', "");
    let text = ui.painter().layout_no_wrap(rest, font, egui::Color32::PLACEHOLDER);
    let width = commands as f32 * glyph + text.size().x;
    let (rect, response) = ui.allocate_exact_size(vec2(width + 16.0 + 2.0, t.density("control")), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word));
    let hot = response.hovered();
    let edge = if response.has_focus() {
        t.color("ring")
    } else if hot {
        t.color("border-strong")
    } else {
        t.color("border")
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        t.radius("control"),
        t.color("field"),
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    let ink = if hot {
        t.color("foreground")
    } else {
        t.color("muted-foreground")
    };
    let mut x = rect.center().x - width / 2.0;
    for _ in 0..commands {
        icons::paint(
            painter,
            pos2(x + glyph / 2.0, rect.center().y),
            Icon::Command,
            glyph,
            ink,
        );
        x += glyph;
    }
    painter.galley(pos2(x, rect.center().y - text.size().y / 2.0), text, ink);
    response.on_hover_text(word)
}

/// The palette's button: 224 wide, a control tall, a field's ground and edge, a magnifier, the word
/// and the keys in a key cap at its end.
fn search_button(ui: &mut Ui, word: &str, keys: &str) -> Response {
    let t = tokens(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(224.0, t.density("control")), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word));
    let hot = response.hovered();
    let focused = response.has_focus();
    let painter = ui.painter();
    let edge = if focused {
        t.color("ring")
    } else if hot {
        t.color("border-strong")
    } else {
        t.color("border")
    };
    painter.rect(
        rect,
        t.radius("control"),
        t.color("field"),
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    let ink = if hot {
        t.color("foreground")
    } else {
        t.color("muted-foreground")
    };
    icons::paint(
        painter,
        pos2(rect.left() + 10.0 + 8.0, rect.center().y),
        Icon::Search,
        16.0,
        ink,
    );
    let label = galley(ui, TextRole::Body, word, rect.width() - 90.0);
    painter.galley(
        pos2(rect.left() + 34.0, rect.center().y - label.size().y / 2.0),
        label,
        ink,
    );
    // the key cap: muted ground, the small radius, the help size
    let cap_text = super::keys_galley(ui, &t, TextRole::Badge, keys, 0.0);
    let cap = Rect::from_min_size(
        pos2(rect.right() - 6.0 - cap_text.size().x - 10.0, rect.center().y - 10.0),
        vec2(cap_text.size().x + 10.0, 20.0),
    );
    painter.rect_filled(cap, t.radius("sm"), t.color("muted"));
    painter.galley(
        pos2(cap.left() + 5.0, cap.center().y - cap_text.size().y / 2.0),
        cap_text,
        t.color("muted-foreground"),
    );
    // keyboard focus: the edge in the ring's color and a 1px ring around it
    if focused {
        painter.rect_stroke(
            rect,
            t.radius("control"),
            Stroke::new(1.0, t.color("ring")),
            egui::StrokeKind::Outside,
        );
    }
    response
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
    live: bool,
    facts: Vec<String>,
    /// A labelled value, and whether the value is code.
    items: Vec<(String, String, bool)>,
}

impl StatusLine {
    pub fn new(note: impl Into<String>) -> Self {
        Self {
            note: note.into(),
            tone: StatusTone::Quiet,
            live: false,
            facts: Vec::new(),
            items: Vec::new(),
        }
    }

    pub fn tone(mut self, tone: StatusTone) -> Self {
        self.tone = tone;
        self
    }

    /// The note says something happening now: a dot in its ink before it, pulsing.
    pub fn live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// A labelled value at the end, as `StatusItem`: the label faint, the value in the foot's ink.
    pub fn item(mut self, label: impl Into<String>, value: impl ToString) -> Self {
        self.items.push((label.into(), value.to_string(), false));
        self
    }

    /// A labelled value that is code, such as a version: the value in the foreground's code face.
    pub fn item_code(mut self, label: impl Into<String>, value: impl ToString) -> Self {
        self.items.push((label.into(), value.to_string(), true));
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
        // the facts first, from the end, 12 in and 12 apart, in at most 70% of the line (`max-w-[70%]`),
        // a value cut short only where they would pass it; the note takes what they leave
        let mut right = rect.right() - 12.0;
        let least = rect.right() - 12.0 - rect.width() * 0.7;
        for (label, value, code) in self.items.iter().rev() {
            let label_width = galley(ui, TextRole::Help, label, 160.0).size().x;
            let room = (right - least - label_width - 6.0).max(48.0);
            let value = if *code {
                let rich = egui::RichText::new(value).font(egui::FontId::monospace(TextRole::Help.font(&t).size));
                WidgetText::from(rich).into_galley(ui, Some(TextWrapMode::Truncate), room, TextRole::Help.font(&t))
            } else {
                galley(ui, TextRole::Help, value, room)
            };
            right -= value.size().x;
            let value_ink = if *code { t.color("foreground") } else { quiet };
            painter.galley(pos2(right, rect.center().y - value.size().y / 2.0), value, value_ink);
            let label = galley(ui, TextRole::Help, label, 160.0);
            right -= 6.0 + label.size().x;
            painter.galley(
                pos2(right, rect.center().y - label.size().y / 2.0),
                label,
                t.color("faint"),
            );
            right -= 12.0;
        }
        for fact in self.facts.iter().rev() {
            let text = galley(ui, TextRole::Help, fact, 240.0);
            right -= text.size().x;
            painter.galley(pos2(right, rect.center().y - text.size().y / 2.0), text, quiet);
            right -= 12.0;
        }
        let mut x = rect.left() + 12.0;
        if self.live {
            let center = pos2(x + 4.0, rect.center().y);
            painter.circle_filled(center, 4.0, ink);
            // a second dot at half strength pulsing out from it, as `animate-ping`: twice its size and
            // gone by three quarters of each second; still while motion is off. Each frame redraws
            // the window, so the pulse asks for 20 a second while it shows and none in the quarter
            // second it is gone
            if ui.style().animation_time > 0.0 {
                let phase = ui.input(|i| i.time) % 1.0;
                if phase < 0.75 {
                    let p = (phase / 0.75) as f32;
                    // cubic-bezier(0, 0, 0.2, 1), near enough: a cubic ease out
                    let eased = 1.0 - (1.0 - p).powi(3);
                    painter.circle_filled(center, 4.0 * (1.0 + eased), ink.gamma_multiply(0.5 * (1.0 - eased)));
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
                } else {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_secs_f64(1.0 - phase));
                }
            }
            x += 16.0;
        }
        let note = galley(ui, TextRole::Help, &self.note, (right - x).max(1.0));
        painter.galley(pos2(x, rect.center().y - note.size().y / 2.0), note, ink);
        response
    }
}

/// The room the page's scroll bar keeps at the page's end, as a thin bar's stable gutter.
pub const SCROLL_GUTTER: f32 = 11.0;

/// The window is a phone's or a folded foldable's: narrower than 768, sokkel-ui's `md`. Below it
/// the rail is a drawer, the top bar keeps its last crumb and its palette button is an icon, and
/// rows are a finger tall.
pub fn is_phone(ctx: &egui::Context) -> bool {
    ctx.content_rect().width() < 768.0
}

/// Whether the shell's drawer is open, and whether the rail is being drawn in it.
#[derive(Clone, Copy, Debug, Default)]
struct Drawer {
    open: bool,
    drawing: bool,
}

fn drawer_id() -> egui::Id {
    egui::Id::unique("sokkel-shell-drawer")
}

fn drawer(ctx: &egui::Context) -> Drawer {
    ctx.data(|d| d.get_temp(drawer_id())).unwrap_or_default()
}

fn set_drawer(ctx: &egui::Context, change: impl FnOnce(&mut Drawer)) {
    let mut state = drawer(ctx);
    change(&mut state);
    ctx.data_mut(|d| d.insert_temp(drawer_id(), state));
}

/// The words the shell says, English by default, as sokkel-ui's `ShellWords`.
#[derive(Clone, Debug)]
pub struct ShellWords {
    /// The top bar's menu button, which opens the rail as a drawer below 768.
    pub menu: String,
    /// What closes the drawer: its scrim and the rail head's control in it.
    pub close: String,
    /// The toggle in an open side panel's corner, and its edge's drag.
    pub hide_panel: String,
    pub resize_panel: String,
    /// The top bar's toggle that opens the side panel while it is shut.
    pub show_panel: String,
}

impl Default for ShellWords {
    fn default() -> Self {
        Self {
            menu: "Open the navigation".into(),
            close: "Close the navigation".into(),
            hide_panel: "Hide the panel".into(),
            resize_panel: "Resize the side panel".into(),
            show_panel: "Show the panel".into(),
        }
    }
}

/// Which part of the frame [`Shell::show`] is asking for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellPart {
    Rail,
    /// The cell at the rail's foot, level with the status line: the account.
    RailFoot,
    Top,
    Page,
    Foot,
}

fn desktop_id() -> egui::Id {
    egui::Id::unique("sokkel-shell-desktop")
}

/// Room at the frame's top start for the window's own buttons, where the shell stands in a desktop
/// window whose title bar is hidden (`Shell::desktop`).
fn desktop_lights(ctx: &egui::Context) -> Option<f32> {
    ctx.data(|d| d.get_temp::<Option<f32>>(desktop_id())).flatten()
}

/// A desktop window whose native title bar is hidden, its content running under where the bar
/// was, as sokkel-ui's desktop shells open theirs (Tauri's `Overlay`, Electron's `hiddenInset`):
/// the shell's top strip is then the window's title bar (`Shell::desktop`).
pub fn desktop_viewport(viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
    viewport
        .with_fullsize_content_view(true)
        .with_titlebar_shown(false)
        .with_title_shown(false)
}

/// How the frame stands in a desktop window whose native title bar is hidden, after sokkel-ui's
/// `DesktopWindow`: the frame's top strip is then the window's title bar.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DesktopWindow {
    /// Room at the strip's start for the window's own buttons, which macOS draws over the frame:
    /// `spacing-traffic-lights` (80) when `Some` without a width of its own. `None` off macOS.
    pub traffic_lights: Option<Option<f32>>,
}

impl DesktopWindow {
    /// The window's buttons drawn over the frame's top start on macOS, nothing elsewhere.
    pub fn native() -> Self {
        Self {
            traffic_lights: cfg!(target_os = "macos").then_some(None),
        }
    }
}

/// The frame: the rail at the start, the top bar over the page, and the status line as one bar
/// across the bottom, within the rect it is given.
pub struct Shell<'a> {
    collapsed: bool,
    framed: bool,
    desktop: Option<DesktopWindow>,
    words: ShellWords,
    banner: Option<ShellBanner<'a>>,
    panel: Option<(&'a mut bool, ShellBanner<'a>)>,
    panel_width: f32,
    /// What the top bar leaves at its end for the shut panel's toggle.
    top_inset: f32,
    bottom: Option<ShellBanner<'a>>,
}

/// What a shell's banner line draws.
type ShellBanner<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

impl Default for Shell<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Shell<'a> {
    pub fn new() -> Self {
        Self {
            collapsed: false,
            framed: false,
            desktop: None,
            words: ShellWords::default(),
            banner: None,
            panel: None,
            panel_width: 352.0,
            top_inset: 0.0,
            bottom: None,
        }
    }

    /// A strip docked across the whole window between the content and the status line, under the
    /// rail and the side panel, as the web's `bottom`: for what belongs to the whole app rather than
    /// one screen, such as a player. It is as tall as what it draws, measured as it is drawn, so a
    /// player grown from its top edge takes its room from the rail and the page; the rail's foot cell
    /// stays level with the status line under it.
    pub fn bottom(mut self, bottom: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.bottom = Some(Box::new(bottom));
        self
    }

    /// Draws the bottom strip standing on `floor` across `left..right`, as tall as it was when last
    /// drawn, and answers with where its top is. A strip drawn another height than it was asks for
    /// the frame to be laid out again.
    fn place_bottom(&mut self, ui: &mut Ui, left: f32, right: f32, floor: f32) -> f32 {
        let Some(bottom) = self.bottom.take() else {
            return floor;
        };
        let held_id = egui::Id::unique("sokkel-shell-bottom");
        let held: f32 = ui.ctx().data(|d| d.get_temp(held_id)).unwrap_or(0.0);
        let top = floor - held;
        let area = Rect::from_min_max(pos2(left, top), pos2(right, floor.max(top + 1.0)));
        let drawn = ui
            .scope_builder(
                UiBuilder::new().max_rect(area).layout(Layout::top_down(Align::Min)),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    bottom(ui);
                },
            )
            .response
            .rect;
        let height = drawn.height().max(0.0);
        if (height - held).abs() > 0.5 {
            ui.ctx().data_mut(|d| d.insert_temp(held_id, height));
            ui.ctx().request_discard("the shell's bottom strip changed its height");
        }
        top
    }

    /// A side panel as the frame's right column while `open`, after the web's `Shell` `panel`: the
    /// window's height down to the status line, its head level with the top bar (a `SidePanel`
    /// fills it), `panel_width` wide at first (352) and dragged wider or narrower by its start edge, with the
    /// toggle that hides it in its head's corner, which sets `open` false.
    pub fn panel(mut self, open: &'a mut bool, panel: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.panel = Some((open, Box::new(panel)));
        self
    }

    /// The width the side panel opens at and a double press on its edge puts back, as the web's
    /// `SidePanel` `defaultSize`; 352 when not given.
    pub fn panel_width(mut self, width: f32) -> Self {
        self.panel_width = width;
        self
    }

    /// Draws the open side panel at the end of `column` and answers with what is left of it.
    fn place_panel(&mut self, ui: &mut Ui, column: Rect) -> Rect {
        let Some((open, panel)) = self.panel.take() else {
            return column;
        };
        if !*open {
            // shut, its toggle stands at the top bar's end, a control square 8 from the edge, and
            // the bar keeps clear of it
            let t = tokens(ui.ctx());
            let top = t.length("spacing-shell-top");
            let control = t.density("control");
            let toggle = Rect::from_center_size(
                pos2(column.right() - 8.0 - control / 2.0, column.top() + top / 2.0),
                vec2(control, control),
            );
            // the top bar's ground and rule under it, as the bar beside it has them
            let strip = Rect::from_min_max(
                pos2(column.right() - control - 8.0, column.top()),
                pos2(column.right(), column.top() + top),
            );
            ui.painter().rect_filled(strip, CornerRadius::ZERO, t.color("sidebar"));
            ui.painter().hline(
                strip.x_range(),
                strip.bottom() - 0.5,
                Stroke::new(1.0, t.color("border-strong")),
            );
            let hit = ui.interact(toggle, egui::Id::unique("sokkel-shell-panel-show"), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.show_panel));
            let ink = if hit.hovered() {
                ui.painter().rect_filled(toggle, t.radius("md"), t.color("hover"));
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            icons::paint(ui.painter(), toggle.center(), Icon::PanelRight, 16.0, ink);
            if hit.clicked() {
                *open = true;
            }
            self.top_inset = control + 8.0;
            return column;
        }
        let t = tokens(ui.ctx());
        let size_id = egui::Id::unique("sokkel-shell-panel-size");
        let width: f32 = ui
            .ctx()
            .data(|d| d.get_temp(size_id))
            .unwrap_or(self.panel_width)
            .clamp(280.0, (column.width() - 320.0).max(280.0));
        let area = Rect::from_min_max(pos2(column.right() - width, column.top()), column.max);
        ui.painter().rect_filled(area, CornerRadius::ZERO, t.color("sidebar"));
        ui.scope_builder(
            UiBuilder::new().max_rect(area).layout(Layout::top_down(Align::Min)),
            |ui| {
                ui.set_clip_rect(area.intersect(ui.clip_rect()));
                panel(ui);
            },
        );
        ui.painter().vline(
            area.left() + 0.5,
            area.y_range(),
            Stroke::new(1.0, t.color("border-strong")),
        );
        // the toggle that hides it, in its head's corner
        let top = t.length("spacing-shell-top");
        let toggle = Rect::from_center_size(pos2(area.right() - 24.0, area.top() + top / 2.0), vec2(28.0, 28.0));
        let hit = ui.interact(toggle, size_id.with("toggle"), Sense::click());
        hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.hide_panel));
        if hit.hovered() {
            ui.painter().rect_filled(toggle, t.radius("md"), t.color("hover"));
        }
        icons::paint(
            ui.painter(),
            toggle.center(),
            Icon::PanelRightClose,
            16.0,
            t.color("muted-foreground"),
        );
        if hit.clicked() {
            *open = false;
        }
        // its start edge drags it wider or narrower; a double press puts it back
        let edge = Rect::from_min_max(
            pos2(area.left() - 4.0, area.top()),
            pos2(area.left() + 4.0, area.bottom()),
        );
        let grip = ui.interact(edge, size_id.with("edge"), Sense::click_and_drag());
        grip.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Slider, true, &self.words.resize_panel));
        if grip.hovered() || grip.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if grip.dragged() {
            let next = (width - grip.drag_delta().x).max(280.0);
            ui.ctx().data_mut(|d| d.insert_temp(size_id, next));
        }
        if grip.double_clicked() {
            ui.ctx().data_mut(|d| d.remove::<f32>(size_id));
        }
        Rect::from_min_max(column.min, pos2(area.left(), column.bottom()))
    }

    /// A line across the page's column between the top bar and the page, such as a `Banner` saying
    /// the server is not answering, as the web's `banner`: the page stands under it.
    pub fn banner(mut self, banner: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.banner = Some(Box::new(banner));
        self
    }

    /// Draws the banner across the top of `page` and answers with the page under it.
    fn place_banner(&mut self, ui: &mut Ui, page: Rect) -> Rect {
        let Some(banner) = self.banner.take() else {
            return page;
        };
        let drawn = ui
            .scope_builder(
                UiBuilder::new().max_rect(page).layout(Layout::top_down(Align::Min)),
                |ui| {
                    ui.set_clip_rect(page.intersect(ui.clip_rect()));
                    ui.spacing_mut().item_spacing.y = 0.0;
                    banner(ui);
                },
            )
            .response
            .rect;
        Rect::from_min_max(pos2(page.left(), drawn.bottom().min(page.bottom())), page.max)
    }

    /// The frame stands in a desktop window whose native title bar is hidden (eframe's
    /// `with_titlebar_shown(false)` and `with_fullsize_content_view(true)`): its top strip, the
    /// rail's head and the top bar, is the window's title bar, dragged to move the window and
    /// double-clicked to zoom it, and with `traffic_lights` the rail's head leaves room at its start
    /// for the window's own buttons.
    pub fn desktop(mut self, desktop: DesktopWindow) -> Self {
        self.desktop = Some(desktop);
        self
    }

    pub fn words(mut self, words: ShellWords) -> Self {
        self.words = words;
        self
    }

    /// Lay the frame out for a collapsed rail.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// An edge around the whole frame, for a frame drawn inside a page (a story) rather than a window.
    pub fn framed(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }

    /// Fill the whole window with the frame, to its edges, whatever margin the `Ui` it is given has:
    /// an app's frame.
    pub fn show_window(self, ui: &mut Ui, part: impl FnMut(ShellPart, &mut Ui)) {
        let window = ui.ctx().content_rect();
        let mut child = ui.new_child(UiBuilder::new().max_rect(window).layout(Layout::top_down(Align::Min)));
        child.set_clip_rect(window);
        self.show(&mut child, window, part);
    }

    /// Fill `rect` with the frame, asking `part` to draw each piece in its place.
    pub fn show(mut self, ui: &mut Ui, rect: Rect, mut part: impl FnMut(ShellPart, &mut Ui)) {
        {
            let t = tokens(ui.ctx());
            let top = t.length("spacing-shell-top");
            // in a desktop window the top strip is its title bar: sensed before the parts, so every
            // control drawn in the strip after it takes its own presses
            let room = self
                .desktop
                .and_then(|d| d.traffic_lights)
                .map(|room| room.unwrap_or_else(|| t.length("spacing-traffic-lights")));
            ui.ctx().data_mut(|d| d.insert_temp(desktop_id(), room));
            if self.desktop.is_some() {
                let strip = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + top));
                let title = ui.interact(strip, desktop_id().with("strip"), Sense::click_and_drag());
                if title.double_clicked() {
                    let zoomed = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!zoomed));
                } else if title.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            }
        }
        if is_phone(ui.ctx()) {
            self.show_phone(ui, rect, part);
            return;
        }
        set_drawer(ui.ctx(), |d| d.open = false);
        let t = tokens(ui.ctx());
        let foot = t.length("spacing-shell-foot");
        let top = t.length("spacing-shell-top");
        let rail_width = Rail::width(self.collapsed);
        // the bottom strip across the window over the foot row, the rail and the column over it
        let floor = self.place_bottom(ui, rect.left(), rect.right(), rect.bottom() - foot);
        // the rail runs the window's height, its foot cell level with the status line under the page
        let rail = Rect::from_min_max(rect.min, pos2(rect.left() + rail_width, floor));
        let rail_foot = Rect::from_min_max(
            pos2(rect.left(), rect.bottom() - foot),
            pos2(rect.left() + rail_width, rect.bottom()),
        );
        // the content's column, less the side panel at its end while it is open
        let column = Rect::from_min_max(pos2(rail.right(), rect.top()), pos2(rect.right(), floor));
        let column = self.place_panel(ui, column);
        let top_bar = Rect::from_min_max(column.min, pos2(column.right() - self.top_inset, rect.top() + top));
        let page = Rect::from_min_max(pos2(column.left(), top_bar.bottom()), column.max);
        ui.painter()
            .rect_filled(page, CornerRadius::ZERO, t.color("background"));
        let page = self.place_banner(ui, page);
        // the page's scroller keeps a thin track at its end whether or not it scrolls, as the web's
        // `scrollbar-gutter: stable` with a thin bar: its content stands 11 short of the window's edge
        let page_content = Rect::from_min_max(page.min, pos2(page.right() - SCROLL_GUTTER, page.bottom()));
        let foot_strip = Rect::from_min_max(pos2(rail.right(), rect.bottom() - foot), rect.max);
        ui.painter()
            .rect_filled(page, CornerRadius::ZERO, t.color("background"));
        for (which, area) in [
            (ShellPart::Rail, rail),
            (ShellPart::RailFoot, rail_foot),
            (ShellPart::Top, top_bar),
            (ShellPart::Page, page_content),
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
        if self.framed {
            ui.painter().rect_stroke(
                rect,
                CornerRadius::ZERO,
                Stroke::new(1.0, t.color("border-strong")),
                egui::StrokeKind::Inside,
            );
        }
        ui.allocate_rect(rect, Sense::hover());
    }

    /// Below 768: no rail in the frame; the top bar, the page and the status line across the
    /// window, and the top bar's menu button opening the rail as a drawer over them, expanded, with
    /// the account at its foot. A press on the scrim, Escape, the drawer's close or a place chosen
    /// in it closes it.
    fn show_phone(mut self, ui: &mut Ui, rect: Rect, mut part: impl FnMut(ShellPart, &mut Ui)) {
        let t = tokens(ui.ctx());
        let ctx = ui.ctx().clone();
        let foot = t.length("spacing-shell-foot");
        let top = t.length("spacing-shell-top");
        let finger = t.length("spacing-finger");
        let top_bar = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + top));
        let floor = self.place_bottom(ui, rect.left(), rect.right(), rect.bottom() - foot);
        let page = Rect::from_min_max(pos2(rect.left(), top_bar.bottom()), pos2(rect.right(), floor));
        ui.painter()
            .rect_filled(page, CornerRadius::ZERO, t.color("background"));
        let page = self.place_banner(ui, page);
        let page_content = Rect::from_min_max(page.min, pos2(page.right() - SCROLL_GUTTER, page.bottom()));
        let foot_strip = Rect::from_min_max(pos2(rect.left(), rect.bottom() - foot), rect.max);
        ui.painter()
            .rect_filled(page, CornerRadius::ZERO, t.color("background"));
        // the top bar's ground and rule under the menu button, then its own row after it, 8 on
        ui.painter()
            .rect_filled(top_bar, CornerRadius::ZERO, t.color("sidebar"));
        // after the window's buttons, where they stand over the bar
        let start = desktop_lights(&ctx).unwrap_or(12.0).max(12.0);
        let menu = Rect::from_min_size(
            pos2(top_bar.left() + start, top_bar.center().y - finger / 2.0),
            vec2(finger, finger),
        );
        let menu_hit = ui.interact(menu, drawer_id().with("menu"), Sense::click());
        menu_hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.menu));
        // as the rail's head control looks, its focus outline inside it
        phone_control(ui.painter(), &t, menu, &menu_hit, Icon::Menu);
        if menu_hit.clicked() {
            set_drawer(&ctx, |d| d.open = true);
        }
        let row = Rect::from_min_max(pos2(menu.right() + 8.0 - 12.0, top_bar.top()), top_bar.max);
        for (which, area) in [
            (ShellPart::Top, row),
            (ShellPart::Page, page_content),
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
        ui.painter().hline(
            top_bar.x_range(),
            top_bar.bottom() - 0.5,
            Stroke::new(1.0, t.color("border-strong")),
        );
        ui.allocate_rect(rect, Sense::hover());

        if !drawer(&ctx).open {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            set_drawer(&ctx, |d| d.open = false);
            return;
        }
        egui::Area::new(drawer_id().with("area"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(&ctx, |ui| {
                // the scrim over the frame, a press on it closing the drawer
                let scrim = ui.interact(rect, drawer_id().with("scrim"), Sense::click());
                scrim.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.close));
                ui.painter().rect_filled(rect, CornerRadius::ZERO, t.color("scrim"));
                if scrim.clicked() {
                    set_drawer(&ctx, |d| d.open = false);
                }
                // the drawer: the rail expanded, the window's height, its account at the foot
                let width = Rail::width(false);
                let panel = Rect::from_min_size(rect.min, vec2(width, rect.height()));
                // the drawer wears the popups' elevation, as the web's `shadow-pop`
                let shadow = t.shadow("elevation-pop");
                ui.painter().add(shadow.as_shape(panel, CornerRadius::ZERO));
                ui.painter().rect_filled(panel, CornerRadius::ZERO, t.color("sidebar"));
                let rail = Rect::from_min_max(panel.min, pos2(panel.right(), panel.bottom() - foot));
                let rail_foot = Rect::from_min_max(pos2(panel.left(), panel.bottom() - foot), panel.max);
                set_drawer(&ctx, |d| d.drawing = true);
                for (which, area) in [(ShellPart::Rail, rail), (ShellPart::RailFoot, rail_foot)] {
                    ui.scope_builder(
                        UiBuilder::new().max_rect(area).layout(Layout::top_down(Align::Min)),
                        |ui| {
                            ui.set_clip_rect(area);
                            part(which, ui);
                        },
                    );
                }
                set_drawer(&ctx, |d| d.drawing = false);
                ui.painter().hline(
                    panel.x_range(),
                    rail_foot.top() + 0.5,
                    Stroke::new(1.0, t.color("border-strong")),
                );
            });
    }
}
