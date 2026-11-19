//! Rows sized by a figure in nested rectangles, after sokkel-ui's `Chart kind="treemap"`: the
//! rows grouped by each field of a `path` in turn, every group's rectangle as large as its share,
//! laid out squarified so the rectangles stay near square.

use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, TextWrapMode, Ui, WidgetText, pos2, vec2};
use sokkel_egui::{TextRole, tokens};

use crate::data::{Row, label_of, number_of};
use crate::panel::{Head, chart_frame, drawn_under, fill_height};
use crate::{Format, SERIES_ORDER};

/// One group of the tree: its name, its total and what is under it.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeGroup {
    pub name: String,
    pub value: f64,
    pub children: Vec<TreeGroup>,
}

/// The rows grouped by each field of `path` in turn, each group's figure the sum of `y` under it,
/// the groups in the order they first come and none without a figure above nothing.
pub fn tree_of(rows: &[Row], path: &[&str], y: &str) -> Vec<TreeGroup> {
    let Some((field, rest)) = path.split_first() else {
        return Vec::new();
    };
    let mut groups: Vec<(String, Vec<Row>)> = Vec::new();
    for row in rows {
        let name = label_of(row.get(field));
        match groups.iter_mut().find(|(one, _)| *one == name) {
            Some((_, kept)) => kept.push(row.clone()),
            None => groups.push((name, vec![row.clone()])),
        }
    }
    groups
        .into_iter()
        .map(|(name, kept)| {
            let value = kept.iter().filter_map(|row| number_of(row.get(y))).sum();
            TreeGroup {
                name,
                value,
                children: tree_of(&kept, rest, y),
            }
        })
        .filter(|group| group.value > 0.0)
        .collect()
}

/// The rectangles for `values` (largest first works best) in `rect`, squarified: each row of
/// rectangles laid along the shorter side while that keeps their worst aspect from growing.
pub fn squarify(values: &[f64], rect: Rect) -> Vec<Rect> {
    let total: f64 = values.iter().sum();
    let mut out = vec![Rect::NOTHING; values.len()];
    if total <= 0.0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return out;
    }
    let scale = f64::from(rect.width() * rect.height()) / total;
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|a, b| values[*b].partial_cmp(&values[*a]).unwrap_or(std::cmp::Ordering::Equal));
    let mut room = rect;
    let mut at = 0;
    while at < order.len() {
        let side = f64::from(room.width().min(room.height()));
        let worst = |row: &[usize]| -> f64 {
            let sum: f64 = row.iter().map(|i| values[*i] * scale).sum();
            let most = row.iter().map(|i| values[*i] * scale).fold(0.0, f64::max);
            let least = row.iter().map(|i| values[*i] * scale).fold(f64::INFINITY, f64::min);
            (side * side * most / (sum * sum)).max(sum * sum / (side * side * least))
        };
        let mut end = at + 1;
        while end < order.len() && worst(&order[at..=end]) <= worst(&order[at..end]) {
            end += 1;
        }
        let row = &order[at..end];
        let sum: f64 = row.iter().map(|i| values[*i] * scale).sum();
        let thick = (sum / side) as f32;
        let mut along = 0.0;
        for i in row {
            let length = (values[*i] * scale / f64::from(thick.max(f32::EPSILON))) as f32;
            out[*i] = if room.width() >= room.height() {
                Rect::from_min_size(pos2(room.left(), room.top() + along), vec2(thick, length))
            } else {
                Rect::from_min_size(pos2(room.left() + along, room.top()), vec2(length, thick))
            };
            along += length;
        }
        room = if room.width() >= room.height() {
            Rect::from_min_max(pos2(room.left() + thick, room.top()), room.max)
        } else {
            Rect::from_min_max(pos2(room.left(), room.top() + thick), room.max)
        };
        at = end;
    }
    out
}

/// A treemap in a chart's panel: the top groups each in a series color, the groups under them
/// edged in the card's ground with their names over them where they fit, the leaves named with
/// their figure; under the pointer, the path to a leaf and its figure.
pub struct TreemapPanel<'a> {
    title: String,
    subtitle: Option<String>,
    rows: &'a [Row],
    path: Vec<String>,
    y: String,
    format: Option<Format>,
    height: f32,
}

