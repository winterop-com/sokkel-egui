//! A graph's canvas, after sokkel-ui-graph's `GraphCanvas`: in its `fitted` frame as tall as its
//! drawing fitted to its width, the zoom buttons in a row at its foot; on a `stage` filling its box,
//! the wheel zooming it, and with `editing` an editor whose gestures it reports.

use std::collections::HashMap;

use egui::{Color32, Id, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, vec2};
use sokkel_egui::{Icon, MenuItem, Tokens, tokens};

use crate::layout::{GraphDirection, GraphSpacing, PlacedGraph, layout};
use crate::node::GraphNode;
use crate::toolbar::{GraphToolbar, GraphToolbarResponse};
use crate::view_buttons::{ButtonsOrientation, GraphControlButton, GraphViewButtons, GraphViewWords};

/// Room left around a fitted graph, as a share of the canvas.
pub const FIT_PADDING: f32 = 0.12;
/// A fit never magnifies a graph past its own size, and never shrinks one below a size its words
/// can be read at.
pub const FIT_MAX_ZOOM: f32 = 1.0;
pub const FIT_MIN_ZOOM: f32 = 0.75;
/// Below this width a fit may go as low as `NARROW_FIT_MIN_ZOOM` to fit the drawing's width.
pub const NARROW_GRAPH: f32 = 560.0;
pub const NARROW_FIT_MIN_ZOOM: f32 = 0.6;
/// How far a reader may zoom by hand, and one press of a zoom button.
pub const MIN_ZOOM: f32 = 0.25;
pub const MAX_ZOOM: f32 = 2.0;
pub const ZOOM_STEP: f32 = 1.4;
/// The ground a fitted frame keeps around its drawing, and the shortest it is drawn.
pub const FITTED_GROUND: f32 = 40.0;
pub const FITTED_MIN_HEIGHT: f32 = 120.0;
/// The row the zoom buttons take under a fitted drawing.
const BUTTON_ROW: f32 = 36.0;
/// An arrowhead's length, and the radius an edge's corners are rounded to.
const ARROW: f32 = 7.0;
pub(crate) const CORNER: f32 = 6.0;
/// How far from the canvas's edge a graph too big to fit at the floor starts.
pub const OVERFLOW_MARGIN: f32 = 24.0;

/// Where a canvas stands, as sokkel-ui-graph's `GraphFrame`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphFrame {
    /// As tall as its drawing at the width it has, the zoom buttons in a row at its foot.
    #[default]
    Fitted,
    /// Fills its box on the canvas's dotted ground; the wheel zooms it, a drag on the ground pans
    /// it, and the zoom buttons stand one over another in its corner.
    Stage,
}

/// How an edge is drawn: `Quiet` for a way not taken yet, `Strong` darker for the way in use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeTone {
    #[default]
    Plain,
    Quiet,
    Strong,
}

/// What travels an edge now: `Flowing` dashes moving toward the target, `Lit` the same stroke
/// standing still (for reduced motion), `Pulse` one brighter handover that settles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeMotion {
    Flowing,
    Lit,
    Pulse,
}

/// An edge from one node to another by their ids, with an arrowhead at its target when `arrow`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub arrow: bool,
    /// The hue its source settled in, as a run's edge out of a step that ended.
    pub tint: Option<Color32>,
    /// Dashed, as an edge into a step that was skipped.
    pub dashed: bool,
    /// Drawn quiet or strong, on a stage.
    pub tone: EdgeTone,
    /// What travels the edge now, on a stage.
    pub motion: Option<EdgeMotion>,
}

impl GraphLink {
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            arrow: false,
            tint: None,
            dashed: false,
            tone: EdgeTone::Plain,
            motion: None,
        }
    }

    pub fn tone(mut self, tone: EdgeTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn motion(mut self, motion: Option<EdgeMotion>) -> Self {
        self.motion = motion;
        self
    }

    pub fn tint(mut self, tint: Option<Color32>) -> Self {
        self.tint = tint;
        self
    }

    pub fn dashed(mut self, dashed: bool) -> Self {
        self.dashed = dashed;
        self
    }

    pub fn arrow(mut self, arrow: bool) -> Self {
        self.arrow = arrow;
        self
    }
}

