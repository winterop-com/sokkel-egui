//! Art for a sign-in screen's brand pane, after sokkel-ui's `SignInArt`: `FlowLines`, a small
//! graph of nodes joined by curved lines with one path lit and flowing, and `LevelBars`, a row of
//! bars breathing. Both are drawn from a seed, the same every time, in the same numbers as the
//! web's, so a seed draws the same picture on both sides.

use egui::epaint::CubicBezierShape;
use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Widget, pos2, vec2};

use crate::tokens;

/// A stream of numbers in [0, 1) that is the same for the same seed, so a drawing never changes
/// between frames: the web's `seeded`, bit for bit (a seed is taken modulo 2^32, 0 as 1).
pub fn seeded(seed: i64) -> impl FnMut() -> f64 {
    let mut state = match seed as u32 {
        0 => 1,
        s => s,
    };
    move || {
        state = state.wrapping_add(0x6d2b_79f5);
        let mut mixed = state;
        mixed = (mixed ^ (mixed >> 15)).wrapping_mul(mixed | 1);
        mixed ^= mixed.wrapping_add((mixed ^ (mixed >> 7)).wrapping_mul(mixed | 61));
        f64::from(mixed ^ (mixed >> 14)) / 4_294_967_296.0
    }
}

/// The heights, 0 to 1, of a row of level bars: a few soft peaks with some grain, the tallest at 1.
pub fn level_heights(count: usize, seed: i64) -> Vec<f64> {
    let mut random = seeded(seed);
    let n = count as f64;
    let peaks: Vec<(f64, f64, f64)> = (0..3)
        .map(|_| {
            let at = random() * n;
            let width = 3.0 + random() * (n / 8.0);
            let height = 0.45 + random() * 0.55;
            (at, width, height)
        })
        .collect();
    let raw: Vec<f64> = (0..count)
        .map(|index| {
            let sum = peaks.iter().fold(0.0, |sum, (at, width, height)| {
                sum + height * (-((index as f64 - at) / width).powi(2)).exp()
            });
            0.08 + sum + random() * 0.14
        })
        .collect();
    let top = raw.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    raw.iter().map(|value| (value / top).max(0.05)).collect()
}

/// A node of a flow graph: its top-left corner in the art's 720 by 560 box, and its column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowNode {
    pub x: f64,
    pub y: f64,
    pub column: usize,
}

/// A line from one node to one in the next column; `lit` on the one path through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowEdge {
    pub from: usize,
    pub to: usize,
    pub lit: bool,
}

/// The art's box, as the web's `viewBox`, and a card node's size in it.
const WIDE: f64 = 720.0;
const HIGH: f64 = 560.0;
const NODE_W: f64 = 72.0;
const NODE_H: f64 = 36.0;

