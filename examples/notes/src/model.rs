//! What the vault holds, as the web's `model.ts`: folders, the notes in them, the reader's own
//! settings, and the words days, times and names are said in.
//!
//! The web reads days and times on the local clock. Here the local zone is a fixed offset from UTC:
//! `set_local_offset` sets it, UTC until then. The few places the web leans on the browser (the
//! `\s` and `\p{L}` classes of its patterns, `toLowerCase`, `localeCompare`) are carried over as
//! helpers at the end of this file.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

use serde::{Deserialize, Serialize};

/// A minute in milliseconds.
pub const MINUTE: i64 = 60_000;
/// An hour in milliseconds.
pub const HOUR: i64 = 60 * MINUTE;
/// A day in milliseconds.
pub const DAY: i64 = 24 * HOUR;

/// How far the local zone is ahead of UTC, in milliseconds: UTC until the app sets it.
static OFFSET: AtomicI64 = AtomicI64::new(0);

/// Sets the local zone, `ms` milliseconds ahead of UTC, as the app starts.
pub fn set_local_offset(ms: i64) {
    OFFSET.store(ms, AtomicOrdering::Relaxed);
}

/// How far the local zone is ahead of UTC, in milliseconds.
pub fn local_offset() -> i64 {
    OFFSET.load(AtomicOrdering::Relaxed)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    /// The folder it sits in, or none at the top of the vault.
    pub parent: Option<String>,
    /// Its place among the folders beside it, in the reader's own order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    /// The note's name, which `[[links]]` name it by; no two notes share one, in any case.
    pub title: String,
    pub folder: Option<String>,
    /// The note as markdown.
    pub body: String,
    /// When it was made and last changed, as milliseconds since 1970.
    pub created: i64,
    pub updated: i64,
    /// The day a daily note is for, `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<String>,
    /// Its place among the notes beside it, in the reader's own order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<i64>,
}

/// The editor's text size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontSize {
    Sm,
    Md,
    Lg,
}

/// How the vault orders each folder: by name, or in the order the reader drags things into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    Name,
    Manual,
}

/// What a note shows, as sokkel-ui's `EditPreviewView`: the editor, its preview, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Edit,
    Preview,
    Both,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub id: String,
    /// The editor's text size.
    pub font_size: FontSize,
    /// Long lines fold at the editor's edge.
    pub wrap: bool,
    pub line_numbers: bool,
    /// What a note opens in: the editor, its preview, or both.
    pub default_view: View,
    /// What each open note shows now, by its id.
    pub views: BTreeMap<String, View>,
    /// The notes open as tabs, in order, and the one shown.
    pub tabs: Vec<String>,
    pub active: Option<String>,
    /// The notes opened most lately, newest first.
    pub recent: Vec<String>,
    /// Whether the panel of links and the outline shows beside a note.
    pub panel: bool,
    /// The folder daily notes go in, and the text a new one starts from.
    pub daily_folder: Option<String>,
    pub template: String,
    /// Whether the graph shows the notes with no link.
    pub orphans: bool,
    /// How the vault orders each folder; a vault kept before there was a choice has none, and reads
    /// as `Manual`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Order>,
}

/// The text a new daily note starts from.
pub const TEMPLATE: &str = "# {{date}}\n\n## Plan\n\n- [ ]\n\n## Log\n\n## Notes\n\n#journal\n";

impl Default for Prefs {
    /// The web's `DEFAULT_PREFS`.
    fn default() -> Self {
        Self {
            id: "me".to_owned(),
            font_size: FontSize::Md,
            wrap: true,
            line_numbers: false,
            default_view: View::Both,
            views: BTreeMap::new(),
            tabs: Vec::new(),
            active: None,
            recent: Vec::new(),
            panel: true,
            daily_folder: Some("daily".to_owned()),
            template: TEMPLATE.to_owned(),
            orphans: true,
            order: Some(Order::Manual),
        }
    }
}

