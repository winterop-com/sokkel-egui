//! Lyrics as lines, read from LRC or plain text, after sokkel-ui-media's `lrc`. LRC times each line
//! with one or more stamps before it, `[mm:ss.xx]`, and may carry metadata tags such as `[ar:Ada]`
//! and an `[offset:+250]` in milliseconds that moves every line earlier. Pure, so which line is sung
//! at a position has tests and [`Lyrics`](crate::Lyrics) only draws it.

use std::collections::BTreeMap;

/// One line, and the second it is sung at when the lyrics are timed.
#[derive(Clone, Debug, PartialEq)]
pub struct LyricLine {
    pub text: String,
    /// Seconds from the start; none on untimed lyrics.
    pub time: Option<f64>,
}

impl LyricLine {
    pub fn new(text: impl Into<String>, time: Option<f64>) -> Self {
        Self {
            text: text.into(),
            time,
        }
    }
}

/// Lyrics read by [`parse_lyrics`].
#[derive(Clone, Debug, Default, PartialEq)]
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

/// A whole line read as a tag, `[name:value]`, the name letters or `#`.
fn tag_of(line: &str) -> Option<(String, String)> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    let name_end = inner
        .find(|c: char| !(c.is_ascii_alphabetic() || c == '#'))
        .unwrap_or(inner.len());
    if name_end == 0 {
        return None;
    }
    let name = &inner[..name_end];
    let rest = inner[name_end..].trim_start();
    let value = rest.strip_prefix(':')?;
    Some((name.to_lowercase(), value.trim().to_owned()))
}

/// Leading ASCII digits of `s`, at least `least` and at most `most` of them, and what follows.
fn digits(s: &str, least: usize, most: usize) -> Option<(&str, &str)> {
    let count = s.bytes().take_while(u8::is_ascii_digit).count();
    if count < least || count > most {
        return None;
    }
    Some((&s[..count], &s[count..]))
}

/// The decimal fraction `digits` writes, so `5` is half a second.
fn fraction(digits: &str) -> f64 {
    format!("0.{digits}").parse().unwrap_or(0.0)
}

/// A time stamp at the start of `s`: `[m:ss]` with an optional fraction after a point or a colon
/// (`[01:02.5]`, `[01:02.50]`, `[01:02:500]`), or with hours before, `[h:mm:ss.xx]`, whose fraction
/// follows a point. Gives the seconds and the rest after the stamp and the space after it.
fn stamp(s: &str) -> Option<(f64, &str)> {
    let inner = s.strip_prefix('[')?;
    let close = inner.find(']')?;
    let body = &inner[..close];
    let rest = inner[close + 1..].trim_start();
    // hours, minutes, seconds and a fraction after a point
    let hours = || -> Option<f64> {
        let (h, after) = digits(body, 1, usize::MAX)?;
        let (m, after) = digits(after.strip_prefix(':')?, 1, 2)?;
        let (sec, after) = digits(after.strip_prefix(':')?, 1, 2)?;
        let (frac, after) = digits(after.strip_prefix('.')?, 1, usize::MAX)?;
        after.is_empty().then(|| {
            h.parse::<f64>().unwrap_or(0.0) * 3600.0
                + m.parse::<f64>().unwrap_or(0.0) * 60.0
                + sec.parse::<f64>().unwrap_or(0.0)
                + fraction(frac)
        })
    };
    let minutes = || -> Option<f64> {
        let (m, after) = digits(body, 1, usize::MAX)?;
        let (sec, after) = digits(after.strip_prefix(':')?, 1, 2)?;
        let frac = if after.is_empty() {
            0.0
        } else {
            let after = after.strip_prefix('.').or_else(|| after.strip_prefix(':'))?;
            let (frac, end) = digits(after, 1, usize::MAX)?;
            if !end.is_empty() {
                return None;
            }
            fraction(frac)
        };
        Some(m.parse::<f64>().unwrap_or(0.0) * 60.0 + sec.parse::<f64>().unwrap_or(0.0) + frac)
    };
    hours().or_else(minutes).map(|seconds| (seconds, rest))
}

