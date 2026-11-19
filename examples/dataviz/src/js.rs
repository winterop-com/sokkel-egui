//! What the web's modules lean on the browser for, so the visualizer gives the same answers: how a
//! number is rounded (`Math.round`), read (`Number`) and written (`String`, `toFixed`,
//! `toLocaleString`, `Intl.NumberFormat` with its decimals, grouping and compact notation), what
//! `\s` and `trim` take as space, how `en-US` collates text (`localeCompare`, with `numeric` or
//! without), how `JSON.parse` reads a document and says what it cannot, and how `JSON.stringify`
//! writes one, and the calendar's days in UTC as `Date` counts them.
//!
//! The rounding, `String` and `toLocaleString` are carried over from the video example
//! (`examples/video/src/js.rs`); the rest is the visualizer's own.

use std::cmp::Ordering;

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

/// A number with `digits` places after the point, as `toFixed` writes it: rounded from its exact
/// value, a half going up.
pub fn to_fixed(value: f64, digits: usize) -> String {
    if !value.is_finite() || value.abs() >= 1e21 {
        return number_to_string(value);
    }
    let exact = format!("{:.1074}", value.abs());
    let (whole, fraction) = exact.split_once('.').unwrap_or((&exact, ""));
    let (whole, fraction) = round_decimal(whole, fraction, digits);
    let sign = if value < 0.0 { "-" } else { "" };
    if digits == 0 {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{fraction}")
    }
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

/// A whole part and a fraction written with the fraction's trailing zeros left out.
fn joined(whole: &str, fraction: &str) -> String {
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        whole.to_owned()
    } else {
        format!("{whole}.{fraction}")
    }
}

/// A number as `toLocaleString('en-US')` writes it: grouped by commas, at most three places after
/// the point, a half going away from zero.
pub fn locale_number(value: f64) -> String {
    if !value.is_finite() {
        return if value.is_nan() {
            "NaN".to_owned()
        } else if value > 0.0 {
            "∞".to_owned()
        } else {
            "-∞".to_owned()
        };
    }
    let (whole, fraction) = shifted(value, 0, 3);
    let written = joined(&grouped(&whole), &fraction);
    if value < 0.0 && written != "0" {
        format!("-{written}")
    } else {
        written
    }
}

/// The compact units `en-US` writes, by the power of ten each stands for.
const UNITS: [(usize, &str); 4] = [(3, "K"), (6, "M"), (9, "B"), (12, "T")];

/// A number as `Intl.NumberFormat('en-US', { notation: 'compact', maximumFractionDigits: 1 })`
/// writes it: `52.4K`, `1.2M`, `950`; a number that rounds up to the next unit is written in it.
pub fn compact(value: f64) -> String {
    if !value.is_finite() {
        return locale_number(value);
    }
    // The whole part's digits, before any rounding.
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
        let (whole, fraction) = shifted(value, power, 1);
        if whole.len() > 3 && unit < UNITS.len() {
            unit += 1;
            continue;
        }
        let written = format!("{}{suffix}", joined(&whole, &fraction));
        return if value < 0.0 && joined(&whole, &fraction) != "0" {
            format!("-{written}")
        } else {
            written
        };
    }
}

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

