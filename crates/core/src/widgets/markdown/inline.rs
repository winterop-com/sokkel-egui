//! The runs of a sentence: bold, italic, struck and code, links written `[words](address)`,
//! `<address>` or bare, and, when asked, `[[links]]` between notes and `#tags`.

/// A run of a sentence and how it is set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    /// Where the run goes when pressed, if it is a link or a tag.
    pub link: Option<RunLink>,
}

/// Where a link goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunLink {
    /// A `[[link]]` to a note: the note it names, and the heading after its `#`.
    Note { target: String, heading: Option<String> },
    /// A `#tag`, named without its `#`.
    Tag(String),
    /// A link to an address.
    Url(String),
}

/// What the reader takes as links besides markdown's own.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Reading {
    /// `[[links]]` between notes.
    pub notes: bool,
    /// `#tags`.
    pub tags: bool,
}

/// The runs of a sentence: `**bold**`, `*italic*` or `_italic_`, `~~struck~~`, `` `code` ``, and
/// links written `[words](address)`, `<address>` or as a bare `https://` or `www.` address; a mark
/// left open is read as written.
pub fn runs(text: &str) -> Vec<Run> {
    read(text, Reading::default())
}

/// The schemes a link may have to be live, as react-markdown keeps them; any other is its words.
fn allowed(href: &str) -> bool {
    let bare: String = href.chars().filter(|c| !c.is_control() && *c != ' ').collect();
    if bare.is_empty() {
        return false;
    }
    if bare.starts_with("//") {
        return true;
    }
    let scheme = bare.split_once(':').map(|(s, _)| s).filter(|s| {
        s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && s.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
    });
    match scheme {
        None => true,
        Some(s) => ["http", "https", "irc", "ircs", "mailto", "xmpp"].contains(&s.to_ascii_lowercase().as_str()),
    }
}

/// The marks that may stand before a `#tag`.
fn opens_tag(c: char) -> bool {
    c.is_whitespace() || "([{\"'\u{201c}\u{2018}".contains(c)
}

fn tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '/' | '-')
}

/// The marks that may stand before a bare address.
fn opens_address(c: char) -> bool {
    c.is_whitespace() || "*_~(".contains(c)
}

pub(super) fn read(text: &str, reading: Reading) -> Vec<Run> {
    let chars: Vec<char> = text.chars().collect();
    let mut reader = Reader {
        chars: &chars,
        out: Vec::new(),
        current: String::new(),
        bold: false,
        italic: false,
        strike: false,
    };
    reader.read(reading);
    reader.out
}

struct Reader<'a> {
    chars: &'a [char],
    out: Vec<Run>,
    current: String,
    bold: bool,
    italic: bool,
    strike: bool,
}

