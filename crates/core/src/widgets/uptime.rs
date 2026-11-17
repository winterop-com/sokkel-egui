//! A thing's record over its last days, after sokkel-ui's `UptimeBar`.

use egui::{Align2, Color32, Id, Key, Rect, Sense, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Day, TextRole, Tokens, tokens};

use super::focus_ring;

/// One day of a thing's record.
#[derive(Clone, Debug, PartialEq)]
pub struct UptimeDay {
    pub date: Day,
    /// The share of the day it was up, from 0 to 1, or `None` when nothing was measured.
    pub uptime: Option<f64>,
    /// The color token the day is drawn in, its state's (`warning`, `critical`, `info`); by its
    /// uptime when `None`.
    pub status: Option<&'static str>,
    /// What happened that day, said in the readout while the day is pointed at.
    pub note: Option<String>,
}

impl UptimeDay {
    pub fn new(date: Day, uptime: Option<f64>) -> Self {
        Self {
            date,
            uptime,
            status: None,
            note: None,
        }
    }
}

/// An uptime written to two decimals, cut rather than rounded: `0.99951` is `99.95%`.
pub fn format_uptime(uptime: f64) -> String {
    format!("{:.2}%", (uptime * 10_000.0).floor() / 100.0)
}

/// The share of the measured days' time that was up, or `None` when none was measured.
pub fn overall_uptime(days: &[UptimeDay]) -> Option<f64> {
    let measured: Vec<f64> = days.iter().filter_map(|d| d.uptime).collect();
    (!measured.is_empty()).then(|| measured.iter().sum::<f64>() / measured.len() as f64)
}

/// The color a day is drawn in by its uptime alone: all of it good, a little lost a warning, more
/// critical, nothing measured the muted ink at 22% over the card.
pub fn uptime_color(t: &Tokens, uptime: Option<f64>) -> Color32 {
    match uptime {
        None => mix(t.color("muted-foreground"), t.color("card"), 0.22),
        Some(u) if u >= 0.999 => t.color("good"),
        Some(u) if u >= 0.99 => t.color("warning"),
        Some(_) => t.color("critical"),
    }
}

/// `share` of `a` over `b`, as `color-mix` mixes two opaque colors.
fn mix(a: Color32, b: Color32, share: f32) -> Color32 {
    let channel = |x: u8, y: u8| (x as f32 * share + y as f32 * (1.0 - share)).round() as u8;
    Color32::from_rgb(channel(a.r(), b.r()), channel(a.g(), b.g()), channel(a.b(), b.b()))
}

/// The words an uptime bar says.
#[derive(Clone, Debug)]
pub struct UptimeBarWords {
    /// The uptime over the days shown, `{}` the uptime written.
    pub uptime: String,
    /// Under the first day shown, `{}` how many days before today it is.
    pub since: String,
    pub today: String,
    pub no_data: String,
}

impl Default for UptimeBarWords {
    fn default() -> Self {
        Self {
            uptime: "{} uptime".into(),
            since: "{} days ago".into(),
            today: "Today".into(),
            no_data: "No data".into(),
        }
    }
}

/// A thing's record over its last days as a row of bars, one a day, oldest at the left and today
/// at the right, each in its state's color or by its uptime. Under the bars the first day shown
/// says how long ago it was, the last says today, and between them the uptime over the days shown;
/// pointing at a day, or moving to it with the arrows, Home and End, puts its date, uptime and note
/// there instead. A box too narrow for every day at `min_bar_width` shows the latest that fit.
pub struct UptimeBar<'a> {
    id: Id,
    days: &'a [UptimeDay],
    min_bar_width: f32,
    words: UptimeBarWords,
}

