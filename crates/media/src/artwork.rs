//! A record's cover, after sokkel-ui-media's `Artwork` and `cover-seed`: a picture the app paints, or
//! a cover drawn in the theme's hues, a gradient between two with one motif over it, chosen from a
//! stable seed.

use egui::{
    Color32, Mesh, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Widget, epaint::QuadraticBezierShape, pos2,
    vec2,
};

use sokkel_egui::{Tokens, tokens};

/// The shapes a drawn cover is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArtworkMotif {
    Sun,
    Rings,
    Bands,
    Dots,
    Wave,
    Peaks,
}

pub const ARTWORK_MOTIFS: [ArtworkMotif; 6] = [
    ArtworkMotif::Sun,
    ArtworkMotif::Rings,
    ArtworkMotif::Bands,
    ArtworkMotif::Dots,
    ArtworkMotif::Wave,
    ArtworkMotif::Peaks,
];

/// The hues a drawn cover is painted in, each a token, so a theme can change them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArtworkHue {
    Blue,
    Teal,
    Green,
    Jade,
    Violet,
    Pink,
    Amber,
    Coral,
}

pub const ARTWORK_HUES: [ArtworkHue; 8] = [
    ArtworkHue::Blue,
    ArtworkHue::Teal,
    ArtworkHue::Green,
    ArtworkHue::Jade,
    ArtworkHue::Violet,
    ArtworkHue::Pink,
    ArtworkHue::Amber,
    ArtworkHue::Coral,
];

impl ArtworkHue {
    /// The color token the hue is.
    pub fn token(self) -> &'static str {
        match self {
            Self::Blue => "kind-blue",
            Self::Teal => "kind-teal",
            Self::Green => "kind-green",
            Self::Jade => "kind-jade",
            Self::Violet => "kind-violet",
            Self::Pink => "kind-pink",
            Self::Amber => "chart-4",
            Self::Coral => "chart-5",
        }
    }
}

/// A drawn cover: a motif over a gradient from one hue to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ArtworkCover {
    pub motif: ArtworkMotif,
    pub from: ArtworkHue,
    pub to: ArtworkHue,
}

/// A string as an unsigned 32-bit number, the same in every build: FNV-1a over its UTF-16 code
/// units, then mixed so every bit depends on every character.
pub fn seed_hash(seed: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for unit in seed.encode_utf16() {
        hash ^= unit as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2_ae35);
    hash ^= hash >> 16;
    hash
}

/// The cover a seed wears: the motif, the first hue and the second from separate bits of the hash,
/// the two hues always different.
pub fn cover_for(seed: &str) -> ArtworkCover {
    let hash = seed_hash(seed);
    let hues = ARTWORK_HUES.len() as u32;
    let motif = ARTWORK_MOTIFS[(hash % ARTWORK_MOTIFS.len() as u32) as usize];
    let first = (hash >> 8) % hues;
    let second = (first + 1 + ((hash >> 16) % (hues - 1))) % hues;
    ArtworkCover {
        motif,
        from: ARTWORK_HUES[first as usize],
        to: ARTWORK_HUES[second as usize],
    }
}

/// The two inks a motif is drawn in over the gradient: `terminal-foreground` and `terminal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverInk {
    Light,
    Dark,
}

impl CoverInk {
    pub fn token(self) -> &'static str {
        match self {
            Self::Light => "terminal-foreground",
            Self::Dark => "terminal",
        }
    }
}

/// One shape of a motif, in a frame `width` wide and 100 tall.
#[derive(Clone, Debug, PartialEq)]
pub enum CoverShape {
    /// A circle, filled, or stroked `stroke` wide.
    Circle {
        center: Pos2,
        radius: f32,
        ink: CoverInk,
        opacity: f32,
        stroke: Option<f32>,
    },
    /// A convex polygon, filled.
    Polygon {
        points: Vec<Pos2>,
        ink: CoverInk,
        opacity: f32,
    },
    /// A line across the frame filled down to its foot.
    Ground {
        line: Vec<Pos2>,
        ink: CoverInk,
        opacity: f32,
    },
    /// A wave across the frame at `y`: two quadratic curves from 4 before its start to 4 past its
    /// end, stroked.
    Wave {
        y: f32,
        width: f32,
        ink: CoverInk,
        opacity: f32,
        stroke: f32,
    },
}

