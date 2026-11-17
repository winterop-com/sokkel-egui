//! A month of days to pick one from, after sokkel-ui's `ui/calendar`: the month and its arrows, the
//! weekdays, and six weeks of days, the chosen one filled and today's washed.

use egui::{Id, Rect, Sense, Ui, Widget, WidgetText, pos2, vec2};

use crate::{Day, Icon, TextRole, icons, tokens};

/// The words a calendar says, English by default.
#[derive(Clone, Debug)]
pub struct CalendarWords {
    pub months: [String; 12],
    /// From Monday to Sunday, as short as the head shows them.
    pub weekdays: [String; 7],
    pub previous: String,
    pub next: String,
}

impl Default for CalendarWords {
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
            weekdays: ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].map(String::from),
            previous: "Go to the previous month".into(),
            next: "Go to the next month".into(),
        }
    }
}

/// The day a week starts on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WeekStart {
    #[default]
    Monday,
    Sunday,
}

/// How big a day is, and the room around the month.
const CELL: f32 = 28.0;
const PAD: f32 = 8.0;
/// From one week's top to the next's.
const WEEK: f32 = 36.0;

/// A month's days, 212 wide: the chosen day `chosen`, a press on a day choosing it.
#[must_use = "add it with `ui.add(...)`"]
pub struct Calendar<'a> {
    id: Id,
    chosen: &'a mut Option<Day>,
    today: Day,
    week_start: WeekStart,
    words: CalendarWords,
    focused: Option<Day>,
}

impl<'a> Calendar<'a> {
    pub fn new(id: impl egui::AsIdSalt, chosen: &'a mut Option<Day>, today: Day) -> Self {
        Self {
            id: Id::unique(id),
            chosen,
            today,
            week_start: WeekStart::Monday,
            words: CalendarWords::default(),
            focused: None,
        }
    }

    pub fn week_start(mut self, week_start: WeekStart) -> Self {
        self.week_start = week_start;
        self
    }

    /// The day the keyboard is on: ringed, and its month shown.
    pub fn focused(mut self, day: Option<Day>) -> Self {
        self.focused = day;
        self
    }

    pub fn words(mut self, words: CalendarWords) -> Self {
        self.words = words;
        self
    }

    /// How big the calendar is, its room around it included.
    pub fn size() -> egui::Vec2 {
        vec2(
            7.0 * CELL + 2.0 * PAD,
            PAD + CELL + 12.0 + CELL + 5.0 + 5.0 * WEEK + CELL + PAD,
        )
    }
}

