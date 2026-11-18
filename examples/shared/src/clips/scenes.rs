//! The pictures the clips are made of, as sokkel-ui's `examples/shared/clips/scenes.ts` draws them:
//! a scene of Tidevann's world painted at any second, the same at the same second every time. Each
//! clip names its scene, its hues and its seed, so no two look alike.
//!
//! The web paints on a 2D canvas; here the same shapes go onto an `egui::Painter` within a `Rect`,
//! in the rectangle's own coordinates as the canvas has them (0 to `w` across, 0 to `h` down). A
//! linear or radial gradient is a mesh whose vertices carry the gradient's colors, a path filled
//! under a curve is a mesh of strips down to its floor, an arc is a circle, and the text is laid on
//! its baseline as the canvas writes it. The numbers, the seeded randomness and the order things
//! are drawn in are the web's.

use std::f64::consts::PI;

use egui::epaint::{Mesh, PathStroke};
use egui::{Color32, FontFamily, FontId, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

/// The scenes a clip can show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SceneKind {
    Sea,
    Lighthouse,
    Train,
    Peaks,
    Aurora,
    Rain,
    Tide,
    Islands,
    Fields,
}

impl SceneKind {
    pub const ALL: [SceneKind; 9] = [
        Self::Sea,
        Self::Lighthouse,
        Self::Train,
        Self::Peaks,
        Self::Aurora,
        Self::Rain,
        Self::Tide,
        Self::Islands,
        Self::Fields,
    ];

    /// The scene's name as the web writes it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sea => "sea",
            Self::Lighthouse => "lighthouse",
            Self::Train => "train",
            Self::Peaks => "peaks",
            Self::Aurora => "aurora",
            Self::Rain => "rain",
            Self::Tide => "tide",
            Self::Islands => "islands",
            Self::Fields => "fields",
        }
    }
}

/// What a clip draws: its scene, two hues (degrees on the color wheel), a seed, and the words on
/// its title card.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSpec {
    pub kind: SceneKind,
    pub hue: f64,
    pub accent: f64,
    pub seed: u32,
    pub title: String,
    pub channel: String,
    /// The clip's length in seconds.
    pub duration: f64,
}

/// A small seeded random source, the same numbers for the same seed, as the web's `seeded`.
#[derive(Clone, Debug)]
pub struct Seeded {
    state: u32,
}

impl Seeded {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// The next number, from 0 up to 1.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let state = self.state;
        let mut mixed = (state ^ (state >> 15)).wrapping_mul(1 | state);
        mixed = mixed.wrapping_add((mixed ^ (mixed >> 7)).wrapping_mul(61 | mixed)) ^ mixed;
        f64::from(mixed ^ (mixed >> 14)) / 4_294_967_296.0
    }
}

/// A number rounded as `Math.round` rounds it: a half up.
fn js_round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor >= 0.5 { floor + 1.0 } else { floor }
}

/// A color as the canvas keeps it before it paints: red, green and blue from 0 to 255, not
/// multiplied by its alpha, which runs from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Rgba {
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    /// The color egui paints with.
    pub fn color32(self) -> Color32 {
        let channel = |value: f64| value.round().clamp(0.0, 255.0) as u8;
        Color32::from_rgba_unmultiplied(
            channel(self.r),
            channel(self.g),
            channel(self.b),
            channel(self.a * 255.0),
        )
    }

    /// The color `t` of the way to `other`, mixed as the canvas mixes a gradient's stops.
    fn mix(self, other: Self, t: f64) -> Self {
        let at = |a: f64, b: f64| a + (b - a) * t;
        Self::new(
            at(self.r, other.r),
            at(self.g, other.g),
            at(self.b, other.b),
            at(self.a, other.a),
        )
    }
}

/// `hsla(hue, saturation%, lightness%, alpha)` as the web writes it, the hue rounded first.
pub fn hsl(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> Rgba {
    let hue = js_round(hue).rem_euclid(360.0) / 60.0;
    let s = (saturation / 100.0).clamp(0.0, 1.0);
    let l = (lightness / 100.0).clamp(0.0, 1.0);
    let t2 = if l <= 0.5 { l * (s + 1.0) } else { l + s - l * s };
    let t1 = l * 2.0 - t2;
    let channel = |mut h: f64| {
        if h < 0.0 {
            h += 6.0;
        }
        if h >= 6.0 {
            h -= 6.0;
        }
        let v = if h < 1.0 {
            (t2 - t1) * h + t1
        } else if h < 3.0 {
            t2
        } else if h < 4.0 {
            (t2 - t1) * (4.0 - h) + t1
        } else {
            t1
        };
        v * 255.0
    };
    Rgba::new(
        channel(hue + 2.0),
        channel(hue),
        channel(hue - 2.0),
        alpha.clamp(0.0, 1.0),
    )
}

/// `rgba(r, g, b, alpha)`.
pub const fn rgba(r: f64, g: f64, b: f64, alpha: f64) -> Rgba {
    Rgba::new(r, g, b, alpha)
}

/// The color a gradient's stops give at `t`, held to the first and last stop beyond them.
fn stop_at(stops: &[(f64, Rgba)], t: f64) -> Rgba {
    let Some(first) = stops.first() else {
        return rgba(0.0, 0.0, 0.0, 0.0);
    };
    if t <= first.0 {
        return first.1;
    }
    for pair in stops.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if t <= b.0 {
            let span = b.0 - a.0;
            return if span <= 0.0 {
                b.1
            } else {
                a.1.mix(b.1, (t - a.0) / span)
            };
        }
    }
    stops[stops.len() - 1].1
}

/// The places a gradient is cut at so a mesh carries it: each stop, and pieces between them, since
/// the canvas mixes the stops before it multiplies by alpha and a mesh mixes after.
fn cuts(stops: &[(f64, Rgba)]) -> Vec<f64> {
    const PIECES: usize = 6;
    let mut out = vec![0.0];
    let mut last = 0.0;
    for &(offset, _) in stops {
        let offset = offset.clamp(0.0, 1.0);
        for piece in 1..=PIECES {
            let at = last + (offset - last) * piece as f64 / PIECES as f64;
            if at > *out.last().unwrap_or(&0.0) {
                out.push(at);
            }
        }
        last = offset;
    }
    for piece in 1..=PIECES {
        let at = last + (1.0 - last) * piece as f64 / PIECES as f64;
        if at > *out.last().unwrap_or(&0.0) {
            out.push(at);
        }
    }
    out
}

