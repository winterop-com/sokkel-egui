//! Lyrics as lines, read from LRC or plain text, as sokkel-ui-media's `lrc.ts` reads them. LRC
//! times each line with one or more stamps before it, `[mm:ss.xx]`, and may carry metadata tags such
//! as `[ar:Ada]` and an `[offset:+250]` in milliseconds that moves every line earlier. Pure, so
//! which line is sung at a position is the same here as on the web.
//!
//! Library logic: this belongs in the media crate, and is kept here until it moves there.

use std::collections::BTreeMap;

use crate::js;

/// One line, and the second it is sung at when the lyrics are timed.
#[derive(Clone, Debug, PartialEq)]
pub struct LyricLine {
    pub text: String,
    /// Seconds from the start; none on untimed lyrics.
    pub time: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ParsedLyrics {
    /// Timed lines in the order they are sung; untimed ones in the order written, blank lines
    /// between verses kept.
    pub lines: Vec<LyricLine>,
    /// Whether the lines follow the music: some line is sung after the start.
    pub timed: bool,
    /// The metadata tags, keyed in lower case: `ti` the title, `ar` the artist, `al` the album,
    /// `length`, `offset` and the rest.
    pub tags: BTreeMap<String, String>,
}

/// The tags LRC defines; a bracketed line with another name, such as `[Chorus: Ada]`, is a line of
/// the lyrics.
const TAGS: [&str; 13] = [
    "ti", "ar", "al", "au", "by", "re", "ve", "tool", "length", "offset", "la", "lang", "#",
];

/// Whether `c` is one the web's `.` leaves out: a line's end.
fn ends_line(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `^\[([A-Za-z#]+)\s*:(.*)\]$`: a tag's name and its value.
fn tag_of(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix('[')?;
    let name_end = rest
        .find(|c: char| !(c.is_ascii_alphabetic() || c == '#'))
        .unwrap_or(rest.len());
    if name_end == 0 {
        return None;
    }
    let name = &rest[..name_end];
    let after = rest[name_end..].trim_start_matches(js::is_space);
    let value = after.strip_prefix(':')?.strip_suffix(']')?;
    if value.chars().any(ends_line) {
        return None;
    }
    Some((name, value))
}

/// The run of ASCII digits at the start of `text`.
fn digits(text: &str) -> &str {
    let end = text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());
    &text[..end]
}

/// The seconds a stamp's fraction says: decimal, so `5` is half a second.
fn decimal(digits: Option<&str>) -> f64 {
    digits.map_or(0.0, |digits| js::number(&format!("0.{digits}")))
}

/// `^\[(?:(\d+):(\d{1,2}):(\d{1,2})\.(\d+)|(\d+):(\d{1,2})(?:[.:](\d+))?)\]\s*`: the seconds a
/// stamp at the start of `text` says, and how long the stamp is with the spaces after it.
fn stamp_of(text: &str) -> Option<(f64, usize)> {
    let body = text.strip_prefix('[')?;
    let first = digits(body);
    if first.is_empty() {
        return None;
    }
    let rest = body[first.len()..].strip_prefix(':')?;
    let second = digits(rest);
    if second.is_empty() || second.len() > 2 {
        return None;
    }
    let after = &rest[second.len()..];
    let close = |after: &str, seconds: f64| -> Option<(f64, usize)> {
        let tail = after.strip_prefix(']')?;
        let spaces = tail.len() - tail.trim_start_matches(js::is_space).len();
        Some((seconds, text.len() - tail.len() + spaces))
    };
    // h:mm:ss.frac
    if let Some(more) = after.strip_prefix(':') {
        let third = digits(more);
        if (1..=2).contains(&third.len())
            && let Some(point) = more[third.len()..].strip_prefix('.')
        {
            let fraction = digits(point);
            if !fraction.is_empty()
                && let Some(found) = close(
                    &point[fraction.len()..],
                    js::number(first) * 3600.0
                        + js::number(second) * 60.0
                        + js::number(third)
                        + decimal(Some(fraction)),
                )
            {
                return Some(found);
            }
        }
    }
    // m:ss, with a fraction after a point or a colon.
    let minutes_seconds = js::number(first) * 60.0 + js::number(second);
    if let Some(point) = after.strip_prefix(['.', ':']) {
        let fraction = digits(point);
        if !fraction.is_empty() {
            return close(&point[fraction.len()..], minutes_seconds + decimal(Some(fraction)));
        }
        return None;
    }
    close(after, minutes_seconds + decimal(None))
}

/// `<(?:\d+:)?\d+:\d{1,2}(?:[.:]\d+)?>\s*`: how long a word's own stamp at the start of `text` is.
fn word_stamp(text: &str) -> Option<usize> {
    let body = text.strip_prefix('<')?;
    let tail = |from: &str| -> Option<usize> {
        let minutes = digits(from);
        if minutes.is_empty() {
            return None;
        }
        let rest = from[minutes.len()..].strip_prefix(':')?;
        let seconds = digits(rest);
        if !(1..=2).contains(&seconds.len()) {
            return None;
        }
        let mut rest = &rest[seconds.len()..];
        if let Some(point) = rest.strip_prefix(['.', ':']) {
            let fraction = digits(point);
            if !fraction.is_empty() && point[fraction.len()..].starts_with('>') {
                rest = &point[fraction.len()..];
            }
        }
        let after = rest.strip_prefix('>')?;
        Some(text.len() - after.trim_start_matches(js::is_space).len())
    };
    let hours = digits(body);
    if !hours.is_empty()
        && let Some(rest) = body[hours.len()..].strip_prefix(':')
        && let Some(found) = tail(rest)
    {
        return Some(found);
    }
    tail(body)
}

/// `text` without the words' own stamps.
fn without_word_stamps(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < text.len() {
        if let Some(length) = word_stamp(&text[at..]) {
            at += length;
            continue;
        }
        let c = text[at..].chars().next().unwrap_or(' ');
        out.push(c);
        at += c.len_utf8();
    }
    out
}

/// `lines` with blank runs closed up to one blank line and none at either end.
fn settle(lines: Vec<LyricLine>) -> Vec<LyricLine> {
    let mut kept: Vec<LyricLine> = Vec::new();
    for line in lines {
        if line.text.is_empty() && kept.last().is_none_or(|last| last.text.is_empty()) {
            continue;
        }
        kept.push(line);
    }
    while kept.last().is_some_and(|last| last.text.is_empty()) {
        kept.pop();
    }
    kept
}

/// Lyrics read from `text`. A line may carry several stamps, `[00:12.00][01:30.00] the chorus`,
/// and is sung at each of them. When some line is sung after the start the lyrics are timed: the
/// stamped lines in the order they are sung, the `offset` tag applied, and unstamped lines left
/// out. Otherwise they are untimed, every line in the order written, as a page to read.
pub fn parse_lyrics(text: &str) -> ParsedLyrics {
    let mut tags = BTreeMap::new();
    let mut stamped: Vec<(String, f64, usize)> = Vec::new();
    let mut plain: Vec<LyricLine> = Vec::new();
    for raw in text.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let line = js::trim(raw);
        if let Some((name, value)) = tag_of(line) {
            let key = name.to_ascii_lowercase();
            if TAGS.contains(&key.as_str()) {
                tags.insert(key, js::trim(value).to_owned());
                continue;
            }
        }
        let mut times = Vec::new();
        let mut rest = line;
        while let Some((seconds, length)) = stamp_of(rest) {
            times.push(seconds);
            rest = &rest[length..];
        }
        let words = js::trim(&without_word_stamps(rest)).to_owned();
        for time in times {
            let order = stamped.len();
            stamped.push((words.clone(), time, order));
        }
        plain.push(LyricLine {
            text: words,
            time: None,
        });
    }
    let timed = stamped.iter().any(|(_, time, _)| *time > 0.0);
    if !timed {
        return ParsedLyrics {
            lines: settle(plain),
            timed: false,
            tags,
        };
    }
    let offset = tags.get("offset").map_or(0.0, |offset| js::number(offset)) / 1000.0;
    let offset = if offset.is_finite() { offset } else { 0.0 };
    stamped.sort_by(|one, other| {
        let by_time = one.1 - other.1;
        if by_time != 0.0 && !by_time.is_nan() {
            by_time.total_cmp(&0.0)
        } else {
            one.2.cmp(&other.2)
        }
    });
    let lines = stamped
        .into_iter()
        .map(|(text, time, _)| LyricLine {
            text,
            time: Some((time - offset).max(0.0)),
        })
        .collect();
    ParsedLyrics {
        lines,
        timed: true,
        tags,
    }
}

/// The line being sung at `position`: the last one started by then, or none before the first and
/// on untimed lines.
pub fn lyric_line_at(lines: &[LyricLine], position: f64) -> Option<usize> {
    let mut found = None;
    for (index, line) in lines.iter().enumerate() {
        let Some(time) = line.time else { continue };
        if time > position {
            break;
        }
        found = Some(index);
    }
    found
}
