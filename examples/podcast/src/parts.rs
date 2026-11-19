//! The words several pages say, as the web's `parts.tsx` writes them: when something came out, a
//! length of time, and a size.

use crate::backend::{DAY, civil};
use crate::js;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// When something came out: today, yesterday, days or weeks ago, or its date.
pub fn ago(at: i64, now: i64) -> String {
    let days = (now - at).div_euclid(DAY);
    if days <= 0 {
        return "Today".to_owned();
    }
    if days == 1 {
        return "Yesterday".to_owned();
    }
    if days < 14 {
        return format!("{days} days ago");
    }
    if days < 60 {
        return format!("{} weeks ago", days / 7);
    }
    let (_, month, day) = civil(at);
    format!("{} {day}", MONTHS[(month - 1) as usize])
}

/// A length of time in words: "45 sec", "12 min", "3 h 20 min".
pub fn spoken(seconds: f64) -> String {
    if seconds < 60.0 {
        return format!("{} sec", js::number_to_string(js::round(seconds)));
    }
    let minutes = js::round(seconds / 60.0);
    if minutes < 60.0 {
        return format!("{} min", js::number_to_string(minutes));
    }
    let hours = (minutes / 60.0).floor();
    let rest = minutes % 60.0;
    if rest == 0.0 {
        format!("{} h", js::number_to_string(hours))
    } else {
        format!("{} h {} min", js::number_to_string(hours), js::number_to_string(rest))
    }
}

/// A size in megabytes, to a tenth.
pub fn megabytes(size: f64) -> String {
    format!("{} MB", js::to_fixed(size, 1))
}
