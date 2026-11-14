//! A number with what it counts, after sokkel-ui's `Stat`, and a row of them in one box, after
//! its dashboard's `KpiRow`.

use std::sync::Arc;

use egui::{Galley, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

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
}

impl Stat {
    pub fn new(label: impl Into<String>, value: impl ToString) -> Self {
        Self {
            label: label.into(),
            value: value.to_string(),
            unit: None,
            change: None,
            note: None,
        }
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

    fn galleys(&self, ui: &Ui, width: f32) -> (Arc<Galley>, Arc<Galley>, Option<Arc<Galley>>, Option<Arc<Galley>>) {
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