/// A canvas `w` by `h` laid on a rectangle of the painter: what the scenes draw with.
pub struct Canvas<'a> {
    painter: &'a Painter,
    left: f64,
    top: f64,
    pub w: f64,
    pub h: f64,
}

impl<'a> Canvas<'a> {
    pub fn new(painter: &'a Painter, rect: Rect) -> Self {
        Self {
            painter,
            left: f64::from(rect.left()),
            top: f64::from(rect.top()),
            w: f64::from(rect.width()),
            h: f64::from(rect.height()),
        }
    }

    pub fn painter(&self) -> &Painter {
        self.painter
    }

    /// A point of the canvas on the screen.
    pub fn at(&self, x: f64, y: f64) -> Pos2 {
        pos2((self.left + x) as f32, (self.top + y) as f32)
    }

    /// A physical pixel, in points: what the canvas covers partly when a shape is thinner.
    fn pixel(&self) -> f64 {
        1.0 / f64::from(self.painter.pixels_per_point())
    }

    /// `fillRect`. A side thinner than a pixel is drawn a pixel wide and as faint as the part of
    /// the pixel it covers, as the canvas's smoothing paints it.
    pub fn rect(&self, x: f64, y: f64, w: f64, h: f64, mut color: Rgba) {
        if color.a <= 0.0 || w == 0.0 || h == 0.0 {
            return;
        }
        let (mut x, mut y, mut w, mut h) = (x.min(x + w), y.min(y + h), w.abs(), h.abs());
        let pixel = self.pixel();
        if w < pixel {
            color.a *= w / pixel;
            x -= (pixel - w) / 2.0;
            w = pixel;
        }
        if h < pixel {
            color.a *= h / pixel;
            y -= (pixel - h) / 2.0;
            h = pixel;
        }
        let rect = Rect::from_two_pos(self.at(x, y), self.at(x + w, y + h));
        self.painter.rect_filled(rect, 0.0, color.color32());
    }

