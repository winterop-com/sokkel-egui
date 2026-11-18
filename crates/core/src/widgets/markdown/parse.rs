//! The blocks of a markdown text, as GitHub-flavored markdown reads them: headings (`#` and
//! underlined), paragraphs, bullet, numbered and task lists whose items hold blocks of their own,
//! quotes and the callouts written as quotes, fenced code and rules. Tables, math and raw HTML are
//! read as paragraphs.

/// One block of a markdown text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading(u8, String),
    Paragraph(String),
    /// A list's items, numbered from the first's number when `Some`.
    List(Option<u32>, Vec<ListItem>),
    /// A quote's blocks; a callout when its first line is `[!NOTE]`, `[!TIP]` and so on.
    Quote(Option<Callout>, Vec<Block>),
    /// A fenced block of code: the language its fence names (empty when none) and its text.
    Code(String, String),
    /// A horizontal rule.
    Rule,
}

/// One item of a list: its blocks, the first paragraph set beside the marker, and whether it is a
/// task, checked or not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[cfg(test)]
impl ListItem {
    /// An item of one paragraph.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            task: None,
            blocks: vec![Block::Paragraph(text.into())],
        }
    }
}

/// The kinds of callout a quote becomes when its first line is `[!NOTE]`, `[!TIP]` and so on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callout {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

