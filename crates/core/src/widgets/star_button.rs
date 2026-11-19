//! A favorite toggle, after sokkel-ui's `StarButton`: an outlined star in the quiet ink that fills
//! in the accent once starred.

use egui::{
    Color32, CornerRadius, Mesh, Painter, Pos2, Response, Role, Sense, Stroke, StrokeKind, Ui, Widget, pos2, vec2,
};

use crate::{Icon, icons, tokens};

/// The words the star says, each with an English default.
#[derive(Clone, Debug)]
pub struct StarButtonWords {
    /// Its label while not starred: what pressing it does.
    pub star: String,
    /// Its label while starred.
    pub unstar: String,
}

impl Default for StarButtonWords {
    fn default() -> Self {
        Self {
            star: "Add to favorites".into(),
            unstar: "Remove from favorites".into(),
        }
    }
}

/// A favorite toggle on the icon button: an outlined star in the quiet ink that fills in the
/// accent (`primary-ink`) once starred, the button pressed while it is. Its tooltip and accessible
/// name say what a press does, so they change with it. A press flips `starred` and marks the
/// response changed. It takes the icon button's sizes (`xs`, the default `icon-sm`, `md`) and its
/// outline; a starred ghost star shows its fill alone, without the pressed ground, so a column of
/// them down a list stays quiet, while an outlined one stands on the pressed (`muted`) ground.
#[must_use = "add it with `ui.add(...)`"]
pub struct StarButton<'a> {
    starred: &'a mut bool,
    outline: bool,
    xs: bool,
    md: bool,
    words: StarButtonWords,
}

impl<'a> StarButton<'a> {
    pub fn new(starred: &'a mut bool) -> Self {
        Self {
            starred,
            outline: false,
            xs: false,
            md: false,
            words: StarButtonWords::default(),
        }
    }

    /// The outline variant, as beside the buttons of a page's header.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// The smallest size, `icon-xs`: `control-xs` square, the star 12.
    pub fn xs(mut self) -> Self {
        self.xs = true;
        self
    }

    /// The control's size, `icon`: `control` square, beside controls of the default size.
    pub fn md(mut self) -> Self {
        self.md = true;
        self
    }

    pub fn words(mut self, words: StarButtonWords) -> Self {
        self.words = words;
        self
    }
}

impl Widget for StarButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let starred = *self.starred;
        // the icon button's square and corners at each size
        let (side, cap) = if self.xs {
            (t.density("control-xs"), 10)
        } else if self.md {
            (super::finger_tall(ui.ctx(), &t, t.density("control")), u8::MAX)
        } else {
            (super::finger_tall(ui.ctx(), &t, t.density("control-sm")), 12)
        };
        let radius = CornerRadius::same(t.radius("control").nw.min(cap));
        let (rect, mut response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
        let label = if starred { &self.words.unstar } else { &self.words.star };
        response.widget_info(|| egui::WidgetInfo::selected(Role::Button, true, starred, label));
        if response.clicked() {
            *self.starred = !starred;
            response.mark_changed();
        }
        let hot = response.hovered();
        let painter = ui.painter();
        let (rest, border) = match (self.outline, t.dark) {
            (true, true) => (t.color_at("input", 0.3), t.color("input")),
            (true, false) => (t.color("background"), t.color("border")),
            (false, _) => (Color32::TRANSPARENT, Color32::TRANSPARENT),
        };
        // pressed, an outlined star stands on the pressed ground; a ghost one keeps its own
        let ground = if hot {
            t.color("hover")
        } else if starred && self.outline {
            t.color("muted")
        } else {
            rest
        };
        painter.rect(rect, radius, ground, Stroke::new(1.0, border), StrokeKind::Inside);
        let ink = if starred {
            t.color("primary-ink")
        } else if hot || self.outline {
            t.color("foreground")
        } else {
            t.color("muted-foreground")
        };
        let glyph = if self.xs { 12.0 } else { 16.0 };
        if starred {
            paint_star_fill(painter, rect.center(), glyph, ink);
        }
        icons::paint(painter, rect.center(), Icon::Star, glyph, ink);
        super::focus_ring(painter, &t, rect, radius, &response);
        let said = label.clone();
        super::tooltip(response, said)
    }
}

/// Lucide's star outline in its 24 box, the points its path passes through, clockwise from the top.
const STAR: [(f32, f32); 20] = [
    (11.525, 2.295),
    (12.475, 2.295),
    (14.785, 6.974),
    (16.38, 8.134),
    (21.546, 8.89),
    (21.84, 9.794),
    (18.104, 13.432),
    (17.493, 15.31),
    (18.375, 20.45),
    (17.604, 21.01),
    (12.986, 18.582),
    (11.013, 18.582),
    (6.396, 21.01),
    (5.626, 20.45),
    (6.507, 15.311),
    (5.896, 13.432),
    (2.16, 9.795),
    (2.454, 8.889),
    (7.619, 8.134),
    (9.216, 6.974),
];

/// The star's inside filled, as `fill-current` fills the icon: a fan from the star's middle, which
/// sees every point of its outline, at the icon's size `size` around `center`.
pub(crate) fn paint_star_fill(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let scale = size / 24.0;
    let at = |(x, y): (f32, f32)| pos2(center.x + (x - 12.0) * scale, center.y + (y - 12.0) * scale);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(at((12.0, 12.6)), color);
    for point in STAR {
        mesh.colored_vertex(at(point), color);
    }
    let n = STAR.len() as u32;
    for index in 0..n {
        mesh.add_triangle(0, 1 + index, 1 + (index + 1) % n);
    }
    painter.add(mesh);
}
