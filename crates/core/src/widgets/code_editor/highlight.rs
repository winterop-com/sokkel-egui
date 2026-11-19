//! The languages' tokens, each kind drawn in its `code-*` color as sokkel-ui's `HIGHLIGHT` draws
//! CodeMirror's tags: one pass per language over the document, tokens as byte ranges in order.

use super::CodeLanguage;

/// What a token is, as CodeMirror's highlight tags group them: each drawn in its `code-*` color,
/// as sokkel-ui's `HIGHLIGHT` draws them. A plugin's language names its tokens by these
/// (`CodeTag`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Keyword,
    Operator,
    Punctuation,
    String,
    Regexp,
    Number,
    Constant,
    Comment,
    Function,
    Variable,
    Type,
    Property,
    Tag,
    Attribute,
    Heading,
    Link,
    Inserted,
    Deleted,
    Invalid,
    /// Markdown's strong words: the text's ink at 600.
    Strong,
    /// Markdown's emphasized words: the text's ink, italic.
    Emphasis,
    /// Markdown's struck words.
    Strikethrough,
    /// A markdown quote: in code-comment, upright.
    Quote,
    /// Text a language reads as a value but colors as text (YAML's plain scalars).
    Literal,
}

impl Kind {
    /// The color token the kind is drawn in; none for the text's own ink.
    pub fn ink(self) -> Option<&'static str> {
        Some(match self {
            Kind::Keyword => "code-keyword",
            Kind::Operator => "code-operator",
            Kind::Punctuation => "code-punctuation",
            Kind::String => "code-string",
            Kind::Regexp => "code-regexp",
            Kind::Number => "code-number",
            Kind::Constant => "code-constant",
            Kind::Comment | Kind::Quote => "code-comment",
            Kind::Function => "code-function",
            Kind::Variable => "code-variable",
            Kind::Type => "code-type",
            Kind::Property => "code-property",
            Kind::Tag => "code-tag",
            Kind::Attribute => "code-attribute",
            Kind::Heading => "code-heading",
            Kind::Link => "code-link",
            Kind::Inserted => "code-inserted",
            Kind::Deleted => "code-deleted",
            Kind::Invalid => "code-invalid",
            Kind::Strong | Kind::Emphasis | Kind::Strikethrough | Kind::Literal => return None,
        })
    }

    /// Text a bracket inside of is not structure.
    pub(crate) fn is_text(self) -> bool {
        matches!(
            self,
            Kind::String | Kind::Comment | Kind::Quote | Kind::Literal | Kind::Regexp
        )
    }
}

/// One token: a byte range and its kind, as a plugin's language gives them too (`CodeToken`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    pub from: usize,
    pub to: usize,
    pub kind: Kind,
}

/// The tokens of a document in a language, in order, never overlapping.
pub(crate) fn tokens(language: CodeLanguage, text: &str) -> Vec<Token> {
    let mut out = Out::default();
    match language {
        CodeLanguage::Plugin(_) => {}
        CodeLanguage::Json => json(text, &mut out),
        CodeLanguage::Yaml => yaml(text, &mut out),
        CodeLanguage::Jq => jq(text, &mut out),
        CodeLanguage::Markdown => markdown(text, &mut out),
        CodeLanguage::Sql => sql(text, &mut out),
        CodeLanguage::Shell => shell(text, &mut out),
        CodeLanguage::Jinja => jinja(text, &mut out),
        CodeLanguage::Text => {}
    }
    out.0
}

#[derive(Default)]
struct Out(Vec<Token>);

impl Out {
    fn push(&mut self, from: usize, to: usize, kind: Kind) {
        if to > from {
            self.0.push(Token { from, to, kind });
        }
    }
}

/// A cursor over one line's bytes, as CodeMirror's `StringStream`.
struct Stream<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Stream<'a> {
    fn new(s: &'a str) -> Self {
        Self { s, pos: 0 }
    }
    fn eol(&self) -> bool {
        self.pos >= self.s.len()
    }
    fn rest(&self) -> &'a str {
        &self.s[self.pos..]
    }
    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }
    fn prev(&self) -> Option<char> {
        self.s[..self.pos].chars().next_back()
    }
    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }
    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
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
    fn eat_while(&mut self, f: impl Fn(char) -> bool) -> usize {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if !f(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
        self.pos - start
    }
    fn skip_to_end(&mut self) {
        self.pos = self.s.len();
    }
}

fn lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut offset = 0;
    text.split('\n').map(move |line| {
        let at = offset;
        offset += line.len() + 1;
        (at, line)
    })
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// A number at the stream: digits with a fraction and an exponent, as JSON and jq write them.
fn eat_number(s: &mut Stream) -> bool {
    let start = s.pos;
    let digits = s.eat_while(|c| c.is_ascii_digit());
    if s.peek() == Some('.') && s.rest()[1..].starts_with(|c: char| c.is_ascii_digit()) {
        s.next();
        s.eat_while(|c| c.is_ascii_digit());
    } else if digits == 0 {
        s.pos = start;
        return false;
    }
    let before_exp = s.pos;
    if s.eat('e') || s.eat('E') {
        let _ = s.eat('+') || s.eat('-');
        if s.eat_while(|c| c.is_ascii_digit()) == 0 {
            s.pos = before_exp;
        }
    }
    true
}

// --- JSON -------------------------------------------------------------------------------------

/// JSON as lang-json tags it: keys as property names, strings, numbers, true and false and null as
/// constants, the brackets, commas and colons as punctuation.
fn json(text: &str, out: &mut Out) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' && bytes[i] != b'\n' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                if i < bytes.len() && bytes[i] == b'"' {
                    i += 1;
                }
                let end = i.min(bytes.len());
                let mut j = end;
                while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\r') {
                    j += 1;
                }
                let kind = if j < bytes.len() && bytes[j] == b':' {
                    Kind::Property
                } else {
                    Kind::String
                };
                out.push(start, end, kind);
            }
            b'{' | b'}' | b'[' | b']' | b',' | b':' => {
                out.push(i, i + 1, Kind::Punctuation);
                i += 1;
            }
            b'-' | b'0'..=b'9' => {
                let mut s = Stream::new(&text[i..]);
                s.eat('-');
                if eat_number(&mut s) {
                    out.push(i, i + s.pos, Kind::Number);
                    i += s.pos;
                } else {
                    i += 1;
                }
            }
            b't' | b'f' | b'n' => {
                let rest = &text[i..];
                let word = rest
                    .find(|c: char| !c.is_ascii_alphabetic())
                    .map_or(rest, |end| &rest[..end]);
                if matches!(word, "true" | "false" | "null") {
                    out.push(i, i + word.len(), Kind::Constant);
                }
                i += word.len().max(1);
            }
            _ => i += 1,
        }
    }
}

