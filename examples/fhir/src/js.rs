//! What the web's modules lean on the browser for, so the workbench gives the same answers: how a
//! number is rounded (`Math.round`), read (`Number`) and written (`String`, `Intl.NumberFormat`
//! with its decimals and grouping, base 36), what `\s` and `trim` take as space, how `en-US`
//! collates text (`localeCompare`), the calendar's days in UTC as `Date` counts them, an instant
//! read and written as `Date` does and as `Intl.DateTimeFormat('en-US')` writes it, JSON read as
//! `JSON.parse` reads it (JavaScriptCore's words for what it cannot read) and written as
//! `JSON.stringify` writes it, base64 as `btoa` and `atob` make and read it, a query string as
//! `URLSearchParams` reads it, and text folded as `normalize('NFD')` without its accents.
//!
//! Carried from the dhis2 example (`examples/dhis2/src/js.rs`), with what the workbench leans on
//! beyond it.

use std::cmp::Ordering;

use serde_json::{Map, Value};

/// Whether a character is one the web's `\s` matches.
pub fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// A text without the spaces at its ends, as `String.prototype.trim` cuts them.
pub fn trim(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// A number rounded as `Math.round` rounds it: to the nearest whole number, a half up.
pub fn round(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let floor = value.floor();
    if value - floor >= 0.5 { floor + 1.0 } else { floor }
}

/// A number as `String(number)` writes it: the fewest digits that read back as it, in figures up
/// to 21 of them and with an exponent past that.
pub fn number_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if value < 0.0 {
        return format!("-{}", number_to_string(-value));
    }
    let written = format!("{value:e}");
    let (mantissa, exponent) = written.split_once('e').unwrap_or((&written, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let k = digits.len() as i64;
    let n = exponent.parse::<i64>().unwrap_or(0) + 1;
    if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let sign = if n - 1 < 0 { '-' } else { '+' };
        let rest = if k > 1 {
            format!(".{}", &digits[1..])
        } else {
            String::new()
        };
        format!("{}{rest}e{sign}{}", &digits[..1], (n - 1).abs())
    }
}

/// The digits of a number's exact decimal value, rounded at `digits` places after the point with a
/// half going up, as `toFixed` rounds.
fn round_decimal(whole: &str, fraction: &str, digits: usize) -> (String, String) {
    let mut kept: Vec<u8> = whole.bytes().chain(fraction.bytes().take(digits)).collect();
    while kept.len() < whole.len() + digits {
        kept.push(b'0');
    }
    if fraction.as_bytes().get(digits).is_some_and(|next| *next >= b'5') {
        let mut at = kept.len();
        loop {
            if at == 0 {
                kept.insert(0, b'1');
                break;
            }
            at -= 1;
            if kept[at] == b'9' {
                kept[at] = b'0';
            } else {
                kept[at] += 1;
                break;
            }
        }
    }
    let split = kept.len() - digits;
    let text = String::from_utf8(kept).unwrap_or_default();
    (text[..split].to_owned(), text[split..].to_owned())
}

/// A number's shortest decimal, the one that reads back as it (as ICU starts from), with its point
/// moved `shift` places to the left, rounded at `digits` places after the point with a half going
/// up: its whole part and its fraction.
fn shifted(value: f64, shift: usize, digits: usize) -> (String, String) {
    let written = format!("{:e}", value.abs());
    let (mantissa, exponent) = written.split_once('e').unwrap_or((&written, "0"));
    let figures: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let point = exponent.parse::<i64>().unwrap_or(0) + 1;
    let (whole, fraction) = if point <= 0 {
        ("0".to_owned(), format!("{}{figures}", "0".repeat((-point) as usize)))
    } else if point as usize >= figures.len() {
        (
            format!("{figures}{}", "0".repeat(point as usize - figures.len())),
            String::new(),
        )
    } else {
        (
            figures[..point as usize].to_owned(),
            figures[point as usize..].to_owned(),
        )
    };
    let (whole, fraction) = (whole.as_str(), fraction.as_str());
    let whole = whole.trim_start_matches('0');
    let padded = format!("{}{whole}", "0".repeat(shift.saturating_sub(whole.len())));
    let split = padded.len() - shift;
    let (head, moved) = padded.split_at(split);
    let fraction = format!("{moved}{fraction}");
    let (head, fraction) = round_decimal(if head.is_empty() { "0" } else { head }, &fraction, digits);
    let head = head.trim_start_matches('0');
    (
        if head.is_empty() {
            "0".to_owned()
        } else {
            head.to_owned()
        },
        fraction,
    )
}

/// A whole part with a comma between each three digits, as `en-US` groups them.
fn grouped(whole: &str) -> String {
    let mut out = String::new();
    for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// The compact units `en-US` writes, by the power of ten each stands for.
const UNITS: [(usize, &str); 4] = [(3, "K"), (6, "M"), (9, "B"), (12, "T")];

/// Whether a number is negative, `-0` included, as `Intl.NumberFormat` writes its sign.
fn negative(value: f64) -> bool {
    value < 0.0 || (value == 0.0 && value.is_sign_negative())
}

/// A whole part with commas between each three digits where `grouping` asks for them, as
/// `useGrouping: true` asks for them always, in compact notation too.
fn grouped_if(whole: &str, grouping: bool) -> String {
    if grouping { grouped(whole) } else { whole.to_owned() }
}

/// A fraction kept to `min` places at the least, its trailing zeros past those left out.
fn fraction_kept(fraction: &str, min: usize) -> String {
    let mut kept = fraction.trim_end_matches('0').to_owned();
    while kept.len() < min {
        kept.push('0');
    }
    kept
}

/// A number as `new Intl.NumberFormat('en-US', options).format` writes it, with
/// `minimumFractionDigits`, `maximumFractionDigits`, `useGrouping` and the standard or compact
/// notation: rounded from its shortest decimal with a half going up, a minus before any negative
/// number, `-0` included.
pub fn intl_number(value: f64, min_fraction: usize, max_fraction: usize, grouping: bool, compact: bool) -> String {
    if !value.is_finite() {
        return if value.is_nan() {
            "NaN".to_owned()
        } else if value > 0.0 {
            "∞".to_owned()
        } else {
            "-∞".to_owned()
        };
    }
    let sign = if negative(value) { "-" } else { "" };
    let max_fraction = max_fraction.max(min_fraction);
    if !compact {
        let (whole, fraction) = shifted(value, 0, max_fraction);
        let fraction = fraction_kept(&fraction, min_fraction);
        let whole = grouped_if(&whole, grouping);
        return if fraction.is_empty() {
            format!("{sign}{whole}")
        } else {
            format!("{sign}{whole}.{fraction}")
        };
    }
    let digits = if value.abs() < 1.0 {
        0
    } else {
        let written = format!("{:e}", value.abs());
        written
            .split_once('e')
            .map_or(0, |(_, exponent)| exponent.parse::<usize>().unwrap_or(0) + 1)
    };
    let mut unit = UNITS
        .iter()
        .rposition(|(power, _)| digits > *power)
        .map_or(0, |at| at + 1);
    loop {
        let (power, suffix) = if unit == 0 { (0, "") } else { UNITS[unit - 1] };
        let (whole, fraction) = shifted(value, power, max_fraction);
        if whole.len() > 3 && unit < UNITS.len() {
            unit += 1;
            continue;
        }
        let fraction = fraction_kept(&fraction, min_fraction);
        let whole = grouped_if(&whole, grouping);
        return if fraction.is_empty() {
            format!("{sign}{whole}{suffix}")
        } else {
            format!("{sign}{whole}.{fraction}{suffix}")
        };
    }
}

/// A text read as `Number(text)` reads it: spaces at its ends left out, nothing as 0, a decimal
/// literal, `Infinity`, or a whole number in hexadecimal, octal or binary after `0x`, `0o` or
/// `0b`; anything else is NaN.
pub fn number_of_text(text: &str) -> f64 {
    let text = trim(text);
    if text.is_empty() {
        return 0.0;
    }
    let lower = text.as_bytes();
    if lower.len() > 2 && lower[0] == b'0' {
        let radix = match lower[1] {
            b'x' | b'X' => 16,
            b'o' | b'O' => 8,
            b'b' | b'B' => 2,
            _ => 0,
        };
        if radix > 0 {
            let mut value = 0.0_f64;
            for c in text[2..].chars() {
                match c.to_digit(radix) {
                    Some(digit) => value = value * f64::from(radix) + f64::from(digit),
                    None => return f64::NAN,
                }
            }
            return value;
        }
    }
    let (sign, body) = match text.as_bytes()[0] {
        b'+' => (1.0, &text[1..]),
        b'-' => (-1.0, &text[1..]),
        _ => (1.0, text),
    };
    if body == "Infinity" {
        return sign * f64::INFINITY;
    }
    if !is_decimal_literal(body) {
        return f64::NAN;
    }
    body.parse::<f64>().map_or(f64::NAN, |value| sign * value)
}

/// Whether a text is a decimal literal without its sign: digits with a point among or after them,
/// or a point before digits, and an exponent after.
fn is_decimal_literal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0;
    let digits = |at: &mut usize| {
        let start = *at;
        while *at < bytes.len() && bytes[*at].is_ascii_digit() {
            *at += 1;
        }
        *at - start
    };
    let whole = digits(&mut at);
    let mut fraction = 0;
    if at < bytes.len() && bytes[at] == b'.' {
        at += 1;
        fraction = digits(&mut at);
    }
    if whole == 0 && fraction == 0 {
        return false;
    }
    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
        at += 1;
        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
            at += 1;
        }
        if digits(&mut at) == 0 {
            return false;
        }
    }
    at == bytes.len()
}

