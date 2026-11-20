//! The calendar as `Date` counts it in UTC, the zone the web's tests run in: days and moments
//! since 1970, `Date.UTC`, `toISOString`, `new Date(text)`, and a day and a time as
//! `toLocaleString('en-US')` and `Intl.DateTimeFormat('en-US')` write them; and `Date.now`.

/// A minute in milliseconds.
pub const MINUTE: i64 = 60_000;

/// An hour in milliseconds.
pub const HOUR: i64 = 60 * MINUTE;

/// A day in milliseconds.
pub const DAY: i64 = 24 * HOUR;

/// Days since 1970-01-01 of a day of the proleptic Gregorian calendar, its month from 1.
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The year, month from 1 and day of a count of days since 1970-01-01.
pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

/// The days since 1970 of `Date.UTC(year, month, day)`, its month from 0 and either part running
/// over into the next.
pub fn date_utc(year: i64, month: i64, day: i64) -> i64 {
    days_from_civil(year + month.div_euclid(12), month.rem_euclid(12) + 1, 1) + day - 1
}

/// The moment `Date.UTC(year, month, date, hours, minutes, seconds)` gives, the month counted from 0.
pub fn utc(year: i64, month: i64, date: i64, hours: i64, minutes: i64, seconds: i64) -> i64 {
    date_utc(year, month, date) * DAY + hours * HOUR + minutes * MINUTE + seconds * 1000
}

/// The weekday of a count of days since 1970, as `getUTCDay` numbers it: Sunday 0.
pub fn weekday(days: i64) -> i64 {
    (days + 4).rem_euclid(7)
}

/// A day as `toISOString().slice(0, 10)` writes it: `YYYY-MM-DD`, or a sign and six digits for a
/// year past four.
pub fn iso_day(days: i64) -> String {
    let (year, month, day) = civil_from_days(days);
    let full = if (0..=9999).contains(&year) {
        format!("{year:04}-{month:02}-{day:02}")
    } else {
        let sign = if year < 0 { '-' } else { '+' };
        format!("{sign}{:06}-{month:02}-{day:02}", year.abs())
    };
    full.chars().take(10).collect()
}

/// The days since 1970 a day read from `YYYY-MM-DD` falls on, as `new Date(`${day}T00:00:00Z`)`
/// reads it: a month from 1 to 12 and a day from 1 to 31, a day past its month's end running over
/// into the next; none for anything else.
pub fn day_number(day: &str) -> Option<i64> {
    let bytes = day.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(at, byte)| at == 4 || at == 7 || byte.is_ascii_digit())
    {
        return None;
    }
    let year: i64 = day[..4].parse().ok()?;
    let month: i64 = day[5..7].parse().ok()?;
    let date: i64 = day[8..10].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&date) {
        return None;
    }
    Some(days_from_civil(year, month, 1) + date - 1)
}

/// An instant in milliseconds since 1970 as `toISOString` writes it.
pub fn iso_instant(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let rest = ms.rem_euclid(86_400_000);
    let (hour, minute, second, milli) = (rest / 3_600_000, rest / 60_000 % 60, rest / 1000 % 60, rest % 1000);
    format!("{}T{hour:02}:{minute:02}:{second:02}.{milli:03}Z", iso_day(days))
}

fn digits_at(text: &str, at: usize, count: usize) -> Option<i64> {
    let part = text.get(at..at + count)?;
    if part.bytes().all(|byte| byte.is_ascii_digit()) {
        part.parse().ok()
    } else {
        None
    }
}

/// The days in a month of a year, its month from 1.
pub fn days_in_month(year: i64, month: i64) -> i64 {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    days_from_civil(next_year, next_month, 1) - days_from_civil(year, month, 1)
}

