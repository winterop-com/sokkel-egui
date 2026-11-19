//! A picture of what is playing, after sokkel-ui's `media/SoundVisualizer` and the shapes of its
//! `media/sound`: a spectrum of bars, an area, a wave or a ring, in one color token or along a ramp
//! of them. egui hears no audio, so the app hands it each frame's levels (or samples, for a wave):
//! from its own analyser through [`band_levels`] and [`settle`], or a still frame.

use std::f32::consts::PI;

use egui::epaint::{TessellationOptions, Tessellator};
use egui::{Color32, Mesh, Painter, Pos2, Rect, Response, Role, Sense, Shape, Stroke, Ui, Vec2, Widget, pos2, vec2};

use crate::tokens;

/// How the sound is drawn: `Spectrum` as bars, `Area` as the bars' tops joined into a filled curve,
/// `Wave` as the waveform itself traced like an oscilloscope, and `Ring` as bars standing out from
/// a circle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SoundKind {
    #[default]
    Spectrum,
    Area,
    Wave,
    Ring,
}

pub const SOUND_KINDS: [SoundKind; 4] = [SoundKind::Spectrum, SoundKind::Area, SoundKind::Wave, SoundKind::Ring];

/// Where a spectrum's bars or an area's curve stand: on the floor, or about the middle line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SoundAlign {
    #[default]
    Bottom,
    Center,
}

/// What places a bar on its ramp: its `Height`, the ramp running up the box so a tall bar passes
/// through every color on its way up, or its `Level`, each bar in the one color its loudness
/// reaches. An area and a wave are colored by height either way, a wave by how far it swings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SoundRampBy {
    #[default]
    Height,
    Level,
}

/// The named ramps, each made of the theme's tokens, quiet first: `heat` from the warning ink to
/// the critical one, `cool` from teal through blue to violet, `lagoon` from green through teal to
/// violet, `ink` from the muted ink to the text's, and `primary` from the muted ink into the
/// theme's primary.
pub const SOUND_RAMPS: [(&str, &[&str]); 5] = [
    ("heat", &["warning", "critical"]),
    ("cool", &["kind-teal", "kind-blue", "kind-violet"]),
    ("lagoon", &["kind-green", "kind-teal", "kind-violet"]),
    ("ink", &["muted-foreground", "foreground"]),
    ("primary", &["muted-foreground", "primary"]),
];

/// A named ramp's stops, as token names.
pub fn sound_ramp(name: &str) -> Option<&'static [&'static str]> {
    SOUND_RAMPS
        .iter()
        .find(|(named, _)| *named == name)
        .map(|(_, stops)| *stops)
}

/// The frequencies the bands are spread over, in hertz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundRange {
    pub low: f32,
    pub high: f32,
}

/// Where music has most of what a picture can show.
pub const MUSIC_RANGE: SoundRange = SoundRange {
    low: 40.0,
    high: 14_000.0,
};
/// Where a voice has its fundamentals and formants.
pub const VOICE_RANGE: SoundRange = SoundRange {
    low: 80.0,
    high: 8000.0,
};

/// One frame of a sixty-per-second screen, in milliseconds, the step the rates are given in.
const FRAME_MS: f32 = 1000.0 / 60.0;
/// How strong a backdrop is drawn, against the full drawing.
const BACKDROP: f32 = 0.32;
/// How strong a silent band is drawn; a band grows to full strength with its level.
const QUIET_INK: f32 = 0.55;
/// The shortest a bar or spoke is drawn, so a silent one still shows as a tick.
const TICK: f32 = 2.0;
/// How many colors a ramp is mixed into.
const SHADES: usize = 24;
/// How strong a mirror's reflection starts at its baseline, fading to nothing at the foot.
const REFLECTION: f32 = 0.45;
/// The room between the bars and their reflection.
const MIRROR_GAP: f32 = 2.0;
/// How much of a reflected spectrum's height the reflection takes, below the baseline.
pub const MIRROR_SHARE: f32 = 0.3;
/// A ring's middle, as a share of its inner circle's radius, across.
const MIDDLE: f32 = 1.6;

/// A live or still picture of sound, filling the box it is given. It draws in one color token
/// (`primary` by default), or along a `ramp` of tokens by height or by level; a spectrum or an
/// area standing on the floor can stand over its faded reflection (`mirror`), and `backdrop` draws
/// it faint behind text. A ring's middle is [`sound_middle`] of the box, for a cover.
///
/// The app passes `levels` (loudness per band, low to high, 0 to 1) for a spectrum, an area or a
/// ring, and `samples` (-1 to 1) for a wave, each frame while it plays; with none, it draws at
/// rest: a row of ticks, a floor, a flat line or a circle.
#[must_use = "add it with `ui.add(...)`, or paint it with `paint`"]
pub struct SoundVisualizer<'a> {
    levels: &'a [f32],
    samples: &'a [f32],
    kind: SoundKind,
    align: SoundAlign,
    bands: Option<usize>,
    floor: f32,
    ceiling: f32,
    bar_width: Option<f32>,
    color: &'a str,
    ramp: Option<&'a [&'a str]>,
    ramp_by: SoundRampBy,
    mirror: bool,
    backdrop: bool,
    label: String,
    height: Option<f32>,
}