impl Widget for Calendar<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = tokens(ui.ctx());
        let (rect, mut response) = ui.allocate_exact_size(Self::size(), Sense::hover());
        // the month shown: kept while the calendar is open, starting at the chosen day's or today's
        let month_id = self.id.with("month");
        let kept = ui.ctx().data(|d| d.get_temp::<(i32, u32)>(month_id));
        // the keyboard's day brings its month as it moves; the month's arrows leave it where it is
        let last_id = self.id.with("focused");
        let moved = self
            .focused
            .filter(|day| ui.ctx().data(|d| d.get_temp::<Day>(last_id)) != Some(*day));
        if let Some(day) = self.focused {
            ui.ctx().data_mut(|d| d.insert_temp(last_id, day));
        }
        let (year, month) = match (moved, kept) {
            (Some(day), _) => {
                let (y, m, _) = day.ymd();
                (y, m)
            }
            (None, Some(kept)) => kept,
            (None, None) => {
                let (y, m, _) = self.chosen.unwrap_or(self.today).ymd();
                (y, m)
            }
        };
        let painter = ui.painter().clone();
        let left = rect.left() + PAD;
        let column = |c: usize| left + c as f32 * CELL + CELL / 2.0;

        // the head: the arrows at the outer columns, the month and year between them
        let head = rect.top() + PAD + CELL / 2.0;
        let mut shown = (year, month);
        for (c, icon, label, step) in [
            (0, Icon::ChevronLeft, &self.words.previous, -1),
            (6, Icon::ChevronRight, &self.words.next, 1),
        ] {
            let place = Rect::from_center_size(pos2(column(c), head), vec2(CELL, CELL));
            let hit = ui.interact(place, self.id.with(("step", step)), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
            if hit.hovered() {
                painter.rect_filled(place, t.radius("control"), t.color("hover"));
            }
            icons::paint(&painter, place.center(), icon, 16.0, t.color("foreground"));
            if hit.clicked() {
                let index = year * 12 + month as i32 - 1 + step;
                shown = (index.div_euclid(12), index.rem_euclid(12) as u32 + 1);
            }
        }
        let title = format!("{} {year}", self.words.months[month as usize - 1]);
        let galley = WidgetText::from(TextRole::Action.rich(&t, title)).into_galley(
            ui,
            None,
            f32::INFINITY,
            TextRole::Action.font(&t),
        );
        painter.galley(
            pos2(rect.center().x - galley.size().x / 2.0, head - galley.size().y / 2.0),
            galley,
            t.color("popover-foreground"),
        );

        // the weekdays, from the week's first
        let first_weekday = match self.week_start {
            WeekStart::Monday => 0,
            WeekStart::Sunday => 6,
        };
        let names = rect.top() + PAD + CELL + 12.0 + CELL / 2.0;
        for c in 0..7 {
            let name = &self.words.weekdays[(first_weekday + c) % 7];
            let galley = WidgetText::from(TextRole::Help.rich(&t, name)).into_galley(
                ui,
                None,
                f32::INFINITY,
                TextRole::Help.font(&t),
            );
            painter.galley(
                pos2(column(c) - galley.size().x / 2.0, names - galley.size().y / 2.0),
                galley,
                t.color("muted-foreground"),
            );
        }

        // six weeks from the one the month's first day falls in
        let first = Day::from_ymd(year, month, 1);
        let offset = (first.weekday() as usize + 7 - first_weekday) % 7;
        let start = first.plus(-(offset as i32));
        let weeks = rect.top() + PAD + CELL + 12.0 + CELL + 5.0;
        for week in 0..6_i32 {
            for c in 0..7_i32 {
                let day = start.plus(week * 7 + c);
                let (_, day_month, number) = day.ymd();
                let place = Rect::from_min_size(
                    pos2(left + c as f32 * CELL, weeks + week as f32 * WEEK),
                    vec2(CELL, CELL),
                );
                let hit = ui.interact(place, self.id.with(("day", day.0)), Sense::click());
                let chosen = *self.chosen == Some(day);
                hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, chosen, day.to_string()));
                let radius = t.radius("control");
                let ink = if chosen {
                    painter.rect_filled(place, radius, t.color("primary"));
                    t.color("primary-foreground")
                } else {
                    if day == self.today || hit.hovered() {
                        painter.rect_filled(place, radius, t.color("muted"));
                    }
                    if day_month == month {
                        t.color("popover-foreground")
                    } else {
                        t.color("muted-foreground")
                    }
                };
                let galley = WidgetText::from(TextRole::Body.rich(&t, number.to_string())).into_galley(
                    ui,
                    None,
                    f32::INFINITY,
                    TextRole::Body.font(&t),
                );
                painter.galley(place.center() - galley.size() / 2.0, galley, ink);
                if self.focused == Some(day) {
                    painter.rect_stroke(
                        place.expand(2.0),
                        egui::CornerRadius::from(radius.nw.saturating_add(2)),
                        egui::Stroke::new(2.0, t.color("ring")),
                        egui::StrokeKind::Outside,
                    );
                }
                if hit.clicked() && !chosen {
                    *self.chosen = Some(day);
                    response.mark_changed();
                }
            }
        }
        if shown != (year, month) {
            ui.ctx().data_mut(|d| d.insert_temp(month_id, shown));
        } else if !ui.ctx().data(|d| d.get_temp::<(i32, u32)>(month_id).is_some()) {
            ui.ctx().data_mut(|d| d.insert_temp(month_id, (year, month)));
        }
        response
    }
}

/// Forget the month a calendar shows, so it opens next on its chosen day's month.
pub fn calendar_forget(ctx: &egui::Context, id: impl egui::AsIdSalt) {
    ctx.data_mut(|d| d.remove::<(i32, u32)>(Id::unique(id).with("month")));
}

/// Two months side by side to pick a span of days from, as sokkel-ui's `Calendar` with
/// `mode="range" numberOfMonths={2}`: a first press starts the span, a second ends it (one before
/// the start starts it there again), and a press once both are set starts a new one. The span's
/// ends are filled in the primary color, the days between on the muted ground in one band, and
/// today is washed.
#[must_use = "add it with `ui.add(...)`"]
pub struct RangeCalendar<'a> {
    id: Id,
    from: &'a mut Option<Day>,
    to: &'a mut Option<Day>,
    today: Day,
    week_start: WeekStart,
    words: CalendarWords,
}

impl<'a> RangeCalendar<'a> {
    pub fn new(id: impl egui::AsIdSalt, from: &'a mut Option<Day>, to: &'a mut Option<Day>, today: Day) -> Self {
        Self {
            id: Id::unique(id),
            from,
            to,
            today,
            week_start: WeekStart::Monday,
            words: CalendarWords::default(),
        }
    }

    pub fn week_start(mut self, week_start: WeekStart) -> Self {
        self.week_start = week_start;
        self
    }

    pub fn words(mut self, words: CalendarWords) -> Self {
        self.words = words;
        self
    }

    /// How big the two months are, 16 between them, their room around them included.
    pub fn size() -> egui::Vec2 {
        let one = Calendar::size();
        vec2(one.x * 2.0 - PAD * 2.0 + 16.0, one.y)
    }
}

