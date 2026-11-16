//! One node as a box, after sokkel-ui-graph's `GraphNodeBox`.

use egui::{Color32, CornerRadius, Painter, Rect, Stroke, StrokeKind, Vec2, pos2, vec2};
use sokkel_egui::{Icon, TextRole, Tokens, mono_font};

/// The narrowest and widest a node box is drawn; past the widest its words are cut short.
pub const NODE_MIN_WIDTH: f32 = 96.0;
pub const NODE_MAX_WIDTH: f32 = 280.0;
/// How tall a node box is with its code line, and without one.
pub const NODE_HEIGHT: f32 = 44.0;
pub const NODE_HEIGHT_SHORT: f32 = 30.0;

/// A box's padding and border across, and the room a dot, a glyph and a gap take.
const CHROME: f32 = 22.0;
const GAP: f32 = 6.0;
const LINE_GAP: f32 = 8.0;
const DOT: f32 = 8.0;
const GLYPH: f32 = 12.0;
/// The extra padding across a round box.
const ROUND_PAD: f32 = 8.0;
/// The duration a box with a state keeps room for, so it does not widen when its step ends.
const DURATION_ROOM: &str = "59m 59s";

/// A node's state: the word pointing at its dot says, and the dot's color.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeStatus {
    pub label: String,
    pub color: Color32,
}

/// One node of a graph, as sokkel-ui-graph's `GraphNodeData`: its key in the code face, a human
/// name, what kind of thing it is, a glyph, a state's dot, and `round` to tell a second kind of
/// node apart without color.
#[derive(Clone, Debug)]
pub struct GraphNode {
    pub id: String,
    /// The node's key, drawn in the code face; the title when it has no name.
    pub code: String,
    pub name: Option<String>,
    /// What sort of thing the node is, on the code line after the code.
    pub kind: Option<String>,
    pub icon: Option<Icon>,
    pub status: Option<NodeStatus>,
    /// Wider corners, as a store beside services.
    pub round: bool,
}

