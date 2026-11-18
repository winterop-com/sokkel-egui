//! Links between notes and tags on them, as sokkel-ui's `lib/wiki-links.ts` reads them: `[[Note]]`
//! links to the note of that name, `[[Note|the words shown]]` shows other words, `[[Note#Heading]]`
//! points at a heading in it, and `#tag` tags the note. Neither is read inside code.
//!
//! Every place in a text is a byte offset into it, where the web counts UTF-16 units: the two agree
//! wherever the text is ASCII.

use std::ops::Range;

use crate::model::{is_letter, is_number, is_space, lowercase, trim};

/// One `[[link]]` in a text: the note it names, the heading after `#`, the words shown after `|`,
/// and where it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WikiLink {
    /// The note's name, trimmed: `Nightly export` in `[[Nightly export#Steps|the steps]]`.
    pub target: String,
    pub heading: Option<String>,
    /// The words drawn in the link's place, when they are not the name.
    pub label: Option<String>,
    /// Where `[[` starts in the text.
    pub from: usize,
    /// Where the text goes on after `]]`.
    pub to: usize,
}

/// The inside of one link, `Note#Heading|words`, read into its parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkName {
    pub target: String,
    pub heading: Option<String>,
    pub label: Option<String>,
}

/// One `#tag` in a text: the tag without its `#`, and where the `#` stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagMatch {
    pub tag: String,
    pub from: usize,
    pub to: usize,
}

/// The `[[link]]` being typed at the caret: where its name starts and what is typed of it so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkQuery {
    pub start: usize,
    pub query: String,
}

/// The text with a range written over in spaces, one for each byte, so every offset still holds.
fn blank(text: &str) -> String {
    " ".repeat(text.len())
}

/// The fence a line opens or closes, ```` ``` ```` or `~~~` and longer, after at most three spaces.
fn fence_marker(line: &str) -> Option<&str> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let mark = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let run = rest.len() - rest.trim_start_matches(mark).len();
    (run >= 3).then(|| &rest[..run])
}

/// The text with its code blanked out, fenced blocks and spans between backticks on one line, so
/// nothing in it is read and every offset still holds.
pub fn without_code(text: &str) -> String {
    let mut fence: Option<String> = None;
    let lines: Vec<String> = text
        .split('\n')
        .map(|line| {
            let marker = fence_marker(line);
            match (&fence, marker) {
                (None, Some(marker)) => {
                    fence = Some(marker.to_owned());
                    blank(line)
                }
                (Some(open), _) => {
                    if let Some(marker) = marker
                        && marker.as_bytes()[0] == open.as_bytes()[0]
                        && marker.len() >= open.len()
                        && trim(line) == marker
                    {
                        fence = None;
                    }
                    blank(line)
                }
                (None, None) => without_spans(line),
            }
        })
        .collect();
    lines.join("\n")
}

/// A line with each span between backticks blanked: a run of backticks, text with none, and as
/// many again, the longer run's extra backticks left outside the span.
fn without_spans(line: &str) -> String {
    let bytes = line.as_bytes();
    let find = |from: usize| {
        bytes[from.min(bytes.len())..]
            .iter()
            .position(|b| *b == b'`')
            .map(|at| at + from)
    };
    let mut out = String::new();
    let mut at = 0;
    let mut open = find(0);
    while let Some(start) = open {
        let mut opened = start;
        while bytes.get(opened) == Some(&b'`') {
            opened += 1;
        }
        let Some(close) = find(opened) else { break };
        let mut closed = close;
        while bytes.get(closed) == Some(&b'`') {
            closed += 1;
        }
        let ticks = (opened - start).min(closed - close);
        out.push_str(&line[at..opened - ticks]);
        out.push_str(&blank(&line[opened - ticks..close + ticks]));
        at = close + ticks;
        open = find(at);
    }
    out.push_str(&line[at..]);
    out
}

/// One link's inside, `Note#Heading|words`, read into its parts.
pub fn read_wiki_link(inside: &str) -> LinkName {
    let (name, label) = match inside.find('|') {
        Some(bar) => (&inside[..bar], Some(trim(&inside[bar + 1..]))),
        None => (inside, None),
    };
    let (target, heading) = match name.find('#') {
        Some(hash) => (trim(&name[..hash]), Some(trim(&name[hash + 1..]))),
        None => (trim(name), None),
    };
    LinkName {
        target: target.to_owned(),
        heading: heading.filter(|one| !one.is_empty()).map(str::to_owned),
        label: label.filter(|one| !one.is_empty()).map(str::to_owned),
    }
}