/// The words a canvas says.
#[derive(Clone, Debug)]
pub struct GraphWords {
    /// What the canvas is, for a screen reader.
    pub label: String,
    pub zoom_in: String,
    pub zoom_out: String,
    pub fit: String,
    /// What the zoom buttons' tooltips say in place of their names.
    pub zoom_in_hint: Option<String>,
    pub zoom_out_hint: Option<String>,
    pub fit_hint: Option<String>,
    /// What an empty canvas says.
    pub empty: String,
    /// What a node tells a keyboard reader.
    pub select: String,
    /// The add button at a node's output, on an editor.
    pub append: String,
    /// The add button at an edge's middle, on an editor.
    pub insert: String,
}

impl Default for GraphWords {
    fn default() -> Self {
        Self {
            label: "Graph".into(),
            zoom_in: "Zoom in".into(),
            zoom_out: "Zoom out".into(),
            fit: "Fit the graph".into(),
            zoom_in_hint: None,
            zoom_out_hint: None,
            fit_hint: None,
            empty: "Nothing to show".into(),
            select: "Press enter or space to select it.".into(),
            append: "Add a step after this one".into(),
            insert: "Add a step here".into(),
        }
    }
}

impl GraphWords {
    /// The words the view's buttons say.
    pub fn view(&self) -> GraphViewWords {
        GraphViewWords {
            zoom_in: self.zoom_in.clone(),
            zoom_out: self.zoom_out.clone(),
            fit: self.fit.clone(),
            zoom_in_hint: self.zoom_in_hint.clone(),
            zoom_out_hint: self.zoom_out_hint.clone(),
            fit_hint: self.fit_hint.clone(),
        }
    }
}

/// A point a gesture ended at: on the screen, and in the graph's own coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphPointer {
    pub screen: Pos2,
    pub at: Pos2,
}

/// An edge by its id (`source->target`, `#n` after a repeat) and its two ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphEdgeRef {
    pub id: String,
    pub source: String,
    pub target: String,
}

/// A menu a canvas opens at a right click: a line naming what it acts on, and its rows. A row is
/// reported by its id (`MenuItem::id`), or its label without one.
#[derive(Clone, Debug, Default)]
pub struct GraphMenu {
    pub head: Option<String>,
    pub items: Vec<MenuItem>,
}

impl GraphMenu {
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self { head: None, items }
    }

    pub fn head(mut self, head: impl Into<String>) -> Self {
        self.head = Some(head.into());
        self
    }
}

/// What a gesture on an editable canvas reported. The canvas changes nothing itself: the app owns
/// the graph, applies what it accepts, and hands the canvas the result.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphEvent {
    /// An edge was drawn from one node's port to another's, and `can_connect` allowed it.
    Connect { source: String, target: String },
    /// A connection was let go over a node it may not join, for the reason `can_connect` gave.
    Refuse {
        source: String,
        target: String,
        reason: String,
    },
    /// A chosen edge was deleted.
    Disconnect(GraphEdgeRef),
    /// A chosen node was deleted; its edges go with it and are not reported apart.
    Remove(String),
    /// A box was dragged and let go with its top-left corner at `at`, in the graph's coordinates.
    Move { node: String, at: Pos2 },
    /// A connection drawn from a node's output was let go over empty ground.
    Drop { source: String, at: GraphPointer },
    /// The add button at a node's output was pressed.
    Append { node: String, at: GraphPointer },
    /// The add button at an edge's middle was pressed.
    Insert { edge: GraphEdgeRef, at: GraphPointer },
    /// A box was right-clicked, or its menu opened from the keyboard.
    NodeMenu { node: String, at: GraphPointer },
    /// The ground was right-clicked.
    PaneMenu { at: GraphPointer },
    /// A row of a node's menu (`node` given) or of the ground's was chosen, by its id.
    MenuPick { node: Option<String>, item: String },
    /// A block dragged in from a palette was let go over the canvas.
    DropItem { item: String, at: GraphPointer },
}

/// Why an edge from a source to a target is refused, or `None` to allow it.
pub type ConnectCheck<'a> = &'a dyn Fn(&str, &str) -> Option<String>;
/// The menu a node opens, asked with its id.
pub type NodeMenuFn<'a> = &'a dyn Fn(&str) -> Option<GraphMenu>;
/// The menu the ground opens, asked with the point in the graph's coordinates.
pub type PaneMenuFn<'a> = &'a dyn Fn(Pos2) -> Option<GraphMenu>;