impl Default for SoundVisualizer<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> SoundVisualizer<'a> {
    pub fn new() -> Self {
        Self {
            levels: &[],
            samples: &[],
            kind: SoundKind::Spectrum,
            align: SoundAlign::Bottom,
            bands: None,
            floor: 0.0,
            ceiling: 1.0,
            bar_width: None,
            color: "primary",
            ramp: None,
            ramp_by: SoundRampBy::Height,
            mirror: false,
            backdrop: false,
            label: "Sound".into(),
            height: None,
        }
    }

    /// Loudness per band from low to high, each 0 to 1, for a spectrum, an area or a ring; stretched
    /// to the bands drawn.
    pub fn levels(mut self, levels: &'a [f32]) -> Self {
        self.levels = levels;
        self
    }

    /// The waveform, each sample -1 to 1, for a wave.
    pub fn samples(mut self, samples: &'a [f32]) -> Self {
        self.samples = samples;
        self
    }

    pub fn kind(mut self, kind: SoundKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn align(mut self, align: SoundAlign) -> Self {
        self.align = align;
        self
    }

    /// How many bands; by default as many as the box holds.
    pub fn bands(mut self, bands: usize) -> Self {
        self.bands = Some(bands);
        self
    }

    /// The least a band stands, as a share of its reach, so a quiet band still shows.
    pub fn floor(mut self, floor: f32) -> Self {
        self.floor = floor;
        self
    }

    /// The most a band stands, as a share of its reach.
    pub fn ceiling(mut self, ceiling: f32) -> Self {
        self.ceiling = ceiling;
        self
    }

    /// The widest a bar is drawn; a row of narrower bars is centered.
    pub fn bar_width(mut self, width: f32) -> Self {
        self.bar_width = Some(width);
        self
    }

    /// The color token the drawing is inked in, by name: `primary`, `chart-3`, `kind-teal`.
    pub fn color(mut self, token: &'a str) -> Self {
        self.color = token;
        self
    }

    /// The color tokens the drawing runs through from quiet to loud, in place of the one color:
    /// a named ramp's ([`sound_ramp`]) or stops of the app's own.
    pub fn ramp(mut self, stops: &'a [&'a str]) -> Self {
        self.ramp = Some(stops);
        self
    }

    pub fn ramp_by(mut self, by: SoundRampBy) -> Self {
        self.ramp_by = by;
        self
    }

    /// A reflection of a spectrum's bars or an area's curve below their baseline, fading toward the
    /// foot; for a drawing standing on the floor.
    pub fn mirror(mut self, mirror: bool) -> Self {
        self.mirror = mirror;
        self
    }

    /// Drawn faint, to stand behind text such as a player's title.
    pub fn backdrop(mut self, backdrop: bool) -> Self {
        self.backdrop = backdrop;
        self
    }

    /// What the picture is called, for a screen reader.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// How tall it is drawn; by default the height the `ui` has left.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Paint the picture into `rect`.
    pub fn paint(&self, painter: &Painter, rect: Rect) {
        let (width, height) = (rect.width(), rect.height());
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let t = tokens(painter.ctx());
        let ink = t.color_or(self.color, t.color("primary"));
        let stops: Vec<Color32> = self
            .ramp
            .unwrap_or(&[])
            .iter()
            .map(|stop| t.color_or(stop, ink))
            .collect();
        let shades = ramp_steps(&stops, if stops.len() == 1 { 1 } else { SHADES });
        let mirrored = self.mirror
            && self.align == SoundAlign::Bottom
            && matches!(self.kind, SoundKind::Spectrum | SoundKind::Area);
        let (base, room) = if mirrored { mirror_base(height) } else { (height, 0.0) };
        let strength = if self.backdrop { BACKDROP } else { 1.0 };
        let centered = self.kind == SoundKind::Wave || self.align == SoundAlign::Center;
        let (inner, reach) = ring_radii(width, height);
        // where along the ramp a place in the box is: up from the baseline, out from the middle
        // line, or out from a ring's circle
        let kind = self.kind;
        let along = move |p: Pos2| -> f32 {
            let share = if kind == SoundKind::Ring {
                ((p - pos2(width / 2.0, height / 2.0)).length() - inner) / reach.max(f32::EPSILON)
            } else if centered {
                (p.y - height / 2.0).abs() / (height / 2.0)
            } else {
                (base - p.y) / base.max(f32::EPSILON)
            };
            share.clamp(0.0, 1.0)
        };
        let shade_at = |share: f32| -> Color32 {
            match shades.len() {
                0 => ink,
                1 => shades[0],
                n => {
                    let at = share * (n - 1) as f32;
                    let below = (at.floor() as usize).min(n - 2);
                    shades[below].lerp_to_gamma(shades[below + 1], at - below as f32)
                }
            }
        };
        // the heights a filled column is cut at, so a gradient up it passes through every shade,
        // and at the top of what a mirror reflects
        let mut cuts: Vec<f32> = Vec::new();
        if shades.len() > 1 && kind != SoundKind::Ring {
            for step in 0..shades.len() {
                let share = step as f32 / (shades.len() - 1) as f32;
                if centered {
                    cuts.push(height / 2.0 - share * height / 2.0);
                    cuts.push(height / 2.0 + share * height / 2.0);
                } else {
                    cuts.push(base - share * base);
                }
            }
        }
        let span = room.min(base);
        if mirrored {
            cuts.push(base - span);
        }
        cuts.sort_by(f32::total_cmp);

        let mut mesh = Mesh::default();
        // one physical pixel, the width an edge fades out over
        let feather = 1.0 / painter.ctx().pixels_per_point();
        let by_level = self.ramp_by == SoundRampBy::Level && shades.len() > 1;
        let tessellator = || {
            let ppp = painter.ctx().pixels_per_point();
            let size = painter.ctx().fonts(|fonts| fonts.font_image_size());
            Tessellator::new(ppp, TessellationOptions::default(), size, vec![])
        };
        match self.kind {
            SoundKind::Wave => {
                let line = (height * 0.01).clamp(1.5, 3.0);
                let points: Vec<Pos2> = wave_points(self.samples, width, (height - line).max(0.0))
                    .into_iter()
                    .map(|p| pos2(p.x, p.y + line / 2.0))
                    .collect();
                let from = mesh.vertices.len();
                tessellator().tessellate_shape(Shape::line(points, Stroke::new(line, Color32::WHITE)), &mut mesh);
                tint(&mut mesh, from, |p| shade_at(along(p)).gamma_multiply(strength));
            }
            SoundKind::Spectrum | SoundKind::Area | SoundKind::Ring => {
                let wanted = self.bands.unwrap_or_else(|| auto_bands(self.kind, width, height));
                let fitted = if self.levels.len() == wanted {
                    self.levels.to_vec()
                } else if self.levels.is_empty() {
                    vec![0.0; wanted]
                } else {
                    resample(self.levels, wanted)
                };
                let span_scale = self.ceiling - self.floor;
                let scaled: Vec<f32> = fitted
                    .iter()
                    .map(|level| self.floor + span_scale * level.clamp(0.0, 1.0))
                    .collect();
                let corner = t.length("radius");
                match self.kind {
                    SoundKind::Spectrum => {
                        let layout = bar_layout(width, scaled.len(), self.bar_width.unwrap_or(f32::INFINITY));
                        let center = self.align == SoundAlign::Center;
                        let least = base.min(TICK);
                        for (index, level) in scaled.iter().enumerate() {
                            let tall = (level * base).max(least);
                            let x = layout.offset + index as f32 * (layout.bar + layout.gap);
                            let y = if center { (base - tall) / 2.0 } else { base - tall };
                            let loud = fitted[index].clamp(0.0, 1.0);
                            let alpha = strength * (QUIET_INK + (1.0 - QUIET_INK) * loud);
                            let round = corner.min(layout.bar / 2.0).min(tall / 2.0);
                            let from = mesh.vertices.len();
                            bar(&mut mesh, x, y, layout.bar, tall, round, !center, &cuts, feather);
                            if by_level {
                                let color = shades[ramp_index(loud, shades.len())].gamma_multiply(alpha);
                                tint(&mut mesh, from, |_| color);
                            } else {
                                tint(&mut mesh, from, |p| shade_at(along(p)).gamma_multiply(alpha));
                            }
                        }
                    }
                    SoundKind::Area => {
                        let center = self.align == SoundAlign::Center;
                        let middle = if center { height / 2.0 } else { base };
                        let reach = ((if center { height / 2.0 } else { base }) - TICK / 2.0).max(0.0);
                        let top = curve(
                            &area_points(&scaled, width, middle - TICK / 2.0, reach - TICK / 2.0),
                            width,
                        );
                        let under: Vec<Pos2> = top.iter().map(|p| pos2(p.x, 2.0 * middle - p.y)).collect();
                        let from = mesh.vertices.len();
                        for pair in 0..top.len().saturating_sub(1) {
                            let (a, b) = (top[pair], top[pair + 1]);
                            if b.x - a.x <= 0.0 {
                                continue;
                            }
                            let (foot_a, foot_b) = if center {
                                (under[pair].y, under[pair + 1].y)
                            } else {
                                (base, base)
                            };
                            column(&mut mesh, a.x, b.x, a.y, b.y, foot_a, foot_b, &cuts);
                        }
                        tint(&mut mesh, from, |p| shade_at(along(p)).gamma_multiply(strength * 0.34));
                        let line = (height * 0.008).clamp(1.5, 3.0);
                        let mut edges = vec![top];
                        if center {
                            edges.push(under);
                        }
                        for edge in edges {
                            let from = mesh.vertices.len();
                            tessellator()
                                .tessellate_shape(Shape::line(edge, Stroke::new(line, Color32::WHITE)), &mut mesh);
                            tint(&mut mesh, from, |p| shade_at(along(p)).gamma_multiply(strength));
                        }
                    }
                    SoundKind::Ring | SoundKind::Wave => {
                        if inner > 0.0 && !scaled.is_empty() {
                            let center = pos2(width / 2.0, height / 2.0);
                            let slot = PI * inner / scaled.len() as f32;
                            let thick = (slot * 0.55).clamp(1.0, 8.0);
                            let from = mesh.vertices.len();
                            tessellator().tessellate_shape(
                                Shape::circle_stroke(center, inner - thick, Stroke::new(1.0, Color32::WHITE)),
                                &mut mesh,
                            );
                            tint(&mut mesh, from, |_| shade_at(0.0).gamma_multiply(strength * 0.3));
                            let spokes = ring_spokes(&scaled, center, inner, reach - thick, TICK);
                            for (index, (foot, tip)) in spokes.into_iter().enumerate() {
                                let loud = fitted[index % fitted.len()].clamp(0.0, 1.0);
                                let alpha = strength * (QUIET_INK + (1.0 - QUIET_INK) * loud);
                                // cut along its length, so a ramp out from the circle shows on it
                                let points: Vec<Pos2> = (0..=8).map(|step| foot.lerp(tip, step as f32 / 8.0)).collect();
                                let from = mesh.vertices.len();
                                let mut tess = tessellator();
                                tess.tessellate_shape(
                                    Shape::line(points, Stroke::new(thick, Color32::WHITE)),
                                    &mut mesh,
                                );
                                if corner > 0.0 {
                                    // round ends, as `lineCap = 'round'`
                                    for end in [foot, tip] {
                                        tess.tessellate_shape(
                                            Shape::circle_filled(end, thick / 2.0, Color32::WHITE),
                                            &mut mesh,
                                        );
                                    }
                                }
                                if by_level {
                                    let color = shades[ramp_index(loud, shades.len())].gamma_multiply(alpha);
                                    tint(&mut mesh, from, |_| color);
                                } else {
                                    tint(&mut mesh, from, |p| shade_at(along(p)).gamma_multiply(alpha));
                                }
                            }
                        }
                    }
                }
            }
        }
        if mirrored && span > 0.0 {
            reflect(&mut mesh, base, span, room);
        }
        mesh.translate(rect.min.to_vec2());
        painter.with_clip_rect(rect).add(mesh);
    }
}

