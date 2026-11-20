//! How the web reads, rounds and writes a number: `Math.round`, `Number(text)`, `String(number)`,
//! `toFixed`, `toString(36)`, `>>> 0`, `toLocaleString('en-US')` and `Intl.NumberFormat('en-US')`
//! with its decimals, grouping and compact notation.

use super::text::trim;

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
/// value, a half going up (the core's), and as `String` writes it from 1e21 on.
pub fn to_fixed(value: f64, digits: usize) -> String {
    if !value.is_finite() || value.abs() >= 1e21 {
        return number_to_string(value);
    }
    sokkel_egui::numbers::to_fixed(value, digits)
}

/// A number rounded to `digits` places and read back, as `Number(value.toFixed(digits))`.
pub fn fixed(value: f64, digits: usize) -> f64 {
    number_of_text(&to_fixed(value, digits))
}

/// A number as `toLocaleString('en-US')` writes it, and as `Intl.NumberFormat('en-US', { notation:
/// 'compact', maximumFractionDigits: 1 })` writes it: the core's.
pub use sokkel_egui::numbers::{compact_number, locale_number};

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

/// A number as `ToUint32` makes it (`value >>> 0`): whole, then wrapped into 32 bits.
pub fn to_uint32(value: f64) -> u32 {
    if !value.is_finite() {
        return 0;
    }
    let whole = value.trunc();
    whole.rem_euclid(4_294_967_296.0) as u32
}
