//! The Stats page's numbers, as the web's `pages/Stats.tsx` counts them from the time listened:
//! this week and in all, the episodes finished, the time speed saved, the shows heard most and the
//! last seven days.

use crate::backend::{DAY, Listen, day, weekday};
use crate::catalog::show_by_id;
use crate::parts::spoken;

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// One of the last seven days: its key, its label and the seconds listened on it.
#[derive(Clone, Debug, PartialEq)]
pub struct DayListened {
    pub key: String,
    pub label: String,
    pub seconds: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stats {
    /// Seconds listened in all, and in the last seven days.
    pub total: i64,
    pub this_week: i64,
    pub finished: i64,
    /// Seconds playing faster than 1x saved.
    pub saved: i64,
    /// The six shows heard most, with their seconds, most first.
    pub top: Vec<(String, i64)>,
    /// The last seven days, today last.
    pub days: Vec<DayListened>,
}

/// One bar of a bar list: its id, label, share of the longest and whether it is marked.
#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub id: String,
    pub label: String,
    pub value: f64,
    pub chosen: bool,
    /// Its time, as the bar list writes it.
    pub written: String,
}

impl Stats {
    pub fn of(listens: &[Listen], now: i64) -> Stats {
        let week: Vec<String> = (0..7).map(|offset| day(-offset, now)).collect();
        let total = listens.iter().map(|row| row.seconds).sum();
        let this_week = listens
            .iter()
            .filter(|row| week.contains(&row.day))
            .map(|row| row.seconds)
            .sum();
        let finished = listens.iter().map(|row| row.finished).sum();
        let saved = listens.iter().map(|row| (row.content - row.seconds).max(0)).sum();
        let mut by_show: Vec<(String, i64)> = Vec::new();
        for row in listens {
            match by_show.iter_mut().find(|(show, _)| *show == row.show) {
                Some((_, seconds)) => *seconds += row.seconds,
                None => by_show.push((row.show.clone(), row.seconds)),
            }
        }
        by_show.sort_by_key(|(_, seconds)| std::cmp::Reverse(*seconds));
        by_show.truncate(6);
        let days = (0..7)
            .map(|index| {
                let offset = index - 6;
                let key = day(offset, now);
                let seconds = listens.iter().filter(|row| row.day == key).map(|row| row.seconds).sum();
                let label = if offset == 0 {
                    "Today".to_owned()
                } else {
                    WEEKDAYS[weekday(now + offset * DAY) as usize].to_owned()
                };
                DayListened { key, label, seconds }
            })
            .collect();
        Stats {
            total,
            this_week,
            finished,
            saved,
            top: by_show,
            days,
        }
    }

    /// The four tiles: label, value as written (Finished as its number) and note.
    pub fn tiles(&self) -> [(&'static str, String, &'static str); 4] {
        [
            ("Listened", spoken(self.total as f64), "in all"),
            ("This week", spoken(self.this_week as f64), "the last seven days"),
            (
                "Finished",
                self.finished.to_string(),
                if self.finished == 1 { "episode" } else { "episodes" },
            ),
            ("Saved by speed", spoken(self.saved as f64), "playing faster than 1x"),
        ]
    }

    /// Time listened by show, as the page's bar list has it.
    pub fn show_bars(&self) -> Vec<Bar> {
        let most = self.top.first().map_or(1.0, |(_, seconds)| *seconds as f64);
        self.top
            .iter()
            .enumerate()
            .map(|(index, (show, seconds))| {
                let value = *seconds as f64 / most;
                Bar {
                    id: show.clone(),
                    label: show_by_id(show).map_or(show.clone(), |found| found.title.clone()),
                    value,
                    chosen: index == 0,
                    written: spoken(value * most),
                }
            })
            .collect()
    }

    /// Time listened by day, as the page's bar list has it.
    pub fn day_bars(&self, now: i64) -> Vec<Bar> {
        let busiest = self.days.iter().map(|one| one.seconds).max().unwrap_or(0).max(1) as f64;
        let today = day(0, now);
        self.days
            .iter()
            .map(|one| {
                let value = one.seconds as f64 / busiest;
                Bar {
                    id: one.key.clone(),
                    label: one.label.clone(),
                    value,
                    chosen: one.key == today,
                    written: spoken(value * busiest),
                }
            })
            .collect()
    }
}