/// Which gestures make a canvas an editor: ports to connect, Delete to remove, and each other
/// gesture when it is turned on, as the web's `GraphEditing` has a callback for it. Without
/// editing a canvas only selects.
#[derive(Clone, Copy, Default)]
pub struct GraphEditing<'a> {
    /// Why an edge from a source to a target is refused, or `None` to allow it: asked while a
    /// connection is drawn, and the reason said beside the pointer.
    pub can_connect: Option<ConnectCheck<'a>>,
    /// Whether a node carries the add button at its output; every node does without this.
    pub can_append: Option<&'a dyn Fn(&str) -> bool>,
    /// The add button at a hovered or chosen node's output (`GraphEvent::Append`).
    pub append: bool,
    /// The add button at a hovered or chosen edge's middle (`GraphEvent::Insert`).
    pub insert: bool,
    /// Boxes drag (`GraphEvent::Move`).
    pub moves: bool,
    /// A connection let go on the ground is reported (`GraphEvent::Drop`).
    pub drop: bool,
    /// A right click on a box is reported (`GraphEvent::NodeMenu`).
    pub node_menu: bool,
    /// A right click on the ground is reported (`GraphEvent::PaneMenu`).
    pub pane_menu: bool,
    /// A palette's block dropped on the canvas is reported (`GraphEvent::DropItem`).
    pub drop_items: bool,
}

impl<'a> GraphEditing<'a> {
    /// An editor that connects, disconnects and removes, with every other gesture off.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every gesture on.
    pub fn all() -> Self {
        Self {
            append: true,
            insert: true,
            moves: true,
            drop: true,
            node_menu: true,
            pane_menu: true,
            drop_items: true,
            ..Self::default()
        }
    }

    pub fn can_connect(mut self, can: ConnectCheck<'a>) -> Self {
        self.can_connect = Some(can);
        self
    }

    pub fn can_append(mut self, can: &'a dyn Fn(&str) -> bool) -> Self {
        self.can_append = Some(can);
        self
    }

    /// Why a connection is refused, or `None`; a node never joins itself.
    pub(crate) fn refusal(&self, source: &str, target: &str) -> Option<String> {
        self.can_connect.and_then(|can| can(source, target))
    }
}

/// Where the drawing stands on a canvas: moved by `x` and `y` from the canvas's corner, and zoomed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphView {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl GraphView {
    /// The view zoomed to `zoom` about `about`, a point on the canvas from its corner.
    pub fn zoomed(self, zoom: f32, about: Vec2) -> Self {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let scale = zoom / self.zoom;
        Self {
            x: about.x - (about.x - self.x) * scale,
            y: about.y - (about.y - self.y) * scale,
            zoom,
        }
    }
}

/// What moves the view of a canvas and where things stand on it, as the web's `GraphControls`: for
/// a toolbar of the app's, for placing what the app adds, and for an overlay or a notice drawn over
/// the canvas after it.
#[derive(Clone, Debug)]
pub struct GraphControls {
    pub(crate) id: Id,
    /// The canvas on the screen.
    pub rect: Rect,
    pub view: GraphView,
    /// Which way the graph runs, so a box the app adds after another goes on the side its edges
    /// leave from.
    pub direction: GraphDirection,
    /// The canvas has nothing on it to zoom or fit.
    pub empty: bool,
    pub(crate) boxes: Vec<(String, Rect)>,
    pub(crate) edges: Vec<(GraphEdgeRef, Vec<Pos2>)>,
}

impl GraphControls {
    /// Where a node's box stands as drawn, in the graph's coordinates.
    pub fn node_box(&self, id: &str) -> Option<Rect> {
        self.boxes.iter().find(|(n, _)| n == id).map(|(_, rect)| *rect)
    }

    /// A point on the screen as a point in the graph's coordinates.
    pub fn to_graph(&self, screen: Pos2) -> Pos2 {
        let at = screen - self.rect.min - vec2(self.view.x, self.view.y);
        (at / self.view.zoom).to_pos2()
    }

    /// A point in the graph's coordinates as a point on the screen.
    pub fn to_screen(&self, at: Pos2) -> Pos2 {
        self.rect.min + vec2(self.view.x, self.view.y) + at.to_vec2() * self.view.zoom
    }

    /// The point a node's edges leave from (`out`) or arrive at, in the graph's coordinates.
    pub fn port(&self, id: &str, out: bool) -> Option<Pos2> {
        self.node_box(id).map(|rect| port_of(rect, self.direction, out))
    }

    /// The middle of an edge's route by length, in the graph's coordinates.
    pub fn edge_middle(&self, edge: &str) -> Option<Pos2> {
        self.edges
            .iter()
            .find(|(e, _)| e.id == edge)
            .and_then(|(_, route)| midpoint(route))
    }