    /// `fillRect` with a linear gradient running down from `from` to `to`.
    pub fn rect_down(&self, x: f64, y: f64, w: f64, h: f64, (from, to): (f64, f64), stops: &[(f64, Rgba)]) {
        let span = to - from;
        let mut rows: Vec<f64> = cuts(stops)
            .into_iter()
            .map(|t| from + span * t)
            .filter(|at| *at > y && *at < y + h)
            .collect();
        rows.insert(0, y);
        rows.push(y + h);
        let color = |at: f64| {
            let t = if span == 0.0 { 0.0 } else { (at - from) / span };
            stop_at(stops, t).color32()
        };
        let mut mesh = Mesh::default();
        for (index, at) in rows.iter().enumerate() {
            mesh.colored_vertex(self.at(x, *at), color(*at));
            mesh.colored_vertex(self.at(x + w, *at), color(*at));
            if index > 0 {
                let base = (index as u32 - 1) * 2;
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base + 1, base + 3, base + 2);
            }
        }
        self.painter.add(Shape::mesh(mesh));
    }

    /// A disc filled with a radial gradient from its center out to `radius`.
    pub fn radial(&self, cx: f64, cy: f64, radius: f64, stops: &[(f64, Rgba)]) {
        const SEGMENTS: usize = 72;
        let rings = cuts(stops);
        let mut mesh = Mesh::default();
        mesh.colored_vertex(self.at(cx, cy), stop_at(stops, 0.0).color32());
        for (ring, t) in rings.iter().enumerate().skip(1) {
            let color = stop_at(stops, *t).color32();
            for segment in 0..SEGMENTS {
                let angle = segment as f64 / SEGMENTS as f64 * PI * 2.0;
                mesh.colored_vertex(
                    self.at(cx + angle.cos() * radius * t, cy + angle.sin() * radius * t),
                    color,
                );
            }
            let ring_at = |ring: usize, segment: usize| -> u32 {
                if ring == 0 {
                    0
                } else {
                    (1 + (ring - 1) * SEGMENTS + segment % SEGMENTS) as u32
                }
            };
            for segment in 0..SEGMENTS {
                let (a, b) = (ring_at(ring, segment), ring_at(ring, segment + 1));
                if ring == 1 {
                    mesh.add_triangle(0, a, b);
                } else {
                    let (c, d) = (ring_at(ring - 1, segment), ring_at(ring - 1, segment + 1));
                    mesh.add_triangle(c, a, b);
                    mesh.add_triangle(c, b, d);
                }
            }
        }
        self.painter.add(Shape::mesh(mesh));
    }

    /// A path whose top edge runs through `points` from left to right, filled down to `floor`, in
    /// a color that may change with the height.
    pub fn fill_under(&self, points: &[(f64, f64)], floor: f64, color: impl Fn(f64) -> Rgba) {
        let mut mesh = Mesh::default();
        for pair in points.windows(2) {
            let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
            let base = mesh.vertices.len() as u32;
            mesh.colored_vertex(self.at(x0, y0), color(y0).color32());
            mesh.colored_vertex(self.at(x1, y1), color(y1).color32());
            mesh.colored_vertex(self.at(x1, floor), color(floor).color32());
            mesh.colored_vertex(self.at(x0, floor), color(floor).color32());
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
        // The top edge smoothed, away from the floor.
        let away = |(x, y): (f64, f64)| (x, y + if floor >= y { -1.0 } else { 1.0 });
        self.feather(&mut mesh, points, false, away, &color);
        self.painter.add(Shape::mesh(mesh));
    }

    /// A pixel-wide band along an edge through `points`, fading from the shape's color to nothing
    /// on the side `away` points to from each point, as the canvas smooths a shape's edge.
    fn feather(
        &self,
        mesh: &mut Mesh,
        points: &[(f64, f64)],
        closed: bool,
        away: impl Fn((f64, f64)) -> (f64, f64),
        color: &impl Fn(f64) -> Rgba,
    ) {
        let count = points.len();
        if count < 2 {
            return;
        }
        let pixel = self.pixel();
        let normal = |a: (f64, f64), b: (f64, f64)| -> (f64, f64) {
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let length = (dx * dx + dy * dy).sqrt();
            if length == 0.0 {
                (0.0, 0.0)
            } else {
                (dy / length, -dx / length)
            }
        };
        let segment = |index: usize| -> Option<(f64, f64)> {
            if index + 1 < count {
                Some(normal(points[index], points[index + 1]))
            } else if closed {
                Some(normal(points[index], points[0]))
            } else {
                None
            }
        };
        let base = mesh.vertices.len() as u32;
        for (index, point) in points.iter().enumerate() {
            let before = if index > 0 {
                segment(index - 1)
            } else if closed {
                segment(count - 1)
            } else {
                None
            };
            let (nx, ny) = match (before, segment(index)) {
                (Some(a), Some(b)) => (a.0 + b.0, a.1 + b.1),
                (Some(a), None) | (None, Some(a)) => a,
                (None, None) => (0.0, 0.0),
            };
            let length = (nx * nx + ny * ny).sqrt();
            let (mut nx, mut ny) = if length == 0.0 {
                (0.0, 0.0)
            } else {
                (nx / length, ny / length)
            };
            let toward = away(*point);
            if nx * (toward.0 - point.0) + ny * (toward.1 - point.1) < 0.0 {
                nx = -nx;
                ny = -ny;
            }
            let fill = color(point.1);
            mesh.colored_vertex(self.at(point.0, point.1), fill.color32());
            mesh.colored_vertex(
                self.at(point.0 + nx * pixel, point.1 + ny * pixel),
                Rgba { a: 0.0, ..fill }.color32(),
            );
        }
        let pairs = if closed { count } else { count - 1 };
        for index in 0..pairs {
            let (a, b) = (index as u32, ((index + 1) % count) as u32);
            mesh.add_triangle(base + a * 2, base + b * 2, base + a * 2 + 1);
            mesh.add_triangle(base + b * 2, base + b * 2 + 1, base + a * 2 + 1);
        }
    }

    /// A closed path around `center` through `points`, filled as the canvas fills a shape that
    /// every ray from its center crosses once.
    pub fn fan(&self, center: (f64, f64), points: &[(f64, f64)], color: Rgba) {
        let fill = color.color32();
        let mut mesh = Mesh::default();
        mesh.colored_vertex(self.at(center.0, center.1), fill);
        for &(x, y) in points {
            mesh.colored_vertex(self.at(x, y), fill);
        }
        let count = points.len() as u32;
        for index in 0..count {
            mesh.add_triangle(0, 1 + index, 1 + (index + 1) % count);
        }
        let away = |(x, y): (f64, f64)| (x + (x - center.0), y + (y - center.1));
        self.feather(&mut mesh, points, true, away, &|_| color);
        self.painter.add(Shape::mesh(mesh));
    }

    /// A convex shape through `points`, filled.
    pub fn polygon(&self, points: &[(f64, f64)], color: Rgba) {
        let points = points.iter().map(|(x, y)| self.at(*x, *y)).collect();
        self.painter
            .add(Shape::convex_polygon(points, color.color32(), Stroke::NONE));
    }

    /// `arc` over the whole turn, filled.
    pub fn circle(&self, x: f64, y: f64, radius: f64, mut color: Rgba) {
        // A dot smaller than a pixel is a pixel's dot as faint as the part of it the dot covers.
        let least = self.pixel() / 2.0;
        let radius = if radius < least {
            color.a *= (radius / least).powi(2);
            least
        } else {
            radius
        };
        self.painter
            .circle_filled(self.at(x, y), radius as f32, color.color32());
    }

    /// `arc` over the whole turn, stroked.
    pub fn ring(&self, x: f64, y: f64, radius: f64, width: f64, color: Rgba) {
        self.painter
            .circle_stroke(self.at(x, y), radius as f32, Stroke::new(width as f32, color.color32()));
    }

    /// A line through `points`, stroked.
    pub fn line(&self, points: &[(f64, f64)], width: f64, color: Rgba) {
        let points: Vec<Pos2> = points.iter().map(|(x, y)| self.at(*x, *y)).collect();
        self.painter
            .add(Shape::line(points, PathStroke::new(width as f32, color.color32())));
    }

    /// A line through `points`, stroked in dashes.
    pub fn dashed(&self, points: &[(f64, f64)], width: f64, color: Rgba, dash: f64) {
        let points: Vec<Pos2> = points.iter().map(|(x, y)| self.at(*x, *y)).collect();
        self.painter.extend(Shape::dashed_line(
            &points,
            Stroke::new(width as f32, color.color32()),
            dash as f32,
            dash as f32,
        ));
    }

    /// How wide `text` is in `font`, as `measureText` measures it.
    pub fn measure(&self, text: &str, font: &FontId) -> f64 {
        let galley = self
            .painter
            .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE);
        f64::from(galley.size().x)
    }

    /// `fillText`: `text` in `font` from `x`, its baseline at `y`.
    pub fn text(&self, x: f64, y: f64, text: &str, font: &FontId, color: Rgba) {
        if color.a <= 0.0 {
            return;
        }
        let fill = color.color32();
        let galley = self.painter.layout_no_wrap(text.to_owned(), font.clone(), fill);
        let baseline = galley
            .rows
            .first()
            .and_then(|row| row.row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y))
            .unwrap_or(font.size * 0.8);
        self.painter.galley(self.at(x, y) - vec2(0.0, baseline), galley, fill);
    }

    /// `text` broken into lines no wider than `width` in `font`, at the words `split` gives.
    pub fn wrap<'t>(&self, words: impl Iterator<Item = &'t str>, font: &FontId, width: f64) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in words {
            let next = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if self.measure(&next, font) > width && !line.is_empty() {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = next;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }
}

