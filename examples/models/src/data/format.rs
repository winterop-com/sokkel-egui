//! How the app writes sizes, counts, contexts and times, as the web's `data/format.ts` writes them.

use crate::js::{intl_date as date_medium, locale_number as locale, number_to_string, round, to_fixed, utc_parts};

pub const GB: f64 = 1_000_000_000.0;
pub const MB: f64 = 1_000_000.0;

/// Bytes in decimal units, as disks are sold: `4.37 GB`, `512 MB`.
pub fn bytes(value: f64) -> String {
    if value >= 1000.0 * GB {
        return format!("{} TB", to_fixed(value / (1000.0 * GB), 2));
    }
    if value >= GB {
        let digits = if value >= 100.0 * GB {
            0
        } else if value >= 10.0 * GB {
            1
        } else {
            2
        };
        return format!("{} GB", to_fixed(value / GB, digits));
    }
    if value >= MB {
        let digits = if value >= 100.0 * MB { 0 } else { 1 };
        return format!("{} MB", to_fixed(value / MB, digits));
    }
    if value >= 1000.0 {
        return format!("{} KB", to_fixed(value / 1000.0, 0));
    }
    format!("{} B", number_to_string(value))
}

/// A rate of bytes a second: `84.2 MB/s`.
pub fn speed(value: f64) -> String {
    format!("{}/s", bytes(value))
}

/// Tokens of context, as the cards say: `128K`, `8K`.
pub fn context(tokens: f64) -> String {
    if tokens >= 1024.0 {
        format!("{}K", number_to_string(round(tokens / 1024.0)))
    } else {
        number_to_string(tokens)
    }
}

/// Billions of parameters: `7B`, `1.5B`, `0.5B`.
pub fn params(billions: f64) -> String {
    let said = if billions >= 10.0 {
        number_to_string(round(billions))
    } else {
        number_to_string(crate::js::fixed(billions, 1))
    };
    format!("{said}B")
}

/// A count with its noun: `1 model`, `12 models`.
pub fn counted(count: f64, noun: &str) -> String {
    counted_as(count, noun, &format!("{noun}s"))
}

/// A count with its noun and the noun's own plural.
pub fn counted_as(count: f64, noun: &str, plural: &str) -> String {
    format!("{} {}", locale(count), if count == 1.0 { noun } else { plural })
}

/// Seconds left as a person says them: `12 s`, `3 min 20 s`, `1 h 4 min`.
pub fn remaining(seconds: f64) -> String {
    if !seconds.is_finite() {
        return "unknown".to_owned();
    }
    let whole = round(seconds).max(0.0) as i64;
    if whole < 60 {
        return format!("{whole} s");
    }
    let minutes = whole / 60;
    if minutes < 60 {
        return format!("{minutes} min {} s", whole % 60);
    }
    format!("{} h {} min", minutes / 60, minutes % 60)
}

/// A day as the tables write it: `Nov 3, 2026`.
pub fn day(time: i64) -> String {
    date_medium(time)
}

/// A time of day: `14:05`.
pub fn clock(time: i64) -> String {
    let (_, _, _, hour, minute, _) = utc_parts(time);
    format!("{hour:02}:{minute:02}")
}

/// A time of day with seconds, for the runner's log: `14:05:12`.
pub fn clock_seconds(time: i64) -> String {
    let (_, _, _, hour, minute, second) = utc_parts(time);
    format!("{hour:02}:{minute:02}:{second:02}")
}

/// How long something has run: `3 min`, `2 h 10 min`, `4 days`.
pub fn uptime(ms: f64) -> String {
    let minutes = (ms / 60_000.0).floor();
    if minutes < 1.0 {
        return "under a minute".to_owned();
    }
    if minutes < 60.0 {
        return format!("{} min", number_to_string(minutes));
    }
    let hours = (minutes / 60.0).floor();
    if hours < 24.0 {
        return format!("{} h {} min", number_to_string(hours), number_to_string(minutes % 60.0));
    }
    counted((hours / 24.0).floor(), "day")
}