/// A small graph of nodes in columns, each joined to its nearest neighbors on either side, with
/// one lit path from the first to the last: the web's `flowGraph`, the same nodes at the same
/// positions for the same seed. `columns` is how many columns (at least 2), `rows` how few and how
/// many nodes a middle column holds.
pub fn flow_graph(seed: i64, columns: usize, rows: (usize, usize)) -> (Vec<FlowNode>, Vec<FlowEdge>) {
    let count_columns = columns.max(2);
    let fewest = rows.0.min(rows.1).max(1);
    let most = fewest.max(rows.0.max(rows.1));
    let mut random = seeded(seed);
    let step = (WIDE - NODE_W - 16.0) / (count_columns - 1) as f64;
    let mut nodes: Vec<FlowNode> = Vec::new();
    let mut by_column: Vec<Vec<usize>> = Vec::new();
    for column in 0..count_columns {
        let ends = column == 0 || column == count_columns - 1;
        let count = if ends {
            1
        } else {
            fewest + (random() * (most - fewest + 1) as f64).floor() as usize
        };
        let spacing = HIGH / (count + 1) as f64;
        by_column.push(Vec::new());
        for row in 0..count {
            let jitter = if ends { 0.0 } else { (random() - 0.5) * spacing * 0.4 };
            by_column[column].push(nodes.len());
            nodes.push(FlowNode {
                x: 8.0 + column as f64 * step,
                y: spacing * (row + 1) as f64 + jitter - NODE_H / 2.0,
                column,
            });
        }
    }
    let mut edges: Vec<FlowEdge> = Vec::new();
    let join = |edges: &mut Vec<FlowEdge>, from: usize, to: usize| {
        if !edges.iter().any(|e| e.from == from && e.to == to) {
            edges.push(FlowEdge { from, to, lit: false });
        }
    };
    let nearest = |nodes: &[FlowNode], y: f64, among: &[usize]| {
        among[1..].iter().fold(among[0], |best, &index| {
            if (nodes[index].y - y).abs() < (nodes[best].y - y).abs() {
                index
            } else {
                best
            }
        })
    };
    for column in 0..count_columns - 1 {
        let here = &by_column[column];
        let next = &by_column[column + 1];
        for &index in next {
            join(&mut edges, nearest(&nodes, nodes[index].y, here), index);
        }
        for &index in here {
            join(&mut edges, index, nearest(&nodes, nodes[index].y, next));
        }
    }
    let mut at = by_column[0][0];
    while nodes[at].column < count_columns - 1 {
        let ways: Vec<usize> = (0..edges.len()).filter(|&i| edges[i].from == at).collect();
        let way = ways[(random() * ways.len() as f64).floor() as usize];
        edges[way].lit = true;
        at = edges[way].to;
    }
    (nodes, edges)
}

/// Whether motion is off for this app (`Motion::Off` sets egui's animation time to nothing).
pub(crate) fn still(ui: &Ui) -> bool {
    ui.style().animation_time <= 0.0
}

/// CSS's `cubic-bezier(x1, y1, x2, y2)` at time `t` in [0, 1]: `ease-out` is (0, 0, 0.58, 1),
/// `ease-in-out` (0.42, 0, 0.58, 1).
pub(crate) fn css_ease(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let curve = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
    };
    // the curve's parameter whose x is `t`, by bisection: x rises with the parameter
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..24 {
        let middle = (low + high) / 2.0;
        if curve(x1, x2, middle) < t {
            low = middle;
        } else {
            high = middle;
        }
    }
    curve(y1, y2, (low + high) / 2.0)
}

/// How a flow graph's nodes are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlowNodes {
    /// Small cards with a dot and a line.
    #[default]
    Card,
    /// Dots alone.
    Dot,
}

/// Art for a sign-in screen's brand pane, after sokkel-ui's `FlowLines`: a few nodes in columns
/// joined by curved lines, one path through them lit in `color` (the theme's primary by default)
/// and flowing toward the last node, which glows; the rest in the border's ink over faint rings.
/// `seed` picks the graph and `columns` and `rows` its shape, the same every time and the same as
/// the web's; `rings`, `lit`, `flow`, `glow` and `nodes` take parts away or change them. Motion
/// stops when the app asks for none. It fills the room it is given, its 720 by 560 box centered
/// and scaled to fit, and is cut at the room's edge.
#[must_use = "add it with `ui.add(...)`"]
pub struct FlowLines {
    seed: i64,
    columns: usize,
    rows: (usize, usize),
    rings: usize,
    lit: bool,
    flow: bool,
    glow: bool,
    nodes: FlowNodes,
    color: Option<Color32>,
}

impl Default for FlowLines {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowLines {
    pub fn new() -> Self {
        Self {
            seed: 3,
            columns: 6,
            rows: (2, 4),
            rings: 6,
            lit: true,
            flow: true,
            glow: true,
            nodes: FlowNodes::Card,
            color: None,
        }
    }

    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = seed;
        self
    }

