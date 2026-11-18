//! A small month for a calendar's sidebar, after sokkel-ui's `MiniMonth`.

use egui::{Align2, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::{Day, Icon, TextRole, icons, tokens};

/// The words a small month says, English by default.
#[derive(Clone, Debug)]
pub struct MiniMonthWords {
    pub months: [String; 12],
    /// From Monday to Sunday, one letter each, over the columns.
    pub weekdays: [String; 7],
    pub previous: String,
    pub next: String,
}

impl Default for MiniMonthWords {
    fn default() -> Self {
        let months = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        Self {
            months: months.map(String::from),
            weekdays: ["M", "T", "W", "T", "F", "S", "S"].map(String::from),
            previous: "Previous month".into(),
            next: "Next month".into(),
        }
    }
}

/// What a small month was asked this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct MiniMonthResponse {
    /// A day pressed.
    pub chosen: Option<Day>,
}

/// A small month, as sokkel-ui's `MiniMonth`: the month's title at medium weight with back and
/// forward, then its days in whole weeks from Monday across the width it is given, each a 28
/// square 30 apart. Today wears the primary ground, the chosen day a ring in the primary color, a
/// day of another month the faint ink; a day in `marked` carries a 4 dot under its number. The
/// month shown is the caller's, any day in it.
pub struct MiniMonth<'a> {
    id: egui::Id,
    month: &'a mut Day,
    value: Option<Day>,
    today: Option<Day>,
    marked: &'a [Day],
    words: MiniMonthWords,
}

impl<'a> MiniMonth<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, month: &'a mut Day) -> Self {
        Self {
            id: egui::Id::unique(("sokkel-mini-month", id)),
            month,
            value: None,
            today: None,
            marked: &[],
            words: MiniMonthWords::default(),
        }
    }

    /// The chosen day, ringed.
    pub fn value(mut self, value: Option<Day>) -> Self {
        self.value = value;
        self
    }

    pub fn today(mut self, today: Day) -> Self {
        self.today = Some(today);
        self
    }

    /// Days that carry a dot.
    pub fn marked(mut self, marked: &'a [Day]) -> Self {
        self.marked = marked;
        self
    }

    pub fn words(mut self, words: MiniMonthWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> MiniMonthResponse {
        let t = tokens(ui.ctx());
        let mut out = MiniMonthResponse::default();
        let (year, month, _) = self.month.ymd();
        let first = Day::from_ymd(year, month, 1);
        let next_first = if month == 12 {
            Day::from_ymd(year + 1, 1, 1)
        } else {
            Day::from_ymd(year, month + 1, 1)
        };
        let start = first.plus(-(first.weekday() as i32));
        let last = next_first.plus(-1);
        let end = last.plus(6 - last.weekday() as i32);
        let weeks = ((end.0 - start.0 + 1) / 7) as usize;
        let width = ui.available_width();
        let column = width / 7.0;
        let height = 28.0 + 4.0 + 28.0 + weeks as f32 * 30.0;
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let painter = ui.painter_at(rect);
        let muted = t.color("muted-foreground");

        // the title, 6 in, with back and forward at the end
        let title_galley = egui::WidgetText::from(
            TextRole::Body
                .rich(&t, format!("{} {year}", self.words.months[(month - 1) as usize]))
                .variation("wght", 500.0),
        )
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            width - 6.0 - 60.0,
            TextRole::Body.font(&t),
        );
        painter.galley(
            pos2(rect.left() + 6.0, rect.top() + 14.0 - title_galley.size().y / 2.0),
            title_galley,
            t.color("foreground"),
        );
        for (index, (icon, word, step)) in [
            (Icon::ChevronLeft, &self.words.previous, -1),
            (Icon::ChevronRight, &self.words.next, 1),
        ]
        .into_iter()
        .enumerate()
        {
            let button = Rect::from_min_size(
                pos2(
                    rect.right() - 28.0 * (2 - index) as f32 - 4.0 * (1 - index) as f32,
                    rect.top(),
                ),
                vec2(28.0, 28.0),
            );
            let hit = ui.interact(button, self.id.with(("step", index)), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word.as_str()));
            let ink = if hit.hovered() {
                painter.rect_filled(button, t.radius("md"), t.color("hover"));
                t.color("foreground")
            } else {
                muted
            };
            icons::paint(&painter, button.center(), icon, 16.0, ink);
            if hit.clicked() {
                let (y, m) = if step < 0 {
                    if month == 1 { (year - 1, 12) } else { (year, month - 1) }
                } else if month == 12 {
                    (year + 1, 1)
                } else {
                    (year, month + 1)
                };
                *self.month = Day::from_ymd(y, m, 1);
            }
        }

        // the weekdays' letters, small and faint
        let head = rect.top() + 28.0 + 4.0;
        let small = TextRole::Help.font(&t);
        for (index, letter) in self.words.weekdays.iter().enumerate() {
            painter.text(
                pos2(rect.left() + column * (index as f32 + 0.5), head + 14.0),
                Align2::CENTER_CENTER,
                letter,
                small.clone(),
                t.color("faint"),
            );
        }

        // the days, a 28 square each in its column, 1 over and under it
        let grid = head + 28.0;
        for offset in 0..(weeks * 7) {
            let day = start.plus(offset as i32);
            let row = offset / 7;
            let col = offset % 7;
            let center = pos2(
                rect.left() + column * (col as f32 + 0.5),
                grid + row as f32 * 30.0 + 15.0,
            );
            let square = Rect::from_center_size(center, vec2(28.0, 28.0));
            let hit = ui.interact(square, self.id.with(("day", day.0)), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, day.full()));
            let today = self.today == Some(day);
            let chosen = self.value == Some(day);
            let inside = day.ymd().1 == month;
            let radius = t.radius("md");
            if today {
                painter.rect_filled(square, radius, t.color("primary"));
            } else if hit.hovered() {
                painter.rect_filled(square, radius, t.color_at("foreground", 0.1));
            }
            if chosen && !today {
                painter.rect_stroke(
                    square,
                    radius,
                    Stroke::new(1.0, t.color("primary")),
                    egui::StrokeKind::Outside,
                );
            }
            let ink = if today {
                t.color("primary-foreground")
            } else if inside {
                t.color("foreground")
            } else {
                t.color("faint")
            };
            let number = egui::WidgetText::from(
                TextRole::Help
                    .rich(&t, day.ymd().2.to_string())
                    .variation("wght", if today { 500.0 } else { 400.0 }),
            )
            .into_galley(ui, None, f32::INFINITY, small.clone());
            painter.galley(center - number.size() / 2.0, number, ink);
            if self.marked.contains(&day) {
                let dot = if today { t.color("primary-foreground") } else { muted };
                painter.circle_filled(pos2(center.x, square.bottom() - 2.0 - 2.0), 2.0, dot);
            }
            if hit.clicked() {
                out.chosen = Some(day);
            }
        }
        out
    }
}
