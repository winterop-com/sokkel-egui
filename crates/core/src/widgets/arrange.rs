//! A dashboard whose panels the reader arranges, and its edit mode, after sokkel-ui's
//! `DashboardArrange` and `DashboardEdit`. Dragging is egui's own pointer, as one engine per job.

use std::cell::RefCell;

use egui::{Id, Key, Modifiers, Order, Rect, Sense, Stroke, Ui, pos2, vec2};

use super::{Button, ButtonSize, ButtonVariant, DashboardGrid, DropdownMenu, IconButton, MenuItem, PanelSpan};
use crate::{Icon, TextRole, icons, tokens};

/// One panel's place in an arranged dashboard: its key and how wide it is. The order of the list
/// is the order drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DashboardLayoutItem {
    pub id: String,
    pub span: PanelSpan,
}

impl DashboardLayoutItem {
    pub fn new(id: impl Into<String>, span: PanelSpan) -> Self {
        Self { id: id.into(), span }
    }
}

/// A kind of panel the palette offers, dragged onto the grid (or pressed) to add one.
#[derive(Clone, Debug)]
pub struct DashboardWidget {
    pub id: String,
    pub label: String,
    pub icon: Option<Icon>,
    /// How wide a new panel of this kind starts.
    pub span: PanelSpan,
}

impl DashboardWidget {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            span: PanelSpan::Third,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn span(mut self, span: PanelSpan) -> Self {
        self.span = span;
        self
    }
}

/// Every width a panel's size menu offers, narrowest first.
pub const ALL_SPANS: [PanelSpan; 6] = [
    PanelSpan::Sixth,
    PanelSpan::Quarter,
    PanelSpan::Third,
    PanelSpan::Half,
    PanelSpan::TwoThirds,
    PanelSpan::Full,
];

/// The words an arranged dashboard says.
#[derive(Clone, Debug)]
pub struct DashboardArrangeWords {
    /// A panel's grip.
    pub move_panel: fn(&str) -> String,
    /// A panel's size menu.
    pub size: fn(&str) -> String,
    pub remove: fn(&str) -> String,
    /// A panel's configure button.
    pub configure: fn(&str) -> String,
    /// Each width the size menu offers.
    pub span: fn(PanelSpan) -> String,
    /// A widget in the palette: what pressing it does.
    pub add: fn(&str) -> String,
    /// The place after the last panel, while arranging.
    pub end: String,
}

impl Default for DashboardArrangeWords {
    fn default() -> Self {
        Self {
            move_panel: |panel| format!("Move {panel}"),
            size: |panel| format!("Size of {panel}"),
            remove: |panel| format!("Remove {panel}"),
            configure: |panel| format!("Configure {panel}"),
            span: |span| {
                match span {
                    PanelSpan::Sixth => "Sixth",
                    PanelSpan::Quarter => "Quarter",
                    PanelSpan::Third => "Third",
                    PanelSpan::Half => "Half",
                    PanelSpan::TwoThirds => "Two thirds",
                    PanelSpan::Full => "Full width",
                }
                .into()
            },
            add: |widget| format!("Add {widget}"),
            end: "Drop here to add at the end".into(),
        }
    }
}

/// Where a thing lands among `ids` once dropped before or after the panel `over`, or at the end
/// for `None`, as sokkel-ui's `arrangedIndex`: its place with the moved panel taken out, or in
/// the whole list for a new one. `None` where a panel lands where it was.
pub fn arranged_index(ids: &[&str], moved: Option<&str>, over: Option<&str>, after: bool) -> Option<usize> {
    let others: Vec<&str> = ids.iter().copied().filter(|id| Some(*id) != moved).collect();
    let at = match over {
        None => others.len(),
        Some(over) => others.iter().position(|id| *id == over)? + usize::from(after),
    };
    if let Some(moved) = moved
        && ids.iter().position(|id| *id == moved) == Some(at)
    {
        return None;
    }
    Some(at)
}

/// The list with the item at `from` moved to `to`, as sokkel-ui's `moveItem`; unchanged when
/// either is out of the list.
pub fn move_item<T>(list: &mut Vec<T>, from: usize, to: usize) {
    if from == to || from >= list.len() || to >= list.len() {
        return;
    }
    let item = list.remove(from);
    list.insert(to, item);
}

/// Marks a cell whose panel is arranged, so its head makes room for the tools.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ArrangeMark;