    /// How many columns of nodes, the first and last holding one each.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns;
        self
    }

    /// How few and how many nodes a middle column holds.
    pub fn rows(mut self, fewest: usize, most: usize) -> Self {
        self.rows = (fewest, most);
        self
    }

    /// How many faint rings behind the graph; 0 for none.
    pub fn rings(mut self, rings: usize) -> Self {
        self.rings = rings;
        self
    }

    /// Whether one path from the first node to the last is lit.
    pub fn lit(mut self, lit: bool) -> Self {
        self.lit = lit;
        self
    }

    /// Whether the lit path flows toward the last node.
    pub fn flow(mut self, flow: bool) -> Self {
        self.flow = flow;
        self
    }

    /// Whether the last node glows in the ink.
    pub fn glow(mut self, glow: bool) -> Self {
        self.glow = glow;
        self
    }

    /// Nodes as small cards with a dot and a line, or as dots alone.
    pub fn nodes(mut self, nodes: FlowNodes) -> Self {
        self.nodes = nodes;
        self
    }

    /// The lit path's ink, the theme's primary when not given.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

/// Room for a piece of art: all it is given, or, where its height is open, its box's shape across
/// the width.
fn art_room(ui: &mut Ui, wide: f32, high: f32) -> (Rect, Response) {
    let room = ui.available_size();
    let height = if room.y.is_finite() && room.y > 0.0 {
        room.y
    } else {
        room.x * high / wide
    };
    ui.allocate_exact_size(vec2(room.x, height), Sense::hover())
}

/// Points along a cubic from `a` to `b`, flattened finely enough for a smooth line.
fn curve_points(a: Pos2, b: Pos2) -> Vec<Pos2> {
    let middle = (a.x + b.x) / 2.0;
    CubicBezierShape::from_points_stroke(
        [a, pos2(middle, a.y), pos2(middle, b.y), b],
        false,
        Color32::TRANSPARENT,
        Stroke::NONE,
    )
    .flatten(Some(0.1))
}

/// The dashes of a line along `points`, `dash` on and `dash` off, as SVG's `stroke-dasharray` with
/// `stroke-dashoffset` of `offset` (both in the line's own lengths): each dash its own polyline.
fn dashes(points: &[Pos2], dash: f32, offset: f32, stroke: Stroke) -> Vec<Shape> {
    let period = dash * 2.0;
    let mut shapes = Vec::new();
    let mut current: Vec<Pos2> = Vec::new();
    let on = |length: f32| (length + offset).rem_euclid(period) < dash;
    let mut walked = 0.0_f32;
    let finish = |current: &mut Vec<Pos2>, shapes: &mut Vec<Shape>| {
        if current.len() > 1 {
            shapes.push(Shape::line(std::mem::take(current), stroke));
        }
        current.clear();
    };
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let length = a.distance(b);
        if length <= 0.0 {
            continue;
        }
        // the places in this piece where a dash starts or ends
        let mut cuts = vec![0.0_f32];
        let start = (walked + offset).rem_euclid(dash);
        let mut next = dash - start;
        while next < length {
            cuts.push(next);
            next += dash;
        }
        cuts.push(length);
        for window in cuts.windows(2) {
            let (from, to) = (window[0], window[1]);
            let lit = on(walked + (from + to) / 2.0);
            let p = a + (b - a) * (from / length);
            let q = a + (b - a) * (to / length);
            if lit {
                if current.is_empty() {
                    current.push(p);
                }
                current.push(q);
            } else {
                finish(&mut current, &mut shapes);
            }
        }
        walked += length;
    }
    finish(&mut current, &mut shapes);
    shapes
}