/// Every `[[...]]` in a text, as the pattern `\[\[([^[\]\n]+?)\]\]` finds them: the range of each.
fn link_ranges(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at + 1 < bytes.len() {
        if bytes[at] == b'[' && bytes[at + 1] == b'[' {
            let inside = at + 2;
            let end = bytes[inside..]
                .iter()
                .position(|b| matches!(b, b'[' | b']' | b'\n'))
                .map_or(bytes.len(), |n| n + inside);
            if end > inside && bytes.get(end) == Some(&b']') && bytes.get(end + 1) == Some(&b']') {
                found.push(at..end + 2);
                at = end + 2;
                continue;
            }
        }
        at += 1;
    }
    found
}

/// The text with every `[[link]]` written over in spaces.
fn without_links(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for range in link_ranges(text) {
        out.push_str(&text[at..range.start]);
        out.push_str(&blank(&text[range.clone()]));
        at = range.end;
    }
    out.push_str(&text[at..]);
    out
}

/// Every `[[link]]` in a text outside its code, in order, a link to nowhere (`[[#Heading]]`) left
/// out.
pub fn wiki_links_in(text: &str) -> Vec<WikiLink> {
    link_ranges(&without_code(text))
        .into_iter()
        .filter_map(|range| {
            let read = read_wiki_link(&text[range.start + 2..range.end - 2]);
            (!read.target.is_empty()).then_some(WikiLink {
                target: read.target,
                heading: read.heading,
                label: read.label,
                from: range.start,
                to: range.end,
            })
        })
        .collect()
}

/// Whether a character may open a tag: a space or an opening mark.
fn opens_tag(c: char) -> bool {
    is_space(c) || matches!(c, '(' | '[' | '{' | '"' | '\'' | '\u{201c}' | '\u{2018}')
}

/// Whether a character may stand in a tag.
fn in_tag(c: char) -> bool {
    is_letter(c) || is_number(c) || matches!(c, '_' | '/' | '-')
}

/// Every `#tag` in a text outside its code and its links, in order. A heading's `# ` is not a tag,
/// nor is `#1`.
pub fn tag_matches(text: &str) -> Vec<TagMatch> {
    let plain = without_links(&without_code(text));
    let mut found = Vec::new();
    // The tag after a `#` at `hash`: the run of tag characters, if one of them is a letter or `_`.
    let tag_at = |hash: usize| -> Option<&str> {
        let rest = plain.get(hash + 1..)?;
        let run = rest.find(|c: char| !in_tag(c)).unwrap_or(rest.len());
        let tag = &rest[..run];
        tag.chars().any(|c| is_letter(c) || c == '_').then_some(tag)
    };
    let mut at = 0;
    while at < plain.len() {
        let c = plain[at..].chars().next().unwrap_or(' ');
        let hash = if at == 0 && c == '#' {
            Some(at)
        } else if opens_tag(c) && plain[at + c.len_utf8()..].starts_with('#') {
            Some(at + c.len_utf8())
        } else {
            None
        };
        if let Some(tag) = hash.and_then(|hash| tag_at(hash).map(|tag| (hash, tag))) {
            let (from, tag) = tag;
            found.push(TagMatch {
                tag: tag.to_owned(),
                from,
                to: from + 1 + tag.len(),
            });
            at = from + 1 + tag.len();
            continue;
        }
        at += c.len_utf8();
    }
    found
}

/// The tags of a text, each once, in the order they first appear, without their `#`.
pub fn tags_in(text: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for found in tag_matches(text) {
        if !tags.contains(&found.tag) {
            tags.push(found.tag);
        }
    }
    tags
}

/// The places a name is written in a text without being linked: whole words, in any case, outside
/// code and outside `[[links]]`, for the mentions a note could turn into links.
pub fn mentions_of(text: &str, name: &str) -> Vec<Range<usize>> {
    let needle = lowercase(trim(name));
    if needle.is_empty() {
        return Vec::new();
    }
    let plain = lowercase(&without_links(&without_code(text)));
    let word = |c: Option<char>| c.is_some_and(|c| is_letter(c) || is_number(c) || c == '_');
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = plain
        .get(from..)
        .and_then(|rest| rest.find(&needle))
        .map(|at| at + from)
    {
        let end = at + needle.len();
        if !word(plain[..at].chars().next_back()) && !word(plain[end..].chars().next()) {
            found.push(at..end);
        }
        from = end;
    }
    found
}

