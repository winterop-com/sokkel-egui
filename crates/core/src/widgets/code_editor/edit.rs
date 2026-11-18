//! What the editor reads off its text and does to it: offsets between bytes and characters,
//! indenting by a unit, a new line at its indent, the bracket matching the one at the caret, the
//! blocks that fold, `[[links]]`, and the find bar's matches.

use super::CodeLanguage;
use super::highlight::{Kind, Token};
use super::regex::Regex;

/// The indent unit: two spaces, CodeMirror's default.
pub(crate) const UNIT: &str = "  ";

/// The start of every line, in bytes and in characters, for turning one into the other.
pub(crate) struct Offsets {
    bytes: Vec<usize>,
    chars: Vec<usize>,
}

impl Offsets {
    pub(crate) fn new(text: &str) -> Self {
        let mut bytes = vec![0];
        let mut chars = vec![0];
        let mut count = 0;
        for (i, c) in text.char_indices() {
            count += 1;
            if c == '\n' {
                bytes.push(i + 1);
                chars.push(count);
            }
        }
        Self { bytes, chars }
    }

    pub(crate) fn lines(&self) -> usize {
        self.bytes.len()
    }

    /// The line (from 0) a byte offset stands on.
    pub(crate) fn line_of(&self, byte: usize) -> usize {
        self.bytes.partition_point(|b| *b <= byte).saturating_sub(1)
    }

    pub(crate) fn line_start(&self, line: usize) -> usize {
        self.bytes[line.min(self.bytes.len() - 1)]
    }

    /// Where a line's text ends, before its line break.
    pub(crate) fn line_end(&self, text: &str, line: usize) -> usize {
        self.bytes.get(line + 1).map_or(text.len(), |next| next - 1)
    }

    pub(crate) fn to_char(&self, text: &str, byte: usize) -> usize {
        let byte = floor_char(text, byte.min(text.len()));
        let line = self.line_of(byte);
        self.chars[line] + text[self.bytes[line]..byte].chars().count()
    }

    pub(crate) fn to_byte(&self, text: &str, char_index: usize) -> usize {
        let line = self.chars.partition_point(|c| *c <= char_index).saturating_sub(1);
        let start = self.bytes[line];
        let skip = char_index - self.chars[line];
        text[start..]
            .char_indices()
            .nth(skip)
            .map_or(text.len(), |(i, _)| start + i)
    }
}

/// The nearest character boundary at or before a byte offset.
pub(crate) fn floor_char(text: &str, mut byte: usize) -> usize {
    byte = byte.min(text.len());
    while !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

/// A change to the text: the range replaced, what goes in, and the selection after, all bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Change {
    pub text: String,
    pub selection: (usize, usize),
}

/// Each line the selection touches indented by a unit, the selection moving with its text.
pub(crate) fn indent_more(text: &str, from: usize, to: usize) -> Change {
    let offsets = Offsets::new(text);
    let first = offsets.line_of(from);
    // a selection that ends at a line's start leaves that line be
    let last = if to > from && to == offsets.line_start(offsets.line_of(to)) {
        offsets.line_of(to - 1)
    } else {
        offsets.line_of(to)
    };
    let mut out = String::with_capacity(text.len() + UNIT.len() * (last - first + 1));
    let mut cursor = 0;
    for line in first..=last {
        let start = offsets.line_start(line);
        out.push_str(&text[cursor..start]);
        out.push_str(UNIT);
        cursor = start;
    }
    out.push_str(&text[cursor..]);
    // the selection moves by the units put in at or before it
    let moved = |at: usize| {
        let line = offsets.line_of(at);
        let lines = if line < first { 0 } else { line.min(last) - first + 1 };
        at + lines * UNIT.len()
    };
    Change {
        text: out,
        selection: (moved(from), moved(to)),
    }
}

