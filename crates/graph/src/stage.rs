//! A graph's canvas in its `stage` frame, after sokkel-ui-graph's `GraphCanvas` on React Flow: the
//! dotted ground the view pans and zooms over, and with `editing` an editor whose gestures it
//! reports.

use std::collections::HashMap;

use egui::{
    CornerRadius, CursorIcon, DragAndDrop, Key, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, pos2,
    vec2,
};
use sokkel_egui::{Button, ButtonSize, DropdownMenu, Icon, MenuItem, TextRole, Tokens, tokens};

use crate::blocks::GraphDragItem;
use crate::canvas::{
    CORNER, EdgeMotion, EdgeTone, GraphCanvas, GraphControls, GraphEdgeRef, GraphEvent, GraphMenu, GraphPointer,
    GraphResponse, GraphView, MAX_ZOOM, MIN_ZOOM, ViewAsk, ZOOM_STEP, fit_view, mix, port_of, rough_route, rounded,
};
use crate::layout::GraphDirection;
use crate::node::NodeIssue;
use crate::toolbar::{GraphToolbarResponse, TOOLBAR_HEIGHT, TOOLBAR_MARGIN, galley};
use crate::view_buttons::{ButtonsOrientation, GraphViewButtons};

/// The stage's dots: their spacing and size, as React Flow's dotted background.
const DOT_GAP: f32 = 18.0;
const DOT_SIZE: f32 = 1.0;
/// A port's side, and an output port grown into an add button.
const PORT: f32 = 10.0;
const PORT_GROWN: f32 = 16.0;
/// The least a port is hit at, however far out the view is zoomed.
const PORT_HIT: f32 = 12.0;
/// The add button at an edge's middle, and the glyph inside an add button.
const INSERT: f32 = 16.0;
const PLUS: f32 = 10.0;
/// How near the pointer comes to an edge to point at it.
const EDGE_HIT: f32 = 6.0;
/// How much a wheel's pixel zooms, as d3-zoom counts it.
const WHEEL_ZOOM: f32 = 0.002;
/// How long a handover's pulse lasts, and how long a flowing edge's dashes take to move one dash on.
const PULSE_SECONDS: f64 = 0.6;
const FLOW_SECONDS: f64 = 0.7;
/// Where the reason a connection is refused stands from the pointer, and the widest it is.
const VERDICT_OFFSET: f32 = 14.0;
const VERDICT_WIDTH: f32 = 256.0;
/// The gap between an empty canvas's words and its action.
const EMPTY_GAP: f32 = 12.0;

/// A box held by the pointer: its top-left corner in the graph, and where the pointer took it.
#[derive(Clone, Debug)]
struct Held {
    node: String,
    at: Pos2,
    grip: Vec2,
}

/// A connection being drawn from a node's port: `outward` from its output.
#[derive(Clone, Debug)]
struct Connecting {
    from: String,
    outward: bool,
}

/// A menu open at a point: a node's, or the ground's when `node` is none; opened the frame after
/// the click, so the click does not close it again.
#[derive(Clone, Debug)]
struct OpenMenu {
    node: Option<String>,
    at: Pos2,
    graph_at: Pos2,
    opened: bool,
}

/// What the stage keeps between frames.
#[derive(Clone, Debug, Default)]
struct StageState {
    moved: Option<GraphView>,
    laid_out: Option<(GraphDirection, u64)>,
    chosen_edge: Option<String>,
    hovered_edge: Option<String>,
    held: Option<Held>,
    connecting: Option<Connecting>,
    menu: Option<OpenMenu>,
    /// The last press landed on the canvas, so its keys are its own.
    active: bool,
    /// When each edge's pulse began.
    pulses: HashMap<String, f64>,
}

/// One edge as the stage draws it.
struct Drawn {
    edge: GraphEdgeRef,
    link: usize,
    route: Vec<Pos2>,
}