impl GraphNode {
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            code: id.clone(),
            id,
            name: None,
            kind: None,
            icon: None,
            status: None,
            round: false,
        }
    }

    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn status(mut self, label: impl Into<String>, color: Color32) -> Self {
        self.status = Some(NodeStatus {
            label: label.into(),
            color,
        });
        self
    }

    pub fn round(mut self, round: bool) -> Self {
        self.round = round;
        self
    }

    fn named(&self) -> bool {
        self.name.as_deref().is_some_and(|n| !n.is_empty())
    }

    fn title(&self) -> &str {
        if self.named() {
            self.name.as_deref().unwrap_or_default()
        } else {
            &self.code
        }
    }

    /// The code line: the code when the node has a name, and its kind, joined by a dot.
    fn line(&self) -> String {
        let code = self.named().then_some(self.code.as_str());
        [code, self.kind.as_deref()]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    }

    fn two_lines(&self) -> bool {
        !self.line().is_empty() || self.status.is_some()
    }

    /// The size the box is drawn at, so none of its words is cut short of the widest a box may be.
    pub fn size(&self, painter: &Painter, t: &Tokens) -> Vec2 {
        let measure = |galley: std::sync::Arc<egui::Galley>| galley.size().x.ceil();
        let head = self.icon.map_or(0.0, |_| GLYPH + GAP)
            + measure(title_galley(painter, t, self.title(), self.named(), 1.0))
            + self.status.as_ref().map_or(0.0, |_| GAP + DOT);
        let line = self.line();
        let said = if line.is_empty() {
            0.0
        } else {
            measure(small_mono(painter, t, &line, 1.0))
        };
        let foot = said
            + if self.status.is_some() {
                LINE_GAP + measure(small_mono(painter, t, DURATION_ROOM, 1.0))
            } else {
                0.0
            };
        let pad = CHROME + if self.round { ROUND_PAD } else { 0.0 };
        let width = (head.max(foot) + pad).ceil().clamp(NODE_MIN_WIDTH, NODE_MAX_WIDTH);
        vec2(
            width,
            if self.two_lines() {
                NODE_HEIGHT
            } else {
                NODE_HEIGHT_SHORT
            },
        )
    }

    /// Paint the box into `rect`, drawn at `zoom`; the box's edge darkens under the pointer and a
    /// chosen box wears a ring.
    pub fn paint(&self, painter: &Painter, t: &Tokens, rect: Rect, zoom: f32, hovered: bool, selected: bool) {
        let radius = t.length("radius-node") * if self.round { 3.0 } else { 1.0 } * zoom;
        let corners = CornerRadius::same(radius.round() as u8);
        for shadow in t.shadows("elevation-node") {
            painter.add(shadow.as_shape(rect, corners));
        }
        let edge = if hovered {
            t.color("muted-foreground")
        } else {
            t.color("node-line")
        };
        painter.rect(
            rect,
            corners,
            t.color("node"),
            Stroke::new(zoom, edge),
            StrokeKind::Inside,
        );
        if selected {
            let ring = rect.expand(2.0 * zoom);
            painter.rect_stroke(
                ring,
                CornerRadius::same((radius + 2.0 * zoom).round() as u8),
                Stroke::new(2.0 * zoom, t.color("foreground")),
                StrokeKind::Outside,
            );
        }
        let pad = (1.0 + 10.0 + if self.round { 4.0 } else { 0.0 }) * zoom;
        let inner = rect.shrink2(vec2(pad, zoom));
        let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        let title_line = t.text("graph-title").line_height;
        let small_line = 16.0;
        let block = if self.two_lines() {
            title_line + 2.0 + small_line
        } else {
            title_line
        } * zoom;
        let top = inner.center().y - block / 2.0;
        let title_mid = top + title_line * zoom / 2.0;
        let mut x = inner.left();
        let muted = t.color("muted-foreground");
        if let Some(icon) = self.icon {
            sokkel_egui::icons::paint(
                &painter,
                pos2(x + GLYPH * zoom / 2.0, title_mid),
                icon,
                GLYPH * zoom,
                muted,
            );
            x += (GLYPH + GAP) * zoom;
        }
        let mut right = inner.right();
        if let Some(status) = &self.status {
            painter.circle_filled(
                pos2(right - DOT * zoom / 2.0, title_mid),
                DOT * zoom / 2.0,
                status.color,
            );
            right -= (DOT + GAP) * zoom;
        }
        let title = title_galley(&painter, t, self.title(), self.named(), zoom);
        let clip = Rect::from_x_y_ranges(x..=right, rect.y_range());
        painter.with_clip_rect(clip.intersect(painter.clip_rect())).galley(
            pos2(x, title_mid - title.size().y / 2.0),
            title,
            t.color("foreground"),
        );
        if self.two_lines() {
            let line = self.line();
            if !line.is_empty() {
                let line_mid = top + (title_line + 2.0 + small_line / 2.0) * zoom;
                let galley = small_mono(&painter, t, &line, zoom);
                painter.galley(pos2(inner.left(), line_mid - galley.size().y / 2.0), galley, muted);
            }
        }
    }
}

/// The title: the name in the graph's title role, or the code in the code face, both semibold.
fn title_galley(painter: &Painter, t: &Tokens, text: &str, named: bool, zoom: f32) -> std::sync::Arc<egui::Galley> {
    let size = t.text("graph-title").size;
    if named {
        let rich = TextRole::GraphTitle
            .rich(t, text)
            .size(size * zoom)
            .line_height(Some(t.text("graph-title").line_height * zoom))
            .variation("wght", 600.0);
        let ctx = painter.ctx();
        egui::WidgetText::from(rich).into_galley_impl(
            ctx,
            &ctx.global_style(),
            egui::text::TextWrapping::no_max_width(),
            egui::FontSelection::Default,
            egui::Align::Center,
        )
    } else {
        painter.layout_no_wrap(
            text.to_owned(),
            mono_font(painter.ctx(), size * zoom, 600.0),
            Color32::PLACEHOLDER,
        )
    }
}

/// Words on the code line: the graph's small role in the code face.
fn small_mono(painter: &Painter, t: &Tokens, text: &str, zoom: f32) -> std::sync::Arc<egui::Galley> {
    painter.layout_no_wrap(
        text.to_owned(),
        mono_font(painter.ctx(), t.text("graph-small").size * zoom, 400.0),
        Color32::PLACEHOLDER,
    )
}