impl Widget for FlowLines {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (rect, response) = art_room(ui, WIDE as f32, HIGH as f32);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let (nodes, edges) = flow_graph(self.seed, self.columns, self.rows);
        let edges: Vec<FlowEdge> = edges
            .into_iter()
            .map(|e| FlowEdge {
                lit: self.lit && e.lit,
                ..e
            })
            .collect();
        let lit_nodes: Vec<usize> = edges.iter().filter(|e| e.lit).flat_map(|e| [e.from, e.to]).collect();
        let last = nodes.len() - 1;
        let dot = self.nodes == FlowNodes::Dot;
        let ink = self.color.unwrap_or_else(|| t.color("primary"));
        // the box centered in the room and scaled to fit it, as `xMidYMid meet`
        let scale = (rect.width() / WIDE as f32).min(rect.height() / HIGH as f32);
        let origin = rect.center() - vec2(WIDE as f32, HIGH as f32) * scale / 2.0;
        let at = |x: f64, y: f64| origin + vec2(x as f32, y as f32) * scale;
        let len = |l: f64| l as f32 * scale;
        let radius = |r: f32| CornerRadius::same((r * scale).round().clamp(0.0, 255.0) as u8);
        let painter = ui.painter_at(rect);

        for ring in 0..self.rings {
            let r = 160.0 + ring as f64 * 100.0;
            painter.circle_stroke(
                at(WIDE - 60.0, HIGH / 2.0),
                len(r),
                Stroke::new(len(1.0), t.color_at("border", 0.4)),
            );
        }
        let ends = |edge: &FlowEdge| {
            let from = nodes[edge.from];
            let to = nodes[edge.to];
            let x1 = if dot { from.x + NODE_W / 2.0 } else { from.x + NODE_W };
            let x2 = if dot { to.x + NODE_W / 2.0 } else { to.x };
            (at(x1, from.y + NODE_H / 2.0), at(x2, to.y + NODE_H / 2.0))
        };
        for edge in edges.iter().filter(|e| !e.lit) {
            let (a, b) = ends(edge);
            painter.add(Shape::line(
                curve_points(a, b),
                Stroke::new(len(1.5), t.color("border")),
            ));
        }
        let moving = self.flow && !still(ui);
        // one dash period every 1.6 seconds, toward the last node, as the web's `line-flow`
        let offset = if moving {
            let cycle = (ui.input(|i| i.time) / 1.6).fract() as f32;
            -24.0 * cycle
        } else {
            0.0
        };
        for edge in edges.iter().filter(|e| e.lit) {
            let (a, b) = ends(edge);
            let points = curve_points(a, b);
            painter.add(Shape::line(
                points.clone(),
                Stroke::new(len(2.0), ink.gamma_multiply(0.45)),
            ));
            if self.flow {
                painter.extend(dashes(&points, len(12.0), offset * scale, Stroke::new(len(2.0), ink)));
            }
        }
        if moving {
            ui.ctx().request_repaint();
        }

        let node_radius = t.length("radius-node");
        for (index, node) in nodes.iter().enumerate() {
            let end = self.glow && index == last;
            let on = lit_nodes.contains(&index);
            let cy = node.y + NODE_H / 2.0;
            if dot {
                let center = at(node.x + NODE_W / 2.0, cy);
                if end {
                    painter.circle_filled(center, len(18.0), ink.gamma_multiply(0.15));
                }
                let edge = if end {
                    ink
                } else if on {
                    ink.gamma_multiply(0.6)
                } else {
                    t.color("border")
                };
                painter.circle(
                    center,
                    len(if end { 10.0 } else { 7.0 }),
                    if end { ink } else { t.color("card") },
                    Stroke::new(len(1.5), edge),
                );
                continue;
            }
            if end {
                painter.rect_filled(
                    Rect::from_min_size(
                        at(node.x - 8.0, node.y - 8.0),
                        vec2(len(NODE_W + 16.0), len(NODE_H + 16.0)),
                    ),
                    radius(node_radius + 8.0),
                    ink.gamma_multiply(0.15),
                );
            }
            let (fill, edge) = if end {
                (ink, ink)
            } else if on {
                (t.color("card"), ink.gamma_multiply(0.5))
            } else {
                (t.color("card"), t.color("border"))
            };
            painter.rect(
                Rect::from_min_size(at(node.x, node.y), vec2(len(NODE_W), len(NODE_H))),
                radius(node_radius),
                fill,
                Stroke::new(len(1.0), edge),
                StrokeKind::Middle,
            );
            let spot = if end {
                t.color("primary-foreground")
            } else if on {
                ink
            } else {
                t.color("muted-foreground")
            };
            painter.circle_filled(at(node.x + 14.0, cy), len(4.0), spot);
            let line = if end {
                t.color("primary-foreground")
            } else {
                t.color_at("muted-foreground", 0.4)
            };
            painter.rect_filled(
                Rect::from_min_size(
                    at(node.x + 26.0, cy - 1.5),
                    vec2(len(if end { 30.0 } else { 24.0 }), len(3.0)),
                ),
                0.0,
                line,
            );
        }
        response
    }
}