/// The shapes a motif is made of, in a frame `width` wide (100 square, 160 wide) and 100 tall.
pub fn cover_shapes(motif: ArtworkMotif, width: f32) -> Vec<CoverShape> {
    let middle = width / 2.0;
    let light = CoverInk::Light;
    let dark = CoverInk::Dark;
    match motif {
        ArtworkMotif::Sun => vec![
            CoverShape::Circle {
                center: pos2(middle + 18.0, 36.0),
                radius: 16.0,
                ink: light,
                opacity: 0.5,
                stroke: None,
            },
            CoverShape::Ground {
                line: vec![
                    pos2(0.0, 74.0),
                    pos2(28.0, 56.0),
                    pos2(50.0, 66.0),
                    pos2(74.0, 48.0),
                    pos2(width, 64.0),
                ],
                ink: dark,
                opacity: 0.18,
            },
            CoverShape::Ground {
                line: vec![pos2(0.0, 86.0), pos2(34.0, 72.0), pos2(62.0, 80.0), pos2(width, 70.0)],
                ink: dark,
                opacity: 0.16,
            },
        ],
        ArtworkMotif::Rings => [10.0, 20.0, 30.0, 40.0, 50.0]
            .iter()
            .enumerate()
            .map(|(i, r)| CoverShape::Circle {
                center: pos2(middle, 50.0),
                radius: *r,
                ink: light,
                opacity: 0.5 - i as f32 * 0.08,
                stroke: Some(3.0),
            })
            .collect(),
        ArtworkMotif::Bands => (0..5)
            .map(|band| {
                let y = 18.0 + band as f32 * 14.0;
                CoverShape::Polygon {
                    points: vec![pos2(0.0, y), pos2(width, y), pos2(width, y + 6.0), pos2(0.0, y + 6.0)],
                    ink: light,
                    opacity: 0.14 + band as f32 * 0.07,
                }
            })
            .collect(),
        ArtworkMotif::Dots => (0..36)
            .map(|i| {
                let column = i % 6;
                let row = i / 6;
                CoverShape::Circle {
                    center: pos2(middle - 40.0 + column as f32 * 16.0, 10.0 + row as f32 * 16.0),
                    radius: 1.5 + ((column + row) % 4) as f32 * 0.9,
                    ink: light,
                    opacity: 0.55,
                    stroke: None,
                }
            })
            .collect(),
        ArtworkMotif::Wave => [38.0, 52.0, 66.0]
            .iter()
            .enumerate()
            .map(|(i, y)| CoverShape::Wave {
                y: *y,
                width,
                ink: light,
                opacity: 0.6 - i as f32 * 0.15,
                stroke: 3.0,
            })
            .collect(),
        ArtworkMotif::Peaks => vec![
            CoverShape::Ground {
                line: vec![
                    pos2(0.0, 100.0),
                    pos2(middle - 20.0, 30.0),
                    pos2(middle + 4.0, 62.0),
                    pos2(middle + 24.0, 42.0),
                    pos2(width, 100.0),
                ],
                ink: dark,
                opacity: 0.22,
            },
            CoverShape::Polygon {
                points: vec![
                    pos2(middle - 20.0, 30.0),
                    pos2(middle - 11.0, 41.0),
                    pos2(middle - 27.0, 39.0),
                ],
                ink: light,
                opacity: 0.7,
            },
            CoverShape::Circle {
                center: pos2(middle - 26.0, 22.0),
                radius: 5.0,
                ink: light,
                opacity: 0.6,
                stroke: None,
            },
        ],
    }
}

/// What paints a picture into its box.
pub(crate) type PicturePaint<'a> = Box<dyn Fn(&Painter, Rect) + 'a>;

/// A record's cover, after sokkel-ui-media's `Artwork`: a gradient from corner to corner between
/// two of the theme's hues with one motif over it in its light and dark inks, chosen from a stable
/// `seed` unless `cover` names it, and filling whatever box it is given as `xMidYMid slice` would.
/// A picture the app has is drawn with [`Artwork::picture`] in its place.
#[must_use = "add it with `ui.add(...)`, or paint it with `paint`"]
pub struct Artwork<'a> {
    cover: ArtworkCover,
    wide: bool,
    picture: Option<PicturePaint<'a>>,
    size: Option<egui::Vec2>,
}

impl<'a> Artwork<'a> {
    /// The cover chosen from `seed`, such as the record's id.
    pub fn new(seed: &str) -> Self {
        Self::of(cover_for(seed))
    }

    /// A cover named outright.
    pub fn of(cover: ArtworkCover) -> Self {
        Self {
            cover,
            wide: false,
            picture: None,
            size: None,
        }
    }

