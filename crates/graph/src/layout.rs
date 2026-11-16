//! Placing a graph in ranks, as sokkel-ui's graphs ask elk's layered layout to: ranks by the
//! longest path, edges against the nodes' order turned round to break cycles, the nodes of a rank
//! ordered to cross fewer edges, placed to straighten their edges, and the edges routed in
//! right-angled steps through the gaps between ranks.

use egui::{Pos2, Rect, Vec2, pos2};

/// Which way a graph's ranks run: left to right, or top to bottom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GraphDirection {
    #[default]
    Right,
    Down,
}

/// The gaps a layout keeps, in points: between ranks, between nodes of a rank, between an edge and
/// a node, and between two edges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphSpacing {
    pub rank: f32,
    pub node: f32,
    pub edge_node: f32,
    pub edge_edge: f32,
}

impl GraphSpacing {
    pub const COMPACT: Self = Self {
        rank: 24.0,
        node: 12.0,
        edge_node: 8.0,
        edge_edge: 6.0,
    };
    pub const NORMAL: Self = Self {
        rank: 40.0,
        node: 20.0,
        edge_node: 14.0,
        edge_edge: 10.0,
    };
    pub const ROOMY: Self = Self {
        rank: 72.0,
        node: 36.0,
        edge_node: 20.0,
        edge_edge: 14.0,
    };
}

impl Default for GraphSpacing {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// A graph placed: each node's box, and each edge's route from its source to its target (empty for
/// an edge to the node it leaves), in the graph's own points with its top left at the origin.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlacedGraph {
    pub nodes: Vec<Rect>,
    pub edges: Vec<Vec<Pos2>>,
}

impl PlacedGraph {
    /// What the drawing takes up, its edges' bends included.
    pub fn bounds(&self) -> Rect {
        let mut bounds = Rect::NOTHING;
        for rect in &self.nodes {
            bounds = bounds.union(*rect);
        }
        for point in self.edges.iter().flatten() {
            bounds.extend_with(*point);
        }
        if bounds.is_negative() { Rect::ZERO } else { bounds }
    }
}

/// One stop of the layout: a node, or a point a long edge passes through on a rank between.
#[derive(Clone, Copy)]
struct Stop {
    /// Its length along the ranks and across them.
    along: f32,
    across: f32,
    /// Where it stands in the order given: a node's index, a passing edge its source's.
    order: f32,
    real: bool,
}

/// One hop of an edge between neighboring ranks, from one stop to the next.
#[derive(Clone, Copy)]
struct Hop {
    from: usize,
    to: usize,
    edge: usize,
}

