//! YAML as the web's `yaml` package (2.9) reads, edits and writes it, for the pipelines: a
//! document of maps, lists and scalars, each node with where it is written, its comments and the
//! style it was written in, read with `parse`, changed in place, and written again with
//! `Document::write` as `doc.toString({ flowCollectionPadding: false, lineWidth: 0 })` writes it.
//!
//! What it reads: block maps and lists (a list under a key at the key's own column too, and the
//! compact forms `- key: value` and `- - item`), flow lists and maps, plain, quoted and block
//! scalars, the core schema's nulls, truths and numbers, comments and blank lines, and a `---` at
//! the start. A document the web would refuse reads as broken: a key twice in a map, a tab as
//! indentation, a quote or bracket never closed, a nested map in a compact one, content where none
//! fits. Anchors, aliases, tags, directives, explicit `?` keys, `:` inside a flow list and a second
//! document are not read: such a document reads as broken here, where the web reads it.
//!
//! Every offset is a byte offset into the text, where the web counts UTF-16 units: the two agree
//! on ASCII.

use serde_json::{Map, Value};

use crate::js::number_to_string;

/// A scalar's value.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
}

/// How a scalar was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Plain,
    Single,
    Double,
    Literal,
    Folded,
}

/// How a number was written, when not in plain figures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Hex,
    Oct,
    Exp,
}

/// What a node is.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Scalar {
        value: Scalar,
        /// None for a node made here rather than read, which is written plain where it can be.
        style: Option<Style>,
        /// The text a plain scalar was read from, which a null or a truth is written as again.
        source: Option<String>,
        format: Option<Format>,
        /// How many places a float was written with, when it ended in a zero: `1.50`.
        min_fraction: usize,
    },
    Map(Vec<Pair>),
    Seq(Vec<Node>),
}

/// A node of a document: a scalar, a map or a list.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub kind: Kind,
    /// Whether a collection is written in brackets: none for one read as a block or made here.
    pub flow: Option<bool>,
    /// Where it is written: its start, the end of its value, and the end of what follows it on its
    /// line and the comments it takes; none for a node made here.
    pub range: Option<[usize; 3]>,
    pub comment_before: Option<String>,
    pub comment: Option<String>,
    pub space_before: bool,
}

/// A key and its value in a map.
#[derive(Clone, Debug, PartialEq)]
pub struct Pair {
    pub key: Node,
    pub value: Node,
}

/// A YAML document: its content, the comments before and after it, and whether it starts with `---`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    pub contents: Option<Node>,
    pub comment_before: Option<String>,
    pub comment: Option<String>,
    pub doc_start: bool,
    /// Whether the content is followed by `...`.
    pub doc_end: bool,
}

impl Node {
    fn of(kind: Kind) -> Self {
        Self {
            kind,
            flow: None,
            range: None,
            comment_before: None,
            comment: None,
            space_before: false,
        }
    }

    /// A scalar made here, as `createNode` makes one.
    pub fn scalar(value: Scalar) -> Self {
        Self::of(Kind::Scalar {
            value,
            style: None,
            source: None,
            format: None,
            min_fraction: 0,
        })
    }

    /// A text scalar made here.
    pub fn text(text: &str) -> Self {
        Self::scalar(Scalar::Str(text.to_owned()))
    }

    /// An empty map made here.
    pub fn map() -> Self {
        Self::of(Kind::Map(Vec::new()))
    }

    /// A list made here.
    pub fn seq(items: Vec<Node>) -> Self {
        Self::of(Kind::Seq(items))
    }

    /// A node made from a value, as `doc.createNode` makes it: a map's keys in their order.
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Null => Self::scalar(Scalar::Null),
            Value::Bool(b) => Self::scalar(Scalar::Bool(*b)),
            Value::Number(n) => Self::scalar(Scalar::Num(n.as_f64().unwrap_or(f64::NAN))),
            Value::String(s) => Self::text(s),
            Value::Array(items) => Self::seq(items.iter().map(Self::from_value).collect()),
            Value::Object(entries) => Self::of(Kind::Map(
                entries
                    .iter()
                    .map(|(key, value)| Pair {
                        key: Self::text(key),
                        value: Self::from_value(value),
                    })
                    .collect(),
            )),
        }
    }

    /// The scalar's value, if the node is one.
    pub fn scalar_value(&self) -> Option<&Scalar> {
        match &self.kind {
            Kind::Scalar { value, .. } => Some(value),
            _ => None,
        }
    }

    /// The text of a text scalar.
    pub fn as_str(&self) -> Option<&str> {
        match self.scalar_value() {
            Some(Scalar::Str(text)) => Some(text),
            _ => None,
        }
    }

    pub fn is_map(&self) -> bool {
        matches!(self.kind, Kind::Map(_))
    }

    pub fn is_seq(&self) -> bool {
        matches!(self.kind, Kind::Seq(_))
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self.kind, Kind::Scalar { .. })
    }

    /// A map's pairs; none for anything else.
    pub fn pairs(&self) -> &[Pair] {
        match &self.kind {
            Kind::Map(pairs) => pairs,
            _ => &[],
        }
    }

    /// A map's pairs to change.
    pub fn pairs_mut(&mut self) -> Option<&mut Vec<Pair>> {
        match &mut self.kind {
            Kind::Map(pairs) => Some(pairs),
            _ => None,
        }
    }

    /// A list's items; none for anything else.
    pub fn items(&self) -> &[Node] {
        match &self.kind {
            Kind::Seq(items) => items,
            _ => &[],
        }
    }

    /// A list's items to change.
    pub fn items_mut(&mut self) -> Option<&mut Vec<Node>> {
        match &mut self.kind {
            Kind::Seq(items) => Some(items),
            _ => None,
        }
    }

    fn find(&self, key: &str) -> Option<usize> {
        self.pairs()
            .iter()
            .position(|pair| pair.key.scalar_value() == Some(&Scalar::Str(key.to_owned())))
    }

    /// The value under a text key of a map, as `map.get(key, true)` finds it.
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.find(key).map(|at| &self.pairs()[at].value)
    }

    /// The value under a text key of a map, to change.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Node> {
        let at = self.find(key)?;
        self.pairs_mut().map(|pairs| &mut pairs[at].value)
    }

    /// Whether a map has a text key.
    pub fn has(&self, key: &str) -> bool {
        self.find(key).is_some()
    }

    /// Takes a key out of a map.
    pub fn delete(&mut self, key: &str) {
        if let Some(at) = self.find(key)
            && let Some(pairs) = self.pairs_mut()
        {
            pairs.remove(at);
        }
    }

    /// Sets a key of a map to a node, as `map.set(key, node)`: in its place when it is there, at the
    /// end when not.
    pub fn set(&mut self, key: &str, value: Node) {
        match self.find(key) {
            Some(at) => {
                if let Some(pairs) = self.pairs_mut() {
                    pairs[at].value = value;
                }
            }
            None => {
                if let Some(pairs) = self.pairs_mut() {
                    pairs.push(Pair {
                        key: Node::text(key),
                        value,
                    });
                }
            }
        }
    }

    /// Sets a key of a map to a plain value, as `map.set(key, value)`: a scalar there keeps its node,
    /// its style and comments, and takes the value.
    pub fn set_scalar(&mut self, key: &str, value: Scalar) {
        if let Some(Node {
            kind: Kind::Scalar { value: old, .. },
            ..
        }) = self.get_mut(key)
        {
            *old = value;
            return;
        }
        self.set(key, Node::scalar(value));
    }

    /// The node as `toJSON` gives it: a map's keys as texts, the last of a key written twice winning.
    pub fn to_js(&self) -> Value {
        match &self.kind {
            Kind::Scalar { value, .. } => scalar_js(value),
            Kind::Seq(items) => Value::Array(items.iter().map(Node::to_js).collect()),
            Kind::Map(pairs) => {
                let mut map = Map::new();
                for pair in pairs {
                    let key = match pair.key.scalar_value() {
                        Some(Scalar::Null) | None => String::new(),
                        Some(Scalar::Str(text)) => text.clone(),
                        Some(Scalar::Bool(b)) => b.to_string(),
                        Some(Scalar::Num(n)) => number_to_string(*n),
                    };
                    map.insert(key, pair.value.to_js());
                }
                Value::Object(map)
            }
        }
    }

    /// Calls `visit` on each scalar in and under the node, in the order they are written: a key
    /// before its value.
    pub fn visit_scalars_mut(&mut self, visit: &mut impl FnMut(&mut Node)) {
        match &mut self.kind {
            Kind::Scalar { .. } => visit(self),
            Kind::Seq(items) => {
                for item in items {
                    item.visit_scalars_mut(visit);
                }
            }
            Kind::Map(pairs) => {
                for pair in pairs {
                    pair.key.visit_scalars_mut(visit);
                    pair.value.visit_scalars_mut(visit);
                }
            }
        }
    }

    /// Calls `visit` on each scalar in and under the node, in the order they are written.
    pub fn visit_scalars(&self, visit: &mut impl FnMut(&Node)) {
        match &self.kind {
            Kind::Scalar { .. } => visit(self),
            Kind::Seq(items) => {
                for item in items {
                    item.visit_scalars(visit);
                }
            }
            Kind::Map(pairs) => {
                for pair in pairs {
                    pair.key.visit_scalars(visit);
                    pair.value.visit_scalars(visit);
                }
            }
        }
    }

    /// Calls `visit` on each list in and under the node, a list before the lists in it.
    pub fn visit_seqs_mut(&mut self, visit: &mut impl FnMut(&mut Node)) {
        if self.is_seq() {
            visit(self);
        }
        match &mut self.kind {
            Kind::Scalar { .. } => {}
            Kind::Seq(items) => {
                for item in items {
                    item.visit_seqs_mut(visit);
                }
            }
            Kind::Map(pairs) => {
                for pair in pairs {
                    pair.key.visit_seqs_mut(visit);
                    pair.value.visit_seqs_mut(visit);
                }
            }
        }
    }
}

/// A number as the web holds it in JSON: whole and safe ones as integers, NaN and the infinities as
/// null, as `JSON.stringify` writes them.
pub fn js_number(value: f64) -> Value {
    if !value.is_finite() {
        return Value::Null;
    }
    serde_json::from_str(&number_to_string(value)).unwrap_or(Value::Null)
}

fn scalar_js(value: &Scalar) -> Value {
    match value {
        Scalar::Null => Value::Null,
        Scalar::Bool(b) => Value::Bool(*b),
        Scalar::Num(n) => js_number(*n),
        Scalar::Str(text) => Value::String(text.clone()),
    }
}

impl Document {
    /// The document's content as `toJS` gives it: null when there is none.
    pub fn to_js(&self) -> Value {
        self.contents.as_ref().map_or(Value::Null, Node::to_js)
    }