/// A count as `toLocaleString('en-US')` writes a whole number: `1,080`.
pub fn locale_count(count: usize) -> String {
    grouped(&count.to_string())
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

// --- JSON -----------------------------------------------------------------------------------------

/// A value `JSON.parse` gives: its objects' keys in the order JavaScript keeps them, the ones that
/// read as array indexes first, rising, then the rest as they first came.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The value under a key of an object, or under an index of an array, as `value[key]` reads it.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries.iter().find(|(name, _)| name == key).map(|(_, value)| value),
            Json::Array(items) => index_of_key(key).and_then(|at| items.get(at as usize)),
            _ => None,
        }
    }

    /// Its keys, as `Object.keys` lists them.
    pub fn keys(&self) -> Vec<String> {
        match self {
            Json::Object(entries) => entries.iter().map(|(name, _)| name.clone()).collect(),
            Json::Array(items) => (0..items.len()).map(|at| at.to_string()).collect(),
            Json::Text(text) => (0..text.encode_utf16().count()).map(|at| at.to_string()).collect(),
            _ => Vec::new(),
        }
    }

    /// Whether it is an object or an array, as `typeof value === 'object'` and not null.
    pub fn is_object(&self) -> bool {
        matches!(self, Json::Object(_) | Json::Array(_))
    }

    /// It as `JSON.stringify` writes it, without spaces.
    pub fn stringify(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Json::Number(value) => {
                if value.is_finite() {
                    out.push_str(&number_to_string(*value));
                } else {
                    out.push_str("null");
                }
            }
            Json::Text(text) => out.push_str(&quoted(text)),
            Json::Array(items) => {
                out.push('[');
                for (at, item) in items.iter().enumerate() {
                    if at > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Object(entries) => {
                out.push('{');
                for (at, (key, value)) in entries.iter().enumerate() {
                    if at > 0 {
                        out.push(',');
                    }
                    out.push_str(&quoted(key));
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// A text as `JSON.stringify` quotes it.
pub fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_default()
}

/// The index a key names where it is one, as an array index is: a whole number below 2^32 - 1
/// written without leading zeros.
fn index_of_key(key: &str) -> Option<u32> {
    if key.is_empty() || (key.len() > 1 && key.starts_with('0')) || !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    key.parse::<u64>()
        .ok()
        .filter(|value| *value < 4_294_967_295)
        .map(|value| value as u32)
}

/// An object's entries in the order JavaScript keeps them.
fn ordered(entries: Vec<(String, Json)>) -> Vec<(String, Json)> {
    let (mut indexes, rest): (Vec<_>, Vec<_>) = entries.into_iter().partition(|(key, _)| index_of_key(key).is_some());
    indexes.sort_by_key(|(key, _)| index_of_key(key));
    indexes.extend(rest);
    indexes
}

/// A text read as `JSON.parse` reads it, or the message bun's `SyntaxError` gives for what it
/// cannot read.
pub fn json_parse(text: &str) -> Result<Json, String> {
    let mut parser = JsonParser {
        chars: text.chars().collect(),
        at: 0,
    };
    let value = parser.value()?;
    parser.space();
    if parser.at < parser.chars.len() {
        return Err(parser.fail("Unable to parse JSON string"));
    }
    Ok(value)
}

/// What a JSON document is read into, a token at a time, as JavaScriptCore's parser reads it.
enum Token {
    Open(char),
    Close(char),
    Comma,
    Colon,
    Text(String),
    Number(f64),
    Word(String),
    End,
}

struct JsonParser {
    chars: Vec<char>,
    at: usize,
}

impl JsonParser {
    fn fail(&self, message: &str) -> String {
        format!("JSON Parse error: {message}")
    }

    fn space(&mut self) {
        while self.at < self.chars.len() && matches!(self.chars[self.at], ' ' | '\t' | '\n' | '\r') {
            self.at += 1;
        }
    }

    fn token(&mut self) -> Result<Token, String> {
        self.space();
        let Some(&c) = self.chars.get(self.at) else {
            return Ok(Token::End);
        };
        match c {
            '{' | '[' => {
                self.at += 1;
                Ok(Token::Open(c))
            }
            '}' | ']' => {
                self.at += 1;
                Ok(Token::Close(c))
            }
            ',' => {
                self.at += 1;
                Ok(Token::Comma)
            }
            ':' => {
                self.at += 1;
                Ok(Token::Colon)
            }
            '"' => self.text(),
            '\'' => Err(self.fail("Single quotes (') are not allowed in JSON")),
            '-' | '0'..='9' => self.number(),
            c if c.is_ascii_alphabetic() || c == '_' || c == '$' => {
                let start = self.at;
                while self
                    .chars
                    .get(self.at)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
                {
                    self.at += 1;
                }
                Ok(Token::Word(self.chars[start..self.at].iter().collect()))
            }
            '.' => Err(self.fail("Unexpected token '.'")),
            _ => Err(self.fail(&format!("Unrecognized token '{c}'"))),
        }
    }

    fn number(&mut self) -> Result<Token, String> {
        let start = self.at;
        if self.chars[self.at] == '-' {
            self.at += 1;
            if !self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                return Err(self.fail("Invalid number"));
            }
        }
        if self.chars[self.at] == '0' {
            self.at += 1;
        } else {
            while self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                self.at += 1;
            }
        }
        if self.chars.get(self.at) == Some(&'.') {
            self.at += 1;
            if !self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                return Err(self.fail("Invalid digits after decimal point"));
            }
            while self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                self.at += 1;
            }
        }
        if matches!(self.chars.get(self.at), Some('e' | 'E')) {
            let before = self.at;
            self.at += 1;
            if matches!(self.chars.get(self.at), Some('+' | '-')) {
                self.at += 1;
            }
            if self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                while self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                    self.at += 1;
                }
            } else {
                self.at = before;
            }
        }
        let written: String = self.chars[start..self.at].iter().collect();
        Ok(Token::Number(written.parse::<f64>().unwrap_or(f64::NAN)))
    }

    fn text(&mut self) -> Result<Token, String> {
        self.at += 1;
        let mut out = String::new();
        let mut units: Vec<u16> = Vec::new();
        let flush = |units: &mut Vec<u16>, out: &mut String| {
            if !units.is_empty() {
                out.push_str(&String::from_utf16_lossy(units));
                units.clear();
            }
        };
        loop {
            let Some(&c) = self.chars.get(self.at) else {
                return Err(self.fail("Unterminated string"));
            };
            match c {
                '"' => {
                    self.at += 1;
                    flush(&mut units, &mut out);
                    return Ok(Token::Text(out));
                }
                '\\' => {
                    let Some(&next) = self.chars.get(self.at + 1) else {
                        return Err(self.fail("Unterminated string"));
                    };
                    let plain = match next {
                        '"' => Some('"'),
                        '\\' => Some('\\'),
                        '/' => Some('/'),
                        'b' => Some('\u{8}'),
                        'f' => Some('\u{c}'),
                        'n' => Some('\n'),
                        'r' => Some('\r'),
                        't' => Some('\t'),
                        'u' => None,
                        other => return Err(self.fail(&format!("Invalid escape character {other}"))),
                    };
                    if let Some(plain) = plain {
                        flush(&mut units, &mut out);
                        out.push(plain);
                        self.at += 2;
                        continue;
                    }
                    let hex: String = self.chars[self.at + 2..(self.at + 6).min(self.chars.len())]
                        .iter()
                        .collect();
                    match u16::from_str_radix(&hex, 16) {
                        Ok(unit) if hex.len() == 4 && hex.chars().all(|c| c.is_ascii_hexdigit()) => {
                            units.push(unit);
                            self.at += 6;
                        }
                        _ => {
                            let shown: String = self.chars[self.at..(self.at + 6).min(self.chars.len())]
                                .iter()
                                .collect();
                            return Err(self.fail(&format!("\"{shown}\" is not a valid unicode escape")));
                        }
                    }
                }
                c if (c as u32) < 0x20 => return Err(self.fail("Unterminated string")),
                c => {
                    flush(&mut units, &mut out);
                    out.push(c);
                    self.at += 1;
                }
            }
        }
    }

    /// A value where one must stand.
    fn value(&mut self) -> Result<Json, String> {
        let token = self.token()?;
        self.value_from(token)
    }

    fn value_from(&mut self, token: Token) -> Result<Json, String> {
        match token {
            Token::Open('[') => self.array(),
            Token::Open(_) => self.object(),
            Token::Text(text) => Ok(Json::Text(text)),
            Token::Number(value) => Ok(Json::Number(value)),
            Token::Word(word) => match word.as_str() {
                "true" => Ok(Json::Bool(true)),
                "false" => Ok(Json::Bool(false)),
                "null" => Ok(Json::Null),
                _ => Err(self.fail(&format!("Unexpected identifier \"{word}\""))),
            },
            Token::End => Err(self.fail("Unexpected EOF")),
            Token::Close(c) => Err(self.fail(&format!("Unexpected token '{c}'"))),
            Token::Comma => Err(self.fail("Unexpected token ','")),
            Token::Colon => Err(self.fail("Unexpected token ':'")),
        }
    }

    /// A token after a value, where an error of its own says no more than that the close was
    /// wanted.
    fn after(&mut self) -> Option<Token> {
        self.token().ok()
    }

    fn array(&mut self) -> Result<Json, String> {
        let mut items = Vec::new();
        let first = self.token()?;
        if matches!(first, Token::Close(']')) {
            return Ok(Json::Array(items));
        }
        items.push(self.value_from(first)?);
        loop {
            match self.after() {
                Some(Token::Close(']')) => return Ok(Json::Array(items)),
                Some(Token::Comma) => {
                    let next = self.token()?;
                    if matches!(next, Token::Close(']')) {
                        return Err(self.fail("Unexpected comma at the end of array expression"));
                    }
                    items.push(self.value_from(next)?);
                }
                _ => return Err(self.fail("Expected ']'")),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        let mut entries: Vec<(String, Json)> = Vec::new();
        let first = self.after();
        let mut key = match first {
            Some(Token::Close('}')) => return Ok(Json::Object(entries)),
            Some(Token::Text(key)) => key,
            _ => return Err(self.fail("Expected '}'")),
        };
        loop {
            if !matches!(self.after(), Some(Token::Colon)) {
                return Err(self.fail("Expected ':' before value in object property definition"));
            }
            let value = self.value()?;
            match entries.iter_mut().find(|(name, _)| *name == key) {
                Some(entry) => entry.1 = value,
                None => entries.push((key, value)),
            }
            match self.after() {
                Some(Token::Close('}')) => return Ok(Json::Object(ordered(entries))),
                Some(Token::Comma) => match self.after() {
                    Some(Token::Text(next)) => key = next,
                    _ => return Err(self.fail("Property name must be a string literal")),
                },
                _ => return Err(self.fail("Expected '}'")),
            }
        }
    }
}
