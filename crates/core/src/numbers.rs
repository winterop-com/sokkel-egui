//! How numbers are written in `en-US`, as the web writes them in sokkel-ui's parts: grouped by
//! commas (`toLocaleString`), short (`Intl.NumberFormat` with `notation: 'compact'`: `48.2K`) and to
//! a few places (`toFixed`), each rounded from the number's exact or shortest decimal as the browser
//! rounds it, so a count reads the same here as on the web.

/// The number as `String(number)` writes it, for what is not finite.
fn not_finite(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_owned()
    } else if value > 0.0 {
        "Infinity".to_owned()
    } else {
        "-Infinity".to_owned()
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
        return if value.is_finite() {
            format!("{value}")
        } else {
            not_finite(value)
        };
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
pub fn compact_number(value: f64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_are_written_as_en_us_writes_them() {
        assert_eq!(compact_number(48_201.0), "48.2K");
        assert_eq!(compact_number(1_249_999.0), "1.2M");
        assert_eq!(compact_number(999_950.0), "1M");
        assert_eq!(compact_number(950.0), "950");
        assert_eq!(locale_number(1_234_567.891_2), "1,234,567.891");
        assert_eq!(to_fixed(1.25, 1), "1.3");
        assert_eq!(to_fixed(0.35, 1), "0.3");
    }
}