    /// Every edge drawn, with its route in the graph's coordinates.
    pub fn edges(&self) -> impl Iterator<Item = (&GraphEdgeRef, &[Pos2])> {
        self.edges.iter().map(|(e, route)| (e, route.as_slice()))
    }

    /// Zoom the canvas in a step about its middle, as its button does.
    pub fn zoom_in(&self, ctx: &egui::Context) {
        self.step(ctx, ZOOM_STEP);
    }

    /// Zoom the canvas out a step about its middle.
    pub fn zoom_out(&self, ctx: &egui::Context) {
        self.step(ctx, 1.0 / ZOOM_STEP);
    }

    /// Fit the whole graph again, and keep it fitted as it changes.
    pub fn fit(&self, ctx: &egui::Context) {
        ctx.data_mut(|d| d.insert_temp(self.id.with("asked"), ViewAsk::Fit));
        ctx.request_repaint();
    }

    fn step(&self, ctx: &egui::Context, factor: f32) {
        ctx.data_mut(|d| d.insert_temp(self.id.with("asked"), ViewAsk::Zoom(factor)));
        ctx.request_repaint();
    }
}

/// A change of view asked from outside the canvas, answered on its next frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ViewAsk {
    Zoom(f32),
    Fit,
}

/// What a canvas answers with.
pub struct GraphResponse {
    /// The canvas's ground.
    pub response: Response,
    /// The node pressed, by id.
    pub pressed: Option<String>,
    /// The ground was pressed: the choice is let go of.
    pub pane_pressed: bool,
    /// What the gestures of an editor reported, in order.
    pub events: Vec<GraphEvent>,
    /// What the toolbar's buttons were pressed for.
    pub toolbar: GraphToolbarResponse,
    /// The index of the app's control pressed, and each one's rectangle on the screen.
    pub extra: Option<usize>,
    pub extra_rects: Vec<Rect>,
    /// The empty canvas's action was pressed, and where it stands.
    pub empty_action: bool,
    pub empty_action_rect: Option<Rect>,
    /// What moves the view, and where things stand.
    pub controls: GraphControls,
}

/// A view the reader moved: a zoom and how far the drawing is moved from where a fit puts it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Moved {
    zoom: f32,
    pan: Vec2,
}

/// A graph laid out in ranks and drawn as node boxes and routed edges, fitted to its width, as
/// sokkel-ui-graph's `GraphCanvas` with `frame="fitted"`. A press on a node chooses it; the
/// buttons zoom in, out and back to the fit, and a drag moves a zoomed drawing.
///
/// The order the nodes are given in is the order the layout keeps, as the web's elk is told
/// (`considerModelOrder`, `MODEL_ORDER` cycle breaking): an edge from a later node to an earlier
/// one is laid out turned round, its target a rank before its source and the edge routed back.
/// Give the nodes in the order their edges run (callers before what they call), not by name, or
/// edges with no cycle among them come out drawn backward.
///
/// On a `Stage` frame it fills its box (or `size`) on the dotted canvas, fits the whole graph until
/// the reader pans or zooms it, and stands its zoom buttons one over another in its corner, the
/// app's `extra_controls` after them, and a `toolbar` over its top left. With `editing` it is an
/// editor: ports to connect, Delete to remove, and the gestures `GraphEditing` turns on, each
/// reported in `GraphResponse::events`. `node_menu` and `pane_menu` open menus at a right click.
pub struct GraphCanvas<'a> {
    pub(crate) id: Id,
    pub(crate) nodes: &'a [GraphNode],
    pub(crate) links: &'a [GraphLink],
    pub(crate) direction: GraphDirection,
    pub(crate) spacing: GraphSpacing,
    pub(crate) selected: Option<&'a str>,
    pub(crate) words: GraphWords,
    pub(crate) frame: GraphFrame,
    pub(crate) editing: Option<GraphEditing<'a>>,
    pub(crate) controls: bool,
    pub(crate) extra_controls: &'a [GraphControlButton],
    pub(crate) toolbar: Option<GraphToolbar>,
    pub(crate) node_menu: Option<NodeMenuFn<'a>>,
    pub(crate) pane_menu: Option<PaneMenuFn<'a>>,
    pub(crate) positions: Option<&'a HashMap<String, Pos2>>,
    pub(crate) empty_action: Option<(Option<Icon>, String)>,
    pub(crate) size: Option<Vec2>,
    pub(crate) fit_min_zoom: f32,
    pub(crate) narrow_fit: bool,
    pub(crate) layout_key: u64,
}

