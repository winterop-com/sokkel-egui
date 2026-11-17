//! Reporting periods by their ids, after sokkel-ui's `lib/periods`: the recent periods of a type,
//! newest first, and any id read back into the period it names.
//!
//! ```text
//! daily       20261108     weekly      2026W45     monthly     202611
//! bimonthly   202611B      quarterly   2026Q4      sixmonthly  2026S2
//! yearly      2026
//! ```
//!
//! A week is an ISO week, starting on a Monday, numbered by the year its Thursday falls in. A
//! two-month period is named by its first month, which is odd. Days are [`Day`]s, never a moment,
//! so a period never moves across a time zone.

use crate::Day;

/// The kinds of period a report may be made for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PeriodType {
    Daily,
    Weekly,
    Monthly,
    Bimonthly,
    Quarterly,
    Sixmonthly,
    Yearly,
}

pub const PERIOD_TYPES: [PeriodType; 7] = [
    PeriodType::Daily,
    PeriodType::Weekly,
    PeriodType::Monthly,
    PeriodType::Bimonthly,
    PeriodType::Quarterly,
    PeriodType::Sixmonthly,
    PeriodType::Yearly,
];

impl PeriodType {
    /// What the type is called: "Daily", "Two-monthly".
    pub fn label(self) -> &'static str {
        match self {
            PeriodType::Daily => "Daily",
            PeriodType::Weekly => "Weekly",
            PeriodType::Monthly => "Monthly",
            PeriodType::Bimonthly => "Two-monthly",
            PeriodType::Quarterly => "Quarterly",
            PeriodType::Sixmonthly => "Six-monthly",
            PeriodType::Yearly => "Yearly",
        }
    }

    /// How many recent periods of the type are listed.
    pub fn recent(self) -> usize {
        match self {
            PeriodType::Daily | PeriodType::Weekly | PeriodType::Monthly => 13,
            PeriodType::Bimonthly | PeriodType::Quarterly | PeriodType::Sixmonthly => 9,
            PeriodType::Yearly => 6,
        }
    }

    /// An id of the type's shape.
    pub fn example(self) -> &'static str {
        match self {
            PeriodType::Daily => "20261108",
            PeriodType::Weekly => "2026W45",
            PeriodType::Monthly => "202611",
            PeriodType::Bimonthly => "202611B",
            PeriodType::Quarterly => "2026Q4",
            PeriodType::Sixmonthly => "2026S2",
            PeriodType::Yearly => "2026",
        }
    }
}

/// One period: its id, what it is called, and its first and last days.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Period {
    pub id: String,
    pub kind: PeriodType,
    pub label: String,
    pub start: Day,
    pub end: Day,
}

const MONTHS: [&str; 12] = [
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

fn month_name(month: u32) -> &'static str {
    MONTHS[(month - 1) as usize]
}

fn last_date(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        Day::from_ymd(year + 1, 1, 1)
    } else {
        Day::from_ymd(year, month + 1, 1)
    };
    next.plus(-1).ymd().2
}

/// The Monday that starts ISO week 1 of `year`: the week that holds the year's first Thursday.
fn week_one(year: i32) -> Day {
    let fourth = Day::from_ymd(year, 1, 4);
    fourth.plus(-(fourth.weekday() as i32))
}

fn weeks_in(year: i32) -> i32 {
    (week_one(year + 1).0 - week_one(year).0) / 7
}

/// The ISO year and week a day falls in.
fn week_of(at: Day) -> (i32, i32) {
    let thursday = at.plus(3 - at.weekday() as i32);
    let year = thursday.ymd().0;
    (year, (thursday.0 - week_one(year).0).div_euclid(7) + 1)
}

fn daily(year: i32, month: u32, date: u32) -> Period {
    let at = Day::from_ymd(year, month, date);
    Period {
        id: format!("{year:04}{month:02}{date:02}"),
        kind: PeriodType::Daily,
        label: format!("{} {date}, {year}", month_name(month)),
        start: at,
        end: at,
    }
}

fn weekly(year: i32, week: i32) -> Period {
    let start = week_one(year).plus((week - 1) * 7);
    let (_, month, date) = start.ymd();
    Period {
        id: format!("{year}W{week}"),
        kind: PeriodType::Weekly,
        label: format!("Week {week}, {year} (from Mon, {} {date})", &month_name(month)[..3]),
        start,
        end: start.plus(6),
    }
}

fn monthly(year: i32, month: u32) -> Period {
    Period {
        id: format!("{year}{month:02}"),
        kind: PeriodType::Monthly,
        label: format!("{} {year}", month_name(month)),
        start: Day::from_ymd(year, month, 1),
        end: Day::from_ymd(year, month, last_date(year, month)),
    }
}

/// A run of months from `first`: two, three or six of them.
fn span(kind: PeriodType, year: i32, first: u32, months: u32) -> Period {
    let last = first + months - 1;
    let id = match kind {
        PeriodType::Bimonthly => format!("{year}{first:02}B"),
        PeriodType::Quarterly => format!("{year}Q{}", (first - 1) / 3 + 1),
        _ => format!("{year}S{}", (first - 1) / 6 + 1),
    };
    Period {
        id,
        kind,
        label: format!("{} to {} {year}", month_name(first), month_name(last)),
        start: Day::from_ymd(year, first, 1),
        end: Day::from_ymd(year, last, last_date(year, last)),
    }
}

fn yearly(year: i32) -> Period {
    Period {
        id: year.to_string(),
        kind: PeriodType::Yearly,
        label: year.to_string(),
        start: Day::from_ymd(year, 1, 1),
        end: Day::from_ymd(year, 12, 31),
    }
}