/// What an arranged panel's head draws as its tools, put where its cell's panel reads it.
#[derive(Clone, Debug)]
struct ArrangeTools {
    move_label: String,
    size_label: String,
    remove_label: String,
    configure_label: Option<String>,
    span: PanelSpan,
    spans: Vec<(PanelSpan, String)>,
    removable: bool,
}

/// What a press on a panel's tools asked for, handed back to the arrangement.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ToolPress {
    Span(PanelSpan),
    Remove,
    Configure,
    Step(i32),
    Grab,
}

/// Draws an arranged panel's tools (a grip, configure, a size menu, remove) at the end of a
/// panel's head, as sokkel-ui's `PanelTools`; nothing outside an arrangement. `cell` is the id of
/// the `Ui` the panel was given. The library's panels call it themselves; a head of an app's own
/// calls it in its end.
pub fn panel_tools(ui: &mut Ui, cell: Id) {
    let Some(tools) = ui.data(|d| d.get_temp::<ArrangeTools>(cell)) else {
        return;
    };
    let mut press = None;
    ui.spacing_mut().item_spacing.x = 2.0;
    // right to left: remove, size, configure, then the grip first in reading order
    if tools.removable
        && ui
            .add(IconButton::new(Icon::X, &tools.remove_label).xs().destructive())
            .clicked()
    {
        press = Some(ToolPress::Remove);
    }
    if !tools.spans.is_empty() {
        let trigger = ui.add(IconButton::new(Icon::StretchHorizontal, &tools.size_label).xs());
        let items: Vec<MenuItem> = tools
            .spans
            .iter()
            .map(|(span, word)| {
                let item = MenuItem::action(word.clone());
                if *span == tools.span {
                    item.icon(Icon::Check)
                } else {
                    item.icon(Icon::Dot)
                }
            })
            .collect();
        if let Some(index) = DropdownMenu::new(&items).end(true).width(160.0).show(&trigger) {
            press = Some(ToolPress::Span(tools.spans[index].0));
        }
    }
    if let Some(label) = &tools.configure_label
        && ui.add(IconButton::new(Icon::Settings2, label).xs()).clicked()
    {
        press = Some(ToolPress::Configure);
    }
    let t = tokens(ui.ctx());
    let side = t.density("control-xs");
    let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
    let grip = ui.interact(rect, grip_id(cell), Sense::click_and_drag());
    grip.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &tools.move_label));
    let hot = grip.hovered() || grip.dragged();
    if hot {
        ui.painter().rect_filled(rect, t.radius("control"), t.color("hover"));
        ui.ctx().set_cursor_icon(if grip.dragged() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        });
    }
    let ink = if hot {
        t.color("foreground")
    } else {
        t.color("muted-foreground")
    };
    icons::paint(ui.painter(), rect.center(), Icon::GripVertical, 12.0, ink);
    super::focus_ring(ui.painter(), &t, rect, t.radius("control"), &grip);
    if grip.drag_started() {
        press = Some(ToolPress::Grab);
    }
    if grip.has_focus() {
        ui.input_mut(|i| {
            let none = Modifiers::NONE;
            if i.consume_key(none, Key::ArrowLeft) || i.consume_key(none, Key::ArrowUp) {
                press = Some(ToolPress::Step(-1));
            } else if i.consume_key(none, Key::ArrowRight) || i.consume_key(none, Key::ArrowDown) {
                press = Some(ToolPress::Step(1));
            }
        });
    }
    if let Some(press) = press {
        ui.data_mut(|d| d.insert_temp(cell, press));
    }
}

/// The grip that last moved its panel from the keyboard, so the focus follows it to its new place.
fn grip_focus_key(arrange: Id) -> Id {
    arrange.with("refocus")
}

/// What is being carried across the grid.
#[derive(Clone, Debug, PartialEq)]
enum Carried {
    Panel(String),
    Widget(usize),
}

/// What an arrangement did this frame.
#[derive(Clone, Debug, Default)]
pub struct DashboardArrangeResponse {
    /// The layout was moved, resized or had a panel removed.
    pub changed: bool,
    /// A widget of the palette, by its id, to add at this index in the layout: dropped there, or
    /// pressed (at the end). The app adds the panel.
    pub added: Option<(String, usize)>,
    /// A panel whose configure button was pressed, by its id.
    pub configure: Option<String>,
}

