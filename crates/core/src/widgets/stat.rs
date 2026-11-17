//! A number with what it counts, after sokkel-ui's `Stat`, and a row of them in one box, after
//! its dashboard's `KpiRow`.

use std::sync::Arc;

use egui::{Color32, Galley, Rect, Response, Sense, Shape, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// One number with what it counts: its label over it, a unit and a change beside it, a note under
/// it. The change is inked by whether it is good, which the caller says, since a rise in failures
/// and a rise in runs mean opposite things.
#[derive(Clone, Debug)]
pub struct Stat {
    label: String,
    value: String,
    unit: Option<String>,
    change: Option<(String, Option<bool>)>,
    note: Option<String>,
    dot: Option<Color32>,
    trend: Option<(Vec<f64>, Color32)>,
    emphasis: bool,
    large: bool,
    min_height: f32,
}

impl Stat {
    pub fn new(label: impl Into<String>, value: impl ToString) -> Self {
        Self {
            label: label.into(),
            value: value.to_string(),
            unit: None,
            change: None,
            note: None,
            dot: None,
            trend: None,
            emphasis: false,
            large: false,
            min_height: 0.0,
        }
    }

    /// A dot before the label in `color`, such as a state's.
    pub fn dot(mut self, color: Color32) -> Self {
        self.dot = Some(color);
        self
    }

    /// A small line under the number: its last values, in `ink` (a `Sparkline`).
    pub fn trend(mut self, values: impl IntoIterator<Item = f64>, ink: Color32) -> Self {
        self.trend = Some((values.into_iter().collect(), ink));
        self
    }

    /// The number at the hero statistic's size, as the web's `size="lg"`.
    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    /// Edges a tile in its dot's color: the number that asks for work.
    pub fn emphasis(mut self, emphasis: bool) -> Self {
        self.emphasis = emphasis;
        self
    }

    /// The stat as a card of its own, `width` wide, pressed as one (`Stat` `variant="tile"` with an
    /// `href`): the label in the label's type after its dot, the number at the statistic's size, 4
    /// further down, the note in the help's type, and the trend 4 under it.
    pub fn tile(self, ui: &mut Ui, width: f32) -> Response {
        let t = tokens(ui.ctx());
        let inner = width - 32.0;
        let lay = |rich: egui::RichText, role: TextRole, wrap: TextWrapMode| {
            WidgetText::from(rich).into_galley(ui, Some(wrap), inner.max(1.0), role.font(&t))
        };
        // a large dot, 10 across, 8 before the label
        let lead = if self.dot.is_some() { 18.0 } else { 0.0 };
        let label = WidgetText::from(TextRole::Label.rich(&t, &self.label)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            (inner - lead).max(1.0),
            TextRole::Label.font(&t),
        );
        let role = if self.large {
            TextRole::StatisticHero
        } else {
            TextRole::Statistic
        };
        let value = lay(role.rich(&t, &self.value), role, TextWrapMode::Truncate);
        let unit = self
            .unit
            .as_ref()
            .map(|unit| lay(TextRole::Body.rich(&t, unit), TextRole::Body, TextWrapMode::Extend));
        let note = self
            .note
            .as_ref()
            .map(|note| lay(TextRole::Help.rich(&t, note), TextRole::Help, TextWrapMode::Wrap));
        let height = 16.0
            + label.size().y
            + 8.0
            + value.size().y
            + note.as_ref().map_or(0.0, |n| 4.0 + n.size().y)
            + self.trend.as_ref().map_or(0.0, |_| 8.0 + SPARK.y)
            + 16.0;
        let (rect, response) = ui.allocate_exact_size(vec2(width, height.max(self.min_height)), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.label));
        let painter = ui.painter();
        let radius = t.radius("surface");
        let hot = response.hovered();
        painter.rect_filled(rect, radius, if hot { t.color("hover") } else { t.color("card") });
        let edge = match (self.emphasis, self.dot) {
            (true, Some(dot)) => dot.gamma_multiply(0.55),
            _ if hot => t.color_at("foreground", 0.25),
            _ => t.color("border-strong"),
        };
        painter.rect_stroke(rect, radius, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
        let mut y = rect.top() + 16.0;
        let x = rect.left() + 16.0;
        if let Some(dot) = self.dot {
            painter.circle_filled(pos2(x + 5.0, y + label.size().y / 2.0), 5.0, dot);
        }
        painter.galley(pos2(x + lead, y), label.clone(), t.color("foreground"));
        y += label.size().y + 8.0;
        let line = baseline(&value);
        painter.galley(pos2(x, y), value.clone(), t.color("card-foreground"));
        if let Some(unit) = unit {
            painter.galley(
                pos2(x + value.size().x + 8.0, y + line - baseline(&unit)),
                unit,
                t.color("muted-foreground"),
            );
        }
        y += value.size().y;
        if let Some(note) = note {
            y += 4.0;
            let height = note.size().y;
            painter.galley(pos2(x, y), note, t.color("muted-foreground"));
            y += height;
        }
        if let Some((values, ink)) = &self.trend {
            y += 8.0;
            sparkline(painter, Rect::from_min_size(pos2(x, y), SPARK), values, *ink);
        }
        super::focus_ring(painter, &t, rect, radius, &response);
        response
    }

    /// What the number is counted in, after it: `/s`, `ms`.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// The change against an earlier period, written for a reader (`+12%`), and whether it is an
    /// improvement; `None` leaves it neutral.
    pub fn change(mut self, change: impl Into<String>, good: Option<bool>) -> Self {
        self.change = Some((change.into(), good));
        self
    }

    /// A short qualifier under the number, wrapping in its room: `INC-41`, `vs last week`.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// How tall the stat stands `width` wide: its label's line, 4, the number's, and 4 and its note.
    fn height(&self, ui: &Ui, width: f32) -> f32 {
        let (label, value, _, note) = self.galleys(ui, width);
        label.size().y + 4.0 + value.size().y + note.map_or(0.0, |n| 4.0 + n.size().y)
    }

    fn galleys(&self, ui: &Ui, width: f32) -> Laid {
        let t = tokens(ui.ctx());
        let lay = |rich: egui::RichText, role: TextRole, wrap: TextWrapMode| {
            WidgetText::from(rich).into_galley(ui, Some(wrap), width.max(1.0), role.font(&t))
        };
        let label = lay(
            TextRole::Help.rich(&t, &self.label),
            TextRole::Help,
            TextWrapMode::Truncate,
        );
        // the number at the page's size, semibold, as `text-page font-semibold`
        let value = lay(
            TextRole::PageTitle.rich(&t, &self.value).variation("wght", 600.0),
            TextRole::PageTitle,
            TextWrapMode::Truncate,
        );
        let unit = self
            .unit
            .as_ref()
            .map(|unit| lay(TextRole::Body.rich(&t, unit), TextRole::Body, TextWrapMode::Extend));
        let note = self
            .note
            .as_ref()
            .map(|note| lay(TextRole::Help.rich(&t, note), TextRole::Help, TextWrapMode::Wrap));
        (label, value, unit, note)
    }

    /// Draw it with its top-left at `at`, `width` wide.
    fn paint(&self, ui: &Ui, at: egui::Pos2, width: f32) {
        let t = tokens(ui.ctx());
        let painter = ui.painter();
        let (label, value, unit, note) = self.galleys(ui, width);
        let mut y = at.y;
        painter.galley(pos2(at.x, y), label.clone(), t.color("muted-foreground"));
        y += label.size().y + 4.0;
        let line = baseline(&value);
        let mut x = at.x + value.size().x;
        painter.galley(pos2(at.x, y), value.clone(), t.color("foreground"));
        // the unit and the change 8 after the number, on its baseline
        if let Some(unit) = unit {
            x += 8.0;
            painter.galley(
                pos2(x, y + line - baseline(&unit)),
                unit.clone(),
                t.color("muted-foreground"),
            );
            x += unit.size().x;
        }
        if let Some((change, good)) = &self.change {
            x += 8.0;
            let ink = match good {
                None => t.color("muted-foreground"),
                Some(true) => t.color("good"),
                Some(false) => t.color("critical"),
            };
            let rising = !change.starts_with('-');
            let rich = TextRole::Help.rich(&t, change).variation("wght", 500.0);
            let words = WidgetText::from(rich).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextRole::Help.font(&t),
            );
            let top = y + line - baseline(&words);
            icons::paint(
                painter,
                pos2(x + 7.0, top + words.size().y / 2.0),
                if rising {
                    Icon::ArrowUpRight
                } else {
                    Icon::ArrowDownRight
                },
                14.0,
                ink,
            );
            painter.galley(pos2(x + 16.0, top), words, ink);
        }
        y += value.size().y;
        if let Some(note) = note {
            painter.galley(pos2(at.x, y + 4.0), note, t.color("faint"));
        }
    }
}

