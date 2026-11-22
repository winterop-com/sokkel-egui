//! Days side by side with the hours running down, after sokkel-ui's `WeekGrid`.

use egui::{Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Day, Moment, TextRole, tokens};

/// One thing on a calendar, as sokkel-ui's `CalendarEvent`: a job's run, a window. It runs from
/// `start` to `end`.
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start: Moment,
    pub end: Moment,
    /// The calendar it belongs to, as a kind color: blue, green, jade, pink, teal or violet.
    pub color: &'static str,
    /// A short line under the title.
    pub detail: Option<String>,
    /// Drawn faded, as one cancelled or skipped.
    pub muted: bool,
}

/// A timed event placed in a day's column: its minutes in the day, its column and how many its
/// cluster splits into.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedEvent {
    pub index: usize,
    pub from: u32,
    pub to: u32,
    pub column: usize,
    pub columns: usize,
}

/// The events that touch `day`, laid out to split the width where they overlap, as sokkel-ui's
/// `layoutDay`: overlapping events (directly or through others) form a cluster, each takes the
/// first column free at its start, and the cluster's events share its column count; an event
/// shorter than `min_minutes` is laid out as if it lasted that long, as it is drawn.
pub fn layout_day(events: &[CalendarEvent], day: Day, min_minutes: u32) -> Vec<PlacedEvent> {
    let at = |m: Moment| i64::from(m.day.0 - day.0) * 1440 + i64::from(m.minutes);
    let mut touching: Vec<(usize, u32, u32, u32)> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| at(e.start) < 1440 && at(e.end) > 0)
        .map(|(i, e)| {
            let from = at(e.start).max(0) as u32;
            let to = at(e.end).min(1440) as u32;
            (i, from, to, (to.max(from + min_minutes)).min(1440))
        })
        .collect();
    touching.sort_by(|a, b| a.1.cmp(&b.1).then(b.2.cmp(&a.2)));
    let mut placed = Vec::new();
    let mut cluster: Vec<usize> = Vec::new();
    let mut ends: Vec<u32> = Vec::new();
    let mut cluster_end = 0u32;
    let mut columns_of = vec![0usize; touching.len()];
    let mut column_at = vec![0usize; touching.len()];
    for (k, (_, from, _, shown)) in touching.iter().enumerate() {
        if *from >= cluster_end {
            for &c in &cluster {
                columns_of[c] = ends.len();
            }
            cluster.clear();
            ends.clear();
        }
        let column = match ends.iter().position(|end| end <= from) {
            Some(free) => {
                ends[free] = *shown;
                free
            }
            None => {
                ends.push(*shown);
                ends.len() - 1
            }
        };
        column_at[k] = column;
        cluster.push(k);
        cluster_end = cluster_end.max(*shown);
    }
    for &c in &cluster {
        columns_of[c] = ends.len();
    }
    for (k, (index, from, to, _)) in touching.iter().enumerate() {
        placed.push(PlacedEvent {
            index: *index,
            from: *from,
            to: *to,
            column: column_at[k],
            columns: columns_of[k],
        });
    }
    placed
}

/// What the week grid was asked this frame.
pub struct WeekGridResponse {
    /// The event pressed, by its index in the events given.
    pub pressed: Option<usize>,
}

/// Days side by side with the hours running down, as sokkel-ui's `WeekGrid`: a gutter of hours
/// (56 wide), each day's head (its weekday muted, its date in a 28 circle, today's in the
/// primary), and its column; each timed event a block from its start to its end in its calendar's
/// kind color (the color at 15% over the card, a 3 edge at its start, its ink), its title, its
/// times from 40 tall and its detail from 60, events that overlap splitting the day's width.
/// Today's column is washed in the primary at 3%, with a line in critical at the current time and
/// the time beside it in the gutter. The hours scroll under the heads, starting at `scroll_to_hour`.
pub struct WeekGrid<'a> {
    id: Id,
    days: &'a [Day],
    events: &'a [CalendarEvent],
    now: Option<Moment>,
    hour_height: f32,
    scroll_to_hour: u32,
    opened: Option<usize>,
}