/// Place a graph of nodes of these sizes, with edges between their indices, running `direction`.
pub fn layout(
    sizes: &[Vec2],
    edges: &[(usize, usize)],
    direction: GraphDirection,
    spacing: GraphSpacing,
) -> PlacedGraph {
    let n = sizes.len();
    if n == 0 {
        return PlacedGraph::default();
    }
    let down = direction == GraphDirection::Down;
    let along = |s: Vec2| if down { s.y } else { s.x };
    let across = |s: Vec2| if down { s.x } else { s.y };

    // an edge against the order given is turned round, which leaves no cycle
    let kept: Vec<(usize, usize, usize, bool)> = edges
        .iter()
        .enumerate()
        .filter(|(_, (s, t))| s != t && *s < n && *t < n)
        .map(|(i, &(s, t))| if s < t { (i, s, t, false) } else { (i, t, s, true) })
        .collect();

    // ranks by the longest path, then a node with nothing before it moved up to just before the
    // first it leads to, as network simplex keeps edges short
    let mut rank = vec![0usize; n];
    for v in 0..n {
        for &(_, s, t, _) in &kept {
            if t == v {
                rank[v] = rank[v].max(rank[s] + 1);
            }
        }
    }
    // the order given is a topological one once edges against it are turned, so one pass settles it
    for v in (0..n).rev() {
        let leads: Vec<usize> = kept.iter().filter(|e| e.1 == v).map(|e| rank[e.2]).collect();
        let led = kept.iter().any(|e| e.2 == v);
        if !led && let Some(first) = leads.iter().min() {
            rank[v] = first - 1;
        }
    }
    let ranks = rank.iter().max().map_or(1, |r| r + 1);

    // stops: the nodes, then a passing point on every rank a long edge crosses
    let mut stops: Vec<Stop> = sizes
        .iter()
        .enumerate()
        .map(|(i, s)| Stop {
            along: along(*s),
            across: across(*s),
            order: i as f32,
            real: true,
        })
        .collect();
    let mut stop_rank = rank.clone();
    let mut hops: Vec<Hop> = Vec::new();
    for &(edge, s, t, _) in &kept {
        let mut from = s;
        for r in rank[s] + 1..rank[t] {
            stops.push(Stop {
                along: 0.0,
                across: 0.0,
                order: s as f32 + 0.5,
                real: false,
            });
            stop_rank.push(r);
            let here = stops.len() - 1;
            hops.push(Hop { from, to: here, edge });
            from = here;
        }
        hops.push(Hop { from, to: t, edge });
    }

    // each rank in the order given, then swept by barycenter, keeping the order crossing fewest
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); ranks];
    for (i, r) in stop_rank.iter().enumerate() {
        layers[*r].push(i);
    }
    for layer in &mut layers {
        layer.sort_by(|a, b| stops[*a].order.total_cmp(&stops[*b].order));
    }
    let mut best = layers.clone();
    let mut fewest = crossings(&layers, &hops, &stop_rank);
    for sweep in 0..8 {
        let downward = sweep % 2 == 0;
        let index = positions(&layers, stops.len());
        let order: Vec<usize> = if downward {
            (1..ranks).collect()
        } else {
            (0..ranks.saturating_sub(1)).rev().collect()
        };
        let mut index = index;
        for r in order {
            let mut keyed: Vec<(f32, usize)> = layers[r]
                .iter()
                .enumerate()
                .map(|(at, &v)| {
                    let others: Vec<f32> = hops
                        .iter()
                        .filter_map(|h| {
                            if downward && h.to == v {
                                Some(index[h.from] as f32)
                            } else if !downward && h.from == v {
                                Some(index[h.to] as f32)
                            } else {
                                None
                            }
                        })
                        .collect();
                    let key = if others.is_empty() {
                        at as f32
                    } else {
                        others.iter().sum::<f32>() / others.len() as f32
                    };
                    (key, v)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(stops[a.1].order.total_cmp(&stops[b.1].order)));
            layers[r] = keyed.into_iter().map(|(_, v)| v).collect();
            index = positions(&layers, stops.len());
        }
        let now = crossings(&layers, &hops, &stop_rank);
        if now < fewest {
            fewest = now;
            best = layers.clone();
        }
    }
    let layers = best;

    // across: each rank packed, then pulled toward its neighbors' centers in alternating sweeps,
    // holding its order and gaps
    let gap = |a: usize, b: usize| {
        if stops[a].real && stops[b].real {
            spacing.node
        } else if stops[a].real || stops[b].real {
            spacing.edge_node
        } else {
            spacing.edge_edge
        }
    };
    let mut center = vec![0.0f32; stops.len()];
    for layer in &layers {
        let mut at = 0.0;
        for (k, &v) in layer.iter().enumerate() {
            if k > 0 {
                at += gap(layer[k - 1], v);
            }
            center[v] = at + stops[v].across / 2.0;
            at += stops[v].across;
        }
    }
    // each sweep pulls a rank toward the rank it came from: down toward those before it, back up
    // toward those after it, the last sweep down
    for sweep in 0..25 {
        let downward = sweep % 2 == 0;
        let order: Vec<usize> = if downward {
            (1..ranks).collect()
        } else {
            (0..ranks.saturating_sub(1)).rev().collect()
        };
        for r in order {
            let layer = &layers[r];
            let wanted: Vec<f32> = layer
                .iter()
                .map(|&v| {
                    // a long edge's passing points pull hardest, so it runs straight, as network
                    // simplex weighs them
                    let weight = |u: usize| match (stops[v].real, stops[u].real) {
                        (true, true) => 1.0,
                        (false, false) => 8.0,
                        _ => 2.0,
                    };
                    let others: Vec<(f32, f32)> = hops
                        .iter()
                        .filter_map(|h| {
                            if downward && h.to == v {
                                Some((center[h.from], weight(h.from)))
                            } else if !downward && h.from == v {
                                Some((center[h.to], weight(h.to)))
                            } else {
                                None
                            }
                        })
                        .collect();
                    if others.is_empty() { center[v] } else { median(others) }
                })
                .collect();
            let apart: Vec<f32> = (1..layer.len())
                .map(|k| stops[layer[k - 1]].across / 2.0 + gap(layer[k - 1], layer[k]) + stops[layer[k]].across / 2.0)
                .collect();
            for (v, c) in layer.iter().zip(spaced(&wanted, &apart)) {
                center[*v] = c;
            }
        }
    }

    // along: each rank as deep as its deepest node, the gap after it widened for the edges whose
    // bends run through it
    let depth: Vec<f32> = (0..ranks)
        .map(|r| layers[r].iter().map(|&v| stops[v].along).fold(0.0, f32::max))
        .collect();

    // ports: a node's edges leave its far side and arrive on its near side, spread along it in the
    // order of their other ends
    let mut start_port = vec![0.0f32; hops.len()];
    let mut end_port = vec![0.0f32; hops.len()];
    for (v, stop) in stops.iter().enumerate() {
        if !stop.real {
            for (h, hop) in hops.iter().enumerate() {
                if hop.from == v {
                    start_port[h] = center[v];
                }
                if hop.to == v {
                    end_port[h] = center[v];
                }
            }
            continue;
        }
        for leaving in [true, false] {
            let mut side: Vec<usize> = (0..hops.len())
                .filter(|&h| if leaving { hops[h].from == v } else { hops[h].to == v })
                .collect();
            side.sort_by(|&a, &b| {
                let other = |h: usize| {
                    if leaving {
                        center[hops[h].to]
                    } else {
                        center[hops[h].from]
                    }
                };
                other(a).total_cmp(&other(b))
            });
            let count = side.len() as f32;
            let step = spacing.edge_edge.min(stop.across / (count + 1.0));
            for (k, &h) in side.iter().enumerate() {
                let at = center[v] + (k as f32 - (count - 1.0) / 2.0) * step;
                if leaving {
                    start_port[h] = at;
                } else {
                    end_port[h] = at;
                }
            }
        }
    }

    // the bends in each gap, one lane each, ordered so that edges running the same way do not cross
    let mut lane = vec![None::<f32>; hops.len()];
    let mut start = vec![0.0f32; ranks];
    let mut at = 0.0;
    for r in 0..ranks {
        start[r] = at;
        at += depth[r];
        if r + 1 == ranks {
            break;
        }
        let mut bending: Vec<usize> = (0..hops.len())
            .filter(|&h| stop_rank[hops[h].from] == r && (start_port[h] - end_port[h]).abs() > 0.5)
            .collect();
        // falling edges, the lowest start first; then rising ones, the highest start first
        bending.sort_by(|&a, &b| {
            let falls = |h: usize| end_port[h] > start_port[h];
            match (falls(a), falls(b)) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (true, true) => start_port[b].total_cmp(&start_port[a]),
                (false, false) => start_port[a].total_cmp(&start_port[b]),
            }
        });
        // bends whose runs do not overlap share a lane; one that overlaps an earlier one goes past it
        let mut lanes: Vec<Vec<(f32, f32)>> = Vec::new();
        let mut lane_of = Vec::new();
        for &h in &bending {
            let span = (start_port[h].min(end_port[h]), start_port[h].max(end_port[h]));
            let clear = |runs: &Vec<(f32, f32)>| {
                runs.iter()
                    .all(|r| span.1 + spacing.edge_edge <= r.0 || r.1 + spacing.edge_edge <= span.0)
            };
            let after = lane_of
                .iter()
                .zip(&bending)
                .filter(|&(_, &other)| {
                    let o = (
                        start_port[other].min(end_port[other]),
                        start_port[other].max(end_port[other]),
                    );
                    !(span.1 + spacing.edge_edge <= o.0 || o.1 + spacing.edge_edge <= span.0)
                })
                .map(|(l, _)| l + 1)
                .max()
                .unwrap_or(0);
            let k = (after..lanes.len()).find(|&k| clear(&lanes[k])).unwrap_or(lanes.len());
            if k == lanes.len() {
                lanes.push(Vec::new());
            }
            lanes[k].push(span);
            lane_of.push(k);
        }
        let count = lanes.len() as f32;
        let needed = 2.0 * spacing.edge_node + (count - 1.0).max(0.0) * spacing.edge_edge;
        let width = spacing.rank.max(needed);
        let first = at + width / 2.0 - (count - 1.0).max(0.0) * spacing.edge_edge / 2.0;
        for (&h, &k) in bending.iter().zip(&lane_of) {
            lane[h] = Some(first + k as f32 * spacing.edge_edge);
        }
        at += width;
    }

    let point = |a: f32, c: f32| if down { pos2(c, a) } else { pos2(a, c) };
    let mut placed = PlacedGraph {
        nodes: (0..n)
            .map(|v| {
                let r = rank[v];
                let a = start[r] + (depth[r] - stops[v].along) / 2.0;
                let c = center[v] - stops[v].across / 2.0;
                let min = point(a, c);
                Rect::from_min_size(min, sizes[v])
            })
            .collect(),
        edges: vec![Vec::new(); edges.len()],
    };
    for &(edge, s, t, turned) in &kept {
        let mut route: Vec<Pos2> = Vec::new();
        for (h, hop) in hops.iter().enumerate().filter(|(_, hop)| hop.edge == edge) {
            let r = stop_rank[hop.from];
            let leave = if hop.from == s {
                start[r] + (depth[r] + stops[s].along) / 2.0
            } else {
                start[r] + depth[r]
            };
            let arrive = if hop.to == t {
                start[r + 1] + (depth[r + 1] - stops[t].along) / 2.0
            } else {
                start[r + 1]
            };
            if hop.from != s {
                // a passing point runs straight across its rank
                route.push(point(start[r], start_port[h]));
            }
            route.push(point(leave, start_port[h]));
            if let Some(x) = lane[h] {
                route.push(point(x, start_port[h]));
                route.push(point(x, end_port[h]));
            }
            route.push(point(arrive, end_port[h]));
        }
        route.dedup_by(|a, b| (*a - *b).length() < 0.01);
        let mut route = straightened(route);
        if turned {
            route.reverse();
        }
        placed.edges[edge] = route;
    }

    // the top left at the origin
    let offset = placed.bounds().min.to_vec2();
    for rect in &mut placed.nodes {
        *rect = rect.translate(-offset);
    }
    for point in placed.edges.iter_mut().flatten() {
        *point -= offset;
    }
    placed
}

