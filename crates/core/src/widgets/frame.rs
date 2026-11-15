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
}

impl RailEntry {
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Icon) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            count: None,
            description: None,
        }
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
}

/// The words a rail says.
#[derive(Clone, Debug)]
pub struct RailWords {
    pub collapse: String,
    pub expand: String,
}

impl Default for RailWords {
    fn default() -> Self {
        Self {
            collapse: "Collapse the navigation".into(),
            expand: "Expand the navigation".into(),
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
        }
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
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &self.words.expand));
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
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &self.words.collapse));
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
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, ShellWords::default().close));
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
        // a first group without a title keeps 4 over its first entry, so the entry's focus is whole
        let untitled_top = if self.groups.first().is_some_and(|g| g.title.is_none()) {
            4.0
        } else {
            0.0
        };
        // collapsed under the window's buttons, the mark's own line (`py-1`) stands over the groups
        let mut y = head.bottom() + untitled_top + if under { 40.0 } else { 0.0 };
        for (index, group) in self.groups.into_iter().enumerate() {
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
            for entry in &group.entries {
                // a described entry: 6 over and under its 22 label line and its 20 description
                let described = !narrow && entry.description.is_some();
                let height = if described { 6.0 + 22.0 + 20.0 + 6.0 } else { row };
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
                let active = *self.active == entry.id;
                hit.widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, &entry.label)
                });
                if hit.clicked() {
                    *self.active = entry.id.clone();
                    // a place chosen in the drawer closes it, as a change of route does
                    set_drawer(ui.ctx(), |d| d.open = false);
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
                // the frame's one keyboard focus look, the button's
                super::focus_ring(&painter, &t, bounds, t.radius("md"), &hit);
                let ink = if active {
                    t.color("sidebar-accent-foreground")
                } else if hit.hovered() {
                    t.color("foreground")
                } else {
                    t.color("muted-foreground")
                };
                if narrow {
                    icons::paint(&painter, bounds.center(), entry.icon, 16.0, ink);
                } else {
                    icons::paint(&painter, pos2(bounds.left() + 12.0 + 8.0, line), entry.icon, 16.0, ink);
                    let role = if active { TextRole::Action } else { TextRole::Body };
                    let label = galley(ui, role, &entry.label, bounds.width() - 80.0);
                    painter.galley(pos2(bounds.left() + 40.0, line - label.size().y / 2.0), label, ink);
                    if let Some(description) = entry.description.as_deref().filter(|_| described) {
                        let words = galley(ui, TextRole::Help, description, bounds.width() - 52.0);
                        painter.galley(
                            pos2(
                                bounds.left() + 40.0,
                                bounds.top() + 6.0 + 22.0 + 10.0 - words.size().y / 2.0,
                            ),
                            words,
                            t.color("muted-foreground"),
                        );
                    }
                    if let Some(count) = entry.count {
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
                y += if narrow { row } else { height } + 4.0;
            }
        }
        response
    }
}

/// The app's mark on its tile's ink: the SVG, drawn in `currentColor`, or the name's initial.
fn paint_mark(ui: &Ui, painter: &egui::Painter, t: &crate::Tokens, tile: Rect, svg: Option<&str>, name: &str) {
    match svg {
        Some(svg) => {
            // the mark's ink is `currentColor`: drawn white, then tinted with the tile's ink
            let white = svg.replace("currentColor", "#ffffff");
            let uri = format!("bytes://rail-mark-{}.svg", egui::Id::new(svg).value());
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
}

impl<'a> TopBar<'a> {
    pub fn new(trail: &'a [&'a str]) -> Self {
        Self {
            trail,
            search: None,
            mark: None,
        }
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
        // on a phone the last crumb alone, and the palette's button an icon
        let phone = is_phone(ui.ctx());
        let first = if phone { self.trail.len().saturating_sub(1) } else { 0 };
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
                    ui.label(role.rich(&t, *crumb).color(ink));
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
        }
    }
}

/// The palette's button on a phone: a magnifier on a finger square, with no edge or ground.
fn search_icon(ui: &mut Ui, word: &str) -> Response {
    let t = tokens(ui.ctx());
    let finger = t.length("spacing-finger");
    let (rect, response) = ui.allocate_exact_size(vec2(finger, finger), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, word));
    phone_control(ui.painter(), &t, rect, &response, Icon::Search);
    response
}

/// The palette's button: 224 wide, a control tall, a field's ground and edge, a magnifier, the word
/// and the keys in a key cap at its end.
fn search_button(ui: &mut Ui, word: &str, keys: &str) -> Response {
    let t = tokens(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(224.0, t.density("control")), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, word));
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
    let cap_text = galley(ui, TextRole::Badge, keys, 80.0);
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
        // the facts first, from the end, 12 in and 12 apart; the note takes what they leave
        let mut right = rect.right() - 12.0;
        for (label, value, code) in self.items.iter().rev() {
            let value = if *code {
                let rich = egui::RichText::new(value).font(egui::FontId::monospace(TextRole::Help.font(&t).size));
                WidgetText::from(rich).into_galley(ui, Some(TextWrapMode::Truncate), 120.0, TextRole::Help.font(&t))
            } else {
                galley(ui, TextRole::Help, value, 120.0)
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
    egui::Id::new("sokkel-shell-drawer")
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
}

impl Default for ShellWords {
    fn default() -> Self {
        Self {
            menu: "Open the navigation".into(),
            close: "Close the navigation".into(),
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
    egui::Id::new("sokkel-shell-desktop")
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
pub struct Shell {
    collapsed: bool,
    framed: bool,
    desktop: Option<DesktopWindow>,
    words: ShellWords,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    pub fn new() -> Self {
        Self {
            collapsed: false,
            framed: false,
            desktop: None,
            words: ShellWords::default(),
        }
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
    pub fn show(self, ui: &mut Ui, rect: Rect, mut part: impl FnMut(ShellPart, &mut Ui)) {
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
        // the rail runs the window's height, its foot cell level with the status line under the page
        let rail = Rect::from_min_max(rect.min, pos2(rect.left() + rail_width, rect.bottom() - foot));
        let rail_foot = Rect::from_min_max(
            pos2(rect.left(), rect.bottom() - foot),
            pos2(rect.left() + rail_width, rect.bottom()),
        );
        let top_bar = Rect::from_min_max(pos2(rail.right(), rect.top()), pos2(rect.right(), rect.top() + top));
        let page = Rect::from_min_max(
            pos2(rail.right(), top_bar.bottom()),
            pos2(rect.right(), rect.bottom() - foot),
        );
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
    fn show_phone(self, ui: &mut Ui, rect: Rect, mut part: impl FnMut(ShellPart, &mut Ui)) {
        let t = tokens(ui.ctx());
        let ctx = ui.ctx().clone();
        let foot = t.length("spacing-shell-foot");
        let top = t.length("spacing-shell-top");
        let finger = t.length("spacing-finger");
        let top_bar = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + top));
        let page = Rect::from_min_max(
            pos2(rect.left(), top_bar.bottom()),
            pos2(rect.right(), rect.bottom() - foot),
        );
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
        menu_hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &self.words.menu));
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
                scrim.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &self.words.close));
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
