//! What DHIS2's two expression languages share, as sokkel-ui-dhis2's `tokens.ts` reads them: one
//! grammar, read a line at a time as a CodeMirror stream parser reads it. Each token is named by
//! the highlight tag the web's editor colors it with: `variableName.special` for a data item in
//! braces (`#{uid.uid}`, `A{name}`, `V{current_date}`) and for `[days]`, `variableName.function` for
//! a known function, `keyword` for `and`, `or`, `not`, `in`, `distinct` and the `deGroup:` tags,
//! `atom` for a wildcard, a reporting-rate metric and an upper-case word such as `SUM`, `bool`,
//! `null`, `number` (a date such as `2026-6-30` too), `string`, `comment`, `operator`, `bracket`,
//! `punctuation`, `invalid`, and `variableName` for every other name, a function or variable the
//! language was not given among them.

use std::collections::HashSet;

use sokkel_egui::{CodeTag, CodeToken};

/// Where a line starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExpressionState {
    /// Inside a block comment carried over from the line before.
    pub comment: bool,
    /// Inside a data item's braces: the letters before its `{`, or none outside one.
    pub item: Option<String>,
    /// The last token was a `.` after a value, so the next name may be a function written after it.
    pub member: bool,
}

/// The names one reading knows.
pub struct Known {
    functions: HashSet<String>,
    variables: HashSet<String>,
}

impl Known {
    pub fn new<F: AsRef<str>, V: AsRef<str>>(
        functions: impl IntoIterator<Item = F>,
        variables: impl IntoIterator<Item = V>,
    ) -> Self {
        Self {
            functions: functions.into_iter().map(|one| one.as_ref().to_owned()).collect(),
            variables: variables.into_iter().map(|one| one.as_ref().to_owned()).collect(),
        }
    }
}

const KEYWORDS: [&str; 5] = ["and", "or", "not", "in", "distinct"];

/// The counts a reporting rate item `R{dataSet.metric}` reads.
const METRICS: [&str; 5] = [
    "REPORTING_RATE",
    "REPORTING_RATE_ON_TIME",
    "ACTUAL_REPORTS",
    "ACTUAL_REPORTS_ON_TIME",
    "EXPECTED_REPORTS",
];

fn word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// The length of a name at the start of `s`: a letter or `_`, then word characters.
fn name_len(s: &str) -> usize {
    match s.chars().next() {
        Some(c) if name_start(c) => 1 + s[1..].find(|c: char| !word_char(c)).unwrap_or(s.len() - 1),
        _ => 0,
    }
}

fn digits(s: &str) -> usize {
    s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len())
}

/// A number: `12`, `1.5`, `.5`, `1e3`.
fn number_len(s: &str) -> usize {
    let mut at = digits(s);
    if at > 0 {
        if s[at..].starts_with('.') {
            at += 1 + digits(&s[at + 1..]);
        }
    } else if s.starts_with('.') && digits(&s[1..]) > 0 {
        at = 1 + digits(&s[1..]);
    } else {
        return 0;
    }
    let rest = &s[at..];
    if rest.starts_with(['e', 'E']) {
        let sign = usize::from(rest[1..].starts_with(['+', '-']));
        let exp = digits(&rest[1 + sign..]);
        if exp > 0 {
            at += 1 + sign + exp;
        }
    }
    at
}

