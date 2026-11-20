//! Jobs and their runs along one time axis, after sokkel-ui's `RunTimeline` (sokkel-design's
//! `run-timeline` spec).

use egui::{Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Day, Icon, TextRole, icons, tokens};

const MINUTE: i64 = 60_000;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

/// The steps ticks are placed at, from the finest.
const STEPS: [i64; 12] = [
    15 * MINUTE,
    30 * MINUTE,
    HOUR,
    2 * HOUR,
    3 * HOUR,
    6 * HOUR,
    12 * HOUR,
    DAY,
    2 * DAY,
    3 * DAY,
    7 * DAY,
    14 * DAY,
];
/// How far a tick's label stays from the current time's, so the two never overlap.
const NOW_ROOM: f32 = 40.0;
/// The least room a tick's label takes.
const TICK_ROOM: f32 = 64.0;

/// One run on a lane: where it starts and, when it has one, where it ends, in milliseconds; its
/// state's name and color, or `None` for a run still to come, drawn hollow.
#[derive(Clone, Debug)]
pub struct TimelineRun {
    pub id: String,
    pub at: i64,
    pub end: Option<i64>,
    pub state: Option<(String, Color32)>,
}

/// One lane: a job, its name and a detail line, and its runs.
#[derive(Clone, Debug)]
pub struct TimelineLane {
    pub id: String,
    pub name: String,
    pub detail: Option<String>,
    pub runs: Vec<TimelineRun>,
}

/// The words a run timeline says.
#[derive(Clone, Debug)]
pub struct RunTimelineWords {
    pub now: String,
    pub scheduled: String,
    pub hint: String,
    /// `Upcoming runs of {lane}`.
    pub upcoming: String,
    /// `Shift {amount}`.
    pub by: String,
}

impl Default for RunTimelineWords {
    fn default() -> Self {
        Self {
            now: "Now".into(),
            scheduled: "Scheduled".into(),
            hint: "Drag the selected job's upcoming runs to shift them".into(),
            upcoming: "Upcoming runs of {lane}".into(),
            by: "Shift {amount}".into(),
        }
    }
}

/// What the timeline was asked this frame.
#[derive(Clone, Debug, Default)]
pub struct RunTimelineResponse {
    /// A lane chosen by a press on its name or its track.
    pub selected: Option<String>,
    /// The chosen lane's upcoming runs dropped this far, in milliseconds: the app moves them.
    pub shifted: Option<(String, i64)>,
}

/// A shift written short and signed: `+2h`, `-1d 6h`, `+45m`.
pub fn format_shift(ms: i64) -> String {
    let sign = if ms < 0 { '-' } else { '+' };
    let mut left = ((ms.abs() as f64) / MINUTE as f64).round() as i64;
    let days = left / (24 * 60);
    left -= days * 24 * 60;
    let hours = left / 60;
    let minutes = left - hours * 60;
    let parts: Vec<String> = [(days, 'd'), (hours, 'h'), (minutes, 'm')]
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, unit)| format!("{n}{unit}"))
        .collect();
    format!("{sign}{}", if parts.is_empty() { "0m".into() } else { parts.join(" ") })
}

/// The ticks along an axis from `from` to `to` that fit `width`: the step and each tick's time,
/// on a clock `offset` ahead of UTC.
pub fn timeline_ticks(from: i64, to: i64, width: f32, offset: i64) -> (i64, Vec<i64>) {
    let span = (to - from).max(1);
    let most = ((width / TICK_ROOM).floor() as i64).max(2);
    let step = STEPS
        .iter()
        .copied()
        .find(|candidate| span as f64 / *candidate as f64 <= most as f64)
        .unwrap_or(STEPS[STEPS.len() - 1]);
    let mut ticks = Vec::new();
    let mut at = ((from + offset) as f64 / step as f64).ceil() as i64 * step - offset;
    while at <= to {
        ticks.push(at);
        at += step;
    }
    (step, ticks)
}