/// `text` without the times of single words in enhanced LRC, `<01:02.50>`, and the space after each.
fn without_word_stamps(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let after = &rest[open..];
        match word_stamp(after) {
            Some(len) => rest = after[len..].trim_start(),
            None => {
                out.push('<');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The length of a word's stamp at the start of `s`, `<(h:)m:ss(.xx)>`.
fn word_stamp(s: &str) -> Option<usize> {
    let inner = s.strip_prefix('<')?;
    let close = inner.find('>')?;
    let body = &inner[..close];
    let parts: Vec<&str> = body.split(':').collect();
    let ok = match parts.as_slice() {
        [m, sec] | [_, m, sec] => {
            let numbers = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
            let head_ok = parts.len() == 2 || numbers(parts[0]);
            let (whole, frac) = sec.split_once('.').map_or((*sec, None), |(w, f)| (w, Some(f)));
            head_ok && numbers(m) && numbers(whole) && whole.len() <= 2 && frac.is_none_or(numbers)
        }
        _ => false,
    };
    // a fraction after a colon, `<01:02:50>`, reads as minutes, seconds and a fraction too
    ok.then_some(close + 2)
}

/// `lines` with blank runs closed up to one blank line and none at either end.
fn settle(lines: Vec<LyricLine>) -> Vec<LyricLine> {
    let mut kept: Vec<LyricLine> = Vec::with_capacity(lines.len());
    for line in lines {
        if line.text.is_empty() && kept.last().is_none_or(|l| l.text.is_empty()) {
            continue;
        }
        kept.push(line);
    }
    while kept.last().is_some_and(|l| l.text.is_empty()) {
        kept.pop();
    }
    kept
}

/// Lyrics read from `text`. A line may carry several stamps, `[00:12.00][01:30.00] the chorus`, and
/// is sung at each of them. When some line is sung after the start the lyrics are timed: the
/// stamped lines in the order they are sung, the `offset` tag applied, and unstamped lines left
/// out. Otherwise they are untimed, every line in the order written, as a page to read; that covers
/// plain text and LRC whose stamps are all at zero.
pub fn parse_lyrics(text: &str) -> ParsedLyrics {
    let mut tags = BTreeMap::new();
    let mut stamped: Vec<(String, f64, usize)> = Vec::new();
    let mut plain: Vec<LyricLine> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.trim();
        if let Some((name, value)) = tag_of(line)
            && TAGS.contains(&name.as_str())
        {
            tags.insert(name, value);
            continue;
        }
        let mut times = Vec::new();
        let mut rest = line;
        while let Some((seconds, after)) = stamp(rest) {
            times.push(seconds);
            rest = after;
        }
        let words = without_word_stamps(rest).trim().to_owned();
        for time in times {
            let order = stamped.len();
            stamped.push((words.clone(), time, order));
        }
        plain.push(LyricLine::new(words, None));
    }
    let timed = stamped.iter().any(|(_, time, _)| *time > 0.0);
    if !timed {
        return ParsedLyrics {
            lines: settle(plain),
            timed: false,
            tags,
        };
    }
    let offset = tags
        .get("offset")
        .and_then(|o| o.trim_start_matches('+').parse::<f64>().ok())
        .filter(|o| o.is_finite())
        .unwrap_or(0.0)
        / 1000.0;
    stamped.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.2.cmp(&b.2)));
    let lines = stamped
        .into_iter()
        .map(|(text, time, _)| LyricLine::new(text, Some((time - offset).max(0.0))))
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

#[cfg(test)]
mod tests {
    use super::*;

    const LRC: &str = "[ti:Low Tide]
[ar:Ada]
[al:Harbor]
[length: 03:12]
[00:12.00]The water leaves the harbor wall
[00:17.5]and every boat lies down
[00:23.250][01:05.00]Low tide, low tide
[00:29.00]
[00:31.00]<00:31.00>we wait <00:32.10>for the turn";

    fn times(lyrics: &ParsedLyrics) -> Vec<f64> {
        lyrics.lines.iter().map(|l| l.time.unwrap_or(f64::NAN)).collect()
    }

    fn texts(lyrics: &ParsedLyrics) -> Vec<&str> {
        lyrics.lines.iter().map(|l| l.text.as_str()).collect()
    }

    #[test]
    fn times_each_line_and_keeps_the_tags() {
        let lyrics = parse_lyrics(LRC);
        assert!(lyrics.timed);
        let tags: Vec<(&str, &str)> = lyrics.tags.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(
            tags,
            [("al", "Harbor"), ("ar", "Ada"), ("length", "03:12"), ("ti", "Low Tide")]
        );
        assert_eq!(
            lyrics.lines[0],
            LyricLine::new("The water leaves the harbor wall", Some(12.0))
        );
        assert_eq!(lyrics.lines[1].time, Some(17.5));
        assert_eq!(lyrics.lines[2].time, Some(23.25));
    }

    #[test]
    fn sings_a_line_with_several_stamps_at_each_in_the_order_sung() {
        let lyrics = parse_lyrics(LRC);
        assert_eq!(times(&lyrics), [12.0, 17.5, 23.25, 29.0, 31.0, 65.0]);
        assert_eq!(lyrics.lines[5].text, "Low tide, low tide");
    }

    #[test]
    fn keeps_a_stamped_blank_line_as_a_pause_and_drops_the_times_of_single_words() {
        let lyrics = parse_lyrics(LRC);
        assert_eq!(lyrics.lines[3], LyricLine::new("", Some(29.0)));
        assert_eq!(lyrics.lines[4].text, "we wait for the turn");
    }

    #[test]
    fn reads_a_fraction_as_decimal_after_a_point_or_a_colon() {
        let lyrics = parse_lyrics("[01:02.5]a\n[01:02.50]b\n[01:02:500]c\n[1:02]d");
        assert_eq!(times(&lyrics), [62.0, 62.5, 62.5, 62.5]);
    }

    #[test]
    fn reads_a_stamp_with_hours_and_a_fraction_past_three_digits() {
        let lyrics = parse_lyrics("[01:02:03.50]an hour in\n[00:04.1234]early <00:04.5678>on");
        assert_eq!(
            lyrics.lines,
            [
                LyricLine::new("early on", Some(4.1234)),
                LyricLine::new("an hour in", Some(3723.5)),
            ]
        );
    }

    #[test]
    fn moves_every_line_earlier_by_a_positive_offset_and_later_by_a_negative_one() {
        assert_eq!(parse_lyrics("[offset:+500]\n[00:10.00]a").lines[0].time, Some(9.5));
        assert_eq!(parse_lyrics("[offset:-250]\n[00:10.00]a").lines[0].time, Some(10.25));
        assert_eq!(
            parse_lyrics("[00:00.20]a\n[00:10.00]b\n[offset:1000]").lines[0].time,
            Some(0.0)
        );
    }

    #[test]
    fn keeps_lines_that_share_a_moment_in_the_order_written() {
        assert_eq!(
            texts(&parse_lyrics("[00:05.00]first\n[00:05.00]second")),
            ["first", "second"]
        );
    }

    #[test]
    fn reads_plain_text_as_a_page_one_blank_line_between_verses() {
        let lyrics = parse_lyrics("\n\nOne\nTwo\n\n\n\nThree\n\n");
        assert!(!lyrics.timed);
        assert_eq!(
            lyrics.lines,
            [
                LyricLine::new("One", None),
                LyricLine::new("Two", None),
                LyricLine::new("", None),
                LyricLine::new("Three", None),
            ]
        );
    }

    #[test]
    fn keeps_a_bracketed_line_that_is_no_tag_as_words() {
        let lyrics = parse_lyrics("[Chorus: Ada]\nLow tide");
        assert_eq!(texts(&lyrics), ["[Chorus: Ada]", "Low tide"]);
        assert!(lyrics.tags.is_empty());
    }

    #[test]
    fn reads_lrc_whose_stamps_are_all_at_zero_as_untimed_the_stamps_gone() {
        let lyrics = parse_lyrics("[ar:Ada]\n[00:00.00]One\n[00:00.00]Two");
        assert!(!lyrics.timed);
        assert_eq!(lyrics.lines, [LyricLine::new("One", None), LyricLine::new("Two", None)]);
    }

    #[test]
    fn has_no_lines_in_an_empty_text() {
        assert!(parse_lyrics("").lines.is_empty());
        assert!(!parse_lyrics("  \n ").timed);
    }

    #[test]
    fn reads_windows_line_ends() {
        assert_eq!(texts(&parse_lyrics("One\r\nTwo")), ["One", "Two"]);
    }

    #[test]
    fn the_line_sung_is_the_last_one_started_and_none_before_the_first() {
        let lines = parse_lyrics(LRC).lines;
        assert_eq!(lyric_line_at(&lines, 0.0), None);
        assert_eq!(lyric_line_at(&lines, 11.99), None);
        assert_eq!(lyric_line_at(&lines, 12.0), Some(0));
        assert_eq!(lyric_line_at(&lines, 20.0), Some(1));
        assert_eq!(lyric_line_at(&lines, 64.0), Some(4));
        assert_eq!(lyric_line_at(&lines, 600.0), Some(5));
    }

    #[test]
    fn no_line_is_sung_on_untimed_lines() {
        let lines = [LyricLine::new("One", None), LyricLine::new("Two", None)];
        assert_eq!(lyric_line_at(&lines, 30.0), None);
    }
}
