//! What the web's modules lean on the browser for, so the models give the same answers: how a
//! number is rounded (`Math.round`), read (`Number`) and written (`String`, `toFixed`,
//! `toLocaleString`, base 36), what `\s`, `\b` and `trim` take as space and as a word, a text's
//! length and slices in UTF-16 units, how `en-US` collates text (`localeCompare`), and the
//! calendar's days and times in UTC as `Date` writes them.
//!
//! Carried from the dhis2 example (`examples/dhis2/src/js.rs`), with what the models lean on
//! besides.

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

// --- What the models lean on besides -------------------------------------------------------------

/// A number with `digits` places after the point, as `toFixed` writes it.
pub fn to_fixed(value: f64, digits: usize) -> String {
    sokkel_egui::numbers::to_fixed(value, digits)
}

/// A number rounded to `digits` places and read back, as `Number(value.toFixed(digits))`.
pub fn fixed(value: f64, digits: usize) -> f64 {
    number_of_text(&to_fixed(value, digits))
}

/// A number as `toLocaleString('en-US')` writes it: `2,184`.
pub fn locale(value: f64) -> String {
    sokkel_egui::numbers::locale_number(value)
}

/// A text's length in UTF-16 units, as `length` counts it.
pub fn len16(text: &str) -> usize {
    text.encode_utf16().count()
}

/// A text's first `count` UTF-16 units, as `slice(0, count)` cuts it.
pub fn slice16(text: &str, count: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().take(count).collect();
    String::from_utf16_lossy(&units)
}

/// Whether a character is one the web's `\w` matches, and so one side of a `\b`.
pub fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Whether `word` stands in `text` with a `\b` on each side, as `/\bword\b/` finds it.
pub fn has_word(text: &str, word: &str) -> bool {
    let first = word.chars().next().is_some_and(is_word);
    let last = word.chars().last().is_some_and(is_word);
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        let start = before.is_some_and(is_word) != first;
        let end = after.is_some_and(is_word) != last;
        start && end
    })
}

/// A text split at each run of spaces, as `split(/\s+/)` splits it: an empty piece at an end that
/// starts or ends with spaces.
pub fn split_spaces(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if is_space(c) {
            let mut end = at + c.len_utf8();
            while let Some(&(next, d)) = chars.peek() {
                if !is_space(d) {
                    break;
                }
                end = next + d.len_utf8();
                chars.next();
            }
            out.push(&text[start..at]);
            start = end;
        }
    }
    out.push(&text[start..]);
    out
}

/// The words of a text: `split(/\s+/).filter(Boolean)`.
pub fn words(text: &str) -> Vec<&str> {
    split_spaces(text).into_iter().filter(|one| !one.is_empty()).collect()
}

/// The keys `Object.keys` gives of a parsed JSON value: an array's indices, an object's keys with
/// those that read as indices first and in order, nothing for anything else.
pub fn object_keys(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(items) => (0..items.len()).map(|index| index.to_string()).collect(),
        serde_json::Value::Object(map) => {
            let index = |key: &str| -> Option<u32> {
                let number: u32 = key.parse().ok()?;
                (number < u32::MAX && number.to_string() == key).then_some(number)
            };
            let mut numbered: Vec<(u32, String)> = map
                .keys()
                .filter_map(|key| index(key).map(|number| (number, key.clone())))
                .collect();
            numbered.sort();
            let mut keys: Vec<String> = numbered.into_iter().map(|(_, key)| key).collect();
            keys.extend(map.keys().filter(|key| index(key).is_none()).cloned());
            keys
        }
        _ => Vec::new(),
    }
}

/// The year, month from 1, day, hours, minutes and seconds of a moment in UTC.
pub fn utc_parts(time: i64) -> (i64, i64, i64, i64, i64, i64) {
    let days = time.div_euclid(86_400_000);
    let rest = time.rem_euclid(86_400_000) / 1000;
    let (year, month, day) = civil_from_days(days);
    (year, month, day, rest / 3600, rest / 60 % 60, rest % 60)
}

/// The months as `en-US` writes them short.
pub const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A day as `toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })`
/// writes it, or `dateStyle: 'medium'`: `Nov 3, 2026`.
pub fn date_medium(time: i64) -> String {
    let (year, month, day, ..) = utc_parts(time);
    format!("{} {day}, {year}", MONTHS[(month - 1) as usize])
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