    /// Drawn for a wide frame, as a video's, rather than a square.
    pub fn wide(mut self, wide: bool) -> Self {
        self.wide = wide;
        self
    }

    /// The record's own picture, painted by the app into the box (the web's `src`); the drawn
    /// cover then shows nowhere.
    pub fn picture(mut self, paint: impl Fn(&Painter, Rect) + 'a) -> Self {
        self.picture = Some(Box::new(paint));
        self
    }

    /// How big it is drawn as a widget; the width the `ui` has left and as tall as its shape by
    /// default.
    pub fn size(mut self, size: egui::Vec2) -> Self {
        self.size = Some(size);
        self
    }

    /// Paint it into `rect`, cut to it.
    pub fn paint(&self, painter: &Painter, rect: Rect) {
        let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        if let Some(picture) = &self.picture {
            picture(&painter, rect);
            return;
        }
        let t = tokens(painter.ctx());
        paint_cover(&painter, &t, self.cover, self.wide, rect);
    }
}

impl Widget for Artwork<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = self.size.unwrap_or_else(|| {
            let w = ui.available_width();
            vec2(w, if self.wide { w * 9.0 / 16.0 } else { w })
        });
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect);
        }
        response
    }
}

/// A drawn cover in `rect`: its frame (160 by 100 wide, else 100 square) scaled to cover the rect,
/// centered.
fn paint_cover(painter: &Painter, t: &Tokens, cover: ArtworkCover, wide: bool, rect: Rect) {
    let frame_width = if wide { 160.0 } else { 100.0 };
    let scale = (rect.width() / frame_width).max(rect.height() / 100.0);
    let origin = rect.center() - vec2(frame_width, 100.0) * scale / 2.0;
    let at = |p: Pos2| origin + p.to_vec2() * scale;
    let frame = Rect::from_min_size(origin, vec2(frame_width, 100.0) * scale);

    // the gradient from the frame's top start to its foot end, as `x1=0 y1=0 x2=1 y2=1`
    let from = t.color(cover.from.token());
    let to = t.color(cover.to.token());
    let middle = from.lerp_to_gamma(to, 0.5);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(frame.left_top(), from);
    mesh.colored_vertex(frame.right_top(), middle);
    mesh.colored_vertex(frame.right_bottom(), to);
    mesh.colored_vertex(frame.left_bottom(), middle);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));

    let ink = |ink: CoverInk, opacity: f32| t.color(ink.token()).gamma_multiply(opacity);
    for shape in cover_shapes(cover.motif, frame_width) {
        match shape {
            CoverShape::Circle {
                center,
                radius,
                ink: i,
                opacity,
                stroke,
            } => match stroke {
                None => {
                    painter.circle_filled(at(center), radius * scale, ink(i, opacity));
                }
                Some(width) => {
                    painter.circle_stroke(at(center), radius * scale, Stroke::new(width * scale, ink(i, opacity)));
                }
            },
            CoverShape::Polygon {
                points,
                ink: i,
                opacity,
            } => {
                let points = points.into_iter().map(at).collect();
                painter.add(Shape::convex_polygon(points, ink(i, opacity), Stroke::NONE));
            }
            CoverShape::Ground { line, ink: i, opacity } => {
                let color = ink(i, opacity);
                let foot = frame.bottom();
                let mut mesh = Mesh::default();
                for (n, p) in line.iter().enumerate() {
                    let p = at(*p);
                    mesh.colored_vertex(p, color);
                    mesh.colored_vertex(pos2(p.x, foot), color);
                    if n > 0 {
                        let a = (2 * (n - 1)) as u32;
                        mesh.add_triangle(a, a + 1, a + 2);
                        mesh.add_triangle(a + 1, a + 3, a + 2);
                    }
                }
                painter.add(Shape::mesh(mesh));
            }
            CoverShape::Wave {
                y,
                width,
                ink: i,
                opacity,
                stroke,
            } => {
                let color = ink(i, opacity);
                let stroke = Stroke::new(stroke * scale, color);
                let middle = width / 2.0;
                // `Q` to the middle, then `T` on, its control point the first's reflected
                let halves = [
                    [pos2(-4.0, y), pos2(width * 0.25, y - 14.0), pos2(middle, y)],
                    [pos2(middle, y), pos2(width * 0.75, y + 14.0), pos2(width + 4.0, y)],
                ];
                for half in halves {
                    painter.add(QuadraticBezierShape::from_points_stroke(
                        half.map(at),
                        false,
                        Color32::TRANSPARENT,
                        stroke,
                    ));
                }
            }
        }
    }
}