/// Each line the selection touches outdented by up to a unit.
pub(crate) fn indent_less(text: &str, from: usize, to: usize) -> Change {
    let offsets = Offsets::new(text);
    let first = offsets.line_of(from);
    let last = offsets.line_of(to);
    let mut removed_by_line = Vec::new();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for line in first..=last {
        let start = offsets.line_start(line);
        out.push_str(&text[cursor..start]);
        let lead = &text[start..];
        let remove = if lead.starts_with('\t') {
            1
        } else {
            lead.bytes().take(UNIT.len()).take_while(|b| *b == b' ').count()
        };
        removed_by_line.push((start, remove));
        cursor = start + remove;
    }
    out.push_str(&text[cursor..]);
    let moved = |at: usize| {
        let mut shift = 0;
        for (start, remove) in &removed_by_line {
            if at >= start + remove {
                shift += remove;
            } else if at > *start {
                shift += at - start;
            }
        }
        at - shift
    };
    Change {
        text: out,
        selection: (moved(from), moved(to)),
    }
}

/// A line break at the caret, the new line at the caret line's indent, a unit deeper after an
/// opening bracket (or a YAML key's colon), and a closing bracket right after the caret put on a
/// line of its own.
pub(crate) fn newline(text: &str, from: usize, to: usize, language: CodeLanguage) -> Change {
    let offsets = Offsets::new(text);
    let line = offsets.line_of(from);
    let start = offsets.line_start(line);
    let indent: String = text[start..from]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let before = text[start..from].trim_end();
    let after = &text[to..];
    let opener = before.chars().next_back();
    let deeper = matches!(opener, Some('{' | '[' | '('))
        || (language == CodeLanguage::Yaml && before.ends_with(':'))
        || (language == CodeLanguage::Yaml && before.trim_start() == "-");
    let closer = after.trim_start_matches([' ', '\t']).chars().next();
    let pair = matches!(
        (opener, closer),
        (Some('{'), Some('}')) | (Some('['), Some(']')) | (Some('('), Some(')'))
    );
    let inner = if deeper {
        format!("{indent}{UNIT}")
    } else {
        indent.clone()
    };
    // spaces before the caret on the line it leaves are dropped
    let cut = start + text[start..from].trim_end_matches([' ', '\t']).len();
    let cut = if cut <= start + indent.len() { from } else { cut };
    let after_spaces = to + (after.len() - after.trim_start_matches([' ', '\t']).len());
    let mut out = String::with_capacity(text.len() + inner.len() * 2 + 2);
    out.push_str(&text[..cut]);
    out.push('\n');
    out.push_str(&inner);
    let caret = out.len();
    if pair {
        out.push('\n');
        out.push_str(&indent);
        out.push_str(&text[after_spaces..]);
    } else {
        out.push_str(&text[to..]);
    }
    Change {
        text: out,
        selection: (caret, caret),
    }
}

const OPEN: &str = "([{";
const CLOSE: &str = ")]}";

fn in_text(tokens: &[Token], at: usize) -> bool {
    let i = tokens.partition_point(|t| t.to <= at);
    tokens
        .get(i)
        .is_some_and(|t| t.from <= at && at < t.to && t.kind.is_text())
}

/// The bracket next to the caret and the one matching it, as byte offsets: the one before the
/// caret first, then the one after, as CodeMirror's bracket matching reads them.
pub(crate) fn matching_bracket(text: &str, tokens: &[Token], caret: usize) -> Option<(usize, usize)> {
    let b = text.as_bytes();
    let candidates = [caret.checked_sub(1), Some(caret)];
    for at in candidates.into_iter().flatten() {
        let Some(&c) = b.get(at) else { continue };
        let c = c as char;
        if in_text(tokens, at) {
            continue;
        }
        if let Some(kind) = OPEN.find(c) {
            let close = CLOSE.as_bytes()[kind];
            let mut depth = 0;
            for (i, &x) in b.iter().enumerate().skip(at) {
                if (x == c as u8 || x == close) && in_text(tokens, i) {
                    continue;
                }
                if x == c as u8 {
                    depth += 1;
                } else if x == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some((at, i));
                    }
                }
            }
        } else if let Some(kind) = CLOSE.find(c) {
            let open = OPEN.as_bytes()[kind];
            let mut depth = 0;
            for i in (0..=at).rev() {
                let x = b[i];
                if (x == c as u8 || x == open) && in_text(tokens, i) {
                    continue;
                }
                if x == c as u8 {
                    depth += 1;
                } else if x == open {
                    depth -= 1;
                    if depth == 0 {
                        return Some((at, i));
                    }
                }
            }
        }
    }
    None
}