// --- YAML -------------------------------------------------------------------------------------

/// The end of a YAML key on a line from `at`: where its `:` stands, followed by a space or the
/// line's end; quoted keys whole.
fn yaml_key_end(line: &str, at: usize) -> Option<usize> {
    let rest = &line[at..];
    if rest.starts_with('"') || rest.starts_with('\'') {
        let quote = rest.as_bytes()[0];
        let mut i = 1;
        let b = rest.as_bytes();
        while i < b.len() && b[i] != quote {
            i += if b[i] == b'\\' && quote == b'"' { 2 } else { 1 };
        }
        let end = (i + 1).min(b.len());
        let after = rest[end..].trim_start_matches([' ', '\t']);
        return (after.starts_with(':') && after[1..].chars().next().is_none_or(char::is_whitespace))
            .then_some(at + end);
    }
    if rest.starts_with(['#', '[', '{', '&', '*', '!', '|', '>', '%', '@', '`']) {
        return None;
    }
    let b = rest.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b':' && (i + 1 == b.len() || b[i + 1] == b' ' || b[i + 1] == b'\t') {
            let key = rest[..i].trim_end();
            return (!key.is_empty()).then_some(at + key.len());
        }
        if b[i] == b'#' && i > 0 && (b[i - 1] == b' ' || b[i - 1] == b'\t') {
            return None;
        }
        i += 1;
    }
    None
}

/// Where a YAML comment starts on a line from `at`: a `#` at the start or after a space, outside quotes.
pub(crate) fn yaml_comment(line: &str, at: usize) -> Option<usize> {
    let b = line.as_bytes();
    let mut quote = None;
    let mut i = at;
    while i < b.len() {
        let c = b[i];
        match quote {
            Some(q) => {
                if c == b'\\' && q == b'"' {
                    i += 1;
                } else if c == q {
                    quote = None;
                }
            }
            None => {
                if (c == b'"' || c == b'\'')
                    && (i == at || matches!(b[i - 1], b' ' | b'\t' | b'[' | b'{' | b',' | b':'))
                {
                    quote = Some(c);
                } else if c == b'#' && (i == 0 || b[i - 1] == b' ' || b[i - 1] == b'\t') {
                    return Some(i);
                }
            }
        }
        i += 1;
    }
    None
}

/// YAML as lang-yaml tags it: keys as property definitions, quoted strings, a block's header as
/// a special string, anchors and aliases as labels, tags as types, comments, and its `-`, `:`,
/// `,` and brackets as punctuation; plain values in the text's ink.
fn yaml(text: &str, out: &mut Out) {
    // the indent a block literal's content stands deeper than, while in one
    let mut block: Option<usize> = None;
    for (offset, line) in lines(text) {
        let indent = line.len() - line.trim_start_matches(' ').len();
        if let Some(header) = block {
            if line.trim().is_empty() || indent > header {
                out.push(offset + indent, offset + line.len(), Kind::Literal);
                continue;
            }
            block = None;
        }
        if line.starts_with('%') {
            let end = line.find(' ').unwrap_or(line.len());
            out.push(offset, offset + end, Kind::Keyword);
            continue;
        }
        if line.starts_with("---") || line.starts_with("...") {
            let rest = &line[3..];
            if rest.is_empty() || rest.starts_with([' ', '\t']) {
                if let Some(c) = yaml_comment(line, 3) {
                    out.push(offset + c, offset + line.len(), Kind::Comment);
                }
                continue;
            }
        }
        let comment = yaml_comment(line, 0);
        let end = comment.unwrap_or(line.len());
        let mut at = indent;
        // the dashes of list items
        loop {
            let rest = &line[at..end];
            let dash = rest.starts_with('-') && (rest.len() == 1 || rest[1..].starts_with([' ', '\t']));
            let question = rest.starts_with('?') && (rest.len() == 1 || rest[1..].starts_with(' '));
            if dash || question {
                out.push(offset + at, offset + at + 1, Kind::Punctuation);
                at += 1;
                at += line[at..end].len() - line[at..end].trim_start().len();
            } else {
                break;
            }
        }
        if let Some(key_end) = yaml_key_end(&line[..end], at) {
            out.push(offset + at, offset + key_end, Kind::Property);
            let colon = key_end + line[key_end..].find(':').unwrap_or(0);
            out.push(offset + colon, offset + colon + 1, Kind::Punctuation);
            at = colon + 1;
            at += line[at..end].len() - line[at..end].trim_start().len();
        }
        yaml_value(line, offset, at, end, out, &mut block, indent);
        if let Some(c) = comment {
            out.push(offset + c, offset + line.len(), Kind::Comment);
        }
    }
}

fn yaml_value(
    line: &str,
    offset: usize,
    mut at: usize,
    end: usize,
    out: &mut Out,
    block: &mut Option<usize>,
    indent: usize,
) {
    let mut flow = 0usize;
    while at < end {
        let rest = &line[at..end];
        let c = rest.chars().next().unwrap_or(' ');
        if c == ' ' || c == '\t' {
            at += 1;
            continue;
        }
        let token_end = |stop: &[char]| -> usize {
            at + rest
                .find(|ch: char| ch == ' ' || ch == '\t' || stop.contains(&ch))
                .unwrap_or(rest.len())
        };
        match c {
            '&' | '*' => {
                let e = token_end(&[',', ']', '}']);
                out.push(offset + at, offset + e, Kind::Attribute);
                at = e;
            }
            '!' => {
                let e = token_end(&[]);
                out.push(offset + at, offset + e, Kind::Type);
                at = e;
            }
            '|' | '>' if flow == 0 => {
                out.push(offset + at, offset + end, Kind::String);
                *block = Some(indent);
                return;
            }
            '"' | '\'' => {
                let b = rest.as_bytes();
                let mut i = 1;
                while i < b.len() && b[i] != c as u8 {
                    i += if b[i] == b'\\' && c == '"' { 2 } else { 1 };
                }
                let e = at + (i + 1).min(b.len());
                let after = line[e..end].trim_start();
                let kind = if flow > 0 && after.starts_with(':') {
                    Kind::Property
                } else {
                    Kind::String
                };
                out.push(offset + at, offset + e, kind);
                at = e;
            }
            '[' | '{' => {
                flow += 1;
                out.push(offset + at, offset + at + 1, Kind::Punctuation);
                at += 1;
            }
            ']' | '}' | ',' if flow > 0 => {
                if c != ',' {
                    flow -= 1;
                }
                out.push(offset + at, offset + at + 1, Kind::Punctuation);
                at += 1;
            }
            ':' if flow > 0 => {
                out.push(offset + at, offset + at + 1, Kind::Punctuation);
                at += 1;
            }
            _ => {
                // a plain scalar, to the end of the line or, in flow, to its next mark
                let e = if flow > 0 {
                    let mut e = at;
                    let b = line.as_bytes();
                    while e < end && !matches!(b[e], b',' | b']' | b'}') {
                        if b[e] == b':' && (e + 1 >= end || b[e + 1] == b' ') {
                            break;
                        }
                        e += 1;
                    }
                    e
                } else {
                    end
                };
                let value = line[at..e].trim_end();
                let kind = if flow > 0 && line[at + value.len()..end].trim_start().starts_with(':') {
                    Kind::Property
                } else {
                    Kind::Literal
                };
                out.push(offset + at, offset + at + value.len(), kind);
                at = e.max(at + 1);
            }
        }
    }
}