/// A date as `.minDate` and `.maxDate` take it, `2026-1-1`, not followed by a digit or a dot.
fn date_len(s: &str) -> usize {
    let b = s.as_bytes();
    if b.len() < 5 || !(b'1'..=b'9').contains(&b[0]) || !b[1..4].iter().all(u8::is_ascii_digit) || b[4] != b'-' {
        return 0;
    }
    let mut at = 5;
    // [01]?\d
    let month = |at: usize| -> Option<usize> {
        if b.get(at).is_some_and(|c| *c == b'0' || *c == b'1') && b.get(at + 1).is_some_and(u8::is_ascii_digit) {
            Some(at + 2)
        } else if b.get(at).is_some_and(u8::is_ascii_digit) {
            Some(at + 1)
        } else {
            None
        }
    };
    let Some(after) = month(at) else { return 0 };
    at = after;
    if b.get(at) != Some(&b'-') {
        return 0;
    }
    at += 1;
    let day = |at: usize| -> Vec<usize> {
        let mut ends = Vec::new();
        if b.get(at).is_some_and(|c| (b'0'..=b'3').contains(c)) && b.get(at + 1).is_some_and(u8::is_ascii_digit) {
            ends.push(at + 2);
        }
        if b.get(at).is_some_and(u8::is_ascii_digit) {
            ends.push(at + 1);
        }
        ends
    };
    // the longest day not followed by a digit or a dot, as the pattern backtracks to
    for end in day(at) {
        if !b.get(end).is_some_and(|c| c.is_ascii_digit() || *c == b'.') {
            return end;
        }
    }
    0
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
        let n = self.rest().len() - self.rest().trim_start().len();
        self.pos += n;
        n > 0
    }
    fn eat(&mut self, c: char) -> bool {
        if self.rest().starts_with(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }
    fn eat_str(&mut self, s: &str) -> bool {
        if self.rest().starts_with(s) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }
    fn next(&mut self) {
        if let Some(c) = self.rest().chars().next() {
            self.pos += c.len_utf8();
        }
    }
    /// Whether a `(` follows, after spaces, without moving.
    fn called(&self) -> bool {
        self.rest().trim_start().starts_with('(')
    }
}

/// The rest of a block comment, up to and including its close when it is on this line.
fn block_comment(stream: &mut Stream, state: &mut ExpressionState) -> &'static str {
    match stream.rest().find("*/") {
        Some(at) => {
            stream.pos += at + 2;
            state.comment = false;
        }
        None => stream.pos = stream.line.len(),
    }
    "comment"
}

/// One token inside a data item's braces.
fn item_token(stream: &mut Stream, state: &mut ExpressionState, known: &Known) -> Option<&'static str> {
    if stream.eat_space() {
        return None;
    }
    if stream.eat('}') {
        state.item = None;
        return Some("variableName.special");
    }
    if stream.eat_str("deGroup:") || stream.eat_str("coGroup:") || stream.eat_str("co:") {
        return Some("keyword");
    }
    if stream.eat('*') {
        return Some("atom");
    }
    if stream.eat('.') || stream.eat('&') {
        return Some("punctuation");
    }
    // a program rule variable's name may hold spaces, dashes and digits, so a word runs to a dot or the brace
    let rest = stream.rest();
    let len = rest
        .find(|c: char| c.is_whitespace() || matches!(c, '.' | '}' | '*' | '&'))
        .unwrap_or(rest.len());
    if len == 0 {
        stream.next();
        return Some("invalid");
    }
    let word = &rest[..len];
    stream.pos += len;
    if state.item.as_deref() == Some("V") {
        return Some(if known.variables.contains(word) {
            "variableName.special"
        } else {
            "variableName"
        });
    }
    if METRICS.contains(&word) {
        return Some("atom");
    }
    Some("variableName.special")
}

/// A name outside braces: a function when known and called, else a keyword, a literal or a name.
fn name_token(stream: &mut Stream, known: &Known, member: bool) -> &'static str {
    let rest = stream.rest();
    if member {
        let len = name_len(rest);
        let name = &rest[..len];
        stream.pos += len;
        return if known.functions.contains(&format!(".{name}")) && stream.called() {
            "variableName.function"
        } else {
            "variableName"
        };
    }
    if let Some(after) = rest.strip_prefix("d2:") {
        let len = name_len(after);
        if len > 0 {
            let program = &rest[..3 + len];
            stream.pos += 3 + len;
            return if known.functions.contains(program) && stream.called() {
                "variableName.function"
            } else {
                "variableName"
            };
        }
    }
    let len = name_len(rest);
    let name = &rest[..len];
    stream.pos += len;
    // a function named with a dot, `orgUnit.group`, is one name when it is known and called
    let after = stream.rest();
    if let Some(dotted) = after.strip_prefix('.') {
        let more = name_len(dotted);
        if more > 0 && dotted[more..].trim_start().starts_with('(') {
            let full = format!("{name}.{}", &dotted[..more]);
            if known.functions.contains(&full) {
                stream.pos += 1 + more;
                return "variableName.function";
            }
        }
    }
    if name == "true" || name == "false" {
        return "bool";
    }
    if name == "null" {
        return "null";
    }
    if KEYWORDS.contains(&name) {
        return "keyword";
    }
    if stream.called() {
        return if known.functions.contains(name) {
            "variableName.function"
        } else {
            "variableName"
        };
    }
    let mut chars = name.chars();
    if name.len() >= 2
        && chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return "atom";
    }
    "variableName"
}