    /// The value under a text key of the top map, as `doc.get(key, true)` finds it.
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.contents.as_ref().filter(|node| node.is_map())?.get(key)
    }

    /// Sets a key of the top map, as `doc.set`: a map is made when the document has none. False
    /// where the web throws: the content is a scalar or a list.
    pub fn set(&mut self, key: &str, value: Node) -> bool {
        match &mut self.contents {
            None => {
                let mut map = Node::map();
                map.set(key, value);
                self.contents = Some(map);
                true
            }
            Some(node) if node.is_map() => {
                node.set(key, value);
                true
            }
            Some(_) => false,
        }
    }
}

// --- Reading -------------------------------------------------------------------------------------

/// A line between content: blank, or a comment at an indent, each with where its line ends.
#[derive(Clone, Debug)]
enum Trivia {
    Blank { end: usize },
    Comment { indent: usize, text: String, end: usize },
}

impl Trivia {
    fn end(&self) -> usize {
        match self {
            Self::Blank { end } | Self::Comment { end, .. } => *end,
        }
    }
}

/// What the web's `resolveProps` reads from the comments and line breaks before a node.
enum Token {
    Indicator,
    Comment(String),
    Newline,
}

fn tokens_of(trivia: &[Trivia]) -> Vec<Token> {
    let mut out = Vec::new();
    for line in trivia {
        if let Trivia::Comment { text, .. } = line {
            out.push(Token::Comment(text.clone()));
        }
        out.push(Token::Newline);
    }
    out
}

/// The comment and the blank line before a node, as `resolveProps` makes them.
#[derive(Clone, Debug, Default)]
struct Props {
    comment: Option<String>,
    space_before: bool,
}

fn props(tokens: &[Token], start_on_newline: bool, seq_item: bool) -> Props {
    let mut space_before = false;
    let mut at_newline = start_on_newline;
    let mut comment = String::new();
    let mut sep = String::new();
    let mut found = false;
    for token in tokens {
        match token {
            Token::Indicator => found = true,
            Token::Comment(text) => {
                let text = if text.is_empty() { " " } else { text };
                if comment.is_empty() {
                    comment = text.to_owned();
                } else {
                    comment.push_str(&sep);
                    comment.push_str(text);
                }
                sep.clear();
                at_newline = false;
            }
            Token::Newline => {
                if at_newline {
                    if !comment.is_empty() {
                        comment.push('\n');
                    } else if !found || !seq_item {
                        space_before = true;
                    }
                } else {
                    sep.push('\n');
                }
                at_newline = true;
            }
        }
    }
    Props {
        comment: (!comment.is_empty()).then_some(comment),
        space_before,
    }
}

/// The comment a node's end takes, as `resolveEnd` joins the comments after it.
fn end_comment(comments: &[(String, usize)]) -> Option<String> {
    let mut comment = String::new();
    for (text, newlines_before) in comments {
        let text = if text.is_empty() { " " } else { text };
        if !comment.is_empty() {
            comment.push_str(&"\n".repeat(*newlines_before));
        }
        comment.push_str(text);
    }
    (!comment.is_empty()).then_some(comment)
}

fn apply(node: &mut Node, props: Props) {
    if props.space_before {
        node.space_before = true;
    }
    if let Some(comment) = props.comment {
        let empty = matches!(&node.kind, Kind::Scalar { source: Some(source), style: Some(Style::Plain), .. } if source.is_empty());
        if empty {
            node.comment = Some(comment);
        } else {
            node.comment_before = Some(comment);
        }
    }
}

struct Broken;

type Read<T> = Result<T, Broken>;

struct Reader<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

fn is_blank(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

impl Reader<'_> {
    fn byte(&self, at: usize) -> Option<u8> {
        self.bytes.get(at).copied()
    }

    fn line_end(&self, at: usize) -> usize {
        self.text[at..].find('\n').map_or(self.text.len(), |end| at + end)
    }

    fn next_line(&self, at: usize) -> usize {
        (self.line_end(at) + 1).min(self.text.len())
    }

    fn line_start(&self, at: usize) -> usize {
        self.text[..at].rfind('\n').map_or(0, |found| found + 1)
    }

    fn column(&self, at: usize) -> usize {
        at - self.line_start(at)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.text.len()
    }

    /// Whether `at` ends a line: a line break, a lone `\r` before one, or the end.
    fn eol(&self, at: usize) -> bool {
        match self.byte(at) {
            None | Some(b'\n') => true,
            Some(b'\r') => matches!(self.byte(at + 1), None | Some(b'\n')),
            _ => false,
        }
    }

    fn skip_blanks(&self, mut at: usize) -> usize {
        while self.byte(at).is_some_and(is_blank) {
            at += 1;
        }
        at
    }

    /// The blank and comment lines from `pos`, a line start, up to the next content.
    fn trivia(&mut self) -> Vec<Trivia> {
        let mut out = Vec::new();
        while !self.at_end() {
            let first = self.skip_blanks(self.pos);
            let end = self.next_line(self.pos);
            if self.eol(first) {
                out.push(Trivia::Blank { end });
            } else if self.byte(first) == Some(b'#') {
                let line_end = self.line_end(first);
                let text = self.text[first + 1..line_end].trim_end_matches('\r').to_owned();
                let indent = self.bytes[self.pos..first].iter().take_while(|b| **b == b' ').count();
                out.push(Trivia::Comment { indent, text, end });
            } else {
                break;
            }
            self.pos = end;
        }
        out
    }

    /// The indent of the content line at `pos` and where its content starts; refused when a tab is
    /// in its indentation.
    fn content_line(&self) -> Read<(usize, usize)> {
        let first = self.skip_blanks(self.pos);
        if self.bytes[self.pos..first].contains(&b'\t') {
            return Err(Broken);
        }
        Ok((first - self.pos, first))
    }

    /// Whether a `-` at `at` starts a list item: a space, a tab or the line's end after it.
    fn is_dash(&self, at: usize) -> bool {
        self.byte(at) == Some(b'-') && (self.eol(at + 1) || self.byte(at + 1).is_some_and(is_blank))
    }

    /// Whether the line from `at` is a document marker, `---` or `...`, at its start.
    fn is_marker(&self, at: usize) -> bool {
        self.column(at) == 0
            && (self.text[at..].starts_with("---") || self.text[at..].starts_with("..."))
            && (self.eol(at + 3) || self.byte(at + 3).is_some_and(is_blank))
    }

    /// Where a key starting at `at` ends, if the line holds one there: a plain or quoted scalar on
    /// one line followed by `:` and a space or the line's end.
    fn key_end(&self, at: usize) -> Option<usize> {
        let end = match self.byte(at)? {
            b'"' | b'\'' => {
                let quote = self.byte(at)?;
                let mut i = at + 1;
                loop {
                    match self.byte(i)? {
                        b'\n' => return None,
                        b'\\' if quote == b'"' => i += 2,
                        b'\'' if quote == b'\'' && self.byte(i + 1) == Some(b'\'') => i += 2,
                        b if b == quote => break i + 1,
                        _ => i += 1,
                    }
                }
            }
            b'[' | b'{' | b'#' | b'&' | b'*' | b'!' | b'|' | b'>' | b'%' | b'@' | b'`' | b',' | b']' | b'}' => {
                return None;
            }
            _ => {
                let mut i = at;
                loop {
                    if self.eol(i) {
                        return None;
                    }
                    let b = self.bytes[i];
                    if b == b':' && (self.eol(i + 1) || self.byte(i + 1).is_some_and(is_blank)) {
                        break i;
                    }
                    if b == b'#' && i > at && is_blank(self.bytes[i - 1]) {
                        return None;
                    }
                    i += 1;
                }
            }
        };
        let colon = self.skip_blanks(end);
        (self.byte(colon) == Some(b':') && (self.eol(colon + 1) || self.byte(colon + 1).is_some_and(is_blank)))
            .then_some(colon)
    }

    /// What follows a value on its line: spaces and a comment, and the line break.
    fn line_rest(&mut self, after: usize) -> Read<(Option<String>, usize)> {
        let at = self.skip_blanks(after);
        if self.eol(at) {
            let end = if at >= self.text.len() { at } else { self.next_line(at) };
            return Ok((None, end));
        }
        if self.byte(at) == Some(b'#') && at > after {
            let line_end = self.line_end(at);
            let text = self.text[at + 1..line_end].trim_end_matches('\r').to_owned();
            return Ok((Some(text), self.next_line(at)));
        }
        Err(Broken)
    }
}

/// A plain scalar's value as the core schema reads it: null, a truth, a number, or the text.
fn resolve_plain(text: &str) -> (Scalar, Option<Format>, usize) {
    let num = |digits: &str| -> f64 { digits.parse::<f64>().unwrap_or(f64::NAN) };
    let digits = |s: &str, radix: u32| !s.is_empty() && s.chars().all(|c| c.is_digit(radix));
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    if matches!(text, "" | "~" | "null" | "Null" | "NULL") {
        return (Scalar::Null, None, 0);
    }
    if matches!(text, "true" | "True" | "TRUE") {
        return (Scalar::Bool(true), None, 0);
    }
    if matches!(text, "false" | "False" | "FALSE") {
        return (Scalar::Bool(false), None, 0);
    }
    if let Some(octal) = text.strip_prefix("0o")
        && digits(octal, 8)
    {
        let value = octal
            .chars()
            .fold(0.0, |total, c| total * 8.0 + f64::from(c.to_digit(8).unwrap_or(0)));
        return (Scalar::Num(value), Some(Format::Oct), 0);
    }
    if digits(unsigned, 10) {
        return (Scalar::Num(num(text)), None, 0);
    }
    if let Some(hex) = text.strip_prefix("0x")
        && digits(hex, 16)
    {
        let value = hex
            .chars()
            .fold(0.0, |total, c| total * 16.0 + f64::from(c.to_digit(16).unwrap_or(0)));
        return (Scalar::Num(value), Some(Format::Hex), 0);
    }
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") {
        let value = if text.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        return (Scalar::Num(value), None, 0);
    }
    if matches!(text, ".nan" | ".NaN" | ".NAN") {
        return (Scalar::Num(f64::NAN), None, 0);
    }
    // A float: digits with a point among or before them, or an exponent.
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(at) => (&unsigned[..at], Some(&unsigned[at + 1..])),
        None => (unsigned, None),
    };
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, fraction)) => {
            (whole.is_empty() && digits(fraction, 10))
                || (digits(whole, 10) && (fraction.is_empty() || digits(fraction, 10)))
        }
        None => exponent.is_some() && digits(mantissa, 10),
    };
    if mantissa_ok {
        match exponent {
            Some(exponent) => {
                let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
                if digits(exponent, 10) {
                    return (Scalar::Num(num(text)), Some(Format::Exp), 0);
                }
            }
            None => {
                let places = match text.find('.') {
                    Some(dot) if text.ends_with('0') => text.len() - dot - 1,
                    _ => 0,
                };
                return (Scalar::Num(num(text)), None, places);
            }
        }
    }
    (Scalar::Str(text.to_owned()), None, 0)
}