// --- jq ---------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum JqFrame {
    String,
    Interpolation(u32),
}

const JQ_CONTROL: &[&str] = &[
    "if", "then", "elif", "else", "end", "reduce", "foreach", "try", "catch", "label", "import", "include", "as",
    "__loc__",
];

/// jq as sokkel-ui's jq parser tags it: strings and their `\(...)`, keywords, `.field`s, `$names`,
/// `@formats`, built-in calls, numbers, operators and comments.
fn jq(text: &str, out: &mut Out) {
    let mut stack: Vec<JqFrame> = Vec::new();
    let mut defining = false;
    for (offset, line) in lines(text) {
        let mut s = Stream::new(line);
        while !s.eol() {
            let start = s.pos;
            let kind = if matches!(stack.last(), Some(JqFrame::String)) {
                if s.eat_str("\\(") {
                    stack.push(JqFrame::Interpolation(0));
                    Some(Kind::String)
                } else if s.eat('\\') {
                    s.next();
                    Some(Kind::Regexp)
                } else if s.eat('"') {
                    stack.pop();
                    Some(Kind::String)
                } else {
                    s.eat_while(|c| c != '"' && c != '\\');
                    Some(Kind::String)
                }
            } else {
                jq_code(&mut s, &mut stack, &mut defining)
            };
            if s.pos == start {
                s.next();
            }
            if let Some(kind) = kind {
                out.push(offset + start, offset + s.pos, kind);
            }
        }
    }
}

fn jq_code(s: &mut Stream, stack: &mut Vec<JqFrame>, defining: &mut bool) -> Option<Kind> {
    if s.eat_while(char::is_whitespace) > 0 {
        return None;
    }
    if let Some(JqFrame::Interpolation(depth)) = stack.last_mut() {
        if s.peek() == Some('(') {
            *depth += 1;
        }
        if s.peek() == Some(')') {
            if *depth == 0 {
                s.next();
                stack.pop();
                return Some(Kind::String);
            }
            *depth -= 1;
        }
    }
    let was_defining = std::mem::take(defining);
    if s.eat('#') {
        s.skip_to_end();
        return Some(Kind::Comment);
    }
    if s.eat('"') {
        stack.push(JqFrame::String);
        return Some(Kind::String);
    }
    if s.peek() == Some('@') {
        s.next();
        if s.eat_while(|c| c.is_ascii_alphanumeric() || c == '_') > 0 {
            return Some(Kind::String);
        }
        return Some(Kind::Invalid);
    }
    if s.peek() == Some('$') {
        s.next();
        if s.eat_while(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') > 0 {
            // `$name` is a variable name, in the text's ink as CodeMirror's `variableName`
            return None;
        }
        return Some(Kind::Invalid);
    }
    if s.peek().is_some_and(|c| c.is_ascii_digit())
        || (s.peek() == Some('.') && s.rest()[1..].starts_with(|c: char| c.is_ascii_digit()))
    {
        eat_number(s);
        return Some(Kind::Number);
    }
    if s.eat_str("..") {
        return Some(Kind::Operator);
    }
    if s.peek() == Some('.') {
        let rest = &s.rest()[1..];
        if rest.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            s.next();
            s.eat_while(|c| c.is_ascii_alphanumeric() || c == '_');
            return Some(Kind::Property);
        }
        if rest.starts_with('"') {
            s.next();
            s.next();
            while let Some(c) = s.next() {
                if c == '\\' {
                    s.next();
                } else if c == '"' {
                    break;
                }
            }
            return Some(Kind::Property);
        }
        s.next();
        return Some(Kind::Operator);
    }
    if s.peek().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') {
        let start = s.pos;
        s.eat_while(|c| c.is_ascii_alphanumeric() || c == '_');
        while s.rest().starts_with("::") && s.rest()[2..].starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            s.pos += 2;
            s.eat_while(|c| c.is_ascii_alphanumeric() || c == '_');
        }
        let word = &s.s[start..s.pos];
        return match word {
            "def" => {
                *defining = true;
                Some(Kind::Keyword)
            }
            w if JQ_CONTROL.contains(&w) => Some(Kind::Keyword),
            "and" | "or" | "not" => Some(Kind::Operator),
            "true" | "false" | "null" => Some(Kind::Constant),
            // a definition's name is in the text's ink; any other name is a call
            _ if was_defining => None,
            _ => Some(Kind::Function),
        };
    }
    for op in [
        "?//", "//=", "|=", "+=", "-=", "*=", "/=", "%=", "//", "==", "!=", "<=", ">=", "|", ",", "+", "-", "*", "/",
        "%", "<", ">", "=", "?",
    ] {
        if s.eat_str(op) {
            return Some(Kind::Operator);
        }
    }
    if s.peek().is_some_and(|c| "()[]{}".contains(c)) {
        s.next();
        return Some(Kind::Punctuation);
    }
    if s.peek().is_some_and(|c| c == ':' || c == ';') {
        s.next();
        return Some(Kind::Punctuation);
    }
    s.next();
    Some(Kind::Invalid)
}

