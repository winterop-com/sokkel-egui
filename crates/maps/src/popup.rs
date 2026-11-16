//! A card over the map pointing at the chosen marker, after sokkel-ui-maps' `MapPopup`.

use egui::{Align2, Color32, Key, Rect, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};
use sokkel_ui_egui::{Button, ButtonSize, ButtonVariant, Icon, IconButton, TextRole, tokens};

use crate::MapView;

/// The gap between the marker and the card, the tip included.
const GAP: f32 = 10.0;
/// How close the card comes to the edge of the map.
const EDGE: f32 = 8.0;

/// One line of a popup: a name, its value, and the color the value is drawn in on the map.
#[derive(Clone, Debug)]
pub struct MapPopupRow {
    pub label: String,
    pub value: String,
    pub color: Option<Color32>,
}

impl MapPopupRow {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            color: None,
        }
    }

    /// A swatch of this color beside the value.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

/// The words a map popup says.
#[derive(Clone, Debug)]
pub struct MapPopupWords {
    pub close: String,
}

impl Default for MapPopupWords {
    fn default() -> Self {
        Self { close: "Close".into() }
    }
}

/// What happened to a popup this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapPopupResponse {
    /// Closed by its button or Escape; the map's marker is let go.
    pub closed: bool,
    /// Its action pressed.
    pub acted: bool,
}

/// A card over the map, pointing at the chosen marker: a title, a line under it, rows of names
/// and values, and an action. It sits above its marker, or below when there is no room above,
/// keeps 8 inside the map's edges, hides while its marker is out of view, and closes on Escape.
pub struct MapPopup {
    title: String,
    description: Option<String>,
    rows: Vec<MapPopupRow>,
    action: Option<String>,
    words: MapPopupWords,
}

impl MapPopup {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            rows: Vec::new(),
            action: None,
            words: MapPopupWords::default(),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn row(mut self, row: MapPopupRow) -> Self {
        self.rows.push(row);
        self
    }