/// A multi-line plain or single-quoted scalar's lines folded, as the web's `unfoldLines` folds them.
fn unfold_lines(source: &str) -> String {
    let mut lines = source.split('\n');
    let first = lines.next().unwrap_or("");
    let rest: Vec<&str> = lines.collect();
    if rest.is_empty() {
        return source.to_owned();
    }
    let trim_end = |line: &str| line.trim_end_matches([' ', '\t']).to_owned();
    let mut out = trim_end(first.trim_end_matches('\r'));
    let mut sep = " ";
    let (middle, last) = rest.split_at(rest.len() - 1);
    for line in middle {
        let line = line.trim_end_matches('\r').trim_matches([' ', '\t']);
        if line.is_empty() {
            if sep == "\n" {
                out.push('\n');
            } else {
                sep = "\n";
            }
        } else {
            out.push_str(sep);
            out.push_str(line);
            sep = " ";
        }
    }
    out.push_str(sep);
    out.push_str(last[0].trim_start_matches([' ', '\t']));
    out
}

/// A double-quoted scalar's value from its source with the quotes, as the web reads its escapes and
/// folds its lines; none when an escape is not one.
fn double_quoted(source: &str) -> Option<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::new();
    let mut i = 1;
    while i + 1 < chars.len() {
        let c = chars[i];
        if c == '\r' && chars.get(i + 1) == Some(&'\n') {
            i += 1;
            continue;
        }
        if c == '\n' {
            let mut fold = String::new();
            while let Some(&next) = chars.get(i + 1) {
                if !matches!(next, ' ' | '\t' | '\n' | '\r') || (next == '\r' && chars.get(i + 2) != Some(&'\n')) {
                    break;
                }
                if next == '\n' {
                    fold.push('\n');
                }
                i += 1;
            }
            out.push_str(if fold.is_empty() { " " } else { &fold });
            i += 1;
            continue;
        }
        if c == '\\' {
            i += 1;
            let next = *chars.get(i)?;
            let code = match next {
                '0' => Some('\0'),
                'a' => Some('\u{7}'),
                'b' => Some('\u{8}'),
                'e' => Some('\u{1b}'),
                'f' => Some('\u{c}'),
                'n' => Some('\n'),
                'r' => Some('\r'),
                't' | '\t' => Some('\t'),
                'v' => Some('\u{b}'),
                'N' => Some('\u{85}'),
                '_' => Some('\u{a0}'),
                'L' => Some('\u{2028}'),
                'P' => Some('\u{2029}'),
                ' ' => Some(' '),
                '"' => Some('"'),
                '/' => Some('/'),
                '\\' => Some('\\'),
                _ => None,
            };
            if let Some(code) = code {
                out.push(code);
            } else if next == '\n' || (next == '\r' && chars.get(i + 1) == Some(&'\n')) {
                if next == '\r' {
                    i += 1;
                }
                while matches!(chars.get(i + 1), Some(' ' | '\t')) {
                    i += 1;
                }
            } else if matches!(next, 'x' | 'u' | 'U') {
                let length = match next {
                    'x' => 2,
                    'u' => 4,
                    _ => 8,
                };
                let hex: String = chars.get(i + 1..i + 1 + length)?.iter().collect();
                let code = u32::from_str_radix(&hex, 16).ok()?;
                out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                i += length;
            } else {
                return None;
            }
            i += 1;
            continue;
        }
        if c == ' ' || c == '\t' {
            let start = i;
            while matches!(chars.get(i + 1), Some(' ' | '\t')) {
                i += 1;
            }
            let next = chars.get(i + 1);
            let breaks = next == Some(&'\n') || (next == Some(&'\r') && chars.get(i + 2) == Some(&'\n'));
            if !breaks {
                out.extend(&chars[start..=i]);
            }
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    Some(out)
}

/// A block scalar's value from its header and the lines under it, as `resolveBlockScalar` reads it.
fn block_value(literal: bool, chomp: Option<u8>, explicit: usize, base: usize, source: &str) -> Option<String> {
    let mut lines: Vec<(usize, &str)> = Vec::new();
    for line in source.split('\n') {
        let spaces = line.len() - line.trim_start_matches(' ').len();
        lines.push((spaces, &line[spaces..]));
    }
    if source.is_empty() {
        lines.clear();
    }
    let blank = |content: &str| content.is_empty() || content == "\r";
    let mut chomp_start = lines.len();
    for at in (0..lines.len()).rev() {
        if blank(lines[at].1) {
            chomp_start = at;
        } else {
            break;
        }
    }
    if chomp_start == 0 {
        return Some(if chomp == Some(b'+') && !lines.is_empty() {
            "\n".repeat((lines.len() - 1).max(1))
        } else {
            String::new()
        });
    }
    let mut trim = base + explicit;
    let mut content_start = 0;
    for (at, (indent, content)) in lines.iter().enumerate().take(chomp_start) {
        if blank(content) {
            if explicit == 0 && *indent > trim {
                trim = *indent;
            }
        } else {
            if *indent < trim {
                return None;
            }
            if explicit == 0 {
                trim = *indent;
            }
            content_start = at;
            break;
        }
    }
    for at in (chomp_start..lines.len()).rev() {
        if lines[at].0 > trim {
            chomp_start = at + 1;
        }
    }
    let pad = |indent: usize| " ".repeat(indent.saturating_sub(trim));
    let mut value = String::new();
    for (indent, _) in lines.iter().take(content_start) {
        value.push_str(&pad(*indent));
        value.push('\n');
    }
    let mut sep = "";
    let mut more_indented = false;
    for (indent, content) in lines.iter().take(chomp_start).skip(content_start) {
        let content = content.strip_suffix('\r').unwrap_or(content);
        if !content.is_empty() && *indent < trim {
            return None;
        }
        if literal {
            value.push_str(sep);
            value.push_str(&pad(*indent));
            value.push_str(content);
            sep = "\n";
        } else if *indent > trim || content.starts_with('\t') {
            if sep == " " {
                sep = "\n";
            } else if !more_indented && sep == "\n" {
                sep = "\n\n";
            }
            value.push_str(sep);
            value.push_str(&pad(*indent));
            value.push_str(content);
            sep = "\n";
            more_indented = true;
        } else if content.is_empty() {
            if sep == "\n" {
                value.push('\n');
            } else {
                sep = "\n";
            }
        } else {
            value.push_str(sep);
            value.push_str(content);
            sep = " ";
            more_indented = false;
        }
    }
    match chomp {
        Some(b'-') => {}
        Some(b'+') => {
            for (indent, _) in lines.iter().skip(chomp_start) {
                value.push('\n');
                value.push_str(&pad(*indent));
            }
            if !value.ends_with('\n') {
                value.push('\n');
            }
        }
        _ => value.push('\n'),
    }
    Some(value)
}

/// Whether every comment among the lines left after a collection is less indented than it, so the
/// lines go to what holds it; a collection at the left edge hands them on always.
fn hands_on(trivia: &[Trivia], indent: usize) -> bool {
    indent == 0
        || trivia.iter().all(|line| match line {
            Trivia::Comment { indent: at, .. } => *at < indent,
            Trivia::Blank { .. } => true,
        })
}

/// Ends a block collection: the lines after it, up to the next content at `next`, are its own
/// trailing comment, or handed on.
fn close(node: &mut Node, indent: usize, trivia: Vec<Trivia>, next: usize) -> Vec<Trivia> {
    if trivia.is_empty() || hands_on(&trivia, indent) {
        return trivia;
    }
    let found = props(&tokens_of(&trivia), true, false);
    if let Some(comment) = found.comment {
        node.comment = Some(match node.comment.take() {
            Some(before) => format!("{before}\n{comment}"),
            None => comment,
        });
    }
    if let Some(range) = &mut node.range {
        range[2] = next;
    }
    Vec::new()
}

impl Reader<'_> {
    /// Takes the comment lines more indented than a block collection right after one of its scalar
    /// or flow values into that value's comment, as the web appends them to the value's end.
    fn absorb(&self, node: &mut Node, indent: usize, trivia: Vec<Trivia>) -> Vec<Trivia> {
        let mut taken = 0;
        for (at, line) in trivia.iter().enumerate() {
            match line {
                Trivia::Comment { indent: col, .. } if *col > indent => taken = at + 1,
                Trivia::Comment { .. } => break,
                Trivia::Blank { .. } => {}
            }
        }
        if taken == 0 {
            return trivia;
        }
        let mut comments: Vec<(String, usize)> = node.comment.take().map(|c| vec![(c, 0)]).unwrap_or_default();
        let mut newlines = usize::from(!comments.is_empty());
        for line in &trivia[..taken] {
            match line {
                Trivia::Comment { text, .. } => {
                    comments.push((text.clone(), newlines));
                    newlines = 1;
                }
                Trivia::Blank { .. } => newlines += 1,
            }
        }
        node.comment = end_comment(&comments);
        if let Some(range) = &mut node.range {
            range[2] = trivia[taken - 1].end();
        }
        trivia[taken..].to_vec()
    }

    /// A node starting at `pos`, at the first content of a line or after an indicator on one: a
    /// block list, a block map, or a value on the line. Answers with it and the lines after it.
    fn block_node(&mut self, parent: isize, props_in: Props) -> Read<(Node, Vec<Trivia>)> {
        let at = self.pos;
        let col = self.column(at);
        let (mut node, rest) = if self.is_dash(at) {
            self.block_seq(col)?
        } else if self.key_end(at).is_some() {
            self.block_map(col)?
        } else {
            let (node, rest) = self.inline(parent)?;
            let indent = usize::try_from(parent).unwrap_or(0);
            let mut node = node;
            let rest = if parent >= 0 && (node.is_scalar() || node.flow == Some(true)) {
                self.absorb(&mut node, indent, rest)
            } else {
                rest
            };
            (node, rest)
        };
        apply(&mut node, props_in);
        Ok((node, rest))
    }

    fn block_map(&mut self, indent: usize) -> Read<(Node, Vec<Trivia>)> {
        let mut pairs: Vec<Pair> = Vec::new();
        let mut key_props = Props::default();
        let start = self.pos;
        loop {
            let key_at = self.pos;
            let colon = self.key_end(key_at).ok_or(Broken)?;
            let mut key = self.inline_key(key_at, colon)?;
            apply(&mut key, std::mem::take(&mut key_props));
            if let Some(value) = key.scalar_value()
                && pairs.iter().any(|pair| pair.key.scalar_value() == Some(value))
            {
                return Err(Broken);
            }
            self.pos = colon + 1;
            let (value, rest) = self.map_value(indent, colon + 1)?;
            pairs.push(Pair { key, value });
            let mut rest = rest;
            if self.at_end() {
                return Ok(self.finish_map(start, pairs, indent, rest, self.text.len()));
            }
            let (col, first) = self.content_line()?;
            if col < indent || self.is_marker(first) {
                return Ok(self.finish_map(start, pairs, indent, rest, first));
            }
            if col > indent || self.is_dash(first) || self.key_end(first).is_none() {
                if col == indent && self.is_dash(first) && indent == 0 {
                    return Err(Broken);
                }
                return Err(Broken);
            }
            key_props = props(&tokens_of(&std::mem::take(&mut rest)), true, false);
            self.pos = first;
        }
    }

    fn finish_map(
        &self,
        start: usize,
        pairs: Vec<Pair>,
        indent: usize,
        rest: Vec<Trivia>,
        next: usize,
    ) -> (Node, Vec<Trivia>) {
        let end = pairs
            .last()
            .and_then(|pair| pair.value.range)
            .map_or(start, |range| range[2]);
        let mut node = Node::of(Kind::Map(pairs));
        node.range = Some([start, end, end]);
        let rest = close(&mut node, indent, rest, next);
        (node, rest)
    }

    fn inline_key(&self, at: usize, colon: usize) -> Read<Node> {
        let end = self.text[at..colon].trim_end_matches([' ', '\t']).len() + at;
        let raw = &self.text[at..end];
        let (value, style, source, format, min_fraction) = match self.bytes[at] {
            b'"' => (
                Scalar::Str(double_quoted(raw).ok_or(Broken)?),
                Style::Double,
                None,
                None,
                0,
            ),
            b'\'' => (
                Scalar::Str(raw[1..raw.len() - 1].replace("''", "'")),
                Style::Single,
                None,
                None,
                0,
            ),
            _ => {
                let (value, format, places) = resolve_plain(raw);
                (value, Style::Plain, Some(raw.to_owned()), format, places)
            }
        };
        let source = source.or_else(|| match &value {
            Scalar::Str(text) => Some(text.clone()),
            _ => None,
        });
        let mut node = Node::of(Kind::Scalar {
            value,
            style: Some(style),
            source,
            format,
            min_fraction,
        });
        node.range = Some([at, end, end]);
        Ok(node)
    }

    /// The value after a key's `:` at `after`, in a map at `indent`.
    fn map_value(&mut self, indent: usize, after: usize) -> Read<(Node, Vec<Trivia>)> {
        let at = self.skip_blanks(after);
        if self.eol(at) || (self.byte(at) == Some(b'#') && at > after) {
            let (line_comment, next) = self.line_rest(after)?;
            self.pos = next;
            let trivia = self.trivia();
            let next_content = if self.at_end() {
                None
            } else {
                let (col, first) = self.content_line()?;
                Some((col, first))
            };
            let nested = next_content.filter(|(col, first)| {
                !self.is_marker(*first) && (*col > indent || (*col == indent && self.is_dash(*first)))
            });
            match nested {
                None => {
                    let mut empty = empty_scalar(after);
                    if let Some(comment) = line_comment {
                        empty.comment = Some(comment);
                    }
                    Ok((empty, trivia))
                }
                Some((col, first)) => {
                    let mut tokens = vec![Token::Indicator];
                    if let Some(comment) = line_comment {
                        tokens.push(Token::Comment(comment));
                    }
                    tokens.push(Token::Newline);
                    tokens.extend(tokens_of(&trivia));
                    let found = props(&tokens, false, false);
                    self.pos = first;
                    if col == indent {
                        let (mut node, rest) = self.block_seq(col)?;
                        apply(&mut node, found);
                        Ok((node, rest))
                    } else {
                        self.block_node(indent as isize, found)
                    }
                }
            }
        } else {
            if self.is_dash(at) || self.key_end(at).is_some() {
                return Err(Broken);
            }
            self.pos = at;
            let (mut node, rest) = self.inline(indent as isize)?;
            let rest = if node.is_scalar() || node.flow == Some(true) {
                self.absorb(&mut node, indent, rest)
            } else {
                rest
            };
            Ok((node, rest))
        }
    }

    fn block_seq(&mut self, indent: usize) -> Read<(Node, Vec<Trivia>)> {
        let start = self.pos;
        let mut items: Vec<Node> = Vec::new();
        let mut item_props = Props::default();
        loop {
            let dash = self.pos;
            let after = dash + 1;
            let at = self.skip_blanks(after);
            let (mut node, rest) = if self.eol(at) || self.byte(at) == Some(b'#') {
                let (line_comment, next) = self.line_rest(after)?;
                self.pos = next;
                let trivia = self.trivia();
                let nested = if self.at_end() {
                    None
                } else {
                    let (col, first) = self.content_line()?;
                    (col > indent && !self.is_marker(first)).then_some(first)
                };
                match nested {
                    None => {
                        let mut empty = empty_scalar(after);
                        let mut found = item_props.clone();
                        if let Some(comment) = line_comment {
                            found.comment = Some(match found.comment {
                                Some(before) => format!("{before}\n{comment}"),
                                None => comment,
                            });
                        }
                        if let Some(comment) = found.comment.take() {
                            empty.comment = Some(comment);
                        }
                        empty.space_before = found.space_before;
                        (empty, trivia)
                    }
                    Some(first) => {
                        let mut tokens = Vec::new();
                        if let Some(comment) = item_props.comment.clone() {
                            tokens.push(Token::Comment(comment));
                            tokens.push(Token::Newline);
                        }
                        tokens.push(Token::Indicator);
                        if let Some(comment) = line_comment {
                            tokens.push(Token::Comment(comment));
                        }
                        tokens.push(Token::Newline);
                        tokens.extend(tokens_of(&trivia));
                        let mut found = props(&tokens, true, true);
                        found.space_before |= item_props.space_before;
                        self.pos = first;
                        self.block_node(indent as isize, found)?
                    }
                }
            } else {
                self.pos = at;
                let col = self.column(at);
                if self.is_dash(at) {
                    let (mut node, rest) = self.block_seq(col)?;
                    apply(&mut node, item_props.clone());
                    (node, rest)
                } else if self.key_end(at).is_some() {
                    let (mut node, rest) = self.block_map(col)?;
                    apply(&mut node, item_props.clone());
                    (node, rest)
                } else {
                    let (mut node, rest) = self.inline(indent as isize)?;
                    let rest = if node.is_scalar() || node.flow == Some(true) {
                        self.absorb(&mut node, indent, rest)
                    } else {
                        rest
                    };
                    apply(&mut node, item_props.clone());
                    (node, rest)
                }
            };
            if node.range.is_none() {
                node.range = Some([after, after, after]);
            }
            items.push(node);
            let finish = |items: Vec<Node>, rest: Vec<Trivia>, next: usize| {
                let end = items.last().and_then(|item| item.range).map_or(start, |range| range[2]);
                let mut node = Node::of(Kind::Seq(items));
                node.range = Some([start, end, end]);
                let rest = close(&mut node, indent, rest, next);
                (node, rest)
            };
            if self.at_end() {
                return Ok(finish(items, rest, self.text.len()));
            }
            let (col, first) = self.content_line()?;
            if self.is_marker(first) {
                return Ok(finish(items, rest, first));
            }
            if col == indent && self.is_dash(first) {
                let mut tokens = tokens_of(&rest);
                tokens.push(Token::Indicator);
                item_props = props(&tokens, true, true);
                self.pos = first;
                continue;
            }
            if col > indent {
                return Err(Broken);
            }
            return Ok(finish(items, rest, first));
        }
    }

    /// A value written on the line at `pos`: a flow collection, a quoted, block or plain scalar, with
    /// what follows it on the line. Lines more indented than `parent` carry a plain or quoted scalar on.
    fn inline(&mut self, parent: isize) -> Read<(Node, Vec<Trivia>)> {
        let at = self.pos;
        match self.byte(at).ok_or(Broken)? {
            b'[' | b'{' => {
                let (mut node, end) = self.flow_collection(at)?;
                let (comment, next) = self.line_rest(end)?;
                node.comment = comment;
                if let Some(range) = &mut node.range {
                    range[2] = next;
                }
                self.pos = next;
                Ok((node, self.trivia()))
            }
            b'"' | b'\'' => {
                let end = self.quoted_end(at)?;
                let raw = &self.text[at..end];
                let (value, style) = if self.bytes[at] == b'"' {
                    (double_quoted(raw).ok_or(Broken)?, Style::Double)
                } else {
                    (unfold_lines(&raw[1..raw.len() - 1]).replace("''", "'"), Style::Single)
                };
                let (comment, next) = self.line_rest(end)?;
                let mut node = Node::of(Kind::Scalar {
                    source: Some(value.clone()),
                    value: Scalar::Str(value),
                    style: Some(style),
                    format: None,
                    min_fraction: 0,
                });
                node.comment = comment;
                node.range = Some([at, end, next]);
                self.pos = next;
                Ok((node, self.trivia()))
            }
            b'|' | b'>' => self.block_scalar(at, parent),
            b'&' | b'*' | b'!' | b'%' | b'@' | b'`' | b',' | b']' | b'}' | b'#' => Err(Broken),
            b'?' | b':' if self.eol(at + 1) || self.byte(at + 1).is_some_and(is_blank) => Err(Broken),
            _ => self.plain(at, parent),
        }
    }

    fn quoted_end(&self, at: usize) -> Read<usize> {
        let quote = self.bytes[at];
        let mut i = at + 1;
        loop {
            match self.byte(i).ok_or(Broken)? {
                b'\\' if quote == b'"' => i += 2,
                b'\'' if quote == b'\'' && self.byte(i + 1) == Some(b'\'') => i += 2,
                b if b == quote => return Ok(i + 1),
                b'\n' => {
                    let next = i + 1;
                    if self.is_marker(next) {
                        return Err(Broken);
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        }
    }

    /// Where a plain scalar's text on one line ends, from `at`: before a comment, or the line's end
    /// with the spaces before it left out. Refused when a `: ` makes it a key.
    fn plain_line_end(&self, at: usize) -> Read<(usize, bool)> {
        let mut i = at;
        let mut end = at;
        loop {
            if self.eol(i) {
                return Ok((end, false));
            }
            let b = self.bytes[i];
            if b == b'#' && i > at && is_blank(self.bytes[i - 1]) {
                return Ok((end, true));
            }
            if b == b':' && (self.eol(i + 1) || self.byte(i + 1).is_some_and(is_blank)) {
                return Err(Broken);
            }
            i += 1;
            if !is_blank(b) {
                end = i;
            }
        }
    }

    fn plain(&mut self, at: usize, parent: isize) -> Read<(Node, Vec<Trivia>)> {
        let (mut end, mut commented) = self.plain_line_end(at)?;
        // Lines more indented than the parent carry the scalar on, until a comment.
        while !commented {
            let saved = self.pos;
            self.pos = self.next_line(end);
            if self.pos == self.line_start(end) || self.pos <= end {
                self.pos = saved;
                break;
            }
            let _ = self.trivia_blank_only();
            if self.at_end() {
                self.pos = saved;
                break;
            }
            let first = self.skip_blanks(self.pos);
            let col = first - self.pos;
            if (col as isize) <= parent || self.byte(first) == Some(b'#') || self.is_marker(first) {
                self.pos = saved;
                break;
            }
            if self.bytes[self.pos..first].contains(&b'\t') {
                return Err(Broken);
            }
            let (line_end, comment) = self.plain_line_end(first)?;
            end = line_end;
            commented = comment;
        }
        let raw = &self.text[at..end];
        let folded = unfold_lines(raw);
        let (value, format, min_fraction) = resolve_plain(&folded);
        let (comment, next) = self.line_rest(end)?;
        let mut node = Node::of(Kind::Scalar {
            value,
            style: Some(Style::Plain),
            source: Some(folded),
            format,
            min_fraction,
        });
        node.comment = comment;
        node.range = Some([at, end, next]);
        self.pos = next;
        Ok((node, self.trivia()))
    }

    /// Skips blank lines from `pos`, a line start.
    fn trivia_blank_only(&mut self) -> usize {
        let mut count = 0;
        while !self.at_end() {
            let first = self.skip_blanks(self.pos);
            if !self.eol(first) {
                break;
            }
            self.pos = self.next_line(self.pos);
            count += 1;
            if first >= self.text.len() {
                break;
            }
        }
        count
    }

    fn block_scalar(&mut self, at: usize, parent: isize) -> Read<(Node, Vec<Trivia>)> {
        let literal = self.bytes[at] == b'|';
        let mut i = at + 1;
        let mut chomp = None;
        let mut explicit = 0;
        while let Some(b) = self.byte(i) {
            match b {
                b'-' | b'+' if chomp.is_none() => chomp = Some(b),
                b'1'..=b'9' if explicit == 0 => explicit = usize::from(b - b'0'),
                _ => break,
            }
            i += 1;
        }
        let (comment, next) = self.line_rest(i)?;
        let base = usize::try_from(parent).unwrap_or(0);
        // The lines under the header: blank ones, and those more indented than the parent.
        let content_start = next;
        let mut end = next;
        let mut last_content_end = next;
        let mut cursor = next;
        while cursor < self.text.len() {
            let first = self.skip_blanks(cursor);
            let line_next = self.next_line(cursor);
            let col = first - cursor;
            if self.eol(first) {
                cursor = line_next;
                end = cursor;
                continue;
            }
            if (col as isize) <= parent || self.is_marker(first) {
                break;
            }
            cursor = line_next;
            end = cursor;
            last_content_end = cursor;
        }
        let source_end = if chomp == Some(b'+') { end } else { last_content_end };
        let source = &self.text[content_start..source_end];
        let value = block_value(literal, chomp, explicit, base, source).ok_or(Broken)?;
        let mut node = Node::of(Kind::Scalar {
            source: Some(value.clone()),
            value: Scalar::Str(value),
            style: Some(if literal { Style::Literal } else { Style::Folded }),
            format: None,
            min_fraction: 0,
        });
        node.comment = comment;
        node.range = Some([at, source_end, source_end]);
        self.pos = source_end;
        Ok((node, self.trivia()))
    }

    /// Skips spaces, line breaks and comments inside a flow collection.
    fn flow_space(&self, mut at: usize) -> usize {
        loop {
            match self.byte(at) {
                Some(b' ' | b'\t' | b'\n' | b'\r') => at += 1,
                Some(b'#') if at > 0 && matches!(self.bytes[at - 1], b' ' | b'\t' | b'\n') => {
                    at = self.line_end(at);
                }
                _ => return at,
            }
        }
    }

    /// A flow list or map from its bracket at `at`, and where it ends.
    fn flow_collection(&self, at: usize) -> Read<(Node, usize)> {
        let map = self.bytes[at] == b'{';
        let close = if map { b'}' } else { b']' };
        let mut i = self.flow_space(at + 1);
        let mut items = Vec::new();
        let mut pairs = Vec::new();
        loop {
            match self.byte(i).ok_or(Broken)? {
                b if b == close => {
                    i += 1;
                    break;
                }
                b',' => return Err(Broken),
                _ => {}
            }
            let (mut first, end) = self.flow_item(i)?;
            let after = self.flow_space(end);
            if let Some(range) = &mut first.range {
                range[2] = after;
            }
            i = after;
            if map {
                if self.byte(i) != Some(b':') {
                    return Err(Broken);
                }
                let value_at = self.flow_space(i + 1);
                let (mut value, end) = self.flow_item(value_at)?;
                let after = self.flow_space(end);
                if let Some(range) = &mut value.range {
                    range[2] = after;
                }
                if pairs
                    .iter()
                    .any(|pair: &Pair| pair.key.scalar_value() == first.scalar_value())
                {
                    return Err(Broken);
                }
                pairs.push(Pair { key: first, value });
                i = after;
            } else {
                if self.byte(i) == Some(b':') {
                    return Err(Broken);
                }
                items.push(first);
            }
            match self.byte(i).ok_or(Broken)? {
                b',' => {
                    // A comment after the comma, on its line, is the item's.
                    let at = self.skip_blanks(i + 1);
                    if self.byte(at) == Some(b'#') {
                        let text = self.text[at + 1..self.line_end(at)].trim_end_matches('\r').to_owned();
                        let last = if map {
                            pairs.last_mut().map(|pair: &mut Pair| &mut pair.value)
                        } else {
                            items.last_mut()
                        };
                        if let Some(last) = last {
                            last.comment = Some(text);
                        }
                    }
                    i = self.flow_space(i + 1);
                }
                b if b == close => {}
                _ => return Err(Broken),
            }
        }
        let mut node = Node::of(if map { Kind::Map(pairs) } else { Kind::Seq(items) });
        node.flow = Some(true);
        node.range = Some([at, i, i]);
        Ok((node, i))
    }

    /// One item of a flow collection at `at`, and where its value ends.
    fn flow_item(&self, at: usize) -> Read<(Node, usize)> {
        match self.byte(at).ok_or(Broken)? {
            b'[' | b'{' => self.flow_collection(at),
            b'"' | b'\'' => {
                let end = self.quoted_end(at)?;
                let raw = &self.text[at..end];
                let (value, style) = if self.bytes[at] == b'"' {
                    (double_quoted(raw).ok_or(Broken)?, Style::Double)
                } else {
                    (unfold_lines(&raw[1..raw.len() - 1]).replace("''", "'"), Style::Single)
                };
                let mut node = Node::of(Kind::Scalar {
                    source: Some(value.clone()),
                    value: Scalar::Str(value),
                    style: Some(style),
                    format: None,
                    min_fraction: 0,
                });
                node.range = Some([at, end, end]);
                Ok((node, end))
            }
            b'&' | b'*' | b'!' | b'%' | b'@' | b'`' | b'|' | b'>' | b'#' | b',' | b']' | b'}' => Err(Broken),
            _ => {
                let mut i = at;
                let mut end = at;
                while let Some(b) = self.byte(i) {
                    if matches!(b, b',' | b'[' | b']' | b'{' | b'}' | b'\n' | b'\r') {
                        break;
                    }
                    if b == b':'
                        && (self
                            .byte(i + 1)
                            .is_none_or(|next| matches!(next, b' ' | b'\t' | b',' | b']' | b'}' | b'\n')))
                    {
                        break;
                    }
                    if b == b'#' && i > at && is_blank(self.bytes[i - 1]) {
                        break;
                    }
                    i += 1;
                    if !is_blank(b) {
                        end = i;
                    }
                }
                let raw = &self.text[at..end];
                let (value, format, min_fraction) = resolve_plain(raw);
                let mut node = Node::of(Kind::Scalar {
                    value,
                    style: Some(Style::Plain),
                    source: Some(raw.to_owned()),
                    format,
                    min_fraction,
                });
                node.range = Some([at, end, end]);
                Ok((node, end))
            }
        }
    }
}

fn empty_scalar(at: usize) -> Node {
    let mut node = Node::of(Kind::Scalar {
        value: Scalar::Null,
        style: Some(Style::Plain),
        source: Some(String::new()),
        format: None,
        min_fraction: 0,
    });
    node.range = Some([at, at, at]);
    node
}

/// The comment before a document's content, as `parsePrelude` joins it, and whether a blank line
/// came after it.
fn prelude(trivia: &[Trivia]) -> (Option<String>, bool) {
    let mut comment = String::new();
    let mut at_comment = false;
    let mut after_empty = false;
    for line in trivia {
        if let Trivia::Comment { text, .. } = line {
            if !comment.is_empty() {
                comment.push_str(if after_empty { "\n\n" } else { "\n" });
            }
            comment.push_str(if text.is_empty() { " " } else { text });
            at_comment = true;
            after_empty = false;
        }
        // The line's break.
        if !at_comment {
            after_empty = true;
        }
        at_comment = false;
    }
    ((!comment.is_empty()).then_some(comment), after_empty)
}

/// Reads a YAML document; none when the web's `parseDocument` finds an error in it.
pub fn parse(text: &str) -> Option<Document> {
    let mut reader = Reader {
        text,
        bytes: text.as_bytes(),
        pos: 0,
    };
    read_document(&mut reader).ok()
}

fn read_document(reader: &mut Reader) -> Read<Document> {
    let before = reader.trivia();
    let mut doc = Document::default();
    let mut start_props = Props::default();
    if !reader.at_end() && reader.text[reader.pos..].starts_with("---") && reader.is_marker(reader.pos) {
        doc.doc_start = true;
        let (comment, next) = reader.line_rest(reader.pos + 3)?;
        reader.pos = next;
        let trivia = reader.trivia();
        let mut tokens = vec![Token::Indicator];
        if let Some(comment) = comment {
            tokens.push(Token::Comment(comment));
        }
        tokens.push(Token::Newline);
        tokens.extend(tokens_of(&trivia));
        start_props = props(&tokens, false, false);
    }
    let mut rest = Vec::new();
    if !reader.at_end() {
        let (_, first) = reader.content_line()?;
        if reader.is_marker(first) {
            return Err(Broken);
        }
        reader.pos = first;
        let (node, after) = reader.block_node(-1, start_props)?;
        doc.contents = Some(node);
        rest = after;
        if !reader.at_end() {
            let (_, first) = reader.content_line()?;
            if !(reader.text[first..].starts_with("...") && reader.is_marker(first)) {
                return Err(Broken);
            }
            doc.doc_end = true;
            let (comment, next) = reader.line_rest(first + 3)?;
            reader.pos = next;
            rest.extend(comment.map(|text| Trivia::Comment {
                indent: 0,
                text,
                end: next,
            }));
            rest.extend(reader.trivia());
            if !reader.at_end() {
                return Err(Broken);
            }
        }
    }
    // The comments after the content end the document.
    let mut comments = Vec::new();
    let mut newlines = 0;
    for line in &rest {
        match line {
            Trivia::Comment { text, .. } => {
                comments.push((text.clone(), newlines));
                newlines = 1;
            }
            Trivia::Blank { .. } => {
                if !comments.is_empty() {
                    newlines += 1;
                }
            }
        }
    }
    doc.comment = end_comment(&comments);
    let (comment, after_empty) = prelude(&before);
    if let Some(comment) = comment {
        let join = |before: Option<String>| match before {
            Some(before) => format!("{comment}\n{before}"),
            None => comment.clone(),
        };
        match &mut doc.contents {
            None => doc.comment_before = Some(comment.clone()),
            _ if after_empty || doc.doc_start => doc.comment_before = Some(comment.clone()),
            Some(node) if node.flow != Some(true) && (!node.pairs().is_empty() || !node.items().is_empty()) => {
                let first = match &mut node.kind {
                    Kind::Map(pairs) => &mut pairs[0].key,
                    Kind::Seq(items) => &mut items[0],
                    Kind::Scalar { .. } => unreachable!("a scalar has no items"),
                };
                first.comment_before = Some(join(first.comment_before.take()));
            }
            Some(node) => node.comment_before = Some(join(node.comment_before.take())),
        }
    }
    Ok(doc)
}

// --- Writing -------------------------------------------------------------------------------------

/// Where a node is being written: its indent, whether inside brackets, whether as a key.
#[derive(Clone, Debug)]
struct Ctx {
    indent: String,
    in_flow: Option<bool>,
    implicit_key: bool,
    force_block_indent: bool,
}

const INDENT_STEP: &str = "  ";

/// A comment's lines each after a `#`, as `stringifyComment` writes them.
fn comment_string(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else if line == " " {
                "#".to_owned()
            } else {
                format!("#{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A written comment's lines indented, as `indentComment` indents them.
fn indent_comment(comment: &str, indent: &str) -> String {
    if !comment.is_empty() && comment.chars().all(|c| c == '\n') {
        return comment[1..].to_owned();
    }
    if indent.is_empty() {
        return comment.to_owned();
    }
    comment
        .split('\n')
        .map(|line| {
            if line.chars().all(|c| c == ' ') {
                line.to_owned()
            } else {
                format!("{indent}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A comment after what is written on a line, as `lineComment` places it.
fn line_comment(written: &str, indent: &str, comment: &str) -> String {
    if written.ends_with('\n') {
        indent_comment(comment, indent)
    } else if comment.contains('\n') {
        format!("\n{}", indent_comment(comment, indent))
    } else {
        format!("{}{comment}", if written.ends_with(' ') { "" } else { " " })
    }
}

fn contains_document_marker(text: &str) -> bool {
    text.split('\n')
        .any(|line| line.starts_with('%') || line.starts_with("---") || line.starts_with("..."))
}

/// Whether a text would read back as something other than text, as the core schema's tests find.
fn reads_as_other(text: &str) -> bool {
    !matches!(resolve_plain(text).0, Scalar::Str(_))
}

/// A run of line breaks with `then` after each run, as `.replace(/\n+/g, '$&' + then)`.
fn after_breaks(text: &str, then: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '\n' && chars.peek() != Some(&'\n') {
            out.push_str(then);
        }
    }
    out
}

fn double_quoted_string(value: &str, ctx: &Ctx) -> String {
    let json = serde_json::to_string(value).unwrap_or_default();
    let json = json.as_bytes();
    let indent = if ctx.indent.is_empty() && contains_document_marker(value) {
        INDENT_STEP.to_owned()
    } else {
        ctx.indent.clone()
    };
    let mut out: Vec<u8> = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < json.len() {
        let mut ch = json[i];
        if ch == b' ' && json.get(i + 1) == Some(&b'\\') && json.get(i + 2) == Some(&b'n') {
            out.extend_from_slice(&json[start..i]);
            out.extend_from_slice(b"\\ ");
            i += 1;
            start = i;
            ch = b'\\';
        }
        if ch == b'\\' {
            match json.get(i + 1) {
                Some(b'u') => {
                    out.extend_from_slice(&json[start..i]);
                    let code = std::str::from_utf8(&json[i + 2..(i + 6).min(json.len())]).unwrap_or("");
                    match code {
                        "0000" => out.extend_from_slice(b"\\0"),
                        "0007" => out.extend_from_slice(b"\\a"),
                        "000b" => out.extend_from_slice(b"\\v"),
                        "001b" => out.extend_from_slice(b"\\e"),
                        "0085" => out.extend_from_slice(b"\\N"),
                        "00a0" => out.extend_from_slice(b"\\_"),
                        "2028" => out.extend_from_slice(b"\\L"),
                        "2029" => out.extend_from_slice(b"\\P"),
                        _ if code.starts_with("00") => {
                            out.extend_from_slice(b"\\x");
                            out.extend_from_slice(&code.as_bytes()[2..]);
                        }
                        _ => out.extend_from_slice(&json[i..(i + 6).min(json.len())]),
                    }
                    i += 5;
                    start = i + 1;
                }
                Some(b'n') => {
                    if ctx.implicit_key || json.get(i + 2) == Some(&b'"') || json.len() < 40 {
                        i += 1;
                    } else {
                        out.extend_from_slice(&json[start..i]);
                        out.extend_from_slice(b"\n\n");
                        while json.get(i + 2) == Some(&b'\\')
                            && json.get(i + 3) == Some(&b'n')
                            && json.get(i + 4) != Some(&b'"')
                        {
                            out.push(b'\n');
                            i += 2;
                        }
                        out.extend_from_slice(indent.as_bytes());
                        if json.get(i + 2) == Some(&b' ') {
                            out.push(b'\\');
                        }
                        i += 1;
                        start = i + 1;
                    }
                }
                _ => i += 1,
            }
        }
        i += 1;
    }
    let text = if start > 0 {
        out.extend_from_slice(&json[start..]);
        out
    } else {
        json.to_vec()
    };
    String::from_utf8(text).unwrap_or_default()
}

fn single_quoted_string(value: &str, ctx: &Ctx) -> String {
    let breaks_with_space =
        value.contains(" \n") || value.contains("\t\n") || value.contains("\n ") || value.contains("\n\t");
    if (ctx.implicit_key && value.contains('\n')) || breaks_with_space {
        return double_quoted_string(value, ctx);
    }
    let indent = if ctx.indent.is_empty() && contains_document_marker(value) {
        INDENT_STEP.to_owned()
    } else {
        ctx.indent.clone()
    };
    format!("'{}'", after_breaks(&value.replace('\'', "''"), &format!("\n{indent}")))
}

fn quoted_string(value: &str, ctx: &Ctx) -> String {
    let has_double = value.contains('"');
    let has_single = value.contains('\'');
    if has_double && !has_single {
        single_quoted_string(value, ctx)
    } else {
        double_quoted_string(value, ctx)
    }
}

/// The more-indented lines of a folded scalar kept apart, as the web's
/// `/(?:^|\n)([\t ].*)(?:([\n\t ]*)\n(?![\n\t ]))?/g` with `$1$2` keeps them.
fn keep_more_indented(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut at = 0;
    let mut copied = 0;
    while at < bytes.len() {
        let line = [(at == 0).then_some(at), (bytes[at] == b'\n').then_some(at + 1)]
            .into_iter()
            .flatten()
            .find(|line| bytes.get(*line).is_some_and(|b| *b == b'\t' || *b == b' '));
        let Some(line) = line else {
            at += 1;
            continue;
        };
        let mut end = line;
        while end < bytes.len() && !matches!(bytes[end], b'\n' | b'\r') {
            end += 1;
        }
        let mut run = end;
        while run < bytes.len() && matches!(bytes[run], b'\n' | b'\t' | b' ') {
            run += 1;
        }
        out.push_str(&text[copied..at]);
        out.push_str(&text[line..end]);
        let next = if run > end && bytes[run - 1] == b'\n' {
            out.push_str(&text[end..run - 1]);
            run
        } else {
            end
        };
        copied = next;
        at = next;
    }
    out.push_str(&text[copied..]);
    out
}

fn block_string(value: &str, comment: Option<&str>, style: Option<Style>, ctx: &Ctx) -> (String, bool) {
    let trailing_blank = {
        let trimmed = value.trim_end_matches(['\t', ' ']);
        trimmed.len() < value.len() && trimmed.ends_with('\n')
    };
    if trailing_blank {
        return (quoted_string(value, ctx), false);
    }
    let indent = if !ctx.indent.is_empty() {
        ctx.indent.clone()
    } else if ctx.force_block_indent || contains_document_marker(value) {
        INDENT_STEP.to_owned()
    } else {
        String::new()
    };
    let literal = style != Some(Style::Folded);
    if value.is_empty() {
        return ((if literal { "|\n" } else { ">\n" }).to_owned(), false);
    }
    let mut chomp_keep = false;
    let end_start = value.trim_end_matches(['\n', '\t', ' ']).len();
    let mut end = value[end_start..].to_owned();
    let chomp = match end.find('\n') {
        None => "-",
        Some(at) if value == end || at != end.len() - 1 => {
            chomp_keep = true;
            "+"
        }
        Some(_) => "",
    };
    let mut body = value[..end_start].to_owned();
    if !end.is_empty() {
        if end.ends_with('\n') {
            end.pop();
        }
        // Each run of breaks not at the start and not running to the end takes the indent after it.
        end = after_breaks_inner(&end, &indent);
    }
    let mut start_with_space = false;
    let mut start_break: Option<usize> = None;
    for (at, c) in body.char_indices() {
        match c {
            ' ' => start_with_space = true,
            '\n' => start_break = Some(at),
            _ => break,
        }
    }
    let cut = start_break.map_or(0, |at| at + 1);
    let mut start = body[..cut].to_owned();
    if !start.is_empty() {
        body = body[start.len()..].to_owned();
        start = after_breaks(&start, &indent);
    }
    let indent_size = if indent.is_empty() { "1" } else { "2" };
    let mut header = format!("{}{chomp}", if start_with_space { indent_size } else { "" });
    if let Some(comment) = comment {
        let flat: String = {
            let mut out = String::new();
            let mut chars = comment.chars().peekable();
            while let Some(c) = chars.next() {
                if c == ' ' && matches!(chars.peek(), Some('\r' | '\n')) {
                    continue;
                }
                if c == '\r' || c == '\n' {
                    while matches!(chars.peek(), Some('\r' | '\n')) {
                        chars.next();
                    }
                    out.push(' ');
                    continue;
                }
                out.push(c);
            }
            out
        };
        header.push(' ');
        header.push_str(&comment_string(&flat));
    }
    if !literal {
        let doubled = {
            let mut out = String::new();
            let mut chars = body.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\n' {
                    let mut run = String::from('\n');
                    while chars.peek() == Some(&'\n') {
                        chars.next();
                        run.push('\n');
                    }
                    out.push('\n');
                    out.push_str(&run);
                } else {
                    out.push(c);
                }
            }
            out
        };
        let folded = after_breaks(&keep_more_indented(&doubled), &indent);
        return (format!(">{header}\n{indent}{start}{folded}{end}"), chomp_keep);
    }
    let body = after_breaks(&body, &indent);
    (format!("|{header}\n{indent}{start}{body}{end}"), chomp_keep)
}

/// Every run of line breaks that has something before it and something other than a break after
/// it takes `indent` after it, as `/(^|(?<!\n))\n+(?!\n|$)/g` with `$&indent` adds it.
fn after_breaks_inner(text: &str, indent: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let from = i;
            while i < bytes.len() && bytes[i] == b'\n' {
                i += 1;
            }
            out.push_str(&text[from..i]);
            if i < bytes.len() {
                out.push_str(indent);
            }
            continue;
        }
        let next = text[i..].chars().next().unwrap_or(' ');
        out.push(next);
        i += next.len_utf8();
    }
    out
}

fn plain_string(value: &str, style: Option<Style>, comment: Option<&str>, ctx: &mut Ctx) -> (String, bool) {
    let in_flow = ctx.in_flow == Some(true);
    if (ctx.implicit_key && value.contains('\n')) || (in_flow && value.contains(['[', ']', '{', '}', ','])) {
        return (quoted_string(value, ctx), false);
    }
    if needs_quotes(value) {
        return if ctx.implicit_key || in_flow || !value.contains('\n') {
            (quoted_string(value, ctx), false)
        } else {
            block_string(value, comment, style, ctx)
        };
    }
    if !ctx.implicit_key && !in_flow && style != Some(Style::Plain) && value.contains('\n') {
        return block_string(value, comment, style, ctx);
    }
    if contains_document_marker(value) {
        if ctx.indent.is_empty() {
            ctx.force_block_indent = true;
            return block_string(value, comment, style, ctx);
        } else if ctx.implicit_key && ctx.indent == INDENT_STEP {
            return (quoted_string(value, ctx), false);
        }
    }
    let written = after_breaks(value, &format!("\n{}", ctx.indent));
    if reads_as_other(&written) {
        return (quoted_string(value, ctx), false);
    }
    (written, false)
}

/// Whether a text cannot be written plain, as the web's pattern of what a plain scalar may not
/// start with, hold or end with finds it.
fn needs_quotes(value: &str) -> bool {
    let bytes = value.as_bytes();
    let first = bytes.first().copied();
    if first.is_some_and(|b| b"\n\t ,[]{}#&*!|>'\"%@`".contains(&b)) {
        return true;
    }
    if matches!(value, "?" | "-") {
        return true;
    }
    if matches!(first, Some(b'?' | b'-')) && matches!(bytes.get(1), Some(b' ' | b'\t')) {
        return true;
    }
    for pair in bytes.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if (matches!(a, b'\n' | b':') && matches!(b, b' ' | b'\t'))
            || (matches!(a, b' ' | b'\t') && b == b'\n')
            || (matches!(a, b'\n' | b'\t' | b' ') && b == b'#')
        {
            return true;
        }
    }
    bytes.last().is_some_and(|b| matches!(b, b'\n' | b'\t' | b' ' | b':'))
}

/// Whether a text holds a character a plain or single-quoted scalar cannot carry, so it is
/// written in double quotes.
fn has_control(value: &str) -> bool {
    value
        .chars()
        .any(|c| matches!(c as u32, 0x00..=0x08 | 0x0b..=0x1f | 0x7f..=0x9f))
}

fn string_node(value: &str, style: Option<Style>, comment: Option<&str>, ctx: &mut Ctx) -> (String, bool) {
    let mut style = style;
    if style != Some(Style::Double) && has_control(value) {
        style = Some(Style::Double);
    }
    let in_flow = ctx.in_flow == Some(true);
    match style {
        Some(Style::Literal | Style::Folded) => {
            if ctx.implicit_key || in_flow {
                (quoted_string(value, ctx), false)
            } else {
                block_string(value, comment, style, ctx)
            }
        }
        Some(Style::Double) => (double_quoted_string(value, ctx), false),
        Some(Style::Single) => (single_quoted_string(value, ctx), false),
        Some(Style::Plain) | None => plain_string(value, style, comment, ctx),
    }
}

/// A number written as `toExponential()` writes it: 1e+3, 1.5e-7.
fn exponential(value: f64) -> String {
    let written = format!("{value:e}");
    let (mantissa, exponent) = written.split_once('e').unwrap_or((&written, "0"));
    let exponent: i64 = exponent.parse().unwrap_or(0);
    format!("{mantissa}e{}{}", if exponent < 0 { '-' } else { '+' }, exponent.abs())
}

fn number_string(value: f64, format: Option<Format>, min_fraction: usize) -> String {
    let whole_positive = value.fract() == 0.0 && value >= 0.0 && value.is_finite();
    match format {
        Some(Format::Hex) if whole_positive => return format!("0x{:x}", value as u64),
        Some(Format::Oct) if whole_positive => return format!("0o{:o}", value as u64),
        Some(Format::Exp) if value.is_finite() => return exponential(value),
        _ => {}
    }
    if !value.is_finite() {
        return if value.is_nan() {
            ".nan"
        } else if value < 0.0 {
            "-.inf"
        } else {
            ".inf"
        }
        .to_owned();
    }
    let mut n = if value == 0.0 && value.is_sign_negative() {
        "-0".to_owned()
    } else {
        number_to_string(value)
    };
    if format.is_none()
        && min_fraction > 0
        && !n.contains('e')
        && n.trim_start_matches('-').starts_with(|c: char| c.is_ascii_digit())
    {
        let point = match n.find('.') {
            Some(at) => at,
            None => {
                n.push('.');
                n.len() - 1
            }
        };
        let have = n.len() - point - 1;
        for _ in have..min_fraction {
            n.push('0');
        }
    }
    n
}

/// A scalar as written; and whether it is a block scalar keeping its line breaks at the end.
fn scalar_string(node: &Node, ctx: &mut Ctx) -> (String, bool, bool) {
    let Kind::Scalar {
        value,
        style,
        source,
        format,
        min_fraction,
    } = &node.kind
    else {
        return (String::new(), false, false);
    };
    match value {
        Scalar::Null => {
            let kept = source
                .as_deref()
                .filter(|s| matches!(*s, "" | "~" | "null" | "Null" | "NULL"));
            (kept.unwrap_or("null").to_owned(), false, false)
        }
        Scalar::Bool(b) => {
            let kept = source.as_deref().filter(|s| {
                matches!(*s, "true" | "True" | "TRUE" | "false" | "False" | "FALSE") && s.starts_with(['t', 'T']) == *b
            });
            (kept.map_or_else(|| b.to_string(), str::to_owned), false, false)
        }
        Scalar::Num(n) => (number_string(*n, *format, *min_fraction), false, false),
        Scalar::Str(text) => {
            let (written, keep) = string_node(text, *style, node.comment.as_deref(), ctx);
            // A block scalar writes its comment in its header.
            let took_comment = matches!(written.as_bytes().first(), Some(b'|' | b'>')) && node.comment.is_some();
            (written, keep, took_comment)
        }
    }
}

/// What writing a node reports back: its text, whether it took its own comment, and whether it
/// keeps its line breaks at the end.
struct Written {
    text: String,
    took_comment: bool,
    chomp_keep: bool,
}

fn write_node(node: &Node, ctx: &mut Ctx) -> Written {
    match &node.kind {
        Kind::Scalar { .. } => {
            let (text, chomp_keep, took_comment) = scalar_string(node, ctx);
            Written {
                text,
                took_comment,
                chomp_keep,
            }
        }
        Kind::Map(_) | Kind::Seq(_) => write_collection(node, ctx),
    }
}

fn write_collection(node: &Node, ctx: &Ctx) -> Written {
    let flow = ctx.in_flow.or(node.flow).unwrap_or(false);
    if flow {
        Written {
            text: write_flow(node, ctx),
            took_comment: false,
            chomp_keep: false,
        }
    } else {
        write_block(node, ctx)
    }
}

fn write_block(node: &Node, ctx: &Ctx) -> Written {
    let (prefix, item_indent, empty) = match &node.kind {
        Kind::Seq(_) => ("- ", format!("{}{INDENT_STEP}", ctx.indent), "[]"),
        _ => ("", ctx.indent.clone(), "{}"),
    };
    let mut lines: Vec<String> = Vec::new();
    let mut chomp_keep = false;
    let add_before = |lines: &mut Vec<String>, comment: Option<&String>, chomp_keep: bool| {
        if let Some(comment) = comment {
            let comment = if chomp_keep {
                comment.trim_start_matches('\n').to_owned()
            } else {
                comment.clone()
            };
            if !comment.is_empty() {
                let written = indent_comment(&comment_string(&comment), &ctx.indent);
                lines.push(written.trim_start().to_owned());
            }
        }
    };
    let push = |lines: &mut Vec<String>, text: String| lines.push(format!("{prefix}{text}"));
    match &node.kind {
        Kind::Seq(items) => {
            for item in items {
                if !chomp_keep && item.space_before {
                    lines.push(String::new());
                }
                add_before(&mut lines, item.comment_before.as_ref(), chomp_keep);
                let mut item_ctx = Ctx {
                    indent: item_indent.clone(),
                    in_flow: ctx.in_flow,
                    implicit_key: false,
                    force_block_indent: ctx.force_block_indent,
                };
                let written = write_node(item, &mut item_ctx);
                let mut text = written.text;
                chomp_keep = written.chomp_keep;
                let comment = if written.took_comment {
                    None
                } else {
                    item.comment.as_ref()
                };
                if let Some(comment) = comment {
                    text.push_str(&line_comment(&text, &item_indent, &comment_string(comment)));
                    if chomp_keep {
                        chomp_keep = false;
                    }
                }
                push(&mut lines, text);
            }
        }
        Kind::Map(pairs) => {
            for pair in pairs {
                if !chomp_keep && pair.key.space_before {
                    lines.push(String::new());
                }
                add_before(&mut lines, pair.key.comment_before.as_ref(), chomp_keep);
                let item_ctx = Ctx {
                    indent: item_indent.clone(),
                    in_flow: ctx.in_flow,
                    implicit_key: false,
                    force_block_indent: ctx.force_block_indent,
                };
                let (text, keep) = write_pair(pair, &item_ctx);
                chomp_keep = keep;
                push(&mut lines, text);
            }
        }
        Kind::Scalar { .. } => {}
    }
    let mut text = if lines.is_empty() {
        empty.to_owned()
    } else {
        let mut text = lines[0].clone();
        for line in &lines[1..] {
            if line.is_empty() {
                text.push('\n');
            } else {
                text.push('\n');
                text.push_str(&ctx.indent);
                text.push_str(line);
            }
        }
        text
    };
    let mut took_comment = false;
    if let Some(comment) = &node.comment {
        text.push('\n');
        text.push_str(&indent_comment(&comment_string(comment), &ctx.indent));
        took_comment = true;
    }
    Written {
        text,
        took_comment,
        chomp_keep: chomp_keep && !took_comment,
    }
}

fn write_flow(node: &Node, ctx: &Ctx) -> String {
    let item_indent = format!(
        "{}{INDENT_STEP}",
        match &node.kind {
            Kind::Seq(_) => format!("{}{INDENT_STEP}", ctx.indent),
            _ => ctx.indent.clone(),
        }
    );
    let (start, end) = match &node.kind {
        Kind::Seq(_) => ("[", "]"),
        _ => ("{", "}"),
    };
    let mut req_newline = false;
    let mut lines: Vec<String> = Vec::new();
    let mut lines_at_value = 0;
    let count = match &node.kind {
        Kind::Seq(items) => items.len(),
        Kind::Map(pairs) => pairs.len(),
        Kind::Scalar { .. } => 0,
    };
    for at in 0..count {
        let mut item_ctx = Ctx {
            indent: item_indent.clone(),
            in_flow: Some(true),
            implicit_key: false,
            force_block_indent: ctx.force_block_indent,
        };
        let (mut text, comment) = match &node.kind {
            Kind::Seq(items) => {
                let item = &items[at];
                if item.space_before {
                    lines.push(String::new());
                }
                if let Some(before) = &item.comment_before {
                    lines.push(
                        indent_comment(&comment_string(before), &ctx.indent)
                            .trim_start()
                            .to_owned(),
                    );
                }
                let written = write_node(item, &mut item_ctx);
                let comment = if written.took_comment {
                    None
                } else {
                    item.comment.clone()
                };
                (written.text, comment)
            }
            Kind::Map(pairs) => {
                let pair = &pairs[at];
                if pair.key.space_before {
                    lines.push(String::new());
                }
                if let Some(before) = &pair.key.comment_before {
                    lines.push(
                        indent_comment(&comment_string(before), &ctx.indent)
                            .trim_start()
                            .to_owned(),
                    );
                }
                if pair.key.comment.is_some() || pair.value.comment_before.is_some() {
                    req_newline = true;
                }
                let comment = pair.value.comment.clone();
                let (text, _) = write_pair(pair, &item_ctx);
                (text, comment)
            }
            Kind::Scalar { .. } => (String::new(), None),
        };
        if comment.is_some() {
            req_newline = true;
        }
        req_newline = req_newline || lines.len() > lines_at_value || text.contains('\n');
        if at < count - 1 {
            text.push(',');
        }
        if let Some(comment) = comment {
            text.push_str(&line_comment(&text, &item_indent, &comment_string(&comment)));
        }
        lines.push(text);
        lines_at_value = lines.len();
    }
    if lines.is_empty() {
        return format!("{start}{end}");
    }
    if req_newline {
        let mut text = start.to_owned();
        for line in &lines {
            if line.is_empty() {
                text.push('\n');
            } else {
                text.push('\n');
                text.push_str(INDENT_STEP);
                text.push_str(&ctx.indent);
                text.push_str(line);
            }
        }
        format!("{text}\n{}{end}", ctx.indent)
    } else {
        format!("{start}{}{end}", lines.join(" "))
    }
}

/// A pair as written, and whether its value keeps its line breaks at the end.
fn write_pair(pair: &Pair, ctx: &Ctx) -> (String, bool) {
    let in_flow = ctx.in_flow == Some(true);
    let key_comment = pair.key.comment.clone();
    let explicit = !pair.key.is_scalar()
        || matches!(
            &pair.key.kind,
            Kind::Scalar {
                style: Some(Style::Literal | Style::Folded),
                ..
            }
        );
    let mut key_ctx = Ctx {
        indent: format!("{}{INDENT_STEP}", ctx.indent),
        in_flow: ctx.in_flow,
        implicit_key: !explicit,
        force_block_indent: ctx.force_block_indent,
    };
    let written = write_node(&pair.key, &mut key_ctx);
    let mut text = written.text;
    let explicit = explicit || (!in_flow && text.len() > 1024);
    let key_comment = if written.took_comment { None } else { key_comment };
    if explicit {
        if let Some(comment) = &key_comment {
            text.push_str(&line_comment(&text, &key_ctx.indent, &comment_string(comment)));
        }
        text = format!("? {text}\n{}:", ctx.indent);
    } else {
        text.push(':');
        if let Some(comment) = &key_comment {
            text.push_str(&line_comment(&text, &key_ctx.indent, &comment_string(comment)));
        }
    }
    let value = &pair.value;
    let mut value_ctx = Ctx {
        indent: key_ctx.indent.clone(),
        in_flow: ctx.in_flow,
        implicit_key: false,
        force_block_indent: ctx.force_block_indent,
    };
    let written = write_node(value, &mut value_ctx);
    let value_text = written.text;
    let mut ws = " ".to_owned();
    if key_comment.is_some() || value.space_before || value.comment_before.is_some() {
        ws = if value.space_before { "\n" } else { "" }.to_owned();
        if let Some(before) = &value.comment_before {
            ws.push('\n');
            ws.push_str(&indent_comment(&comment_string(before), &value_ctx.indent));
        }
        if value_text.is_empty() && !in_flow {
            if ws == "\n" && value.comment.is_some() {
                ws = "\n\n".to_owned();
            }
        } else {
            ws.push('\n');
            ws.push_str(&value_ctx.indent);
        }
    } else if !explicit && !value.is_scalar() {
        let has_newline = value_text.contains('\n');
        let empty = value.pairs().is_empty() && value.items().is_empty();
        let flow = ctx.in_flow.or(value.flow).unwrap_or(empty);
        if has_newline || !flow {
            ws = format!("\n{}", value_ctx.indent);
        }
    } else if value_text.is_empty() || value_text.starts_with('\n') {
        ws = String::new();
    }
    text.push_str(&ws);
    text.push_str(&value_text);
    let mut chomp_keep = false;
    if !in_flow {
        if let (Some(comment), false) = (&value.comment, written.took_comment) {
            text.push_str(&line_comment(&text, &value_ctx.indent, &comment_string(comment)));
        } else if written.chomp_keep {
            chomp_keep = true;
        }
    }
    (text, chomp_keep)
}

impl Document {
    /// The document written again, as `doc.toString({ flowCollectionPadding: false, lineWidth: 0 })`.
    pub fn write(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        if self.doc_start {
            lines.push("---".to_owned());
        }
        if let Some(before) = &self.comment_before {
            if lines.len() != 1 {
                lines.insert(0, String::new());
            }
            lines.insert(0, indent_comment(&comment_string(before), ""));
        }
        let mut chomp_keep = false;
        let mut content_comment: Option<String> = None;
        match &self.contents {
            Some(node) => {
                if node.space_before && self.doc_start {
                    lines.push(String::new());
                }
                if let Some(before) = &node.comment_before {
                    lines.push(indent_comment(&comment_string(before), ""));
                }
                let mut ctx = Ctx {
                    indent: String::new(),
                    in_flow: None,
                    implicit_key: false,
                    force_block_indent: self.comment.is_some(),
                };
                content_comment = node.comment.clone();
                let written = write_node(node, &mut ctx);
                let mut body = written.text;
                if written.took_comment {
                    content_comment = None;
                }
                if content_comment.is_none() && written.chomp_keep {
                    chomp_keep = true;
                }
                if let Some(comment) = &content_comment {
                    body.push_str(&line_comment(&body, "", &comment_string(comment)));
                }
                if body.starts_with(['|', '>']) && lines.last().is_some_and(|line| line == "---") {
                    let last = lines.len() - 1;
                    lines[last] = format!("--- {body}");
                } else {
                    lines.push(body);
                }
            }
            None => lines.push("null".to_owned()),
        }
        if self.doc_end {
            match &self.comment {
                Some(comment) => {
                    let written = comment_string(comment);
                    if written.contains('\n') {
                        lines.push("...".to_owned());
                        lines.push(indent_comment(&written, ""));
                    } else {
                        lines.push(format!("... {written}"));
                    }
                }
                None => lines.push("...".to_owned()),
            }
        } else if let Some(comment) = &self.comment {
            let comment = if chomp_keep {
                comment.trim_start_matches('\n').to_owned()
            } else {
                comment.clone()
            };
            if !comment.is_empty() {
                if (!chomp_keep || content_comment.is_some()) && lines.last().is_some_and(|line| !line.is_empty()) {
                    lines.push(String::new());
                }
                lines.push(indent_comment(&comment_string(&comment), ""));
            }
        }
        format!("{}\n", lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pipeline_reads_and_writes_back_as_it_was() {
        let text = "name: x\nsteps:\n  - id: fetch # c\n    needs: [a, \"b\"]\n    with:\n      url: 'q'\n\n  - id: two\n    script: |\n      abc\nx: 1.50\n";
        let doc = parse(text).expect("it reads");
        assert_eq!(doc.write(), text);
        let steps = doc.get("steps").expect("steps");
        assert_eq!(steps.items()[0].range.map(|range| range[0]), Some(19));
    }

    #[test]
    fn what_the_web_refuses_is_broken() {
        for text in [
            "k: v\nk: w\n",
            "k: v\n\tj: w\n",
            "k: [a\n",
            "a: b: c\n",
            "- a\nb: 1\n",
            "  a: 1\n b: 2\n",
        ] {
            assert!(parse(text).is_none(), "{text:?}");
        }
    }
}