/// jq's built-in filters, each with its signature's tail and what it does, as sokkel-ui offers them.
pub(crate) const JQ_BUILTINS: &[(&str, &str)] = &[
    (
        "length",
        "The length of a string, array or object; the absolute value of a number.",
    ),
    ("keys", "An object's keys, sorted, or an array's indexes."),
    ("keys_unsorted", "An object's keys in their order."),
    ("values", "The input unless it is null."),
    ("has(key)", "Whether the input has the key or index."),
    ("in(object)", "Whether the input is a key of the object."),
    ("map(f)", "Runs f on each item of an array and collects the results."),
    (
        "map_values(f)",
        "Runs f on each value of an object or array, keeping the keys.",
    ),
    ("path(f)", "The paths f selects, as arrays."),
    ("paths", "Every path in the input."),
    ("getpath(path)", "The value at the path."),
    ("setpath(path; value)", "The input with the value at the path set."),
    ("del(f)", "The input without what f selects."),
    ("to_entries", "An object as an array of {key, value}."),
    ("from_entries", "An array of {key, value} as an object."),
    ("with_entries(f)", "Runs f on each {key, value} of an object."),
    ("select(f)", "The input when f is true, else nothing."),
    ("empty", "No output at all."),
    ("error(message)", "Stops with an error carrying the message."),
    ("add", "Adds the items of an array together."),
    ("any", "Whether any item of an array is true."),
    ("all", "Whether every item of an array is true."),
    ("flatten", "An array of arrays as one array."),
    ("range(n)", "The numbers from 0 up to n."),
    ("floor", "A number rounded down."),
    ("sqrt", "A number's square root."),
    ("tostring", "The input as a string."),
    ("tonumber", "A string read as a number."),
    ("type", "The type of the input, as a string."),
    ("sort", "An array sorted."),
    ("sort_by(f)", "An array sorted by what f gives for each item."),
    ("group_by(f)", "An array grouped by what f gives for each item."),
    ("unique", "An array sorted with duplicates removed."),
    ("unique_by(f)", "An array with one item for each value f gives."),
    ("min", "The smallest item of an array."),
    ("max", "The largest item of an array."),
    ("min_by(f)", "The item for which f gives the smallest value."),
    ("max_by(f)", "The item for which f gives the largest value."),
    ("reverse", "An array or string in reverse."),
    ("contains(x)", "Whether the input contains x."),
    ("inside(x)", "Whether x contains the input."),
    ("startswith(s)", "Whether a string starts with s."),
    ("endswith(s)", "Whether a string ends with s."),
    ("ltrimstr(s)", "A string without s at its start."),
    ("rtrimstr(s)", "A string without s at its end."),
    ("split(s)", "A string split at each s."),
    ("join(s)", "The strings of an array joined with s between."),
    ("ascii_downcase", "A string in lowercase."),
    ("ascii_upcase", "A string in uppercase."),
    ("test(regex)", "Whether a string matches the regular expression."),
    ("match(regex)", "The matches of the regular expression."),
    ("capture(regex)", "The regular expression's named groups as an object."),
    ("sub(regex; s)", "A string with the first match replaced by s."),
    ("gsub(regex; s)", "A string with every match replaced by s."),
    ("recurse", "The input and everything inside it."),
    ("env", "The environment variables as an object."),
    ("input", "The next input."),
    ("inputs", "Every remaining input."),
    ("debug", "The input, also written to the error stream."),
    ("tojson", "The input as JSON text."),
    ("fromjson", "JSON text read as a value."),
    ("now", "The current time in seconds since the epoch."),
    ("todate", "Seconds since the epoch as an ISO 8601 date."),
    ("fromdate", "An ISO 8601 date as seconds since the epoch."),
    ("limit(n; f)", "The first n outputs of f."),
    ("first", "The first item of an array."),
    ("last", "The last item of an array."),
    ("nth(n)", "The nth item of an array."),
    ("indices(s)", "The indexes at which s occurs."),
    ("isempty(f)", "Whether f gives no output."),
    ("objects", "The input when it is an object."),
    ("arrays", "The input when it is an array."),
    ("strings", "The input when it is a string."),
    ("numbers", "The input when it is a number."),
    ("tostream", "The input as a stream of path and leaf pairs."),
    ("fromstream(f)", "A value built from a stream of path and leaf pairs."),
    ("splits(regex)", "The parts of a string between the matches."),
    ("ascii", "The character of a code point."),
    ("explode", "A string as an array of code points."),
    ("implode", "An array of code points as a string."),
    ("abs", "A number's absolute value."),
];

/// jq's keywords, offered beside its built-ins.
pub(crate) const JQ_KEYWORDS: &[&str] = &[
    "def", "if", "then", "elif", "else", "end", "reduce", "foreach", "try", "catch", "label", "import", "include",
    "as", "and", "or", "not",
];

// --- Markdown ---------------------------------------------------------------------------------

/// Markdown as sokkel-ui's markdown parser tags it: fences and their code, headings, rules,
/// quotes, list marks, `[[links]]`, code, strong, emphasized and struck words, links and
/// addresses, `#tags` and escapes.
fn markdown(text: &str, out: &mut Out) {
    let mut fence: Option<String> = None;
    for (offset, line) in lines(text) {
        let fence_mark = fence_of(line);
        if let Some(open) = &fence {
            let closes = fence_mark.as_ref().is_some_and(|m| {
                m.starts_with(open.chars().next().unwrap_or('`')) && m.len() >= open.len() && line.trim() == m
            });
            out.push(
                offset,
                offset + line.len(),
                if closes { Kind::Punctuation } else { Kind::String },
            );
            if closes {
                fence = None;
            }
            continue;
        }
        if let Some(mark) = fence_mark {
            fence = Some(mark);
            out.push(offset, offset + line.len(), Kind::Punctuation);
            continue;
        }
        let trimmed = line.trim_start_matches(' ');
        let lead = line.len() - trimmed.len();
        if lead <= 3 {
            let hashes = trimmed.len() - trimmed.trim_start_matches('#').len();
            if (1..=6).contains(&hashes) && trimmed[hashes..].chars().next().is_none_or(char::is_whitespace) {
                out.push(offset, offset + line.len(), Kind::Heading);
                continue;
            }
            let rule = trimmed.trim_end();
            if let Some(c) = rule.chars().next()
                && matches!(c, '-' | '*' | '_')
                && rule.chars().filter(|x| *x == c).count() >= 3
                && rule.chars().all(|x| x == c || x == ' ')
            {
                out.push(offset, offset + line.len(), Kind::Punctuation);
                continue;
            }
            if trimmed.starts_with('>') {
                let mut e = lead + 1;
                if line[e..].starts_with(' ') {
                    e += 1;
                }
                out.push(offset, offset + e, Kind::Quote);
                markdown_inline(line, offset, e, out);
                continue;
            }
        }
        let start = list_mark(line);
        if start > 0 {
            out.push(offset, offset + start, Kind::Punctuation);
        }
        markdown_inline(line, offset, start, out);
    }
}

/// A fence that opens or closes a block on this line: three or more backticks or tildes, at most
/// three spaces in.
fn fence_of(line: &str) -> Option<String> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let c = trimmed.chars().next()?;
    if c != '`' && c != '~' {
        return None;
    }
    let n = trimmed.len() - trimmed.trim_start_matches(c).len();
    (n >= 3).then(|| c.to_string().repeat(n))
}

