//! A network of things and the links between them, after sokkel-ui-graph's `NetworkGraph` with
//! `layout="force"`: each thing a dot in its kind color, larger the more links it has, its name
//! under it, and each link a straight line from dot to dot, on a stage the view fits the whole of.

use std::hash::{Hash, Hasher};

use egui::{Align2, Color32, CursorIcon, Id, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, pos2, vec2};
use sokkel_egui::{Icon, TextRole, Tokens, tokens};

use crate::canvas::{MAX_ZOOM, MIN_ZOOM, ZOOM_STEP, mix};
use crate::force::{CHANGED_HEAT, DRAG_HEAT, FORCE_MOST_TICKS, FORCE_WARM_TICKS, ForceBody, ForceSimulation};

/// The width a node's name may take under its dot, the height of its line, and the gap between.
const NAME_WIDTH: f64 = 120.0;
const NAME_LINE: f64 = 15.0;
const NAME_GAP: f64 = 4.0;
/// How far a fit zooms out, so a network is seen whole; its names are small marks anyway.
const NETWORK_FIT_ZOOM: f32 = 0.35;
/// Room left around a fitted network, as a share of the stage.
const FIT_PADDING: f32 = 0.12;
/// Where a drawing too large for the stage starts, from its edge.
const OVERFLOW_MARGIN: f32 = 24.0;
/// The ring round the current node: a gap in the stage's color, then the ring.
const RING_GAP: f32 = 2.0;
const RING: f32 = 2.0;
/// The stage's dots: their spacing and size, as React Flow's dotted background.
const DOT_GAP: f32 = 18.0;
const DOT_SIZE: f32 = 1.0;
/// One zoom button's side, and the cluster's margin from the stage's corner.
const BUTTON: f32 = 28.0;
const BUTTON_MARGIN: f32 = 8.0;
/// The opacity of a node another one's neighbors leave out, a faint node's dot, and a link
/// the pointed-at node does not touch.
const DIM: f32 = 0.25;
const FAINT: f32 = 0.5;
const DIM_LINK: f32 = 0.15;
/// How much a wheel's pixel zooms, as d3-zoom counts it.
const WHEEL_ZOOM: f32 = 0.002;

/// A dot's size by how many links it has: more links, a larger dot.
pub fn dot_size(links: usize) -> u32 {
    (8.0 + ((links as f64).sqrt() * 4.0).min(14.0)).round() as u32
}

/// The categorical colors a node can wear, each a theme's `kind-*` token, in the order of
/// sokkel-ui's `KIND_COLORS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KindColor {
    Blue,
    Violet,
    Green,
    Pink,
    Teal,
    Jade,
}

impl KindColor {
    /// The colors in `KIND_COLORS`' order, the order an app hands them out in.
    pub const ALL: [KindColor; 6] = [
        Self::Blue,
        Self::Violet,
        Self::Green,
        Self::Pink,
        Self::Teal,
        Self::Jade,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Violet => "violet",
            Self::Green => "green",
            Self::Pink => "pink",
            Self::Teal => "teal",
            Self::Jade => "jade",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// The color a name wears, picked steadily from its letters, as sokkel-ui's `kindOf`.
    pub fn of(name: &str) -> Self {
        Self::from_name(sokkel_egui::kind_of(name)).unwrap_or(Self::Blue)
    }

    /// The color as a solid fill, as `bg-kind-*`.
    pub fn fill(self, t: &Tokens) -> Color32 {
        t.color(&format!("kind-{}", self.name()))
    }
}

/// One thing in a network: a note, a person, a service.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkNode {
    pub id: String,
    pub label: String,
    /// What the node belongs to, such as its folder, which picks its color; one color for all
    /// without it.
    pub group: Option<String>,
    /// The node's color, over the one its group picks.
    pub color: Option<KindColor>,
    /// Drawn quieter, such as a note linked to that does not exist yet.
    pub faint: bool,
}