/// The points of a quadratic curve from `from` to `to` bent toward `control`, `steps` pieces long.
fn quadratic(from: (f64, f64), control: (f64, f64), to: (f64, f64), steps: usize) -> Vec<(f64, f64)> {
    (0..=steps)
        .map(|step| {
            let t = step as f64 / steps as f64;
            let u = 1.0 - t;
            (
                u * u * from.0 + 2.0 * u * t * control.0 + t * t * to.0,
                u * u * from.1 + 2.0 * u * t * control.1 + t * t * to.1,
            )
        })
        .collect()
}

/// A font of the face the canvas names: the sans one, or the monospace one.
pub fn font(size: f64, mono: bool) -> FontId {
    FontId::new(
        size as f32,
        if mono {
            FontFamily::Monospace
        } else {
            FontFamily::Proportional
        },
    )
}

/// The sky: a gradient down from `top` to `bottom` over the canvas's top `until` of it.
fn sky(c: &Canvas, top: Rgba, bottom: Rgba, until: f64) {
    c.rect_down(
        0.0,
        0.0,
        c.w,
        c.h * until,
        (0.0, c.h * until),
        &[(0.0, top), (1.0, bottom)],
    );
}

/// A band of water or hills whose top edge is a sum of slow waves, filled down to the bottom.
#[allow(clippy::too_many_arguments)]
fn wave(c: &Canvas, base: f64, height: f64, length: f64, shift: f64, fill: Rgba) {
    let mut points = Vec::new();
    let mut x = 0.0;
    while x <= c.w + 8.0 {
        let y = base
            + ((x / length) * PI * 2.0 + shift).sin() * height
            + ((x / (length * 0.37)) * PI * 2.0 - shift * 1.7).sin() * height * 0.3;
        points.push((x, y));
        x += 8.0;
    }
    c.fill_under(&points, c.h, |_| fill);
}

fn stars(c: &Canvas, seed: u32, t: f64, count: usize, until: f64) {
    let (w, h) = (c.w, c.h);
    let mut random = Seeded::new(seed);
    for _ in 0..count {
        let x = random.next() * w;
        let y = random.next() * h * until;
        let speed = 0.6 + random.next() * 1.4;
        let phase = random.next() * 6.0;
        let twinkle = 0.45 + 0.55 * (t * speed + phase).sin().abs();
        let size = if random.next() < 0.1 { 2.2 } else { 1.2 };
        let side = size * (w / 1280.0) + 0.6;
        c.rect(x, y, side, side, rgba(255.0, 255.0, 255.0, twinkle * 0.85));
    }
}

/// A sailing boat at `x`, `y`, `scale` times its own size, leaning by `tilt` radians.
fn boat(c: &Canvas, x: f64, y: f64, scale: f64, tilt: f64, color: Rgba) {
    let (sin, cos) = tilt.sin_cos();
    let place = |points: &[(f64, f64)]| -> Vec<(f64, f64)> {
        points
            .iter()
            .map(|(px, py)| {
                let (px, py) = (px * scale, py * scale);
                (x + px * cos - py * sin, y + px * sin + py * cos)
            })
            .collect()
    };
    c.polygon(&place(&[(-40.0, 0.0), (40.0, 0.0), (28.0, 14.0), (-30.0, 14.0)]), color);
    c.polygon(&place(&[(-2.0, -46.0), (2.0, -46.0), (2.0, 0.0), (-2.0, 0.0)]), color);
    c.polygon(&place(&[(4.0, -44.0), (34.0, -6.0), (4.0, -6.0)]), color);
}

fn sea(c: &Canvas, spec: &SceneSpec, t: f64) {
    let (w, h) = (c.w, c.h);
    let (hue, accent) = (spec.hue, spec.accent);
    let progress = t / spec.duration;
    sky(
        c,
        hsl(hue, 55.0, 22.0 + progress * 20.0, 1.0),
        hsl(accent, 75.0, 62.0 + progress * 8.0, 1.0),
        0.62,
    );
    let sun_y = h * (0.62 - progress * 0.32);
    c.radial(
        w * 0.68,
        sun_y,
        h * 0.22,
        &[
            (0.0, hsl(accent + 20.0, 95.0, 88.0, 1.0)),
            (0.25, hsl(accent + 10.0, 90.0, 70.0, 0.9)),
            (1.0, hsl(accent, 90.0, 60.0, 0.0)),
        ],
    );
    c.rect_down(
        0.0,
        h * 0.6,
        w,
        h * 0.4,
        (h * 0.6, h),
        &[(0.0, hsl(hue, 50.0, 38.0, 1.0)), (1.0, hsl(hue, 60.0, 14.0, 1.0))],
    );
    // The sun's path on the water.
    for index in 0..14 {
        let index = f64::from(index);
        let y = h * 0.63 + index * h * 0.025;
        let width = (w * 0.05 + index * w * 0.01) * (0.6 + 0.4 * (t * 2.0 + index).sin());
        c.rect(
            w * 0.68 - width / 2.0,
            y,
            width,
            h * 0.006,
            hsl(accent + 15.0, 90.0, 80.0, 0.35 - index * 0.02),
        );
    }
    wave(c, h * 0.7, h * 0.012, w * 0.5, t * 0.8, hsl(hue, 45.0, 30.0, 0.7));
    let bob = (t * 1.6).sin() * h * 0.008;
    boat(
        c,
        w * (0.1 + progress * 0.5),
        h * 0.72 + bob,
        w / 1280.0,
        (t * 1.6 + 1.0).sin() * 0.05,
        hsl(hue, 40.0, 10.0, 1.0),
    );
    wave(
        c,
        h * 0.78,
        h * 0.018,
        w * 0.42,
        t * 1.2 + 2.0,
        hsl(hue, 50.0, 24.0, 0.85),
    );
    wave(
        c,
        h * 0.88,
        h * 0.024,
        w * 0.33,
        t * 1.6 + 4.0,
        hsl(hue, 55.0, 17.0, 0.95),
    );
}

