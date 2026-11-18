//! What is held over a canvas, after sokkel-ui-graph's `GraphOverlay`: something such as a block
//! picker beside a place in the graph or on the screen, and `GraphNotice`, a short message across
//! the canvas's foot.

use egui::{Area, Id, Key, Modifiers, Order, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use sokkel_egui::{Icon, TextRole, tokens};

use crate::canvas::{CORNER, GraphControls, rough_route, rounded};

/// How far an overlay keeps from the canvas's edges and from its anchor.
const MARGIN: f32 = 8.0;
/// The least height an overlay's content is let have, however little room there is.
const LEAST_ROOM: f32 = 120.0;
/// The link line's width and dashes, and the dot at its end.
const LINK: f32 = 2.0;
const LINK_DOT: f32 = 4.0;
/// A notice's margin from the canvas's foot, its padding, and its parts' sizes.
const NOTICE_MARGIN: f32 = 12.0;
const NOTICE_PAD_LEFT: f32 = 10.0;
const NOTICE_PAD_RIGHT: f32 = 6.0;
const NOTICE_PAD_Y: f32 = 6.0;
const NOTICE_GAP: f32 = 8.0;
const NOTICE_GLYPH: f32 = 16.0;
const NOTICE_CLOSE: f32 = 24.0;
const NOTICE_CLOSE_GLYPH: f32 = 14.0;

/// Where an overlay stands. In the graph, following the view as it pans and zooms: a point in the
/// graph's coordinates, a point beside a node's output (`dx` and `dy` away from its port), or the
/// middle of an edge, by its id. On the screen, standing still: a point such as a pointer's, a
/// rectangle such as the button that opened it, or the middle of the canvas, for one opened from
/// the keyboard.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphAnchor {
    Point(Pos2),
    Node { node: String, dx: f32, dy: f32 },
    Edge(String),
    Screen(Pos2),
    Rect(Rect),
    Center,
}

/// What an overlay is placed beside, in the canvas's own pixels from its corner: a `Point` it opens
/// below and to the right of, a `Box` it opens below or above without covering, or the `Center`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OverlayTarget {
    Point(Pos2),
    Box(Rect),
    Center,
}

/// Where an overlay of `size` stands on a canvas of `canvas` beside `target`, its top left from the
/// canvas's corner, and the most height its content has. Beside a point it opens below and to the
/// right, turning to the side with room. Beside a box it opens below it, lined up with its start,
/// or above it when there is no room below, and on the side with more room when neither holds it.
pub fn place_overlay(target: OverlayTarget, size: Vec2, canvas: Vec2) -> (Pos2, f32) {
    let most = vec2(canvas.x - MARGIN - size.x, canvas.y - MARGIN - size.y);
    let within = |value: f32, highest: f32| value.min(highest).max(MARGIN);
    match target {
        OverlayTarget::Center => (
            pos2(
                within((canvas.x - size.x) / 2.0, most.x),
                within((canvas.y - size.y) / 2.0, most.y),
            ),
            LEAST_ROOM.max(canvas.y - MARGIN * 2.0),
        ),
        OverlayTarget::Point(p) => {
            let mut left = p.x + MARGIN;
            let mut top = p.y + MARGIN;
            if left + size.x > canvas.x - MARGIN {
                left = (p.x - MARGIN - size.x).min(most.x).max(MARGIN);
            }
            if top + size.y > canvas.y - MARGIN {
                top = (p.y - MARGIN - size.y).min(most.y).max(MARGIN);
            }
            (pos2(left, top), LEAST_ROOM.max(canvas.y - MARGIN * 2.0))
        }
        OverlayTarget::Box(b) => {
            let below = MARGIN.max(b.bottom() + MARGIN);
            let above = (canvas.y - MARGIN).min(b.top() - MARGIN);
            let room_below = canvas.y - MARGIN - below;
            let room_above = above - MARGIN;
            let mut left = b.left();
            if left + size.x > canvas.x - MARGIN {
                left = b.right() - size.x;
            }
            let left = within(left, most.x);
            let top = if size.y <= room_below || (size.y > room_above && room_below >= room_above) {
                below
            } else {
                MARGIN.max(above - size.y)
            };
            let room = LEAST_ROOM.max(if room_below >= room_above {
                room_below
            } else {
                room_above
            });
            (pos2(left, top), room)
        }
    }
}

/// What an overlay answers with: what its content gave back, and whether Escape or a press outside
/// asked it to close.
pub struct GraphOverlayResponse<R> {
    pub inner: Option<R>,
    pub closed: bool,
    /// The overlay on the screen.
    pub rect: Rect,
}

/// Something held over a canvas, such as a `BlockPicker`: beside a place in the graph, following it
/// as the view pans and zooms, or beside a place on the screen, such as the button that opened it.
/// It opens on the side with room, never over the button it stands beside, and stays inside the
/// canvas, its content told the height and width it has. Given `link`, a dashed line runs from that
/// node's output to the place, routed in right-angled steps as the edges are, for a step about to be
/// added there. Escape and a press outside close it; a press on a `Rect` anchor, or on `ignore`,
/// does not, so the button that opened it can close it. Show it after the canvas, with the
/// controls the canvas answered with.
pub struct GraphOverlay {
    id: Id,
    at: GraphAnchor,
    link: Option<String>,
    ignore: Option<Rect>,
}