impl Widget for SoundVisualizer<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(
            ui.available_width(),
            self.height.unwrap_or_else(|| ui.available_height()),
        );
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(Role::Image, true, &self.label));
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect);
        }
        response
    }
}

/// Where a ring's middle is drawn in a box, a circle's bounds inside its inner circle: for a
/// cover or a play button.
pub fn sound_middle(rect: Rect) -> Rect {
    let (inner, _) = ring_radii(rect.width(), rect.height());
    let side = (inner * MIDDLE).floor();
    Rect::from_center_size(rect.center(), Vec2::splat(side))
}

/// Recolors the vertices from `from` on, each white at some alpha, by where they stand.
fn tint(mesh: &mut Mesh, from: usize, color: impl Fn(Pos2) -> Color32) {
    for vertex in &mut mesh.vertices[from..] {
        let alpha = vertex.color.a() as f32 / 255.0;
        vertex.color = color(vertex.pos).gamma_multiply(alpha);
    }
}

/// A bar of strips cut at `cuts`, its corners rounded by `round`: the top two, and the foot's too
/// unless it stands on the floor (`footed`). Its edges fade out over `feather`, a pixel, so its
/// sides and corners are smooth as the canvas draws them.
#[allow(clippy::too_many_arguments)]
fn bar(mesh: &mut Mesh, x: f32, y: f32, wide: f32, tall: f32, round: f32, footed: bool, cuts: &[f32], feather: f32) {
    let (top, bottom) = (y, y + tall);
    let mut levels = vec![top, bottom];
    let round = round.max(0.0);
    if round > 0.0 {
        for step in 1..8 {
            let rise = round * (1.0 - (step as f32 / 8.0 * PI / 2.0).cos());
            levels.push(top + rise);
            if !footed {
                levels.push(bottom - rise);
            }
        }
        levels.push(top + round);
        if !footed {
            levels.push(bottom - round);
        }
    }
    levels.extend(cuts.iter().copied().filter(|cut| *cut > top && *cut < bottom));
    levels.sort_by(f32::total_cmp);
    levels.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    let inset = |level: f32| -> f32 {
        if round <= 0.0 {
            0.0
        } else if level < top + round {
            let d = top + round - level;
            round - (round * round - d * d).max(0.0).sqrt()
        } else if !footed && level > bottom - round {
            let d = level - (bottom - round);
            round - (round * round - d * d).max(0.0).sqrt()
        } else {
            0.0
        }
    };
    let half = feather / 2.0;
    let clear = Color32::TRANSPARENT;
    // each row: the fringe outside the left edge, the left edge, the right edge, the fringe outside
    // the right; a clear row over the top, and under the foot unless it stands on the floor
    let mut rows: Vec<(f32, f32, bool)> = vec![(top - half, inset(top), false)];
    rows.extend(levels.iter().map(|level| (*level, inset(*level), true)));
    if !footed {
        rows.push((bottom + half, inset(bottom), false));
    }
    let first = mesh.vertices.len() as u32;
    for (level, side, solid) in &rows {
        let ink = if *solid { Color32::WHITE } else { clear };
        let left = (x + side + half).min(x + wide / 2.0);
        let right = (x + wide - side - half).max(x + wide / 2.0);
        mesh.colored_vertex(pos2(x + side - half, *level), clear);
        mesh.colored_vertex(pos2(left, *level), ink);
        mesh.colored_vertex(pos2(right, *level), ink);
        mesh.colored_vertex(pos2(x + wide - side + half, *level), clear);
    }
    for row in 0..rows.len().saturating_sub(1) as u32 {
        let a = first + row * 4;
        for lane in 0..3 {
            let (top_left, below_left) = (a + lane, a + 4 + lane);
            mesh.add_triangle(top_left, top_left + 1, below_left);
            mesh.add_triangle(top_left + 1, below_left + 1, below_left);
        }
    }
}