/// How many notes the recent list keeps.
pub const RECENT: usize = 12;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// The year, month (1 to 12) and day of a day number since 1970.
fn civil(day: i64) -> (i64, i64, i64) {
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
fn day_from(year: i64, month: i64, date: i64) -> i64 {
    // Howard Hinnant's days_from_civil
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + date - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The local day number since 1970 of a moment.
fn local_day(at: i64) -> i64 {
    (at + local_offset()).div_euclid(DAY)
}

fn write_day(day: i64) -> String {
    let (year, month, date) = civil(day);
    format!("{year}-{month:02}-{date:02}")
}

/// The local day `offset` days from the day of `from`, as `YYYY-MM-DD`.
pub fn day(offset: i64, from: i64) -> String {
    write_day(local_day(from) + offset)
}

/// A `YYYY-MM-DD` day as a local date, given as its day number since 1970, as `new Date(year,
/// month - 1, date)` reads it: a month or a day past its end runs on into the next, a year below
/// 100 is in the 1900s. None where the web's date is invalid.
pub fn date_of(text: &str) -> Option<i64> {
    let mut parts = text.split('-').map(js_number);
    let year = parts.next().unwrap_or(Some(0))?;
    let month = parts.next().unwrap_or(Some(1))?;
    let date = parts.next().unwrap_or(Some(1))?;
    let year = if (0..=99).contains(&year) { year + 1900 } else { year };
    let months = year * 12 + month - 1;
    Some(day_from(months.div_euclid(12), months.rem_euclid(12) + 1, 1) + date - 1)
}

/// A part of a day as `Number` reads it, a whole number alone: blank is 0, anything else not a
/// whole number is none.
fn js_number(part: &str) -> Option<i64> {
    let part = trim(part);
    if part.is_empty() {
        return Some(0);
    }
    let digits = part.strip_prefix(['+', '-']).unwrap_or(part);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

/// A day written out for a reader, as `toLocaleDateString('en-GB', ...)` writes it: Wednesday, 18
/// November 2026. A day the web cannot read is `Invalid Date`.
pub fn long_day(text: &str) -> String {
    let Some(day) = date_of(text) else {
        return "Invalid Date".to_owned();
    };
    let (year, month, date) = civil(day);
    let weekday = (day + 4).rem_euclid(7) as usize;
    format!("{}, {date} {} {year}", WEEKDAYS[weekday], MONTHS[(month - 1) as usize])
}

/// A daily note's text from the template, with `{{date}}` and `{{long}}` filled in for the day.
pub fn from_template(template: &str, for_day: &str) -> String {
    template
        .replace("{{date}}", for_day)
        .replace("{{long}}", &long_day(for_day))
}

/// A time of day on the local clock, 14:05.
pub fn clock(at: i64) -> String {
    let within = (at + local_offset()).rem_euclid(DAY);
    format!("{:02}:{:02}", within / HOUR, within % HOUR / MINUTE)
}

/// How many words a text holds: the runs between spaces holding a letter or a digit.
pub fn word_count(text: &str) -> usize {
    text.split(is_space)
        .filter(|word| word.chars().any(|c| is_letter(c) || is_number(c)))
        .count()
}

/// A name for a note or a folder that none of `taken` has, in any case: the name, or the name and a
/// number.
pub fn free_name(name: &str, taken: &[&str]) -> String {
    let lower: HashSet<String> = taken.iter().map(|one| lowercase(one)).collect();
    if !lower.contains(&lowercase(name)) {
        return name.to_owned();
    }
    (2..)
        .map(|count| format!("{name} {count}"))
        .find(|next| !lower.contains(&lowercase(next)))
        .unwrap_or_default()
}

/// Why a name cannot be a note's, or none when it can: empty, holding a character links read, or
/// taken.
pub fn bad_title(title: &str, taken: &[&str]) -> Option<String> {
    let trimmed = trim(title);
    if trimmed.is_empty() {
        return Some("A note needs a name.".to_owned());
    }
    if trimmed.contains(['[', ']', '|', '#']) {
        return Some("A name cannot hold [ ] | or #, which links read.".to_owned());
    }
    let lower = lowercase(trimmed);
    if taken.iter().any(|one| lowercase(one) == lower) {
        return Some(format!("A note named {trimmed} is already in the vault."));
    }
    None
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

/// Whether a character is one the web's `\s` matches.
pub fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// Whether a character is a letter, as the web's `\p{L}` reads one.
pub fn is_letter(c: char) -> bool {
    c.is_alphabetic()
}

/// Whether a character is a number, as the web's `\p{N}` reads one.
pub fn is_number(c: char) -> bool {
    c.is_numeric()
}

/// A text without the spaces at its ends, as `String.prototype.trim` cuts them.
pub fn trim(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// A text without the spaces at its start, as `trimStart` cuts them.
pub fn trim_start(text: &str) -> &str {
    text.trim_start_matches(is_space)
}

/// A text without the spaces at its end, as `trimEnd` cuts them.
pub fn trim_end(text: &str) -> &str {
    text.trim_end_matches(is_space)
}

/// A text in lower case, as `toLowerCase` writes it.
pub fn lowercase(text: &str) -> String {
    text.to_lowercase()
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

/// One unit the collator weighs: a character, or with `numeric` a run of digits by its value.
#[derive(Debug)]
enum Unit {
    Char {
        /// The group (marks, other symbols, digits, letters) and the weight within it.
        primary: (u8, u32),
        /// Whether it carries a mark over its plain form.
        accent: bool,
        upper: bool,
    },
    /// A run of digits without its leading zeros.
    Number(String),
}

fn units(text: &str, numeric: bool) -> Vec<Unit> {
    let mut out: Vec<Unit> = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if ('\u{300}'..='\u{36f}').contains(&c) {
            // A combining mark weighs nothing alone and marks the character before it.
            if let Some(Unit::Char { accent, .. }) = out.last_mut() {
                *accent = true;
            }
            continue;
        }
        if c == '\u{ad}' || (c.is_control() && !matches!(c, '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\u{85}')) {
            continue;
        }
        if c.is_ascii_digit() {
            if numeric {
                let mut run = String::from(c);
                while let Some(&next) = chars.peek().filter(|next| next.is_ascii_digit()) {
                    run.push(next);
                    chars.next();
                }
                let value = run.trim_start_matches('0');
                out.push(Unit::Number(if value.is_empty() { "0" } else { value }.to_owned()));
            } else {
                out.push(Unit::Char {
                    primary: (3, c as u32),
                    accent: false,
                    upper: false,
                });
            }
            continue;
        }
        let mut buffer = [0; 4];
        let written: &str = c.encode_utf8(&mut buffer);
        if let Some(place) = MARKS.iter().position(|mark| mark.contains(written)) {
            out.push(Unit::Char {
                primary: (1, place as u32),
                accent: false,
                upper: false,
            });
            continue;
        }
        if is_letter(c) {
            let upper = c.is_uppercase();
            let folded = fold(c);
            let (weight, accent) = match folded {
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
            out.push(Unit::Char {
                primary: (4, weight),
                accent,
                upper,
            });
            continue;
        }
        out.push(Unit::Char {
            primary: (2, c as u32),
            accent: false,
            upper: false,
        });
    }
    out
}

/// Two texts in the order `localeCompare(b, 'en', { numeric })` gives them, as ICU's root order
/// has it for the characters the vault holds: spaces and marks, then digits (with `numeric` a run
/// of them by its value), then letters; a letter's mark after it, and lower case before upper.
pub fn locale_compare(a: &str, b: &str, numeric: bool) -> Ordering {
    let (left, right) = (units(a, numeric), units(b, numeric));
    let primary = |unit: &Unit| -> (u8, u32) {
        match unit {
            Unit::Char { primary, .. } => *primary,
            Unit::Number(_) => (3, 0),
        }
    };
    for (one, other) in left.iter().zip(&right) {
        let order = match (one, other) {
            (Unit::Number(x), Unit::Number(y)) => x.len().cmp(&y.len()).then_with(|| x.cmp(y)),
            _ => primary(one).cmp(&primary(other)),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    let by_length = left.len().cmp(&right.len());
    if by_length != Ordering::Equal {
        return by_length;
    }
    let accents = |units: &[Unit]| -> Vec<bool> {
        units
            .iter()
            .map(|unit| matches!(unit, Unit::Char { accent: true, .. }))
            .collect()
    };
    let cases = |units: &[Unit]| -> Vec<bool> {
        units
            .iter()
            .map(|unit| matches!(unit, Unit::Char { upper: true, .. }))
            .collect()
    };
    accents(&left)
        .cmp(&accents(&right))
        .then_with(|| cases(&left).cmp(&cases(&right)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_794_992_400_000;

    #[test]
    fn days_read_as_the_web_reads_them() {
        assert_eq!(day(0, NOW), "2026-11-18");
        assert_eq!(day(-1, NOW), "2026-11-17");
        assert_eq!(day(-13, NOW), "2026-11-05");
        assert_eq!(long_day("2026-11-18"), "Wednesday, 18 November 2026");
        assert_eq!(long_day("2026-13-01"), "Friday, 1 January 2027");
        assert_eq!(long_day("x"), "Invalid Date");
        assert_eq!(clock(NOW + 5 * MINUTE), "09:05");
        assert_eq!(
            from_template(&format!("{TEMPLATE}{{{{long}}}}"), "2026-11-18"),
            "# 2026-11-18\n\n## Plan\n\n- [ ]\n\n## Log\n\n## Notes\n\n#journal\nWednesday, 18 November 2026"
        );
    }

    #[test]
    fn names_read_as_the_web_reads_them() {
        assert_eq!(free_name("Untitled", &["untitled", "Untitled 2"]), "Untitled 3");
        assert_eq!(free_name("New", &[]), "New");
        assert_eq!(bad_title("  ", &[]).as_deref(), Some("A note needs a name."));
        assert_eq!(
            bad_title("a#b", &[]).as_deref(),
            Some("A name cannot hold [ ] | or #, which links read.")
        );
        assert_eq!(
            bad_title(" Grace ", &["grace"]).as_deref(),
            Some("A note named Grace is already in the vault.")
        );
        assert_eq!(bad_title("ok", &["grace"]), None);
        assert_eq!(word_count("  one two \u{2014} 3 ** x-y "), 4);
        assert_eq!(base36(1_794_992_401_000), "mwlw4ors");
    }

    #[test]
    fn names_sort_as_the_web_sorts_them() {
        let sorted = |numeric: bool| {
            let mut list = vec![
                "a",
                "A",
                "b",
                "B",
                "a-b",
                "a b",
                "ab",
                "a_b",
                "a:b",
                "a.b",
                "item 2",
                "item 10",
                "Item 2",
                "x9",
                "x10",
                "x09",
                "2026-11-05",
                "2026-11-17",
                "10",
                "9",
                "Zebra",
                "apple",
                "#tag",
                "(x)",
                "",
            ];
            list.sort_by(|a, b| locale_compare(a, b, numeric));
            list
        };
        assert_eq!(
            sorted(true),
            [
                "",
                "(x)",
                "#tag",
                "9",
                "10",
                "2026-11-05",
                "2026-11-17",
                "a",
                "A",
                "a b",
                "a_b",
                "a-b",
                "a:b",
                "a.b",
                "ab",
                "apple",
                "b",
                "B",
                "item 2",
                "Item 2",
                "item 10",
                "x9",
                "x09",
                "x10",
                "Zebra"
            ]
        );
        assert_eq!(
            sorted(false),
            [
                "",
                "(x)",
                "#tag",
                "10",
                "2026-11-05",
                "2026-11-17",
                "9",
                "a",
                "A",
                "a b",
                "a_b",
                "a-b",
                "a:b",
                "a.b",
                "ab",
                "apple",
                "b",
                "B",
                "item 10",
                "item 2",
                "Item 2",
                "x09",
                "x10",
                "x9",
                "Zebra"
            ]
        );
        assert_eq!(locale_compare("\u{e4}", "a", true), Ordering::Greater);
        assert_eq!(locale_compare("\u{e4}", "b", true), Ordering::Less);
        assert_eq!(locale_compare("\u{e9}", "e\u{301}", true), Ordering::Equal);
        assert_eq!(locale_compare("A1", "a01", true), Ordering::Greater);
    }
}