impl NetworkNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            group: None,
            color: None,
            faint: false,
        }
    }

    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn color(mut self, color: Option<KindColor>) -> Self {
        self.color = color;
        self
    }

    pub fn faint(mut self, faint: bool) -> Self {
        self.faint = faint;
        self
    }

    /// The dot's fill: its color, else its group's, else the muted ink.
    fn fill(&self, t: &Tokens) -> Color32 {
        match (&self.color, &self.group) {
            (Some(color), _) => color.fill(t),
            (None, Some(group)) => KindColor::of(group).fill(t),
            (None, None) => t.color("muted-foreground"),
        }
    }
}

/// One link between two nodes; a network's links have no direction to read them in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkLink {
    pub source: String,
    pub target: String,
}

impl NetworkLink {
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
        }
    }
}

/// The words a network says.
#[derive(Clone, Debug)]
pub struct NetworkWords {
    /// What the stage is, for a screen reader.
    pub label: String,
    /// What a network with no node says.
    pub empty: String,
    pub zoom_in: String,
    pub zoom_out: String,
    pub fit: String,
}

impl Default for NetworkWords {
    fn default() -> Self {
        Self {
            label: "Graph".into(),
            empty: "Nothing to show".into(),
            zoom_in: "Zoom in".into(),
            zoom_out: "Zoom out".into(),
            fit: "Fit the graph".into(),
        }
    }
}

/// What a network answers with: its stage, the node pressed this frame and the node pointed at.
pub struct NetworkResponse {
    pub response: Response,
    pub pressed: Option<String>,
    pub hovered: Option<String>,
}

/// Where the drawing stands on the stage: moved by `x` and `y` from the stage's corner, and zoomed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct View {
    x: f32,
    y: f32,
    zoom: f32,
}

impl View {
    fn at(self, origin: Pos2, x: f64, y: f64) -> Pos2 {
        origin + vec2(self.x + x as f32 * self.zoom, self.y + y as f32 * self.zoom)
    }

    /// The view zoomed to `zoom` about `about`, a point on the stage from its corner.
    fn zoomed(self, zoom: f32, about: Vec2) -> Self {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let scale = zoom / self.zoom;
        Self {
            x: about.x - (about.x - self.x) * scale,
            y: about.y - (about.y - self.y) * scale,
            zoom,
        }
    }
}

/// The live layout, kept between frames: the simulation for the network it was made for, the
/// node held, and the view once the reader moved it.
#[derive(Clone, Default)]
struct State {
    signature: u64,
    simulation: Option<ForceSimulation>,
    held: Option<String>,
    /// Where the held dot's middle stands from the pointer, so it keeps the place it was taken at.
    grip: Vec2,
    moved: Option<View>,
}

/// Things and the links between them with no direction to read them in, such as notes that link
/// to each other, as sokkel-ui-graph's `NetworkGraph` with `layout="force"`.
///
/// Each thing is a dot in its kind color, larger the more links it has, with its name under it,
/// and each link a straight line between two dots. A force simulation (d3-force's, ported to the
/// bit) lays it out: linked dots pull together, every dot pushes the others away, and the network
/// settles, its first 150 ticks at once and the rest one a frame (all at once with motion off). The
/// view fits the whole of it until the reader moves it.
///
/// Pointing at a dot lights it, its links and its neighbors, and fades the rest. A dot can be
/// dragged, its neighbors following and the rest making room, and settles back among them once let
/// go; dragging one stops the view's fit, as a pan does. `current` rings the node the reader
/// stands on and sets its name strong, and a press on a dot answers its id. The wheel zooms the
/// stage, a drag on its ground pans it, and the buttons at its foot zoom in, out, and back to the fit.
pub struct NetworkGraph<'a> {
    id: Id,
    nodes: &'a [NetworkNode],
    links: &'a [NetworkLink],
    current: Option<&'a str>,
    selected: Option<&'a str>,
    size: Option<Vec2>,
    words: NetworkWords,
}