/// The period of a type that holds a day.
pub fn period_of(kind: PeriodType, at: Day) -> Period {
    let (year, month, date) = at.ymd();
    match kind {
        PeriodType::Daily => daily(year, month, date),
        PeriodType::Weekly => {
            let (year, week) = week_of(at);
            weekly(year, week)
        }
        PeriodType::Monthly => monthly(year, month),
        PeriodType::Bimonthly => span(kind, year, month - (month - 1) % 2, 2),
        PeriodType::Quarterly => span(kind, year, month - (month - 1) % 3, 3),
        PeriodType::Sixmonthly => span(kind, year, if month <= 6 { 1 } else { 7 }, 6),
        PeriodType::Yearly => yearly(year),
    }
}

/// The recent periods of a type, newest first: the one holding `today` and `count` in all.
pub fn recent_periods(kind: PeriodType, today: Day, count: usize) -> Vec<Period> {
    let mut periods: Vec<Period> = Vec::with_capacity(count);
    if count == 0 {
        return periods;
    }
    periods.push(period_of(kind, today));
    while periods.len() < count {
        let start = periods[periods.len() - 1].start;
        periods.push(period_of(kind, start.plus(-1)));
    }
    periods
}

fn digits(text: &str, count: usize) -> Option<u32> {
    (text.len() == count && text.bytes().all(|b| b.is_ascii_digit())).then(|| text.parse().ok())?
}

/// The period an id names, read by its shape, or `None` when it names none: a month past 12, a
/// day past the month's end, a week past the year's last, a two-month period on an even month.
/// With `kind`, an id of any other type is `None` too.
pub fn parse_period(id: &str, kind: Option<PeriodType>) -> Option<Period> {
    let text = id.trim();
    let read = || -> Option<Period> {
        if let Some(rest) = text.strip_suffix('B') {
            let (year, month) = (digits(rest.get(..4)?, 4)?, digits(rest.get(4..)?, 2)?);
            return ((1..=11).contains(&month) && month % 2 == 1)
                .then(|| span(PeriodType::Bimonthly, year as i32, month, 2));
        }
        if let Some((year, week)) = text.split_once('W') {
            let (year, number) = (digits(year, 4)? as i32, week.parse::<i32>().ok()?);
            let fine = !week.is_empty() && week.len() <= 2 && week.bytes().all(|b| b.is_ascii_digit());
            return (fine && !week.starts_with('0') && number >= 1 && number <= weeks_in(year))
                .then(|| weekly(year, number));
        }
        if let Some((year, quarter)) = text.split_once('Q') {
            let (year, quarter) = (digits(year, 4)? as i32, digits(quarter, 1)?);
            return (1..=4)
                .contains(&quarter)
                .then(|| span(PeriodType::Quarterly, year, (quarter - 1) * 3 + 1, 3));
        }
        if let Some((year, half)) = text.split_once('S') {
            let (year, half) = (digits(year, 4)? as i32, digits(half, 1)?);
            return (1..=2)
                .contains(&half)
                .then(|| span(PeriodType::Sixmonthly, year, if half == 1 { 1 } else { 7 }, 6));
        }
        match text.len() {
            8 => {
                let (year, month, date) = (
                    digits(&text[..4], 4)? as i32,
                    digits(&text[4..6], 2)?,
                    digits(&text[6..], 2)?,
                );
                ((1..=12).contains(&month) && date >= 1 && date <= last_date(year, month))
                    .then(|| daily(year, month, date))
            }
            6 => {
                let (year, month) = (digits(&text[..4], 4)? as i32, digits(&text[4..], 2)?);
                (1..=12).contains(&month).then(|| monthly(year, month))
            }
            4 => Some(yearly(digits(text, 4)? as i32)),
            _ => None,
        }
    };
    read().filter(|period| kind.is_none_or(|kind| period.kind == kind))
}

/// What is wrong with a typed id for a type, in a sentence, or `None` when it names a period of
/// that type.
pub fn period_issue(id: &str, kind: PeriodType) -> Option<String> {
    let text = id.trim();
    if text.is_empty() {
        return Some("Type a period id.".into());
    }
    if parse_period(text, Some(kind)).is_some() {
        return None;
    }
    let wanted = kind.label().to_lowercase();
    Some(match parse_period(text, None) {
        Some(other) => format!(
            "{text} is a {} period; this one is {wanted}, such as {}.",
            other.kind.label().to_lowercase(),
            kind.example()
        ),
        None => format!(
            "{text} is not a period id. A {wanted} period is written like {}.",
            kind.example()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periods_read_as_sokkel_ui_reads_them() {
        let today = Day::from_ymd(2026, 11, 18);
        let months = recent_periods(PeriodType::Monthly, today, 3);
        assert_eq!(
            months.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["202611", "202610", "202609"]
        );
        assert_eq!(months[0].label, "November 2026");
        let week = period_of(PeriodType::Weekly, today);
        assert_eq!(week.id, "2026W47");
        assert_eq!(week.label, "Week 47, 2026 (from Mon, Nov 16)");
        assert_eq!(period_of(PeriodType::Quarterly, today).id, "2026Q4");
        assert_eq!(period_of(PeriodType::Bimonthly, today).id, "202611B");
        assert_eq!(parse_period("202612B", None), None);
        assert_eq!(parse_period("2025W53", None), None);
        assert_eq!(parse_period("2026W53", None).map(|p| p.id), Some("2026W53".into()));
        assert_eq!(parse_period("2020W53", None).map(|p| p.id), Some("2020W53".into()));
        assert_eq!(parse_period("20260231", None), None);
        assert_eq!(
            period_issue("2026Q4", PeriodType::Monthly).unwrap(),
            "2026Q4 is a quarterly period; this one is monthly, such as 202611."
        );
    }
}