/// Jobs and their runs along one time axis, after sokkel-ui's `RunTimeline`: a box on the card, a
/// legend of the states the runs show over a ruler of ticks on the clock `offset` ahead of UTC, and
/// a lane per job, its name (192, 112 below 640) and detail at the start and its runs where they
/// fell, each a 16 tall mark (4 wide) or a bar as long as it took in its state's color, those to
/// come hollow, and the current time's line across every lane. A press on a lane chooses it; with
/// `shift`, the chosen lane's upcoming runs drag sideways as one, snapped.
pub struct RunTimeline<'a> {
    id: egui::Id,
    lanes: &'a [TimelineLane],
    from: i64,
    to: i64,
    now: i64,
    offset: i64,
    selected: Option<&'a str>,
    states: &'a [(String, Color32)],
    shift: Option<i64>,
    words: RunTimelineWords,
}

impl<'a> RunTimeline<'a> {
    /// `states` names the states in the order the legend lists them, each with its color.
    pub fn new(
        id: impl egui::AsIdSalt,
        lanes: &'a [TimelineLane],
        span: (i64, i64),
        now: i64,
        states: &'a [(String, Color32)],
    ) -> Self {
        Self {
            id: egui::Id::unique(id),
            lanes,
            from: span.0,
            to: span.1,
            now,
            offset: 0,
            selected: None,
            states,
            shift: None,
            words: RunTimelineWords::default(),
        }
    }

