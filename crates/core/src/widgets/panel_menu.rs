//! A dashboard panel's own tools, after sokkel-ui's `DashboardPanel` with `menu` and `expandable`:
//! an expand button and a menu at the end of its head, and the window an expanded panel's content
//! opens in, as its `WindowedPane` with `fullscreen`.

use egui::{Context, Id, Rect, Sense, Stroke, TextWrapMode, Ui, UiBuilder, WidgetText, vec2};

use super::{DropdownMenu, IconButton, MenuItem};
use crate::{Icon, TextRole, tokens};

/// The words a panel's tools and its window say.
#[derive(Clone, Debug)]
pub struct PanelWords {
    /// The expand button's label.
    pub expand: String,
    /// The menu button's label.
    pub menu: String,
    /// The window's close button's label.
    pub close: String,
    /// Said in the panel's place while its content is in the window.
    pub away: String,
}

impl Default for PanelWords {
    fn default() -> Self {
        Self {
            expand: "Expand".into(),
            menu: "Panel actions".into(),
            close: "Close".into(),
            away: "Shown in a window.".into(),
        }
    }
}

/// What a press on a panel's tools asked for this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelAsked {
    /// The expand button: open the panel's content in its window.
    Expand,
    /// The menu's item at this index.
    Item(usize),
}

/// What a panel's head draws as its tools, put where its cell's panel reads it.
#[derive(Clone, Debug)]
struct HeadTools {
    items: Vec<MenuItem>,
    expandable: bool,
    expanded: bool,
    words: PanelWords,
}

/// A panel's own tools: `items` behind its more button, and with `expandable` a button that opens
/// its content in a window. Wraps the drawing of any dashboard panel (a `DashboardPanel`, a
/// `StatPanel`, a chart's panel), whose head draws them at its end, before an arrangement's tools.
pub struct PanelMenu {
    head: HeadTools,
}

impl PanelMenu {
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self {
            head: HeadTools {
                items,
                expandable: false,
                expanded: false,
                words: PanelWords::default(),
            },
        }
    }

    /// An expand button before the menu, which opens the content in its window.
    pub fn expandable(mut self, expandable: bool) -> Self {
        self.head.expandable = expandable;
        self
    }

    /// The content is in its window now: the panel says so in its place.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.head.expanded = expanded;
        self
    }

    pub fn words(mut self, words: PanelWords) -> Self {
        self.head.words = words;
        self
    }

    /// Draw the panel `panel` draws in `ui`, with these tools in its head; what they asked for.
    pub fn show(self, ui: &mut Ui, panel: impl FnOnce(&mut Ui)) -> Option<PanelAsked> {
        let cell = ui.scope_id();
        ui.data_mut(|d| {
            d.insert_temp(cell, self.head);
            d.remove::<PanelAsked>(asked_key(cell));
        });
        panel(ui);
        ui.data_mut(|d| {
            d.remove::<HeadTools>(cell);
            let asked = d.get_temp::<PanelAsked>(asked_key(cell));
            d.remove::<PanelAsked>(asked_key(cell));
            asked
        })
    }
}

fn quiet_key(cell: Id) -> Id {
    cell.with("panel-tools-quiet")
}

/// Whether the panel in `cell` shows its own tools only while the pointer is over it or one has the
/// focus, as a chart's frame does (`quiet`), or always. The library's chart panels call it.
pub fn quiet_tools(ui: &Ui, cell: Id, quiet: bool) {
    ui.data_mut(|d| d.insert_temp(quiet_key(cell), quiet));
}

fn asked_key(cell: Id) -> Id {
    cell.with("panel-asked")
}

/// Draws a panel's own tools at the end of its head, right to left: the menu, then expand.
pub(crate) fn head_tools(ui: &mut Ui, cell: Id) {
    let Some(head) = ui.data(|d| d.get_temp::<HeadTools>(cell)) else {
        return;
    };
    let mut asked = None;
    let quiet = ui.data(|d| d.get_temp::<bool>(quiet_key(cell))).unwrap_or(false);
    let hot_key = cell.with("panel-tools-hot");
    let hot = ui.data(|d| d.get_temp::<bool>(hot_key)).unwrap_or(false);
    if quiet && !hot {
        // there to press and to reach from the keyboard, unseen until the pointer is over the panel
        ui.set_opacity(0.0);
    }
    // a chart's tools sit closer, their icons 14 as `size-3.5`
    let (gap, glyph) = if quiet { (0.0, 14.0) } else { (8.0, 16.0) };
    ui.spacing_mut().item_spacing.x = gap;
    let mut now_hot = false;
    if !head.items.is_empty() {
        let trigger = ui.add(IconButton::new(Icon::Ellipsis, &head.words.menu).glyph(glyph));
        if let Some(index) = DropdownMenu::new(&head.items).end(true).width(176.0).show(&trigger) {
            asked = Some(PanelAsked::Item(index));
        }
        now_hot |= trigger.has_focus() || egui::Popup::is_id_open(ui.ctx(), trigger.id.with("menu"));
    }
    if head.expandable {
        let expand = ui.add(IconButton::new(Icon::Maximize2, &head.words.expand).glyph(14.0));
        if expand.clicked() {
            asked = Some(PanelAsked::Expand);
        }
        now_hot |= expand.has_focus();
    }
    ui.data_mut(|d| d.insert_temp(hot_key, now_hot));
    if let Some(asked) = asked {
        ui.data_mut(|d| d.insert_temp(asked_key(cell), asked));
    }
}

/// The height a panel's body took when last drawn, kept for the place it leaves while away.
#[derive(Clone, Copy)]
struct BodyHeight(f32);

