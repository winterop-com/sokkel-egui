//! Times said in words, after sokkel-ui's `RelativeTime`.

const MINUTE: f64 = 60_000.0;
const HOUR: f64 = 60.0 * MINUTE;
const DAY: f64 = 24.0 * HOUR;

/// A time `distance` milliseconds from now (negative for the past) said in words, as sokkel-ui's
/// `relativeTime` says it in English by default: the largest whole unit the distance holds ("48
/// minutes ago", "in 2 hours"), "now" within a minute, and one day, week, month or year said as
/// "yesterday", "last week", "tomorrow", "next month".
pub fn relative_time(distance: f64) -> String {
    if distance.abs() < MINUTE {
        return "now".into();
    }
    let units = [
        ("year", 365.0 * DAY),
        ("month", 30.0 * DAY),
        ("week", 7.0 * DAY),
        ("day", DAY),
        ("hour", HOUR),
        ("minute", MINUTE),
    ];
    for (unit, size) in units {
        let count = (distance / size).trunc() as i64;
        if count == 0 {
            continue;
        }
        let past = count < 0;
        let n = count.abs();
        return match (unit, n, past) {
            ("day", 1, true) => "yesterday".into(),
            ("day", 1, false) => "tomorrow".into(),
            (_, 1, true) if unit != "hour" && unit != "minute" => format!("last {unit}"),
            (_, 1, false) if unit != "hour" && unit != "minute" => format!("next {unit}"),
            _ => {
                let plural = if n == 1 { "" } else { "s" };
                if past {
                    format!("{n} {unit}{plural} ago")
                } else {
                    format!("in {n} {unit}{plural}")
                }
            }
        };
    }
    "now".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_read_as_the_web_says_them() {
        assert_eq!(relative_time(-48.0 * MINUTE), "48 minutes ago");
        assert_eq!(relative_time(-30_000.0), "now");
        assert_eq!(relative_time(-1.2 * DAY), "yesterday");
        assert_eq!(relative_time(-3.2 * DAY), "3 days ago");
        assert_eq!(relative_time(-9.0 * DAY), "last week");
        assert_eq!(relative_time(2.5 * HOUR), "in 2 hours");
        assert_eq!(relative_time(-HOUR), "1 hour ago");
    }
}