/// An ISO text read as `new Date(text)` reads it, in milliseconds since 1970, the zone UTC: a year,
/// a month or a day alone, or a day and a time with or without seconds, a fraction and an offset;
/// none for anything else, as `Date` makes an invalid one.
pub fn parse_instant(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    let year = digits_at(text, 0, 4)?;
    let mut at = 4;
    let (mut month, mut day) = (1, 1);
    if bytes.len() > at && bytes[at] == b'-' {
        month = digits_at(text, at + 1, 2)?;
        at += 3;
        if bytes.len() > at && bytes[at] == b'-' {
            day = digits_at(text, at + 1, 2)?;
            at += 3;
        }
    }
    // A day past the month's end runs over into the next, as `Date` reads it, up to the 31st.
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let mut ms = days_from_civil(year, month, day) * 86_400_000;
    if at == bytes.len() {
        return Some(ms);
    }
    if bytes[at] != b'T' {
        return None;
    }
    let hour = digits_at(text, at + 1, 2)?;
    if bytes.get(at + 3) != Some(&b':') {
        return None;
    }
    let minute = digits_at(text, at + 4, 2)?;
    at += 6;
    let mut second = 0;
    let mut milli = 0;
    if bytes.get(at) == Some(&b':') {
        second = digits_at(text, at + 1, 2)?;
        at += 3;
        if bytes.get(at) == Some(&b'.') {
            let start = at + 1;
            let mut end = start;
            while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            if end == start {
                return None;
            }
            let padded: String = text[start..end].chars().chain("000".chars()).take(3).collect();
            milli = padded.parse::<i64>().ok()?;
            at = end;
        }
    }
    if hour > 24 || minute > 59 || second > 59 {
        return None;
    }
    ms += hour * 3_600_000 + minute * 60_000 + second * 1000 + milli;
    let rest = &text[at..];
    if rest.is_empty() || rest == "Z" {
        return Some(ms);
    }
    let sign = match rest.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    if rest.len() != 6 || rest.as_bytes()[3] != b':' {
        return None;
    }
    let (oh, om) = (digits_at(rest, 1, 2)?, digits_at(rest, 4, 2)?);
    Some(ms - sign * (oh * 3_600_000 + om * 60_000))
}

/// The months as `en-US` writes them short.
pub const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A month's short name in `en-US`, its number from 1.
pub fn month_short(month: i64) -> &'static str {
    MONTHS[(month.clamp(1, 12) - 1) as usize]
}

/// The year, month from 1, day, hours, minutes and seconds of a moment in UTC.
pub fn utc_parts(time: i64) -> (i64, i64, i64, i64, i64, i64) {
    let days = time.div_euclid(86_400_000);
    let rest = time.rem_euclid(86_400_000) / 1000;
    let (year, month, day) = civil_from_days(days);
    (year, month, day, rest / 3600, rest / 60 % 60, rest % 60)
}

/// A day of milliseconds as `Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric', year:
/// 'numeric' })` writes it, the zone UTC: `Jan 5, 2026`.
pub fn intl_date(ms: i64) -> String {
    let (year, month, day) = civil_from_days(ms.div_euclid(86_400_000));
    format!("{} {day}, {year}", MONTHS[(month - 1) as usize])
}

/// An instant as `Intl.DateTimeFormat('en-US')` writes it with the hour and the minute too, the zone
/// UTC: `Jan 5, 2026 at 9:00 AM`.
pub fn intl_date_time(ms: i64) -> String {
    let rest = ms.rem_euclid(86_400_000);
    let (hour, minute) = (rest / 3_600_000, rest / 60_000 % 60);
    let shown = if hour % 12 == 0 { 12 } else { hour % 12 };
    let half = if hour < 12 { "AM" } else { "PM" };
    format!("{} at {shown}:{minute:02} {half}", intl_date(ms))
}

/// A day without its year: `Nov 3`.
pub fn date_short(time: i64) -> String {
    let (_, month, day, ..) = utc_parts(time);
    format!("{} {day}", MONTHS[(month - 1) as usize])
}

/// The start of the day a moment falls in, in UTC, the zone the tests run in.
pub fn start_of_day(time: i64) -> i64 {
    time.div_euclid(86_400_000) * 86_400_000
}

/// A moment as `new Date(ms).toLocaleString('en-US')` writes it in UTC: `11/19/2026, 9:05:00 AM`,
/// the space before the half of the day a plain one as bun writes it.
pub fn locale_date_time(ms: i64) -> String {
    let (year, month, day, hour, minute, second) = utc_parts(ms);
    let twelve = if hour % 12 == 0 { 12 } else { hour % 12 };
    let half = if hour < 12 { "AM" } else { "PM" };
    format!("{month}/{day}/{year}, {twelve}:{minute:02}:{second:02} {half}")
}

/// The moment now, in milliseconds since 1970, as `Date.now` gives it.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as i64)
}