/// A sparkline's size, as the web's default: 120 by 28.
const SPARK: egui::Vec2 = egui::vec2(120.0, 28.0);

/// A line of values drawn small in `rect`, with no axes, its last value marked with a dot, after
/// sokkel-ui's `Sparkline`: the trend under a stat. Values are spaced evenly, 2 in from the edges.
pub fn sparkline(painter: &egui::Painter, rect: Rect, values: &[f64], ink: Color32) {
    if values.is_empty() {
        return;
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = if max - min == 0.0 { 1.0 } else { max - min };
    let step = if values.len() > 1 {
        (rect.width() - 4.0) / (values.len() - 1) as f32
    } else {
        0.0
    };
    let points: Vec<egui::Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            pos2(
                rect.left() + 2.0 + i as f32 * step,
                rect.top() + 2.0 + (rect.height() - 4.0) * (1.0 - ((v - min) / span) as f32),
            )
        })
        .collect();
    let last = *points.last().unwrap_or(&rect.center());
    painter.add(Shape::line(points, Stroke::new(1.5, ink)));
    painter.circle_filled(last, 2.25, ink);
}

/// Where a galley's first line stands, from its top.
fn baseline(galley: &Galley) -> f32 {
    galley
        .rows
        .first()
        .and_then(|r| r.glyphs.first().map(|g| r.pos.y + g.pos.y))
        .unwrap_or(galley.size().y)
}