/// A dashboard whose panels the reader arranges, as sokkel-ui's `DashboardArrange`. While
/// `arranging`, each panel's head holds a grip that drags it, with the grip focused the arrow keys
/// move it a place; a menu of widths and a remove button, and with `configurable` a button that
/// opens its options. A line at a panel's side shows where a dragged one lands. Widgets in the
/// palette are dragged onto the grid to add a panel there, or pressed to add one at the end.
pub struct DashboardArrange<'a> {
    id: Id,
    layout: &'a mut Vec<DashboardLayoutItem>,
    label: &'a dyn Fn(&DashboardLayoutItem) -> String,
    arranging: bool,
    palette: &'a [DashboardWidget],
    spans: &'a [PanelSpan],
    removable: bool,
    configurable: Option<&'a dyn Fn(&DashboardLayoutItem) -> bool>,
    dense: bool,
    words: DashboardArrangeWords,
}

impl<'a> DashboardArrange<'a> {
    pub fn new(
        id: impl std::hash::Hash + std::fmt::Debug,
        layout: &'a mut Vec<DashboardLayoutItem>,
        label: &'a dyn Fn(&DashboardLayoutItem) -> String,
    ) -> Self {
        Self {
            id: Id::new(("sokkel-arrange", id)),
            layout,
            label,
            arranging: true,
            palette: &[],
            spans: &ALL_SPANS,
            removable: true,
            configurable: None,
            dense: false,
            words: DashboardArrangeWords::default(),
        }
    }

    /// Whether the panels are being arranged; off, it is a plain dashboard.
    pub fn arranging(mut self, arranging: bool) -> Self {
        self.arranging = arranging;
        self
    }

    /// The kinds of panel the palette offers while arranging.
    pub fn palette(mut self, palette: &'a [DashboardWidget]) -> Self {
        self.palette = palette;
        self
    }

    /// The widths a panel's size menu offers; empty for none.
    pub fn spans(mut self, spans: &'a [PanelSpan]) -> Self {
        self.spans = spans;
        self
    }