/// Draws the words that stand in an expanded panel's place, as tall as its body was, and true;
/// false where the panel in `cell` is not away, so it draws its body and reports its height with
/// [`body_drawn`]. The library's panels call it themselves.
pub fn panel_away(ui: &mut Ui, cell: Id) -> bool {
    let Some(head) = ui.data(|d| d.get_temp::<HeadTools>(cell)) else {
        return false;
    };
    if !head.expanded {
        return false;
    }
    let t = tokens(ui.ctx());
    let height = ui
        .data(|d| d.get_temp::<BodyHeight>(cell))
        .map_or(96.0, |h| h.0)
        .max(96.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    super::arrange::dashed_rect(ui.painter(), rect, Stroke::new(1.0, t.color("border")));
    let galley = WidgetText::from(TextRole::Body.rich(&t, &head.words.away)).into_galley(
        ui,
        Some(TextWrapMode::Wrap),
        (rect.width() - 32.0).max(1.0),
        TextRole::Body.font(&t),
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, t.color("muted-foreground"));
    true
}

/// Keeps the height of the body a panel in `cell` drew, for the place it leaves when expanded.
pub fn body_drawn(ui: &Ui, cell: Id, height: f32) {
    ui.data_mut(|d| d.insert_temp(cell, BodyHeight(height)));
}

/// Marks the `Ui` a panel's content is drawn in inside its window.
#[derive(Clone, Copy)]
struct Windowed;

/// Whether a panel drawn in `ui` is drawn in its window: its head and card left out, its content
/// filling what the window has.
pub fn windowed(ui: &Ui) -> bool {
    ui.data(|d| d.get_temp::<Windowed>(ui.scope_id())).is_some()
}

/// The window an expanded panel's content opens in: over the page, 24 in from the window's edges
/// (12 on a phone), its head the title and a close button over a rule, the content filling the
/// rest with 16 around. The panel is drawn in it as in the grid; it leaves its head and card out.
pub struct PanelWindow {
    id: Id,
    title: String,
    words: PanelWords,
}

impl PanelWindow {
    pub fn new(id: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self {
            id: Id::unique(id),
            title: title.into(),
            words: PanelWords::default(),
        }
    }

    pub fn words(mut self, words: PanelWords) -> Self {
        self.words = words;
        self
    }

    /// Show the window, `panel` drawing the panel in it. True when it asks to close: its close
    /// button, Escape, or a press on the shade.
    pub fn show(self, ctx: &Context, panel: impl FnOnce(&mut Ui)) -> bool {
        let t = tokens(ctx);
        let screen = ctx.content_rect();
        let inset = if super::is_phone(ctx) { 12.0 } else { 24.0 };
        let area = screen.shrink(inset);
        let mut close = false;
        let response = egui::Modal::new(self.id)
            .backdrop_color(
                t.paint("dialog.base.rest.backdrop")
                    .and_then(|p| p.ground)
                    .unwrap_or_else(|| t.color_at("shade", 0.1)),
            )
            .frame(super::dialog_frame(&t))
            .area(
                egui::Modal::default_area(self.id.with("area"))
                    .fixed_pos(area.min)
                    .pivot(egui::Align2::LEFT_TOP),
            )
            .show(ctx, |ui| {
                let inner = vec2(area.width() - 2.0, area.height() - 2.0);
                ui.set_min_size(inner);
                ui.set_max_size(inner);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let pad = t.density("panel-pad");
                let (head, _) = ui.allocate_exact_size(vec2(inner.x, t.density("panel-head")), Sense::hover());
                ui.painter()
                    .hline(head.x_range(), head.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
                let mut bar = ui.new_child(
                    UiBuilder::new()
                        .max_rect(head.shrink2(vec2(pad, 0.0)))
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                if bar.add(IconButton::new(Icon::X, &self.words.close)).clicked() {
                    close = true;
                }
                let room = bar.available_width() - 8.0;
                bar.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(
                            TextRole::SectionTitle
                                .rich(&t, &self.title)
                                .color(t.color("popover-foreground")),
                        )
                        .truncate(),
                    );
                    ui.set_max_width(room.max(1.0));
                });
                let body = Rect::from_min_max(
                    head.left_bottom(),
                    head.left_bottom() + vec2(inner.x, inner.y - head.height()),
                )
                .shrink(16.0);
                let mut content = ui.new_child(
                    UiBuilder::new()
                        .id_salt("panel-window-body")
                        .max_rect(body)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                content.set_clip_rect(body.intersect(ui.clip_rect()));
                let id = content.scope_id();
                content.data_mut(|d| d.insert_temp(id, Windowed));
                panel(&mut content);
                ui.data_mut(|d| d.remove::<Windowed>(id));
            });
        close || response.should_close()
    }
}

/// Whether the panel in `cell` has tools of its own to draw in its head.
pub(crate) fn has_head_tools(ui: &Ui, cell: Id) -> bool {
    ui.data(|d| d.get_temp::<HeadTools>(cell))
        .is_some_and(|h| h.expandable || !h.items.is_empty())
}

/// Draws a panel's body, or while the panel is away in its window the words in its place: the
/// body then drawn unseen beside it, so it still hands back what it returns.
pub(crate) fn body_or_away<R>(ui: &mut Ui, cell: Id, body: impl FnOnce(&mut Ui) -> R) -> R {
    if panel_away(ui, cell) {
        let mut unseen = ui.new_child(UiBuilder::new().id_salt("away-body").invisible());
        return body(&mut unseen);
    }
    let top = ui.cursor().top();
    let out = body(ui);
    body_drawn(ui, cell, ui.min_rect().bottom() - top);
    out
}