impl<'a> UptimeBar<'a> {
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, days: &'a [UptimeDay]) -> Self {
        Self {
            id: Id::unique(("sokkel-uptime", id_salt)),
            days,
            min_bar_width: 3.0,
            words: UptimeBarWords::default(),
        }
    }

    /// The narrowest a day is drawn; 3 when not given.
    pub fn min_bar_width(mut self, width: f32) -> Self {
        self.min_bar_width = width;
        self
    }

    pub fn words(mut self, words: UptimeBarWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the bar; returns the day pressed, or chosen with Enter.
    pub fn show(self, ui: &mut Ui) -> Option<&'a UptimeDay> {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        const GAP: f32 = 1.0;
        let fit = (((width + GAP) / (self.min_bar_width + GAP)).floor() as usize).max(1);
        let shown = &self.days[self.days.len().saturating_sub(fit)..];
        let overall = overall_uptime(shown);
        let (strip, response) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let count = shown.len().max(1);
        let bar = (width - GAP * (count - 1) as f32) / count as f32;
        let at = |pos: egui::Pos2| (((pos.x - strip.left()) / (bar + GAP)) as usize).min(count - 1);
        // the day pointed at, or moved to with the keys while the strip has focus
        let key_id = self.id.with("active");
        let mut active = if response.has_focus() {
            ui.data(|d| d.get_temp::<usize>(key_id))
        } else {
            None
        };
        if response.has_focus() {
            let last = shown.len().saturating_sub(1);
            let from = active.unwrap_or(last);
            ui.input(|i| {
                if i.key_pressed(Key::ArrowLeft) {
                    active = Some(from.saturating_sub(1));
                } else if i.key_pressed(Key::ArrowRight) {
                    active = Some((from + 1).min(last));
                } else if i.key_pressed(Key::Home) {
                    active = Some(0);
                } else if i.key_pressed(Key::End) {
                    active = Some(last);
                }
            });
            if let Some(a) = active {
                ui.data_mut(|d| d.insert_temp(key_id, a));
            }
        } else {
            ui.data_mut(|d| d.remove_temp::<usize>(key_id));
        }
        if let Some(pos) = response.hover_pos() {
            active = Some(at(pos));
        }
        let mut chosen = None;
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            chosen = shown.get(at(pos));
        }
        if response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            chosen = active.and_then(|a| shown.get(a));
        }
        let radius = t.radius("mini");
        let painter = ui.painter();
        for (i, day) in shown.iter().enumerate() {
            let left = strip.left() + i as f32 * (bar + GAP);
            let rect = Rect::from_min_size(pos2(left, strip.top()), vec2(bar, strip.height()));
            let color = day.status.map_or_else(|| uptime_color(&t, day.uptime), |s| t.color(s));
            let color = if active == Some(i) {
                color.gamma_multiply(0.6)
            } else {
                color
            };
            painter.rect_filled(rect, radius, color);
        }
        focus_ring(painter, &t, strip, radius, &response);
        // the readout 6 under the bars, whatever spacing holds the bar's parts
        let spacing = ui.spacing().item_spacing.y;
        ui.add_space(6.0 - spacing);
        let ink = t.color("muted-foreground");
        let font = TextRole::Help.font(&t);
        let since = self
            .words
            .since
            .replace("{}", &shown.len().saturating_sub(1).to_string());
        let (line, _) = ui.allocate_exact_size(vec2(width, t.text("help").line_height), Sense::hover());
        let painter = ui.painter();
        let left = painter.text(line.left_center(), Align2::LEFT_CENTER, since, font.clone(), ink);
        let right = painter.text(
            line.right_center(),
            Align2::RIGHT_CENTER,
            &self.words.today,
            font.clone(),
            ink,
        );
        let middle = match active.and_then(|a| shown.get(a)) {
            Some(day) => {
                let uptime = day.uptime.map_or(self.words.no_data.clone(), format_uptime);
                let mut job = egui::text::LayoutJob::default();
                let rest = match &day.note {
                    Some(note) => format!(" · {uptime} · {note}"),
                    None => format!(" · {uptime}"),
                };
                for (text, color) in [(day.date.medium(), t.color("foreground")), (rest, ink)] {
                    job.append(&text, 0.0, egui::TextFormat::simple(font.clone(), color));
                }
                WidgetText::from(job)
            }
            None => {
                let words = overall.map_or(self.words.no_data.clone(), |o| {
                    self.words.uptime.replace("{}", &format_uptime(o))
                });
                let mut job = egui::text::LayoutJob::default();
                job.append(&words, 0.0, egui::TextFormat::simple(font.clone(), ink));
                WidgetText::from(job)
            }
        };
        let room = (right.left() - left.right() - 24.0).max(1.0);
        let galley = middle.into_galley(ui, Some(TextWrapMode::Truncate), room, font);
        let x = (line.center().x - galley.size().x / 2.0)
            .max(left.right() + 12.0)
            .min(right.left() - 12.0 - galley.size().x);
        ui.painter()
            .galley(pos2(x, line.center().y - galley.size().y / 2.0), galley, ink);
        chosen
    }
}