/// A level bar's width and the gap after it, the bars' height, and one breath, in seconds.
const BAR: f32 = 8.0;
const BAR_GAP: f32 = 4.0;
const BARS_HIGH: f32 = 560.0;
const BREATH: f64 = 3.2;

/// Art for a sign-in screen's brand pane, after sokkel-ui's `LevelBars`: a row of level bars, the
/// run around the tallest peak lit in `color` (the theme's primary by default) and the rest in the
/// muted ground, each breathing slowly at its own phase. `seed` picks the shape, the same every
/// time; motion stops when the app asks for none, or with `breathe(false)`, the bars resting at
/// full height. It fills the room it is given, standing on the room's foot.
#[must_use = "add it with `ui.add(...)`"]
pub struct LevelBars {
    seed: i64,
    count: usize,
    lit: usize,
    breathe: bool,
    color: Option<Color32>,
}

impl Default for LevelBars {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelBars {
    pub fn new() -> Self {
        Self {
            seed: 7,
            count: 56,
            lit: 9,
            breathe: true,
            color: None,
        }
    }

    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = seed;
        self
    }

    /// How many bars.
    pub fn count(mut self, count: usize) -> Self {
        self.count = count.max(1);
        self
    }

    /// How many bars around the tallest are lit; 0 for none.
    pub fn lit(mut self, lit: usize) -> Self {
        self.lit = lit;
        self
    }

    /// Whether the bars breathe.
    pub fn breathe(mut self, breathe: bool) -> Self {
        self.breathe = breathe;
        self
    }

