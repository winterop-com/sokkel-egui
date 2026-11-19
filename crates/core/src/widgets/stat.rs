//! A number with what it counts, after sokkel-ui's `Stat`, and a row of them in one box, after
//! its dashboard's `KpiRow`.

use std::sync::Arc;

use egui::{Color32, Galley, Rect, Response, Sense, Shape, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// One number with what it counts, after sokkel-ui's `Stat`: its label over it (after a dot), a
/// unit and a change beside it, a note and a trend under it. The change is inked by whether it is
/// good, which the caller says, since a rise in failures and a rise in runs mean opposite things.
///
/// Added with `ui.add(...)` it is the plain stat, sitting in whatever holds it; [`Stat::tile`]
/// draws it as a card of its own. Either is a control only with [`Stat::href`] or
/// [`Stat::selectable`]: then the whole stat is one press, named by its label and value.
#[derive(Clone, Debug)]
#[must_use = "add it with `ui.add(...)` or draw it with `.tile(ui, width)`"]
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
    href: Option<String>,
    selectable: bool,
    min_height: f32,
    /// A state as a chip in place of the number, as the web's `value={<StatusChip />}`.
    chip: Option<(String, crate::ChipTone)>,
}

/// A stat's pieces laid out for its width.
struct Laid {
    label: Arc<Galley>,
    value: Arc<Galley>,
    unit: Option<Arc<Galley>>,
    change: Option<Arc<Galley>>,
    note: Option<Arc<Galley>>,
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
            href: None,
            selectable: false,
            min_height: 0.0,
            chip: None,
        }
    }

    /// A state as a chip where the number stands, in the number's line, as a tile whose value is a
    /// `StatusChip`.
    pub fn chip(mut self, label: impl Into<String>, tone: crate::ChipTone) -> Self {
        self.chip = Some((label.into(), tone));
        self
    }

    /// A dot before the label in `color`, such as a state's: small in a plain stat, large in a tile.
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

    /// Edges a tile in its dot's color at 55% alpha: the number that asks for work.
    pub fn emphasis(mut self, emphasis: bool) -> Self {
        self.emphasis = emphasis;
        self
    }

    /// Where the stat leads, such as a list filtered to what it counts: the stat becomes one
    /// control, and a press opens the address.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// The stat as one control whose press the app answers (the response's `clicked`), as the
    /// web's `onSelect`.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// What the number is counted in, after it: `/s`, `ms`.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// The change against an earlier period, written for a reader (`+12%`), and whether it is an
    /// improvement; `None` leaves it neutral. A change written with a minus turns the arrow down.
    pub fn change(mut self, change: impl Into<String>, good: Option<bool>) -> Self {
        self.change = Some((change.into(), good));
        self
    }

    /// A short qualifier under the number, wrapping in its room: `INC-41`, `vs last week`.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    fn interactive(&self) -> bool {
        self.href.is_some() || self.selectable
    }

    /// The name a screen reader says: the label and the value.
    fn name(&self) -> String {
        let value = match (&self.chip, &self.unit) {
            (Some((chip, _)), _) => chip.clone(),
            (None, Some(unit)) => format!("{} {unit}", self.value),
            (None, None) => self.value.clone(),
        };
        format!("{} {value}", self.label)
    }

    /// Lay the pieces out `width` wide, as a plain stat or a tile.
    fn lay(&self, ui: &Ui, width: f32, tile: bool) -> Laid {
        let t = tokens(ui.ctx());
        let lay = |rich: egui::RichText, size: f32, wrap: TextWrapMode, width: f32| {
            WidgetText::from(rich).into_galley(ui, Some(wrap), width.max(1.0), egui::FontId::proportional(size))
        };
        let lead = self.dot.map_or(0.0, |_| self.dot_size(tile) + 8.0);
        let label = if tile {
            lay(
                TextRole::Label.rich(&t, &self.label),
                t.text("label").size,
                TextWrapMode::Truncate,
                width - lead,
            )
        } else {
            lay(
                TextRole::Help.rich(&t, &self.label),
                t.text("help").size,
                TextWrapMode::Truncate,
                width - lead,
            )
        };
        // a tile's number at the statistic's role, the hero's when large; plain, `text-page
        // font-semibold`: the page's size at 600, with none of the page title's tracking. Never
        // cut short: as the web's, a number wider than its room runs into the padding.
        let value = if self.large || tile {
            let role = if self.large {
                TextRole::StatisticHero
            } else {
                TextRole::Statistic
            };
            lay(
                role.rich(&t, &self.value),
                role.font(&t).size,
                TextWrapMode::Extend,
                width,
            )
        } else {
            let page = t.text("page");
            lay(
                egui::RichText::new(&self.value)
                    .size(page.size)
                    .line_height(Some(page.line_height))
                    .variation("wght", 600.0),
                page.size,
                TextWrapMode::Extend,
                width,
            )
        };
        let sm = t.text("sm");
        let unit = self.unit.as_ref().map(|unit| {
            lay(
                egui::RichText::new(unit)
                    .size(sm.size)
                    .line_height(Some(sm.line_height))
                    .variation("wght", sm.weight),
                sm.size,
                TextWrapMode::Extend,
                f32::INFINITY,
            )
        });
        let change = self.change.as_ref().map(|(change, _)| {
            lay(
                TextRole::Help.rich(&t, change).variation("wght", 500.0),
                t.text("help").size,
                TextWrapMode::Extend,
                f32::INFINITY,
            )
        });
        let note = self.note.as_ref().map(|note| {
            lay(
                TextRole::Help.rich(&t, note),
                t.text("help").size,
                TextWrapMode::Wrap,
                width,
            )
        });
        Laid {
            label,
            value,
            unit,
            change,
            note,
        }
    }

    /// The dot's size: 6 beside a plain stat's small label, 10 in a tile.
    fn dot_size(&self, tile: bool) -> f32 {
        if tile { 10.0 } else { 6.0 }
    }

    /// How wide the number's line runs: the value, and the unit and the change 8 after it.
    fn line_width(laid: &Laid) -> f32 {
        let mut x = laid.value.size().x;
        if let Some(unit) = &laid.unit {
            x += 8.0 + unit.size().x;
        }
        if let Some(change) = &laid.change {
            x += 8.0 + 14.0 + 2.0 + change.size().x;
        }
        x
    }

    /// How tall the stat stands, laid out: the label's line, 4 (8 in a tile), the number's, 4 and
    /// the note, and 8 and the trend.
    fn stack_height(&self, laid: &Laid, tile: bool) -> f32 {
        laid.label.size().y
            + if tile { 8.0 } else { 4.0 }
            + laid.value.size().y
            + laid.note.as_ref().map_or(0.0, |n| 4.0 + n.size().y)
            + self.trend.as_ref().map_or(0.0, |_| 8.0 + SPARK.y)
    }

    /// The stat as a card of its own, `width` wide (`Stat` `variant="tile"`): the label in the
    /// label's type after its dot, the number at the statistic's size, 4 further down, the change
    /// after it, the note in the help's type, and the trend under it. A control only with
    /// [`Stat::href`] or [`Stat::selectable`].
    pub fn tile(self, ui: &mut Ui, width: f32) -> Response {
        let t = tokens(ui.ctx());
        let inner = width - 32.0;
        let laid = self.lay(ui, inner, true);
        let height = 16.0 + self.stack_height(&laid, true) + 16.0;
        let interactive = self.interactive();
        let sense = if interactive { Sense::click() } else { Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(vec2(width, height.max(self.min_height)), sense);
        self.answer(ui, &response);
        let radius = t.radius("surface");
        let hot = interactive && response.hovered();
        let painter = ui.painter();
        painter.rect_filled(rect, radius, if hot { t.color("hover") } else { t.color("card") });
        let edge = match (self.emphasis, self.dot) {
            (true, Some(dot)) => dot.gamma_multiply(0.55),
            _ if hot => t.color_at("foreground", 0.25),
            _ => t.color("border-strong"),
        };
        painter.rect_stroke(rect, radius, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
        self.paint(ui, &laid, pos2(rect.left() + 16.0, rect.top() + 16.0), true);
        if interactive {
            super::focus_ring(ui.painter(), &t, rect, radius, &response);
        }
        response
    }

    /// Say what the stat is, and follow its address on a press.
    fn answer(&self, ui: &Ui, response: &Response) {
        let interactive = self.interactive();
        let name = self.name();
        let role = match (&self.href, interactive) {
            (Some(_), _) => egui::Role::Link,
            (None, true) => egui::Role::Button,
            (None, false) => egui::Role::Label,
        };
        response.widget_info(|| egui::WidgetInfo::labeled(role, true, &name));
        if interactive && response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked()
            && let Some(href) = &self.href
        {
            ui.ctx().open_url(egui::OpenUrl::new_tab(href));
        }
    }

    /// Draw the laid-out stat with its top-left at `at`.
    fn paint(&self, ui: &Ui, laid: &Laid, at: egui::Pos2, tile: bool) {
        let t = tokens(ui.ctx());
        let painter = ui.painter();
        let mut y = at.y;
        let mut x = at.x;
        if let Some(dot) = self.dot {
            let size = self.dot_size(tile);
            painter.circle_filled(pos2(x + size / 2.0, y + laid.label.size().y / 2.0), size / 2.0, dot);
            x += size + 8.0;
        }
        let (label_ink, note_ink) = if tile {
            (t.color("foreground"), t.color("muted-foreground"))
        } else {
            (t.color("muted-foreground"), t.color("faint"))
        };
        painter.galley(pos2(x, y), laid.label.clone(), label_ink);
        y += laid.label.size().y + if tile { 8.0 } else { 4.0 };
        if let Some((label, tone)) = &self.chip {
            // the chip on the number's line, sitting where the figures would
            let center = y + laid.value.size().y - 2.0 - 10.0;
            super::chip::paint_at(ui, at.x, center, label, *tone);
        } else {
            let value_ink = if tile {
                t.color("card-foreground")
            } else {
                t.color("foreground")
            };
            let line = baseline(&laid.value);
            painter.galley(pos2(at.x, y), laid.value.clone(), value_ink);
            // the unit and the change 8 after the number, on its baseline
            let mut x = at.x + laid.value.size().x;
            if let Some(unit) = &laid.unit {
                x += 8.0;
                painter.galley(
                    pos2(x, y + line - baseline(unit)),
                    unit.clone(),
                    t.color("muted-foreground"),
                );
                x += unit.size().x;
            }
            if let (Some((change, good)), Some(words)) = (&self.change, &laid.change) {
                x += 8.0;
                let ink = match good {
                    None => t.color("muted-foreground"),
                    Some(true) => t.color("good"),
                    Some(false) => t.color("critical"),
                };
                let top = y + line - baseline(words);
                // the arrow, 14, then the change 2 after it
                icons::paint(
                    painter,
                    pos2(x + 7.0, top + words.size().y / 2.0),
                    if change.starts_with('-') {
                        Icon::ArrowDownRight
                    } else {
                        Icon::ArrowUpRight
                    },
                    14.0,
                    ink,
                );
                painter.galley(pos2(x + 16.0, top), words.clone(), ink);
            }
        }
        y += laid.value.size().y;
        if let Some(note) = &laid.note {
            y += 4.0;
            painter.galley(pos2(at.x, y), note.clone(), note_ink);
            y += note.size().y;
        }
        if let Some((values, ink)) = &self.trend {
            // the column's 4 and the trend's own 4 above it
            y += 8.0;
            sparkline(painter, Rect::from_min_size(pos2(at.x, y), SPARK), values, *ink);
        }
    }
}

impl egui::Widget for Stat {
    /// The plain stat: as wide as its longest line (within the room it is given), on the ground it
    /// stands on.
    fn ui(self, ui: &mut Ui) -> Response {
        let room = ui.available_width();
        let laid = self.lay(ui, room, false);
        let lead = self.dot.map_or(0.0, |_| self.dot_size(false) + 8.0);
        let width = (lead + laid.label.size().x)
            .max(Self::line_width(&laid))
            .max(laid.note.as_ref().map_or(0.0, |n| n.size().x))
            .max(self.trend.as_ref().map_or(0.0, |_| SPARK.x))
            .min(room);
        let height = self.stack_height(&laid, false);
        let sense = if self.interactive() {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), sense);
        self.answer(ui, &response);
        if ui.is_rect_visible(rect) {
            self.paint(ui, &laid, rect.min, false);
            if self.interactive() {
                let t = tokens(ui.ctx());
                super::focus_ring(ui.painter(), &t, rect, t.radius("control"), &response);
            }
        }
        response
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
            .map(|row| {
                row.iter()
                    .map(|s| s.stack_height(&s.lay(ui, cell - 32.0, false), false))
                    .fold(0.0, f32::max)
                    + 32.0
            })
            .collect();
        let (rect, response) = ui.allocate_exact_size(vec2(width, heights.iter().sum()), Sense::hover());
        let radius = t.radius("surface");
        ui.painter().rect_filled(rect, radius, t.color("card"));
        let rule = Stroke::new(1.0, t.color("border"));
        // a number wider than its cell runs on, cut only at the box's edge (`overflow-hidden`)
        let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        inside.set_clip_rect(rect.intersect(ui.clip_rect()));
        let mut top = rect.top();
        for (r, row) in rows.iter().enumerate() {
            let height = heights[r];
            for (c, stat) in row.iter().enumerate() {
                let left = rect.left() + c as f32 * cell;
                let laid = stat.lay(ui, cell - 32.0, false);
                stat.paint(&inside, &laid, pos2(left + 16.0, top + 16.0), false);
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