/// Each stop's index in its rank.
fn positions(layers: &[Vec<usize>], count: usize) -> Vec<usize> {
    let mut index = vec![0; count];
    for layer in layers {
        for (k, &v) in layer.iter().enumerate() {
            index[v] = k;
        }
    }
    index
}

/// How many pairs of hops cross between neighboring ranks.
fn crossings(layers: &[Vec<usize>], hops: &[Hop], rank: &[usize]) -> usize {
    let index = positions(layers, rank.len());
    let mut count = 0;
    for (i, a) in hops.iter().enumerate() {
        for b in &hops[i + 1..] {
            if rank[a.from] != rank[b.from] {
                continue;
            }
            let (a1, a2, b1, b2) = (index[a.from], index[a.to], index[b.from], index[b.to]);
            if (a1 < b1 && a2 > b2) || (a1 > b1 && a2 < b2) {
                count += 1;
            }
        }
    }
    count
}

/// The weighted middle of some values: where half the weight lies either side, or halfway between
/// the two values the half falls between.
fn median(mut values: Vec<(f32, f32)>) -> f32 {
    values.sort_by(|a, b| a.0.total_cmp(&b.0));
    let half = values.iter().map(|v| v.1).sum::<f32>() / 2.0;
    let mut below = 0.0;
    for (k, (value, weight)) in values.iter().enumerate() {
        below += weight;
        if (below - half).abs() < 1e-4 {
            return (value + values.get(k + 1).map_or(*value, |next| next.0)) / 2.0;
        }
        if below > half {
            return *value;
        }
    }
    values.last().map_or(0.0, |v| v.0)
}