/// The blocks of a markdown text.
pub fn blocks(text: &str) -> Vec<Block> {
    let lines: Vec<String> = text.lines().map(|line| line.replace('\t', "    ")).collect();
    parse(&lines)
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// A fence's opening line: its indent, its character, how many of them, and the language it names.
fn fence_open(line: &str) -> Option<(usize, char, usize, String)> {
    let at = indent(line);
    if at > 3 {
        return None;
    }
    let rest = &line[at..];
    let mark = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let count = rest.chars().take_while(|c| *c == mark).count();
    if count < 3 {
        return None;
    }
    let info = rest[count..].trim();
    if mark == '`' && info.contains('`') {
        return None;
    }
    let language = info.split_whitespace().next().unwrap_or("").to_owned();
    Some((at, mark, count, language))
}

fn fence_close(line: &str, mark: char, count: usize) -> bool {
    let trimmed = line.trim();
    indent(line) <= 3 && trimmed.chars().all(|c| c == mark) && trimmed.chars().count() >= count
}

/// A `#` heading: its level and its text without the closing `#`s.
fn atx(line: &str) -> Option<(u8, String)> {
    let at = indent(line);
    if at > 3 {
        return None;
    }
    let rest = &line[at..];
    let hashes = rest.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let body = &rest[hashes..];
    if !body.starts_with(' ') {
        return None;
    }
    let mut text = body.trim();
    let stripped = text.trim_end_matches('#');
    if stripped.is_empty() || stripped.ends_with(' ') {
        text = stripped.trim_end();
    }
    if text.is_empty() {
        return None;
    }
    Some((hashes as u8, text.to_owned()))
}

/// A rule: three or more `-`, `*` or `_`, alone on the line but for spaces.
fn rule(line: &str) -> bool {
    if indent(line) > 3 {
        return false;
    }
    let marks: Vec<char> = line.chars().filter(|c| *c != ' ').collect();
    marks.len() >= 3 && matches!(marks[0], '-' | '*' | '_') && marks.iter().all(|c| *c == marks[0])
}

/// A setext underline: `===` makes the paragraph above a level 1 heading, `---` a level 2 one.
fn setext(line: &str) -> Option<u8> {
    let trimmed = line.trim();
    if indent(line) > 3 || trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().all(|c| c == '=') {
        Some(1)
    } else if trimmed.chars().all(|c| c == '-') {
        Some(2)
    } else {
        None
    }
}

fn quote_line(line: &str) -> bool {
    indent(line) <= 3 && line.trim_start().starts_with('>')
}

/// A quote line without its `>` and the one space after it.
fn unquote(line: &str) -> String {
    let rest = &line.trim_start()[1..];
    rest.strip_prefix(' ').unwrap_or(rest).to_owned()
}

/// A list item's marker: its number for a numbered one, the column its text starts at, and the
/// text after it on the marker's line.
struct Marker {
    number: Option<u32>,
    content: usize,
    rest: String,
}

fn list_marker(line: &str) -> Option<Marker> {
    let at = indent(line);
    if at > 3 {
        return None;
    }
    let s = &line[at..];
    let first = s.chars().next()?;
    let (number, width) = if matches!(first, '-' | '*' | '+') {
        (None, 1)
    } else {
        let digits = s.chars().take_while(|c| c.is_ascii_digit()).count();
        if !(1..=9).contains(&digits) || !matches!(s[digits..].chars().next(), Some('.' | ')')) {
            return None;
        }
        (Some(s[..digits].parse().ok()?), digits + 1)
    };
    let after = &s[width..];
    if after.trim().is_empty() {
        return Some(Marker {
            number,
            content: at + width + 1,
            rest: String::new(),
        });
    }
    if !after.starts_with(' ') {
        return None;
    }
    let spaces = after.chars().take_while(|c| *c == ' ').count();
    // five or more spaces start indented code: the text starts one space after the marker
    let spaces = if spaces > 4 { 1 } else { spaces };
    let content = at + width + spaces;
    Some(Marker {
        number,
        content,
        rest: line[content..].to_owned(),
    })
}

/// Whether a line starts a block of its own, so it ends a paragraph rather than continuing it.
fn starts_block(line: &str) -> bool {
    fence_open(line).is_some() || atx(line).is_some() || rule(line) || quote_line(line) || list_marker(line).is_some()
}

/// A callout's marker at the start of a quote's first paragraph, and the text after it.
fn callout(text: &str) -> Option<(Callout, &str)> {
    let rest = text.strip_prefix("[!")?;
    let close = rest.find(']')?;
    let kind = match rest[..close].to_ascii_lowercase().as_str() {
        "note" => Callout::Note,
        "tip" => Callout::Tip,
        "important" => Callout::Important,
        "warning" => Callout::Warning,
        "caution" => Callout::Caution,
        _ => return None,
    };
    Some((kind, rest[close + 1..].trim_start()))
}

fn parse(lines: &[String]) -> Vec<Block> {
    let mut out = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let flush = |paragraph: &mut Vec<String>, out: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            out.push(Block::Paragraph(paragraph.join(" ")));
            paragraph.clear();
        }
    };
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if blank(line) {
            flush(&mut paragraph, &mut out);
            i += 1;
            continue;
        }
        if !paragraph.is_empty()
            && let Some(level) = setext(line)
        {
            out.push(Block::Heading(level, paragraph.join(" ")));
            paragraph.clear();
            i += 1;
            continue;
        }
        if let Some((at, mark, count, language)) = fence_open(line) {
            flush(&mut paragraph, &mut out);
            let mut body = Vec::new();
            i += 1;
            while i < lines.len() && !fence_close(&lines[i], mark, count) {
                let inner = &lines[i];
                let strip = indent(inner).min(at);
                body.push(inner[strip..].to_owned());
                i += 1;
            }
            i += 1;
            out.push(Block::Code(language, body.join("\n")));
            continue;
        }
        if let Some((level, text)) = atx(line) {
            flush(&mut paragraph, &mut out);
            out.push(Block::Heading(level, text));
            i += 1;
            continue;
        }
        if rule(line) {
            flush(&mut paragraph, &mut out);
            out.push(Block::Rule);
            i += 1;
            continue;
        }
        if quote_line(line) {
            flush(&mut paragraph, &mut out);
            let mut inner: Vec<String> = Vec::new();
            while i < lines.len() {
                let next = &lines[i];
                if quote_line(next) {
                    inner.push(unquote(next));
                } else if !blank(next) && inner.last().is_some_and(|last| !blank(last)) && !starts_block(next) {
                    // a lazy line goes on with the quote's paragraph
                    inner.push(next.trim().to_owned());
                } else {
                    break;
                }
                i += 1;
            }
            let mut read = parse(&inner);
            let mut kind = None;
            if let Some(Block::Paragraph(first)) = read.first_mut()
                && let Some((found, rest)) = callout(first)
            {
                kind = Some(found);
                if rest.is_empty() {
                    read.remove(0);
                } else {
                    *first = rest.to_owned();
                }
            }
            out.push(Block::Quote(kind, read));
            continue;
        }
        if let Some(marker) = list_marker(line)
            // a numbered item interrupts a paragraph only when it counts from 1
            && (paragraph.is_empty() || marker.number.is_none_or(|n| n == 1))
        {
            flush(&mut paragraph, &mut out);
            let mut item: Vec<String> = vec![marker.rest.clone()];
            i += 1;
            while i < lines.len() {
                let next = &lines[i];
                if blank(next) {
                    item.push(String::new());
                } else if indent(next) >= marker.content {
                    item.push(next[marker.content..].to_owned());
                } else if item.last().is_some_and(|last| !blank(last)) && !starts_block(next) {
                    // a lazy line goes on with the item's paragraph
                    item.push(next.trim().to_owned());
                } else {
                    break;
                }
                i += 1;
            }
            while item.len() > 1 && item.last().is_some_and(|last| blank(last)) {
                item.pop();
            }
            let mut task = None;
            let first = item[0].clone();
            for (mark, checked) in [("[ ]", false), ("[x]", true), ("[X]", true)] {
                if let Some(rest) = first.strip_prefix(mark)
                    && rest.starts_with([' ', '\t'])
                    && !rest.trim().is_empty()
                {
                    task = Some(checked);
                    item[0] = rest.trim_start().to_owned();
                }
            }
            let read = ListItem {
                task,
                blocks: parse(&item),
            };
            match out.last_mut() {
                Some(Block::List(kind, items)) if kind.is_some() == marker.number.is_some() => items.push(read),
                _ => out.push(Block::List(marker.number, vec![read])),
            }
            continue;
        }
        paragraph.push(line.trim().to_owned());
        i += 1;
    }
    flush(&mut paragraph, &mut out);
    out
}