impl<'a> GraphCanvas<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, nodes: &'a [GraphNode], links: &'a [GraphLink]) -> Self {
        Self {
            id: Id::unique(("sokkel-graph", id)),
            nodes,
            links,
            direction: GraphDirection::Right,
            spacing: GraphSpacing::NORMAL,
            selected: None,
            words: GraphWords::default(),
            frame: GraphFrame::Fitted,
            editing: None,
            controls: true,
            extra_controls: &[],
            toolbar: None,
            node_menu: None,
            pane_menu: None,
            positions: None,
            empty_action: None,
            size: None,
            fit_min_zoom: FIT_MIN_ZOOM,
            narrow_fit: true,
            layout_key: 0,
        }
    }

    pub fn frame(mut self, frame: GraphFrame) -> Self {
        self.frame = frame;
        self
    }

    /// Makes a stage an editor.
    pub fn editing(mut self, editing: Option<GraphEditing<'a>>) -> Self {
        self.editing = editing;
        self
    }

    /// False hides the zoom buttons, for a toolbar that carries them.
    pub fn controls(mut self, controls: bool) -> Self {
        self.controls = controls;
        self
    }

    /// The app's own buttons after the zoom buttons, in their cluster; they stay while the canvas
    /// is empty.
    pub fn extra_controls(mut self, extra: &'a [GraphControlButton]) -> Self {
        self.extra_controls = extra;
        self
    }

    /// A bar over the stage's top-left corner; a fit keeps its height clear.
    pub fn toolbar(mut self, toolbar: Option<GraphToolbar>) -> Self {
        self.toolbar = toolbar;
        self
    }

    /// The menu a right click on a box opens (or Shift+F10 on the chosen one), asked each frame it
    /// is open; `None` opens none.
    pub fn node_menu(mut self, menu: &'a dyn Fn(&str) -> Option<GraphMenu>) -> Self {
        self.node_menu = Some(menu);
        self
    }

    /// The menu a right click on the ground opens, asked with the point in the graph's coordinates.
    pub fn pane_menu(mut self, menu: &'a dyn Fn(Pos2) -> Option<GraphMenu>) -> Self {
        self.pane_menu = Some(menu);
        self
    }

    /// Where nodes were moved to, by id, over where the layout put them: each box's top-left corner
    /// in the graph's coordinates.
    pub fn positions(mut self, positions: Option<&'a HashMap<String, Pos2>>) -> Self {
        self.positions = positions;
        self
    }

    /// A button an empty stage offers under its words, such as one that adds the first step.
    pub fn empty_action(mut self, label: impl Into<String>, icon: Option<Icon>) -> Self {
        self.empty_action = Some((icon, label.into()));
        self
    }

    /// The stage's size; the room it is given when left out.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    /// The least zoom a fit goes to: 0.75 when left out.
    pub fn fit_min_zoom(mut self, zoom: f32) -> Self {
        self.fit_min_zoom = zoom;
        self
    }

    /// False keeps a narrow canvas's fit at `fit_min_zoom`, rather than letting it go to 0.6.
    pub fn narrow_fit(mut self, narrow: bool) -> Self {
        self.narrow_fit = narrow;
        self
    }

    /// A value that changes with the layout chosen: the view fits again, even after the reader
    /// moved it.
    pub fn layout_key(mut self, key: u64) -> Self {
        self.layout_key = key;
        self
    }

    pub fn direction(mut self, direction: GraphDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn spacing(mut self, spacing: GraphSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    /// The node marked as chosen.
    pub fn selected(mut self, selected: Option<&'a str>) -> Self {
        self.selected = selected;
        self
    }

    pub fn words(mut self, words: GraphWords) -> Self {
        self.words = words;
        self
    }

    /// Draw the canvas; the id of the node pressed, if one was.
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        self.show_response(ui).pressed
    }

    /// Draw the canvas, and answer with everything it was told.
    pub fn show_response(self, ui: &mut Ui) -> GraphResponse {
        match self.frame {
            GraphFrame::Fitted => self.show_fitted(ui),
            GraphFrame::Stage => self.show_stage(ui),
        }
    }

    fn show_fitted(self, ui: &mut Ui) -> GraphResponse {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let placed = self.placed(ui, &t);
        let drawing = placed.bounds();
        let fit = fitted_zoom(width, drawing.width());
        let height = if self.nodes.is_empty() {
            FITTED_MIN_HEIGHT
        } else {
            (drawing.height() * fit + FITTED_GROUND).ceil().max(FITTED_MIN_HEIGHT) + BUTTON_ROW
        };
        let (rect, ground) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
        ground.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Unknown, true, &self.words.label));
        let moved_id = self.id.with("moved");
        let mut moved = ui.data(|d| d.get_temp::<Moved>(moved_id));
        if ground.dragged()
            && let Some(view) = &mut moved
        {
            view.pan += ground.drag_delta();
        }
        let zoom = moved.map_or(fit, |m| m.zoom);
        // the fit centers the drawing in the whole frame, the buttons' row included
        let fitted_at = rect.center() - drawing.size() * zoom / 2.0;
        let origin = fitted_at + moved.map_or(Vec2::ZERO, |m| m.pan);
        let at = |p: Pos2| origin + (p - drawing.min) * zoom;
        let painter = ui.painter_at(rect);

        let stroke = Stroke::new(1.4 * zoom, mix(t.color("graph-edge"), t.color("muted-foreground"), 0.7));
        for (link, route) in self.links.iter().zip(&placed.edges) {
            let points: Vec<Pos2> = route.iter().map(|p| at(*p)).collect();
            let stroke = match link.tint {
                Some(tint) => Stroke::new(stroke.width, tint),
                None => stroke,
            };
            paint_edge(&painter, &points, link.arrow, link.dashed, zoom, stroke);
        }

        let mut pressed = None;
        for (node, rect) in self.nodes.iter().zip(&placed.nodes) {
            let shown = Rect::from_min_max(at(rect.min), at(rect.max));
            let response = ui.interact(
                shown.intersect(painter.clip_rect()),
                self.id.with(("node", &node.id)),
                Sense::click(),
            );
            let title = node.name.clone().unwrap_or_else(|| node.code.clone());
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::Role::Button,
                    true,
                    self.selected == Some(node.id.as_str()),
                    &title,
                )
            });
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            node.paint(
                &painter,
                &t,
                shown,
                zoom,
                response.hovered(),
                self.selected == Some(node.id.as_str()),
            );
            if let Some(status) = &node.status {
                response
                    .on_hover_text(&status.label)
                    .clicked()
                    .then(|| pressed = Some(node.id.clone()));
            } else if response.clicked() {
                pressed = Some(node.id.clone());
            }
        }

        // the zoom buttons, in a row at the bottom right
        let buttons = GraphViewButtons::new(self.id)
            .orientation(ButtonsOrientation::Row)
            .words(self.words.view())
            .show_in_corner(ui, &painter, rect);
        if buttons.zoom_in || buttons.zoom_out || buttons.fit {
            let now = moved.unwrap_or(Moved {
                zoom: fit,
                pan: Vec2::ZERO,
            });
            moved = if buttons.fit {
                None
            } else {
                let next = if buttons.zoom_in {
                    now.zoom * ZOOM_STEP
                } else {
                    now.zoom / ZOOM_STEP
                };
                let next = next.clamp(MIN_ZOOM, MAX_ZOOM);
                // zoom about the frame's center
                let center_off = rect.center() - (fitted_at + now.pan);
                let scaled = center_off * (next / now.zoom);
                let fitted_next = rect.center() - drawing.size() * next / 2.0;
                Some(Moved {
                    zoom: next,
                    pan: rect.center() - scaled - fitted_next,
                })
            };
        }
        ui.data_mut(|d| match moved {
            Some(view) => {
                d.insert_temp(moved_id, view);
            }
            None => d.remove::<Moved>(moved_id),
        });
        let controls = GraphControls {
            id: self.id,
            rect,
            view: GraphView {
                x: origin.x - rect.min.x - drawing.min.x * zoom,
                y: origin.y - rect.min.y - drawing.min.y * zoom,
                zoom,
            },
            direction: self.direction,
            empty: self.nodes.is_empty(),
            boxes: self
                .nodes
                .iter()
                .map(|n| n.id.clone())
                .zip(placed.nodes.iter().copied())
                .collect(),
            edges: Vec::new(),
        };
        GraphResponse {
            response: ground,
            pressed,
            pane_pressed: false,
            events: Vec::new(),
            toolbar: GraphToolbarResponse::default(),
            extra: None,
            extra_rects: Vec::new(),
            empty_action: false,
            empty_action_rect: None,
            controls,
        }
    }

    /// The graph placed, kept while its nodes' sizes and links stay the same.
    pub(crate) fn placed(&self, ui: &Ui, t: &Tokens) -> PlacedGraph {
        let painter = ui.painter();
        let sizes: Vec<Vec2> = self.nodes.iter().map(|n| n.size(painter, t)).collect();
        let index = |id: &str| self.nodes.iter().position(|n| n.id == id);
        let edges: Vec<(usize, usize)> = self
            .links
            .iter()
            .map(|l| match (index(&l.source), index(&l.target)) {
                (Some(s), Some(t)) => (s, t),
                // a link to a node not drawn is laid out as nothing
                _ => (usize::MAX, usize::MAX),
            })
            .collect();
        let key = self.id.with((
            sizes.iter().map(|s| (s.x.to_bits(), s.y.to_bits())).collect::<Vec<_>>(),
            &edges,
            self.direction,
        ));
        if let Some(placed) = ui.data(|d| d.get_temp::<PlacedGraph>(key)) {
            return placed;
        }
        let placed = layout(&sizes, &edges, self.direction, self.spacing);
        ui.data_mut(|d| d.insert_temp(key, placed.clone()));
        placed
    }
}