/// A whole number in base 36, as `Number.prototype.toString(36)` writes it.
pub fn base36(value: i64) -> String {
    if value == 0 {
        return "0".to_owned();
    }
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut left = value.unsigned_abs();
    let mut out = Vec::new();
    while left > 0 {
        out.push(digits[(left % 36) as usize]);
        left /= 36;
    }
    out.reverse();
    let text = String::from_utf8(out).unwrap_or_default();
    if value < 0 { format!("-{text}") } else { text }
}

// --- The calendar ---------------------------------------------------------------------------------

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

// --- Collation ------------------------------------------------------------------------------------

/// Spaces, punctuation and symbols in the order `en-US` collates them, before digits and letters.
const MARKS: &str = "\t\u{b}\u{c}\r\n \u{a0}_-–—,;:!?.…·'‘’\"“”()[]{}@*/\\&#%`^+<=>|~$";

/// A letter's base letters, its accent's weight and whether it is an expansion, for the accented
/// Latin letters a text in English meets; none for a letter of its own.
fn latin(c: char) -> Option<(&'static str, u32, bool)> {
    let lower = c.to_lowercase().next().unwrap_or(c);
    let (base, accent) = match lower {
        'á' => ("a", 1),
        'à' => ("a", 2),
        'â' => ("a", 3),
        'ä' => ("a", 4),
        'ã' => ("a", 5),
        'å' => ("a", 6),
        'ç' => ("c", 1),
        'é' => ("e", 1),
        'è' => ("e", 2),
        'ê' => ("e", 3),
        'ë' => ("e", 4),
        'í' => ("i", 1),
        'ì' => ("i", 2),
        'î' => ("i", 3),
        'ï' => ("i", 4),
        'ñ' => ("n", 5),
        'ó' => ("o", 1),
        'ò' => ("o", 2),
        'ô' => ("o", 3),
        'ö' => ("o", 4),
        'õ' => ("o", 5),
        'ø' => ("o", 7),
        'ú' => ("u", 1),
        'ù' => ("u", 2),
        'û' => ("u", 3),
        'ü' => ("u", 4),
        'ý' => ("y", 1),
        'ÿ' => ("y", 4),
        'æ' => return Some(("ae", 0, true)),
        'œ' => return Some(("oe", 0, true)),
        'ß' => return Some(("ss", 0, true)),
        _ => return None,
    };
    Some((base, accent, false))
}