/// Each bracket that is structure, with its depth, for coloring brackets by depth.
pub(crate) fn bracket_depths(text: &str, tokens: &[Token]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut depth: usize = 0;
    let mut t = 0;
    for (i, c) in text.bytes().enumerate() {
        let opens = OPEN.as_bytes().contains(&c);
        if !opens && !CLOSE.as_bytes().contains(&c) {
            continue;
        }
        while t < tokens.len() && tokens[t].to <= i {
            t += 1;
        }
        if tokens.get(t).is_some_and(|tok| tok.from <= i && tok.kind.is_text()) {
            continue;
        }
        if !opens {
            depth = depth.saturating_sub(1);
        }
        out.push((i, depth));
        if opens {
            depth += 1;
        }
    }
    out
}

/// A block that folds: the line it opens on and the lines it hides, from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fold {
    pub line: usize,
    pub first: usize,
    pub last: usize,
}

/// The blocks a language knows: JSON's objects and lists spanning lines, YAML's indented blocks.
pub(crate) fn folds(text: &str, tokens: &[Token], language: CodeLanguage) -> Vec<Fold> {
    let offsets = Offsets::new(text);
    let mut out = Vec::new();
    match language {
        CodeLanguage::Json => {
            let mut stack: Vec<usize> = Vec::new();
            for (at, _) in bracket_depths(text, tokens) {
                let c = text.as_bytes()[at];
                if c == b'{' || c == b'[' {
                    stack.push(at);
                } else if (c == b'}' || c == b']')
                    && let Some(open) = stack.pop()
                {
                    let line = offsets.line_of(open);
                    let close = offsets.line_of(at);
                    if close > line + 1 {
                        out.push(Fold {
                            line,
                            first: line + 1,
                            last: close - 1,
                        });
                    }
                }
            }
            out.sort_by_key(|f| f.line);
            out.dedup_by_key(|f| f.line);
        }
        CodeLanguage::Yaml => {
            let lines: Vec<&str> = text.split('\n').collect();
            let indent = |l: &str| l.len() - l.trim_start_matches(' ').len();
            let blank = |l: &str| l.trim().is_empty() || l.trim_start().starts_with('#');
            for (n, line) in lines.iter().enumerate() {
                if blank(line) {
                    continue;
                }
                let own = indent(line);
                let mut last = None;
                for (m, next) in lines.iter().enumerate().skip(n + 1) {
                    if blank(next) {
                        continue;
                    }
                    if indent(next) > own {
                        last = Some(m);
                    } else {
                        break;
                    }
                }
                if let Some(last) = last {
                    out.push(Fold {
                        line: n,
                        first: n + 1,
                        last,
                    });
                }
            }
        }
        _ => {}
    }
    out
}

/// One `[[link]]`: the note it names, the heading after `#`, and where it stands in bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WikiLink {
    pub target: String,
    pub heading: Option<String>,
    pub from: usize,
    pub to: usize,
}

/// The text with its code blanked, fenced blocks and backtick spans, every offset kept.
fn without_code(text: &str) -> String {
    let mut fence: Option<String> = None;
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let trimmed = line.trim_start_matches(' ');
        let mark: Option<String> = (line.len() - trimmed.len() <= 3)
            .then(|| {
                let c = trimmed.chars().next()?;
                if c != '`' && c != '~' {
                    return None;
                }
                let n = trimmed.len() - trimmed.trim_start_matches(c).len();
                (n >= 3).then(|| c.to_string().repeat(n))
            })
            .flatten();
        let blank = |s: &str| " ".repeat(s.len());
        if fence.is_none() && mark.is_some() {
            fence = mark;
            out.push_str(&blank(line));
            continue;
        }
        if let Some(open) = &fence {
            if mark
                .as_ref()
                .is_some_and(|m| m.starts_with(&open[..1]) && m.len() >= open.len() && line.trim() == m)
            {
                fence = None;
            }
            out.push_str(&blank(line));
            continue;
        }
        let b = line.as_bytes();
        let mut kept = line.to_owned().into_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] != b'`' {
                i += 1;
                continue;
            }
            let open = i;
            while i < b.len() && b[i] == b'`' {
                i += 1;
            }
            let ticks = i - open;
            let Some(close) = line[i..].find(&"`".repeat(ticks)).map(|c| i + c) else {
                break;
            };
            for x in kept.iter_mut().take(close + ticks).skip(open) {
                *x = b' ';
            }
            i = close + ticks;
        }
        out.push_str(&String::from_utf8(kept).unwrap_or_else(|_| line.to_owned()));
    }
    out
}

