//! Days and moments as the web's `model.ts` counts them: a moment is milliseconds since 1970, a day
//! is a day number since 1970, and the local day is read at the zone the app was given, as the
//! browser reads its own. Dates are written as the web's `Intl` formats write them in English.

use std::sync::atomic::{AtomicI64, Ordering};

/// A day in milliseconds.
pub const DAY: i64 = 86_400_000;

/// An hour in milliseconds.
pub const HOUR: i64 = 3_600_000;

/// How far the local zone is ahead of UTC, in milliseconds: UTC until the app sets it.
static OFFSET: AtomicI64 = AtomicI64::new(0);

/// Sets the local zone, `seconds` ahead of UTC, as the app starts.
pub fn set_local_offset(seconds: i64) {
    OFFSET.store(seconds * 1000, Ordering::Relaxed);
}

fn offset() -> i64 {
    OFFSET.load(Ordering::Relaxed)
}

/// The local day a moment falls on.
pub fn day_of(at: i64) -> i64 {
    (at + offset()).div_euclid(DAY)
}

/// Local midnight of a day.
pub fn start_of(day: i64) -> i64 {
    day * DAY - offset()
}

/// The year, month (1 to 12) and day of a day number.
pub fn civil(day: i64) -> (i64, i64, i64) {
    // Howard Hinnant's civil_from_days
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The day number of a date, as `Date.UTC` reads one: a month past twelve or below one runs into
/// the next or the last year, and a day of zero is the last of the month before.
pub fn day_from(year: i64, month: i64, date: i64) -> i64 {
    let year = year + (month - 1).div_euclid(12);
    let month = (month - 1).rem_euclid(12) + 1;
    // Howard Hinnant's days_from_civil
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468 + (date - 1)
}

/// The day of the week, 0 for Sunday.
pub fn weekday(day: i64) -> i64 {
    (day + 4).rem_euclid(7)
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn month_word(month: i64) -> &'static str {
    MONTHS[(month - 1) as usize]
}

/// `2026-11-08`.
pub fn iso_day(day: i64) -> String {
    let (y, m, d) = civil(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// `2026-11-08` as a day number.
pub fn day_from_iso(iso: &str) -> Option<i64> {
    let mut parts = iso.split('-');
    let y = parts.next()?.parse().ok()?;
    let m = parts.next()?.parse().ok()?;
    let d = parts.next()?.parse().ok()?;
    (parts.next().is_none() && (1..=12).contains(&m) && (1..=31).contains(&d)).then(|| day_from(y, m, d))
}

/// `Nov 8`.
pub fn short_day(day: i64) -> String {
    let (_, m, d) = civil(day);
    format!("{} {d}", month_word(m))
}

/// `Nov 8, 2026`.
pub fn long_day(day: i64) -> String {
    let (y, m, d) = civil(day);
    format!("{} {d}, {y}", month_word(m))
}

/// `Nov 2026`.
pub fn month_name(day: i64) -> String {
    let (y, m, _) = civil(day);
    format!("{} {y}", month_word(m))
}

/// `14:05`, at the local zone.
pub fn clock(at: i64) -> String {
    let minutes = (at + offset()).rem_euclid(DAY) / 60_000;
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// `3 teams`, `1 team`, the count written with its thousands.
pub fn counted(count: usize, word: &str) -> String {
    counted_as(count, word, &format!("{word}s"))
}

pub fn counted_as(count: usize, word: &str, plural: &str) -> String {
    format!("{} {}", thousands(count as i64), if count == 1 { word } else { plural })
}

/// `12,345`.
pub fn thousands(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if value < 0 { format!("-{out}") } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_read_as_the_web_reads_them() {
        assert_eq!(day_from(2026, 1, 1), 20_454);
        assert_eq!(civil(20_775), (2026, 11, 18));
        assert_eq!(day_from(2026, 11 - 12, 1), day_from(2025, 11, 1));
        assert_eq!(day_from(2026, 3, 0), day_from(2026, 2, 28));
        assert_eq!(weekday(20_775), 3);
        assert_eq!(short_day(20_775), "Nov 18");
        assert_eq!(month_name(20_775), "Nov 2026");
        assert_eq!(thousands(1_234_567), "1,234,567");
        assert_eq!(counted(1, "team"), "1 team");
    }
}