impl GraphCanvas<'_> {
    pub(crate) fn show_stage(self, ui: &mut Ui) -> GraphResponse {
        let t = tokens(ui.ctx());
        let size = self.size.unwrap_or_else(|| ui.available_size());
        let (rect, ground) = ui.allocate_exact_size(size, Sense::click_and_drag());
        ground.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Unknown, true, &self.words.label));
        let state_id = self.id.with("stage");
        let mut state: StageState = ui.data_mut(|d| d.get_temp(state_id)).unwrap_or_default();
        let editable = self.editing.is_some();
        let editing = self.editing.unwrap_or_default();
        let now = ui.input(|i| i.time);
        let mut events = Vec::new();
        let mut pressed = None;
        let mut pane_pressed = false;

        // a new layout chosen fits the view again, even one the reader had moved
        let laid_out = (self.direction, self.layout_key);
        if state.laid_out != Some(laid_out) {
            state.laid_out = Some(laid_out);
            state.moved = None;
        }

        // the boxes: where the layout put them, where the app moved them, or where the pointer holds one
        let placed = self.placed(ui, &t);
        let boxes: Vec<Rect> = self
            .nodes
            .iter()
            .zip(&placed.nodes)
            .map(|(node, laid)| {
                let at = match &state.held {
                    Some(held) if held.node == node.id => Some(held.at),
                    _ => self.positions.and_then(|p| p.get(&node.id).copied()),
                };
                at.map_or(*laid, |at| Rect::from_min_size(at, laid.size()))
            })
            .collect();
        let moved_by_hand = |id: &str| {
            self.positions.is_some_and(|p| p.contains_key(id)) || state.held.as_ref().is_some_and(|h| h.node == id)
        };
        let index = |id: &str| self.nodes.iter().position(|n| n.id == id);
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut drawn: Vec<Drawn> = Vec::new();
        for (k, link) in self.links.iter().enumerate() {
            let base = format!("{}->{}", link.source, link.target);
            let count = seen.entry(base.clone()).or_insert(0);
            let id = if *count == 0 { base } else { format!("{base}#{count}") };
            *count += 1;
            let (Some(s), Some(e)) = (index(&link.source), index(&link.target)) else {
                continue;
            };
            if s == e {
                continue;
            }
            // a route the layout drew no longer fits an edge whose node was moved
            let laid = placed.edges.get(k).filter(|r| r.len() >= 2);
            let route = match laid {
                Some(route) if !moved_by_hand(&link.source) && !moved_by_hand(&link.target) => route.clone(),
                _ => rough_route(boxes[s], boxes[e], self.direction),
            };
            drawn.push(Drawn {
                edge: GraphEdgeRef {
                    id,
                    source: link.source.clone(),
                    target: link.target.clone(),
                },
                link: k,
                route,
            });
        }
        let mut drawing = Rect::NOTHING;
        for b in &boxes {
            drawing = drawing.union(*b);
        }
        for p in drawn.iter().flat_map(|d| &d.route) {
            drawing.extend_with(*p);
        }
        if drawing.is_negative() {
            drawing = Rect::ZERO;
        }
        let empty = self.nodes.is_empty();

        // the view: a fit until the reader moves it, kept clear of the toolbar
        let top = if self.toolbar.is_some() {
            TOOLBAR_MARGIN + TOOLBAR_HEIGHT
        } else {
            0.0
        };
        let fitted = fit_view(drawing, rect.size(), top, self.fit_min_zoom, self.narrow_fit);
        let asked: Option<ViewAsk> = ui.data_mut(|d| {
            let asked = d.get_temp(self.id.with("asked"));
            d.remove::<ViewAsk>(self.id.with("asked"));
            asked
        });
        let mut view = state.moved.unwrap_or(fitted);
        match asked {
            Some(ViewAsk::Fit) => {
                state.moved = None;
                view = fitted;
            }
            Some(ViewAsk::Zoom(factor)) => {
                view = view.zoomed(view.zoom * factor, rect.size() / 2.0);
                state.moved = Some(view);
            }
            None => {}
        }
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let over_canvas = pointer.is_some_and(|p| rect.contains(p) && ui.ctx().layer_id_at(p) == Some(ui.layer_id()));
        if ground.dragged() && state.held.is_none() && state.connecting.is_none() {
            view.x += ground.drag_delta().x;
            view.y += ground.drag_delta().y;
            state.moved = Some(view);
        }
        if over_canvas && let Some(pointer) = pointer {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let factor = 2f32.powf(scroll * WHEEL_ZOOM) * pinch;
            if factor != 1.0 {
                view = view.zoomed(view.zoom * factor, pointer - rect.min);
                state.moved = Some(view);
                ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            }
        }
        let zoom = view.zoom;
        let at = |p: Pos2| rect.min + vec2(view.x, view.y) + p.to_vec2() * zoom;
        let to_graph = |p: Pos2| ((p - rect.min - vec2(view.x, view.y)) / zoom).to_pos2();
        let shown: Vec<Rect> = boxes.iter().map(|b| Rect::from_min_max(at(b.min), at(b.max))).collect();
        let painter = ui.painter_at(rect);
        let clip = painter.clip_rect();

        // a press anywhere decides whose the keys are
        if ui.input(|i| i.pointer.any_pressed())
            && let Some(p) = ui.input(|i| i.pointer.press_origin())
        {
            state.active = rect.contains(p);
        }

        // the nodes answer the pointer: a press chooses one, a drag moves it on an editor
        let moves = editable && editing.moves;
        let mut node_hovered = vec![false; self.nodes.len()];
        for (k, node) in self.nodes.iter().enumerate() {
            let sense = if moves { Sense::click_and_drag() } else { Sense::click() };
            let response = ui.interact(shown[k].intersect(clip), self.id.with(("node", &node.id)), sense);
            let title = node.name.clone().unwrap_or_else(|| node.code.clone());
            let chosen = self.selected == Some(node.id.as_str());
            response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, chosen, &title));
            node_hovered[k] = response.hovered() || response.dragged();
            if response.hovered() {
                ui.ctx().set_cursor_icon(if moves {
                    CursorIcon::Grab
                } else {
                    CursorIcon::PointingHand
                });
            }
            if response.clicked() {
                pressed = Some(node.id.clone());
                state.chosen_edge = None;
            }
            if response.secondary_clicked()
                && let Some(p) = response.interact_pointer_pos()
            {
                self.open_node_menu(&mut state, &mut events, &node.id, p, to_graph(p), &editing, editable);
            }
            if moves && response.drag_started() {
                let press = ui.input(|i| i.pointer.press_origin()).unwrap_or(shown[k].center());
                state.held = Some(Held {
                    node: node.id.clone(),
                    at: boxes[k].min,
                    grip: to_graph(press) - boxes[k].min,
                });
                state.moved = Some(view);
            }
            if moves
                && response.dragged()
                && let (Some(held), Some(p)) = (&mut state.held, response.interact_pointer_pos())
                && held.node == node.id
            {
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                held.at = to_graph(p) - held.grip;
            }
            if moves
                && response.drag_stopped()
                && let Some(held) = state.held.take()
            {
                events.push(GraphEvent::Move {
                    node: held.node,
                    at: held.at,
                });
            }
        }

        // the ports of an editor: a drag from one draws a connection, a click on an output adds a step
        let mut port_rects: Vec<(Rect, Rect)> = Vec::new();
        let mut port_hovered: Vec<(bool, bool)> = Vec::new();
        if editable {
            for (k, node) in self.nodes.iter().enumerate() {
                let appends = editing.append && editing.can_append.is_none_or(|can| can(&node.id));
                let grown = appends && (node_hovered[k] || self.selected == Some(node.id.as_str()));
                let side = |big: bool| (if big { PORT_GROWN } else { PORT }) * zoom;
                let input =
                    Rect::from_center_size(at(port_of(boxes[k], self.direction, false)), Vec2::splat(side(false)));
                let output =
                    Rect::from_center_size(at(port_of(boxes[k], self.direction, true)), Vec2::splat(side(grown)));
                let hit = |r: Rect| Rect::from_center_size(r.center(), r.size().max(Vec2::splat(PORT_HIT)));
                let into = ui.interact(
                    hit(input).intersect(clip),
                    self.id.with(("in", &node.id)),
                    Sense::drag(),
                );
                let out_sense = if appends {
                    Sense::click_and_drag()
                } else {
                    Sense::drag()
                };
                let out = ui.interact(hit(output).intersect(clip), self.id.with(("out", &node.id)), out_sense);
                if appends {
                    out.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.append));
                }
                for (response, outward) in [(&into, false), (&out, true)] {
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                    }
                    if response.drag_started() {
                        state.connecting = Some(Connecting {
                            from: node.id.clone(),
                            outward,
                        });
                        state.chosen_edge = None;
                    }
                    if response.drag_stopped()
                        && let Some(connecting) = state.connecting.take()
                    {
                        let end = response.interact_pointer_pos().or(pointer);
                        self.end_connection(&mut events, &connecting, end, &shown, &editing, to_graph);
                    }
                }
                if appends && out.clicked() {
                    let p = out.interact_pointer_pos().unwrap_or(output.center());
                    events.push(GraphEvent::Append {
                        node: node.id.clone(),
                        at: GraphPointer {
                            screen: p,
                            at: to_graph(p),
                        },
                    });
                }
                let out_hovered = out.hovered();
                let into_hovered = into.hovered();
                if appends && out_hovered {
                    out.on_hover_text(&self.words.append);
                }
                port_rects.push((input, output));
                port_hovered.push((into_hovered, out_hovered));
            }
        }

        // the edge under the pointer, on an editor whose edges take an add button
        let screen_routes: Vec<Vec<Pos2>> = drawn
            .iter()
            .map(|d| rounded(&d.route.iter().map(|p| at(*p)).collect::<Vec<_>>(), CORNER * zoom))
            .collect();
        let near_edge = |p: Pos2| -> Option<usize> {
            screen_routes
                .iter()
                .enumerate()
                .map(|(k, line)| (k, distance_to(line, p)))
                .filter(|(_, d)| *d <= EDGE_HIT)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(k, _)| k)
        };
        let insert_rect = |edge: &str| -> Option<Rect> {
            let d = drawn.iter().find(|d| d.edge.id == edge)?;
            let middle = crate::canvas::midpoint(&d.route)?;
            Some(Rect::from_center_size(at(middle), Vec2::splat(INSERT * zoom)))
        };
        if editable && editing.insert {
            let over_insert = state
                .hovered_edge
                .as_deref()
                .and_then(insert_rect)
                .zip(pointer)
                .is_some_and(|(r, p)| r.contains(p));
            if !over_insert {
                state.hovered_edge = if ground.hovered() {
                    pointer.and_then(near_edge).map(|k| drawn[k].edge.id.clone())
                } else {
                    None
                };
            }
        }

        // the add button at an edge's middle, while the edge is pointed at or chosen
        let mut insert_shown: Option<(Rect, bool)> = None;
        if editable && editing.insert && state.connecting.is_none() {
            let target = state.hovered_edge.clone().or_else(|| state.chosen_edge.clone());
            if let Some(edge) = target
                && let Some(spot) = insert_rect(&edge)
                && let Some(d) = drawn.iter().find(|d| d.edge.id == edge)
            {
                let response = ui.interact(spot.intersect(clip), self.id.with(("insert", &edge)), Sense::click());
                response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.insert));
                let hovered = response.hovered();
                if response.clicked() {
                    let p = response.interact_pointer_pos().unwrap_or(spot.center());
                    events.push(GraphEvent::Insert {
                        edge: d.edge.clone(),
                        at: GraphPointer {
                            screen: p,
                            at: to_graph(p),
                        },
                    });
                    state.hovered_edge = None;
                }
                insert_shown = Some((spot, hovered));
            }
        }

        // the ground: a press lets go of the choice or chooses the edge under it
        if ground.clicked() {
            match pointer.and_then(near_edge).filter(|_| editable) {
                Some(k) => {
                    state.chosen_edge = Some(drawn[k].edge.id.clone());
                }
                None => {
                    pane_pressed = true;
                    state.chosen_edge = None;
                }
            }
        }
        if ground.secondary_clicked()
            && let Some(p) = ground.interact_pointer_pos()
        {
            let graph_at = to_graph(p);
            if self.pane_menu.and_then(|menu| menu(graph_at)).is_some() {
                state.menu = Some(OpenMenu {
                    node: None,
                    at: p,
                    graph_at,
                    opened: false,
                });
            }
            if editable && editing.pane_menu {
                events.push(GraphEvent::PaneMenu {
                    at: GraphPointer {
                        screen: p,
                        at: graph_at,
                    },
                });
            }
        }

        // a block dragged in from a palette
        let dropping = editable
            && editing.drop_items
            && DragAndDrop::has_payload_of_type::<GraphDragItem>(ui.ctx())
            && pointer.is_some_and(|p| rect.contains(p));
        if dropping
            && ui.input(|i| i.pointer.any_released())
            && let (Some(item), Some(p)) = (DragAndDrop::take_payload::<GraphDragItem>(ui.ctx()), pointer)
        {
            events.push(GraphEvent::DropItem {
                item: item.0.clone(),
                at: GraphPointer {
                    screen: p,
                    at: to_graph(p),
                },
            });
        }

        // the keys, while the last press was on the canvas and no field has them
        if state.active && !ui.ctx().text_edit_focused() {
            let (delete, escape, menu_key) = ui.input(|i| {
                (
                    i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                    i.key_pressed(Key::Escape),
                    i.key_pressed(Key::F10) && i.modifiers.shift,
                )
            });
            if delete && editable {
                let gone = self.selected.map(str::to_owned);
                if let Some(node) = &gone {
                    events.push(GraphEvent::Remove(node.clone()));
                }
                if let Some(edge) = state.chosen_edge.take()
                    && let Some(d) = drawn.iter().find(|d| d.edge.id == edge)
                    && gone.as_deref() != Some(d.edge.source.as_str())
                    && gone.as_deref() != Some(d.edge.target.as_str())
                {
                    events.push(GraphEvent::Disconnect(d.edge.clone()));
                }
            }
            if escape {
                if state.menu.is_some() {
                    state.menu = None;
                } else {
                    state.connecting = None;
                    state.chosen_edge = None;
                }
            }
            if menu_key
                && let Some(chosen) = self.selected
                && let Some(k) = index(chosen)
            {
                let p = shown[k].center();
                self.open_node_menu(&mut state, &mut events, chosen, p, to_graph(p), &editing, editable);
            }
        }

        // the ground and its dots
        painter.rect_filled(rect, t.radius("surface"), t.color("canvas"));
        paint_dots(&painter, &t, rect, view);

        // the edges, under the boxes
        let resting = mix(t.color("graph-edge"), t.color("muted-foreground"), 0.7);
        let moving_ink = mix(t.color("foreground"), t.color("graph-edge"), 0.75);
        let still = ui.style().animation_time <= 0.0;
        state.pulses.retain(|edge, _| {
            drawn
                .iter()
                .any(|d| &d.edge.id == edge && self.links[d.link].motion == Some(EdgeMotion::Pulse))
        });
        let mut animating = false;
        for (d, line) in drawn.iter().zip(&screen_routes) {
            let link = &self.links[d.link];
            let chosen = state.chosen_edge.as_deref() == Some(d.edge.id.as_str());
            let base = match (link.tone, link.tint) {
                (EdgeTone::Strong, _) => t.color("muted-foreground"),
                (_, Some(tint)) => mix(tint, resting, 0.35),
                _ => resting,
            };
            let moving = matches!(link.motion, Some(EdgeMotion::Flowing | EdgeMotion::Lit));
            let mut stroke = if chosen {
                Stroke::new(2.25 * zoom, t.color("foreground"))
            } else if moving {
                Stroke::new(2.0 * zoom, moving_ink)
            } else if link.tone == EdgeTone::Strong {
                Stroke::new(1.75 * zoom, base)
            } else {
                Stroke::new(1.4 * zoom, base)
            };
            if link.motion == Some(EdgeMotion::Pulse) && !chosen {
                let began = *state.pulses.entry(d.edge.id.clone()).or_insert(now);
                let u = ((now - began) / PULSE_SECONDS).clamp(0.0, 1.0) as f32;
                if u < 1.0 {
                    // ease out from the ink at 3 wide into the resting stroke
                    let eased = 1.0 - (1.0 - u) * (1.0 - u);
                    stroke = Stroke::new(
                        egui::lerp((3.0 * zoom)..=stroke.width, eased),
                        mix(stroke.color, t.color("foreground"), eased),
                    );
                    animating = true;
                }
            }
            if link.motion == Some(EdgeMotion::Flowing) && !still {
                let phase = ((now % FLOW_SECONDS) / FLOW_SECONDS) as f32;
                painter.extend(Shape::dashed_line_with_offset(
                    line,
                    stroke,
                    &[6.0 * zoom],
                    &[5.0 * zoom],
                    (1.0 - phase) * 11.0 * zoom,
                ));
                animating = true;
            } else if link.dashed {
                painter.extend(Shape::dashed_line(line, stroke, 4.0 * zoom, 4.0 * zoom));
            } else {
                painter.add(Shape::line(line.clone(), stroke));
            }
            if link.arrow && line.len() >= 2 {
                let tip = line[line.len() - 1];
                let way = (tip - line[line.len() - 2]).normalized();
                let length = 7.0 * zoom;
                let base = tip - way * length;
                let side = way.rot90() * length / 2.0;
                painter.add(Shape::convex_polygon(
                    vec![tip, base + side, base - side],
                    stroke.color,
                    Stroke::NONE,
                ));
            }
        }
        if animating {
            ui.ctx().request_repaint();
        }

        // the connection being drawn, from its port to the pointer
        let mut verdict: Option<(Pos2, String)> = None;
        if let (Some(connecting), Some(p)) = (&state.connecting, pointer)
            && let Some(k) = index(&connecting.from)
        {
            let from = at(port_of(boxes[k], self.direction, connecting.outward));
            paint_connection(&painter, &t, from, p, self.direction, connecting.outward, zoom);
            if let Some(over) = node_under(&shown, p).map(|k| self.nodes[k].id.as_str())
                && over != connecting.from
            {
                let (source, target) = if connecting.outward {
                    (connecting.from.as_str(), over)
                } else {
                    (over, connecting.from.as_str())
                };
                if let Some(reason) = editing.refusal(source, target) {
                    verdict = Some((p, reason));
                }
            }
        }

        // the boxes, their issue marks saying what is wrong
        for (k, node) in self.nodes.iter().enumerate() {
            let chosen = self.selected == Some(node.id.as_str());
            let painted = node.paint(&painter, &t, shown[k], zoom, node_hovered[k], chosen);
            if let Some(mark) = painted.mark {
                let said = spoken(&node.issues);
                ui.interact(mark.intersect(clip), self.id.with(("issues", &node.id)), Sense::hover())
                    .on_hover_text(said);
            }
        }

        // the ports over the boxes
        if editable {
            let connecting = state.connecting.is_some();
            for (k, ((input, output), (into_hovered, out_hovered))) in port_rects.iter().zip(&port_hovered).enumerate()
            {
                let node = &self.nodes[k];
                let appends = editing.append && editing.can_append.is_none_or(|can| can(&node.id));
                let grown = output.width() > PORT * zoom + 0.5;
                paint_port(&painter, &t, *input, *into_hovered, connecting, zoom);
                paint_port(&painter, &t, *output, *out_hovered, connecting, zoom);
                if appends && grown {
                    let ink = if *out_hovered {
                        t.color("background")
                    } else {
                        t.color("muted-foreground")
                    };
                    sokkel_egui::icons::paint(&painter, output.center(), Icon::Plus, PLUS * zoom, ink);
                }
            }
        }

        // the add button at an edge's middle
        if let Some((spot, hovered)) = insert_shown {
            let (fill, edge, ink) = if hovered {
                (t.color("ring"), t.color("ring"), t.color("background"))
            } else {
                (t.color("card"), t.color("border-strong"), t.color("muted-foreground"))
            };
            painter.circle(spot.center(), spot.width() / 2.0, fill, Stroke::new(zoom, edge));
            sokkel_egui::icons::paint(&painter, spot.center(), Icon::Plus, PLUS * zoom, ink);
        }

        // a block held over the canvas: its edge dashed in the ring tone
        if dropping {
            let r = rect.shrink(1.0);
            let outline = [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ];
            painter.extend(Shape::dashed_line(
                &outline,
                Stroke::new(2.0, t.color("ring")),
                4.0,
                3.0,
            ));
        }

        // an empty canvas says so, and offers its action
        let mut empty_action = false;
        let mut empty_action_rect = None;
        if empty {
            let words = galley(ui, TextRole::Body.rich(&t, &self.words.empty));
            let tall = t.density("control-sm");
            let action_height = if self.empty_action.is_some() {
                tall + EMPTY_GAP
            } else {
                0.0
            };
            let block = words.size().y + action_height;
            let top = rect.center().y - block / 2.0;
            let words_at = pos2(rect.center().x - words.size().x / 2.0, top);
            let said = Rect::from_min_size(words_at, words.size());
            painter.galley(words_at, words, t.color("muted-foreground"));
            ui.interact(said, self.id.with("empty"), Sense::hover())
                .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &self.words.empty));
            if let Some((icon, label)) = &self.empty_action {
                let mut button = Button::new(label).size(ButtonSize::Sm);
                if let Some(icon) = icon {
                    button = button.icon(*icon);
                }
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_center_size(
                            pos2(rect.center().x, said.bottom() + EMPTY_GAP + tall / 2.0),
                            vec2(rect.width(), tall),
                        ))
                        .layout(egui::Layout::top_down(egui::Align::Center)),
                );
                let response = child.add(button);
                empty_action = response.clicked();
                empty_action_rect = Some(response.rect);
            }
        }

        // why a connection is refused, beside the pointer
        if let Some((p, reason)) = verdict {
            paint_verdict(ui, &painter, &t, rect, p, &reason);
        }

        // the zoom buttons in the corner, the app's after them
        let buttons = if self.controls && (!empty || !self.extra_controls.is_empty()) {
            GraphViewButtons::new(self.id)
                .orientation(ButtonsOrientation::Column)
                .words(self.words.view())
                .extra(self.extra_controls)
                .idle(empty)
                .hide_when_idle(true)
                .show_in_corner(ui, &painter, rect)
        } else {
            Default::default()
        };
        let mut step = |factor: Option<f32>| {
            match factor {
                Some(factor) => {
                    let next = (view.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
                    state.moved = Some(view.zoomed(next, rect.size() / 2.0));
                }
                None => state.moved = None,
            }
            ui.ctx().request_repaint();
        };
        if buttons.zoom_in {
            step(Some(ZOOM_STEP));
        }
        if buttons.zoom_out {
            step(Some(1.0 / ZOOM_STEP));
        }
        if buttons.fit {
            step(None);
        }

        // the toolbar over the top left
        let toolbar = match self.toolbar.clone() {
            Some(bar) => {
                let answer = bar.show_at(ui, &painter, rect.min + Vec2::splat(TOOLBAR_MARGIN), self.id, !empty);
                if answer.zoom_in {
                    step(Some(ZOOM_STEP));
                }
                if answer.zoom_out {
                    step(Some(1.0 / ZOOM_STEP));
                }
                if answer.fit {
                    step(None);
                }
                answer
            }
            None => GraphToolbarResponse::default(),
        };

        // the menu a right click opened
        if let Some(open) = state.menu.clone() {
            let asked = match &open.node {
                Some(node) => self.node_menu.and_then(|menu| menu(node)),
                None => self.pane_menu.and_then(|menu| menu(open.graph_at)),
            };
            match asked {
                Some(menu) => {
                    if let Some(item) = show_menu(ui, &mut state, &open, &menu, self.id) {
                        events.push(GraphEvent::MenuPick {
                            node: open.node.clone(),
                            item,
                        });
                    }
                }
                None => state.menu = None,
            }
        }

        let controls = GraphControls {
            id: self.id,
            rect,
            view,
            direction: self.direction,
            empty,
            boxes: self
                .nodes
                .iter()
                .map(|n| n.id.clone())
                .zip(boxes.iter().copied())
                .collect(),
            edges: drawn.into_iter().map(|d| (d.edge, d.route)).collect(),
        };
        ui.data_mut(|d| d.insert_temp(state_id, state));
        GraphResponse {
            response: ground,
            pressed,
            pane_pressed,
            events,
            toolbar,
            extra: buttons.extra,
            extra_rects: buttons.extra_rects,
            empty_action,
            empty_action_rect,
            controls,
        }
    }

    /// Open a node's menu at `p`, when it has one, and report the right click on an editor.
    #[allow(clippy::too_many_arguments)]
    fn open_node_menu(
        &self,
        state: &mut StageState,
        events: &mut Vec<GraphEvent>,
        node: &str,
        p: Pos2,
        graph_at: Pos2,
        editing: &crate::canvas::GraphEditing<'_>,
        editable: bool,
    ) {
        if self.node_menu.and_then(|menu| menu(node)).is_some() {
            state.menu = Some(OpenMenu {
                node: Some(node.to_owned()),
                at: p,
                graph_at,
                opened: false,
            });
        }
        if editable && editing.node_menu {
            events.push(GraphEvent::NodeMenu {
                node: node.to_owned(),
                at: GraphPointer {
                    screen: p,
                    at: graph_at,
                },
            });
        }
    }

    /// A connection let go: joined to the node under the pointer when allowed, refused when not,
    /// or let go on the ground.
    fn end_connection(
        &self,
        events: &mut Vec<GraphEvent>,
        connecting: &Connecting,
        end: Option<Pos2>,
        shown: &[Rect],
        editing: &crate::canvas::GraphEditing<'_>,
        to_graph: impl Fn(Pos2) -> Pos2,
    ) {
        let Some(p) = end else {
            return;
        };
        let Some(k) = node_under(shown, p) else {
            if connecting.outward && editing.drop {
                events.push(GraphEvent::Drop {
                    source: connecting.from.clone(),
                    at: GraphPointer {
                        screen: p,
                        at: to_graph(p),
                    },
                });
            }
            return;
        };
        let over = &self.nodes[k].id;
        if *over == connecting.from {
            return;
        }
        let (source, target) = if connecting.outward {
            (connecting.from.clone(), over.clone())
        } else {
            (over.clone(), connecting.from.clone())
        };
        match editing.refusal(&source, &target) {
            Some(reason) => events.push(GraphEvent::Refuse { source, target, reason }),
            None => events.push(GraphEvent::Connect { source, target }),
        }
    }
}