/// Every `[[link]]` in a text outside its code, in order.
pub(crate) fn wiki_links(text: &str) -> Vec<WikiLink> {
    let plain = without_code(text);
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = plain[at..].find("[[") {
        let from = at + found;
        match super::highlight::wiki_end(&plain[from..]) {
            Some(len) => {
                let inside = &text[from + 2..from + len - 2];
                let name = inside.split('|').next().unwrap_or("");
                let (target, heading) = match name.split_once('#') {
                    Some((t, h)) => (t.trim(), Some(h.trim().to_owned()).filter(|h| !h.is_empty())),
                    None => (name.trim(), None),
                };
                if !target.is_empty() {
                    out.push(WikiLink {
                        target: target.to_owned(),
                        heading,
                        from,
                        to: from + len,
                    });
                }
                at = from + len;
            }
            None => at = from + 2,
        }
    }
    out
}

/// The `[[link]]` being typed at the caret: where its name starts and what is typed of it so far.
pub(crate) fn wiki_query(text: &str, caret: usize) -> Option<(usize, &str)> {
    let start = text[..caret].rfind('\n').map_or(0, |n| n + 1);
    let line = &text[start..caret];
    let open = line.rfind("[[")?;
    let query = &line[open + 2..];
    (!query.contains(['[', ']'])).then_some((caret - query.len(), query))
}

/// The word under a byte offset: letters, digits, `_`, and a leading `$` or `@`.
pub(crate) fn word_at(text: &str, at: usize) -> Option<(usize, usize)> {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '$' || c == '@';
    let at = floor_char(text, at);
    let start = text[..at]
        .char_indices()
        .rev()
        .take_while(|(_, c)| word(*c))
        .last()
        .map_or(at, |(i, _)| i);
    let end = text[at..].find(|c: char| !word(c)).map_or(text.len(), |e| at + e);
    (end > start).then_some((start, end))
}

/// What the find bar looks for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Query {
    pub search: String,
    pub case: bool,
    pub regexp: bool,
    pub word: bool,
}

/// Every match of a query in the text, as byte ranges.
pub(crate) fn matches(text: &str, query: &Query) -> Vec<(usize, usize)> {
    if query.search.is_empty() {
        return Vec::new();
    }
    let found: Vec<(usize, usize)> = if query.regexp {
        match Regex::new(&query.search, !query.case) {
            Some(r) => r.find_all(text),
            None => Vec::new(),
        }
    } else if query.case {
        text.match_indices(&query.search)
            .map(|(i, m)| (i, i + m.len()))
            .collect()
    } else {
        let lower = text.to_lowercase();
        let needle = query.search.to_lowercase();
        if lower.len() == text.len() {
            lower.match_indices(&needle).map(|(i, m)| (i, i + m.len())).collect()
        } else {
            // a lowercase that changes lengths: compare character by character
            let chars: Vec<(usize, char)> = text.char_indices().collect();
            let want: Vec<char> = needle.chars().collect();
            let mut out = Vec::new();
            let mut i = 0;
            while i + want.len() <= chars.len() {
                if chars[i..i + want.len()]
                    .iter()
                    .zip(&want)
                    .all(|((_, c), w)| c.to_lowercase().eq(w.to_lowercase()))
                {
                    let end = chars.get(i + want.len()).map_or(text.len(), |(b, _)| *b);
                    out.push((chars[i].0, end));
                    i += want.len().max(1);
                } else {
                    i += 1;
                }
            }
            out
        }
    };
    if !query.word {
        return found;
    }
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    found
        .into_iter()
        .filter(|(a, b)| !word(text[..*a].chars().next_back()) && !word(text[*b..].chars().next()))
        .collect()
}