/// A column of an area from its sloping top down to its sloping foot, cut at `cuts`.
#[allow(clippy::too_many_arguments)]
fn column(mesh: &mut Mesh, x0: f32, x1: f32, top0: f32, top1: f32, foot0: f32, foot1: f32, cuts: &[f32]) {
    let first = mesh.vertices.len() as u32;
    let mut rows = vec![(top0, top1)];
    let (high, low) = (top0.max(top1), foot0.min(foot1));
    rows.extend(
        cuts.iter()
            .filter(|cut| **cut > high && **cut < low)
            .map(|cut| (*cut, *cut)),
    );
    rows.push((foot0, foot1));
    for (left, right) in &rows {
        mesh.colored_vertex(pos2(x0, *left), Color32::WHITE);
        mesh.colored_vertex(pos2(x1, *right), Color32::WHITE);
    }
    for row in 0..rows.len().saturating_sub(1) as u32 {
        let a = first + row * 2;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
}

/// Draws what stands within `span` above the baseline again below it, upside down past a small
/// gap, fading toward the foot.
fn reflect(mesh: &mut Mesh, base: f32, span: f32, room: f32) {
    let mut copy = Mesh::default();
    let top = base - span - 0.01;
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let corners = [triangle[0], triangle[1], triangle[2]].map(|i| mesh.vertices[i as usize]);
        let middle = corners.iter().map(|v| v.pos.y).sum::<f32>() / 3.0;
        if middle < top || middle > base + 0.01 {
            continue;
        }
        let first = copy.vertices.len() as u32;
        for vertex in corners {
            let y = (2.0 * base + MIRROR_GAP - vertex.pos.y).max(base + MIRROR_GAP);
            let fade = (1.0 - (y - base) / room).clamp(0.0, 1.0);
            copy.colored_vertex(pos2(vertex.pos.x, y), vertex.color.gamma_multiply(REFLECTION * fade));
        }
        copy.add_triangle(first, first + 1, first + 2);
    }
    mesh.append(copy);
}