/// The node whose box, or a port at its edge, is under a point.
fn node_under(shown: &[Rect], p: Pos2) -> Option<usize> {
    shown.iter().rposition(|r| r.expand(PORT / 2.0).contains(p))
}

/// How far a point is from a line through some points.
fn distance_to(line: &[Pos2], p: Pos2) -> f32 {
    line.windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let ab = b - a;
            let u = if ab.length_sq() == 0.0 {
                0.0
            } else {
                ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
            };
            (a + ab * u - p).length()
        })
        .fold(f32::INFINITY, f32::min)
}

/// What a node's issues say, one to a line.
fn spoken(issues: &[NodeIssue]) -> String {
    issues.iter().map(|i| i.text.as_str()).collect::<Vec<_>>().join("\n")
}

/// The stage's dots, moved and zoomed with the view, as React Flow's dotted background.
fn paint_dots(painter: &egui::Painter, t: &Tokens, rect: Rect, view: GraphView) {
    let gap = DOT_GAP * view.zoom;
    if gap < 4.0 {
        return;
    }
    let radius = DOT_SIZE * view.zoom / 2.0;
    let color = t.color("panel-dot");
    // the pattern's first dot, from the view's corner, less half a gap, plus the dot's radius
    let start = |offset: f32| (offset - gap / 2.0 + radius).rem_euclid(gap);
    let mut y = rect.top() + start(view.y);
    while y < rect.bottom() {
        let mut x = rect.left() + start(view.x);
        while x < rect.right() {
            painter.circle_filled(pos2(x, y), radius, color);
            x += gap;
        }
        y += gap;
    }
}