/// The zoom a fitted frame settles at for a drawing this wide in a frame this wide.
pub fn fitted_zoom(width: f32, drawing: f32) -> f32 {
    if drawing <= 0.0 || width <= 0.0 {
        return FIT_MAX_ZOOM;
    }
    let floor = if width >= NARROW_GRAPH {
        FIT_MIN_ZOOM
    } else {
        let across = width * (1.0 - 2.0 * FIT_PADDING) / drawing;
        FIT_MIN_ZOOM.min(across.max(NARROW_FIT_MIN_ZOOM))
    };
    (width / (drawing * (1.0 + FIT_PADDING))).clamp(floor, FIT_MAX_ZOOM)
}

/// The least zoom a fit goes to on a canvas this wide: `min_zoom`, or on a narrow canvas with
/// `narrow_fit` as little as fits the drawing's width, down to 0.6.
pub fn fit_floor(width: f32, drawing: f32, min_zoom: f32, narrow_fit: bool) -> f32 {
    if !narrow_fit || width >= NARROW_GRAPH {
        return min_zoom;
    }
    let across = if drawing > 0.0 {
        width * (1.0 - 2.0 * FIT_PADDING) / drawing
    } else {
        min_zoom
    };
    min_zoom.min(across.max(NARROW_FIT_MIN_ZOOM))
}