impl<'a> WeekGrid<'a> {
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, days: &'a [Day], events: &'a [CalendarEvent]) -> Self {
        Self {
            id: Id::unique(("sokkel-week", id_salt)),
            days,
            events,
            now: None,
            hour_height: 48.0,
            scroll_to_hour: 7,
            opened: None,
        }
    }

    /// The current time, for the line across today.
    pub fn now(mut self, now: Moment) -> Self {
        self.now = Some(now);
        self
    }

    pub fn hour_height(mut self, height: f32) -> Self {
        self.hour_height = height;
        self
    }

    /// The hour at the top of the view when it first draws.
    pub fn scroll_to_hour(mut self, hour: u32) -> Self {
        self.scroll_to_hour = hour.min(23);
        self
    }

    /// The event shown as open, ringed, by its index in the events given.
    pub fn opened(mut self, opened: Option<usize>) -> Self {
        self.opened = opened;
        self
    }

    pub fn show(self, ui: &mut Ui) -> WeekGridResponse {
        let t = tokens(ui.ctx());
        let gutter = 56.0;
        let width = ui.available_width();
        let count = self.days.len().max(1) as f32;
        let column = (width - gutter) / count;
        let rule = Stroke::new(1.0, t.color("border"));
        let font = TextRole::Help.font(&t);
        let today = self.now.map(|n| n.day);
        let mut pressed = None;
        // the heads: the weekday, and the date in its circle
        let (heads, _) = ui.allocate_exact_size(vec2(width, 44.0), Sense::hover());
        ui.painter().hline(heads.x_range(), heads.bottom() - 0.5, rule);
        for (i, day) in self.days.iter().enumerate() {
            let cell = Rect::from_min_size(
                pos2(heads.left() + gutter + column * i as f32, heads.top()),
                vec2(column, 44.0),
            );
            ui.painter().vline(cell.left() + 0.5, cell.y_range(), rule);
            let is_today = Some(*day) == today;
            let weekday = &day.with_weekday()[..3];
            let (_, _, date) = day.ymd();
            let words = WidgetText::from(TextRole::Body.rich(&t, weekday)).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextRole::Body.font(&t),
            );
            let total = words.size().x + 6.0 + 28.0;
            let x = cell.center().x - total / 2.0;
            let ink = if is_today {
                t.color("primary-ink")
            } else {
                t.color("muted-foreground")
            };
            ui.painter()
                .galley(pos2(x, cell.center().y - words.size().y / 2.0), words, ink);
            let circle = pos2(x + total - 14.0, cell.center().y);
            if is_today {
                ui.painter().circle_filled(circle, 14.0, t.color("primary"));
            }
            ui.painter().text(
                circle,
                Align2::CENTER_CENTER,
                date.to_string(),
                TextRole::Label.font(&t),
                if is_today {
                    t.color("primary-foreground")
                } else {
                    t.color("foreground")
                },
            );
        }
        // the hours, scrolling under the heads
        let hour = self.hour_height;
        let scroll = egui::ScrollArea::vertical()
            .id_salt(self.id.with("hours"))
            .auto_shrink([false, false])
            .vertical_scroll_offset(if ui.data(|d| d.get_temp::<bool>(self.id.with("scrolled")).is_none()) {
                ui.data_mut(|d| d.insert_temp(self.id.with("scrolled"), true));
                // half a label above the hour, so its time is not cut at the top
                (hour * self.scroll_to_hour as f32 - 8.0).max(0.0)
            } else {
                ui.data(|d| d.get_temp::<f32>(self.id.with("offset"))).unwrap_or(0.0)
            });
        let output = scroll.show(ui, |ui| {
            let (body, _) = ui.allocate_exact_size(vec2(width, 24.0 * hour), Sense::hover());
            let painter = ui.painter();
            for h in 0..24 {
                let y = body.top() + h as f32 * hour;
                painter.hline((body.left() + gutter)..=body.right(), y + 0.5, rule);
                if h > 0 {
                    painter.text(
                        pos2(body.left() + gutter - 8.0, y),
                        Align2::RIGHT_CENTER,
                        format!("{h:02}:00"),
                        font.clone(),
                        t.color("muted-foreground"),
                    );
                }
            }
            for (i, day) in self.days.iter().enumerate() {
                let left = body.left() + gutter + column * i as f32;
                let col = Rect::from_min_size(pos2(left, body.top()), vec2(column, body.height()));
                if Some(*day) == today {
                    ui.painter().rect_filled(col, 0.0, t.color_at("primary", 0.03));
                }
                ui.painter().vline(left + 0.5, col.y_range(), rule);
                for placed in layout_day(self.events, *day, 30) {
                    let event = &self.events[placed.index];
                    let lane = (column - 4.0) / placed.columns as f32;
                    let top = body.top() + placed.from as f32 / 60.0 * hour + 1.0;
                    let height = (((placed.to - placed.from) as f32 / 60.0) * hour).max(hour / 2.0) - 2.0;
                    let block = Rect::from_min_size(
                        pos2(left + 2.0 + lane * placed.column as f32, top),
                        vec2(lane - 2.0, height),
                    );
                    let id = self.id.with(("event", placed.index, i));
                    let hit = ui.interact(block, id, Sense::click());
                    hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &event.title));
                    if hit.clicked() {
                        pressed = Some(placed.index);
                    }
                    paint_block(ui, &t, block, event, hit.hovered(), self.opened == Some(placed.index));
                }
                // the line at the current time across today
                if let Some(now) = self.now
                    && now.day == *day
                {
                    let y = body.top() + now.minutes as f32 / 60.0 * hour;
                    ui.painter()
                        .hline(left..=left + column, y, Stroke::new(1.5, t.color("critical")));
                    ui.painter().circle_filled(pos2(left, y), 4.0, t.color("critical"));
                    let words = format!("{:02}:{:02}", now.minutes / 60, now.minutes % 60);
                    let galley = ui
                        .painter()
                        .layout_no_wrap(words, font.clone(), t.color("critical-ink"));
                    let tag = Rect::from_min_size(
                        pos2(
                            body.left() + gutter - 4.0 - galley.size().x - 8.0,
                            y - galley.size().y / 2.0 - 1.0,
                        ),
                        galley.size() + vec2(8.0, 2.0),
                    );
                    ui.painter().rect_filled(tag, t.radius("sm"), t.color("background"));
                    ui.painter()
                        .galley(tag.min + vec2(4.0, 1.0), galley, t.color("critical-ink"));
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(self.id.with("offset"), output.state.clamped_offset().y));
        WeekGridResponse { pressed }
    }
}