/// The length of a list item's mark and its checkbox, `- `, `1. `, `- [x] `; zero for none.
fn list_mark(line: &str) -> usize {
    let lead = line.len() - line.trim_start().len();
    let rest = &line[lead..];
    let mark = if rest.starts_with(['-', '*', '+']) {
        1
    } else {
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        if digits > 0 && rest[digits..].starts_with(['.', ')']) {
            digits + 1
        } else {
            return 0;
        }
    };
    let after = &rest[mark..];
    let space = after.len() - after.trim_start().len();
    if space == 0 || after.trim().is_empty() && space == after.len() && after.is_empty() {
        return 0;
    }
    let mut end = lead + mark + space;
    let rest = &line[end..];
    if rest.len() >= 3 && rest.starts_with('[') && matches!(&rest[1..2], " " | "x" | "X") && &rest[2..3] == "]" {
        let after = &rest[3..];
        let space = after.len() - after.trim_start().len();
        if space > 0 {
            end += 3 + space;
        }
    }
    end
}

fn markdown_inline(line: &str, offset: usize, from: usize, out: &mut Out) {
    let mut s = Stream::new(line);
    s.pos = from;
    let mut address = false;
    while !s.eol() {
        let start = s.pos;
        let after_link = std::mem::take(&mut address);
        let rest = s.rest();
        let before = s.prev();
        let kind = if after_link
            && rest.starts_with('(')
            && let Some(e) = rest.find(')')
        {
            s.pos += e + 1;
            Some(Kind::Link)
        } else if rest.starts_with('_') && before.is_some_and(|c| c.is_alphanumeric()) {
            s.eat_while(word_char);
            None
        } else if rest.starts_with("[[")
            && let Some(e) = wiki_end(rest)
        {
            s.pos += e;
            Some(Kind::Link)
        } else if rest.starts_with('`')
            && let Some(e) = code_span(rest)
        {
            s.pos += e;
            Some(Kind::String)
        } else if let Some(e) = delimited(rest, &["**", "__"]) {
            s.pos += e;
            Some(Kind::Strong)
        } else if let Some(e) = delimited(rest, &["*", "_"]) {
            s.pos += e;
            Some(Kind::Emphasis)
        } else if let Some(e) = delimited(rest, &["~~"]) {
            s.pos += e;
            Some(Kind::Strikethrough)
        } else if let Some(e) = link_words(rest) {
            s.pos += e;
            address = true;
            Some(Kind::Link)
        } else if let Some(e) = address_at(rest) {
            s.pos += e;
            Some(Kind::Link)
        } else if rest.starts_with('#')
            && before.is_none_or(|c| c.is_whitespace() || "([{\"'\u{201c}\u{2018}".contains(c))
            && let Some(e) = tag_end(rest)
        {
            s.pos += e;
            Some(Kind::Attribute)
        } else if rest.starts_with('\\') && rest.len() > 1 {
            s.next();
            s.next();
            Some(Kind::Regexp)
        } else {
            s.next();
            s.eat_while(|c| !"[`*_~!(<h#\\".contains(c));
            None
        };
        if let Some(kind) = kind {
            out.push(offset + start, offset + s.pos, kind);
        }
    }
}

/// The length of a `[[link]]` at the start of `rest`.
pub(crate) fn wiki_end(rest: &str) -> Option<usize> {
    let inner = &rest[2..];
    let close = inner.find("]]")?;
    let name = &inner[..close];
    (!name.is_empty() && !name.contains(['[', ']', '\n'])).then_some(close + 4)
}

fn code_span(rest: &str) -> Option<usize> {
    let ticks = rest.len() - rest.trim_start_matches('`').len();
    let open = &rest[..ticks];
    let body = &rest[ticks..];
    let close = body.find(open)?;
    (close > 0 && !body[..close].contains('`')).then_some(ticks + close + ticks)
}

/// The length of `**words**` (or another mark) at the start of `rest`: the mark, words that start
/// and end with no space, the mark again.
fn delimited(rest: &str, marks: &[&str]) -> Option<usize> {
    for mark in marks {
        if !rest.starts_with(mark) {
            continue;
        }
        let body = &rest[mark.len()..];
        if body.starts_with(char::is_whitespace) || body.is_empty() {
            continue;
        }
        let single = mark.len() == 1;
        if single && body.starts_with(['*', '_']) {
            continue;
        }
        let mut from = 0;
        while let Some(found) = body[from..].find(mark) {
            let at = from + found;
            if at > 0 {
                let last = body[..at].chars().next_back()?;
                let next = body[at + mark.len()..].chars().next();
                if !last.is_whitespace()
                    && (!single || (last != '*' && last != '_' && next.is_none_or(|c| c != '*' && c != '_')))
                {
                    return Some(mark.len() + at + mark.len());
                }
            }
            from = at + mark.len();
        }
    }
    None
}

fn link_words(rest: &str) -> Option<usize> {
    let open = usize::from(rest.starts_with('!'));
    if !rest[open..].starts_with('[') {
        return None;
    }
    let close = rest[open + 1..].find(']')? + open + 1;
    (!rest[open + 1..close].contains('\n') && rest[close + 1..].starts_with('(')).then_some(close + 1)
}

fn address_at(rest: &str) -> Option<usize> {
    if rest.starts_with("<http://") || rest.starts_with("<https://") {
        let close = rest.find('>')?;
        return (!rest[..close].contains(char::is_whitespace)).then_some(close + 1);
    }
    if rest.starts_with("http://") || rest.starts_with("https://") {
        let scheme = rest.find("://")? + 3;
        let end = rest[scheme..]
            .find(|c: char| c.is_whitespace() || c == ')' || c == '<')
            .map_or(rest.len(), |e| scheme + e);
        return (end > scheme).then_some(end);
    }
    None
}

/// The length of a `#tag` at the start of `rest`: letters, digits, `_`, `-` and `/`, one of them
/// not a digit.
fn tag_end(rest: &str) -> Option<usize> {
    let body = &rest[1..];
    let len = body
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '/'))
        .unwrap_or(body.len());
    let tag = &body[..len];
    tag.chars().any(|c| c.is_alphabetic() || c == '_').then_some(1 + len)
}

// --- SQL --------------------------------------------------------------------------------------