    /// The lit bars' ink, the theme's primary when not given.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

impl Widget for LevelBars {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let wide = self.count as f32 * (BAR + BAR_GAP) - BAR_GAP;
        let (rect, response) = art_room(ui, wide, BARS_HIGH);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let heights = level_heights(self.count, self.seed);
        let peak = heights.iter().position(|h| *h == 1.0).unwrap_or(0) as i64;
        let count = self.count as i64;
        let lit = self.lit as i64;
        let start = (peak - lit / 2).min(count - lit).max(0);
        let mut random = seeded(self.seed + 1);
        let ink = self.color.unwrap_or_else(|| t.color("primary"));
        // the bars' box centered across the room and standing on its foot, as `xMidYMax meet`
        let scale = (rect.width() / wide).min(rect.height() / BARS_HIGH);
        let origin = pos2(rect.center().x - wide * scale / 2.0, rect.bottom() - BARS_HIGH * scale);
        let moving = self.breathe && !still(ui);
        let time = ui.input(|i| i.time);
        let painter = ui.painter_at(rect);
        for (index, height) in heights.iter().enumerate() {
            let delay = random() * BREATH;
            let on = (index as i64) >= start && (index as i64) < start + lit;
            // scaleY from 1 to 0.62 and back over a breath, eased in and out each way, from the foot
            let breath = if moving {
                // the web's delay is the two decimals it writes
                let delay = (delay * 100.0).round() / 100.0;
                let phase = ((time + delay) / BREATH).fract() as f32;
                let (from, to, along) = if phase < 0.5 {
                    (1.0, 0.62, phase * 2.0)
                } else {
                    (0.62, 1.0, phase * 2.0 - 1.0)
                };
                from + (to - from) * css_ease(0.42, 0.0, 0.58, 1.0, along)
            } else {
                1.0
            };
            let tall = BARS_HIGH * *height as f32 * breath;
            let x = index as f32 * (BAR + BAR_GAP);
            let bar = Rect::from_min_max(
                origin + vec2(x, BARS_HIGH - tall) * scale,
                origin + vec2(x + BAR, BARS_HIGH) * scale,
            );
            painter.rect_filled(bar, 0.0, if on { ink } else { t.color("muted") });
        }
        if moving {
            ui.ctx().request_repaint();
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_is_the_webs() {
        // printed by the web's `seeded` under bun
        let mut random = seeded(3);
        assert_eq!(random(), 0.720_226_783_771_067_9);
        assert_eq!(random(), 0.038_662_160_513_922_57);
        assert_eq!(random(), 0.456_192_192_621_529_1);
        assert_eq!(seeded(0)(), 0.627_073_940_588_161_3);
        assert_eq!(seeded(0)(), seeded(1)());
    }

    #[test]
    fn flow_graph_is_the_webs() {
        // the default graph, printed by the web's `flowGraph(3)` under bun
        let (nodes, edges) = flow_graph(3, 6, (2, 4));
        assert_eq!(nodes.len(), 15);
        let expect = [
            (0, 0, 8.0, 262.0),
            (1, 1, 134.4, 73.332_064_791_023_73),
            (3, 1, 134.4, 298.956_774_832_308_27),
            (6, 2, 260.8, 243.060_831_729_322_67),
            (9, 3, 387.200_000_000_000_05, 238.681_201_374_158_26),
            (13, 4, 513.6, 392.122_925_313_189_6),
            (14, 5, 640.0, 262.0),
        ];
        for (index, column, x, y) in expect {
            assert_eq!(nodes[index], FlowNode { x, y, column }, "node {index}");
        }
        let joined: Vec<String> = edges.iter().map(|e| format!("{}-{}", e.from, e.to)).collect();
        assert_eq!(
            joined.join(" "),
            "0-1 0-2 0-3 0-4 1-5 2-6 4-7 3-6 5-8 6-9 7-10 8-11 9-12 10-13 12-14 11-14 13-14"
        );
        let lit: Vec<String> = edges
            .iter()
            .filter(|e| e.lit)
            .map(|e| format!("{}-{}", e.from, e.to))
            .collect();
        assert_eq!(lit.join(" "), "0-3 3-6 6-9 9-12 12-14");

        let (nodes, edges) = flow_graph(7, 6, (2, 4));
        assert_eq!(nodes[2].y, 390.942_436_580_856_6);
        assert_eq!(nodes[12].y, 312.084_544_984_996_34);
        assert_eq!(edges.len(), 18);
        let lit: Vec<String> = edges
            .iter()
            .filter(|e| e.lit)
            .map(|e| format!("{}-{}", e.from, e.to))
            .collect();
        assert_eq!(lit.join(" "), "0-2 2-5 5-9 9-13 13-14");
    }

    #[test]
    fn level_heights_peak_at_one() {
        let heights = level_heights(56, 7);
        assert_eq!(heights.len(), 56);
        // printed by the web's `levelHeights(56, 7)` under bun; `exp` may differ in its last bit
        assert_eq!(heights[0], 1.0);
        assert!((heights[10] - 0.191_789_985_181_803_1).abs() < 1e-12);
        assert!((heights[55] - 0.153_439_712_416_788_8).abs() < 1e-12);
        assert!(heights.iter().all(|h| (0.05..=1.0).contains(h)));
    }

    #[test]
    fn ease_ends_where_it_starts_and_stops() {
        assert!(css_ease(0.0, 0.0, 0.58, 1.0, 0.0).abs() < 1e-4);
        assert!((css_ease(0.0, 0.0, 0.58, 1.0, 1.0) - 1.0).abs() < 1e-4);
    }
}