    pub fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }

    /// Which panels have a configure button while arranging.
    pub fn configurable(mut self, configurable: &'a dyn Fn(&DashboardLayoutItem) -> bool) -> Self {
        self.configurable = Some(configurable);
        self
    }

    pub fn dense(mut self, dense: bool) -> Self {
        self.dense = dense;
        self
    }

    pub fn words(mut self, words: DashboardArrangeWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the panels, `render` drawing each in its cell: a `DashboardPanel` or a chart panel.
    pub fn show(self, ui: &mut Ui, render: impl FnMut(&mut Ui, &DashboardLayoutItem)) -> DashboardArrangeResponse {
        let t = tokens(ui.ctx());
        let mut response = DashboardArrangeResponse::default();
        let carried_key = self.id.with("carried");
        let mut carried: Option<Carried> = if self.arranging {
            ui.data(|d| d.get_temp::<Option<Carried>>(carried_key)).flatten()
        } else {
            None
        };
        let show_end = self.arranging && !self.palette.is_empty();
        let pointer = ui.ctx().pointer_latest_pos();
        // the palette
        if show_end {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                for (index, widget) in self.palette.iter().enumerate() {
                    let chip = palette_chip(ui, widget, &(self.words.add)(&widget.label));
                    if chip.clicked() {
                        response.added = Some((widget.id.clone(), self.layout.len()));
                    }
                    if chip.drag_started() {
                        carried = Some(Carried::Widget(index));
                    }
                }
            });
            ui.add_space(12.0);
        }
        // the grid, each cell's rect and tool press noted as it is drawn
        let cells: RefCell<Vec<(String, Rect, Option<ToolPress>)>> = RefCell::new(Vec::new());
        let render = RefCell::new(render);
        let layout_now: Vec<DashboardLayoutItem> = self.layout.clone();
        let refocus = ui.data_mut(|d| d.remove_temp::<String>(grip_focus_key(self.id)));
        DashboardGrid::new(self.id).dense(self.dense).show(ui, |grid| {
            for item in &layout_now {
                let cells = &cells;
                let render = &render;
                let words = &self.words;
                let label = (self.label)(item);
                let configurable = self.configurable.is_some_and(|f| f(item));
                let dragging = carried == Some(Carried::Panel(item.id.clone()));
                let arranging = self.arranging;
                let spans = self.spans;
                let removable = self.removable;
                let refocus = refocus.as_deref() == Some(item.id.as_str());
                grid.cell(item.span, move |ui| {
                    let cell = ui.id();
                    if arranging {
                        ui.data_mut(|d| {
                            d.insert_temp(
                                cell,
                                ArrangeTools {
                                    move_label: (words.move_panel)(&label),
                                    size_label: (words.size)(&label),
                                    remove_label: (words.remove)(&label),
                                    configure_label: configurable.then(|| (words.configure)(&label)),
                                    span: item.span,
                                    spans: spans.iter().map(|s| (*s, (words.span)(*s))).collect(),
                                    removable,
                                },
                            );
                            d.insert_temp(cell, ArrangeMark);
                            d.remove::<ToolPress>(cell);
                        });
                    } else {
                        ui.data_mut(|d| {
                            d.remove::<ArrangeTools>(cell);
                            d.remove::<ArrangeMark>(cell);
                        });
                    }
                    if dragging {
                        ui.set_opacity(0.4);
                    }
                    (render.borrow_mut())(ui, item);
                    let press = ui.data(|d| d.get_temp::<ToolPress>(cell));
                    ui.data_mut(|d| d.remove::<ToolPress>(cell));
                    if refocus {
                        // the grip moved from the keyboard keeps the focus in its new place
                        ui.memory_mut(|m| m.request_focus(grip_id(cell)));
                    }
                    cells.borrow_mut().push((item.id.clone(), ui.min_rect(), press));
                });
            }
        });
        let cells = cells.into_inner();
        // the place after the last panel
        let end = show_end.then(|| {
            ui.add_space(t.density("section-gap"));
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::hover());
            rect
        });
        // the presses on the panels' tools
        for (id, _, press) in &cells {
            let Some(press) = press else { continue };
            let Some(from) = self.layout.iter().position(|i| &i.id == id) else {
                continue;
            };
            match *press {
                ToolPress::Span(span) => {
                    self.layout[from].span = span;
                    response.changed = true;
                }
                ToolPress::Remove => {
                    self.layout.remove(from);
                    response.changed = true;
                }
                ToolPress::Configure => response.configure = Some(id.clone()),
                ToolPress::Grab => carried = Some(Carried::Panel(id.clone())),
                ToolPress::Step(by) => {
                    let to = from as i32 + by;
                    if to >= 0 && (to as usize) < self.layout.len() {
                        move_item(self.layout, from, to as usize);
                        response.changed = true;
                        ui.data_mut(|d| d.insert_temp(grip_focus_key(self.id), id.clone()));
                    }
                }
            }
            break;
        }
        // where a carried thing would land
        let ids: Vec<&str> = layout_now.iter().map(|i| i.id.as_str()).collect();
        let moved = match &carried {
            Some(Carried::Panel(id)) => Some(id.as_str()),
            _ => None,
        };
        let aim = carried.as_ref().and(pointer).and_then(|at| {
            if let Some(end) = end
                && end.contains(at)
            {
                return arranged_index(&ids, moved, None, true).map(|index| (None, true, index));
            }
            let (id, rect, _) = cells.iter().find(|(_, rect, _)| rect.expand(6.0).contains(at))?;
            if Some(id.as_str()) == moved {
                return None;
            }
            let after = at.x > rect.center().x;
            arranged_index(&ids, moved, Some(id), after).map(|index| (Some(id.clone()), after, index))
        });
        let painter = ui.painter();
        if self.arranging {
            // each panel outlined, dashed, 2 out
            for (_, rect, _) in &cells {
                dashed_rect(painter, rect.expand(2.0), Stroke::new(1.0, t.color("border-strong")));
            }
        }
        if let Some(end) = end {
            let on = matches!(aim, Some((None, _, _)));
            if on {
                painter.rect_filled(end, t.radius("surface"), t.color_at("primary", 0.05));
            }
            let edge = if on {
                t.color("primary")
            } else {
                t.color("border-strong")
            };
            dashed_rect(painter, end, Stroke::new(1.0, edge));
            let ink = if on {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            let words = painter.layout_no_wrap(self.words.end.clone(), TextRole::Body.font(&t), ink);
            let total = 16.0 + 6.0 + words.size().x;
            let x = end.center().x - total / 2.0;
            icons::paint(painter, pos2(x + 8.0, end.center().y), Icon::Plus, 16.0, ink);
            painter.galley(pos2(x + 22.0, end.center().y - words.size().y / 2.0), words, ink);
        }
        if let Some((Some(id), after, _)) = &aim
            && let Some((_, rect, _)) = cells.iter().find(|(each, _, _)| each == id)
        {
            let x = if *after { rect.right() + 6.0 } else { rect.left() - 6.0 };
            painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(2.0, t.color("primary")),
            );
        }
        // what follows the pointer: a card with the carried thing's name
        if let (Some(thing), Some(at)) = (&carried, pointer) {
            let name = match thing {
                Carried::Panel(id) => layout_now.iter().find(|i| &i.id == id).map(|i| (self.label)(i)),
                Carried::Widget(index) => self.palette.get(*index).map(|w| w.label.clone()),
            }
            .unwrap_or_default();
            let layer = ui
                .ctx()
                .layer_painter(egui::LayerId::new(Order::Tooltip, self.id.with("preview")));
            let galley = layer.layout_no_wrap(name, TextRole::Label.font(&t), t.color("foreground"));
            let card = Rect::from_min_size(at + vec2(8.0, 8.0), vec2(galley.size().x + 24.0, 40.0));
            layer.rect_filled(card, t.radius("surface"), t.color("card"));
            layer.rect_stroke(
                card,
                t.radius("surface"),
                Stroke::new(1.0, t.color("border-strong")),
                egui::StrokeKind::Inside,
            );
            layer.galley(
                pos2(card.left() + 12.0, card.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        }
        // let go: the carried thing lands where it is aimed
        let released = ui.input(|i| !i.pointer.primary_down());
        // (taking it first, so a drop aimed nowhere still ends the carry)
        if released
            && let Some(thing) = carried.take()
            && let Some((_, _, index)) = aim
        {
            match thing {
                Carried::Panel(id) => {
                    if let Some(from) = self.layout.iter().position(|i| i.id == id) {
                        move_item(self.layout, from, index.min(self.layout.len() - 1));
                        response.changed = true;
                    }
                }
                Carried::Widget(w) => {
                    if let Some(widget) = self.palette.get(w) {
                        response.added = Some((widget.id.clone(), index));
                    }
                }
            }
        }
        if carried.is_some() {
            ui.ctx().request_repaint();
        }
        ui.data_mut(|d| d.insert_temp(carried_key, carried));
        response
    }
}