impl GraphOverlay {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, at: GraphAnchor) -> Self {
        Self {
            id: Id::unique(("sokkel-graph-overlay", id)),
            at,
            link: None,
            ignore: None,
        }
    }

    /// The node a pending connection is drawn from.
    pub fn link(mut self, node: Option<String>) -> Self {
        self.link = node;
        self
    }

    /// A rectangle a press on does not close the overlay, such as the button that toggles it.
    pub fn ignore(mut self, rect: Option<Rect>) -> Self {
        self.ignore = rect;
        self
    }

    /// Where the anchor stands, in the canvas's pixels, or `None` while what it names is not drawn.
    fn target(&self, controls: &GraphControls) -> Option<OverlayTarget> {
        let local = |screen: Pos2| (screen - controls.rect.min).to_pos2();
        match &self.at {
            GraphAnchor::Point(p) => Some(OverlayTarget::Point(local(controls.to_screen(*p)))),
            GraphAnchor::Node { node, dx, dy } => {
                let port = controls.port(node, true)?;
                Some(OverlayTarget::Point(local(controls.to_screen(port + vec2(*dx, *dy)))))
            }
            GraphAnchor::Edge(edge) => {
                let (edge, _) = controls.edges.iter().find(|(e, _)| &e.id == edge)?;
                let a = controls.port(&edge.source, true)?;
                let b = controls.port(&edge.target, false)?;
                Some(OverlayTarget::Point(local(controls.to_screen(a.lerp(b, 0.5)))))
            }
            GraphAnchor::Screen(p) => Some(OverlayTarget::Point(local(*p))),
            GraphAnchor::Rect(r) => Some(OverlayTarget::Box(r.translate(-controls.rect.min.to_vec2()))),
            GraphAnchor::Center => Some(OverlayTarget::Center),
        }
    }

    /// Show the overlay over the canvas `controls` describe; `add` draws its content, told the most
    /// height and the most width it has.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        controls: &GraphControls,
        add: impl FnOnce(&mut Ui, f32, f32) -> R,
    ) -> GraphOverlayResponse<R> {
        let t = tokens(ui.ctx());
        let canvas = controls.rect;
        let Some(target) = self.target(controls) else {
            return GraphOverlayResponse {
                inner: None,
                closed: false,
                rect: Rect::NOTHING,
            };
        };
        let size_id = self.id.with("size");
        let size: Vec2 = ui.data(|d| d.get_temp(size_id)).unwrap_or_default();
        let (left_top, room) = place_overlay(target, size, canvas.size());

        // the link from the node a step would follow to where it goes
        let end = match target {
            OverlayTarget::Center => canvas.center(),
            OverlayTarget::Point(p) => canvas.min + p.to_vec2(),
            OverlayTarget::Box(b) => canvas.min + b.center().to_vec2(),
        };
        if let Some(start) = self.link.as_deref().and_then(|node| controls.port(node, true)) {
            let start = controls.to_screen(start);
            let route = rough_route(
                Rect::from_min_size(start, Vec2::ZERO),
                Rect::from_min_size(end, Vec2::ZERO),
                controls.direction,
            );
            let line = rounded(&route, CORNER);
            let painter = ui.painter_at(canvas);
            painter.extend(Shape::dashed_line(&line, Stroke::new(LINK, t.color("ring")), 5.0, 4.0));
            painter.circle_filled(end, LINK_DOT, t.color("ring"));
        }

        let width = canvas.width() - MARGIN * 2.0;
        let shown = Area::new(self.id)
            .order(Order::Foreground)
            .fixed_pos(canvas.min + left_top.to_vec2())
            .constrain(false)
            .show(ui.ctx(), |ui| {
                ui.set_max_width(width);
                add(ui, room, width)
            });
        let rect = shown.response.rect;
        if (rect.size() - size).length() > 0.5 {
            ui.data_mut(|d| d.insert_temp(size_id, rect.size()));
            ui.ctx().request_repaint();
        }
        let escape = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        let outside = ui.input(|i| i.pointer.any_pressed())
            && ui.input(|i| i.pointer.press_origin()).is_some_and(|p| {
                let opener = match &self.at {
                    GraphAnchor::Rect(r) => r.contains(p),
                    _ => false,
                };
                !rect.contains(p) && !opener && !self.ignore.is_some_and(|r| r.contains(p))
            });
        GraphOverlayResponse {
            inner: Some(shown.inner),
            closed: escape || outside,
            rect,
        }
    }
}

/// The words a graph notice says.
#[derive(Clone, Debug)]
pub struct GraphNoticeWords {
    pub close: String,
}

impl Default for GraphNoticeWords {
    fn default() -> Self {
        Self {
            close: "Dismiss".into(),
        }
    }
}