/// The id a heading is given under `anchors`: its text lowercased, spaces as dashes, punctuation
/// dropped, as sokkel-ui's `headingId` makes it.
pub fn heading_id(text: &str) -> String {
    let kept: String = text
        .to_lowercase()
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Gives each heading id once: the first heading of a text keeps it, and each later one with the
/// same text takes `-1`, `-2` and so on after it, as GitHub numbers them.
#[derive(Default)]
pub(super) struct UniqueIds(std::collections::HashMap<String, usize>);

impl UniqueIds {
    pub(super) fn take(&mut self, id: String) -> String {
        let mut out = id.clone();
        while self.0.contains_key(&out) {
            let count = self.0.get(&id).copied().unwrap_or(0) + 1;
            self.0.insert(id.clone(), count);
            out = format!("{id}-{count}");
        }
        self.0.insert(out.clone(), 0);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_are_read() {
        let read =
            blocks("### Hand-over\n\nThe person on call\nhands over.\n\n1. Read the log\n2. Retry\n- one\n- two");
        assert_eq!(
            read,
            [
                Block::Heading(3, "Hand-over".into()),
                Block::Paragraph("The person on call hands over.".into()),
                Block::List(Some(1), vec![ListItem::text("Read the log"), ListItem::text("Retry")]),
                Block::List(None, vec![ListItem::text("one"), ListItem::text("two")]),
            ]
        );
    }

    #[test]
    fn an_indented_line_goes_on_with_its_item() {
        let read = blocks("- Each job has a note: [[nightly-export]]\n  and [[import-orders]].\n- The projects");
        assert_eq!(
            read,
            [Block::List(
                None,
                vec![
                    ListItem::text("Each job has a note: [[nightly-export]] and [[import-orders]]."),
                    ListItem::text("The projects"),
                ]
            )]
        );
    }

    #[test]
    fn tasks_nested_lists_quotes_code_and_rules_are_read() {
        let read = blocks(
            "- [x] Read the log\n- [ ] Ask linus\n  - first\n- [ ]\n\n> [!WARNING]\n> Never retry twice.\n\n> Said once\n\n```sh\nretry --step validate\n```\n\n---\nUnder\n===",
        );
        assert_eq!(
            read,
            [
                Block::List(
                    None,
                    vec![
                        ListItem {
                            task: Some(true),
                            blocks: vec![Block::Paragraph("Read the log".into())]
                        },
                        ListItem {
                            task: Some(false),
                            blocks: vec![
                                Block::Paragraph("Ask linus".into()),
                                Block::List(None, vec![ListItem::text("first")])
                            ]
                        },
                        ListItem::text("[ ]"),
                    ]
                ),
                Block::Quote(
                    Some(Callout::Warning),
                    vec![Block::Paragraph("Never retry twice.".into())]
                ),
                Block::Quote(None, vec![Block::Paragraph("Said once".into())]),
                Block::Code("sh".into(), "retry --step validate".into()),
                Block::Rule,
                Block::Heading(1, "Under".into()),
            ]
        );
    }

    #[test]
    fn heading_ids_are_made_once() {
        let mut ids = UniqueIds::default();
        assert_eq!(ids.take(heading_id("Start here!")), "start-here");
        assert_eq!(ids.take(heading_id("Start  here")), "start-here-1");
        assert_eq!(ids.take(heading_id("Start here")), "start-here-2");
    }
}