/// One collation element: its primary weights, its accent and its case.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Element {
    primary: Vec<u32>,
    secondary: u32,
    tertiary: u32,
}

const CLASS_MARK: u32 = 1;
const CLASS_DIGIT: u32 = 2;
const CLASS_LETTER: u32 = 3;
const CLASS_OTHER: u32 = 4;

/// A text's collation elements: marks by their place in `MARKS`, digits by their value or, with
/// `numeric`, each run of them by the number it writes, letters by their base letter with the
/// accent and the case after; control characters are left out.
fn elements(text: &str, numeric: bool) -> Vec<Element> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        if c.is_ascii_digit() {
            if numeric {
                let start = at;
                while at < chars.len() && chars[at].is_ascii_digit() {
                    at += 1;
                }
                let run: String = chars[start..at].iter().collect();
                let run = run.trim_start_matches('0');
                let mut primary = vec![CLASS_DIGIT, run.len() as u32];
                primary.extend(run.bytes().map(u32::from));
                out.push(Element {
                    primary,
                    secondary: 0,
                    tertiary: 0,
                });
                continue;
            }
            out.push(Element {
                primary: vec![CLASS_DIGIT, 1, u32::from(c)],
                secondary: 0,
                tertiary: 0,
            });
        } else if let Some(place) = MARKS.chars().position(|mark| mark == c) {
            out.push(Element {
                primary: vec![CLASS_MARK, place as u32],
                secondary: 0,
                tertiary: 0,
            });
        } else if c.is_control() {
        } else if let Some((base, accent, expansion)) = latin(c) {
            let upper = u32::from(c.is_uppercase());
            for letter in base.chars() {
                out.push(Element {
                    primary: vec![CLASS_LETTER, u32::from(letter)],
                    secondary: accent,
                    tertiary: upper + if expansion { 2 } else { 0 },
                });
            }
        } else if c.is_ascii_alphabetic() {
            out.push(Element {
                primary: vec![CLASS_LETTER, u32::from(c.to_ascii_lowercase())],
                secondary: 0,
                tertiary: u32::from(c.is_ascii_uppercase()),
            });
        } else {
            let lower = c.to_lowercase().next().unwrap_or(c);
            out.push(Element {
                primary: vec![CLASS_OTHER, u32::from(lower)],
                secondary: 0,
                tertiary: u32::from(lower != c),
            });
        }
        at += 1;
    }
    out
}

