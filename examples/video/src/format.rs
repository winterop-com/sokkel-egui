//! How the site writes its numbers and ages, as the web's `format.ts` writes them.

use crate::js::{compact, locale_number, number_to_string, round, to_fixed};

/// A count written short: `52.4K`, `1.2M`.
pub fn short(count: f64) -> String {
    compact(count)
}

/// A count and its noun: `1 video`, `1,204 videos`; `many` is `one` with an `s` unless given.
pub fn plural(count: f64, one: &str, many: Option<&str>) -> String {
    let noun = if count == 1.0 {
        one.to_owned()
    } else {
        many.map_or_else(|| format!("{one}s"), str::to_owned)
    };
    format!("{} {noun}", locale_number(count))
}

/// How many watched it, written short: `52.4K views`.
pub fn views_of(count: f64) -> String {
    format!("{} {}", short(count), if count == 1.0 { "view" } else { "views" })
}

/// How long ago something was put up, from the days since: `3 days ago`, `2 months ago`.
pub fn days_ago(days: f64) -> String {
    if days < 1.0 {
        return "today".to_owned();
    }
    if days < 2.0 {
        return "yesterday".to_owned();
    }
    if days < 14.0 {
        return format!("{} days ago", number_to_string(days.floor()));
    }
    if days < 60.0 {
        return format!("{} weeks ago", number_to_string((days / 7.0).floor()));
    }
    if days < 365.0 {
        return format!("{} months ago", number_to_string((days / 30.0).floor()));
    }
    let years = (days / 365.0).floor();
    if years == 1.0 {
        "1 year ago".to_owned()
    } else {
        format!("{} years ago", number_to_string(years))
    }
}

/// A size in bytes, written short: `4.8 MB`.
pub fn bytes_of(bytes: f64) -> String {
    if bytes < 1024.0 {
        return format!("{} B", number_to_string(bytes));
    }
    if bytes < 1024.0 * 1024.0 {
        return format!("{} kB", to_fixed(bytes / 1024.0, 0));
    }
    format!("{} MB", to_fixed(bytes / (1024.0 * 1024.0), 1))
}

/// A bitrate, written short: `850 kbps`, `1.6 Mbps`.
pub fn bitrate_of(bits: f64) -> String {
    if bits >= 1_000_000.0 {
        format!("{} Mbps", to_fixed(bits / 1_000_000.0, 1))
    } else {
        format!("{} kbps", number_to_string(round(bits / 1000.0)))
    }
}

/// A second of a video as a site writes it in a link: `1:05`.
pub fn clock_of(seconds: f64) -> String {
    let whole = seconds.floor();
    format!(
        "{}:{:0>2}",
        number_to_string((whole / 60.0).floor()),
        number_to_string(whole % 60.0)
    )
}
