//! A calendar day, without a clock or a zone: what a due date is, after sokkel-ui's `YYYY-MM-DD`
//! days and date-fns' calendar arithmetic.

use std::fmt;

/// A day, as the days since 1970-01-01.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day(pub i32);

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTH_NAMES: [&str; 12] = [
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
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

impl Day {
    /// The day of a year, month (1 to 12) and day of the month, after Howard Hinnant's
    /// `days_from_civil`.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        let y = if month <= 2 { year - 1 } else { year };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = month as i32;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i32 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Day(era * 146_097 + doe - 719_468)
    }

    /// Its year, month and day of the month.
    pub fn ymd(self) -> (i32, u32, u32) {
        let z = self.0 + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = yoe + era * 400 + i32::from(month <= 2);
        (year, month, day)
    }

    /// A `YYYY-MM-DD` day.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        let day = parts.next()?.parse().ok()?;
        (1..=12).contains(&month).then(|| Day::from_ymd(year, month, day))
    }

    /// Today, in the zone the clock is kept in (UTC, as the system clock has no zone).
    pub fn today() -> Self {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Day((seconds / 86_400) as i32)
    }

    pub fn plus(self, days: i32) -> Self {
        Day(self.0 + days)
    }

    /// 0 for Monday to 6 for Sunday.
    pub fn weekday(self) -> u32 {
        // 1970-01-01 was a Thursday
        ((self.0 + 3).rem_euclid(7)) as u32
    }

    /// `Nov 15`, as date-fns' `MMM d`.
    pub fn short(self) -> String {
        let (_, month, day) = self.ymd();
        format!("{} {day}", MONTHS[month as usize - 1])
    }

    /// `Monday, November 16`, as `Intl.DateTimeFormat`'s long weekday, long month and day in English.
    pub fn full(self) -> String {
        let (_, month, day) = self.ymd();
        format!(
            "{}, {} {day}",
            WEEKDAYS[self.weekday() as usize],
            MONTH_NAMES[month as usize - 1]
        )
    }

    /// `Nov 3, 2026`, as `Intl.DateTimeFormat`'s medium date in English.
    pub fn medium(self) -> String {
        let (year, _, _) = self.ymd();
        format!("{}, {year}", self.short())
    }

    /// `Mon, Nov 2`, as `Intl.DateTimeFormat`'s short weekday, month and day in English.
    pub fn with_weekday(self) -> String {
        format!("{}, {}", &WEEKDAYS[self.weekday() as usize][..3], self.short())
    }

    /// `Sunday 15 November`, as date-fns' `EEEE d MMMM`.
    pub fn long(self) -> String {
        let (_, month, day) = self.ymd();
        format!(
            "{} {day} {}",
            WEEKDAYS[self.weekday() as usize],
            MONTH_NAMES[month as usize - 1]
        )
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = self.ymd();
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_round_trip_and_name_themselves() {
        let day = Day::parse("2026-11-16").unwrap();
        assert_eq!(day.to_string(), "2026-11-16");
        assert_eq!(day.weekday(), 0, "2026-11-16 is a Monday");
        assert_eq!(day.plus(-1).short(), "Nov 15");
        assert_eq!(day.plus(-1).long(), "Sunday 15 November");
        assert_eq!(Day::parse("2024-03-01").unwrap().plus(-1).to_string(), "2024-02-29");
    }
}
