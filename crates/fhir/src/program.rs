//! What FHIRPath and CQL share, as sokkel-ui-fhir's `program.ts` reads them: one grammar, read a
//! line at a time as a CodeMirror stream parser reads it, each language spelling its keywords,
//! operators, punctuation, numbers, quoted names and bound names its own way ([`Syntax`]). Each
//! token is named by the highlight tag the web's editor colors it with: `keyword`, `bool`, `null`,
//! `number` (a date or a time literal too), `string`, `comment`, `operator`, `bracket`,
//! `punctuation`, `variableName.function` for a function, `propertyName` for a member after a dot,
//! `variableName.special` for a name bound outside the program, `invalid`, and `variableName` for
//! every other name.

use sokkel_egui::{CodeTag, CodeToken};

/// Where a line starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgramState {
    /// Inside a block comment carried over from the line before.
    pub comment: bool,
    /// The last token was a `.`, so the next name is a member even when spelled like a keyword.
    pub member: bool,
}

/// How long a token of a kind is at the start of the rest of a line; 0 when there is none.
pub type Matcher = fn(&str) -> usize;

/// What one language spells its own way.
pub struct Syntax {
    pub keywords: &'static [&'static str],
    pub operator: Matcher,
    pub punctuation: Matcher,
    pub number: Matcher,
    /// A name written in quotes.
    pub quoted_name: Matcher,
    /// A name bound outside the program, or none when the language has none.
    pub binding: Option<Matcher>,
}

/// A word character, as a pattern's `\w`.
pub fn word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The length of a name at the start of `s`, `[A-Za-z_]\w*`.
pub fn name_len(s: &str) -> usize {
    match s.chars().next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            1 + s[1..].find(|c: char| !word_char(c)).unwrap_or(s.len() - 1)
        }
        _ => 0,
    }
}

/// How many ASCII digits `s` starts with.
pub fn digits(s: &str) -> usize {
    s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len())
}

/// Where a text in `quote`s ends, `quote` and every escape inside: its length with both quotes,
/// or none when it does not close on this line.
pub fn quoted(s: &str, quote: char) -> Option<usize> {
    match scan_quoted(s, quote) {
        Quoted::Closed(len) => Some(len),
        _ => None,
    }
}

enum Quoted {
    /// Not a quoted text at all.
    None,
    Closed(usize),
    /// Open to the line's end, every escape in it whole.
    Open,
}

fn scan_quoted(s: &str, quote: char) -> Quoted {
    if !s.starts_with(quote) {
        return Quoted::None;
    }
    let mut chars = s.char_indices().skip(1);
    while let Some((at, c)) = chars.next() {
        if c == quote {
            return Quoted::Closed(at + c.len_utf8());
        }
        if c == '\\' && chars.next().is_none() {
            // an escape with nothing after it reads neither as a closed text nor an open one
            return Quoted::None;
        }
    }
    Quoted::Open
}

/// Two digits at `at`.
fn two_digits(b: &[u8], at: usize) -> bool {
    b.get(at).is_some_and(u8::is_ascii_digit) && b.get(at + 1).is_some_and(u8::is_ascii_digit)
}

/// `(:\d{2}(:\d{2}(\.\d+)?)?)?` from `at`: where it ends.
fn time_tail(b: &[u8], mut at: usize) -> usize {
    if b.get(at) == Some(&b':') && two_digits(b, at + 1) {
        at += 3;
        if b.get(at) == Some(&b':') && two_digits(b, at + 1) {
            at += 3;
            if b.get(at) == Some(&b'.') && b.get(at + 1).is_some_and(u8::is_ascii_digit) {
                at += 1;
                while b.get(at).is_some_and(u8::is_ascii_digit) {
                    at += 1;
                }
            }
        }
    }
    at
}

/// A date, a time or a date-time written as a literal, `@2026-01-01T10:00:00Z` or `@T10:00`: its
/// length, or 0.
pub fn temporal_len(s: &str) -> usize {
    let b = s.as_bytes();
    if b.first() != Some(&b'@') {
        return 0;
    }
    if b.get(1) == Some(&b'T') {
        return if two_digits(b, 2) { time_tail(b, 4) } else { 0 };
    }
    if !(two_digits(b, 1) && two_digits(b, 3)) {
        return 0;
    }
    let mut at = 5;
    if b.get(at) == Some(&b'-') && two_digits(b, at + 1) {
        at += 3;
        if b.get(at) == Some(&b'-') && two_digits(b, at + 1) {
            at += 3;
        }
    }
    if b.get(at) == Some(&b'T') {
        at += 1;
        if two_digits(b, at) {
            at = time_tail(b, at + 2);
        }
        if b.get(at) == Some(&b'Z') {
            at += 1;
        } else if b.get(at).is_some_and(|c| *c == b'+' || *c == b'-')
            && two_digits(b, at + 1)
            && b.get(at + 3) == Some(&b':')
            && two_digits(b, at + 4)
        {
            at += 6;
        }
    }
    at
}

/// A cursor over one line, as CodeMirror's `StringStream`.
struct Stream<'a> {
    line: &'a str,
    pos: usize,
}