/// The direction a flow is laid out in for the width its canvas has: down when narrow, else right.
pub fn fit_direction(width: f32) -> GraphDirection {
    if width > 0.0 && width < NARROW_GRAPH {
        GraphDirection::Down
    } else {
        GraphDirection::Right
    }
}

/// The view that fits `drawing` on a canvas of `canvas`, `top` kept clear for a toolbar, as the
/// web's `fitGraph`: React Flow's `getViewportForBounds` with 12% padding between the fit's floor
/// and 1, and what does not fit hung off the far edges, its start at the margin.
pub fn fit_view(drawing: Rect, canvas: Vec2, top: f32, min_zoom: f32, narrow_fit: bool) -> GraphView {
    let (width, height) = (canvas.x, canvas.y - top);
    if drawing.width() <= 0.0 && drawing.height() <= 0.0 || width <= 0.0 || height <= 0.0 {
        return GraphView {
            x: 0.0,
            y: top,
            zoom: 1.0,
        };
    }
    let least = fit_floor(width, drawing.width(), min_zoom, narrow_fit);
    let pad = |side: f32| ((side - side / (1.0 + FIT_PADDING)) * 0.5).floor();
    let (px, py) = (pad(width), pad(height));
    let across = if drawing.width() > 0.0 {
        (width - 2.0 * px) / drawing.width()
    } else {
        f32::INFINITY
    };
    let down = if drawing.height() > 0.0 {
        (height - 2.0 * py) / drawing.height()
    } else {
        f32::INFINITY
    };
    let zoom = across.min(down).clamp(least, FIT_MAX_ZOOM);
    let mut x = width / 2.0 - drawing.center().x * zoom;
    let mut y = height / 2.0 - drawing.center().y * zoom;
    // the paddings the centered view leaves, made up where one is short
    let left = (drawing.left() * zoom + x).floor();
    let above = (drawing.top() * zoom + y).floor();
    let right = (width - (drawing.right() * zoom + x)).floor();
    let below = (height - (drawing.bottom() * zoom + y)).floor();
    x = x - (left - px).min(0.0) + (right - px).min(0.0);
    y = y - (above - py).min(0.0) + (below - py).min(0.0);
    if drawing.width() * zoom > width {
        x = OVERFLOW_MARGIN - drawing.left() * zoom;
    }
    if drawing.height() * zoom > height {
        y = OVERFLOW_MARGIN - drawing.top() * zoom;
    }
    GraphView { x, y: y + top, zoom }
}