impl Widget for RangeCalendar<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = tokens(ui.ctx());
        let (rect, mut response) = ui.allocate_exact_size(Self::size(), Sense::hover());
        // the first month shown: kept while the calendar is open, starting at the span's or today's
        let month_id = self.id.with("month");
        let (year, month) = ui
            .ctx()
            .data(|d| d.get_temp::<(i32, u32)>(month_id))
            .unwrap_or_else(|| {
                let (y, m, _) = self.from.unwrap_or(self.today).ymd();
                (y, m)
            });
        let painter = ui.painter().clone();
        let head = rect.top() + PAD + CELL / 2.0;
        let mut shown = (year, month);
        // the arrows at the outer ends
        for (x, icon, label, step) in [
            (
                rect.left() + PAD + CELL / 2.0,
                Icon::ChevronLeft,
                &self.words.previous,
                -1,
            ),
            (rect.right() - PAD - CELL / 2.0, Icon::ChevronRight, &self.words.next, 1),
        ] {
            let place = Rect::from_center_size(pos2(x, head), vec2(CELL, CELL));
            let hit = ui.interact(place, self.id.with(("step", step)), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
            if hit.hovered() {
                painter.rect_filled(place, t.radius("control"), t.color("hover"));
            }
            icons::paint(&painter, place.center(), icon, 16.0, t.color("foreground"));
            if hit.clicked() {
                let index = year * 12 + month as i32 - 1 + step;
                shown = (index.div_euclid(12), index.rem_euclid(12) as u32 + 1);
            }
        }
        let first_weekday = match self.week_start {
            WeekStart::Monday => 0,
            WeekStart::Sunday => 6,
        };
        let (from, to) = (*self.from, *self.to);
        let mut pressed = None;
        for shift in 0..2 {
            let index = year * 12 + month as i32 - 1 + shift;
            let (y, m) = (index.div_euclid(12), index.rem_euclid(12) as u32 + 1);
            let left = rect.left() + PAD + shift as f32 * (7.0 * CELL + 16.0);
            let center = left + 3.5 * CELL;
            let title = format!("{} {y}", self.words.months[m as usize - 1]);
            let galley = WidgetText::from(TextRole::Action.rich(&t, title)).into_galley(
                ui,
                None,
                f32::INFINITY,
                TextRole::Action.font(&t),
            );
            painter.galley(
                pos2(center - galley.size().x / 2.0, head - galley.size().y / 2.0),
                galley,
                t.color("popover-foreground"),
            );
            let names = rect.top() + PAD + CELL + 12.0 + CELL / 2.0;
            for c in 0..7 {
                let name = &self.words.weekdays[(first_weekday + c) % 7];
                painter.text(
                    pos2(left + c as f32 * CELL + CELL / 2.0, names),
                    egui::Align2::CENTER_CENTER,
                    name,
                    TextRole::Help.font(&t),
                    t.color("muted-foreground"),
                );
            }
            let first = Day::from_ymd(y, m, 1);
            let offset = (first.weekday() as usize + 7 - first_weekday) % 7;
            let start = first.plus(-(offset as i32));
            let weeks = rect.top() + PAD + CELL + 12.0 + CELL + 5.0;
            let radius = t.radius("control");
            for week in 0..6_i32 {
                for c in 0..7_i32 {
                    let day = start.plus(week * 7 + c);
                    let (_, day_month, number) = day.ymd();
                    // a day of a neighboring month stands empty, as the web's two months draw them
                    if day_month != m {
                        continue;
                    }
                    let place = Rect::from_min_size(
                        pos2(left + c as f32 * CELL, weeks + week as f32 * WEEK),
                        vec2(CELL, CELL),
                    );
                    let hit = ui.interact(place, self.id.with(("day", day.0)), Sense::click());
                    let is_start = from == Some(day);
                    let is_end = to == Some(day);
                    let between = matches!((from, to), (Some(a), Some(b)) if a < day && day < b);
                    let chosen = is_start || is_end;
                    hit.widget_info(|| {
                        egui::WidgetInfo::selected(egui::Role::Button, true, chosen || between, day.to_string())
                    });
                    let ink = if chosen {
                        // the end's band toward the span's middle, under its fill
                        let spans = matches!((from, to), (Some(a), Some(b)) if a != b);
                        if spans {
                            let half = if is_start {
                                Rect::from_min_max(place.center_top(), place.right_bottom())
                            } else {
                                Rect::from_min_max(place.left_top(), place.center_bottom())
                            };
                            painter.rect_filled(half, 0.0, t.color("muted"));
                        }
                        painter.rect_filled(place, radius, t.color("primary"));
                        t.color("primary-foreground")
                    } else if between {
                        painter.rect_filled(place, 0.0, t.color("muted"));
                        t.color("foreground")
                    } else {
                        if day == self.today || hit.hovered() {
                            painter.rect_filled(place, radius, t.color("muted"));
                        }
                        t.color("popover-foreground")
                    };
                    painter.text(
                        place.center(),
                        egui::Align2::CENTER_CENTER,
                        number.to_string(),
                        TextRole::Body.font(&t),
                        ink,
                    );
                    if hit.clicked() {
                        pressed = Some(day);
                    }
                }
            }
        }
        if let Some(day) = pressed {
            match (*self.from, *self.to) {
                (Some(start), None) if day >= start => *self.to = Some(day),
                _ => {
                    *self.from = Some(day);
                    *self.to = None;
                }
            }
            response.mark_changed();
        }
        ui.ctx().data_mut(|d| d.insert_temp(month_id, shown));
        response
    }
}
