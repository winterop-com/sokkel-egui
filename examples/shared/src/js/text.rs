//! How the web reads a text: what `\s`, `\w` and `\b` match and `trim` cuts, a text's length and
//! slices in UTF-16 units, splitting it into words, `replaceAll`, folding it to match
//! (`normalize('NFD')` without its marks), and `Array.prototype.sort`'s stable merge.

use std::cmp::Ordering;

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

/// A text's length in UTF-16 units, as `length` counts it.
pub fn len16(text: &str) -> usize {
    text.encode_utf16().count()
}

/// A text's first `count` UTF-16 units, as `slice(0, count)` cuts it.
pub fn slice16(text: &str, count: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().take(count).collect();
    String::from_utf16_lossy(&units)
}

/// The byte offset of the `units`-th UTF-16 unit of a text, held within it; a unit inside a
/// character that takes two counts as the start of that character.
pub fn byte_at(text: &str, units: usize) -> usize {
    let mut counted = 0;
    for (at, c) in text.char_indices() {
        if counted >= units {
            return at;
        }
        counted += c.len_utf16();
        if counted > units {
            return at;
        }
    }
    text.len()
}

/// Where a `slice(start, end)` of a list `len` long starts and ends, as the web reads its numbers:
/// a fraction cut toward zero, NaN as 0, a negative one from the end.
pub fn slice_range(len: usize, start: f64, end: Option<f64>) -> (usize, usize) {
    let place = |at: f64| -> usize {
        let at = if at.is_nan() { 0.0 } else { at.trunc() };
        let len = len as f64;
        let at = if at < 0.0 { (len + at).max(0.0) } else { at.min(len) };
        at as usize
    };
    let from = place(start);
    let to = end.map_or(len, place);
    (from, to.max(from))
}

/// A part of a text by UTF-16 units, as `slice` cuts it.
pub fn slice(text: &str, start: f64, end: Option<f64>) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let (from, to) = slice_range(units.len(), start, end);
    String::from_utf16_lossy(&units[from..to])
}

/// Whether a character is one the web's `\w` matches, and so one side of a `\b`.
pub fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Whether `word` stands in `text` with a `\b` on each side, as `/\bword\b/` finds it.
pub fn has_word(text: &str, word: &str) -> bool {
    let first = word.chars().next().is_some_and(is_word);
    let last = word.chars().last().is_some_and(is_word);
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        let start = before.is_some_and(is_word) != first;
        let end = after.is_some_and(is_word) != last;
        start && end
    })
}

/// A text split at each run of spaces, as `split(/\s+/)` splits it: an empty piece at an end that
/// starts or ends with spaces.
pub fn split_spaces(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if is_space(c) {
            let mut end = at + c.len_utf8();
            while let Some(&(next, d)) = chars.peek() {
                if !is_space(d) {
                    break;
                }
                end = next + d.len_utf8();
                chars.next();
            }
            out.push(&text[start..at]);
            start = end;
        }
    }
    out.push(&text[start..]);
    out
}

/// The words of a text: `split(/\s+/).filter(Boolean)`.
pub fn words(text: &str) -> Vec<&str> {
    split_spaces(text).into_iter().filter(|one| !one.is_empty()).collect()
}

/// The words of a text as the web splits them: lowercased, on runs of `\s`, the empty ones left out.
pub fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(is_space)
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Every `search` in a text replaced, as `replaceAll` with a text replaces it: `$$`, `$&`, `` $` ``
/// and `$'` in the replacement stand for a dollar, the match, and what comes before and after it.
pub fn replace_all(text: &str, search: &str, replacement: &str) -> String {
    let mut found: Vec<usize> = Vec::new();
    if search.is_empty() {
        found.extend(text.char_indices().map(|(at, _)| at));
        found.push(text.len());
    } else {
        let mut from = 0;
        while let Some(at) = text[from..].find(search) {
            found.push(from + at);
            from += at + search.len();
        }
    }
    let mut out = String::new();
    let mut last = 0;
    for at in found {
        out.push_str(&text[last..at]);
        let end = at + search.len();
        let mut chars = replacement.char_indices().peekable();
        while let Some((_, c)) = chars.next() {
            if c != '$' {
                out.push(c);
                continue;
            }
            match chars.peek().map(|(_, next)| *next) {
                Some('$') => {
                    out.push('$');
                    chars.next();
                }
                Some('&') => {
                    out.push_str(search);
                    chars.next();
                }
                Some('`') => {
                    out.push_str(&text[..at]);
                    chars.next();
                }
                Some('\'') => {
                    out.push_str(&text[end..]);
                    chars.next();
                }
                _ => out.push('$'),
            }
        }
        last = end;
    }
    out.push_str(&text[last..]);
    out
}

/// The letter an accented Latin letter is under `normalize('NFD')` without its marks.
fn unaccented(c: char) -> char {
    const FROM: &str = "ÀÁÂÃÄÅÇÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåçèéêëìíîïñòóôõöùúûüýÿĀāĂăĄąĆćĈĉĊċČčĎďĒēĔĕĖėĘęĚěĜĝĞğĠġĢģĤĥĨĩĪīĬĭĮįİĴĵĶķĹĺĻļĽľŃńŅņŇňŌōŎŏŐőŔŕŖŗŘřŚśŜŝŞşŠšŢţŤťŨũŪūŬŭŮůŰűŲųŴŵŶŷŸŹźŻżŽž";
    const TO: &str = "AAAAAACEEEEIIIINOOOOOUUUUYaaaaaaceeeeiiiinooooouuuuyyAaAaAaCcCcCcCcDdEeEeEeEeEeGgGgGgGgHhIiIiIiIiIJjKkLlLlLlNnNnNnOoOoOoRrRrRrSsSsSsSsTtTtUuUuUuUuUuUuWwYyYZzZzZz";
    FROM.chars()
        .position(|one| one == c)
        .and_then(|at| TO.chars().nth(at))
        .unwrap_or(c)
}

/// A text as the web folds it to match: `normalize('NFD')`, its combining marks left out, in lower
/// case.
pub fn fold(text: &str) -> String {
    text.chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .map(unaccented)
        .collect::<String>()
        .to_lowercase()
}

/// A list sorted stably, as the web's `Array.prototype.sort` sorts it: a merge of runs doubling in
/// width, each taking from the right only when `order` puts it first, so an order that is not
/// consistent (a NaN read as a tie) never stops it.
pub fn sort_by<T: Clone>(list: &mut [T], mut order: impl FnMut(&T, &T) -> Ordering) {
    let count = list.len();
    let mut source: Vec<T> = list.to_vec();
    let mut target: Vec<T> = list.to_vec();
    let mut width = 1;
    while width < count {
        let mut start = 0;
        while start < count {
            let (mut left, left_end) = (start, (start + width).min(count));
            let (mut right, right_end) = (left_end, (left_end + width).min(count));
            for slot in target.iter_mut().take(right_end).skip(start) {
                if right < right_end && (left >= left_end || order(&source[right], &source[left]) == Ordering::Less) {
                    *slot = source[right].clone();
                    right += 1;
                } else {
                    *slot = source[left].clone();
                    left += 1;
                }
            }
            start += 2 * width;
        }
        std::mem::swap(&mut source, &mut target);
        width *= 2;
    }
    list.clone_from_slice(&source);
}