/// Two texts compared as `a.localeCompare(b, 'en-US', { numeric })` compares them: by their
/// letters, digits and marks first, then their accents, then their case, lowercase first; texts
/// alike at every level are equal.
pub fn locale_compare(a: &str, b: &str, numeric: bool) -> Ordering {
    let (a, b) = (elements(a, numeric), elements(b, numeric));
    let primary = |of: &[Element]| of.iter().map(|one| one.primary.clone()).collect::<Vec<_>>();
    primary(&a)
        .cmp(&primary(&b))
        .then_with(|| {
            let secondary = |of: &[Element]| of.iter().map(|one| one.secondary).collect::<Vec<_>>();
            secondary(&a).cmp(&secondary(&b))
        })
        .then_with(|| {
            let tertiary = |of: &[Element]| of.iter().map(|one| one.tertiary).collect::<Vec<_>>();
            tertiary(&a).cmp(&tertiary(&b))
        })
}

// --- Instants ---------------------------------------------------------------------------------------

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

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

/// A month's short name in `en-US`, its number from 1.
pub fn month_short(month: i64) -> &'static str {
    MONTHS[(month.clamp(1, 12) - 1) as usize]
}

// --- JSON -------------------------------------------------------------------------------------------

/// A number as JSON holds it: a whole one as an integer, so it is written without a point.
pub fn number(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0 {
        Value::from(value as i64)
    } else {
        serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
    }
}

fn quote_into(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A text as `JSON.stringify` quotes it.
pub fn quote(text: &str) -> String {
    let mut out = String::new();
    quote_into(text, &mut out);
    out
}

fn write_json(value: &Value, indent: usize, depth: usize, out: &mut String) {
    let pad = |out: &mut String, depth: usize| {
        if indent > 0 {
            out.push('\n');
            out.push_str(&" ".repeat(indent * depth));
        }
    };
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(figure) => {
            let value = figure.as_f64().unwrap_or(0.0);
            out.push_str(&if value.is_finite() {
                number_to_string(value)
            } else {
                "null".to_owned()
            });
        }
        Value::String(text) => quote_into(text, out),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                pad(out, depth + 1);
                write_json(item, indent, depth + 1, out);
            }
            pad(out, depth);
            out.push(']');
        }
        Value::Object(fields) => {
            if fields.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (at, (key, item)) in fields.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                pad(out, depth + 1);
                quote_into(key, out);
                out.push(':');
                if indent > 0 {
                    out.push(' ');
                }
                write_json(item, indent, depth + 1, out);
            }
            pad(out, depth);
            out.push('}');
        }
    }
}

/// A value as `JSON.stringify(value)` writes it.
pub fn stringify(value: &Value) -> String {
    let mut out = String::new();
    write_json(value, 0, 0, &mut out);
    out
}

/// A value as `JSON.stringify(value, null, 2)` writes it.
pub fn pretty(value: &Value) -> String {
    let mut out = String::new();
    write_json(value, 2, 0, &mut out);
    out
}