/// A port: a quiet square on the box's edge, lit in the ring tone under the pointer and while a
/// connection is drawn.
fn paint_port(painter: &egui::Painter, t: &Tokens, spot: Rect, hovered: bool, connecting: bool, zoom: f32) {
    let (fill, edge) = if hovered {
        (t.color("ring"), t.color("ring"))
    } else if connecting {
        (mix(t.color("ring"), t.color("card"), 0.45), t.color("ring"))
    } else {
        (t.color("card"), t.color("border-strong"))
    };
    let corners = CornerRadius::same((t.length("radius-chip") * zoom).round() as u8);
    painter.rect(
        spot,
        corners,
        fill,
        Stroke::new(zoom.max(0.5), edge),
        StrokeKind::Inside,
    );
}

/// The connection being drawn: a dashed curve in the ring tone from its port to the pointer, as
/// React Flow's connection line bends out of a port's side.
fn paint_connection(
    painter: &egui::Painter,
    t: &Tokens,
    from: Pos2,
    to: Pos2,
    direction: GraphDirection,
    outward: bool,
    zoom: f32,
) {
    // the side the line leaves by, and the side it arrives on, as unit steps
    let leave = match (direction, outward) {
        (GraphDirection::Down, true) => vec2(0.0, 1.0),
        (GraphDirection::Down, false) => vec2(0.0, -1.0),
        (GraphDirection::Right, true) => vec2(1.0, 0.0),
        (GraphDirection::Right, false) => vec2(-1.0, 0.0),
    };
    let offset = |distance: f32| {
        if distance >= 0.0 {
            0.5 * distance
        } else {
            0.25 * 25.0 * (-distance).sqrt()
        }
    };
    let along = (to - from).dot(leave);
    let c1 = from + leave * offset(along);
    let c2 = to - leave * offset(along);
    let curve: Vec<Pos2> = (0..=24)
        .map(|k| {
            let u = k as f32 / 24.0;
            let v = 1.0 - u;
            (from.to_vec2() * v * v * v
                + c1.to_vec2() * 3.0 * v * v * u
                + c2.to_vec2() * 3.0 * v * u * u
                + to.to_vec2() * u * u * u)
                .to_pos2()
        })
        .collect();
    painter.extend(Shape::dashed_line(
        &curve,
        Stroke::new(2.0 * zoom, t.color("ring")),
        5.0 * zoom,
        4.0 * zoom,
    ));
}