fn lighthouse(c: &Canvas, spec: &SceneSpec, t: f64) {
    let (w, h) = (c.w, c.h);
    let (hue, accent) = (spec.hue, spec.accent);
    sky(c, hsl(hue, 60.0, 6.0, 1.0), hsl(hue, 50.0, 24.0, 1.0), 0.75);
    stars(c, spec.seed, t, 140, 0.7);
    c.circle(w * 0.18, h * 0.2, h * 0.05, hsl(accent, 30.0, 92.0, 0.9));
    // The headland and the tower.
    let mut headland = quadratic((w * 0.55, h), (w * 0.62, h * 0.66), (w * 0.78, h * 0.64), 24);
    headland.push((w, h * 0.62));
    c.fill_under(&headland, h, |_| hsl(hue, 40.0, 8.0, 1.0));
    let tower_x = w * 0.76;
    c.polygon(
        &[
            (tower_x - w * 0.022, h * 0.65),
            (tower_x + w * 0.022, h * 0.65),
            (tower_x + w * 0.013, h * 0.36),
            (tower_x - w * 0.013, h * 0.36),
        ],
        hsl(0.0, 0.0, 88.0, 1.0),
    );
    for band in 0..3 {
        c.rect(
            tower_x - w * 0.02,
            h * (0.42 + f64::from(band) * 0.08),
            w * 0.04,
            h * 0.025,
            hsl(accent, 70.0, 45.0, 1.0),
        );
    }
    // The beam turning.
    let angle = t * 1.1;
    let lamp = (tower_x, h * 0.34);
    let reach = w * 0.9;
    let spread = 0.12;
    let facing = angle.cos();
    let far = lamp.0 + reach * facing;
    let open = reach * spread * (angle.sin() + 0.2).abs();
    if far != lamp.0 {
        let mut mesh = Mesh::default();
        mesh.colored_vertex(c.at(lamp.0, lamp.1), hsl(accent + 20.0, 100.0, 85.0, 0.75).color32());
        let end = hsl(accent + 20.0, 100.0, 85.0, 0.0).color32();
        mesh.colored_vertex(c.at(far, lamp.1 - open), end);
        mesh.colored_vertex(c.at(far, lamp.1 + open), end);
        mesh.add_triangle(0, 1, 2);
        c.painter().add(Shape::mesh(mesh));
    }
    c.circle(lamp.0, lamp.1, h * 0.018, hsl(accent + 20.0, 100.0, 85.0, 1.0));
    wave(c, h * 0.82, h * 0.016, w * 0.4, t * 1.3, hsl(hue, 50.0, 12.0, 0.95));
    wave(c, h * 0.9, h * 0.02, w * 0.3, t * 1.8 + 3.0, hsl(hue, 55.0, 7.0, 1.0));
}

/// A line of hills from 14 seeded heights, moved left by `offset`, filled to the bottom.
#[allow(clippy::too_many_arguments)]
fn ridge(c: &Canvas, seed: u32, base: f64, height: f64, fill: Rgba, offset: f64) {
    let w = c.w;
    let mut random = Seeded::new(seed);
    let points: Vec<f64> = (0..14).map(|_| random.next()).collect();
    let mut line = Vec::new();
    let mut x = -w;
    while x <= w * 2.0 {
        let index = (((((x + offset) / (w / 12.0)) % 14.0) + 14.0).floor() as i64).rem_euclid(14) as usize;
        line.push((x, base - points[index] * height));
        x += w / 12.0;
    }
    c.fill_under(&line, c.h, |_| fill);
}

fn train(c: &Canvas, spec: &SceneSpec, t: f64) {
    let (w, h) = (c.w, c.h);
    let (hue, accent) = (spec.hue, spec.accent);
    sky(c, hsl(hue, 55.0, 40.0, 1.0), hsl(accent, 80.0, 72.0, 1.0), 0.7);
    c.circle(w * 0.3, h * 0.38, h * 0.08, hsl(accent + 10.0, 90.0, 85.0, 0.9));
    ridge(
        c,
        spec.seed,
        h * 0.55,
        h * 0.18,
        hsl(hue, 25.0, 50.0, 1.0),
        (t * w) / 60.0,
    );
    ridge(
        c,
        spec.seed.wrapping_add(1),
        h * 0.64,
        h * 0.12,
        hsl(accent, 35.0, 42.0, 1.0),
        (t * w) / 25.0,
    );
    c.rect(0.0, h * 0.7, w, h * 0.3, hsl(accent, 40.0, 30.0, 1.0));
    // The line and the train going by.
    c.rect(0.0, h * 0.715, w, h * 0.008, hsl(hue, 20.0, 18.0, 1.0));
    let length = w * 0.75;
    let lap = 14.0;
    let x = ((t % lap) / lap) * (w + length * 1.4) - length * 1.2;
    for car in 0..6 {
        let left = x + f64::from(car) * (length / 6.0);
        let body = if car == 5 {
            hsl(hue, 50.0, 30.0, 1.0)
        } else {
            hsl(hue + 180.0, 25.0, 82.0, 1.0)
        };
        c.rect(left, h * 0.655, length / 6.0 - w * 0.006, h * 0.058, body);
        for pane in 0..4 {
            c.rect(
                left + (f64::from(pane) + 0.5) * (length / 26.0),
                h * 0.668,
                length / 40.0,
                h * 0.014,
                hsl(accent + 20.0, 90.0, 70.0, 0.9),
            );
        }
    }
    // Poles rushing by in front.
    let pole_color = hsl(hue, 30.0, 14.0, 1.0);
    let spacing = w * 0.3;
    let shift = (t * w * 0.45) % spacing;
    for pole in -1..5 {
        let left = f64::from(pole) * spacing - shift;
        c.rect(left, h * 0.42, w * 0.008, h * 0.58, pole_color);
        c.rect(left - w * 0.02, h * 0.45, w * 0.048, h * 0.008, pole_color);
    }
    let width = (h / 400.0).max(1.0);
    for pole in -1..5 {
        let left = f64::from(pole) * spacing - shift;
        let wire = quadratic(
            (left, h * 0.455),
            (left + spacing / 2.0, h * 0.5),
            (left + spacing, h * 0.455),
            16,
        );
        c.line(&wire, width, hsl(hue, 30.0, 14.0, 0.7));
    }
}

