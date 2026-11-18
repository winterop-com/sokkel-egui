//! What the web's modules lean on the browser for, carried over from the cut example
//! (`examples/cut/src/js.rs`) so music gives the same answers: how a number is rounded
//! (`Math.round`), written (`String`, `toFixed`, base 36) and read (`Number`), what `\s` and `trim`
//! take as space, and the 32-bit sums `>>> 0` and `Math.imul` make.

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

/// A number as `ToUint32` makes it (`value >>> 0`): whole, then wrapped into 32 bits.
pub fn to_uint32(value: f64) -> u32 {
    if !value.is_finite() {
        return 0;
    }
    let whole = value.trunc();
    whole.rem_euclid(4_294_967_296.0) as u32
}

/// A text read as `Number(text)` reads it: spaces at the ends cut, nothing as 0, `0x`, `0o` and
/// `0b` numbers, `Infinity`, and otherwise a decimal literal or NaN.
pub fn number(text: &str) -> f64 {
    let text = text.trim_matches(|c: char| is_space(c));
    if text.is_empty() {
        return 0.0;
    }
    let radix = |digits: &str, base: u32| -> f64 {
        if digits.is_empty() {
            return f64::NAN;
        }
        let mut value = 0.0_f64;
        for c in digits.chars() {
            match c.to_digit(base) {
                Some(digit) => value = value * f64::from(base) + f64::from(digit),
                None => return f64::NAN,
            }
        }
        value
    };
    let lower = text.get(..2).map(str::to_ascii_lowercase);
    match lower.as_deref() {
        Some("0x") => return radix(&text[2..], 16),
        Some("0o") => return radix(&text[2..], 8),
        Some("0b") => return radix(&text[2..], 2),
        _ => {}
    }
    let (sign, body) = match text.as_bytes()[0] {
        b'+' => (1.0, &text[1..]),
        b'-' => (-1.0, &text[1..]),
        _ => (1.0, text),
    };
    if body == "Infinity" {
        return sign * f64::INFINITY;
    }
    // A decimal literal: digits, a point, digits (one side at least), an exponent.
    let bytes = body.as_bytes();
    let mut at = 0;
    let digits = |at: &mut usize| {
        let from = *at;
        while *at < bytes.len() && bytes[*at].is_ascii_digit() {
            *at += 1;
        }
        *at - from
    };
    let mut seen = digits(&mut at);
    if at < bytes.len() && bytes[at] == b'.' {
        at += 1;
        seen += digits(&mut at);
    }
    if seen == 0 {
        return f64::NAN;
    }
    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
        at += 1;
        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
            at += 1;
        }
        if digits(&mut at) == 0 {
            return f64::NAN;
        }
    }
    if at != bytes.len() {
        return f64::NAN;
    }
    body.parse::<f64>().map_or(f64::NAN, |value| sign * value)
}

/// The moment now, in milliseconds since 1970, as `Date.now` gives it.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as i64)
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
        assert_eq!(base36(1_794_992_400_000), "mwlw4o00");
        assert_eq!(round(-0.5), 0.0);
        assert_eq!(round(2.5), 3.0);
        assert_eq!(number(" +250 "), 250.0);
        assert_eq!(number(""), 0.0);
        assert_eq!(number("0x10"), 16.0);
        assert!(number("inf").is_nan());
        assert!(number("1.2.3").is_nan());
        assert_eq!(number(".5"), 0.5);
        assert_eq!(to_uint32(-1.0), 4_294_967_295);
    }
}
