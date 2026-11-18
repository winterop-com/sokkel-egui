//! What the web's modules lean on the browser for, carried over so the studio gives the same
//! answers: how a number is written (`String`, `toFixed`, `toLocaleString`), read (`Number`) and
//! rounded (`Math.round`), how texts are ordered (`localeCompare`), counted and cut (UTF-16 units),
//! how a moment is written (`toISOString`), and how `replaceAll` fills in its replacement.
//!
//! The collation is the one notes carries (`examples/notes/src/model.rs`), held there against bun.

use std::cmp::Ordering;

/// A minute in milliseconds.
pub const MINUTE: i64 = 60_000;
/// An hour in milliseconds.
pub const HOUR: i64 = 60 * MINUTE;
/// A day in milliseconds.
pub const DAY: i64 = 24 * HOUR;

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

/// How long a text is as the web counts it, in UTF-16 units.
pub fn length(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The byte offset of the `units`-th UTF-16 unit of a text, held within it; a unit inside a
/// character that takes two counts as the start of that character.
pub fn byte_at(text: &str, units: usize) -> usize {
    let mut counted = 0;
    for (at, c) in text.char_indices() {
        if counted >= units {
            return at;
        }
        counted += c.len_utf16();
        if counted > units {
            return at;
        }
    }
    text.len()
}

/// Where a `slice(start, end)` of a list `len` long starts and ends, as the web reads its numbers:
/// a fraction cut toward zero, NaN as 0, a negative one from the end.
pub fn slice_range(len: usize, start: f64, end: Option<f64>) -> (usize, usize) {
    let place = |at: f64| -> usize {
        let at = if at.is_nan() { 0.0 } else { at.trunc() };
        let len = len as f64;
        let at = if at < 0.0 { (len + at).max(0.0) } else { at.min(len) };
        at as usize
    };
    let from = place(start);
    let to = end.map_or(len, place);
    (from, to.max(from))
}

/// A part of a text by UTF-16 units, as `slice` cuts it.
pub fn slice(text: &str, start: f64, end: Option<f64>) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let (from, to) = slice_range(units.len(), start, end);
    String::from_utf16_lossy(&units[from..to])
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

/// A text read as a number, as `Number(text)` reads it: blank is 0, `0x`, `0o` and `0b` read in
/// their base, anything else not a number is NaN.
pub fn to_number(text: &str) -> f64 {
    let text = trim(text);
    if text.is_empty() {
        return 0.0;
    }
    for (prefix, radix) in [("0x", 16), ("0X", 16), ("0o", 8), ("0O", 8), ("0b", 2), ("0B", 2)] {
        if let Some(digits) = text.strip_prefix(prefix) {
            if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
                return f64::NAN;
            }
            return digits.chars().fold(0.0, |total, c| {
                total * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
            });
        }
    }
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    if unsigned == "Infinity" {
        return if text.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    if !decimal_literal(unsigned) {
        return f64::NAN;
    }
    text.parse().unwrap_or(f64::NAN)
}

/// Whether a text is a decimal number without its sign: digits with a point among or before them,
/// and an exponent.
fn decimal_literal(text: &str) -> bool {
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
    if whole + fraction == 0 {
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

/// A number rounded as `Math.round` rounds it: to the nearest whole number, a half up.
pub fn round(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let floor = value.floor();
    if value - floor >= 0.5 { floor + 1.0 } else { floor }
}

/// The digits of a number's exact decimal value, rounded at `digits` places after the point with a
/// half going up, as `toFixed` and the locale's figures round.
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

/// Whole digits with a comma between each three, as en-US groups them.
fn grouped(whole: &str) -> String {
    let mut out = String::new();
    for (at, c) in whole.chars().enumerate() {
        if at > 0 && (whole.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A number as `toLocaleString('en-US')` writes it: grouped by thousands, at most three places
/// after the point, a half rounded away from zero.
pub fn locale_number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "\u{221e}" } else { "-\u{221e}" }.to_owned();
    }
    // The shortest digits that read back as the number, rounded as decimals.
    let written = format!("{:e}", value.abs());
    let (mantissa, exponent) = written.split_once('e').unwrap_or((&written, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let point = exponent.parse::<i64>().unwrap_or(0) + 1;
    let (whole, fraction) = if point <= 0 {
        ("0".to_owned(), format!("{}{digits}", "0".repeat((-point) as usize)))
    } else if point as usize >= digits.len() {
        (
            format!("{digits}{}", "0".repeat(point as usize - digits.len())),
            String::new(),
        )
    } else {
        (digits[..point as usize].to_owned(), digits[point as usize..].to_owned())
    };
    let (whole, fraction) = round_decimal(&whole, &fraction, 3);
    let fraction = fraction.trim_end_matches('0');
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    let sign = if value < 0.0 { "-" } else { "" };
    if fraction.is_empty() {
        format!("{sign}{}", grouped(whole))
    } else {
        format!("{sign}{}.{fraction}", grouped(whole))
    }
}

/// A number written in base 36, as `Number.prototype.toString(36)` writes a whole one.
pub fn base36(value: i64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_owned();
    }
    let mut left = value.unsigned_abs();
    let mut out = Vec::new();
    while left > 0 {
        out.push(DIGITS[(left % 36) as usize]);
        left /= 36;
    }
    if value < 0 {
        out.push(b'-');
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// The year, month (1 to 12) and day of a day number since 1970.
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

/// The day number since 1970 of a date, the month 1 to 12.
pub fn day_from(year: i64, month: i64, date: i64) -> i64 {
    // Howard Hinnant's days_from_civil
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + date - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The moment `Date.UTC(year, month, date, hours, minutes, seconds)` gives, the month counted from 0.
pub fn utc(year: i64, month: i64, date: i64, hours: i64, minutes: i64, seconds: i64) -> i64 {
    day_from(year, month + 1, date) * DAY + hours * HOUR + minutes * MINUTE + seconds * 1000
}

/// A moment as `toISOString` writes it: 2026-11-02T02:00:14.000Z.
pub fn iso(at: i64) -> String {
    let (year, month, date) = civil(at.div_euclid(DAY));
    let within = at.rem_euclid(DAY);
    format!(
        "{year:04}-{month:02}-{date:02}T{:02}:{:02}:{:02}.{:03}Z",
        within / HOUR,
        within % HOUR / MINUTE,
        within % MINUTE / 1000,
        within % 1000
    )
}

/// Every `search` in a text replaced, as `replaceAll` with a text replaces it: `$$`, `$&`, `` $` ``
/// and `$'` in the replacement stand for a dollar, the match, and what comes before and after it.
pub fn replace_all(text: &str, search: &str, replacement: &str) -> String {
    let mut found: Vec<usize> = Vec::new();
    if search.is_empty() {
        found.extend(text.char_indices().map(|(at, _)| at));
        found.push(text.len());
    } else {
        let mut from = 0;
        while let Some(at) = text[from..].find(search) {
            found.push(from + at);
            from += at + search.len();
        }
    }
    let mut out = String::new();
    let mut last = 0;
    for at in found {
        out.push_str(&text[last..at]);
        let end = at + search.len();
        let mut chars = replacement.char_indices().peekable();
        while let Some((_, c)) = chars.next() {
            if c != '$' {
                out.push(c);
                continue;
            }
            match chars.peek().map(|(_, next)| *next) {
                Some('$') => {
                    out.push('$');
                    chars.next();
                }
                Some('&') => {
                    out.push_str(search);
                    chars.next();
                }
                Some('`') => {
                    out.push_str(&text[..at]);
                    chars.next();
                }
                Some('\'') => {
                    out.push_str(&text[end..]);
                    chars.next();
                }
                _ => out.push('$'),
            }
        }
        last = end;
    }
    out.push_str(&text[last..]);
    out
}

/// The marks and spaces the collator orders before digits and letters, in its order; the
/// characters of one entry weigh the same.
const MARKS: &[&str] = &[
    "\t", "\n", "\u{b}", "\u{c}", "\r", "\u{85}", " ", "\u{a0}", "_", "-", "\u{2013}", "\u{2014}", ",", ";", ":", "!",
    "\u{a1}", "?", "\u{bf}", ".", "\u{2026}", "\u{b7}", "'", "\u{2018}", "\u{2019}", "\"", "\u{201c}", "\u{201d}",
    "\u{ab}", "\u{bb}", "(", ")", "[", "]", "{", "}", "\u{a7}", "\u{b6}", "@", "*", "/", "\\", "&", "#", "%",
    "\u{2022}", "`", "\u{b4}", "^", "\u{af}", "\u{a8}", "\u{b8}", "\u{b0}", "\u{a9}", "\u{ae}", "\u{2190}", "\u{2192}",
    "+", "\u{b1}", "\u{f7}", "\u{d7}", "<", "=", ">", "\u{ac}", "|", "\u{a6}", "~", "\u{2713}", "\u{a4}", "\u{a2}",
    "$", "\u{a3}", "\u{a5}", "\u{20ac}", "\u{2122}",
];

/// The Latin letters with a mark the collator reads as their plain letter first, each followed by
/// that letter.
const FOLDS: &str = "ÀaÁaÂaÃaÄaÅaÇcÈeÉeÊeËeÌiÍiÎiÏiÐdÑnÒoÓoÔoÕoÖoØoÙuÚuÛuÜuÝyàaáaâaãaäaåaçcèeéeêeëeìiíiîiïiðdñnòoóoôoõoöoøoùuúuûuüuýyÿyĀaāaĂaăaĄaąaĆcćcĈcĉcĊcċcČcčcĎdďdĐdđdĒeēeĔeĕeĖeėeĘeęeĚeěeĜgĝgĞgğgĠgġgĢgģgĤhĥhĦhħhĨiĩiĪiīiĬiĭiĮiįiİiĴjĵjĶkķkĹlĺlĻlļlĽlľlĿlŀlŁlłlŃnńnŅnņnŇnňnŌoōoŎoŏoŐoőoŔrŕrŖrŗrŘrřrŚsśsŜsŝsŞsşsŠsšsŢtţtŤtťtŨuũuŪuūuŬuŭuŮuůuŰuűuŲuųuŴwŵwŶyŷyŸyŹzźzŻzżzŽzžzſsƠoơoƯuưuǍaǎaǏiǐiǑoǒoǓuǔuǕuǖuǗuǘuǙuǚuǛuǜuǞaǟaǠaǡaǦgǧgǨkǩkǪoǫoǬoǭoǰjǴgǵgǸnǹnǺaǻaǾoǿoȀaȁaȂaȃaȄeȅeȆeȇeȈiȉiȊiȋiȌoȍoȎoȏoȐrȑrȒrȓrȔuȕuȖuȗuȘsșsȚtțtȞhȟhȦaȧaȨeȩeȪoȫoȬoȭoȮoȯoȰoȱoȲyȳy";

/// The plain letter a Latin letter with a mark is read as first, if it is one.
fn fold(c: char) -> Option<char> {
    let mut pairs = FOLDS.chars();
    while let (Some(marked), Some(plain)) = (pairs.next(), pairs.next()) {
        if marked == c {
            return Some(plain);
        }
    }
    None
}

/// One character the collator weighs: its group (marks, other symbols, digits, letters) and weight
/// within it, whether it carries a mark over its plain form, and its case.
#[derive(Debug)]
struct Unit {
    primary: (u8, u32),
    accent: bool,
    upper: bool,
}

fn units(text: &str) -> Vec<Unit> {
    let mut out: Vec<Unit> = Vec::new();
    for c in text.chars() {
        if ('\u{300}'..='\u{36f}').contains(&c) {
            // A combining mark weighs nothing alone and marks the character before it.
            if let Some(last) = out.last_mut() {
                last.accent = true;
            }
            continue;
        }
        if c == '\u{ad}' || (c.is_control() && !matches!(c, '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\u{85}')) {
            continue;
        }
        let plain = |primary| Unit {
            primary,
            accent: false,
            upper: false,
        };
        if c.is_ascii_digit() {
            out.push(plain((3, c as u32)));
            continue;
        }
        let mut buffer = [0; 4];
        let written: &str = c.encode_utf8(&mut buffer);
        if let Some(place) = MARKS.iter().position(|mark| mark.contains(written)) {
            out.push(plain((1, place as u32)));
            continue;
        }
        if c.is_alphabetic() {
            let (weight, accent) = match fold(c) {
                Some(plain) => (plain as u32, true),
                None => {
                    let lower = c.to_lowercase().next().unwrap_or(c);
                    if lower.is_ascii_lowercase() {
                        (lower as u32, false)
                    } else {
                        (0x10000 + lower as u32, false)
                    }
                }
            };
            out.push(Unit {
                primary: (4, weight),
                accent,
                upper: c.is_uppercase(),
            });
            continue;
        }
        out.push(plain((2, c as u32)));
    }
    out
}

/// Two texts in the order `a.localeCompare(b)` gives them, as ICU's root order has it: spaces and
/// marks, then digits, then letters; a letter's mark after it, and lower case before upper.
pub fn locale_compare(a: &str, b: &str) -> Ordering {
    let (left, right) = (units(a), units(b));
    for (one, other) in left.iter().zip(&right) {
        let order = one.primary.cmp(&other.primary);
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len()
        .cmp(&right.len())
        .then_with(|| {
            let accents = |list: &[Unit]| list.iter().map(|unit| unit.accent).collect::<Vec<_>>();
            accents(&left).cmp(&accents(&right))
        })
        .then_with(|| {
            let cases = |list: &[Unit]| list.iter().map(|unit| unit.upper).collect::<Vec<_>>();
            cases(&left).cmp(&cases(&right))
        })
}

/// A list sorted stably, as the web's `Array.prototype.sort` sorts it: a merge of runs doubling in
/// width, each taking from the right only when `order` puts it first, so an order that is not
/// consistent (a NaN read as a tie) never stops it.
pub fn sort_by<T: Clone>(list: &mut [T], mut order: impl FnMut(&T, &T) -> Ordering) {
    let count = list.len();
    let mut source: Vec<T> = list.to_vec();
    let mut target: Vec<T> = list.to_vec();
    let mut width = 1;
    while width < count {
        let mut start = 0;
        while start < count {
            let (mut left, left_end) = (start, (start + width).min(count));
            let (mut right, right_end) = (left_end, (left_end + width).min(count));
            for slot in target.iter_mut().take(right_end).skip(start) {
                if right < right_end && (left >= left_end || order(&source[right], &source[left]) == Ordering::Less) {
                    *slot = source[right].clone();
                    right += 1;
                } else {
                    *slot = source[left].clone();
                    left += 1;
                }
            }
            start += 2 * width;
        }
        std::mem::swap(&mut source, &mut target);
        width *= 2;
    }
    list.clone_from_slice(&source);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_as_the_web_writes_them() {
        assert_eq!(number_to_string(160.0), "160");
        assert_eq!(number_to_string(0.1), "0.1");
        assert_eq!(number_to_string(1e21), "1e+21");
        assert_eq!(number_to_string(123_456_789_012_345_680_000.0), "123456789012345680000");
        assert_eq!(number_to_string(1.5e-7), "1.5e-7");
        assert_eq!(number_to_string(0.000_001), "0.000001");
        assert_eq!(number_to_string(-2.5), "-2.5");
        assert_eq!(to_fixed(1.25, 1), "1.3");
        assert_eq!(to_fixed(1.35, 1), "1.4");
        assert_eq!(to_fixed(0.05, 1), "0.1");
        assert_eq!(to_fixed(59.96, 1), "60.0");
        assert_eq!(locale_number(1234.0), "1,234");
        assert_eq!(locale_number(1_234_567.891_5), "1,234,567.892");
        assert_eq!(locale_number(0.5), "0.5");
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(0.499_999_999_999_999_94), 0.0);
        assert_eq!(to_number(" 12 "), 12.0);
        assert_eq!(to_number("0x1F"), 31.0);
        assert!(to_number("1.2.3").is_nan());
        assert_eq!(to_number("1."), 1.0);
        assert_eq!(to_number(""), 0.0);
        assert!(to_number("inf").is_nan());
        assert_eq!(base36(1_794_992_401_000), "mwlw4ors");
        assert_eq!(iso(utc(2026, 9, 20, 0, 0, 0)), "2026-10-20T00:00:00.000Z");
        assert_eq!(replace_all("a-b-c", "-", "[$&$$]"), "a[-$]b[-$]c");
        assert_eq!(replace_all("ab", "", "-"), "-a-b-");
        assert_eq!(slice("abcdef", 1.0, Some(-1.0)), "bcde");
    }
}