/// An event's block: its kind color at 15% over the card, a 3 edge at its start, its title, its
/// times from 40 tall and its detail from 60, faded when muted, ringed when open.
fn paint_block(ui: &Ui, t: &crate::Tokens, block: Rect, event: &CalendarEvent, hovered: bool, open: bool) {
    let kind = format!("kind-{}", event.color);
    let ink = t.color(&format!("{kind}-ink"));
    let fade = |c: Color32| if event.muted { c.gamma_multiply(0.6) } else { c };
    let radius = t.radius("md");
    let painter = ui.painter_at(block.expand(3.0));
    painter.rect_filled(block, radius, fade(t.color("card")));
    painter.rect_filled(block, radius, fade(t.color_at(&kind, 0.15)));
    painter.rect_filled(
        Rect::from_min_size(block.min, vec2(3.0, block.height())),
        CornerRadius {
            nw: radius.nw,
            sw: radius.sw,
            ne: 0,
            se: 0,
        },
        fade(t.color(&kind)),
    );
    if open {
        painter.rect_stroke(
            block,
            radius,
            Stroke::new(2.0, t.color("ring")),
            egui::StrokeKind::Outside,
        );
    } else if hovered {
        painter.rect_stroke(
            block,
            radius,
            Stroke::new(1.0, t.color_at(&kind, 0.5)),
            egui::StrokeKind::Inside,
        );
    }
    let roomy = block.height() >= 40.0;
    let x = block.left() + 3.0 + 6.0;
    let room = (block.width() - 3.0 - 12.0).max(1.0);
    let line = |text: &str, wght: f32| {
        WidgetText::from(TextRole::Help.rich(t, text).variation("wght", wght)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            room,
            TextRole::Help.font(t),
        )
    };
    let title = line(&event.title, t.weight(crate::Weight::Medium));
    let mut y = if roomy {
        block.top() + 4.0
    } else {
        block.center().y - title.size().y / 2.0
    };
    let step = 16.0;
    painter.galley(pos2(x, y), title, fade(ink));
    if roomy {
        y += step;
        let times = format!(
            "{:02}:{:02} - {:02}:{:02}",
            event.start.minutes / 60,
            event.start.minutes % 60,
            event.end.minutes / 60,
            event.end.minutes % 60
        );
        painter.galley(pos2(x, y), line(&times, 400.0), fade(ink.gamma_multiply(0.8)));
        if block.height() >= 60.0
            && let Some(detail) = &event.detail
        {
            y += step;
            painter.galley(pos2(x, y), line(detail, 400.0), fade(ink.gamma_multiply(0.8)));
        }
    }
}