impl Reader<'_> {
    fn push(&mut self) {
        if !self.current.is_empty() {
            self.out.push(Run {
                text: std::mem::take(&mut self.current),
                bold: self.bold,
                italic: self.italic,
                strike: self.strike,
                code: false,
                link: None,
            });
        }
    }

    fn closes(&self, from: usize, mark: &str) -> bool {
        let rest: String = self.chars[from.min(self.chars.len())..].iter().collect();
        rest.contains(mark)
    }

    fn starts(&self, at: usize, word: &str) -> bool {
        word.chars()
            .enumerate()
            .all(|(k, w)| self.chars.get(at + k).map(|c| c.to_ascii_lowercase()) == Some(w))
    }

    /// A link's runs in the current marks, its words read as a sentence of their own; without a
    /// link, the words alone.
    fn link(&mut self, words: &str, link: Option<RunLink>) {
        self.push();
        for mut run in read(words, Reading::default()) {
            run.bold |= self.bold;
            run.italic |= self.italic;
            run.strike |= self.strike;
            run.link = link.clone();
            self.out.push(run);
        }
    }

    fn read(&mut self, reading: Reading) {
        let chars = self.chars;
        let n = chars.len();
        let mut i = 0;
        while i < n {
            let c = chars[i];
            // an escaped mark is its character
            if c == '\\' && chars.get(i + 1).is_some_and(|next| next.is_ascii_punctuation()) {
                self.current.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '`' {
                let ticks = chars[i..].iter().take_while(|c| **c == '`').count();
                let mut close = None;
                let mut j = i + ticks;
                while j < n {
                    if chars[j] == '`' {
                        let run = chars[j..].iter().take_while(|c| **c == '`').count();
                        if run == ticks {
                            close = Some(j);
                            break;
                        }
                        j += run;
                    } else {
                        j += 1;
                    }
                }
                if let Some(close) = close {
                    self.push();
                    let mut code: String = chars[i + ticks..close].iter().collect();
                    if code.len() > 2 && code.starts_with(' ') && code.ends_with(' ') && !code.trim().is_empty() {
                        code = code[1..code.len() - 1].to_owned();
                    }
                    self.out.push(Run {
                        text: code,
                        bold: false,
                        italic: false,
                        strike: false,
                        code: true,
                        link: None,
                    });
                    i = close + ticks;
                } else {
                    self.current.extend(std::iter::repeat_n('`', ticks));
                    i += ticks;
                }
                continue;
            }
            if reading.notes && c == '[' && chars.get(i + 1) == Some(&'[') {
                let end = (i + 2..n.saturating_sub(1)).find(|j| chars[*j] == ']' && chars[*j + 1] == ']');
                let inside: Option<String> = end.map(|end| chars[i + 2..end].iter().collect());
                if let (Some(end), Some(inside)) = (end, inside)
                    && !inside.contains(['[', ']', '\n'])
                {
                    let (name, label) = match inside.split_once('|') {
                        Some((name, label)) => (name, Some(label.trim()).filter(|l| !l.is_empty())),
                        None => (inside.as_str(), None),
                    };
                    let (target, heading) = match name.split_once('#') {
                        Some((target, heading)) => (target.trim(), Some(heading.trim()).filter(|h| !h.is_empty())),
                        None => (name.trim(), None),
                    };
                    if !target.is_empty() {
                        let words = match (label, heading) {
                            (Some(label), _) => label.to_owned(),
                            (None, Some(heading)) => format!("{target} \u{203a} {heading}"),
                            (None, None) => target.to_owned(),
                        };
                        self.push();
                        self.out.push(Run {
                            text: words,
                            bold: self.bold,
                            italic: self.italic,
                            strike: self.strike,
                            code: false,
                            link: Some(RunLink::Note {
                                target: target.to_owned(),
                                heading: heading.map(str::to_owned),
                            }),
                        });
                        i = end + 2;
                        continue;
                    }
                }
            }
            if c == '[' {
                // `[words](address)`: the words up to the matching `]`, the address up to `)`
                let mut depth = 0;
                let mut close = None;
                for (j, ch) in chars.iter().enumerate().skip(i) {
                    match ch {
                        '[' => depth += 1,
                        ']' => {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(j);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(close) = close
                    && chars.get(close + 1) == Some(&'(')
                    && let Some(end) = (close + 2..n).find(|j| chars[*j] == ')')
                {
                    let words: String = chars[i + 1..close].iter().collect();
                    let inside: String = chars[close + 2..end].iter().collect();
                    let inside = inside.trim();
                    // a title after the address is left out
                    let address = inside.split_once([' ', '\t']).map_or(inside, |(a, _)| a);
                    let address = address.trim_start_matches('<').trim_end_matches('>');
                    let link = allowed(address).then(|| RunLink::Url(address.to_owned()));
                    self.link(&words, link);
                    i = end + 1;
                    continue;
                }
            }
            if c == '<'
                && let Some(end) = (i + 1..n).find(|j| chars[*j] == '>')
            {
                let inside: String = chars[i + 1..end].iter().collect();
                let scheme = inside.split_once(':').map(|(s, _)| s);
                if !inside.contains([' ', '<'])
                    && scheme.is_some_and(|s| {
                        s.len() >= 2 && s.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
                    })
                    && allowed(&inside)
                {
                    self.link(&inside.clone(), Some(RunLink::Url(inside)));
                    i = end + 1;
                    continue;
                }
            }
            if (i == 0 || opens_address(chars[i - 1]))
                && (self.starts(i, "https://") || self.starts(i, "http://") || self.starts(i, "www."))
            {
                let mut end = i;
                while end < n && !chars[end].is_whitespace() && chars[end] != '<' {
                    end += 1;
                }
                // trailing marks end the sentence, not the address; a `)` only when unmatched
                while end > i {
                    let last = chars[end - 1];
                    let opened = chars[i..end].iter().filter(|c| **c == '(').count();
                    let closed = chars[i..end].iter().filter(|c| **c == ')').count();
                    if "?!.,:*_~'\";".contains(last) || (last == ')' && closed > opened) {
                        end -= 1;
                    } else {
                        break;
                    }
                }
                let words: String = chars[i..end].iter().collect();
                let host = words.trim_start_matches("https://").trim_start_matches("http://");
                if host.contains('.') && !host.starts_with('.') {
                    let address = if words.to_ascii_lowercase().starts_with("www.") {
                        format!("http://{words}")
                    } else {
                        words.clone()
                    };
                    // the words are the address as written, marks and all
                    self.push();
                    self.out.push(Run {
                        text: words,
                        bold: self.bold,
                        italic: self.italic,
                        strike: self.strike,
                        code: false,
                        link: Some(RunLink::Url(address)),
                    });
                    i = end;
                    continue;
                }
            }
            if reading.tags && c == '#' && (i == 0 || opens_tag(chars[i - 1])) {
                let len = chars[i + 1..].iter().take_while(|c| tag_char(**c)).count();
                let tag: String = chars[i + 1..i + 1 + len].iter().collect();
                if tag.chars().any(|c| c.is_alphabetic() || c == '_') {
                    self.push();
                    self.out.push(Run {
                        text: format!("#{tag}"),
                        bold: self.bold,
                        italic: self.italic,
                        strike: self.strike,
                        code: false,
                        link: Some(RunLink::Tag(tag)),
                    });
                    i += 1 + len;
                    continue;
                }
            }
            if c == '~' && chars.get(i + 1) == Some(&'~') && (self.strike || self.closes(i + 2, "~~")) {
                self.push();
                self.strike = !self.strike;
                i += 2;
                continue;
            }
            if c == '*' && chars.get(i + 1) == Some(&'*') && (self.bold || self.closes(i + 2, "**")) {
                self.push();
                self.bold = !self.bold;
                i += 2;
                continue;
            }
            if c == '*' && chars.get(i + 1) == Some(&'*') {
                // a pair of stars that opens nothing is read as written
                self.current.push_str("**");
                i += 2;
                continue;
            }
            if (c == '*' || c == '_') && (self.italic || self.closes(i + 1, &c.to_string())) {
                // an underscore inside a word is a word's, not a mark
                let inside = c == '_'
                    && i > 0
                    && chars[i - 1].is_alphanumeric()
                    && chars.get(i + 1).is_some_and(|n| n.is_alphanumeric());
                if !inside {
                    self.push();
                    self.italic = !self.italic;
                    i += 1;
                    continue;
                }
            }
            self.current.push(c);
            i += 1;
        }
        self.push();
    }
}

/// A run's text cut where `needle` stands, in any case: each piece and whether it is a match.
pub(super) fn marks(text: &str, needle: &str) -> Vec<(String, bool)> {
    let needle: Vec<char> = needle.trim().chars().flat_map(char::to_lowercase).collect();
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = chars.iter().map(|c| c.to_lowercase().next().unwrap_or(*c)).collect();
    if needle.is_empty() || needle.len() > chars.len() {
        return vec![(text.to_owned(), false)];
    }
    let mut out = Vec::new();
    let mut at = 0;
    let mut i = 0;
    while i + needle.len() <= chars.len() {
        if lower[i..i + needle.len()] == needle[..] {
            if i > at {
                out.push((chars[at..i].iter().collect(), false));
            }
            out.push((chars[i..i + needle.len()].iter().collect(), true));
            i += needle.len();
            at = i;
        } else {
            i += 1;
        }
    }
    if at < chars.len() {
        out.push((chars[at..].iter().collect(), false));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(read: &[Run]) -> Vec<(&str, bool, bool, bool)> {
        read.iter()
            .map(|r| (r.text.as_str(), r.bold, r.italic, r.code))
            .collect()
    }

    #[test]
    fn runs_are_read() {
        let read = runs("Press **Edit** to move `nightly-export`, *now*; a sync_inventory stays, ** stays");
        assert_eq!(
            shape(&read),
            [
                ("Press ", false, false, false),
                ("Edit", true, false, false),
                (" to move ", false, false, false),
                ("nightly-export", false, false, true),
                (", ", false, false, false),
                ("now", false, true, false),
                ("; a sync_inventory stays, ** stays", false, false, false),
            ]
        );
    }

    #[test]
    fn links_notes_and_tags_are_read() {
        let read = super::read(
            "See [[Retry a run]], [[Runbooks#Steps]], [[harbor|the harbor]], [the **docs**](https://example.org/docs \"Docs\"), https://example.org/runs. and #welcome, not `[[code]]` nor #1",
            Reading {
                notes: true,
                tags: true,
            },
        );
        let links: Vec<(&str, Option<&RunLink>)> = read
            .iter()
            .filter(|r| r.link.is_some())
            .map(|r| (r.text.as_str(), r.link.as_ref()))
            .collect();
        assert_eq!(
            links,
            [
                (
                    "Retry a run",
                    Some(&RunLink::Note {
                        target: "Retry a run".into(),
                        heading: None
                    })
                ),
                (
                    "Runbooks \u{203a} Steps",
                    Some(&RunLink::Note {
                        target: "Runbooks".into(),
                        heading: Some("Steps".into())
                    })
                ),
                (
                    "the harbor",
                    Some(&RunLink::Note {
                        target: "harbor".into(),
                        heading: None
                    })
                ),
                ("the ", Some(&RunLink::Url("https://example.org/docs".into()))),
                ("docs", Some(&RunLink::Url("https://example.org/docs".into()))),
                (
                    "https://example.org/runs",
                    Some(&RunLink::Url("https://example.org/runs".into()))
                ),
                ("#welcome", Some(&RunLink::Tag("welcome".into()))),
            ]
        );
        assert!(read.iter().any(|r| r.code && r.text == "[[code]]"));
        assert!(read.iter().any(|r| r.text.contains("#1")));
    }

    #[test]
    fn a_link_to_a_script_is_its_words() {
        let read = runs("[run](javascript:alert(1))");
        assert!(read.iter().all(|r| r.link.is_none()));
    }

    #[test]
    fn marks_are_found_in_any_case() {
        assert_eq!(
            marks("Started Start here.", "start here"),
            [
                ("Started ".to_owned(), false),
                ("Start here".to_owned(), true),
                (".".to_owned(), false)
            ]
        );
    }
}