const SQL_KEYWORDS: &[&str] = &[
    "absolute",
    "action",
    "add",
    "after",
    "all",
    "allocate",
    "alter",
    "and",
    "any",
    "are",
    "as",
    "asc",
    "assertion",
    "at",
    "authorization",
    "before",
    "begin",
    "between",
    "both",
    "breadth",
    "by",
    "call",
    "cascade",
    "cascaded",
    "case",
    "cast",
    "catalog",
    "check",
    "close",
    "collate",
    "collation",
    "column",
    "commit",
    "condition",
    "connect",
    "connection",
    "constraint",
    "constraints",
    "constructor",
    "continue",
    "corresponding",
    "count",
    "create",
    "cross",
    "cube",
    "current",
    "cursor",
    "cycle",
    "data",
    "day",
    "deallocate",
    "declare",
    "default",
    "deferrable",
    "deferred",
    "delete",
    "depth",
    "deref",
    "desc",
    "describe",
    "descriptor",
    "deterministic",
    "diagnostics",
    "disconnect",
    "distinct",
    "do",
    "domain",
    "drop",
    "dynamic",
    "each",
    "else",
    "elseif",
    "end",
    "equals",
    "escape",
    "except",
    "exception",
    "exec",
    "execute",
    "exists",
    "exit",
    "external",
    "fetch",
    "first",
    "for",
    "foreign",
    "found",
    "from",
    "free",
    "full",
    "function",
    "general",
    "get",
    "global",
    "go",
    "goto",
    "grant",
    "group",
    "grouping",
    "handle",
    "having",
    "hold",
    "hour",
    "identity",
    "if",
    "immediate",
    "in",
    "indicator",
    "initially",
    "inner",
    "inout",
    "input",
    "insert",
    "intersect",
    "into",
    "is",
    "isolation",
    "join",
    "key",
    "language",
    "last",
    "lateral",
    "leading",
    "leave",
    "left",
    "level",
    "like",
    "limit",
    "local",
    "locator",
    "loop",
    "map",
    "match",
    "method",
    "minute",
    "modifies",
    "module",
    "month",
    "names",
    "natural",
    "nesting",
    "new",
    "next",
    "no",
    "none",
    "not",
    "of",
    "offset",
    "old",
    "on",
    "only",
    "open",
    "option",
    "or",
    "order",
    "ordinality",
    "out",
    "outer",
    "output",
    "overlaps",
    "pad",
    "parameter",
    "partial",
    "path",
    "prepare",
    "preserve",
    "primary",
    "prior",
    "privileges",
    "procedure",
    "public",
    "read",
    "reads",
    "recursive",
    "redo",
    "ref",
    "references",
    "referencing",
    "relative",
    "release",
    "repeat",
    "resignal",
    "restrict",
    "result",
    "return",
    "returns",
    "revoke",
    "right",
    "role",
    "rollback",
    "rollup",
    "routine",
    "row",
    "rows",
    "savepoint",
    "schema",
    "scroll",
    "search",
    "second",
    "section",
    "select",
    "session",
    "session_user",
    "set",
    "sets",
    "signal",
    "similar",
    "size",
    "some",
    "space",
    "specific",
    "sql",
    "sqlexception",
    "sqlstate",
    "sqlwarning",
    "start",
    "state",
    "static",
    "system_user",
    "table",
    "temporary",
    "then",
    "timezone_hour",
    "timezone_minute",
    "to",
    "trailing",
    "transaction",
    "translation",
    "treat",
    "trigger",
    "under",
    "undo",
    "union",
    "unique",
    "unnest",
    "until",
    "update",
    "usage",
    "user",
    "using",
    "value",
    "values",
    "view",
    "when",
    "whenever",
    "where",
    "while",
    "with",
    "without",
    "work",
    "write",
    "year",
    "zone",
];

const SQL_TYPES: &[&str] = &[
    "array",
    "binary",
    "bit",
    "blob",
    "boolean",
    "char",
    "character",
    "clob",
    "date",
    "dec",
    "decimal",
    "double",
    "float",
    "int",
    "integer",
    "interval",
    "large",
    "national",
    "nchar",
    "nclob",
    "numeric",
    "object",
    "precision",
    "real",
    "smallint",
    "time",
    "timestamp",
    "varchar",
    "varying",
    "bigint",
    "text",
    "uuid",
    "json",
    "jsonb",
];