fn peaks(c: &Canvas, spec: &SceneSpec, t: f64) {
    let (w, h) = (c.w, c.h);
    let (hue, accent) = (spec.hue, spec.accent);
    let progress = t / spec.duration;
    sky(
        c,
        hsl(hue + progress * 40.0, 50.0, 30.0 + progress * 10.0, 1.0),
        hsl(accent, 60.0, 75.0, 1.0),
        0.8,
    );
    ridge(c, spec.seed, h * 0.5, h * 0.3, hsl(hue, 20.0, 70.0, 1.0), t * 4.0);
    ridge(
        c,
        spec.seed.wrapping_add(7),
        h * 0.66,
        h * 0.25,
        hsl(hue, 25.0, 45.0, 1.0),
        t * 9.0,
    );
    ridge(
        c,
        spec.seed.wrapping_add(13),
        h * 0.84,
        h * 0.2,
        hsl(hue, 30.0, 22.0, 1.0),
        t * 18.0,
    );
    // Snow falling.
    let mut random = Seeded::new(spec.seed.wrapping_add(99));
    for flake in 0..160 {
        let speed = 0.04 + random.next() * 0.08;
        let start_x = random.next();
        let start_y = random.next();
        let size = (1.0 + random.next() * 2.5) * (w / 1280.0);
        let y = ((start_y + t * speed) % 1.0) * h;
        let x = ((start_x + (t * 0.8 + f64::from(flake)).sin() * 0.01 + t * 0.006) % 1.0) * w;
        c.circle(x, y, size, rgba(255.0, 255.0, 255.0, 0.85));
    }
}

fn aurora(c: &Canvas, spec: &SceneSpec, t: f64) {
    let (w, h) = (c.w, c.h);
    let (hue, accent) = (spec.hue, spec.accent);
    sky(c, hsl(hue, 60.0, 5.0, 1.0), hsl(hue, 45.0, 16.0, 1.0), 0.7);
    stars(c, spec.seed, t, 180, 0.65);
    // Curtains swaying, each strip the curtain's gradient at the strip's strength.
    let step = js_round(w / 200.0).max(4.0);
    for curtain in 0..3 {
        let curtain = f64::from(curtain);
        let tint = accent + curtain * 25.0;
        let mut x = 0.0;
        while x < w {
            let sway = (x / (w * 0.18) + t * (0.3 + curtain * 0.1) + curtain).sin() * h * 0.06;
            let top = h * (0.12 + curtain * 0.06) + sway;
            let bright = 0.18 + 0.14 * (x / (w * 0.05) + t * 1.4 + curtain * 2.0).sin();
            if bright > 0.0 {
                c.rect_down(
                    x,
                    top,
                    step,
                    h * 0.32,
                    (top, top + h * 0.32),
                    &[
                        (0.0, hsl(tint, 90.0, 60.0, 0.0)),
                        (0.7, hsl(tint, 90.0, 60.0, bright)),
                        (1.0, hsl(tint, 90.0, 60.0, 0.0)),
                    ],
                );
            }
            x += step;
        }
    }
    // The fjord's walls and their mirror in still water.
    c.fill_under(
        &[
            (0.0, h * 0.68),
            (w * 0.15, h * 0.48),
            (w * 0.32, h * 0.62),
            (w * 0.42, h * 0.68),
            (w * 0.62, h * 0.68),
            (w * 0.8, h * 0.5),
            (w, h * 0.58),
        ],
        h * 0.68,
        |_| hsl(hue, 40.0, 6.0, 1.0),
    );
    c.rect_down(
        0.0,
        h * 0.68,
        w,
        h * 0.32,
        (h * 0.68, h),
        &[(0.0, hsl(accent, 45.0, 18.0, 1.0)), (1.0, hsl(hue, 50.0, 4.0, 1.0))],
    );
    for line in 0..9 {
        let line = f64::from(line);
        c.rect(
            w * 0.1 + (t + line).sin() * w * 0.02,
            h * (0.72 + line * 0.03),
            w * 0.8,
            h * 0.004,
            hsl(accent, 80.0, 60.0, 0.08),
        );
    }
}

fn rain(c: &Canvas, spec: &SceneSpec, t: f64) {
    let (w, h) = (c.w, c.h);
    let (hue, accent) = (spec.hue, spec.accent);
    sky(c, hsl(hue, 35.0, 8.0, 1.0), hsl(hue, 30.0, 22.0, 1.0), 1.0);
    let mut random = Seeded::new(spec.seed);
    // The skyline, its windows going on and off.
    let mut x = 0.0;
    while x < w {
        let width = w * (0.05 + random.next() * 0.07);
        let height = h * (0.25 + random.next() * 0.4);
        c.rect(
            x,
            h * 0.8 - height,
            width,
            height,
            hsl(hue, 20.0, 10.0 + random.next() * 6.0, 1.0),
        );
        let mut row = 0.0;
        while row < height / (h * 0.04) - 1.0 {
            let mut column = 0.0;
            while column < width / (w * 0.016) - 1.0 {
                let lit = (row * 12.9 + column * 78.2 + x + (t / 3.0).floor() * 0.7).sin() > 0.35;
                if lit {
                    c.rect(
                        x + w * 0.006 + column * w * 0.016,
                        h * 0.8 - height + h * 0.015 + row * h * 0.04,
                        w * 0.007,
                        h * 0.018,
                        hsl(accent, 80.0, 70.0, 0.8),
                    );
                }
                column += 1.0;
            }
            row += 1.0;
        }
        x += width + w * 0.008;
    }
    c.rect_down(
        0.0,
        h * 0.8,
        w,
        h * 0.2,
        (h * 0.8, h),
        &[(0.0, hsl(hue, 25.0, 14.0, 1.0)), (1.0, hsl(hue, 30.0, 6.0, 1.0))],
    );
    for glow in 0..8 {
        let glow = f64::from(glow);
        c.rect(
            glow * w * 0.13,
            h * 0.82,
            w * 0.05,
            h * 0.18,
            hsl(accent, 80.0, 65.0, 0.06 + 0.03 * (t * 2.0 + glow).sin()),
        );
    }
    // Rain.
    let width = (w / 1280.0).max(1.0);
    for _ in 0..220 {
        let speed = 0.9 + random.next() * 0.6;
        let drop_x = random.next() * w;
        let drop_y = ((random.next() + t * speed) % 1.0) * h;
        c.line(
            &[(drop_x, drop_y), (drop_x - w * 0.004, drop_y + h * 0.035)],
            width,
            rgba(200.0, 220.0, 255.0, 0.35),
        );
    }
}