/// A curve through the points, joined through the midpoints between neighbors and run flat to both
/// edges, as a polyline.
fn curve(points: &[Pos2], width: f32) -> Vec<Pos2> {
    let (Some(first), Some(end)) = (points.first(), points.last()) else {
        return Vec::new();
    };
    let mut line = vec![pos2(0.0, first.y), *first];
    let mut at = *first;
    for index in 1..points.len().saturating_sub(1) {
        let here = points[index];
        let next = points[index + 1];
        let to = pos2((here.x + next.x) / 2.0, (here.y + next.y) / 2.0);
        for step in 1..=6 {
            let s = step as f32 / 6.0;
            let a = at.lerp(here, s);
            let b = here.lerp(to, s);
            line.push(a.lerp(b, s));
        }
        at = to;
    }
    line.push(*end);
    line.push(pos2(width, end.y));
    line
}

/// `count` colors evenly along the stops, quiet first, each mixed in oklab between the two stops
/// either side of it, so a ramp through a few stops reads as one smooth run.
pub fn ramp_steps(stops: &[Color32], count: usize) -> Vec<Color32> {
    let Some(first) = stops.first() else {
        return Vec::new();
    };
    if stops.len() == 1 || count == 1 {
        return vec![*first; count];
    }
    (0..count)
        .map(|index| {
            let at = index as f32 / (count - 1) as f32 * (stops.len() - 1) as f32;
            let below = (at.floor() as usize).min(stops.len() - 2);
            let share = ((at - below as f32) * 100.0).round() / 100.0;
            mix_oklab(stops[below], stops[below + 1], share)
        })
        .collect()
}

/// `share` of the way from `from` to `to`, mixed in oklab as `color-mix(in oklab, ...)`.
fn mix_oklab(from: Color32, to: Color32, share: f32) -> Color32 {
    if share <= 0.0 {
        return from;
    }
    if share >= 1.0 {
        return to;
    }
    let a = to_oklab(from);
    let b = to_oklab(to);
    let mixed = [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * share);
    let alpha = from.a() as f32 + (to.a() as f32 - from.a() as f32) * share;
    from_oklab(mixed, alpha.round() as u8)
}