/// The id of the grip `panel_tools` draws for the panel in `cell`.
fn grip_id(cell: Id) -> Id {
    cell.with("sokkel-grip")
}

/// A widget in the palette, as sokkel-ui's `PaletteChip`: 32 tall, the strong border, the card's
/// ground, its icon muted before its label.
fn palette_chip(ui: &mut Ui, widget: &DashboardWidget, add: &str) -> egui::Response {
    let t = tokens(ui.ctx());
    let galley = ui
        .painter()
        .layout_no_wrap(widget.label.clone(), TextRole::Body.font(&t), t.color("foreground"));
    let width = 10.0 + widget.icon.map_or(0.0, |_| 16.0 + 6.0) + galley.size().x + 10.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click_and_drag());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, add));
    let painter = ui.painter();
    let radius = t.radius("control");
    let ground = if response.hovered() {
        t.color("hover")
    } else {
        t.color("card")
    };
    painter.rect_filled(rect, radius, ground);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, t.color("border-strong")),
        egui::StrokeKind::Inside,
    );
    let mut x = rect.left() + 10.0;
    if let Some(icon) = widget.icon {
        icons::paint(
            painter,
            pos2(x + 8.0, rect.center().y),
            icon,
            16.0,
            t.color("muted-foreground"),
        );
        x += 22.0;
    }
    painter.galley(
        pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        t.color("foreground"),
    );
    super::focus_ring(painter, &t, rect, radius, &response);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    response.on_hover_text(add)
}

/// A rectangle's edge drawn dashed, as `outline-dashed` and `border-dashed` draw it.
fn dashed_rect(painter: &egui::Painter, rect: Rect, stroke: Stroke) {
    let r = rect.shrink(0.5);
    let corners = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
        r.left_top(),
    ];
    for pair in corners.windows(2) {
        painter.extend(egui::Shape::dashed_line(pair, stroke, 4.0, 3.0));
    }
}

/// The words a dashboard's edit buttons say.
#[derive(Clone, Debug)]
pub struct DashboardEditWords {
    pub edit: String,
    pub done: String,
    pub cancel: String,
    pub reset: String,
    /// Why Reset is shut while the layout is already the default.
    pub is_default: String,
}