/// The water level at an hour of the day the clip shows, from 0 (low water) to 1 (high water).
pub fn level_at(hours: f64) -> f64 {
    0.5 + 0.42 * ((hours / 12.42) * PI * 2.0).sin() + 0.06 * ((hours / 6.2) * PI * 2.0).sin()
}

fn tide(c: &Canvas, spec: &SceneSpec, t: f64) {
    let (w, h) = (c.w, c.h);
    let (hue, accent) = (spec.hue, spec.accent);
    sky(c, hsl(hue, 35.0, 14.0, 1.0), hsl(hue, 40.0, 8.0, 1.0), 1.0);
    let left = w * 0.08;
    let right = w * 0.92;
    let top = h * 0.16;
    let bottom = h * 0.78;
    let grid = (w / 1280.0).max(1.0);
    let label = font(js_round(h * 0.03), true);
    let mut hour = 0;
    while hour <= 24 {
        let x = left + ((right - left) * f64::from(hour)) / 24.0;
        c.line(&[(x, top), (x, bottom)], grid, hsl(hue, 25.0, 40.0, 0.5));
        c.text(
            x - w * 0.02,
            bottom + h * 0.05,
            &format!("{hour:02}:00"),
            &label,
            hsl(hue, 20.0, 70.0, 0.8),
        );
        hour += 3;
    }
    let now = (t / spec.duration) * 24.0;
    let curve = |hours: f64| bottom - level_at(hours) * (bottom - top);
    let x_of = |hours: f64| left + ((right - left) * hours) / 24.0;
    let mut line = Vec::new();
    let mut hours = 0.0;
    while hours <= now {
        line.push((x_of(hours), curve(hours)));
        hours += 0.1;
    }
    let mut filled = vec![(left, bottom)];
    filled.extend(line.iter().copied());
    filled.push((x_of(now), bottom));
    let stops = [
        (0.0, hsl(accent, 70.0, 55.0, 0.55)),
        (1.0, hsl(accent, 70.0, 35.0, 0.1)),
    ];
    c.fill_under(&filled, bottom, |y| stop_at(&stops, (y - top) / (bottom - top)));
    c.line(&line, (w / 500.0).max(2.0), hsl(accent, 80.0, 70.0, 1.0));
    let mark = hsl(accent, 90.0, 80.0, 1.0);
    c.circle(x_of(now), curve(now), h * 0.014, mark);
    let level = level_at(now);
    c.text(
        left,
        top - h * 0.04,
        &format!("{:.2} m", level * 4.2),
        &font(js_round(h * 0.045), false),
        mark,
    );
    // The harbor wall with the water against it.
    wave(
        c,
        h * (0.98 - level * 0.08),
        h * 0.008,
        w * 0.3,
        t * 1.4,
        hsl(hue, 50.0, 30.0, 0.9),
    );
}

fn islands(c: &Canvas, spec: &SceneSpec, t: f64) {
    let (w, h) = (c.w, c.h);
    let (hue, accent) = (spec.hue, spec.accent);
    c.radial(
        w * 0.5,
        h * 0.5,
        w * 0.7,
        &[(0.0, hsl(hue, 55.0, 42.0, 1.0)), (1.0, hsl(hue, 60.0, 22.0, 1.0))],
    );
    let mut random = Seeded::new(spec.seed);
    let mut places = Vec::new();
    for _ in 0..6 {
        let cx = w * (0.12 + random.next() * 0.76);
        let cy = h * (0.15 + random.next() * 0.7);
        let radius = h * (0.05 + random.next() * 0.08);
        places.push((cx, cy));
        let bumps: Vec<f64> = (0..10).map(|_| 0.7 + random.next() * 0.5).collect();
        for (ring, color) in [
            (1.25, hsl(hue - 10.0, 50.0, 62.0, 0.5)),
            (1.0, hsl(accent, 35.0, 58.0, 1.0)),
            (0.6, hsl(accent + 20.0, 40.0, 42.0, 1.0)),
        ] {
            let points: Vec<(f64, f64)> = (0..10)
                .map(|step| {
                    let angle = (f64::from(step) / 10.0) * PI * 2.0;
                    let reach = radius * ring * bumps[step as usize];
                    (cx + angle.cos() * reach, cy + angle.sin() * reach * 0.8)
                })
                .collect();
            c.fan((cx, cy), &points, color);
        }
    }
    // The ferry going round the islands, its wake behind it.
    let lap = spec.duration / 2.0;
    let along = ((t % lap) / lap) * places.len() as f64;
    let from = places[(along.floor() as usize) % places.len()];
    let to = places[(along.floor() as usize + 1) % places.len()];
    let part = along % 1.0;
    let width = (w / 900.0).max(1.0);
    let mut route = places.clone();
    route.push(places[0]);
    c.dashed(&route, width, rgba(255.0, 255.0, 255.0, 0.5), w * 0.008);
    let ferry_x = from.0 + (to.0 - from.0) * part;
    let ferry_y = from.1 + (to.1 - from.1) * part;
    c.circle(ferry_x, ferry_y, h * 0.014, hsl(accent + 30.0, 90.0, 85.0, 1.0));
    for ring in 1..=3 {
        let radius = h * 0.014 + ((t * 20.0 + f64::from(ring) * 12.0) % 36.0) * (h / 720.0);
        c.ring(ferry_x, ferry_y, radius, width, hsl(0.0, 0.0, 100.0, 0.25));
    }
}

/// A wind turbine standing at `x`, `y`, its blades turned to `angle`.
fn turbine(c: &Canvas, x: f64, y: f64, size: f64, angle: f64, color: Rgba) {
    c.rect(x - size * 0.03, y, size * 0.06, size * 1.4, color);
    for blade in 0..3 {
        let turn = angle + f64::from(blade + 1) * (PI * 2.0) / 3.0;
        let (sin, cos) = turn.sin_cos();
        let place = |(px, py): (f64, f64)| (x + px * cos - py * sin, y + px * sin + py * cos);
        c.polygon(
            &[
                place((0.0, 0.0)),
                place((size * 0.05, -size * 0.1)),
                place((0.0, -size * 0.75)),
                place((-size * 0.04, -size * 0.1)),
            ],
            color,
        );
    }
}