    /// The clock the ticks read, this far ahead of UTC.
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = offset;
        self
    }

    pub fn selected(mut self, selected: Option<&'a str>) -> Self {
        self.selected = selected;
        self
    }

    /// The chosen lane's upcoming runs drag sideways, snapped to `snap` milliseconds.
    pub fn shift(mut self, snap: i64) -> Self {
        self.shift = Some(snap.max(MINUTE));
        self
    }

    pub fn words(mut self, words: RunTimelineWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> RunTimelineResponse {
        let t = tokens(ui.ctx());
        let mut out = RunTimelineResponse::default();
        let width = ui.available_width();
        let narrow = width < 640.0;
        let name_width = if narrow { 112.0 } else { 192.0 };
        let help = TextRole::Help;
        let tiny = egui::FontId::proportional(t.text("2xs").size);
        let muted = t.color("muted-foreground");
        let border = t.color("border");
        let drag_id = self.id.with("drag");
        let dragged: Option<f32> = ui.data(|d| d.get_temp(drag_id));

        // the legend's entries: the states the runs show, then Scheduled when one is to come
        let shown: Vec<&(String, Color32)> = self
            .states
            .iter()
            .filter(|(name, _)| {
                self.lanes
                    .iter()
                    .flat_map(|l| &l.runs)
                    .any(|r| r.state.as_ref().is_some_and(|(n, _)| n == name))
            })
            .collect();
        let upcoming_any = self.lanes.iter().flat_map(|l| &l.runs).any(|r| r.state.is_none());
        let can_shift = self.shift.is_some()
            && self.selected.is_some_and(|id| {
                self.lanes
                    .iter()
                    .any(|l| l.id == id && l.runs.iter().any(|r| r.state.is_none()))
            });

        let top = ui.cursor().top();
        let left = ui.cursor().left();
        let ground = ui.painter().add(egui::Shape::Noop);
        let mut y = top;

        // the legend
        if !shown.is_empty() || upcoming_any || can_shift {
            let mut x = left + 12.0;
            let line = help.line_height(&t);
            let mut entries: Vec<(String, Option<Color32>)> =
                shown.iter().map(|(n, c)| (n.clone(), Some(*c))).collect();
            if upcoming_any {
                entries.push((self.words.scheduled.clone(), None));
            }
            let mut row_y = y + 8.0;
            for (label, color) in entries {
                let galley = WidgetText::from(help.rich(&t, &label)).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    help.font(&t),
                );
                let entry_width = 8.0 + 6.0 + galley.size().x;
                if x + entry_width > left + width - 12.0 && x > left + 12.0 {
                    x = left + 12.0;
                    row_y += line + 4.0;
                }
                let middle = row_y + line / 2.0;
                match color {
                    Some(color) => {
                        ui.painter().circle_filled(pos2(x + 4.0, middle), 4.0, color);
                    }
                    None => {
                        ui.painter()
                            .circle_stroke(pos2(x + 4.0, middle), 3.5, Stroke::new(1.0, muted));
                    }
                }
                ui.painter().galley(pos2(x + 14.0, row_y), galley, muted);
                x += entry_width + 16.0;
            }
            if can_shift && !narrow {
                let galley = WidgetText::from(help.rich(&t, &self.words.hint)).into_galley(
                    ui,
                    Some(TextWrapMode::Truncate),
                    (left + width - 12.0 - x).max(1.0),
                    help.font(&t),
                );
                ui.painter().galley(
                    pos2(left + width - 12.0 - galley.size().x, row_y),
                    galley,
                    t.color("faint"),
                );
            }
            y = row_y + line + 8.0;
            ui.painter()
                .hline(left..=left + width, y - 0.5, Stroke::new(1.0, border));
        }

        // the ruler
        let track_left = left + name_width;
        let track_width = (width - name_width).max(1.0);
        let x_of = |at: i64| {
            let share = ((at - self.from) as f32 / (self.to - self.from).max(1) as f32).clamp(0.0, 1.0);
            track_left + share * track_width
        };
        let axis = Rect::from_min_size(pos2(left, y), vec2(width, 32.0));
        let (step, ticks) = timeline_ticks(self.from, self.to, track_width, self.offset);
        let now_x = x_of(self.now);
        let now_in = self.now >= self.from && self.now <= self.to;
        for tick in &ticks {
            let x = x_of(*tick);
            let share = (x - track_left) / track_width;
            if !(0.02..=0.98).contains(&share) || (now_in && (x - now_x).abs() < NOW_ROOM) {
                continue;
            }
            let wall = tick + self.offset;
            let label = if wall.rem_euclid(DAY) == 0 || step >= DAY {
                Day(wall.div_euclid(DAY) as i32).short()
            } else {
                let minutes = wall.rem_euclid(DAY) / MINUTE;
                format!("{:02}:{:02}", minutes / 60, minutes % 60)
            };
            ui.painter().text(
                pos2(x, axis.center().y),
                egui::Align2::CENTER_CENTER,
                label,
                tiny.clone(),
                muted,
            );
        }
        if now_in {
            let words = self.words.now.to_uppercase();
            let galley = WidgetText::from(egui::RichText::new(words).font(tiny.clone()).strong()).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                tiny.clone(),
            );
            let tab = Rect::from_center_size(pos2(now_x, axis.center().y), vec2(galley.size().x + 8.0, 16.0));
            ui.painter().rect_filled(tab, t.radius("mini"), t.color("primary"));
            ui.painter().galley(
                pos2(tab.left() + 4.0, tab.center().y - galley.size().y / 2.0),
                galley,
                t.color("primary-foreground"),
            );
        }
        y = axis.bottom();
        ui.painter()
            .hline(left..=left + width, y - 0.5, Stroke::new(1.0, border));

        // the lanes
        let lanes_top = y;
        for (index, lane) in self.lanes.iter().enumerate() {
            let chosen = self.selected == Some(lane.id.as_str());
            let name = WidgetText::from(
                egui::RichText::new(&lane.name)
                    .font(crate::mono_font(
                        ui.ctx(),
                        crate::mono_small_size(&t, TextRole::Help.font(&t).size),
                        400.0,
                    ))
                    .line_height(Some(TextRole::Body.line_height(&t))),
            )
            .into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                name_width - 24.0,
                TextRole::Body.font(&t),
            );
            let detail = lane.detail.as_ref().map(|d| {
                WidgetText::from(help.rich(&t, d)).into_galley(
                    ui,
                    Some(TextWrapMode::Truncate),
                    name_width - 24.0,
                    help.font(&t),
                )
            });
            let words = name.size().y + detail.as_ref().map_or(0.0, |d| d.size().y);
            let height = (words + 16.0).max(48.0);
            let row = Rect::from_min_size(pos2(left, y), vec2(width, height));
            let hit = ui.interact(row, self.id.with(("lane", index)), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, chosen, &lane.name));
            if chosen {
                ui.painter().rect_filled(row, CornerRadius::ZERO, t.color("accent"));
                ui.painter()
                    .vline(row.left() + 1.0, row.y_range(), Stroke::new(2.0, t.color("primary")));
            } else if hit.hovered() {
                let name_area = Rect::from_min_size(row.min, vec2(name_width, height));
                ui.painter()
                    .rect_filled(name_area, CornerRadius::ZERO, t.color("muted"));
                let track = Rect::from_min_max(pos2(track_left, row.top()), row.max);
                ui.painter()
                    .rect_filled(track, CornerRadius::ZERO, t.color_at("muted", 0.5));
            }
            if hit.clicked() {
                out.selected = Some(lane.id.clone());
            }
            let text_top = row.center().y - words / 2.0;
            let name_height = name.size().y;
            ui.painter()
                .galley(pos2(left + 12.0, text_top), name, t.color("foreground"));
            if let Some(detail) = detail {
                ui.painter()
                    .galley(pos2(left + 12.0, text_top + name_height), detail, muted);
            }
            // the grid lines and the runs
            for tick in &ticks {
                let x = x_of(*tick);
                ui.painter()
                    .vline(x.round() + 0.5, row.y_range(), Stroke::new(1.0, border));
            }
            let middle = row.center().y;
            let carried = if chosen && can_shift { dragged } else { None };
            for run in &lane.runs {
                let end = run.end.unwrap_or(run.at);
                if end < self.from || run.at > self.to {
                    continue;
                }
                let mut x0 = x_of(run.at);
                let mut x1 = x_of(end).max(x0 + 4.0);
                if run.end.is_none() {
                    x1 = x0 + 4.0;
                }
                if let (Some(by), None) = (carried, &run.state) {
                    // its place before the drag, dashed
                    let ghost = Rect::from_min_max(pos2(x0, middle - 8.0), pos2(x1, middle + 8.0));
                    ui.painter().rect_stroke(
                        ghost,
                        t.radius("mini"),
                        Stroke::new(1.0, t.color("border-strong")),
                        StrokeKind::Inside,
                    );
                    x0 += by;
                    x1 += by;
                }
                let mark = Rect::from_min_max(pos2(x0, middle - 8.0), pos2(x1, middle + 8.0));
                match &run.state {
                    Some((_, color)) => {
                        ui.painter().rect_filled(mark, t.radius("mini"), *color);
                    }
                    None => {
                        ui.painter().rect_filled(mark, t.radius("mini"), t.color("card"));
                        ui.painter()
                            .rect_stroke(mark, t.radius("mini"), Stroke::new(1.0, muted), StrokeKind::Inside);
                    }
                }
            }
            // the chosen lane's upcoming runs, as one thing to drag
            if chosen && let Some(snap) = self.shift.filter(|_| can_shift) {
                let upcoming: Vec<i64> = lane.runs.iter().filter(|r| r.state.is_none()).map(|r| r.at).collect();
                let first = upcoming.iter().copied().min().unwrap_or(self.now).max(self.now);
                let last = upcoming.iter().copied().max().unwrap_or(self.now);
                let by = carried.unwrap_or(0.0);
                let handle = Rect::from_min_max(
                    pos2(x_of(first).min(x_of(last)) + by, row.top() + 4.0),
                    pos2(x_of(last) + 4.0 + 8.0 + by, row.bottom() - 4.0),
                );
                let grip = ui.interact(handle, self.id.with(("handle", index)), Sense::drag());
                let name = self.words.upcoming.replace("{lane}", &lane.name);
                grip.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Slider, true, &name));
                let wash = if grip.hovered() || grip.dragged() { 0.10 } else { 0.05 };
                ui.painter()
                    .rect_filled(handle, t.radius("mini"), t.color_at("primary", wash));
                let dashes = egui::Shape::dashed_line(
                    &[
                        handle.left_top(),
                        handle.right_top(),
                        handle.right_bottom(),
                        handle.left_bottom(),
                        handle.left_top(),
                    ],
                    Stroke::new(1.0, t.color_at("primary", 0.5)),
                    4.0,
                    3.0,
                );
                ui.painter().extend(dashes);
                icons::paint(
                    ui.painter(),
                    pos2(handle.left() + 2.0 + 7.0, handle.center().y),
                    Icon::GripVertical,
                    14.0,
                    t.color("primary"),
                );
                if grip.hovered() || grip.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                let per_ms = track_width / (self.to - self.from).max(1) as f32;
                let to_ms = |px: f32| ((px / per_ms) as f64 / snap as f64).round() as i64 * snap;
                if grip.dragged() {
                    let mut moved = carried.unwrap_or(0.0) + grip.drag_delta().x;
                    // never before now
                    let least = (x_of(self.now) - x_of(first)).min(0.0) + per_ms * MINUTE as f32;
                    moved = moved.max(least);
                    ui.data_mut(|d| d.insert_temp(drag_id, moved));
                    if let Some(pointer) = grip.interact_pointer_pos() {
                        let card = self.words.by.replace("{amount}", &format_shift(to_ms(moved)));
                        let mono = crate::mono_font(ui.ctx(), crate::mono_small_size(&t, help.font(&t).size), 400.0);
                        let galley = WidgetText::from(egui::RichText::new(card).font(mono.clone())).into_galley(
                            ui,
                            Some(TextWrapMode::Extend),
                            f32::INFINITY,
                            mono,
                        );
                        let place = Rect::from_min_size(pointer + vec2(12.0, 12.0), galley.size() + vec2(16.0, 8.0));
                        let painter = ui
                            .ctx()
                            .layer_painter(egui::LayerId::new(egui::Order::Tooltip, self.id.with("card")));
                        painter.rect(
                            place,
                            t.radius("md"),
                            t.color("popover"),
                            Stroke::new(1.0, border),
                            StrokeKind::Inside,
                        );
                        painter.galley(place.min + vec2(8.0, 4.0), galley, t.color("popover-foreground"));
                    }
                }
                if grip.drag_stopped() {
                    let by = to_ms(carried.unwrap_or(0.0));
                    ui.data_mut(|d| d.remove::<f32>(drag_id));
                    if by != 0 {
                        out.shifted = Some((lane.id.clone(), by));
                    }
                }
            }
            if index + 1 < self.lanes.len() {
                ui.painter()
                    .hline(row.x_range(), row.bottom() - 0.5, Stroke::new(1.0, border));
            }
            y = row.bottom();
        }
        // the names' column's rule, and the current time's line over the lanes
        ui.painter()
            .vline(track_left - 0.5, axis.top()..=y, Stroke::new(1.0, border));
        if now_in {
            ui.painter()
                .vline(now_x, lanes_top..=y, Stroke::new(1.0, t.color("primary")));
        }
        let rect = Rect::from_min_max(pos2(left, top), pos2(left + width, y));
        ui.painter().set(
            ground,
            egui::Shape::rect_filled(rect, t.radius("surface"), t.color("card")),
        );
        ui.painter()
            .rect_stroke(rect, t.radius("surface"), Stroke::new(1.0, border), StrokeKind::Inside);
        ui.allocate_rect(rect, Sense::hover());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_read_as_the_web_writes_them() {
        assert_eq!(format_shift(2 * HOUR), "+2h");
        assert_eq!(format_shift(-(DAY + 6 * HOUR)), "-1d 6h");
        assert_eq!(format_shift(0), "+0m");
        assert_eq!(format_shift(45 * MINUTE), "+45m");
    }
}