impl<'a> NetworkGraph<'a> {
    pub fn new(id: impl Hash + std::fmt::Debug, nodes: &'a [NetworkNode], links: &'a [NetworkLink]) -> Self {
        Self {
            id: Id::unique(("sokkel-network", id)),
            nodes,
            links,
            current: None,
            selected: None,
            size: None,
            words: NetworkWords::default(),
        }
    }

    /// The node the reader stands on, such as the note open now: ringed, its name set strong.
    pub fn current(mut self, current: Option<&'a str>) -> Self {
        self.current = current;
        self
    }

    /// The node marked as chosen: ringed as the current one is.
    pub fn selected(mut self, selected: Option<&'a str>) -> Self {
        self.selected = selected;
        self
    }

    /// The stage's size; all the room the `Ui` has when left out.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    pub fn words(mut self, words: NetworkWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> NetworkResponse {
        let t = tokens(ui.ctx());
        let size = self.size.unwrap_or_else(|| ui.available_size());
        let (rect, ground) = ui.allocate_exact_size(size, Sense::click_and_drag());
        ground.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Unknown, true, &self.words.label));
        let painter = ui.painter_at(rect);
        let corners = t.radius("surface");
        painter.rect_filled(rect, corners, t.color("canvas"));

        // the links kept: between two nodes drawn, never a node to itself
        let index = |id: &str| self.nodes.iter().position(|n| n.id == id);
        let kept: Vec<(usize, usize)> = self
            .links
            .iter()
            .filter_map(|l| Some((index(&l.source)?, index(&l.target)?)))
            .filter(|(s, t)| s != t)
            .collect();
        let mut degree = vec![0usize; self.nodes.len()];
        for &(s, t) in &kept {
            degree[s] += 1;
            degree[t] += 1;
        }
        let dots: Vec<f64> = degree.iter().map(|d| dot_size(*d) as f64).collect();

        let state_id = self.id.with("state");
        let mut state = ui.data_mut(|d| d.remove_temp::<State>(state_id)).unwrap_or_default();
        let still = ui.style().animation_time <= 0.0;
        let signature = self.signature(&dots, &kept);
        if state.simulation.is_none() || state.signature != signature {
            state.simulation = Some(self.simulation(state.simulation.as_ref(), &dots, &kept, still));
            state.signature = signature;
            state.held = None;
        } else if let Some(simulation) = &mut state.simulation
            && (state.held.is_some() || !simulation.settled())
        {
            // one tick a frame, as d3's timer ticks, while it cools or a dot is held
            simulation.tick();
        }
        let simulation = state.simulation.as_mut().expect("made above");

        if self.nodes.is_empty() {
            self.dots(
                &painter,
                &t,
                rect,
                View {
                    x: 0.0,
                    y: 0.0,
                    zoom: 1.0,
                },
            );
            let said = painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                &self.words.empty,
                TextRole::Body.font(&t),
                t.color("muted-foreground"),
            );
            ui.interact(said, self.id.with("empty"), Sense::hover())
                .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &self.words.empty));
            ui.data_mut(|d| d.insert_temp(state_id, state));
            return NetworkResponse {
                response: ground,
                pressed: None,
                hovered: None,
            };
        }

        // each node's box: its dot's middle at the body, its name under the dot
        let boxes = |simulation: &ForceSimulation| -> Vec<[f64; 4]> {
            simulation
                .bodies()
                .iter()
                .zip(&dots)
                .map(|(body, dot)| {
                    let (x, y) = (body.x - NAME_WIDTH / 2.0, body.y - dot / 2.0);
                    [x, y, NAME_WIDTH, dot + NAME_GAP + NAME_LINE]
                })
                .collect()
        };
        let mut view = state.moved.unwrap_or_else(|| fit(&boxes(simulation), rect.size()));

        // the reader moves the view: a drag on the ground pans, the wheel zooms about the pointer
        if ground.dragged() {
            view.x += ground.drag_delta().x;
            view.y += ground.drag_delta().y;
            state.moved = Some(view);
        }
        if ui.rect_contains_pointer(rect)
            && let Some(pointer) = ui.input(|i| i.pointer.hover_pos())
        {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let factor = 2f32.powf(scroll * WHEEL_ZOOM) * pinch;
            if factor != 1.0 {
                view = view.zoomed(view.zoom * factor, pointer - rect.min);
                state.moved = Some(view);
            }
        }

        // the nodes answer the pointer: a press chooses one, a drag holds it
        let mut pressed = None;
        let mut hovered = None;
        let shown = boxes(simulation);
        for (k, node) in self.nodes.iter().enumerate() {
            let [x, y, w, h] = shown[k];
            let spot = Rect::from_min_max(view.at(rect.min, x, y), view.at(rect.min, x + w, y + h));
            let response = ui.interact(
                spot.intersect(rect),
                self.id.with(("node", &node.id)),
                Sense::click_and_drag(),
            );
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::Role::Button,
                    true,
                    self.current == Some(node.id.as_str()) || self.selected == Some(node.id.as_str()),
                    &node.label,
                )
            });
            if response.hovered() || response.dragged() {
                hovered = Some(k);
                ui.ctx().set_cursor_icon(if response.dragged() {
                    CursorIcon::Grabbing
                } else {
                    CursorIcon::Grab
                });
            }
            if response.drag_started() {
                state.held = Some(node.id.clone());
                state.moved = Some(view);
                let body = &simulation.bodies()[k];
                let press = ui.input(|i| i.pointer.press_origin()).unwrap_or(spot.center());
                state.grip = view.at(rect.min, body.x, body.y) - press;
                simulation.set_alpha_target(DRAG_HEAT);
            }
            if response.dragged()
                && state.held.as_deref() == Some(node.id.as_str())
                && let Some(pointer) = response.interact_pointer_pos()
            {
                // the pointer's place in the network's own coordinates
                let at = (pointer + state.grip - rect.min - vec2(view.x, view.y)) / view.zoom;
                let body = &mut simulation.bodies_mut()[k];
                body.fx = Some(at.x as f64);
                body.fy = Some(at.y as f64);
                body.x = at.x as f64;
                body.y = at.y as f64;
            }
            if response.drag_stopped() && state.held.as_deref() == Some(node.id.as_str()) {
                state.held = None;
                let body = &mut simulation.bodies_mut()[k];
                body.fx = None;
                body.fy = None;
                simulation.set_alpha_target(0.0);
            }
            if response.clicked() {
                pressed = Some(node.id.clone());
            }
        }
        if state.held.is_some() || !simulation.settled() {
            ui.ctx().request_repaint();
        }

        // what the pointer lights: the node pointed at and its neighbors
        let lit: Option<Vec<bool>> = hovered.map(|h| {
            let mut near = vec![false; self.nodes.len()];
            near[h] = true;
            for &(s, t) in &kept {
                if s == h {
                    near[t] = true;
                }
                if t == h {
                    near[s] = true;
                }
            }
            near
        });

        self.dots(&painter, &t, rect, view);
        let bodies = simulation.bodies();
        let middle = |k: usize| view.at(rect.min, bodies[k].x, bodies[k].y);
        let edge = mix(t.color("graph-edge"), t.color("muted-foreground"), 0.7);
        for &(s, e) in &kept {
            let stroke = match hovered {
                Some(h) if s == h || e == h => Stroke::new(1.5 * view.zoom, t.color("foreground")),
                Some(_) => Stroke::new(view.zoom, edge.gamma_multiply(DIM_LINK)),
                None => Stroke::new(view.zoom, edge),
            };
            painter.line_segment([middle(s), middle(e)], stroke);
        }

        let title = t.text("graph-title");
        for (k, node) in self.nodes.iter().enumerate() {
            let shade = if lit.as_ref().is_some_and(|near| !near[k]) {
                DIM
            } else {
                1.0
            };
            let center = middle(k);
            let radius = dots[k] as f32 / 2.0 * view.zoom;
            if self.current == Some(node.id.as_str()) || self.selected == Some(node.id.as_str()) {
                painter.circle_filled(
                    center,
                    radius + (RING_GAP + RING) * view.zoom,
                    t.color("ring").gamma_multiply(shade),
                );
                painter.circle_filled(center, radius + RING_GAP * view.zoom, t.color("canvas"));
            }
            let fill = node
                .fill(&t)
                .gamma_multiply(if node.faint { FAINT } else { 1.0 } * shade);
            painter.circle_filled(center, radius, fill);

            let ink = if node.faint { "muted-foreground" } else { "foreground" };
            let strong = self.current == Some(node.id.as_str());
            let rich = TextRole::GraphTitle
                .rich(&t, &node.label)
                .size(title.size * view.zoom)
                .line_height(Some(title.line_height * view.zoom))
                .variation("wght", if strong { 600.0 } else { title.weight })
                .color(t.color(ink).gamma_multiply(shade));
            let mut job = egui::text::LayoutJob::default();
            rich.append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::Center);
            job.wrap = egui::text::TextWrapping::truncate_at_width(NAME_WIDTH as f32 * view.zoom);
            let galley = painter.layout_job(job);
            let top = center.y + radius + NAME_GAP as f32 * view.zoom;
            painter.galley(
                pos2(center.x - galley.size().x / 2.0, top),
                galley,
                t.color(ink).gamma_multiply(shade),
            );
        }

        if let Some(moved) = self.buttons(ui, &painter, &t, rect, view) {
            state.moved = moved;
            ui.ctx().request_repaint();
        }
        ui.data_mut(|d| d.insert_temp(state_id, state));

        NetworkResponse {
            response: ground,
            pressed,
            hovered: hovered.map(|k| self.nodes[k].id.clone()),
        }
    }

    /// What the placement is a function of: the nodes, their dots' sizes and the links kept.
    fn signature(&self, dots: &[f64], kept: &[(usize, usize)]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for (node, dot) in self.nodes.iter().zip(dots) {
            node.id.hash(&mut hasher);
            dot.to_bits().hash(&mut hasher);
        }
        kept.hash(&mut hasher);
        hasher.finish()
    }

    /// A simulation for the network as it is now, from where the nodes it had stood: a network
    /// that changed starts there, cooler, and moves only where it must; a new one starts warm.
    fn simulation(
        &self,
        before: Option<&ForceSimulation>,
        dots: &[f64],
        kept: &[(usize, usize)],
        still: bool,
    ) -> ForceSimulation {
        let bodies: Vec<ForceBody> = self
            .nodes
            .iter()
            .zip(dots)
            .map(|(node, dot)| {
                // the room a dot keeps the others from its middle, with room for the name under it
                let body = ForceBody::new(node.id.clone(), dot / 2.0 + 38.0);
                match before.and_then(|b| b.bodies().iter().find(|was| was.id == node.id)) {
                    Some(was) => body.at(was.x, was.y),
                    None => body,
                }
            })
            .collect();
        let had = bodies.iter().any(|b| !b.x.is_nan());
        let links: Vec<(&str, &str)> = kept
            .iter()
            .map(|&(s, t)| (self.nodes[s].id.as_str(), self.nodes[t].id.as_str()))
            .collect();
        let mut simulation = ForceSimulation::new(bodies, &links);
        if had {
            simulation.set_alpha(CHANGED_HEAT);
        }
        simulation.settle(if still {
            FORCE_MOST_TICKS
        } else if had {
            0
        } else {
            FORCE_WARM_TICKS
        });
        simulation
    }

    /// The stage's dots, moved and zoomed with the view, as React Flow's dotted background.
    fn dots(&self, painter: &egui::Painter, t: &Tokens, rect: Rect, view: View) {
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

    /// The zoom buttons, one over another at the stage's bottom right: the view a press leaves,
    /// `Some(None)` for the fit again.
    fn buttons(&self, ui: &Ui, painter: &egui::Painter, t: &Tokens, rect: Rect, view: View) -> Option<Option<View>> {
        let group = Rect::from_min_size(
            rect.right_bottom() - vec2(BUTTON + 2.0 + BUTTON_MARGIN, 3.0 * BUTTON + 2.0 + BUTTON_MARGIN),
            vec2(BUTTON + 2.0, 3.0 * BUTTON + 2.0),
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
            egui::StrokeKind::Inside,
        );
        let buttons = [
            (Icon::ZoomIn, &self.words.zoom_in),
            (Icon::ZoomOut, &self.words.zoom_out),
            (Icon::Scan, &self.words.fit),
        ];
        let mut moved = None;
        for (k, (icon, word)) in buttons.into_iter().enumerate() {
            let spot = Rect::from_min_size(group.min + vec2(1.0, 1.0 + k as f32 * BUTTON), Vec2::splat(BUTTON));
            let response = ui.interact(spot, self.id.with(("button", k)), Sense::click());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word.as_str()));
            if k > 0 {
                painter.hline(spot.x_range(), spot.top() - 0.5, Stroke::new(1.0, t.color("border")));
            }
            let ink = if response.hovered() {
                painter.rect_filled(spot, 0.0, t.color("hover"));
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            sokkel_egui::icons::paint(painter, spot.center(), icon, 14.0, ink);
            if response.on_hover_text(word.as_str()).clicked() {
                let center = rect.size() / 2.0;
                moved = Some(match k {
                    0 => Some(view.zoomed(view.zoom * ZOOM_STEP, center)),
                    1 => Some(view.zoomed(view.zoom / ZOOM_STEP, center)),
                    _ => None,
                });
            }
        }
        moved
    }
}