struct Reader<'a> {
    bytes: &'a [u8],
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn peek(&mut self) -> Option<u8> {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.at += 1;
        }
        self.bytes.get(self.at).copied()
    }

    fn token_error(&self) -> String {
        match self.text[self.at..].chars().next() {
            None => "Unexpected EOF".to_owned(),
            Some('\'') => "Single quotes (') are not allowed in JSON".to_owned(),
            Some(c) if c.is_alphabetic() || c == '_' || c == '$' => {
                let word: String = self.text[self.at..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
                    .collect();
                format!("Unexpected identifier \"{word}\"")
            }
            Some(c) => format!("Unexpected token '{c}'"),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.at += 1;
        let mut out = String::new();
        loop {
            let Some(c) = self.text[self.at..].chars().next() else {
                return Err("Unterminated string".to_owned());
            };
            self.at += c.len_utf8();
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let Some(next) = self.text[self.at..].chars().next() else {
                        return Err("Unterminated string".to_owned());
                    };
                    self.at += next.len_utf8();
                    match next {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let hex = self.text.get(self.at..self.at + 4).unwrap_or("");
                            let code = u32::from_str_radix(hex, 16)
                                .ok()
                                .filter(|_| hex.len() == 4)
                                .ok_or_else(|| "\"\\u\" must be followed by 4 hex digits".to_owned())?;
                            self.at += 4;
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        _ => return Err("Invalid escape character".to_owned()),
                    }
                }
                c if (c as u32) < 0x20 => return Err("Unterminated string".to_owned()),
                c => out.push(c),
            }
        }
    }

    fn digits(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.at;
        if self.bytes[self.at] == b'-' {
            self.at += 1;
            if !self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                return Err("Invalid number".to_owned());
            }
        }
        if self.bytes[self.at] == b'0' {
            self.at += 1;
        } else {
            self.digits();
        }
        if self.bytes.get(self.at) == Some(&b'.') && self.bytes.get(self.at + 1).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
            self.digits();
        }
        if matches!(self.bytes.get(self.at), Some(b'e' | b'E')) {
            let mut end = self.at + 1;
            if matches!(self.bytes.get(end), Some(b'+' | b'-')) {
                end += 1;
            }
            if self.bytes.get(end).is_some_and(u8::is_ascii_digit) {
                self.at = end;
                self.digits();
            }
        }
        let value: f64 = self.text[start..self.at].parse().unwrap_or(f64::NAN);
        Ok(number(value))
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek() {
            None => Err("Unexpected EOF".to_owned()),
            Some(b'"') => self.string().map(Value::String),
            Some(b'{') => {
                self.at += 1;
                let mut fields = Map::new();
                if self.peek() == Some(b'}') {
                    self.at += 1;
                    return Ok(Value::Object(fields));
                }
                loop {
                    match self.peek() {
                        Some(b'"') => {}
                        None => return Err("Expected '}'".to_owned()),
                        Some(b'\'') => return Err(self.token_error()),
                        Some(_) => return Err("Property name must be a string literal".to_owned()),
                    }
                    let key = self.string()?;
                    if self.peek() != Some(b':') {
                        return Err("Expected ':' before value in object property definition".to_owned());
                    }
                    self.at += 1;
                    let item = self.value()?;
                    fields.insert(key, item);
                    match self.peek() {
                        Some(b',') => self.at += 1,
                        Some(b'}') => {
                            self.at += 1;
                            return Ok(Value::Object(fields));
                        }
                        _ => return Err("Expected '}'".to_owned()),
                    }
                }
            }
            Some(b'[') => {
                self.at += 1;
                let mut items = Vec::new();
                if self.peek() == Some(b']') {
                    self.at += 1;
                    return Ok(Value::Array(items));
                }
                loop {
                    items.push(self.value()?);
                    match self.peek() {
                        Some(b',') => {
                            self.at += 1;
                            if self.peek() == Some(b']') {
                                return Err("Unexpected comma at the end of array expression".to_owned());
                            }
                        }
                        Some(b']') => {
                            self.at += 1;
                            return Ok(Value::Array(items));
                        }
                        _ => return Err("Expected ']'".to_owned()),
                    }
                }
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => {
                for (word, value) in [
                    ("true", Value::Bool(true)),
                    ("false", Value::Bool(false)),
                    ("null", Value::Null),
                ] {
                    let after = self.text[self.at..].strip_prefix(word);
                    if after.is_some_and(|rest| {
                        !rest
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '$')
                    }) {
                        self.at += word.len();
                        return Ok(value);
                    }
                }
                Err(self.token_error())
            }
        }
    }
}