fn fields(c: &Canvas, spec: &SceneSpec, t: f64) {
    let (w, h) = (c.w, c.h);
    let (hue, accent) = (spec.hue, spec.accent);
    sky(c, hsl(hue, 60.0, 55.0, 1.0), hsl(hue, 50.0, 82.0, 1.0), 0.7);
    let mut random = Seeded::new(spec.seed);
    for cloud in 0..5 {
        let cloud = f64::from(cloud);
        let x = ((random.next() + t * 0.01 * (1.0 + cloud * 0.2)) % 1.3) * w - w * 0.15;
        let y = h * (0.1 + random.next() * 0.25);
        for puff in 0..4 {
            let puff = f64::from(puff);
            c.circle(
                x + puff * w * 0.03,
                y + puff.sin() * h * 0.01,
                h * (0.03 + (puff % 2.0) * 0.015),
                rgba(255.0, 255.0, 255.0, 0.75),
            );
        }
    }
    wave(c, h * 0.6, h * 0.04, w * 0.9, 0.5, hsl(accent, 35.0, 50.0, 1.0));
    for index in 0..4 {
        let index = f64::from(index);
        turbine(
            c,
            w * (0.2 + index * 0.2),
            h * (0.42 + (index % 2.0) * 0.05),
            h * 0.16,
            t * (1.2 + index * 0.15) + index,
            hsl(0.0, 0.0, 96.0, 1.0),
        );
    }
    wave(c, h * 0.72, h * 0.05, w * 0.7, 2.0, hsl(accent, 45.0, 38.0, 1.0));
    wave(c, h * 0.86, h * 0.04, w * 0.5, 4.0, hsl(accent + 10.0, 50.0, 26.0, 1.0));
    // Grass moving in the wind along the near hill.
    let width = (w / 1000.0).max(1.0);
    for blade in 0..140 {
        let blade = f64::from(blade);
        let x = (blade / 140.0) * w;
        let y = h * 0.9 + blade.sin() * h * 0.02;
        c.line(
            &[(x, y), (x + (t * 2.0 + blade * 0.3).sin() * w * 0.006, y - h * 0.04)],
            width,
            hsl(accent + 15.0, 55.0, 20.0, 1.0),
        );
    }
}

/// How long the title card stands at the start, in seconds, fading over its last second.
pub const CARD: f64 = 3.0;

/// One frame of a clip on `rect`: its scene at second `t`, the channel's mark and the time in its
/// corners, and, with `card`, the title card over the first seconds.
pub fn draw_frame(painter: &Painter, rect: Rect, spec: &SceneSpec, t: f64, card: bool) {
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    let c = Canvas::new(&painter, rect);
    let (w, h) = (c.w, c.h);
    match spec.kind {
        SceneKind::Sea => sea(&c, spec, t),
        SceneKind::Lighthouse => lighthouse(&c, spec, t),
        SceneKind::Train => train(&c, spec, t),
        SceneKind::Peaks => peaks(&c, spec, t),
        SceneKind::Aurora => aurora(&c, spec, t),
        SceneKind::Rain => rain(&c, spec, t),
        SceneKind::Tide => tide(&c, spec, t),
        SceneKind::Islands => islands(&c, spec, t),
        SceneKind::Fields => fields(&c, spec, t),
    }
    let small = js_round(h * 0.03);
    let faint = rgba(255.0, 255.0, 255.0, 0.55);
    let sans = font(small, false);
    c.text(
        w - c.measure(&spec.channel, &sans) - w * 0.03,
        h - h * 0.045,
        &spec.channel,
        &sans,
        faint,
    );
    let mono = font(small, true);
    let clock = format!("{}:{:02}", (t / 60.0).floor(), (t % 60.0).floor());
    c.text(w - c.measure(&clock, &mono) - w * 0.03, h * 0.07, &clock, &mono, faint);
    if !card || t >= CARD {
        return;
    }
    let alpha = (CARD - t).min(1.0);
    c.rect(0.0, h * 0.34, w, h * 0.32, rgba(0.0, 0.0, 0.0, 0.45 * alpha));
    let big = font(js_round(h * 0.06), false);
    let lines: Vec<String> = c
        .wrap(spec.title.split(' '), &big, w * 0.84)
        .into_iter()
        .take(2)
        .collect();
    for (index, line) in lines.iter().enumerate() {
        c.text(
            w * 0.08,
            h * 0.45 + index as f64 * h * 0.08,
            line,
            &big,
            rgba(255.0, 255.0, 255.0, alpha),
        );
    }
    c.text(
        w * 0.08,
        h * 0.45 + lines.len() as f64 * h * 0.08 + h * 0.01,
        &spec.channel,
        &font(js_round(h * 0.035), false),
        rgba(255.0, 255.0, 255.0, 0.8 * alpha),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_is_the_webs() {
        // The web's `seeded(11)`, `seeded(348)` and `seeded(4294967295)`, their first numbers.
        let mut one = Seeded::new(11);
        assert_eq!(one.next(), 0.5115870486479253);
        assert_eq!(one.next(), 0.5299464082345366);
        let mut two = Seeded::new(348);
        assert_eq!(two.next(), 0.9318338285665959);
        let mut three = Seeded::new(u32::MAX);
        assert_eq!(three.next(), 0.8964226141106337);
    }

    #[test]
    fn hsl_is_the_css_one() {
        assert_eq!(hsl(0.0, 100.0, 50.0, 1.0).color32(), Color32::from_rgb(255, 0, 0));
        assert_eq!(hsl(120.0, 100.0, 25.0, 1.0).color32(), Color32::from_rgb(0, 128, 0));
        assert_eq!(hsl(480.0, 100.0, 50.0, 1.0).color32(), Color32::from_rgb(0, 255, 0));
        assert_eq!(hsl(212.0, 55.0, 22.0, 1.0).color32(), Color32::from_rgb(25, 54, 87));
    }
}
