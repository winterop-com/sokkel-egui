//! When a job's schedule fires, on the UTC clock, as the web's `schedule.ts`.

use crate::model::{Cadence, DAY, HOUR, MINUTE, Schedule, js_round, start_of_day, weekday_of};

/// A cadence a schedule picks, with its label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CadenceOption {
    pub value: Cadence,
    pub label: &'static str,
}

pub const CADENCES: [CadenceOption; 5] = [
    CadenceOption {
        value: Cadence::Quarter,
        label: "Every 15 minutes",
    },
    CadenceOption {
        value: Cadence::Hourly,
        label: "Hourly",
    },
    CadenceOption {
        value: Cadence::Daily,
        label: "Daily",
    },
    CadenceOption {
        value: Cadence::Weekdays,
        label: "Weekdays",
    },
    CadenceOption {
        value: Cadence::Weekly,
        label: "Weekly",
    },
];

/// How many fires `fires_between` gives at most, as the web's default.
pub const FIRES: usize = 2000;

/// The hour and minute of an `HH:mm` time; a part that is missing or not a number reads as 0.
fn parts(at: &str) -> (i64, i64) {
    let mut split = at.split(':');
    let mut next = || split.next().and_then(|part| part.trim().parse().ok()).unwrap_or(0);
    let hour = next();
    let minute = next();
    (hour, minute)
}

fn weekday(at: i64) -> i64 {
    (at.div_euclid(DAY) + 4).rem_euclid(7)
}

/// Whether the schedule fires on the UTC day starting at `day`.
fn fires_on(schedule: &Schedule, start: i64, day: i64) -> bool {
    match schedule.cadence {
        Cadence::Weekdays => (1..=5).contains(&weekday(day)),
        Cadence::Weekly => weekday(day) == weekday(start),
        _ => true,
    }
}

/// The first time the schedule fires after `after`, never before its first day; none when it
/// never fires, as for a first day the web cannot read.
pub fn next_fire(schedule: &Schedule, after: i64) -> Option<i64> {
    let start = start_of_day(&schedule.starts)?;
    let from = after.max(start - 1);
    let (hour, minute) = parts(&schedule.at);
    if matches!(schedule.cadence, Cadence::Quarter | Cadence::Hourly) {
        let quarter = schedule.cadence == Cadence::Quarter;
        let step = if quarter { 15 * MINUTE } else { HOUR };
        let offset = (if quarter { minute % 15 } else { minute }) * MINUTE;
        return Some((from - offset).div_euclid(step) * step + offset + step);
    }
    let mut day = from.div_euclid(DAY) * DAY;
    for _ in 0..15 {
        let at = day + hour * HOUR + minute * MINUTE;
        if at > from && fires_on(schedule, start, day) {
            return Some(at);
        }
        day += DAY;
    }
    None
}

/// Every time the schedule fires after `from`, up to and with `to`, `limit` at most (`FIRES` is
/// the web's default).
pub fn fires_between(schedule: &Schedule, from: i64, to: i64, limit: usize) -> Vec<i64> {
    let mut fires = Vec::new();
    let mut next = next_fire(schedule, from);
    while let Some(at) = next {
        if at > to || fires.len() >= limit {
            break;
        }
        fires.push(at);
        next = next_fire(schedule, at);
    }
    fires
}

/// A schedule as a reader says it: `Daily at 02:00 UTC`, `Every 15 minutes`, or Manual only.
pub fn say_schedule(schedule: Option<&Schedule>) -> String {
    let Some(schedule) = schedule else {
        return "Manual only".to_owned();
    };
    let (_, minute) = parts(&schedule.at);
    match schedule.cadence {
        Cadence::Quarter if minute % 15 == 0 => "Every 15 minutes".to_owned(),
        Cadence::Quarter => format!("Every 15 minutes from :{:02}", minute % 15),
        Cadence::Hourly => format!("Hourly at :{minute:02}"),
        Cadence::Daily => format!("Daily at {} UTC", schedule.at),
        Cadence::Weekdays => format!("Weekdays at {} UTC", schedule.at),
        Cadence::Weekly => format!("{}s at {} UTC", weekday_of(&schedule.starts), schedule.at),
    }
}

