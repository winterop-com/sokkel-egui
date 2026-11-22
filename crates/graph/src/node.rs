//! One node as a box, after sokkel-ui-graph's `GraphNodeBox`.

use egui::{Color32, CornerRadius, Painter, Rect, Shape, Stroke, StrokeKind, Vec2, pos2, vec2};
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
/// The room an issue mark takes, and the glyph inside it.
const MARK: f32 = 20.0;
const MARK_GLYPH: f32 = 16.0;
/// The height of the line a step's progress is drawn along its foot.
const PROGRESS: f32 = 2.0;

/// How bad an issue on a node is: a warning is allowed, a critical one stops a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueTone {
    Critical,
    Warning,
}

impl IssueTone {
    /// The tone's color and its ink, as the tokens name them.
    fn keys(self) -> (&'static str, &'static str) {
        match self {
            IssueTone::Critical => ("critical", "critical-ink"),
            IssueTone::Warning => ("warning", "warning-ink"),
        }
    }

    fn icon(self) -> Icon {
        match self {
            IssueTone::Critical => Icon::CircleAlert,
            IssueTone::Warning => Icon::TriangleAlert,
        }
    }
}

/// What is wrong with a node, as a form field says it.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeIssue {
    pub tone: IssueTone,
    pub text: String,
}

impl NodeIssue {
    pub fn critical(text: impl Into<String>) -> Self {
        Self {
            tone: IssueTone::Critical,
            text: text.into(),
        }
    }

    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            tone: IssueTone::Warning,
            text: text.into(),
        }
    }
}

/// The worst tone among some issues: critical over warning, and `None` when there are none.
pub fn worst_issue(issues: &[NodeIssue]) -> Option<IssueTone> {
    if issues.iter().any(|i| i.tone == IssueTone::Critical) {
        Some(IssueTone::Critical)
    } else if issues.is_empty() {
        None
    } else {
        Some(IssueTone::Warning)
    }
}