/// Centers as near as can be to `wanted`, in order, each at least `apart[k]` past the one before:
/// the least-squares fit, by pooling neighbors that would overlap.
fn spaced(wanted: &[f32], apart: &[f32]) -> Vec<f32> {
    let mut offset = vec![0.0f32; wanted.len()];
    for k in 1..wanted.len() {
        offset[k] = offset[k - 1] + apart[k - 1];
    }
    // with the gaps taken out, the centers need only not fall: pool adjacent violators
    let mut blocks: Vec<(f32, usize)> = Vec::new();
    for (k, w) in wanted.iter().enumerate() {
        blocks.push((w - offset[k], 1));
        while blocks.len() > 1 {
            let (b, nb) = blocks[blocks.len() - 1];
            let (a, na) = blocks[blocks.len() - 2];
            if a <= b {
                break;
            }
            blocks.pop();
            let last = blocks.len() - 1;
            blocks[last] = ((a * na as f32 + b * nb as f32) / (na + nb) as f32, na + nb);
        }
    }
    let mut out = Vec::with_capacity(wanted.len());
    for (value, count) in blocks {
        for _ in 0..count {
            out.push(value + offset[out.len()]);
        }
    }
    out
}

/// A route without the points that lie on a straight run between their neighbors.
fn straightened(route: Vec<Pos2>) -> Vec<Pos2> {
    if route.len() < 3 {
        return route;
    }
    let mut out = vec![route[0]];
    for k in 1..route.len() - 1 {
        let (a, b, c) = (out[out.len() - 1], route[k], route[k + 1]);
        let flat = ((a.x - b.x).abs() < 0.01 && (b.x - c.x).abs() < 0.01)
            || ((a.y - b.y).abs() < 0.01 && (b.y - c.y).abs() < 0.01);
        if !flat {
            out.push(b);
        }
    }
    out.push(route[route.len() - 1]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxes(n: usize) -> Vec<Vec2> {
        vec![egui::vec2(100.0, 44.0); n]
    }

    #[test]
    fn a_chain_runs_right_one_rank_apart() {
        let placed = layout(
            &boxes(3),
            &[(0, 1), (1, 2)],
            GraphDirection::Right,
            GraphSpacing::NORMAL,
        );
        assert_eq!(placed.nodes[0].left(), 0.0);
        assert_eq!(placed.nodes[1].left(), 140.0);
        assert_eq!(placed.nodes[2].left(), 280.0);
        assert_eq!(placed.nodes[0].center().y, placed.nodes[2].center().y);
        assert_eq!(placed.edges[0], vec![pos2(100.0, 22.0), pos2(140.0, 22.0)]);
    }

    #[test]
    fn a_chain_runs_down_when_told() {
        let placed = layout(&boxes(2), &[(0, 1)], GraphDirection::Down, GraphSpacing::NORMAL);
        assert_eq!(placed.nodes[1].top(), 84.0);
        assert_eq!(placed.nodes[0].center().x, placed.nodes[1].center().x);
    }

    #[test]
    fn an_edge_against_the_order_closes_a_cycle_and_is_routed_back() {
        let placed = layout(
            &boxes(2),
            &[(0, 1), (1, 0)],
            GraphDirection::Right,
            GraphSpacing::NORMAL,
        );
        assert!(placed.nodes[1].left() > placed.nodes[0].right());
        let back = &placed.edges[1];
        assert_eq!(
            back.first().unwrap().x,
            placed.nodes[1].left(),
            "it leaves its source's near side"
        );
        assert_eq!(
            back.last().unwrap().x,
            placed.nodes[0].right(),
            "and arrives on its target's far side"
        );
    }

    #[test]
    fn nodes_of_a_rank_keep_their_gap() {
        let placed = layout(
            &boxes(3),
            &[(0, 2), (1, 2)],
            GraphDirection::Right,
            GraphSpacing::NORMAL,
        );
        assert!(placed.nodes[1].top() - placed.nodes[0].bottom() >= 20.0 - 0.01);
        // the one they lead to stands between them
        let middle = (placed.nodes[0].center().y + placed.nodes[1].center().y) / 2.0;
        assert!((placed.nodes[2].center().y - middle).abs() < 1.0);
    }

    #[test]
    fn a_long_edge_passes_the_ranks_between() {
        let placed = layout(
            &boxes(3),
            &[(0, 1), (1, 2), (0, 2)],
            GraphDirection::Right,
            GraphSpacing::NORMAL,
        );
        assert_eq!(placed.nodes[2].left(), 280.0);
        let long = &placed.edges[2];
        assert!(long.len() >= 2);
        // it passes beside the middle node, not through it
        for pair in long.windows(2) {
            let run = Rect::from_two_pos(pair[0], pair[1]);
            assert!(!run.shrink(0.5).intersects(placed.nodes[1].shrink(0.5)), "{long:?}");
        }
    }

    #[test]
    fn a_long_edge_runs_straight_and_its_target_stands_level_with_it() {
        // services calling on stores, the last store calling back on one before it
        let mut sizes = vec![egui::vec2(122.0, 44.0); 5];
        sizes.extend([egui::vec2(130.0, 44.0); 3]);
        let edges = [(0, 1), (1, 5), (1, 6), (1, 7), (2, 5), (2, 6), (3, 5), (4, 5), (7, 6)];
        let placed = layout(&sizes, &edges, GraphDirection::Right, GraphSpacing::NORMAL);
        let (api, storage, search) = (placed.nodes[1], placed.nodes[6], placed.nodes[7]);
        assert!(
            search.left() > storage.right(),
            "the store calling back stands a rank on"
        );
        assert!(
            (search.center().y - api.center().y).abs() < 4.0,
            "level with the one calling on it"
        );
        assert!(
            storage.center().y > api.center().y,
            "the long edge passes above what it skips"
        );
        assert_eq!(placed.nodes[0].center().y, api.center().y, "a lone caller stands level");
    }
}