/// The schedule moved `by` milliseconds, as dragging its upcoming runs along the timeline asks: its
/// time of day for a daily one, its minute for one that fires every hour or quarter, and its first
/// day too for a weekly one carried past midnight.
pub fn shift_schedule(schedule: &Schedule, by: f64) -> Schedule {
    let (hour, minute) = parts(&schedule.at);
    let moved = hour * 60 + minute + js_round(by / MINUTE as f64) as i64;
    if matches!(schedule.cadence, Cadence::Quarter | Cadence::Hourly) {
        let within = if schedule.cadence == Cadence::Quarter { 15 } else { 60 };
        return Schedule {
            at: format!("{hour:02}:{:02}", moved.rem_euclid(within)),
            ..schedule.clone()
        };
    }
    let minutes = moved.rem_euclid(1440);
    let at = format!("{:02}:{:02}", minutes / 60, minutes % 60);
    if schedule.cadence != Cadence::Weekly {
        return Schedule { at, ..schedule.clone() };
    }
    let days = moved.div_euclid(1440);
    let starts = start_of_day(&schedule.starts)
        .map(|start| crate::model::day_of(start + days * DAY))
        .unwrap_or_else(|| schedule.starts.clone());
    Schedule {
        cadence: schedule.cadence,
        at,
        starts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::JOBS;

    const NOW: i64 = 1_794_992_400_000;

    fn job(id: &str) -> Schedule {
        JOBS.iter()
            .find(|job| job.id == id)
            .and_then(|job| job.schedule.clone())
            .unwrap()
    }

    #[test]
    fn says_each_schedule_as_the_web() {
        let said: Vec<String> = JOBS.iter().map(|job| say_schedule(job.schedule.as_ref())).collect();
        assert_eq!(
            said,
            [
                "Daily at 02:00 UTC",
                "Hourly at :15",
                "Sundays at 03:30 UTC",
                "Weekdays at 07:00 UTC",
                "Every 15 minutes"
            ]
        );
        assert_eq!(say_schedule(None), "Manual only");
        let odd = Schedule::new(Cadence::Quarter, "00:07", "2026-09-01");
        assert_eq!(say_schedule(Some(&odd)), "Every 15 minutes from :07");
    }

    #[test]
    fn fires_as_the_web() {
        let next = [
            ("nightly-export", 1_795_053_600_000),
            ("sync-inventory", 1_794_993_300_000),
            ("rebuild-index", 1_795_318_200_000),
            ("send-digest", 1_795_071_600_000),
            ("import-orders", 1_794_993_300_000),
        ];
        for (id, at) in next {
            assert_eq!(next_fire(&job(id), NOW), Some(at), "{id}");
        }
        let day = NOW - DAY;
        assert_eq!(fires_between(&job("nightly-export"), day, NOW, 5), [1_794_967_200_000]);
        assert_eq!(
            fires_between(&job("sync-inventory"), day, NOW, 5),
            [
                1_794_906_900_000,
                1_794_910_500_000,
                1_794_914_100_000,
                1_794_917_700_000,
                1_794_921_300_000
            ]
        );
        assert_eq!(fires_between(&job("rebuild-index"), day, NOW, 5), Vec::<i64>::new());
        assert_eq!(fires_between(&job("send-digest"), day, NOW, 5), [1_794_985_200_000]);
        assert_eq!(
            fires_between(&job("import-orders"), day, NOW, 5),
            [
                1_794_906_900_000,
                1_794_907_800_000,
                1_794_908_700_000,
                1_794_909_600_000,
                1_794_910_500_000
            ]
        );
        let week = NOW - 7 * DAY;
        let counts = [
            ("nightly-export", 7),
            ("sync-inventory", 168),
            ("rebuild-index", 1),
            ("send-digest", 5),
            ("import-orders", 672),
        ];
        for (id, count) in counts {
            assert_eq!(fires_between(&job(id), week, NOW, FIRES).len(), count, "{id}");
        }
        let later = Schedule::new(Cadence::Weekly, "03:30", "2026-12-05");
        assert_eq!(next_fire(&later, NOW), Some(1_796_441_400_000));
    }

    #[test]
    fn shifts_as_the_web() {
        let cases = [
            (
                Cadence::Daily,
                "02:00",
                "2026-09-01",
                -3.0 * 3_600_000.0,
                "23:00",
                "2026-09-01",
            ),
            (
                Cadence::Weekly,
                "23:30",
                "2026-09-06",
                45.0 * 60_000.0,
                "00:15",
                "2026-09-07",
            ),
            (
                Cadence::Weekly,
                "00:30",
                "2026-09-06",
                -45.0 * 60_000.0,
                "23:45",
                "2026-09-05",
            ),
            (
                Cadence::Quarter,
                "00:05",
                "2026-09-01",
                13.0 * 60_000.0,
                "00:03",
                "2026-09-01",
            ),
            (
                Cadence::Hourly,
                "00:15",
                "2026-09-01",
                -20.0 * 60_000.0 - 30_000.0,
                "00:55",
                "2026-09-01",
            ),
            (Cadence::Hourly, "00:15", "2026-09-01", 90_000.0, "00:17", "2026-09-01"),
        ];
        for (cadence, at, starts, by, moved, day) in cases {
            let shifted = shift_schedule(&Schedule::new(cadence, at, starts), by);
            assert_eq!(shifted, Schedule::new(cadence, moved, day), "{at} by {by}");
        }
    }
}