fn to_oklab(color: Color32) -> [f32; 3] {
    let lin = |c: u8| egui::ecolor::linear_f32_from_gamma_u8(c);
    let (r, g, b) = (lin(color.r()), lin(color.g()), lin(color.b()));
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

fn from_oklab([l, a, b]: [f32; 3], alpha: u8) -> Color32 {
    let l_ = (l + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m_ = (l - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s_ = (l - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    let r = 4.076_741_7 * l_ - 3.307_711_6 * m_ + 0.230_969_94 * s_;
    let g = -1.268_438 * l_ + 2.609_757_4 * m_ - 0.341_319_38 * s_;
    let bl = -0.004_196_086_3 * l_ - 0.703_418_6 * m_ + 1.707_614_7 * s_;
    let gamma = |c: f32| egui::ecolor::gamma_u8_from_linear_f32(c.clamp(0.0, 1.0));
    Color32::from_rgba_unmultiplied(gamma(r), gamma(g), gamma(bl), alpha)
}

/// Which of `count` steps along a ramp a level from 0 to 1 is drawn in.
pub fn ramp_index(level: f32, count: usize) -> usize {
    if count <= 1 || level.is_nan() || level <= 0.0 {
        return 0;
    }
    (level.min(1.0) * (count - 1) as f32).round() as usize
}

/// Where bars mirrored below their baseline stand, in a box this tall: the baseline, and the room
/// the reflection has under it.
pub fn mirror_base(height: f32) -> (f32, f32) {
    let room = (height * MIRROR_SHARE).round().max(0.0);
    ((height - room).max(0.0), room)
}

/// A band's weight at the low end and at the high end: music is loud at the bottom and thin at the top.
const LOW_WEIGHT: f32 = 0.82;
const HIGH_WEIGHT: f32 = 1.2;
/// Above 1, quiet bands sink and loud ones stand out.
const CONTRAST: f32 = 1.35;

/// A band's level as drawn, from an analyser's byte for it (0 to 255) and the band's place from low
/// (0) to high (1).
pub fn shape_level(byte: f32, place: f32) -> f32 {
    if byte.is_nan() || byte <= 0.0 {
        return 0.0;
    }
    let weight = LOW_WEIGHT + (HIGH_WEIGHT - LOW_WEIGHT) * place.clamp(0.0, 1.0);
    ((byte.min(255.0) / 255.0).powf(CONTRAST) * weight).min(1.0)
}

/// Folds an analyser's frequency bins (bytes from 0 to half the sample rate) into `count` bands
/// spaced evenly in pitch, low to high, each from 0 to 1.
pub fn band_levels(bins: &[u8], count: usize, sample_rate: f32, range: SoundRange) -> Vec<f32> {
    if bins.is_empty() || count == 0 || sample_rate.is_nan() || sample_rate <= 0.0 {
        return Vec::new();
    }
    let bin_hz = sample_rate / 2.0 / bins.len() as f32;
    let low = (bin_hz / 2.0).max(range.low);
    let high = (low * 1.01).max((sample_rate / 2.0).min(range.high));
    let ratio = high / low;
    let mean = |from: f32, to: f32| {
        let first = (from.floor() as usize).min(bins.len() - 1);
        let last = bins.len().min((first + 1).max(to.ceil() as usize));
        bins[first..last].iter().map(|b| *b as f32).sum::<f32>() / (last - first) as f32
    };
    let between = |at: f32| {
        let position = (at - 0.5).clamp(0.0, (bins.len() - 1) as f32);
        let below = position.floor() as usize;
        let above = (below + 1).min(bins.len() - 1);
        let lean = position - below as f32;
        bins[below] as f32 * (1.0 - lean) + bins[above] as f32 * lean
    };
    (0..count)
        .map(|band| {
            let from = low * ratio.powf(band as f32 / count as f32) / bin_hz;
            let to = low * ratio.powf((band + 1) as f32 / count as f32) / bin_hz;
            let byte = if to - from < 1.0 {
                between((from * to).sqrt())
            } else {
                mean(from, to)
            };
            let place = if count == 1 {
                0.0
            } else {
                band as f32 / (count - 1) as f32
            };
            shape_level(byte, place)
        })
        .collect()
}

/// The quietest an analyser's byte reads, in decibels, as Web Audio's `minDecibels` by default.
const MIN_DECIBELS: f32 = -100.0;
/// The loudest, as `maxDecibels`.
const MAX_DECIBELS: f32 = -30.0;

/// The frequency bins an analyser reads from a window of samples, as Web Audio's
/// `getByteFrequencyData` with its `fftSize` the window's length and no smoothing: the window under
/// a Blackman window, each bin's magnitude in decibels laid from `minDecibels` (0) to `maxDecibels`
/// (255). For an app that has the sound as samples rather than a live audio graph, to feed
/// [`band_levels`]; half as many bins as samples.
pub fn frequency_bytes(samples: &[f32]) -> Vec<u8> {
    let n = samples.len();
    if n < 2 {
        return Vec::new();
    }
    let tau = std::f32::consts::TAU;
    let windowed: Vec<f32> = samples
        .iter()
        .enumerate()
        .map(|(i, x)| {
            let a = i as f32 / n as f32;
            let w = 0.42 - 0.5 * (tau * a).cos() + 0.08 * (2.0 * tau * a).cos();
            x * w
        })
        .collect();
    (0..n / 2)
        .map(|k| {
            // the bin's turn per sample, walked by rotation rather than a sine per sample
            let step = -tau * k as f32 / n as f32;
            let (sin, cos) = step.sin_cos();
            let (mut re, mut im) = (0.0_f32, 0.0_f32);
            let (mut c, mut s) = (1.0_f32, 0.0_f32);
            for x in &windowed {
                re += x * c;
                im += x * s;
                let next = c * cos - s * sin;
                s = c * sin + s * cos;
                c = next;
            }
            let magnitude = (re * re + im * im).sqrt() / n as f32;
            let decibels = 20.0 * magnitude.max(1e-12).log10();
            let byte = 255.0 * (decibels - MIN_DECIBELS) / (MAX_DECIBELS - MIN_DECIBELS);
            byte.clamp(0.0, 255.0) as u8
        })
        .collect()
}

/// Levels stretched or squeezed to `count`, read straight between neighbors.
pub fn resample(levels: &[f32], count: usize) -> Vec<f32> {
    if count == 0 || levels.is_empty() {
        return Vec::new();
    }
    if levels.len() == count {
        return levels.to_vec();
    }
    if levels.len() == 1 || count == 1 {
        return vec![levels[0]; count];
    }
    (0..count)
        .map(|index| {
            let at = index as f32 / (count - 1) as f32 * (levels.len() - 1) as f32;
            let below = at.floor() as usize;
            let above = (below + 1).min(levels.len() - 1);
            levels[below] + (levels[above] - levels[below]) * (at - below as f32)
        })
        .collect()
}

/// How far a band moves toward a louder reading in one sixtieth of a second, and toward a quieter one.
pub const ATTACK: f32 = 0.55;
pub const RELEASE: f32 = 0.14;

/// This frame's levels settled against the last frame's: a band rises most of the way to a louder
/// reading at once and falls back slowly. The rates are per sixtieth of a second; a band the last
/// frame did not have starts from the floor.
pub fn settle(previous: &[f32], target: &[f32], elapsed_ms: f32) -> Vec<f32> {
    let steps = elapsed_ms.max(0.0) / FRAME_MS;
    let rise = 1.0 - (1.0 - ATTACK).powf(steps);
    let fall = 1.0 - (1.0 - RELEASE).powf(steps);
    target
        .iter()
        .enumerate()
        .map(|(index, wanted)| {
            let current = previous.get(index).copied().unwrap_or(0.0);
            current + (wanted - current) * if *wanted > current { rise } else { fall }
        })
        .collect()
}

/// Whether every value is within `threshold` of nothing: a frame not worth painting again.
pub fn is_quiet(values: &[f32], threshold: f32) -> bool {
    values.iter().all(|value| value.abs() <= threshold)
}

/// How wide a bar is, the room between two, and where the first one starts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarLayout {
    pub bar: f32,
    pub gap: f32,
    pub offset: f32,
}

/// Where `count` bars go across `width`. The gap is a share of each bar's slot, at least a pixel;
/// bars are never wider than `widest`, the row centered in the room left over.
pub fn bar_layout(width: f32, count: usize, widest: f32) -> BarLayout {
    if count == 0 || width <= 0.0 {
        return BarLayout {
            bar: 0.0,
            gap: 0.0,
            offset: 0.0,
        };
    }
    let n = count as f32;
    let slot = width / n;
    let gap = if slot < 3.0 {
        0.0
    } else {
        (slot * 0.22).round().max(1.0)
    };
    let bar = ((width - gap * (n - 1.0)) / n).min(widest).max(0.5);
    let used = bar * n + gap * (n - 1.0);
    BarLayout {
        bar,
        gap,
        offset: ((width - used) / 2.0).max(0.0),
    }
}

/// How many bands a kind draws in a box of this size when the app names no count.
pub fn auto_bands(kind: SoundKind, width: f32, height: f32) -> usize {
    let clamp = |value: f32, least: f32, most: f32| value.clamp(least, most).round() as usize;
    match kind {
        SoundKind::Spectrum => clamp(width / 8.0, 8.0, 192.0),
        SoundKind::Area => clamp(width / 14.0, 12.0, 96.0),
        // half the circle carries the bands and the other half mirrors them
        SoundKind::Ring => clamp(PI * ring_radii(width, height).0 / 7.0, 12.0, 96.0),
        SoundKind::Wave => 0,
    }
}

/// A ring's middle circle and how far its spokes reach past it, for a box of this size.
pub fn ring_radii(width: f32, height: f32) -> (f32, f32) {
    let half = (width.min(height) / 2.0).max(0.0);
    (half * 0.46, half * 0.46)
}

/// The tops of the bands across the width, one point at each band's middle, standing up from
/// `base` by `reach` at full level.
pub fn area_points(levels: &[f32], width: f32, base: f32, reach: f32) -> Vec<Pos2> {
    if levels.is_empty() || width <= 0.0 {
        return Vec::new();
    }
    let step = width / levels.len() as f32;
    levels
        .iter()
        .enumerate()
        .map(|(index, level)| pos2(step * (index as f32 + 0.5), base - level.clamp(0.0, 1.0) * reach))
        .collect()
}

/// How many samples a wave shows across its width: a few hundredths of a second.
pub const WAVE_SPAN: usize = 640;

/// Where a wave's window starts: the first rise through zero, as an oscilloscope triggers, so one
/// frame's trace lands where the last one was. Zero when the signal never crosses.
pub fn trigger_at(samples: &[f32], span: usize) -> usize {
    let last = samples.len().saturating_sub(span);
    (1..=last)
        .find(|index| samples[index - 1] < 0.0 && samples[*index] >= 0.0)
        .unwrap_or(0)
}

/// How much of a wave's held peak is kept from one sixtieth of a second to the next.
pub const PEAK_HOLD: f32 = 0.99;
/// The most a quiet wave is enlarged.
pub const MOST_GAIN: f32 = 4.0;
const WAVE_HEADROOM: f32 = 0.9;

/// A wave's held peak after this frame: caught up at once by a louder sample, falling back slowly.
pub fn hold_peak(previous: f32, samples: &[f32], elapsed_ms: f32) -> f32 {
    let peak = samples.iter().fold(0.0_f32, |most, s| most.max(s.abs()));
    peak.max(previous * PEAK_HOLD.powf(elapsed_ms.max(0.0) / FRAME_MS))
}

/// How much a wave with this held peak is enlarged, so a quiet mix still fills the room.
pub fn wave_gain(peak: f32) -> f32 {
    if peak.is_nan() || peak <= 0.0 {
        return 1.0;
    }
    MOST_GAIN.min(WAVE_HEADROOM / peak)
}

/// The wave's trace across `width`, its middle at half the height and full scale at the edges.
pub fn wave_points(samples: &[f32], width: f32, height: f32) -> Vec<Pos2> {
    if width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }
    let middle = height / 2.0;
    if samples.len() < 2 {
        return vec![pos2(0.0, middle), pos2(width, middle)];
    }
    let last = (samples.len() - 1) as f32;
    samples
        .iter()
        .enumerate()
        .map(|(index, sample)| pos2(index as f32 / last * width, middle - sample.clamp(-1.0, 1.0) * middle))
        .collect()
}

/// The ring's spokes around `center`, each its foot on the circle and its tip: the bands from the
/// top down the right side, low to high, and the same again mirrored down the left. Each stands
/// out from the inner circle by its level of `reach`, at least `least`.
pub fn ring_spokes(levels: &[f32], center: Pos2, inner: f32, reach: f32, least: f32) -> Vec<(Pos2, Pos2)> {
    let count = levels.len() as f32;
    let mut spokes = Vec::with_capacity(levels.len() * 2);
    for side in [1.0_f32, -1.0] {
        for (index, raw) in levels.iter().enumerate() {
            let angle = -PI / 2.0 + side * PI * ((index as f32 + 0.5) / count);
            let length = (raw.clamp(0.0, 1.0) * reach).max(least);
            let (sin, cos) = angle.sin_cos();
            spokes.push((
                pos2(center.x + cos * inner, center.y + sin * inner),
                pos2(center.x + cos * (inner + length), center.y + sin * (inner + length)),
            ));
        }
    }
    spokes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_silence_as_nothing_and_full_scale_as_at_most_full_height() {
        assert_eq!(shape_level(0.0, 0.0), 0.0);
        assert_eq!(shape_level(f32::NAN, 0.5), 0.0);
        assert!((shape_level(255.0, 1.0) - 1.0).abs() < 1e-6);
        assert!(shape_level(400.0, 0.0) <= 1.0);
        assert!(shape_level(128.0, 1.0) > shape_level(128.0, 0.0));
    }

    #[test]
    fn folds_bins_into_bands() {
        let bins = vec![0u8; 1024];
        assert!(band_levels(&bins, 32, 48_000.0, MUSIC_RANGE).iter().all(|l| *l == 0.0));
        assert!(band_levels(&[], 32, 48_000.0, MUSIC_RANGE).is_empty());
        let mut low = vec![0u8; 1024];
        low[3] = 255;
        let levels = band_levels(&low, 32, 48_000.0, MUSIC_RANGE);
        assert_eq!(levels.len(), 32);
        let loudest = levels
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap_or(0);
        assert!(loudest < 6);
    }

    #[test]
    fn resamples_by_reading_between_neighbors() {
        assert_eq!(resample(&[0.1, 0.5, 0.9], 3), vec![0.1, 0.5, 0.9]);
        assert_eq!(resample(&[0.0, 1.0], 5), vec![0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_eq!(resample(&[0.0, 0.5, 1.0, 0.5, 0.0], 3), vec![0.0, 1.0, 0.0]);
        assert_eq!(resample(&[0.4], 3), vec![0.4, 0.4, 0.4]);
        assert!(resample(&[], 3).is_empty());
    }

    #[test]
    fn settles_fast_up_and_slow_down() {
        let risen = settle(&[0.0], &[1.0], FRAME_MS)[0];
        let fallen = settle(&[1.0], &[0.0], FRAME_MS)[0];
        assert!((risen - ATTACK).abs() < 1e-5);
        assert!((fallen - (1.0 - RELEASE)).abs() < 1e-5);
        assert_eq!(settle(&[], &[1.0, 1.0], FRAME_MS), vec![ATTACK, ATTACK]);
        let long = settle(&[0.0], &[1.0], FRAME_MS * 2.0)[0];
        let short = settle(&settle(&[0.0], &[1.0], FRAME_MS), &[1.0], FRAME_MS)[0];
        assert!((long - short).abs() < 1e-5);
    }

    #[test]
    fn lays_bars_across_the_width() {
        let layout = bar_layout(400.0, 32, f32::INFINITY);
        assert_eq!(layout.offset, 0.0);
        assert!((layout.bar * 32.0 + layout.gap * 31.0 - 400.0).abs() < 1e-3);
        let narrow = bar_layout(800.0, 10, 12.0);
        assert_eq!(narrow.bar, 12.0);
        assert!(narrow.offset > 0.0);
        assert_eq!(bar_layout(100.0, 60, f32::INFINITY).gap, 0.0);
        assert_eq!(
            bar_layout(0.0, 10, f32::INFINITY),
            BarLayout {
                bar: 0.0,
                gap: 0.0,
                offset: 0.0
            }
        );
    }

    #[test]
    fn counts_bands_by_the_room() {
        assert_eq!(auto_bands(SoundKind::Spectrum, 20.0, 300.0), 8);
        assert_eq!(auto_bands(SoundKind::Spectrum, 10_000.0, 300.0), 192);
        assert!(auto_bands(SoundKind::Area, 1440.0, 300.0) < auto_bands(SoundKind::Spectrum, 1440.0, 300.0));
        assert_eq!(
            auto_bands(SoundKind::Ring, 1440.0, 200.0),
            auto_bands(SoundKind::Ring, 200.0, 200.0)
        );
        assert_eq!(auto_bands(SoundKind::Wave, 1440.0, 300.0), 0);
    }

    #[test]
    fn triggers_at_the_first_rise_through_zero() {
        assert_eq!(trigger_at(&[0.5, 0.2, -0.1, -0.3, 0.1, 0.4, 0.2, 0.0], 4), 4);
        assert_eq!(trigger_at(&[0.1, 0.2, 0.3], 2), 0);
        assert_eq!(trigger_at(&[], 2), 0);
    }

    #[test]
    fn mixes_a_ramp_between_its_stops() {
        let red = Color32::from_rgb(255, 0, 0);
        let blue = Color32::from_rgb(0, 0, 255);
        let steps = ramp_steps(&[red, blue], 5);
        assert_eq!(steps.len(), 5);
        assert_eq!(steps[0], red);
        assert_eq!(steps[4], blue);
        assert_eq!(ramp_steps(&[red], 3), vec![red; 3]);
        assert_eq!(ramp_index(0.0, 24), 0);
        assert_eq!(ramp_index(1.0, 24), 23);
        assert_eq!(sound_ramp("heat"), Some(&["warning", "critical"][..]));
    }

    #[test]
    fn names_the_named_ramps_from_tokens_only() {
        assert!(sound_ramp("neon").is_none());
    }
}