/// The `[[link]]` being typed at the caret, if any: where its name starts and what is typed of it
/// so far.
pub fn wiki_link_query(text: &str, caret: usize) -> Option<LinkQuery> {
    let caret = caret.min(text.len());
    // As `lastIndexOf('\n', caret - 1)`: the last line end at or before the place before the caret.
    let upto = (caret.max(1)).min(text.len());
    let start = text
        .get(..upto)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |at| at + 1);
    let line = text.get(start..caret).unwrap_or("");
    let open = line.rfind("[[")?;
    let query = &line[open + 2..];
    if query.contains(['[', ']']) {
        return None;
    }
    Some(LinkQuery {
        start: caret - query.len(),
        query: query.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(target: &str, heading: Option<&str>, label: Option<&str>, from: usize, to: usize) -> WikiLink {
        WikiLink {
            target: target.to_owned(),
            heading: heading.map(str::to_owned),
            label: label.map(str::to_owned),
            from,
            to,
        }
    }

    #[test]
    fn links_read_as_the_web_reads_them() {
        assert_eq!(
            wiki_links_in("See [[Retry a run]] and [[nightly-export#Steps|the steps]]."),
            [
                link("Retry a run", None, None, 4, 19),
                link("nightly-export", Some("Steps"), Some("the steps"), 24, 58)
            ]
        );
        assert_eq!(
            wiki_links_in("[[ Spaced  ]] [[#Heading only]] [[a|]] [[b#]] [[c#h|w]]"),
            [
                link("Spaced", None, None, 0, 13),
                link("a", None, None, 32, 38),
                link("b", None, None, 39, 45),
                link("c", Some("h"), Some("w"), 46, 55)
            ]
        );
        assert_eq!(
            wiki_links_in("[[[nested]]] [[one\ntwo]] [[x]y]] [[]] [[ok]]"),
            [link("nested", None, None, 1, 11), link("ok", None, None, 38, 44)]
        );
        assert_eq!(
            wiki_links_in("Code `[[not a link]]` and ``a `[[x]]` b`` then [[yes]]"),
            [link("x", None, None, 31, 36), link("yes", None, None, 47, 54)]
        );
        assert_eq!(
            wiki_links_in("```\n[[fenced]]\n```\n[[after]]\n~~~~\n[[tilde]]\n~~~\n~~~~\n[[out]]"),
            [link("after", None, None, 19, 28), link("out", None, None, 53, 60)]
        );
    }

    #[test]
    fn tags_read_as_the_web_reads_them() {
        let text = "#tag at start, #two-words #1 #a1 (#paren) \"#quote\" x#no #with/slash #\u{fc}n\u{ef}code #_u [[l #nottag]] `#code`";
        assert_eq!(
            tags_in(text),
            [
                "tag",
                "two-words",
                "a1",
                "paren",
                "quote",
                "with/slash",
                "\u{fc}n\u{ef}code",
                "_u"
            ]
        );
        assert_eq!(tags_in("# Heading\n#tag\n  #indented #a#b"), ["tag", "indented", "a"]);
        let found = tag_matches("x #ab (#c)");
        assert_eq!((found[0].from, found[0].to, found[1].from, found[1].to), (2, 5, 7, 9));
    }

    #[test]
    fn mentions_and_queries_read_as_the_web_reads_them() {
        assert_eq!(
            mentions_of(
                "nightly-export ran; Nightly-Export again; nightly-exports no; [[nightly-export]] linked; `nightly-export` code",
                "nightly-export"
            ),
            [0..14, 20..34]
        );
        assert_eq!(
            mentions_of("grace and graceful, grace_x, (grace).", "Grace"),
            [0..5, 30..35]
        );
        assert_eq!(mentions_of("nothing", ""), []);
        assert_eq!(
            wiki_link_query("see [[Ret", 9),
            Some(LinkQuery {
                start: 6,
                query: "Ret".to_owned()
            })
        );
        assert_eq!(wiki_link_query("a\n[[x]] b", 9), None);
        assert_eq!(
            wiki_link_query("[[a", 3),
            Some(LinkQuery {
                start: 2,
                query: "a".to_owned()
            })
        );
    }
}
