//! A sleep timer for a long listen, after sokkel-ui-media's `SleepTimer.tsx`: when playback stops,
//! the menu that sets it (`SleepTimerMenu`), and the pure steps `useSleepTimer` takes, telling
//! playing on from a seek and a chapter's end from a jump. An app keeps the timer's state itself,
//! counting the minutes on its own clock.

use serde::{Serialize, Serializer};

use crate::chapters::chapter_index;

/// When playback stops: after a number of minutes of playing, at the end of the chapter playing,
/// or at the end of what plays. None is never.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SleepTimer {
    Minutes(f64),
    Chapter,
    End,
}

impl Serialize for SleepTimer {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SleepTimer::Minutes(minutes) => serializer.serialize_f64(*minutes),
            SleepTimer::Chapter => serializer.serialize_str("chapter"),
            SleepTimer::End => serializer.serialize_str("end"),
        }
    }
}

/// The minutes the menu offers unless told others.
pub const SLEEP_MINUTES: [f64; 6] = [5.0, 10.0, 15.0, 30.0, 45.0, 60.0];

/// The fastest a listener plays, and the seconds a report may lag, for telling playing on from a seek.
const FASTEST: f64 = 4.0;
const LAG: f64 = 1.0;

/// Whether a position report `seconds` after the last plays on from `from` to `to`, as playback
/// does, rather than seeking: forward, and no further than playing at any speed could go in that time.
pub fn played_on(from: f64, to: f64, seconds: f64) -> bool {
    let step = to - from;
    step >= 0.0 && step <= seconds.max(0.0) * FASTEST + LAG
}

/// A timer to the end of the chapter, after a position report: whether the chapter it waited on
/// has ended, played on from it into a later one, and else the chapter it waits on now.
pub fn chapter_timer_step(
    chapters: &[f64],
    anchor: i64,
    from: Option<f64>,
    to: f64,
    seconds: f64,
    playing: bool,
) -> (bool, i64) {
    let index = chapter_index(chapters, to);
    if index == anchor {
        return (false, anchor);
    }
    let ended = playing && index > anchor && from.is_some_and(|from| played_on(from, to, seconds));
    (ended, index)
}

/// The words the sleep timer says.
#[derive(Clone, Debug)]
pub struct SleepTimerWords {
    /// The button's and the menu's name.
    pub label: String,
    pub off: String,
    /// `{minutes} minutes`.
    pub minutes: String,
    /// The timer to the end of the chapter playing.
    pub chapter: String,
    /// The timer to the end of what plays.
    pub end: String,
    /// `{time} left`: what the button says of a running timer.
    pub left: String,
}

impl Default for SleepTimerWords {
    fn default() -> Self {
        Self {
            label: "Sleep timer".into(),
            off: "Off".into(),
            minutes: "{minutes} minutes".into(),
            chapter: "End of chapter".into(),
            end: "End of episode".into(),
            left: "{time} left".into(),
        }
    }
}

/// What the sleep timer's menu was asked this frame.
#[derive(Clone, Debug)]
pub struct SleepTimerResponse {
    pub response: egui::Response,
    /// A timer was picked from the menu, or none for Off (the web's `onTimerChange`).
    pub asked: Option<Option<SleepTimer>>,
}

/// A whole number of minutes as written, without a fraction.
fn minutes_written(minutes: f64) -> String {
    if minutes.fract() == 0.0 {
        format!("{}", minutes as i64)
    } else {
        format!("{minutes}")
    }
}

/// A sleep timer for a long listen, a podcast or a book, after sokkel-ui-media's `SleepTimerMenu`:
/// a moon in the player's transport, such as [`MediaBar`](crate::MediaBar)'s `extra`, a ghost
/// button as tall as the bar's others, that opens a menu of minutes, the end of the chapter (with
/// `chapters`) and the end of the episode, the timer set checked. The moon is `muted-foreground`
/// without a timer and `primary-ink` with one (`terminal-accent` over a picture), with the time
/// left beside it in the code face while it counts minutes.
///
/// It is controlled: the app keeps the timer and the seconds `remaining`, and pauses as it runs out
/// (`played_on` and `chapter_timer_step` decide a chapter's end).
#[must_use = "show it with `.show(ui)`"]
pub struct SleepTimerMenu<'a> {
    timer: Option<SleepTimer>,
    remaining: Option<f64>,
    minutes: &'a [f64],
    chapters: bool,
    tone: sokkel_egui::MediaTone,
    words: SleepTimerWords,
}

impl<'a> SleepTimerMenu<'a> {
    pub fn new(timer: Option<SleepTimer>) -> Self {
        Self {
            timer,
            remaining: None,
            minutes: &SLEEP_MINUTES,
            chapters: false,
            tone: sokkel_egui::MediaTone::Surface,
            words: SleepTimerWords::default(),
        }
    }