/// A text read as `JSON.parse` reads it, or the words JavaScriptCore refuses it with. (An object's
/// keys keep the order they are written in, where JavaScript would put whole-number keys first.)
pub fn parse_json(text: &str) -> Result<Value, String> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        text,
        at: 0,
    };
    let value = reader.value().map_err(|why| format!("JSON Parse error: {why}"))?;
    if reader.peek().is_some() {
        return Err("JSON Parse error: Unable to parse JSON string".to_owned());
    }
    Ok(value)
}

// --- Base64 -----------------------------------------------------------------------------------------

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Bytes as `btoa` writes them, padded.
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for at in 0..4 {
            if at <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - 6 * at)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Base64 read as `atob` reads it: spaces left out, the padding optional; none for anything it
/// refuses.
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut clean: Vec<u8> = text
        .bytes()
        .filter(|byte| !matches!(byte, b'\t' | b'\n' | b'\x0c' | b'\r' | b' '))
        .collect();
    if clean.len().is_multiple_of(4) {
        for _ in 0..2 {
            if clean.last() == Some(&b'=') {
                clean.pop();
            }
        }
    }
    if clean.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::new();
    let mut bits = 0u32;
    let mut count = 0;
    for byte in clean {
        let value = ALPHABET.iter().position(|one| *one == byte)? as u32;
        bits = (bits << 6) | value;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    Some(out)
}

// --- Query strings ----------------------------------------------------------------------------------

fn decode_form(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' => out.push(b' '),
            b'%' if at + 2 < bytes.len() && bytes[at + 1].is_ascii_hexdigit() && bytes[at + 2].is_ascii_hexdigit() => {
                out.push(u8::from_str_radix(&text[at + 1..at + 3], 16).unwrap_or(b'%'));
                at += 3;
                continue;
            }
            byte => out.push(byte),
        }
        at += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A query string's pairs as `new URLSearchParams(text).entries()` gives them.
pub fn search_params(text: &str) -> Vec<(String, String)> {
    let text = text.strip_prefix('?').unwrap_or(text);
    text.split('&')
        .filter(|piece| !piece.is_empty())
        .map(|piece| match piece.split_once('=') {
            Some((key, value)) => (decode_form(key), decode_form(value)),
            None => (decode_form(piece), String::new()),
        })
        .collect()
}

// --- Folding ----------------------------------------------------------------------------------------

/// The letter an accented Latin letter is under `normalize('NFD')` without its marks.
fn unaccented(c: char) -> char {
    const FROM: &str = "ÀÁÂÃÄÅÇÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåçèéêëìíîïñòóôõöùúûüýÿĀāĂăĄąĆćĈĉĊċČčĎďĒēĔĕĖėĘęĚěĜĝĞğĠġĢģĤĥĨĩĪīĬĭĮįİĴĵĶķĹĺĻļĽľŃńŅņŇňŌōŎŏŐőŔŕŖŗŘřŚśŜŝŞşŠšŢţŤťŨũŪūŬŭŮůŰűŲųŴŵŶŷŸŹźŻżŽž";
    const TO: &str = "AAAAAACEEEEIIIINOOOOOUUUUYaaaaaaceeeeiiiinooooouuuuyyAaAaAaCcCcCcCcDdEeEeEeEeEeGgGgGgGgHhIiIiIiIiIJjKkLlLlLlNnNnNnOoOoOoRrRrRrSsSsSsSsTtTtUuUuUuUuUuUuWwYyYZzZzZz";
    FROM.chars()
        .position(|one| one == c)
        .and_then(|at| TO.chars().nth(at))
        .unwrap_or(c)
}

/// A text as the web folds it to match: `normalize('NFD')`, its combining marks left out, in lower
/// case.
pub fn fold(text: &str) -> String {
    text.chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .map(unaccented)
        .collect::<String>()
        .to_lowercase()
}

/// The part of a text before its `n`-th UTF-16 unit, as `slice(0, n)` cuts it.
pub fn slice_units(text: &str, n: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().take(n).collect();
    String::from_utf16_lossy(&units)
}