/// Why a connection is refused, in a small box beside the pointer.
fn paint_verdict(ui: &Ui, painter: &egui::Painter, t: &Tokens, frame: Rect, p: Pos2, reason: &str) {
    let mut job = egui::text::LayoutJob::default();
    TextRole::Help
        .rich(t, reason)
        .color(t.color("popover-foreground"))
        .append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::Min);
    let pad = vec2(8.0, 6.0);
    let glyph = 14.0;
    let gap = 6.0;
    job.wrap.max_width = VERDICT_WIDTH - 2.0 * pad.x - glyph - gap;
    let words = painter.layout_job(job);
    let size = vec2(
        2.0 * pad.x + glyph + gap + words.size().x,
        2.0 * pad.y + words.size().y.max(glyph),
    );
    let mut min = p + Vec2::splat(VERDICT_OFFSET);
    min.x = min.x.min(frame.right() - size.x - 4.0);
    min.y = min.y.min(frame.bottom() - size.y - 4.0);
    let spot = Rect::from_min_size(min, size);
    let corners = t.radius("control");
    for shadow in t.shadows("elevation-pop") {
        painter.add(shadow.as_shape(spot, corners));
    }
    painter.rect(
        spot,
        corners,
        t.color("popover"),
        Stroke::new(1.0, t.color_at("critical", 0.6)),
        StrokeKind::Inside,
    );
    sokkel_egui::icons::paint(
        painter,
        pos2(
            spot.left() + pad.x + glyph / 2.0,
            spot.top() + pad.y + glyph / 2.0 + 1.0,
        ),
        Icon::CircleAlert,
        glyph,
        t.color("critical-ink"),
    );
    painter.galley(
        pos2(spot.left() + pad.x + glyph + gap, spot.top() + pad.y),
        words,
        t.color("popover-foreground"),
    );
}

/// The menu open at a point, as a dropdown menu from an empty trigger there; the id of the row
/// chosen, or its label without one.
fn show_menu(ui: &Ui, state: &mut StageState, open: &OpenMenu, menu: &GraphMenu, id: egui::Id) -> Option<String> {
    let trigger: Response = ui.interact(
        Rect::from_min_size(open.at, Vec2::ZERO),
        id.with("menu-at"),
        Sense::hover(),
    );
    let popup = trigger.id.with("menu");
    if !open.opened {
        egui::Popup::open_id(ui.ctx(), popup);
        if let Some(menu) = &mut state.menu {
            menu.opened = true;
        }
        ui.ctx().request_repaint();
    } else if !egui::Popup::is_id_open(ui.ctx(), popup) {
        state.menu = None;
        return None;
    }
    let mut items = Vec::new();
    if let Some(head) = &menu.head {
        items.push(MenuItem::Label(head.clone()));
    }
    items.extend(menu.items.iter().cloned());
    let picked = DropdownMenu::new(&items).width(180.0).show(&trigger)?;
    state.menu = None;
    match &items[picked] {
        MenuItem::Action { id, label, .. } => Some(id.clone().unwrap_or_else(|| label.clone())),
        _ => None,
    }
}
