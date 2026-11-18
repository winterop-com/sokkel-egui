//! A network's force layout, after sokkel-ui-graph's `force.ts` on d3-force 3.0.1: links pull
//! linked bodies to `FORCE_LINK` apart, every body pushes the others away (Barnes-Hut over a
//! quadtree, as d3-quadtree 3.0.2 builds it), none overlaps another, and a weak pull toward the
//! middle keeps parts with no link between them in view.
//!
//! The arithmetic is d3's, step for step and in its order, so the same bodies and links settle at
//! the same places as on the web, to the last bit: bodies with no place start on d3's phyllotaxis
//! spiral, and coincident bodies are jiggled apart from d3's seeded linear congruential source
//! (seed 1), one source shared by every force as d3 shares it.

use std::collections::HashMap;

/// How long a link pulls toward, between the two bodies' middles.
pub const FORCE_LINK: f64 = 110.0;
/// Ticks run at once before the first drawing, so it already shows the network's shape; the rest
/// settle live, one a frame.
pub const FORCE_WARM_TICKS: usize = 150;
/// The most ticks run at once with motion reduced, where the whole settling happens before it is
/// drawn.
pub const FORCE_MOST_TICKS: usize = 600;
/// How warm a drag keeps the simulation while a body is held, so the rest follow it.
pub const DRAG_HEAT: f64 = 0.3;
/// How warm a network that changed starts, from where its bodies stood.
pub const CHANGED_HEAT: f64 = 0.3;

/// The repulsion between every two bodies, and how far it reaches.
const CHARGE: f64 = -380.0;
const CHARGE_REACH: f64 = 600.0;
/// The pull toward the middle, on each axis.
const CENTERING: f64 = 0.07;
/// The alpha below which the simulation counts as settled.
const ALPHA_MIN: f64 = 0.004;
/// d3's own alpha floor, which its decay is counted from before force.ts lowers `alphaMin`.
const D3_ALPHA_MIN: f64 = 0.001;
/// How much speed a body keeps from one tick to the next.
const VELOCITY_DECAY: f64 = 0.6;
/// Barnes-Hut's accuracy, squared (d3's `theta` of 0.9).
const THETA2: f64 = 0.81;
/// The closest two bodies count as for the repulsion, squared.
const DISTANCE_MIN2: f64 = 1.0;
/// d3's spiral for bodies with no place: its first radius and the golden angle.
const INITIAL_RADIUS: f64 = 10.0;

/// One body in the simulation: a node by its id, held apart from the others by `radius`. `x` and
/// `y` are its middle; NaN puts it on d3's starting spiral. `fx` and `fy` pin it where a drag holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct ForceBody {
    pub id: String,
    pub radius: f64,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub fx: Option<f64>,
    pub fy: Option<f64>,
}

impl ForceBody {
    /// A body with no place yet, which the simulation starts on its spiral.
    pub fn new(id: impl Into<String>, radius: f64) -> Self {
        Self {
            id: id.into(),
            radius,
            x: f64::NAN,
            y: f64::NAN,
            vx: f64::NAN,
            vy: f64::NAN,
            fx: None,
            fy: None,
        }
    }

    /// The body starting from a place it had.
    pub fn at(mut self, x: f64, y: f64) -> Self {
        self.x = x;
        self.y = y;
        self
    }
}

/// d3's linear congruential source: the same numbers, from the same seed, as `lcg()` in d3-force.
#[derive(Clone, Debug)]
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        const M: u64 = 1 << 32;
        self.0 = (1_664_525 * self.0 + 1_013_904_223) % M;
        self.0 as f64 / M as f64
    }

    /// A nudge too small to see, for two bodies at the same place.
    fn jiggle(&mut self) -> f64 {
        (self.next() - 0.5) * 1e-6
    }
}

/// One link, as d3's link force keeps it: its ends, strength, bias and length.
#[derive(Clone, Debug)]
struct Spring {
    source: usize,
    target: usize,
    strength: f64,
    bias: f64,
    distance: f64,
}