impl Default for DashboardEditWords {
    fn default() -> Self {
        Self {
            edit: "Edit".into(),
            done: "Done".into(),
            cancel: "Cancel".into(),
            reset: "Reset".into(),
            is_default: "The dashboard is as it comes.".into(),
        }
    }
}

/// A dashboard's edit mode, as sokkel-ui's `useDashboardEdit`: while editing, every move, width,
/// removal and addition goes into a draft; Done keeps it, Cancel throws it away, Reset puts the
/// default in the draft. The app holds it beside the kept layout.
#[derive(Clone, Debug, Default)]
pub struct DashboardEdit {
    draft: Option<Vec<DashboardLayoutItem>>,
}

impl DashboardEdit {
    pub fn editing(&self) -> bool {
        self.draft.is_some()
    }

    /// The layout to draw and arrange: the draft while editing, else the kept one.
    pub fn layout<'a>(&'a mut self, kept: &'a mut Vec<DashboardLayoutItem>) -> &'a mut Vec<DashboardLayoutItem> {
        match &mut self.draft {
            Some(draft) => draft,
            None => kept,
        }
    }

    pub fn edit(&mut self, kept: &[DashboardLayoutItem]) {
        self.draft = Some(kept.to_vec());
    }

    /// Keeps the draft, when it differs, in `kept` and leaves edit mode; true when it was kept.
    pub fn done(&mut self, kept: &mut Vec<DashboardLayoutItem>) -> bool {
        match self.draft.take() {
            Some(draft) if &draft != kept => {
                *kept = draft;
                true
            }
            _ => false,
        }
    }

    pub fn cancel(&mut self) {
        self.draft = None;
    }

    pub fn reset(&mut self, default: &[DashboardLayoutItem]) {
        self.draft = Some(default.to_vec());
    }

    /// Whether the draft differs from the kept layout.
    pub fn changed(&self, kept: &[DashboardLayoutItem]) -> bool {
        self.draft.as_deref().is_some_and(|d| d != kept)
    }
}

/// What the edit buttons did this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DashboardEditOutcome {
    /// Done kept a changed layout.
    pub saved: bool,
}

/// A dashboard's edit buttons for its page header, as sokkel-ui's `DashboardEditActions`: Edit,
/// and while editing Reset (shut while the layout is the default), Cancel and Done. Drawn in a
/// right-to-left row such as `PageHeader`'s actions, they add themselves last first.
pub struct DashboardEditActions<'a> {
    edit: &'a mut DashboardEdit,
    kept: &'a mut Vec<DashboardLayoutItem>,
    default: Option<&'a [DashboardLayoutItem]>,
    words: DashboardEditWords,
}

impl<'a> DashboardEditActions<'a> {
    pub fn new(edit: &'a mut DashboardEdit, kept: &'a mut Vec<DashboardLayoutItem>) -> Self {
        Self {
            edit,
            kept,
            default: None,
            words: DashboardEditWords::default(),
        }
    }

    /// The layout Reset puts back; no Reset without it.
    pub fn default_layout(mut self, default: &'a [DashboardLayoutItem]) -> Self {
        self.default = Some(default);
        self
    }

    pub fn words(mut self, words: DashboardEditWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> DashboardEditOutcome {
        let mut outcome = DashboardEditOutcome::default();
        if !self.edit.editing() {
            if ui
                .add(
                    Button::new(&self.words.edit)
                        .icon(Icon::Pencil)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm),
                )
                .clicked()
            {
                self.edit.edit(self.kept);
            }
            return outcome;
        }
        if ui
            .add(Button::new(&self.words.done).icon(Icon::Check).size(ButtonSize::Sm))
            .clicked()
        {
            outcome.saved = self.edit.done(self.kept);
            return outcome;
        }
        if ui
            .add(
                Button::new(&self.words.cancel)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm),
            )
            .clicked()
        {
            self.edit.cancel();
            return outcome;
        }
        if let Some(default) = self.default {
            let shown = self.edit.draft.as_deref().unwrap_or(self.kept);
            let is_default = shown == default;
            let mut reset = Button::new(&self.words.reset)
                .icon(Icon::RotateCcw)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Sm);
            if is_default {
                reset = reset.reason(&self.words.is_default);
            }
            if ui.add(reset).clicked() && !is_default {
                self.edit.reset(default);
            }
        }
        outcome
    }
}