impl<'a> TreemapPanel<'a> {
    pub fn new(title: impl Into<String>, rows: &'a [Row], path: &[&str], y: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            rows,
            path: path.iter().map(|one| (*one).to_owned()).collect(),
            y: y.into(),
            format: None,
            height: 256.0,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// How a figure is written in the labels and the tooltip.
    pub fn format(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Some(std::sync::Arc::new(format));
        self
    }

    /// The map's height, 256 when not given.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let head = Head {
            subtitle: self.subtitle.as_deref(),
            ..Head::new(&self.title)
        };
        let path: Vec<&str> = self.path.iter().map(String::as_str).collect();
        let tree = tree_of(self.rows, &path, &self.y);
        let format = self.format.clone();
        let written = move |value: f64| match &format {
            Some(format) => format(value),
            None => crate::data::format_number(value),
        };
        chart_frame(ui, head, |ui| {
            let height = fill_height(ui, self.height);
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let t = tokens(ui.ctx());
            let values: Vec<f64> = tree.iter().map(|one| one.value).collect();
            let places = squarify(&values, rect);
            let mut hovered: Option<(String, f64)> = None;
            let pointer = response.hover_pos();
            for (index, (group, place)) in tree.iter().zip(&places).enumerate() {
                let color = t.color(SERIES_ORDER[index % SERIES_ORDER.len()]);
                paint_group(
                    ui,
                    &t,
                    group,
                    *place,
                    color,
                    0,
                    &group.name,
                    &written,
                    pointer,
                    &mut hovered,
                );
            }
            if let Some((said, value)) = hovered {
                response.on_hover_text_at_pointer(format!("{said}: {}", written(value)));
            }
            drawn_under(ui, ui.min_rect().bottom());
        });
    }
}

/// One group's rectangle and those under it: a top group in its color with its name in a band at
/// its top, those under it laid out in what is left, a leaf named with its figure.
#[allow(clippy::too_many_arguments)]
fn paint_group(
    ui: &Ui,
    t: &sokkel_egui::Tokens,
    group: &TreeGroup,
    place: Rect,
    color: Color32,
    depth: usize,
    said: &str,
    written: &dyn Fn(f64) -> String,
    pointer: Option<egui::Pos2>,
    hovered: &mut Option<(String, f64)>,
) {
    if place.width() < 1.0 || place.height() < 1.0 {
        return;
    }
    let painter = ui.painter();
    let gap = t.color("card");
    if group.children.is_empty() {
        let fill = color.gamma_multiply(match depth {
            0 => 1.0,
            1 => 0.85,
            _ => 0.7,
        });
        painter.rect_filled(place, CornerRadius::same(2), fill);
        painter.rect_stroke(place, CornerRadius::same(2), Stroke::new(1.0, gap), StrokeKind::Inside);
        let room = place.width() - 8.0;
        if room > 24.0 && place.height() > 20.0 {
            let ink = t.color("primary-foreground");
            let name = WidgetText::from(TextRole::Help.rich(t, &group.name).color(ink)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                room,
                TextRole::Help.font(t),
            );
            let top = place.top() + 4.0;
            let tall = name.size().y;
            painter.galley(pos2(place.left() + 4.0, top), name, ink);
            if place.height() > tall * 2.0 + 8.0 {
                painter.text(
                    pos2(place.left() + 4.0, top + tall),
                    Align2::LEFT_TOP,
                    written(group.value),
                    TextRole::Help.font(t),
                    ink,
                );
            }
        }
        if pointer.is_some_and(|at| place.contains(at)) {
            *hovered = Some((said.to_owned(), group.value));
        }
        return;
    }
    // a band with the group's name over those under it, where there is room for one
    let band = if place.height() > 48.0 && place.width() > 40.0 {
        20.0
    } else {
        0.0
    };
    painter.rect_filled(place, CornerRadius::same(2), color.gamma_multiply(0.35));
    if band > 0.0 {
        let ink = t.color("foreground");
        let name = WidgetText::from(TextRole::Help.rich(t, &group.name).color(ink)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            place.width() - 8.0,
            TextRole::Help.font(t),
        );
        painter.galley(pos2(place.left() + 4.0, place.top() + 2.0), name, ink);
    }
    let inner = Rect::from_min_max(
        pos2(place.left() + 2.0, place.top() + band + 2.0),
        place.max - vec2(2.0, 2.0),
    );
    let values: Vec<f64> = group.children.iter().map(|one| one.value).collect();
    for (child, at) in group.children.iter().zip(squarify(&values, inner)) {
        let path = format!("{said} / {}", child.name);
        paint_group(ui, t, child, at, color, depth + 1, &path, written, pointer, hovered);
    }
    painter.rect_stroke(place, CornerRadius::same(2), Stroke::new(1.0, gap), StrokeKind::Inside);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Cell;

    #[test]
    fn rows_group_by_the_path_and_squarify_fills_the_room() {
        let rows: Vec<Row> = [
            ("hub", "packed", 4.0),
            ("hub", "packed", 2.0),
            ("hub", "unified", 2.0),
            ("runner", "packed", 1.0),
        ]
        .into_iter()
        .map(|(source, format, size)| {
            Row::of([
                ("source", Cell::from(source)),
                ("format", Cell::from(format)),
                ("size", Cell::Number(size)),
            ])
        })
        .collect();
        let tree = tree_of(&rows, &["source", "format"], "size");
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].value, 8.0);
        assert_eq!(tree[0].children[0].value, 6.0);
        let room = Rect::from_min_size(pos2(0.0, 0.0), vec2(90.0, 10.0));
        let places = squarify(&[6.0, 2.0, 1.0], room);
        let area: f32 = places.iter().map(|one| one.area()).sum();
        assert!((area - 900.0).abs() < 0.5, "{area}");
        assert!(places.iter().all(|one| room.expand(0.01).contains_rect(*one)));
    }
}