    /// A ghost button across the card's foot.
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.action = Some(label.into());
        self
    }

    pub fn words(mut self, words: MapPopupWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the card at the map's chosen marker, after the map has been drawn this frame; nothing
    /// when no marker is chosen.
    pub fn show(self, ui: &mut Ui, map: &mut MapView) -> MapPopupResponse {
        let mut answer = MapPopupResponse::default();
        let Some(point) = map.selected.as_deref().and_then(|id| map.point_of(id)) else {
            return answer;
        };
        let room = map.room();
        if !room.contains(point) {
            return answer;
        }
        // the card stands in the map's layer, over a map gone full screen too
        let mut layered = ui.new_child(UiBuilder::new().layer_id(map.layer()).max_rect(room));
        let ui = &mut layered;
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            map.selected = None;
            answer.closed = true;
            return answer;
        }
        let t = tokens(ui.ctx());
        let line: f32 = 20.0;
        let width = 256.0_f32.min(room.width() - EDGE * 2.0);
        let block = 2.0 + line + if self.description.is_some() { line } else { 0.0 } + 2.0;
        let head = 8.0 + block.max(t.density("control-xs")) + 8.0;
        let rows = if self.rows.is_empty() {
            0.0
        } else {
            self.rows.len() as f32 * line + (self.rows.len() - 1) as f32 * 4.0 + 10.0
        };
        let foot = if self.action.is_some() {
            1.0 + 4.0 + t.density("control-sm") + 4.0
        } else {
            0.0
        };
        let height = head + rows + foot;

        let above = point.y - GAP - height >= room.top() + EDGE;
        let below = point.y + GAP + height <= room.bottom() - EDGE;
        let up = above || !below;
        let left = (point.x - width / 2.0).clamp(
            room.left() + EDGE,
            (room.right() - width - EDGE).max(room.left() + EDGE),
        );
        let top = if up { point.y - GAP - height } else { point.y + GAP };
        let card = Rect::from_min_size(pos2(left, top), vec2(width, height));
        let tip_x = point.x.clamp(card.left() + 14.0, card.right() - 14.0);

        // the card takes the pointer, so pressing it does not press the map under it
        let _ = ui.interact(card, ui.id().with("map-popup"), Sense::click());
        let ink = t.color("popover-foreground");
        let ring = t.color_at("foreground", 0.10);
        let painter = ui.painter().clone();
        painter.add(t.shadow("elevation-pop").as_shape(card, t.radius("surface")));
        painter.rect(
            card,
            t.radius("surface"),
            t.color("popover"),
            Stroke::new(1.0, ring),
            StrokeKind::Outside,
        );
        // the tip, a corner of a turned square showing past the card's edge
        let edge_y = if up { card.bottom() } else { card.top() };
        let reach = if up { 5.5 } else { -5.5 };
        let tip = [
            pos2(tip_x - 5.5, edge_y),
            pos2(tip_x, edge_y + reach),
            pos2(tip_x + 5.5, edge_y),
        ];
        painter.add(Shape::convex_polygon(tip.to_vec(), t.color("popover"), Stroke::NONE));
        painter.add(Shape::line(tip.to_vec(), Stroke::new(1.0, ring)));

        // the head: title and description, the close button at the right
        let text_left = card.left() + 12.0;
        let mut y = card.top() + 8.0 + 2.0;
        let title = TextRole::Label.rich(&t, &self.title).color(ink);
        let mut text_ui = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(
            pos2(text_left, y),
            pos2(card.right() - 6.0 - t.density("control-xs") - 8.0, y + block),
        )));
        text_ui.spacing_mut().item_spacing.y = 0.0;
        text_ui.add(egui::Label::new(title).truncate());
        if let Some(description) = &self.description {
            text_ui.add(
                egui::Label::new(TextRole::Help.rich(&t, description).color(t.color("muted-foreground"))).truncate(),
            );
        }
        let close_side = t.density("control-xs");
        let close = Rect::from_min_size(
            pos2(card.right() - 6.0 - close_side, card.top() + 8.0),
            vec2(close_side, close_side),
        );
        let mut close_ui = ui.new_child(UiBuilder::new().max_rect(close));
        if close_ui.add(IconButton::new(Icon::X, &self.words.close).xs()).clicked() {
            map.selected = None;
            answer.closed = true;
        }

        // the rows: names at the left, values at the right with their swatch
        y = card.top() + head;
        for row in &self.rows {
            let middle = y + line / 2.0;
            painter.text(
                pos2(text_left, middle),
                Align2::LEFT_CENTER,
                &row.label,
                TextRole::Help.font(&t),
                t.color("muted-foreground"),
            );
            let value = painter.layout_no_wrap(row.value.clone(), TextRole::Badge.font(&t), ink);
            let value_left = card.right() - 12.0 - value.size().x;
            painter.galley(pos2(value_left, middle - value.size().y / 2.0), value, ink);
            if let Some(color) = row.color {
                let swatch = Rect::from_center_size(pos2(value_left - 6.0 - 5.0, middle), vec2(10.0, 10.0));
                painter.rect(
                    swatch,
                    t.radius("mini"),
                    color,
                    Stroke::new(1.0, t.color_at("foreground", 0.15)),
                    StrokeKind::Inside,
                );
            }
            y += line + 4.0;
        }

        // the action across the foot, under a rule
        if let Some(action) = &self.action {
            let rule = card.bottom() - foot;
            painter.hline(card.x_range(), rule, Stroke::new(1.0, t.color("border")));
            let place = Rect::from_min_max(
                pos2(card.left() + 4.0, rule + 1.0 + 4.0),
                pos2(card.right() - 4.0, card.bottom() - 4.0),
            );
            let mut action_ui = ui.new_child(UiBuilder::new().max_rect(place));
            let button = Button::new(action.as_str())
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Sm);
            if action_ui.add(button).clicked() {
                answer.acted = true;
            }
        }
        answer
    }
}