impl<'a> Stream<'a> {
    fn rest(&self) -> &'a str {
        &self.line[self.pos..]
    }

    fn eol(&self) -> bool {
        self.pos >= self.line.len()
    }

    fn eat_space(&mut self) -> bool {
        let rest = self.rest();
        let n = rest.len() - rest.trim_start().len();
        self.pos += n;
        n > 0
    }

    fn take(&mut self, len: usize) -> bool {
        self.pos += len;
        len > 0
    }

    fn next(&mut self) {
        if let Some(c) = self.rest().chars().next() {
            self.pos += c.len_utf8();
        }
    }
}

/// The rest of a block comment, up to and including its close when it is on this line.
fn block_comment(stream: &mut Stream, state: &mut ProgramState) -> &'static str {
    match stream.rest().find("*/") {
        Some(at) => {
            stream.pos += at + 2;
            state.comment = false;
        }
        None => stream.pos = stream.line.len(),
    }
    "comment"
}

/// What a word is: a member after a dot, a function before a parenthesis, else a keyword or a name.
fn word(stream: &Stream, member: bool, text: &str, keywords: &[&str]) -> &'static str {
    let called = stream.rest().trim_start().starts_with('(');
    if member {
        return if called {
            "variableName.function"
        } else {
            "propertyName"
        };
    }
    if keywords.contains(&text) {
        return match text {
            "true" | "false" => "bool",
            "null" => "null",
            _ => "keyword",
        };
    }
    if called {
        "variableName.function"
    } else {
        "variableName"
    }
}

/// One token of a program written in a language of this shape; none for the spaces between.
fn program_token(stream: &mut Stream, state: &mut ProgramState, syntax: &Syntax) -> Option<&'static str> {
    if state.comment {
        return Some(block_comment(stream, state));
    }
    if stream.eat_space() {
        return None;
    }
    let member = state.member;
    state.member = false;
    let rest = stream.rest();
    if rest.starts_with("//") {
        stream.pos = stream.line.len();
        return Some("comment");
    }
    if stream.take(usize::from(rest.starts_with("/*")) * 2) {
        state.comment = true;
        return Some(block_comment(stream, state));
    }
    match scan_quoted(rest, '\'') {
        Quoted::Closed(len) => {
            stream.pos += len;
            return Some("string");
        }
        Quoted::Open => {
            stream.pos = stream.line.len();
            return Some("invalid");
        }
        Quoted::None => {}
    }
    if stream.take((syntax.quoted_name)(rest)) {
        return Some("variableName");
    }
    if let Some(binding) = syntax.binding
        && stream.take(binding(rest))
    {
        return Some("variableName.special");
    }
    if stream.take(temporal_len(rest)) {
        return Some("number");
    }
    let name = name_len(rest);
    if name > 0 {
        stream.pos += name;
        return Some(word(stream, member, &rest[..name], syntax.keywords));
    }
    if stream.take((syntax.number)(rest)) {
        return Some("number");
    }
    if stream.take(usize::from(rest.starts_with(['{', '}', '(', ')', '[', ']']))) {
        return Some("bracket");
    }
    if stream.take((syntax.operator)(rest)) {
        return Some("operator");
    }
    if stream.take(usize::from(rest.starts_with('.'))) {
        state.member = true;
        return Some("punctuation");
    }
    if stream.take((syntax.punctuation)(rest)) {
        return Some("punctuation");
    }
    stream.next();
    Some("invalid")
}

/// Every token of a document in order, as the web's parser names them: byte ranges and tags,
/// whitespace left out.
pub fn read(text: &str, syntax: &Syntax) -> Vec<(usize, usize, &'static str)> {
    let mut state = ProgramState::default();
    let mut out = Vec::new();
    let mut offset = 0;
    for line in text.split('\n') {
        let mut stream = Stream { line, pos: 0 };
        while !stream.eol() {
            let start = stream.pos;
            let tag = program_token(&mut stream, &mut state, syntax);
            if stream.pos == start {
                stream.next();
            }
            if let Some(tag) = tag {
                out.push((offset + start, offset + stream.pos, tag));
            }
        }
        offset += line.len() + 1;
    }
    out
}

/// The editor's tag for a web tag; none for a plain name, drawn in the text's ink.
pub fn code_tag(tag: &str) -> Option<CodeTag> {
    Some(match tag {
        "variableName.special" => CodeTag::Variable,
        "variableName.function" => CodeTag::Function,
        "propertyName" => CodeTag::Property,
        "keyword" => CodeTag::Keyword,
        "bool" | "null" => CodeTag::Constant,
        "number" => CodeTag::Number,
        "string" => CodeTag::String,
        "comment" => CodeTag::Comment,
        "operator" => CodeTag::Operator,
        "bracket" | "punctuation" => CodeTag::Punctuation,
        "invalid" => CodeTag::Invalid,
        _ => return None,
    })
}

/// A document's tokens as the code editor colors them.
pub fn code_tokens(text: &str, syntax: &Syntax) -> Vec<CodeToken> {
    read(text, syntax)
        .into_iter()
        .filter_map(|(from, to, tag)| code_tag(tag).map(|kind| CodeToken { from, to, kind }))
        .collect()
}