/// Whether a token's kind is a text kind the language's words are never offered in.
pub(crate) fn in_string_or_comment(tokens: &[Token], at: usize) -> bool {
    let i = tokens.partition_point(|t| t.to < at);
    tokens
        .get(i)
        .is_some_and(|t| t.from < at && at <= t.to && matches!(t.kind, Kind::String | Kind::Comment))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::code_editor::highlight::tokens;

    #[test]
    fn offsets_turn_bytes_into_characters_and_back() {
        let text = "a\u{e5}b\nc\u{f8}d";
        let o = Offsets::new(text);
        assert_eq!(o.to_char(text, 3), 2);
        assert_eq!(o.to_byte(text, 2), 3);
        assert_eq!(o.to_byte(text, 5), 6);
        assert_eq!(o.line_of(6), 1);
    }

    #[test]
    fn tab_indents_each_line_of_the_selection() {
        let change = indent_more("a\nb\nc", 2, 3);
        assert_eq!(change.text, "a\n  b\nc");
        assert_eq!(change.selection, (4, 5));
        let back = indent_less(&change.text, 4, 5);
        assert_eq!(back.text, "a\nb\nc");
        assert_eq!(back.selection, (2, 3));
    }

    #[test]
    fn enter_keeps_the_indent_and_opens_a_bracket() {
        let change = newline("  {}", 3, 3, CodeLanguage::Json);
        assert_eq!(change.text, "  {\n    \n  }");
        assert_eq!(change.selection, (8, 8));
        let yaml = newline("steps:", 6, 6, CodeLanguage::Yaml);
        assert_eq!(yaml.text, "steps:\n  ");
    }

    #[test]
    fn the_matching_bracket_skips_strings() {
        let text = r#"{"a": "}", "b": [1]}"#;
        let t = tokens(CodeLanguage::Json, text);
        assert_eq!(matching_bracket(text, &t, 1), Some((0, text.len() - 1)));
    }

    #[test]
    fn json_and_yaml_blocks_fold() {
        let text = "{\n  \"a\": [\n    1\n  ]\n}";
        let t = tokens(CodeLanguage::Json, text);
        let f = folds(text, &t, CodeLanguage::Json);
        assert_eq!(
            f,
            vec![
                Fold {
                    line: 0,
                    first: 1,
                    last: 3
                },
                Fold {
                    line: 1,
                    first: 2,
                    last: 2
                }
            ]
        );
        let y = folds("a:\n  b: 1\n  c: 2\nd: 3", &[], CodeLanguage::Yaml);
        assert_eq!(
            y,
            vec![Fold {
                line: 0,
                first: 1,
                last: 2
            }]
        );
    }

    #[test]
    fn wiki_links_outside_code() {
        let text = "See [[Runs#Failed|the runs]] and `[[not]]`.";
        let links = wiki_links(text);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Runs");
        assert_eq!(links[0].heading.as_deref(), Some("Failed"));
        assert_eq!(&text[links[0].from..links[0].to], "[[Runs#Failed|the runs]]");
        assert_eq!(wiki_query("x [[Ni", 6), Some((4, "Ni")));
    }

    #[test]
    fn search_by_case_word_and_regexp() {
        let q = |s: &str, case, regexp, word| Query {
            search: s.into(),
            case,
            regexp,
            word,
        };
        assert_eq!(matches("Run run rerun", &q("run", false, false, false)).len(), 3);
        assert_eq!(matches("Run run rerun", &q("run", true, false, false)).len(), 2);
        assert_eq!(matches("Run run rerun", &q("run", false, false, true)).len(), 2);
        assert_eq!(matches("a1 b22", &q("\\d+", false, true, false)), vec![(1, 2), (4, 6)]);
    }
}