/// SQL as lang-sql's standard dialect tags it: keywords, types, strings, numbers, true, false and
/// null, comments, operators and punctuation; names in the text's ink.
fn sql(text: &str, out: &mut Out) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if (c == b'-' && b.get(i + 1) == Some(&b'-')) || c == b'#' {
            let e = text[i..].find('\n').map_or(text.len(), |e| i + e);
            out.push(i, e, Kind::Comment);
            i = e;
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            let e = text[i + 2..].find("*/").map_or(text.len(), |e| i + 2 + e + 2);
            out.push(i, e, Kind::Comment);
            i = e;
        } else if c == b'\'' || c == b'"' || c == b'`' {
            let mut j = i + 1;
            while j < b.len() {
                if b[j] == c {
                    if b.get(j + 1) == Some(&c) {
                        j += 2;
                        continue;
                    }
                    break;
                }
                if b[j] == b'\\' && c == b'\'' {
                    j += 1;
                }
                j += 1;
            }
            let e = (j + 1).min(b.len());
            out.push(i, e, Kind::String);
            i = e;
        } else if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit)) {
            let mut s = Stream::new(&text[i..]);
            if !eat_number(&mut s) {
                s.next();
            }
            out.push(i, i + s.pos, Kind::Number);
            i += s.pos;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let e = text[i..]
                .find(|x: char| !(x.is_ascii_alphanumeric() || x == '_' || x == '$'))
                .map_or(text.len(), |e| i + e);
            let word = text[i..e].to_ascii_lowercase();
            let kind = if matches!(word.as_str(), "true" | "false" | "null" | "unknown") {
                Some(Kind::Constant)
            } else if SQL_TYPES.contains(&word.as_str()) {
                Some(Kind::Type)
            } else if SQL_KEYWORDS.contains(&word.as_str()) {
                Some(Kind::Keyword)
            } else {
                None
            };
            if let Some(kind) = kind {
                out.push(i, e, kind);
            }
            i = e;
        } else if b"+-*/<>=!|%^&~".contains(&c) {
            let mut e = i + 1;
            while e < b.len() && b"<>=!|".contains(&b[e]) {
                e += 1;
            }
            out.push(i, e, Kind::Operator);
            i = e;
        } else if b"(),;.[]{}".contains(&c) {
            out.push(i, i + 1, Kind::Punctuation);
            i += 1;
        } else {
            i += text[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
}

// --- Shell ------------------------------------------------------------------------------------

const SHELL_KEYWORDS: &[&str] = &[
    "if", "then", "do", "else", "elif", "while", "until", "for", "in", "esac", "fi", "fin", "fil", "done", "exit",
    "set", "unset", "export", "function",
];

/// Shell as CodeMirror's legacy shell mode tags it: keywords, true and false, strings and
/// backquoted commands, comments, `-flags` as attributes, numbers and `+`, `=` as operators.
fn shell(text: &str, out: &mut Out) {
    // the quote a string left open at the end of a line
    let mut open: Option<char> = None;
    for (offset, line) in lines(text) {
        let mut s = Stream::new(line);
        if let Some(q) = open {
            let start = s.pos;
            open = shell_string(&mut s, q);
            out.push(
                offset + start,
                offset + s.pos,
                if q == '`' { Kind::Quote } else { Kind::String },
            );
        }
        while !s.eol() {
            let start = s.pos;
            let sol = start == 0 || line[..start].trim().is_empty();
            if s.eat_while(char::is_whitespace) > 0 {
                continue;
            }
            let c = s.next().unwrap_or(' ');
            let kind = match c {
                '\\' => {
                    s.next();
                    None
                }
                '\'' | '"' | '`' => {
                    open = shell_string(&mut s, c);
                    Some(if c == '`' { Kind::Quote } else { Kind::String })
                }
                '#' => {
                    if !(sol && s.peek() == Some('!')) {
                        s.skip_to_end();
                        Some(Kind::Comment)
                    } else {
                        s.skip_to_end();
                        None
                    }
                }
                '$' => {
                    if s.eat('{') {
                        s.eat_while(|c| c != '}');
                        s.eat('}');
                    } else if s.eat('(') {
                        s.eat_while(|c| c != ')');
                        s.eat(')');
                    } else {
                        s.eat_while(|c| word_char(c) || "?#@*!$-".contains(c));
                    }
                    None
                }
                '+' | '=' => Some(Kind::Operator),
                '-' => {
                    s.eat('-');
                    s.eat_while(word_char);
                    Some(Kind::Attribute)
                }
                c if c.is_ascii_digit() => {
                    s.eat_while(|c| c.is_ascii_digit());
                    if s.eol() || !s.peek().is_some_and(word_char) {
                        Some(Kind::Number)
                    } else {
                        s.eat_while(|c| word_char(c) || c == '-');
                        None
                    }
                }
                c if word_char(c) => {
                    s.eat_while(|c| word_char(c) || c == '-');
                    let word = &line[start..s.pos];
                    if s.peek() == Some('=') {
                        None
                    } else if word == "true" || word == "false" {
                        Some(Kind::Constant)
                    } else if SHELL_KEYWORDS.contains(&word) {
                        Some(Kind::Keyword)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(kind) = kind {
                out.push(offset + start, offset + s.pos, kind);
            }
        }
    }
}

/// Reads a shell string to its closing quote; the quote when it is still open at the line's end.
fn shell_string(s: &mut Stream, quote: char) -> Option<char> {
    while let Some(c) = s.next() {
        if c == '\\' && quote != '\'' {
            s.next();
        } else if c == quote {
            return None;
        }
    }
    Some(quote)
}

// --- Jinja ------------------------------------------------------------------------------------

const JINJA_KEYWORDS: &[&str] = &[
    "and",
    "as",
    "block",
    "endblock",
    "by",
    "cycle",
    "debug",
    "else",
    "elif",
    "endfor",
    "endif",
    "extends",
    "filter",
    "endfilter",
    "firstof",
    "do",
    "for",
    "endmacro",
    "if",
    "ifchanged",
    "endifchanged",
    "ifequal",
    "endifequal",
    "ignoremissing",
    "in",
    "include",
    "load",
    "macro",
    "not",
    "now",
    "or",
    "parsed",
    "regroup",
    "reversed",
    "spaceless",
    "call",
    "endcall",
    "endspaceless",
    "ssi",
    "templatetag",
    "openblock",
    "closeblock",
    "openvariable",
    "closevariable",
    "without",
    "context",
    "with",
    "endwith",
    "openbrace",
    "closebrace",
    "opencomment",
    "closecomment",
    "widthratio",
    "url",
    "trans",
    "endtrans",
    "noop",
    "blocktrans",
    "endblocktrans",
    "pluralize",
    "autoescape",
    "endautoescape",
    "set",
    "endset",
    "is",
    "import",
    "from",
    "raw",
    "endraw",
];

/// Jinja as CodeMirror's legacy jinja2 mode tags it: `{{ }}` and `{% %}` as tags, `{# #}` comments,
/// and inside the tags keywords, true, false and none, strings, numbers and operators.
fn jinja(text: &str, out: &mut Out) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let Some(found) = text[i..].find('{') else { break };
        let at = i + found;
        let next = b.get(at + 1).copied();
        match next {
            Some(b'#') => {
                let e = text[at + 2..].find("#}").map_or(text.len(), |e| at + 2 + e + 2);
                out.push(at, e, Kind::Comment);
                i = e;
            }
            Some(b'{') | Some(b'%') => {
                let close = if next == Some(b'{') { "}}" } else { "%}" };
                let mut open_end = at + 2;
                if matches!(b.get(open_end), Some(b'-') | Some(b'+')) {
                    open_end += 1;
                }
                out.push(at, open_end, Kind::Tag);
                let end = text[open_end..].find(close).map(|e| open_end + e);
                let inner_end = end.unwrap_or(text.len());
                let mut inner_stop = inner_end;
                if inner_stop > open_end && matches!(b[inner_stop - 1], b'-' | b'+') {
                    inner_stop -= 1;
                }
                jinja_inner(text, open_end, inner_stop, out);
                if let Some(e) = end {
                    out.push(inner_stop, e + 2, Kind::Tag);
                    i = e + 2;
                } else {
                    i = text.len();
                }
            }
            _ => i = at + 1,
        }
    }
}

fn jinja_inner(text: &str, from: usize, to: usize, out: &mut Out) {
    let b = text.as_bytes();
    let mut i = from;
    while i < to {
        let c = b[i];
        if c == b'"' || c == b'\'' {
            let mut j = i + 1;
            while j < to && b[j] != c {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            let e = (j + 1).min(to);
            out.push(i, e, Kind::String);
            i = e;
        } else if c.is_ascii_digit() {
            let mut s = Stream::new(&text[i..to]);
            eat_number(&mut s);
            let e = i + s.pos.max(1);
            out.push(i, e, Kind::Number);
            i = e;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let e = text[i..to]
                .find(|x: char| !(x.is_ascii_alphanumeric() || x == '_'))
                .map_or(to, |e| i + e);
            let word = &text[i..e];
            if matches!(word, "true" | "false" | "none" | "True" | "False" | "None") {
                out.push(i, e, Kind::Constant);
            } else if JINJA_KEYWORDS.contains(&word) {
                out.push(i, e, Kind::Keyword);
            }
            i = e;
        } else if b"=-+*/%<>!|~.".contains(&c) {
            out.push(i, i + 1, Kind::Operator);
            i += 1;
        } else {
            i += text[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each token's text and kind, for reading a test.
    fn read(language: CodeLanguage, text: &str) -> Vec<(String, Kind)> {
        tokens(language, text)
            .into_iter()
            .filter(|t| t.kind != Kind::Literal)
            .map(|t| (text[t.from..t.to].to_owned(), t.kind))
            .collect()
    }

    fn has(found: &[(String, Kind)], text: &str, kind: Kind) -> bool {
        found.iter().any(|(t, k)| t == text && *k == kind)
    }

    #[test]
    fn json_keys_strings_numbers_and_constants() {
        let found = read(
            CodeLanguage::Json,
            r#"{"name": "nightly-export", "retries": 2, "on": true, "x": null}"#,
        );
        assert!(has(&found, "\"name\"", Kind::Property));
        assert!(has(&found, "\"nightly-export\"", Kind::String));
        assert!(has(&found, "2", Kind::Number));
        assert!(has(&found, "true", Kind::Constant));
        assert!(has(&found, "null", Kind::Constant));
        assert!(has(&found, "{", Kind::Punctuation));
        assert!(has(&found, ":", Kind::Punctuation));
    }

    #[test]
    fn yaml_keys_strings_comments_anchors_and_dashes() {
        let text = "# a pipeline\nname: nightly-export\nsteps:\n  - id: \"fetch\" # first\n    base: &base\n    <<: *base\nrun: |\n  echo [x]\n";
        let found = read(CodeLanguage::Yaml, text);
        assert!(has(&found, "# a pipeline", Kind::Comment));
        assert!(has(&found, "name", Kind::Property));
        assert!(has(&found, "id", Kind::Property));
        assert!(has(&found, "\"fetch\"", Kind::String));
        assert!(has(&found, "# first", Kind::Comment));
        assert!(has(&found, "-", Kind::Punctuation));
        assert!(has(&found, "&base", Kind::Attribute));
        assert!(has(&found, "*base", Kind::Attribute));
        assert!(has(&found, "|", Kind::String));
        assert!(
            !found.iter().any(|(t, _)| t == "[" || t == "nightly-export"),
            "plain values and block text keep the ink"
        );
    }

    #[test]
    fn jq_fields_calls_strings_and_keywords() {
        let found = read(
            CodeLanguage::Jq,
            r#"def f: .rows | map(select(.state == "failed")) | if length > 0 then @csv else empty end # done"#,
        );
        assert!(has(&found, "def", Kind::Keyword));
        assert!(
            !found.iter().any(|(t, _)| t == "f"),
            "a definition's name keeps the ink"
        );
        assert!(has(&found, ".rows", Kind::Property));
        assert!(has(&found, "map", Kind::Function));
        assert!(has(&found, "==", Kind::Operator));
        assert!(has(&found, "\"", Kind::String));
        assert!(has(&found, "if", Kind::Keyword));
        assert!(has(&found, "0", Kind::Number));
        assert!(has(&found, "@csv", Kind::String));
        assert!(has(&found, "# done", Kind::Comment));
        let interpolated = read(CodeLanguage::Jq, r#""run \(.id) done""#);
        assert!(has(&interpolated, "\\(", Kind::String));
        assert!(has(&interpolated, ".id", Kind::Property));
    }

    #[test]
    fn markdown_headings_links_tags_code_and_marks() {
        let text = "# Nightly export\n\n- [x] See [[Runs#Failed]] and #harbor\n> quoted\nSome **strong**, *soft*, `code` and [a link](https://example.org).\n```\nraw [[not]]\n```";
        let found = read(CodeLanguage::Markdown, text);
        assert!(has(&found, "# Nightly export", Kind::Heading));
        assert!(has(&found, "- [x] ", Kind::Punctuation));
        assert!(has(&found, "[[Runs#Failed]]", Kind::Link));
        assert!(has(&found, "#harbor", Kind::Attribute));
        assert!(has(&found, "> ", Kind::Quote));
        assert!(has(&found, "**strong**", Kind::Strong));
        assert!(has(&found, "*soft*", Kind::Emphasis));
        assert!(has(&found, "`code`", Kind::String));
        assert!(has(&found, "[a link]", Kind::Link));
        assert!(has(&found, "(https://example.org)", Kind::Link));
        assert!(has(&found, "raw [[not]]", Kind::String));
        assert!(has(&found, "```", Kind::Punctuation));
    }

    #[test]
    fn sql_keywords_types_strings_and_comments() {
        let found = read(
            CodeLanguage::Sql,
            "SELECT id, CAST(total AS integer) FROM runs WHERE state = 'failed' AND x IS NULL -- recent\nLIMIT 10;",
        );
        assert!(has(&found, "SELECT", Kind::Keyword));
        assert!(has(&found, "integer", Kind::Type));
        assert!(has(&found, "'failed'", Kind::String));
        assert!(has(&found, "NULL", Kind::Constant));
        assert!(has(&found, "-- recent", Kind::Comment));
        assert!(has(&found, "10", Kind::Number));
        assert!(has(&found, "=", Kind::Operator));
        assert!(has(&found, ";", Kind::Punctuation));
        assert!(!found.iter().any(|(t, _)| t == "runs"));
    }

    #[test]
    fn shell_keywords_strings_flags_and_comments() {
        let found = read(
            CodeLanguage::Shell,
            "if true; then\n  curl -fsSL \"https://example.org/$JOB\" | grep --count ok # check\nfi",
        );
        assert!(has(&found, "if", Kind::Keyword));
        assert!(has(&found, "true", Kind::Constant));
        assert!(has(&found, "-fsSL", Kind::Attribute));
        assert!(has(&found, "--count", Kind::Attribute));
        assert!(has(&found, "\"https://example.org/$JOB\"", Kind::String));
        assert!(has(&found, "# check", Kind::Comment));
        assert!(has(&found, "fi", Kind::Keyword));
    }

    #[test]
    fn jinja_tags_keywords_and_comments() {
        let found = read(
            CodeLanguage::Jinja,
            "Hello {{ name | upper }}{% if count > 1 %}s{% endif %}{# note #}",
        );
        assert!(has(&found, "{{", Kind::Tag));
        assert!(has(&found, "}}", Kind::Tag));
        assert!(has(&found, "{%", Kind::Tag));
        assert!(has(&found, "if", Kind::Keyword));
        assert!(has(&found, "1", Kind::Number));
        assert!(has(&found, "|", Kind::Operator));
        assert!(has(&found, "endif", Kind::Keyword));
        assert!(has(&found, "{# note #}", Kind::Comment));
        assert!(!found.iter().any(|(t, _)| t == "Hello"));
    }

    #[test]
    fn text_has_no_tokens() {
        assert!(tokens(CodeLanguage::Text, "if true { 1 }").is_empty());
    }
}