/// A force simulation of bodies and their links, with force.ts's forces: `link` (110 long),
/// `charge` (-380, out to 600), `collide` (each body's radius), and `x` and `y` (0.07 toward the
/// middle), applied in that order on each tick, as d3 applies its forces in the order they were added.
#[derive(Clone, Debug)]
pub struct ForceSimulation {
    bodies: Vec<ForceBody>,
    springs: Vec<Spring>,
    alpha: f64,
    alpha_min: f64,
    alpha_decay: f64,
    alpha_target: f64,
    random: Lcg,
}

impl ForceSimulation {
    /// A simulation of `bodies`, stopped, with a link for each `(source, target)` pair of ids. A
    /// link to an id no body has is left out (d3 would throw), so callers keep only known ones.
    pub fn new<S: AsRef<str>>(mut bodies: Vec<ForceBody>, links: &[(S, S)]) -> Self {
        // d3's initializeNodes: pinned bodies at their pins, the rest without a place on the spiral
        let angle = std::f64::consts::PI * (3.0 - 5f64.sqrt());
        for (i, body) in bodies.iter_mut().enumerate() {
            if let Some(fx) = body.fx {
                body.x = fx;
            }
            if let Some(fy) = body.fy {
                body.y = fy;
            }
            if body.x.is_nan() || body.y.is_nan() {
                let radius = INITIAL_RADIUS * (0.5 + i as f64).sqrt();
                let (sin, cos) = sin_cos(i as f64 * angle);
                body.x = radius * cos;
                body.y = radius * sin;
            }
            if body.vx.is_nan() || body.vy.is_nan() {
                body.vx = 0.0;
                body.vy = 0.0;
            }
        }
        // d3's link initialize: each body's link count, then each link's bias and strength
        let index: HashMap<&str, usize> = bodies.iter().enumerate().map(|(i, b)| (b.id.as_str(), i)).collect();
        let ends: Vec<(usize, usize)> = links
            .iter()
            .filter_map(|(s, t)| Some((*index.get(s.as_ref())?, *index.get(t.as_ref())?)))
            .collect();
        let mut count = vec![0usize; bodies.len()];
        for &(s, t) in &ends {
            count[s] += 1;
            count[t] += 1;
        }
        let springs = ends
            .iter()
            .map(|&(source, target)| {
                let (cs, ct) = (count[source] as f64, count[target] as f64);
                Spring {
                    source,
                    target,
                    strength: 1.0 / cs.min(ct),
                    bias: cs / (cs + ct),
                    distance: FORCE_LINK,
                }
            })
            .collect();
        Self {
            bodies,
            springs,
            alpha: 1.0,
            alpha_min: ALPHA_MIN,
            alpha_decay: 1.0 - D3_ALPHA_MIN.powf(1.0 / 300.0),
            alpha_target: 0.0,
            random: Lcg(1),
        }
    }

    pub fn bodies(&self) -> &[ForceBody] {
        &self.bodies
    }

    pub fn bodies_mut(&mut self) -> &mut [ForceBody] {
        &mut self.bodies
    }