    /// The seconds left of a timer in minutes, written on the button while it runs.
    pub fn remaining(mut self, seconds: Option<f64>) -> Self {
        self.remaining = seconds;
        self
    }

    /// The minutes offered; [`SLEEP_MINUTES`] by default.
    pub fn minutes(mut self, minutes: &'a [f64]) -> Self {
        self.minutes = minutes;
        self
    }

    /// Whether End of chapter is offered, for what has chapters.
    pub fn chapters(mut self, chapters: bool) -> Self {
        self.chapters = chapters;
        self
    }

    pub fn tone(mut self, tone: sokkel_egui::MediaTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn words(mut self, words: SleepTimerWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> SleepTimerResponse {
        use egui::{CornerRadius, Sense, vec2};
        use sokkel_egui::{DropdownMenu, Icon, MediaTone, MenuItem, format_time, icons, mono_font, mono_size, tokens};

        let t = tokens(ui.ctx());
        let said = &self.words;
        let counting = match (self.timer, self.remaining) {
            (Some(SleepTimer::Minutes(_)), Some(left)) => Some(format_time(left)),
            _ => None,
        };
        let state = match (self.timer, &counting) {
            (None, _) => None,
            (Some(_), Some(time)) => Some(said.left.replace("{time}", time)),
            (Some(SleepTimer::Chapter), None) => Some(said.chapter.clone()),
            (Some(SleepTimer::End), None) => Some(said.end.clone()),
            (Some(SleepTimer::Minutes(minutes)), None) => {
                Some(said.minutes.replace("{minutes}", &minutes_written(minutes)))
            }
        };
        let label = match &state {
            None => said.label.clone(),
            Some(state) => format!("{}: {state}", said.label),
        };
        let side = t.density("control-sm");
        let radius = CornerRadius::same(t.radius("control").nw.min(12));
        let mono = mono_font(ui.ctx(), mono_size(&t, t.text("xs").size), 400.0);
        let text = counting
            .clone()
            .map(|time| ui.painter().layout_no_wrap(time, mono, egui::Color32::PLACEHOLDER));
        let width = match &text {
            None => side,
            Some(galley) => (12.0 + 16.0 + 6.0 + galley.size().x).max(side),
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, side), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        let hot = response.hovered();
        if hot {
            let ground = match self.tone {
                MediaTone::Surface => t.color("hover"),
                MediaTone::Overlay => t.color_at("terminal-foreground", 0.15),
            };
            ui.painter().rect_filled(rect, radius, ground);
        }
        let ink = match (self.tone, self.timer.is_some()) {
            (MediaTone::Surface, true) => t.color("primary-ink"),
            (MediaTone::Surface, false) if hot => t.color("foreground"),
            (MediaTone::Surface, false) => t.color("muted-foreground"),
            (MediaTone::Overlay, true) => t.color("terminal-accent"),
            (MediaTone::Overlay, false) => t.color("terminal-foreground"),
        };
        match text {
            None => icons::paint(ui.painter(), rect.center(), Icon::Moon, 16.0, ink),
            Some(galley) => {
                let left = rect.left() + 6.0;
                icons::paint(
                    ui.painter(),
                    egui::pos2(left + 8.0, rect.center().y),
                    Icon::Moon,
                    16.0,
                    ink,
                );
                ui.painter().galley(
                    egui::pos2(left + 22.0, rect.center().y - galley.size().y / 2.0),
                    galley,
                    ink,
                );
            }
        }
        let response = response.on_hover_text(label);

        // the menu: off, the minutes, and the ends
        let mut items = Vec::new();
        let mut timers = Vec::new();
        let mut add = |items: &mut Vec<MenuItem>, word: String, timer: Option<SleepTimer>| {
            let item = MenuItem::action(word);
            items.push(if timer == self.timer {
                item.icon(Icon::Check)
            } else {
                item
            });
            timers.push(Some(timer));
        };
        add(&mut items, said.off.clone(), None);
        items.push(MenuItem::Separator);
        for minutes in self.minutes {
            add(
                &mut items,
                said.minutes.replace("{minutes}", &minutes_written(*minutes)),
                Some(SleepTimer::Minutes(*minutes)),
            );
        }
        items.push(MenuItem::Separator);
        if self.chapters {
            add(&mut items, said.chapter.clone(), Some(SleepTimer::Chapter));
        }
        add(&mut items, said.end.clone(), Some(SleepTimer::End));
        // each item's timer, the rules asking nothing
        let mut by_item = Vec::with_capacity(items.len());
        let mut picked = timers.into_iter();
        for item in &items {
            by_item.push(match item {
                MenuItem::Separator => None,
                _ => picked.next().flatten(),
            });
        }
        let chosen = DropdownMenu::new(&items).end(true).width(96.0).show(&response);
        SleepTimerResponse {
            response,
            asked: chosen.and_then(|index| by_item.get(index).copied().flatten()),
        }
    }
}