/// The view that fits `boxes` on a stage this size, as sokkel-ui-graph's `fitGraph` on a stage:
/// React Flow's `getViewportForBounds` with 12% padding, zoomed no further out than
/// `NETWORK_FIT_ZOOM` and no further in than 1, a drawing still too large starting at the margin.
fn fit(boxes: &[[f64; 4]], stage: Vec2) -> View {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &[x, y, w, h] in boxes {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x + w);
        y1 = y1.max(y + h);
    }
    let (bx, by, bw, bh) = (x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32);
    let (width, height) = (stage.x, stage.y);
    let pad = |side: f32| ((side - side / (1.0 + FIT_PADDING)) * 0.5).floor();
    let (px, py) = (pad(width), pad(height));
    let zoom = ((width - 2.0 * px) / bw)
        .min((height - 2.0 * py) / bh)
        .clamp(NETWORK_FIT_ZOOM, 1.0);
    let mut x = width / 2.0 - (bx + bw / 2.0) * zoom;
    let mut y = height / 2.0 - (by + bh / 2.0) * zoom;
    // the paddings the centered view leaves, made up where one is short
    let left = (bx * zoom + x).floor();
    let top = (by * zoom + y).floor();
    let right = (width - ((bx + bw) * zoom + x)).floor();
    let bottom = (height - ((by + bh) * zoom + y)).floor();
    x = x - (left - px).min(0.0) + (right - px).min(0.0);
    y = y - (top - py).min(0.0) + (bottom - py).min(0.0);
    if bw * zoom > width {
        x = OVERFLOW_MARGIN - bx * zoom;
    }
    if bh * zoom > height {
        y = OVERFLOW_MARGIN - by * zoom;
    }
    View { x, y, zoom }
}