/// What painting a box left where a reader can point: its issue mark, if it has one.
#[derive(Clone, Copy, Debug, Default)]
pub struct NodePainted {
    pub mark: Option<Rect>,
}

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
    /// How long the step took, at the code line's end.
    pub duration: Option<String>,
    /// The box's edge in this hue, 2 wide over a wash of it, as a step that failed.
    pub tint: Option<Color32>,
    /// The box's edge dashed, as a step the run has not reached.
    pub dashed: bool,
    /// What is wrong with the node: the box takes the worst issue's edge, and a mark says them.
    pub issues: Vec<NodeIssue>,
    /// The box's edge in this color, as a run state gives it, and how wide.
    pub border: Option<Color32>,
    pub border_width: f32,
    /// The box's ground in place of the node's, as a failed step's wash.
    pub fill: Option<Color32>,
    /// The title in the muted ink, as a step skipped or cancelled.
    pub muted: bool,
    /// Where data enters rather than a step that runs: dashed on the canvas's ground, no shadow.
    pub source: bool,
    /// How far along the node is, from 0 to 1, drawn as a line along its foot in the status color.
    pub progress: Option<f32>,
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
            duration: None,
            tint: None,
            dashed: false,
            issues: Vec::new(),
            border: None,
            border_width: 1.0,
            fill: None,
            muted: false,
            source: false,
            progress: None,
        }
    }

    pub fn issues(mut self, issues: Vec<NodeIssue>) -> Self {
        self.issues = issues;
        self
    }

    /// The box's edge in `color`, `width` wide.
    pub fn border(mut self, color: Option<Color32>, width: f32) -> Self {
        self.border = color;
        self.border_width = width;
        self
    }

    pub fn fill(mut self, fill: Option<Color32>) -> Self {
        self.fill = fill;
        self
    }

    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    pub fn source(mut self, source: bool) -> Self {
        self.source = source;
        self
    }

    pub fn progress(mut self, progress: Option<f32>) -> Self {
        self.progress = progress.map(|p| p.clamp(0.0, 1.0));
        self
    }

    pub fn duration(mut self, duration: Option<String>) -> Self {
        self.duration = duration;
        self
    }

    pub fn tint(mut self, tint: Option<Color32>) -> Self {
        self.tint = tint;
        self
    }

    pub fn dashed(mut self, dashed: bool) -> Self {
        self.dashed = dashed;
        self
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
            + self.status.as_ref().map_or(0.0, |_| GAP + DOT)
            + if self.issues.is_empty() { 0.0 } else { GAP + MARK - 4.0 };
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
    pub fn paint(
        &self,
        painter: &Painter,
        t: &Tokens,
        rect: Rect,
        zoom: f32,
        hovered: bool,
        selected: bool,
    ) -> NodePainted {
        let mut painted = NodePainted::default();
        let radius = t.length("radius-node") * if self.round { 3.0 } else { 1.0 } * zoom;
        let corners = CornerRadius::same(radius.round() as u8);
        if !self.source {
            for shadow in t.shadows("elevation-node") {
                painter.add(shadow.as_shape(rect, corners));
            }
        }
        let issue = worst_issue(&self.issues);
        // the edge: the pointer's, else an issue's, else a state's, else the node's line
        let edge = if hovered {
            match issue {
                Some(tone) => t.color(tone.keys().0),
                None => t.color("muted-foreground"),
            }
        } else if let Some(tone) = issue {
            t.color_at(tone.keys().0, 0.8)
        } else if let Some(border) = self.border {
            border
        } else {
            t.color("node-line")
        };
        let ground = if self.source {
            t.color("canvas")
        } else {
            self.fill.unwrap_or(t.color("node"))
        };
        if self.source || (self.border.is_some() || issue.is_some()) && self.tint.is_none() && !self.dashed {
            let width = if issue.is_some() || hovered {
                1.0
            } else {
                self.border_width
            };
            if self.source {
                painter.rect_filled(rect, corners, ground);
                dashed_outline(painter, rect, zoom, Stroke::new(zoom, edge));
            } else {
                painter.rect(
                    rect,
                    corners,
                    ground,
                    Stroke::new(width * zoom, edge),
                    StrokeKind::Inside,
                );
            }
        } else {
            self.paint_plain(painter, t, rect, corners, zoom, edge);
        }
        if selected {
            let ring = rect.expand(2.0 * zoom);
            painter.rect_stroke(
                ring,
                CornerRadius::same((radius + 2.0 * zoom).round() as u8),
                Stroke::new(2.0 * zoom, t.color("foreground")),
                StrokeKind::Outside,
            );
        }
        if let Some(progress) = self.progress {
            let hue = self
                .status
                .as_ref()
                .map_or(t.color("status-running"), |status| status.color);
            let foot = Rect::from_min_max(pos2(rect.left(), rect.bottom() - PROGRESS * zoom), rect.right_bottom());
            let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
            painter.rect_filled(foot, CornerRadius::ZERO, hue.gamma_multiply(0.18));
            let done = Rect::from_min_size(foot.min, vec2(foot.width() * progress, foot.height()));
            painter.rect_filled(done, CornerRadius::ZERO, hue);
        }
        self.paint_words(painter, t, rect, zoom, issue, &mut painted);
        painted
    }

    /// The box as the runs page draws it: its node ground and line, a failed step's tint or a
    /// dashed edge for a step not reached.
    fn paint_plain(&self, painter: &Painter, t: &Tokens, rect: Rect, corners: CornerRadius, zoom: f32, edge: Color32) {
        match (self.tint, self.dashed) {
            (Some(tint), _) => {
                let wash = tint.gamma_multiply(0.08);
                painter.rect_filled(rect, corners, t.color("node"));
                painter.rect(rect, corners, wash, Stroke::new(2.0 * zoom, tint), StrokeKind::Inside);
            }
            (None, true) => {
                painter.rect_filled(rect, corners, t.color("node"));
                dashed_outline(painter, rect, zoom, Stroke::new(zoom, edge));
            }
            (None, false) => {
                painter.rect(
                    rect,
                    corners,
                    t.color("node"),
                    Stroke::new(zoom, edge),
                    StrokeKind::Inside,
                );
            }
        }
    }

    /// The words, the glyph, the status dot and the issue mark inside the box.
    fn paint_words(
        &self,
        painter: &Painter,
        t: &Tokens,
        rect: Rect,
        zoom: f32,
        issue: Option<IssueTone>,
        painted: &mut NodePainted,
    ) {
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
        if let Some(tone) = issue {
            // the mark reaches 6 into the box's padding, as `-mr-1.5`
            let mark = Rect::from_center_size(
                pos2(right + 6.0 * zoom - MARK * zoom / 2.0, title_mid),
                Vec2::splat(MARK * zoom),
            );
            sokkel_egui::icons::paint(
                &painter,
                mark.center(),
                tone.icon(),
                MARK_GLYPH * zoom,
                t.color(tone.keys().1),
            );
            painted.mark = Some(mark);
            right = mark.left() - GAP * zoom;
        }
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
            if self.muted { muted } else { t.color("foreground") },
        );
        if self.two_lines() {
            let line = self.line();
            if !line.is_empty() {
                let line_mid = top + (title_line + 2.0 + small_line / 2.0) * zoom;
                let galley = small_mono(&painter, t, &line, zoom);
                painter.galley(pos2(inner.left(), line_mid - galley.size().y / 2.0), galley, muted);
            }
            if let Some(duration) = &self.duration {
                let line_mid = top + (title_line + 2.0 + small_line / 2.0) * zoom;
                let galley = small_mono(&painter, t, duration, zoom);
                painter.galley(
                    pos2(inner.right() - galley.size().x, line_mid - galley.size().y / 2.0),
                    galley,
                    muted,
                );
            }
        }
    }
}

/// A box's edge dashed, as a step not reached or a source.
fn dashed_outline(painter: &Painter, rect: Rect, zoom: f32, stroke: Stroke) {
    let r = rect.shrink(0.5 * zoom);
    let outline = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
        r.left_top(),
    ];
    painter.extend(Shape::dashed_line(&outline, stroke, 3.0 * zoom, 2.0 * zoom));
}

/// The title: the name in the graph's title role, or the code in the code face, both semibold.
fn title_galley(painter: &Painter, t: &Tokens, text: &str, named: bool, zoom: f32) -> std::sync::Arc<egui::Galley> {
    let size = t.text("graph-title").size;
    if named {
        let rich = TextRole::GraphTitle
            .rich(t, text)
            .size(size * zoom)
            .line_height(Some(t.text("graph-title").line_height * zoom))
            .variation("wght", t.weight(sokkel_egui::Weight::Semibold));
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
            mono_font(painter.ctx(), size * zoom, t.weight(sokkel_egui::Weight::Semibold)),
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
