//! A graph's canvas, after sokkel-ui-graph's `GraphCanvas` in its `fitted` frame: as tall as its
//! drawing fitted to its width, the zoom buttons in a row at its foot.

use egui::{Color32, CornerRadius, Id, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, vec2};
use sokkel_egui::{Icon, Tokens, tokens};

use crate::layout::{GraphDirection, GraphSpacing, PlacedGraph, layout};
use crate::node::GraphNode;

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
/// One zoom button's side.
const BUTTON: f32 = 28.0;
/// An arrowhead's length, and the radius an edge's corners are rounded to.
const ARROW: f32 = 7.0;
const CORNER: f32 = 6.0;

/// An edge from one node to another by their ids, with an arrowhead at its target when `arrow`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub arrow: bool,
}

impl GraphLink {
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            arrow: false,
        }
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
}

impl Default for GraphWords {
    fn default() -> Self {
        Self {
            label: "Graph".into(),
            zoom_in: "Zoom in".into(),
            zoom_out: "Zoom out".into(),
            fit: "Fit the graph".into(),
        }
    }
}

/// A view the reader moved: a zoom and how far the drawing is moved from where a fit puts it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Moved {
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
pub struct GraphCanvas<'a> {
    id: Id,
    nodes: &'a [GraphNode],
    links: &'a [GraphLink],
    direction: GraphDirection,
    spacing: GraphSpacing,
    selected: Option<&'a str>,
    words: GraphWords,
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
        }
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
            paint_edge(&painter, &points, link.arrow, zoom, stroke);
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
        let group = Rect::from_min_size(
            rect.right_bottom() - vec2(3.0 * BUTTON + 2.0 + 8.0, BUTTON + 2.0 + 8.0),
            vec2(3.0 * BUTTON + 2.0, BUTTON + 2.0),
        );
        let corners = t.radius("control");
        for shadow in t.shadows("elevation-node") {
            painter.add(shadow.as_shape(group, corners));
        }
        painter.rect(
            group,
            corners,
            t.color("card"),
            Stroke::new(1.0, t.color("border")),
            StrokeKind::Inside,
        );
        let buttons = [
            (Icon::ZoomIn, &self.words.zoom_in),
            (Icon::ZoomOut, &self.words.zoom_out),
            (Icon::Scan, &self.words.fit),
        ];
        for (k, (icon, word)) in buttons.into_iter().enumerate() {
            let spot = Rect::from_min_size(group.min + vec2(1.0 + k as f32 * BUTTON, 1.0), Vec2::splat(BUTTON));
            let response = ui.interact(spot, self.id.with(("button", k)), Sense::click());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word.as_str()));
            if k > 0 {
                painter.vline(spot.left() - 0.5, spot.y_range(), Stroke::new(1.0, t.color("border")));
            }
            let ink = if response.hovered() {
                painter.rect_filled(spot.shrink(0.0), CornerRadius::ZERO, t.color("hover"));
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            sokkel_egui::icons::paint(&painter, spot.center(), icon, 14.0, ink);
            let response = response.on_hover_text(word.as_str());
            if response.clicked() {
                let now = moved.unwrap_or(Moved {
                    zoom: fit,
                    pan: Vec2::ZERO,
                });
                moved = match k {
                    0 | 1 => {
                        let next = if k == 0 {
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
                    }
                    _ => None,
                };
            }
        }
        ui.data_mut(|d| match moved {
            Some(view) => {
                d.insert_temp(moved_id, view);
            }
            None => d.remove::<Moved>(moved_id),
        });
        pressed
    }

    /// The graph placed, kept while its nodes' sizes and links stay the same.
    fn placed(&self, ui: &Ui, t: &Tokens) -> PlacedGraph {
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

/// An edge along its route, its corners rounded, ending in an arrowhead when it has one.
fn paint_edge(painter: &egui::Painter, points: &[Pos2], arrow: bool, zoom: f32, stroke: Stroke) {
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
    let mut line = vec![route[0]];
    for k in 1..route.len() - 1 {
        let (a, corner, b) = (route[k - 1], route[k], route[k + 1]);
        let inward = (CORNER * zoom)
            .min((corner - a).length() / 2.0)
            .min((b - corner).length() / 2.0);
        let from = corner + (a - corner).normalized() * inward;
        let to = corner + (b - corner).normalized() * inward;
        for s in 0..=6 {
            let u = s as f32 / 6.0;
            let p = from.lerp(corner, u).lerp(corner.lerp(to, u), u);
            line.push(p);
        }
    }
    line.push(route[route.len() - 1]);
    painter.add(Shape::line(line, stroke));
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
fn mix(a: Color32, b: Color32, share: f32) -> Color32 {
    let channel = |x: u8, y: u8| (x as f32 * share + y as f32 * (1.0 - share)).round() as u8;
    Color32::from_rgb(channel(a.r(), b.r()), channel(a.g(), b.g()), channel(a.b(), b.b()))
}