/// The opening of a data item: a data element, attribute, constant, program data element, program
/// indicator, indicator, group count, reporting rate, variable; its letters.
fn item_open(rest: &str) -> Option<&str> {
    if rest.starts_with("OUG{") {
        return Some("OUG");
    }
    let mut chars = rest.chars();
    match (chars.next(), chars.next()) {
        (Some(c), Some('{')) if "#ACDINRVX".contains(c) => Some(&rest[..1]),
        _ => None,
    }
}

/// One token of an expression.
fn expression_token(stream: &mut Stream, state: &mut ExpressionState, known: &Known) -> Option<&'static str> {
    // a data item never runs across lines, so one left open ends with its line
    if stream.pos == 0 {
        state.item = None;
    }
    if state.comment {
        return Some(block_comment(stream, state));
    }
    if state.item.is_some() {
        return item_token(stream, state, known);
    }
    if stream.eat_space() {
        return None;
    }
    let member = state.member;
    state.member = false;
    let rest = stream.rest();
    if stream.eat_str("/*") {
        state.comment = true;
        return Some(block_comment(stream, state));
    }
    if let Some(quote) = rest.chars().next().filter(|c| *c == '\'' || *c == '"') {
        let mut escaped = false;
        for (at, c) in rest.char_indices().skip(1) {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == quote {
                stream.pos += at + 1;
                return Some("string");
            }
        }
        stream.pos = stream.line.len();
        return Some("invalid");
    }
    if let Some(kind) = item_open(rest) {
        state.item = Some(kind.to_owned());
        stream.pos += kind.len() + 1;
        return Some("variableName.special");
    }
    if let Some(inner) = rest.strip_prefix('[') {
        let len = name_len(inner);
        if len > 0 && inner[len..].starts_with(']') {
            stream.pos += len + 2;
            return Some("variableName.special");
        }
    }
    if stream.eat_str("PS_EVENTDATE:") {
        return Some("keyword");
    }
    let date = date_len(rest);
    if date > 0 {
        stream.pos += date;
        return Some("number");
    }
    if rest.chars().next().is_some_and(name_start) {
        return Some(name_token(stream, known, member));
    }
    let number = number_len(rest);
    if number > 0 {
        stream.pos += number;
        return Some("number");
    }
    if stream.eat('.') {
        state.member = true;
        return Some("punctuation");
    }
    if rest.starts_with(['(', ')', '[', ']', '{', '}']) {
        stream.pos += 1;
        return Some("bracket");
    }
    for operator in [
        "===", "!==", "==", "!=", "<=", ">=", "&&", "||", "+", "-", "*", "/", "%", "^", "<", ">", "!",
    ] {
        if stream.eat_str(operator) {
            return Some("operator");
        }
    }
    if stream.eat(',') {
        return Some("punctuation");
    }
    stream.next();
    Some("invalid")
}

/// Every token of a document in order, as the web's parser names them: byte ranges and tags,
/// whitespace left out.
pub fn read(text: &str, known: &Known) -> Vec<(usize, usize, &'static str)> {
    let mut state = ExpressionState::default();
    let mut out = Vec::new();
    let mut offset = 0;
    for line in text.split('\n') {
        let mut stream = Stream { line, pos: 0 };
        while !stream.eol() {
            let start = stream.pos;
            let tag = expression_token(&mut stream, &mut state, known);
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
        "keyword" => CodeTag::Keyword,
        "atom" | "bool" | "null" => CodeTag::Constant,
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
pub fn code_tokens(text: &str, known: &Known) -> Vec<CodeToken> {
    read(text, known)
        .into_iter()
        .filter_map(|(from, to, tag)| code_tag(tag).map(|kind| CodeToken { from, to, kind }))
        .collect()
}