    pub fn body_mut(&mut self, id: &str) -> Option<&mut ForceBody> {
        self.bodies.iter_mut().find(|b| b.id == id)
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    pub fn set_alpha(&mut self, alpha: f64) {
        self.alpha = alpha;
    }

    pub fn alpha_min(&self) -> f64 {
        self.alpha_min
    }

    pub fn alpha_target(&self) -> f64 {
        self.alpha_target
    }

    /// The alpha the simulation cools or warms toward: `DRAG_HEAT` while a body is held, 0 after.
    pub fn set_alpha_target(&mut self, target: f64) {
        self.alpha_target = target;
    }

    /// Whether the simulation has cooled below its floor, as d3's timer stops there.
    pub fn settled(&self) -> bool {
        self.alpha < self.alpha_min
    }

    /// Ticks until the simulation settles or `most` ticks have run, as force.ts's `settle`.
    pub fn settle(&mut self, most: usize) {
        let mut tick = 0;
        while tick < most && self.alpha >= self.alpha_min {
            self.tick();
            tick += 1;
        }
    }

    /// One tick: alpha steps toward its target, each force moves the bodies' speeds, and each body
    /// not pinned moves by its speed, which decays.
    pub fn tick(&mut self) {
        self.alpha += (self.alpha_target - self.alpha) * self.alpha_decay;
        let alpha = self.alpha;
        self.link(alpha);
        self.charge(alpha);
        self.collide();
        for body in &mut self.bodies {
            body.vx += (0.0 - body.x) * CENTERING * alpha;
        }
        for body in &mut self.bodies {
            body.vy += (0.0 - body.y) * CENTERING * alpha;
        }
        for body in &mut self.bodies {
            match body.fx {
                None => {
                    body.vx *= VELOCITY_DECAY;
                    body.x += body.vx;
                }
                Some(fx) => {
                    body.x = fx;
                    body.vx = 0.0;
                }
            }
            match body.fy {
                None => {
                    body.vy *= VELOCITY_DECAY;
                    body.y += body.vy;
                }
                Some(fy) => {
                    body.y = fy;
                    body.vy = 0.0;
                }
            }
        }
    }

    /// d3's `forceLink`: each link pulls or pushes its ends toward its length, shared by their bias.
    fn link(&mut self, alpha: f64) {
        for spring in &self.springs {
            let (s, t) = (&self.bodies[spring.source], &self.bodies[spring.target]);
            let mut x = t.x + t.vx - s.x - s.vx;
            if x == 0.0 || x.is_nan() {
                x = self.random.jiggle();
            }
            let (s, t) = (&self.bodies[spring.source], &self.bodies[spring.target]);
            let mut y = t.y + t.vy - s.y - s.vy;
            if y == 0.0 || y.is_nan() {
                y = self.random.jiggle();
            }
            let mut l = (x * x + y * y).sqrt();
            l = (l - spring.distance) / l * alpha * spring.strength;
            x *= l;
            y *= l;
            let b = spring.bias;
            let t = &mut self.bodies[spring.target];
            t.vx -= x * b;
            t.vy -= y * b;
            let b = 1.0 - b;
            let s = &mut self.bodies[spring.source];
            s.vx += x * b;
            s.vy += y * b;
        }
    }

    /// d3's `forceManyBody`: every body pushes the others, far groups of them counted as one.
    fn charge(&mut self, alpha: f64) {
        let xs: Vec<f64> = self.bodies.iter().map(|b| b.x).collect();
        let ys: Vec<f64> = self.bodies.iter().map(|b| b.y).collect();
        let mut tree = Quadtree::new(&xs, &ys);
        // accumulate: each cell's total charge and its charge-weighted middle
        tree.visit_after(|tree, cell, _| {
            let mut strength = 0.0;
            match tree.cells[cell].kind {
                Kind::Inner(children) => {
                    let (mut x, mut y, mut weight) = (0.0, 0.0, 0.0);
                    for child in children.into_iter().flatten() {
                        let q = &tree.cells[child];
                        let c = q.value.abs();
                        if c != 0.0 && !c.is_nan() {
                            strength += q.value;
                            weight += c;
                            x += c * q.x;
                            y += c * q.y;
                        }
                    }
                    tree.cells[cell].x = x / weight;
                    tree.cells[cell].y = y / weight;
                }
                Kind::Leaf { data, .. } => {
                    tree.cells[cell].x = xs[data];
                    tree.cells[cell].y = ys[data];
                    let mut at = Some(cell);
                    while let Some(q) = at {
                        strength += CHARGE;
                        at = tree.cells[q].next();
                    }
                }
            }
            tree.cells[cell].value = strength;
        });
        let reach2 = CHARGE_REACH * CHARGE_REACH;
        for i in 0..self.bodies.len() {
            let random = &mut self.random;
            let bodies = &mut self.bodies;
            tree.visit(|tree, cell, [x1, _, x2, _]| {
                let quad = &tree.cells[cell];
                if quad.value == 0.0 || quad.value.is_nan() {
                    return true;
                }
                let node = &bodies[i];
                let mut x = quad.x - node.x;
                let mut y = quad.y - node.y;
                let w = x2 - x1;
                let mut l = x * x + y * y;
                // Barnes-Hut: a cell far enough away counts as one body at its middle
                if w * w / THETA2 < l {
                    if l < reach2 {
                        if x == 0.0 {
                            x = random.jiggle();
                            l += x * x;
                        }
                        if y == 0.0 {
                            y = random.jiggle();
                            l += y * y;
                        }
                        if l < DISTANCE_MIN2 {
                            l = (DISTANCE_MIN2 * l).sqrt();
                        }
                        let node = &mut bodies[i];
                        node.vx += x * quad.value * alpha / l;
                        node.vy += y * quad.value * alpha / l;
                    }
                    return true;
                }
                let Kind::Leaf { data, .. } = quad.kind else {
                    return false;
                };
                if l >= reach2 {
                    return false;
                }
                if data != i || quad.next().is_some() {
                    if x == 0.0 {
                        x = random.jiggle();
                        l += x * x;
                    }
                    if y == 0.0 {
                        y = random.jiggle();
                        l += y * y;
                    }
                    if l < DISTANCE_MIN2 {
                        l = (DISTANCE_MIN2 * l).sqrt();
                    }
                }
                let mut at = Some(cell);
                while let Some(q) = at {
                    let Kind::Leaf { data, next } = tree.cells[q].kind else {
                        break;
                    };
                    if data != i {
                        let w = CHARGE * alpha / l;
                        let node = &mut bodies[i];
                        node.vx += x * w;
                        node.vy += y * w;
                    }
                    at = next;
                }
                false
            });
        }
    }

    /// d3's `forceCollide`: two bodies closer than their radii together are pushed apart, the
    /// smaller one more.
    fn collide(&mut self) {
        let xs: Vec<f64> = self.bodies.iter().map(|b| b.x + b.vx).collect();
        let ys: Vec<f64> = self.bodies.iter().map(|b| b.y + b.vy).collect();
        let radii: Vec<f64> = self.bodies.iter().map(|b| b.radius).collect();
        let mut tree = Quadtree::new(&xs, &ys);
        // prepare: each cell's largest radius
        tree.visit_after(|tree, cell, _| {
            let r = match tree.cells[cell].kind {
                Kind::Leaf { data, .. } => radii[data],
                Kind::Inner(children) => {
                    let mut r = 0.0;
                    for child in children.into_iter().flatten() {
                        if tree.cells[child].r > r {
                            r = tree.cells[child].r;
                        }
                    }
                    r
                }
            };
            tree.cells[cell].r = r;
        });
        for i in 0..self.bodies.len() {
            let ri = radii[i];
            let ri2 = ri * ri;
            let xi = self.bodies[i].x + self.bodies[i].vx;
            let yi = self.bodies[i].y + self.bodies[i].vy;
            let random = &mut self.random;
            let bodies = &mut self.bodies;
            tree.visit(|tree, cell, [x0, y0, x1, y1]| {
                let quad = &tree.cells[cell];
                let mut rj = quad.r;
                let mut r = ri + rj;
                if let Kind::Leaf { data, .. } = quad.kind {
                    if data > i {
                        let other = &bodies[data];
                        let mut x = xi - other.x - other.vx;
                        let mut y = yi - other.y - other.vy;
                        let mut l = x * x + y * y;
                        if l < r * r {
                            if x == 0.0 {
                                x = random.jiggle();
                                l += x * x;
                            }
                            if y == 0.0 {
                                y = random.jiggle();
                                l += y * y;
                            }
                            let root = l.sqrt();
                            l = (r - root) / root;
                            x *= l;
                            y *= l;
                            rj *= rj;
                            r = rj / (ri2 + rj);
                            bodies[i].vx += x * r;
                            bodies[i].vy += y * r;
                            r = 1.0 - r;
                            bodies[data].vx -= x * r;
                            bodies[data].vy -= y * r;
                        }
                    }
                    return false;
                }
                x0 > xi + r || x1 < xi - r || y0 > yi + r || y1 < yi - r
            });
        }
    }
}

/// A quadtree cell: four children, or a body with any others at the very same place after it.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Inner([Option<usize>; 4]),
    Leaf { data: usize, next: Option<usize> },
}