/// What a notice says: a refusal (`Critical`) or anything else (`Info`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NoticeTone {
    Critical,
    #[default]
    Info,
}

/// A short message across the foot of a canvas: why a connection was refused, or what a gesture
/// did. Show it after the canvas, with the controls the canvas answered with; it answers true when
/// its close button was pressed.
pub struct GraphNotice {
    text: String,
    tone: NoticeTone,
    closable: bool,
    words: GraphNoticeWords,
}

impl GraphNotice {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: NoticeTone::Info,
            closable: true,
            words: GraphNoticeWords::default(),
        }
    }

    pub fn tone(mut self, tone: NoticeTone) -> Self {
        self.tone = tone;
        self
    }

    /// False leaves the close button out.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    pub fn words(mut self, words: GraphNoticeWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui, controls: &GraphControls) -> bool {
        let t = tokens(ui.ctx());
        let canvas = controls.rect;
        let critical = self.tone == NoticeTone::Critical;
        let close = if self.closable { NOTICE_CLOSE } else { 0.0 };
        let chrome = NOTICE_PAD_LEFT
            + NOTICE_GLYPH
            + NOTICE_GAP
            + NOTICE_PAD_RIGHT
            + if self.closable { NOTICE_GAP + close } else { 0.0 };
        let most = (canvas.width() - 2.0 * NOTICE_MARGIN - chrome).max(40.0);
        let mut job = egui::text::LayoutJob::default();
        TextRole::Body.rich(&t, &self.text).append_to(
            &mut job,
            ui.style(),
            egui::FontSelection::Default,
            egui::Align::Min,
        );
        job.wrap.max_width = most;
        let words = ui.painter().layout_job(job);
        let height = 2.0 * NOTICE_PAD_Y + (words.size().y + 2.0).max(close).max(NOTICE_GLYPH + 2.0);
        let width = chrome + words.size().x.ceil();
        let rect = Rect::from_min_size(
            pos2(
                canvas.center().x - width / 2.0,
                canvas.bottom() - NOTICE_MARGIN - height,
            ),
            vec2(width, height),
        );
        let painter = ui.painter_at(canvas);
        let id = controls.id.with("notice");
        let body = ui.interact(rect, id, Sense::click());
        body.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &self.text));
        let corners = t.radius("control");
        for shadow in t.shadows("elevation-pop") {
            painter.add(shadow.as_shape(rect, corners));
        }
        let edge = if critical {
            t.color_at("critical", 0.6)
        } else {
            t.color("border")
        };
        painter.rect(
            rect,
            corners,
            t.color("popover"),
            Stroke::new(1.0, edge),
            StrokeKind::Inside,
        );
        let (icon, ink) = if critical {
            (Icon::CircleAlert, t.color("critical-ink"))
        } else {
            (Icon::Info, t.color("muted-foreground"))
        };
        let top = rect.top() + NOTICE_PAD_Y;
        sokkel_egui::icons::paint(
            &painter,
            pos2(
                rect.left() + NOTICE_PAD_LEFT + NOTICE_GLYPH / 2.0,
                top + 2.0 + NOTICE_GLYPH / 2.0,
            ),
            icon,
            NOTICE_GLYPH,
            ink,
        );
        let words_at = pos2(rect.left() + NOTICE_PAD_LEFT + NOTICE_GLYPH + NOTICE_GAP, top + 1.0);
        painter.galley(words_at, words, t.color("popover-foreground"));
        if !self.closable {
            return false;
        }
        let spot = Rect::from_min_size(pos2(rect.right() - NOTICE_PAD_RIGHT - close, top), Vec2::splat(close));
        let response = ui.interact(spot, id.with("close"), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.close));
        let ink = if response.hovered() {
            painter.rect_filled(spot, t.radius("sm"), t.color("hover"));
            t.color("foreground")
        } else {
            t.color("muted-foreground")
        };
        sokkel_egui::icons::paint(&painter, spot.center(), Icon::X, NOTICE_CLOSE_GLYPH, ink);
        response.on_hover_text(&self.words.close).clicked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beside_a_point_it_opens_below_right_and_turns_where_there_is_no_room() {
        let canvas = vec2(500.0, 400.0);
        let size = vec2(200.0, 150.0);
        let (at, _) = place_overlay(OverlayTarget::Point(pos2(100.0, 100.0)), size, canvas);
        assert_eq!(at, pos2(108.0, 108.0));
        let (at, _) = place_overlay(OverlayTarget::Point(pos2(450.0, 380.0)), size, canvas);
        assert_eq!(at, pos2(242.0, 222.0));
    }

    #[test]
    fn beside_a_box_it_opens_above_when_below_has_no_room() {
        let canvas = vec2(500.0, 400.0);
        let button = Rect::from_min_size(pos2(460.0, 360.0), vec2(28.0, 28.0));
        let (at, room) = place_overlay(OverlayTarget::Box(button), vec2(256.0, 200.0), canvas);
        assert_eq!(at.y, 360.0 - 8.0 - 200.0);
        assert_eq!(at.x, 488.0 - 256.0);
        assert_eq!(room, 344.0);
    }
}