/// The point a box's edges leave from (`out`) or arrive at, on the sides its direction gives.
pub(crate) fn port_of(rect: Rect, direction: GraphDirection, out: bool) -> Pos2 {
    match (direction, out) {
        (GraphDirection::Down, true) => rect.center_bottom(),
        (GraphDirection::Down, false) => rect.center_top(),
        (GraphDirection::Right, true) => rect.right_center(),
        (GraphDirection::Right, false) => rect.left_center(),
    }
}

/// A route between two boxes with no layout's route, as one moved by hand: out of the source's far
/// side and into the target's near side, through two points halfway along.
pub fn rough_route(source: Rect, target: Rect, direction: GraphDirection) -> Vec<Pos2> {
    let start = port_of(source, direction, true);
    let end = port_of(target, direction, false);
    match direction {
        GraphDirection::Down => {
            let middle = (start.y + end.y) / 2.0;
            vec![start, Pos2::new(start.x, middle), Pos2::new(end.x, middle), end]
        }
        GraphDirection::Right => {
            let middle = (start.x + end.x) / 2.0;
            vec![start, Pos2::new(middle, start.y), Pos2::new(middle, end.y), end]
        }
    }
}

/// The middle of a route, by length.
pub(crate) fn midpoint(points: &[Pos2]) -> Option<Pos2> {
    if points.len() < 2 {
        return points.first().copied();
    }
    let total: f32 = points.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    let mut left = total / 2.0;
    for w in points.windows(2) {
        let length = (w[1] - w[0]).length();
        if left <= length {
            let share = if length == 0.0 { 0.0 } else { left / length };
            return Some(w[0] + (w[1] - w[0]) * share);
        }
        left -= length;
    }
    points.last().copied()
}

/// A route's points with each bend turned on a radius of `bend`, as a line to draw.
pub(crate) fn rounded(route: &[Pos2], bend: f32) -> Vec<Pos2> {
    let mut line = vec![route[0]];
    for k in 1..route.len() - 1 {
        let (a, corner, b) = (route[k - 1], route[k], route[k + 1]);
        let inward = bend.min((corner - a).length() / 2.0).min((b - corner).length() / 2.0);
        let from = corner + (a - corner).normalized() * inward;
        let to = corner + (b - corner).normalized() * inward;
        for s in 0..=6 {
            let u = s as f32 / 6.0;
            let p = from.lerp(corner, u).lerp(corner.lerp(to, u), u);
            line.push(p);
        }
    }
    line.push(route[route.len() - 1]);
    line
}

/// An edge along its route, its corners rounded, ending in an arrowhead when it has one.
fn paint_edge(painter: &egui::Painter, points: &[Pos2], arrow: bool, dashed: bool, zoom: f32, stroke: Stroke) {
    if points.len() < 2 {
        return;
    }
    let mut route = points.to_vec();
    let tip = route[route.len() - 1];
    let before = route[route.len() - 2];
    let way = (tip - before).normalized();
    if arrow {
        // the line stops at the arrowhead's base
        let last = route.len() - 1;
        route[last] = tip - way * ARROW * zoom * 0.8;
    }
    let line = rounded(&route, CORNER * zoom);
    if dashed {
        painter.extend(Shape::dashed_line(&line, stroke, 4.0 * zoom, 3.0 * zoom));
    } else {
        painter.add(Shape::line(line, stroke));
    }
    if arrow {
        let length = ARROW * zoom;
        let base = tip - way * length;
        let side = way.rot90() * length / 2.0;
        painter.add(Shape::convex_polygon(
            vec![tip, base + side, base - side],
            stroke.color,
            Stroke::NONE,
        ));
    }
}

/// `share` of `a` over `b`, as `color-mix` mixes two opaque colors.
pub(crate) fn mix(a: Color32, b: Color32, share: f32) -> Color32 {
    let channel = |x: u8, y: u8| (x as f32 * share + y as f32 * (1.0 - share)).round() as u8;
    Color32::from_rgb(channel(a.r(), b.r()), channel(a.g(), b.g()), channel(a.b(), b.b()))
}
