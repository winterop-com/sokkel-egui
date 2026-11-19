//! What the web's modules lean on the browser for, carried over from the podcast example
//! (`examples/podcast/src/js.rs`) so the site gives the same answers: how a number is rounded
//! (`Math.round`) and written (`String`, `toFixed`, `toLocaleString`, a compact
//! `Intl.NumberFormat`), what `\s` and `trim` take as space, and how a moment is written in
//! `en-US`.

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

/// The moment now, in milliseconds since 1970, as `Date.now` gives it.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as i64)
}

/// The words of a text as the web splits them: lowercased, on runs of `\s`, the empty ones left out.
pub fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(is_space)
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
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

/// A moment as `new Date(ms).toLocaleString('en-US')` writes it in UTC: `11/19/2026, 9:05:00 AM`, the space before the half of the day a plain one as bun writes it.
pub fn locale_date_time(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let of_day = ms.rem_euclid(86_400_000) / 1000;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let (hour, minute, second) = (of_day / 3600, of_day / 60 % 60, of_day % 60);
    let twelve = if hour % 12 == 0 { 12 } else { hour % 12 };
    let half = if hour < 12 { "AM" } else { "PM" };
    format!("{month}/{day}/{year}, {twelve}:{minute:02}:{second:02} {half}")
}

/// A text as `encodeURIComponent` writes it: every byte of its UTF-8 but the unreserved ones as
/// `%XX`.
pub fn encode_uri_component(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_as_the_web_writes_them() {
        assert_eq!(number_to_string(0.5), "0.5");
        assert_eq!(number_to_string(15.5), "15.5");
        assert_eq!(number_to_string(-2.0), "-2");
        assert_eq!(to_fixed(1.25, 1), "1.3");
        assert_eq!(to_fixed(0.05, 1), "0.1");
        assert_eq!(to_fixed(-0.04, 1), "-0.0");
        assert_eq!(round(-0.5), 0.0);
        assert_eq!(round(2.5), 3.0);
    }
}