/// A stat laid out: its label, its value, its unit and its note.
type Laid = (Arc<Galley>, Arc<Galley>, Option<Arc<Galley>>, Option<Arc<Galley>>);

/// A row of stats in one box, after sokkel-ui's `KpiRow`: as many equal columns as stats (two on a
/// phone), each 16 in, ruled apart, the box as tall as its tallest.
pub struct KpiRow {
    items: Vec<Stat>,
}

impl KpiRow {
    pub fn new(items: impl IntoIterator<Item = Stat>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let columns = if super::is_phone(ui.ctx()) {
            2
        } else {
            self.items.len().max(1)
        };
        let cell = width / columns as f32;
        let rows: Vec<&[Stat]> = self.items.chunks(columns).collect();
        let heights: Vec<f32> = rows
            .iter()
            .map(|row| row.iter().map(|s| s.height(ui, cell - 32.0)).fold(0.0, f32::max) + 32.0)
            .collect();
        let (rect, response) = ui.allocate_exact_size(vec2(width, heights.iter().sum()), Sense::hover());
        let radius = t.radius("surface");
        ui.painter().rect_filled(rect, radius, t.color("card"));
        let rule = Stroke::new(1.0, t.color("border"));
        let mut top = rect.top();
        for (r, row) in rows.iter().enumerate() {
            let height = heights[r];
            for (c, stat) in row.iter().enumerate() {
                let left = rect.left() + c as f32 * cell;
                stat.paint(ui, pos2(left + 16.0, top + 16.0), cell - 32.0);
                if c + 1 < row.len() {
                    ui.painter().vline(left + cell - 0.5, top..=top + height, rule);
                }
            }
            if r + 1 < rows.len() {
                ui.painter().hline(rect.x_range(), top + height - 0.5, rule);
            }
            top += height;
        }
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, t.color("border-strong")),
            egui::StrokeKind::Inside,
        );
        response
    }
}

/// Stat tiles in columns, every tile of a row as tall as its tallest, as the web's grid of
/// `Stat` tiles stretches them; answers with the tile pressed.
pub struct StatTiles {
    id: egui::Id,
    columns: crate::Columns,
}

impl StatTiles {
    /// `count` columns, each at least `min` wide, `gap` apart.
    pub fn new(id: impl egui::AsIdSalt, count: usize, min: f32, gap: f32) -> Self {
        Self {
            id: egui::Id::unique(id),
            columns: crate::Columns::new(count).min(min).gap(gap),
        }
    }

    pub fn show(self, ui: &mut Ui, tiles: Vec<Stat>) -> Option<usize> {
        let tallest: f32 = ui.data(|d| d.get_temp(self.id)).unwrap_or(0.0);
        let mut measured = 0.0_f32;
        let mut pressed = None;
        let count = tiles.len();
        let mut tiles: Vec<Option<Stat>> = tiles.into_iter().map(Some).collect();
        self.columns.show(ui, count, |ui, index, width| {
            if let Some(mut tile) = tiles[index].take() {
                tile.min_height = tallest;
                let response = tile.tile(ui, width);
                measured = measured.max(response.rect.height());
                if response.clicked() {
                    pressed = Some(index);
                }
            }
        });
        // a tile taller than the row was: the next frame stretches every tile to it
        if measured > tallest + 0.5 {
            ui.data_mut(|d| d.insert_temp(self.id, measured));
            ui.ctx().request_repaint();
        }
        pressed
    }
}