/// A cell and what the forces count in it: its charge and charge's middle, or its largest radius.
#[derive(Clone, Copy, Debug)]
struct Cell {
    kind: Kind,
    value: f64,
    x: f64,
    y: f64,
    r: f64,
}

impl Cell {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            value: 0.0,
            x: 0.0,
            y: 0.0,
            r: 0.0,
        }
    }

    fn next(&self) -> Option<usize> {
        match self.kind {
            Kind::Leaf { next, .. } => next,
            Kind::Inner(_) => None,
        }
    }
}

/// d3-quadtree's tree over points `xs[i], ys[i]`, built as `quadtree(nodes, x, y)` builds it: the
/// extent covered from whole numbers and doubled, then each point added in order.
struct Quadtree {
    cells: Vec<Cell>,
    root: Option<usize>,
    extent: [f64; 4],
}

impl Quadtree {
    fn new(xs: &[f64], ys: &[f64]) -> Self {
        let mut tree = Self {
            cells: Vec::with_capacity(xs.len() * 2),
            root: None,
            extent: [f64::NAN; 4],
        };
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (&x, &y) in xs.iter().zip(ys) {
            if x.is_nan() || y.is_nan() {
                continue;
            }
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
        if x0 > x1 || y0 > y1 {
            return tree;
        }
        tree.cover(x0, y0);
        tree.cover(x1, y1);
        for i in 0..xs.len() {
            tree.add(xs, ys, i);
        }
        tree
    }

    /// Grows the extent to take in a point; the tree is still empty while it is covered.
    fn cover(&mut self, x: f64, y: f64) {
        let [mut x0, mut y0, mut x1, mut y1] = self.extent;
        if x0.is_nan() {
            x0 = x.floor();
            x1 = x0 + 1.0;
            y0 = y.floor();
            y1 = y0 + 1.0;
        } else {
            let mut z = if x1 - x0 != 0.0 { x1 - x0 } else { 1.0 };
            while x0 > x || x >= x1 || y0 > y || y >= y1 {
                let i = (usize::from(y < y0) << 1) | usize::from(x < x0);
                z *= 2.0;
                match i {
                    0 => {
                        x1 = x0 + z;
                        y1 = y0 + z;
                    }
                    1 => {
                        x0 = x1 - z;
                        y1 = y0 + z;
                    }
                    2 => {
                        x1 = x0 + z;
                        y0 = y1 - z;
                    }
                    _ => {
                        x0 = x1 - z;
                        y0 = y1 - z;
                    }
                }
            }
        }
        self.extent = [x0, y0, x1, y1];
    }

    fn push(&mut self, kind: Kind) -> usize {
        self.cells.push(Cell::new(kind));
        self.cells.len() - 1
    }

    fn set_child(&mut self, parent: usize, i: usize, child: usize) {
        if let Kind::Inner(children) = &mut self.cells[parent].kind {
            children[i] = Some(child);
        }
    }

    /// d3-quadtree's `add`: down to the cell the point falls in, splitting a leaf until the old
    /// point and the new one part, or chaining the new one before an old one at the same place.
    fn add(&mut self, xs: &[f64], ys: &[f64], d: usize) {
        let (x, y) = (xs[d], ys[d]);
        if x.is_nan() || y.is_nan() {
            return;
        }
        let leaf = self.push(Kind::Leaf { data: d, next: None });
        let Some(mut node) = self.root else {
            self.root = Some(leaf);
            return;
        };
        let [mut x0, mut y0, mut x1, mut y1] = self.extent;
        let mut parent: Option<usize> = None;
        let mut i = 0;
        while let Kind::Inner(children) = self.cells[node].kind {
            let xm = (x0 + x1) / 2.0;
            let right = x >= xm;
            if right {
                x0 = xm
            } else {
                x1 = xm
            }
            let ym = (y0 + y1) / 2.0;
            let bottom = y >= ym;
            if bottom {
                y0 = ym
            } else {
                y1 = ym
            }
            i = (usize::from(bottom) << 1) | usize::from(right);
            parent = Some(node);
            match children[i] {
                Some(child) => node = child,
                None => {
                    self.set_child(node, i, leaf);
                    return;
                }
            }
        }
        let Kind::Leaf { data: old, .. } = self.cells[node].kind else {
            return;
        };
        let (xp, yp) = (xs[old], ys[old]);
        if x == xp && y == yp {
            self.cells[leaf].kind = Kind::Leaf {
                data: d,
                next: Some(node),
            };
            match parent {
                Some(p) => self.set_child(p, i, leaf),
                None => self.root = Some(leaf),
            }
            return;
        }
        loop {
            let inner = self.push(Kind::Inner([None; 4]));
            match parent {
                Some(p) => self.set_child(p, i, inner),
                None => self.root = Some(inner),
            }
            parent = Some(inner);
            let xm = (x0 + x1) / 2.0;
            let right = x >= xm;
            if right {
                x0 = xm
            } else {
                x1 = xm
            }
            let ym = (y0 + y1) / 2.0;
            let bottom = y >= ym;
            if bottom {
                y0 = ym
            } else {
                y1 = ym
            }
            i = (usize::from(bottom) << 1) | usize::from(right);
            let j = (usize::from(yp >= ym) << 1) | usize::from(xp >= xm);
            if i != j {
                self.set_child(inner, j, node);
                self.set_child(inner, i, leaf);
                return;
            }
        }
    }

    /// d3-quadtree's `visit`: each cell before its children, children 0 to 3, skipping the
    /// children of a cell the callback answers true for. The callback gets the cell's extent.
    fn visit(&self, mut callback: impl FnMut(&Self, usize, [f64; 4]) -> bool) {
        let Some(root) = self.root else { return };
        let mut quads = vec![(root, self.extent)];
        while let Some((cell, [x0, y0, x1, y1])) = quads.pop() {
            if !callback(self, cell, [x0, y0, x1, y1])
                && let Kind::Inner(children) = self.cells[cell].kind
            {
                let (xm, ym) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                if let Some(c) = children[3] {
                    quads.push((c, [xm, ym, x1, y1]));
                }
                if let Some(c) = children[2] {
                    quads.push((c, [x0, ym, xm, y1]));
                }
                if let Some(c) = children[1] {
                    quads.push((c, [xm, y0, x1, ym]));
                }
                if let Some(c) = children[0] {
                    quads.push((c, [x0, y0, xm, ym]));
                }
            }
        }
    }

    /// d3-quadtree's `visitAfter`: each cell after its children.
    fn visit_after(&mut self, mut callback: impl FnMut(&mut Self, usize, [f64; 4])) {
        let Some(root) = self.root else { return };
        let mut quads = vec![(root, self.extent)];
        let mut next = Vec::new();
        while let Some((cell, [x0, y0, x1, y1])) = quads.pop() {
            if let Kind::Inner(children) = self.cells[cell].kind {
                let (xm, ym) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                if let Some(c) = children[0] {
                    quads.push((c, [x0, y0, xm, ym]));
                }
                if let Some(c) = children[1] {
                    quads.push((c, [xm, y0, x1, ym]));
                }
                if let Some(c) = children[2] {
                    quads.push((c, [x0, ym, xm, y1]));
                }
                if let Some(c) = children[3] {
                    quads.push((c, [xm, ym, x1, y1]));
                }
            }
            next.push((cell, [x0, y0, x1, y1]));
        }
        while let Some((cell, extent)) = next.pop() {
            callback(self, cell, extent);
        }
    }
}

/// The sine and cosine of `x`, correctly rounded, as a browser's `Math.sin` and `Math.cos` give
/// them (V8's, from glibc, round correctly where the system's own may be an ulp off), so the
/// spiral the bodies start on is the web's to the bit. Worked in double-double arithmetic: `x`
/// less the nearest multiple of pi/2 (pi/2 in four parts, as fdlibm splits it), then Taylor series.
pub fn sin_cos(x: f64) -> (f64, f64) {
    if !x.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    // pi/2 as fdlibm's __ieee754_rem_pio2 splits it: 33 bits, 33 more, 33 more, and the rest
    const PIO2: [f64; 4] = [
        1.570_796_326_734_125_6,
        6.077_100_506_303_966e-11,
        2.022_266_248_711_166_5e-21,
        8.478_427_660_368_9e-32,
    ];
    let k = (x * std::f64::consts::FRAC_2_PI).round();
    let mut r = Dd::new(x - k * PIO2[0], 0.0);
    for part in &PIO2[1..] {
        r = r.sub(Dd::product(k, *part));
    }
    let r2 = r.mul(r);
    let series = |mut term: Dd, mut n: f64| {
        let mut sum = term;
        loop {
            term = term.mul(r2).div((n + 1.0) * (n + 2.0)).neg();
            if term.hi.abs() < sum.hi.abs() * 1e-36 || term.hi == 0.0 {
                return sum;
            }
            sum = sum.add(term);
            n += 2.0;
        }
    };
    let s = series(r, 1.0).round();
    let c = series(Dd::new(1.0, 0.0), 0.0).round();
    match (k as i64).rem_euclid(4) {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

/// A double-double: `hi + lo`, about 106 bits.
#[derive(Clone, Copy, Debug)]
struct Dd {
    hi: f64,
    lo: f64,
}

impl Dd {
    fn new(hi: f64, lo: f64) -> Self {
        let s = hi + lo;
        Self {
            hi: s,
            lo: lo - (s - hi),
        }
    }

    fn two_sum(a: f64, b: f64) -> Self {
        let s = a + b;
        let v = s - a;
        Self {
            hi: s,
            lo: (a - (s - v)) + (b - v),
        }
    }

    fn product(a: f64, b: f64) -> Self {
        let p = a * b;
        Self {
            hi: p,
            lo: a.mul_add(b, -p),
        }
    }

    fn add(self, other: Self) -> Self {
        let s = Self::two_sum(self.hi, other.hi);
        let t = Self::two_sum(self.lo, other.lo);
        let s = Self::new(s.hi, s.lo + t.hi);
        Self::new(s.hi, s.lo + t.lo)
    }

    fn neg(self) -> Self {
        Self {
            hi: -self.hi,
            lo: -self.lo,
        }
    }

    fn sub(self, other: Self) -> Self {
        self.add(other.neg())
    }

    fn mul(self, other: Self) -> Self {
        let p = Self::product(self.hi, other.hi);
        Self::new(p.hi, p.lo + (self.hi * other.lo + self.lo * other.hi))
    }

    fn div(self, d: f64) -> Self {
        let q = self.hi / d;
        let p = Self::product(q, d);
        let e = (self.hi - p.hi - p.lo + self.lo) / d;
        Self::new(q, e)
    }

    /// The nearest double.
    fn round(self) -> f64 {
        self.hi + self.lo
    }
}
