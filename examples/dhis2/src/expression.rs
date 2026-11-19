//! The simulated server's reading of its two expression languages, as the web's
//! `data/expression.ts` reads them: a tokenizer and a parser for the subset the example runs, and
//! an evaluator. What an expression means is the server's business, so it lives here, in the
//! example's backend; `check` says what is wrong with one.
//!
//! Every place in an expression (a token's, a node's, an error's) counts UTF-16 units from its
//! start, as the web's string indexes do, so an editor's marks land where the web's land.

use serde_json::{Value as Json, json};

use crate::js::{civil_from_days, date_utc, is_space, iso_day, number_of_text, number_to_string, round};

/// The two languages: aggregate expressions (indicators, validation rules) and program rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Expression,
    Rule,
}

impl Language {
    pub fn key(self) -> &'static str {
        match self {
            Language::Expression => "expression",
            Language::Rule => "rule",
        }
    }
}

/// A value an expression gives or reads.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Number(f64),
    Text(String),
    Bool(bool),
}

impl Value {
    pub fn to_json(&self) -> Json {
        match self {
            Value::Null => Json::Null,
            Value::Number(number) => json!(number),
            Value::Text(text) => json!(text),
            Value::Bool(flag) => json!(flag),
        }
    }

    /// Whether this is null or the empty text, as a missing value is.
    pub fn is_missing(&self) -> bool {
        matches!(self, Value::Null) || matches!(self, Value::Text(text) if text.is_empty())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub from: usize,
    pub to: usize,
}

/// What an item names: `#{}` a data element (or a rule variable), `A{}` an attribute, `C{}` a
/// constant, `R{}` a reporting rate, `I{}` a program indicator, `D{}` a program data element,
/// `N{}` an indicator, `OUG{}` an organization unit group, `V{}` a program variable, `[days]` the
/// days in the period.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Hash,
    A,
    C,
    R,
    I,
    D,
    N,
    Oug,
    V,
    Days,
}

impl ItemKind {
    pub fn key(self) -> &'static str {
        match self {
            ItemKind::Hash => "#",
            ItemKind::A => "A",
            ItemKind::C => "C",
            ItemKind::R => "R",
            ItemKind::I => "I",
            ItemKind::D => "D",
            ItemKind::N => "N",
            ItemKind::Oug => "OUG",
            ItemKind::V => "V",
            ItemKind::Days => "days",
        }
    }

    fn of(unit: u16) -> Option<ItemKind> {
        Some(match char::from_u32(u32::from(unit))? {
            '#' => ItemKind::Hash,
            'A' => ItemKind::A,
            'C' => ItemKind::C,
            'R' => ItemKind::R,
            'I' => ItemKind::I,
            'D' => ItemKind::D,
            'N' => ItemKind::N,
            'V' => ItemKind::V,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    Number(f64),
    Text(String),
    Boolean(bool),
    Item {
        kind: ItemKind,
        body: String,
    },
    Name(String),
    Call {
        name: String,
        args: Vec<Node>,
        name_span: Span,
    },
    After {
        name: String,
        target: Box<Node>,
        args: Vec<Node>,
        name_span: Span,
    },
    Unary {
        op: String,
        arg: Box<Node>,
    },
    Binary {
        op: String,
        left: Box<Node>,
        right: Box<Node>,
    },
}

/// An expression's tree: each node with the span it was read from.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub from: usize,
    pub to: usize,
    pub kind: NodeKind,
}

fn span_json(span: Span) -> Json {
    json!({ "from": span.from, "to": span.to })
}

impl Node {
    pub fn span(&self) -> Span {
        Span {
            from: self.from,
            to: self.to,
        }
    }

    /// The node as the web's object writes it.
    pub fn to_json(&self) -> Json {
        let mut out = match &self.kind {
            NodeKind::Number(value) => json!({ "type": "number", "value": value }),
            NodeKind::Text(value) => json!({ "type": "string", "value": value }),
            NodeKind::Boolean(value) => json!({ "type": "boolean", "value": value }),
            NodeKind::Item { kind, body } => json!({ "type": "item", "kind": kind.key(), "body": body }),
            NodeKind::Name(name) => json!({ "type": "name", "name": name }),
            NodeKind::Call { name, args, name_span } => json!({
                "type": "call", "name": name,
                "args": args.iter().map(Node::to_json).collect::<Vec<_>>(),
                "nameSpan": span_json(*name_span),
            }),
            NodeKind::After {
                name,
                target,
                args,
                name_span,
            } => json!({
                "type": "after", "name": name, "target": target.to_json(),
                "args": args.iter().map(Node::to_json).collect::<Vec<_>>(),
                "nameSpan": span_json(*name_span),
            }),
            NodeKind::Unary { op, arg } => json!({ "type": "unary", "op": op, "arg": arg.to_json() }),
            NodeKind::Binary { op, left, right } => {
                json!({ "type": "binary", "op": op, "left": left.to_json(), "right": right.to_json() })
            }
        };
        out["from"] = json!(self.from);
        out["to"] = json!(self.to);
        out
    }
}

/// Where an expression cannot be read, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionError {
    pub message: String,
    pub span: Span,
}

fn fail<T>(message: impl Into<String>, from: usize, to: usize) -> Result<T, ExpressionError> {
    Err(ExpressionError {
        message: message.into(),
        span: Span { from, to },
    })
}

// --- Tokens ---------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Number(f64),
    Text(String),
    Item(ItemKind, String),
    Name(String),
    Op(&'static str),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub from: usize,
    pub to: usize,
    pub kind: TokenKind,
}

impl Token {
    /// The token as the web's object writes it.
    pub fn to_json(&self) -> Json {
        let mut out = match &self.kind {
            TokenKind::Number(value) => json!({ "kind": "number", "value": value }),
            TokenKind::Text(value) => json!({ "kind": "string", "value": value }),
            TokenKind::Item(item, body) => json!({ "kind": "item", "item": item.key(), "body": body }),
            TokenKind::Name(text) => json!({ "kind": "name", "text": text }),
            TokenKind::Op(text) => json!({ "kind": "op", "text": text }),
            TokenKind::End => json!({ "kind": "end" }),
        };
        out["from"] = json!(self.from);
        out["to"] = json!(self.to);
        out
    }

    /// The text an operator or a name is written with.
    fn op_text(&self) -> Option<&str> {
        match &self.kind {
            TokenKind::Op(text) => Some(text),
            TokenKind::Name(text) => Some(text),
            _ => None,
        }
    }
}

const OPERATORS: [&str; 19] = [
    "&&", "||", "==", "!=", "<=", ">=", "<", ">", "+", "-", "*", "/", "%", "^", "!", "(", ")", ",", ".",
];

fn text_of(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

fn index_of(units: &[u16], what: &str, from: usize) -> Option<usize> {
    let what: Vec<u16> = what.encode_utf16().collect();
    if from > units.len() {
        return None;
    }
    (from..=units.len().saturating_sub(what.len()))
        .find(|at| units.len() >= what.len() && units[*at..*at + what.len()] == what[..])
}

fn starts(units: &[u16], at: usize, what: &str) -> bool {
    let what: Vec<u16> = what.encode_utf16().collect();
    units.len() >= at + what.len() && units[at..at + what.len()] == what[..]
}

fn unit_is(units: &[u16], at: usize, test: impl Fn(char) -> bool) -> bool {
    units
        .get(at)
        .and_then(|unit| char::from_u32(u32::from(*unit)))
        .is_some_and(test)
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// The length of a number at `at`, as `^(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?` matches it.
fn number_at(units: &[u16], at: usize) -> Option<usize> {
    let mut end = at;
    if unit_is(units, end, is_digit) {
        while unit_is(units, end, is_digit) {
            end += 1;
        }
        if unit_is(units, end, |c| c == '.') {
            end += 1;
            while unit_is(units, end, is_digit) {
                end += 1;
            }
        }
    } else if unit_is(units, end, |c| c == '.') && unit_is(units, end + 1, is_digit) {
        end += 1;
        while unit_is(units, end, is_digit) {
            end += 1;
        }
    } else {
        return None;
    }
    if unit_is(units, end, |c| c == 'e' || c == 'E') {
        let mut exp = end + 1;
        if unit_is(units, exp, |c| c == '+' || c == '-') {
            exp += 1;
        }
        if unit_is(units, exp, is_digit) {
            while unit_is(units, exp, is_digit) {
                exp += 1;
            }
            end = exp;
        }
    }
    Some(end - at)
}

/// The length of a name at `at`, as `^(?:d2:)?[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*` matches it.
fn name_at(units: &[u16], at: usize) -> Option<usize> {
    let rest = |start: usize| -> Option<usize> {
        if !unit_is(units, start, is_name_start) {
            return None;
        }
        let mut end = start + 1;
        while unit_is(units, end, is_word) {
            end += 1;
        }
        while unit_is(units, end, |c| c == '.') && unit_is(units, end + 1, is_name_start) {
            end += 2;
            while unit_is(units, end, is_word) {
                end += 1;
            }
        }
        Some(end)
    };
    let end = if starts(units, at, "d2:") {
        rest(at + 3).or_else(|| rest(at))
    } else {
        rest(at)
    }?;
    Some(end - at)
}

/// The tokens of an expression, comments and spaces left out.
pub fn tokenize(source: &str) -> Result<Vec<Token>, ExpressionError> {
    let units: Vec<u16> = source.encode_utf16().collect();
    let len = units.len();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < len {
        if unit_is(&units, at, is_space) {
            while unit_is(&units, at, is_space) {
                at += 1;
            }
            continue;
        }
        if starts(&units, at, "/*") {
            match index_of(&units, "*/", at + 2) {
                None => return fail("The comment is not closed with */.", at, len),
                Some(end) => at = end + 2,
            }
            continue;
        }
        if starts(&units, at, "//") {
            at = index_of(&units, "\n", at).unwrap_or(len);
            continue;
        }
        let item = if starts(&units, at, "OUG{") {
            Some((ItemKind::Oug, 4))
        } else {
            ItemKind::of(units[at])
                .filter(|_| unit_is(&units, at + 1, |c| c == '{'))
                .map(|kind| (kind, 2))
        };
        if let Some((kind, open)) = item {
            let end = index_of(&units, "}", at);
            let newline = index_of(&units, "\n", at);
            match end {
                Some(end) if newline.is_none_or(|newline| newline >= end) => {
                    tokens.push(Token {
                        from: at,
                        to: end + 1,
                        kind: TokenKind::Item(kind, text_of(&units[at + open..end])),
                    });
                    at = end + 1;
                }
                _ => {
                    let written = if kind == ItemKind::Oug { "OUG" } else { kind.key() };
                    return fail(
                        format!("The braces of {written}{{ are not closed."),
                        at,
                        newline.unwrap_or(len),
                    );
                }
            }
            continue;
        }
        if starts(&units, at, "[days]") {
            tokens.push(Token {
                from: at,
                to: at + 6,
                kind: TokenKind::Item(ItemKind::Days, String::new()),
            });
            at += 6;
            continue;
        }
        if unit_is(&units, at, |c| c == '\'' || c == '"') {
            let quote = units[at];
            let mut end = at + 1;
            let mut text = Vec::new();
            while end < len && units[end] != quote {
                if units[end] == u16::from(b'\\') && end + 1 < len {
                    end += 1;
                }
                text.push(units[end]);
                end += 1;
            }
            if end >= len {
                return fail("The text is not closed with its quote.", at, len);
            }
            tokens.push(Token {
                from: at,
                to: end + 1,
                kind: TokenKind::Text(text_of(&text)),
            });
            at = end + 1;
            continue;
        }
        if let Some(size) = number_at(&units, at) {
            tokens.push(Token {
                from: at,
                to: at + size,
                kind: TokenKind::Number(number_of_text(&text_of(&units[at..at + size]))),
            });
            at += size;
            continue;
        }
        if let Some(size) = name_at(&units, at) {
            tokens.push(Token {
                from: at,
                to: at + size,
                kind: TokenKind::Name(text_of(&units[at..at + size])),
            });
            at += size;
            continue;
        }
        match OPERATORS.iter().find(|op| starts(&units, at, op)) {
            None => {
                return fail(
                    format!("\"{}\" has no meaning here.", text_of(&units[at..at + 1])),
                    at,
                    at + 1,
                );
            }
            Some(op) => {
                tokens.push(Token {
                    from: at,
                    to: at + op.len(),
                    kind: TokenKind::Op(op),
                });
                at += op.len();
            }
        }
    }
    tokens.push(Token {
        from: len,
        to: len,
        kind: TokenKind::End,
    });
    Ok(tokens)
}

// --- Parsing --------------------------------------------------------------------------------------

/// A binary operator's precedence, `and` and `or` among them.
fn binary(text: &str) -> Option<u8> {
    Some(match text {
        "||" | "or" => 1,
        "&&" | "and" => 2,
        "==" | "!=" => 3,
        "<" | ">" | "<=" | ">=" => 4,
        "+" | "-" => 5,
        "*" | "/" | "%" => 6,
        "^" => 8,
        _ => return None,
    })
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    len: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.at]
    }

    fn next(&mut self) -> Token {
        let token = self.tokens[self.at].clone();
        self.at += 1;
        token
    }

    fn expect(&mut self, text: &str, what: &str) -> Result<Token, ExpressionError> {
        let token = self.peek();
        if token.op_text() != Some(text) {
            let message = if token.kind == TokenKind::End {
                format!("{what} is missing at the end.")
            } else {
                format!("{what} was expected here.")
            };
            return fail(message, token.from, token.to);
        }
        Ok(self.next())
    }

    fn expression(&mut self, min: u8) -> Result<Node, ExpressionError> {
        let mut left = self.unary()?;
        loop {
            let token = self.peek();
            let Some(text) = token.op_text().map(str::to_owned) else {
                return Ok(left);
            };
            let Some(precedence) = binary(&text) else {
                return Ok(left);
            };
            if precedence < min {
                return Ok(left);
            }
            self.next();
            let right = self.expression(if text == "^" { precedence } else { precedence + 1 })?;
            let op = match text.as_str() {
                "and" => "&&".to_owned(),
                "or" => "||".to_owned(),
                _ => text,
            };
            left = Node {
                from: left.from,
                to: right.to,
                kind: NodeKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
    }

    fn unary(&mut self) -> Result<Node, ExpressionError> {
        let token = self.peek().clone();
        if let Some(text) = token.op_text()
            && matches!(text, "-" | "+" | "!" | "not")
        {
            let op = if text == "not" { "!" } else { text }.to_owned();
            self.next();
            let arg = self.expression(7)?;
            return Ok(Node {
                from: token.from,
                to: arg.to,
                kind: NodeKind::Unary { op, arg: Box::new(arg) },
            });
        }
        let primary = self.primary()?;
        self.postfix(primary)
    }

    fn args(&mut self) -> Result<(Vec<Node>, usize), ExpressionError> {
        self.expect("(", "An opening parenthesis")?;
        let mut list = Vec::new();
        if self.peek().op_text() != Some(")") {
            loop {
                list.push(self.expression(1)?);
                if self.peek().op_text() != Some(",") {
                    break;
                }
                self.next();
            }
        }
        let close = self.expect(")", "A closing parenthesis")?;
        Ok((list, close.to))
    }

    fn postfix(&mut self, target: Node) -> Result<Node, ExpressionError> {
        let mut node = target;
        while self.peek().kind == TokenKind::Op(".")
            && matches!(
                self.tokens.get(self.at + 1).map(|one| &one.kind),
                Some(TokenKind::Name(_))
            )
        {
            self.next();
            let name = self.next();
            let TokenKind::Name(text) = &name.kind else {
                unreachable!("a name follows the dot");
            };
            let (args, to) = self.args()?;
            node = Node {
                from: node.from,
                to,
                kind: NodeKind::After {
                    name: format!(".{text}"),
                    target: Box::new(node),
                    args,
                    name_span: Span {
                        from: name.from - 1,
                        to: name.to,
                    },
                },
            };
        }
        Ok(node)
    }

    fn primary(&mut self) -> Result<Node, ExpressionError> {
        let token = self.next();
        let (from, to) = (token.from, token.to);
        match token.kind {
            TokenKind::Number(value) => Ok(Node {
                from,
                to,
                kind: NodeKind::Number(value),
            }),
            TokenKind::Text(value) => Ok(Node {
                from,
                to,
                kind: NodeKind::Text(value),
            }),
            TokenKind::Item(kind, body) => Ok(Node {
                from,
                to,
                kind: NodeKind::Item { kind, body },
            }),
            TokenKind::Name(text) => {
                if text == "true" || text == "false" {
                    return Ok(Node {
                        from,
                        to,
                        kind: NodeKind::Boolean(text == "true"),
                    });
                }
                if self.peek().op_text() == Some("(") {
                    let (args, end) = self.args()?;
                    return Ok(Node {
                        from,
                        to: end,
                        kind: NodeKind::Call {
                            name: text,
                            args,
                            name_span: Span { from, to },
                        },
                    });
                }
                Ok(Node {
                    from,
                    to,
                    kind: NodeKind::Name(text),
                })
            }
            TokenKind::Op(text) => {
                if text == "(" {
                    let inner = self.expression(1)?;
                    let close = self.expect(")", "A closing parenthesis")?;
                    return Ok(Node {
                        from,
                        to: close.to,
                        kind: inner.kind,
                    });
                }
                fail(format!("\"{text}\" cannot start a value."), from, to)
            }
            TokenKind::End => fail(
                if self.at == 1 {
                    "The expression is empty."
                } else {
                    "A value is missing at the end."
                },
                from,
                to,
            ),
        }
    }
}

/// An expression as a tree, or where it cannot be read.
pub fn parse(source: &str) -> Result<Node, ExpressionError> {
    let tokens = tokenize(source)?;
    let len = source.encode_utf16().count();
    let mut parser = Parser { tokens, at: 0, len };
    let root = parser.expression(1)?;
    let left = parser.peek();
    if left.kind != TokenKind::End {
        return fail("This is left over after the expression ends.", left.from, parser.len);
    }
    Ok(root)
}

/// Every node of a tree, the root first.
pub fn walk<'a>(node: &'a Node, visit: &mut dyn FnMut(&'a Node)) {
    visit(node);
    match &node.kind {
        NodeKind::Call { args, .. } => {
            for arg in args {
                walk(arg, visit);
            }
        }
        NodeKind::After { target, args, .. } => {
            walk(target, visit);
            for arg in args {
                walk(arg, visit);
            }
        }
        NodeKind::Unary { arg, .. } => walk(arg, visit),
        NodeKind::Binary { left, right, .. } => {
            walk(left, visit);
            walk(right, visit);
        }
        _ => {}
    }
}

// --- Evaluation -----------------------------------------------------------------------------------

/// What an evaluation reads from outside the expression: an item's value, the period moved by
/// `offset` months for one written with `.periodOffset`.
pub trait Scope {
    fn item(&self, kind: ItemKind, body: &str, offset: f64) -> Value;
}

impl<F: Fn(ItemKind, &str, f64) -> Value> Scope for F {
    fn item(&self, kind: ItemKind, body: &str, offset: f64) -> Value {
        self(kind, body, offset)
    }
}

/// The functions the example evaluates, with how many arguments each takes (`None` for no most).
pub const EVALUATED: &[(&str, usize, Option<usize>)] = &[
    ("if", 3, Some(3)),
    ("isNull", 1, Some(1)),
    ("isNotNull", 1, Some(1)),
    ("firstNonNull", 1, None),
    ("greatest", 1, None),
    ("least", 1, None),
    ("log", 1, Some(2)),
    ("log10", 1, Some(1)),
    ("removeZeros", 1, Some(1)),
    (".periodOffset", 1, Some(1)),
    ("d2:hasValue", 1, Some(1)),
    ("d2:daysBetween", 2, Some(2)),
    ("d2:weeksBetween", 2, Some(2)),
    ("d2:monthsBetween", 2, Some(2)),
    ("d2:yearsBetween", 2, Some(2)),
    ("d2:addDays", 2, Some(2)),
    ("d2:ceil", 1, Some(1)),
    ("d2:floor", 1, Some(1)),
    ("d2:round", 1, Some(2)),
    ("d2:concatenate", 1, None),
    ("d2:length", 1, Some(1)),
    ("d2:left", 2, Some(2)),
    ("d2:right", 2, Some(2)),
    ("d2:substring", 3, Some(3)),
    ("d2:modulus", 2, Some(2)),
    ("d2:zing", 1, Some(1)),
    ("d2:oizp", 1, Some(1)),
    ("d2:validatePattern", 2, Some(2)),
];

/// How many arguments a function the example evaluates takes; none for one it does not.
pub fn evaluated(name: &str) -> Option<(usize, Option<usize>)> {
    EVALUATED
        .iter()
        .find(|(one, _, _)| *one == name)
        .map(|(_, min, max)| (*min, *max))
}

/// A value as a number: false and nothing as 0, a text that does not read as one as 0.
pub fn number(value: &Value) -> f64 {
    match value {
        Value::Number(number) => *number,
        Value::Bool(flag) => f64::from(u8::from(*flag)),
        Value::Null => 0.0,
        Value::Text(text) if text.is_empty() => 0.0,
        Value::Text(text) => {
            let read = number_of_text(text);
            if read.is_nan() { 0.0 } else { read }
        }
    }
}

/// Whether a value counts as true.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Bool(flag) => *flag,
        Value::Number(number) => *number != 0.0,
        Value::Text(text) => !text.is_empty() && text != "false",
        Value::Null => false,
    }
}

/// A value as `String(value)` writes it, nothing as the empty text.
pub fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Number(number) => number_to_string(*number),
        Value::Text(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
    }
}

const DAY: f64 = 86_400_000.0;

/// A day written at the start of a text as `YYYY-MM-DD`, as days since 1970 the way `Date.UTC`
/// counts them: a month or a day past its end running over, a year below 100 taken as 1900 and
/// after.
fn date(value: &Value) -> Option<i64> {
    let written = text(value);
    let bytes = written.as_bytes();
    let digits = |from: usize, to: usize| {
        bytes
            .get(from..to)
            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
    };
    if !(digits(0, 4) && bytes.get(4) == Some(&b'-') && digits(5, 7) && bytes.get(7) == Some(&b'-') && digits(8, 10)) {
        return None;
    }
    let read = |from: usize, to: usize| written[from..to].parse::<i64>().unwrap_or(0);
    let mut year = read(0, 4);
    if (0..=99).contains(&year) {
        year += 1900;
    }
    Some(date_utc(year, read(5, 7) - 1, read(8, 10)))
}

fn months_between(from: i64, to: i64) -> f64 {
    let (ay, am, ad) = civil_from_days(from);
    let (by, bm, bd) = civil_from_days(to);
    let mut months = (by - ay) * 12 + bm - am;
    // A month part way through does not count, either way: the count is cut toward zero.
    if months > 0 && bd < ad {
        months -= 1;
    } else if months < 0 && bd > ad {
        months += 1;
    }
    months as f64
}

/// `x ** y` as JavaScript gives it: NaN for a NaN power and for one or minus one to an infinite one.
pub fn power(x: f64, y: f64) -> f64 {
    if y.is_nan() {
        return f64::NAN;
    }
    if y == 0.0 {
        return 1.0;
    }
    if x.abs() == 1.0 && y.is_infinite() {
        return f64::NAN;
    }
    x.powf(y)
}

/// `Math.max` over numbers: NaN when any is, +0 above -0.
pub fn js_max(values: &[f64]) -> f64 {
    let mut out = f64::NEG_INFINITY;
    for value in values {
        if value.is_nan() {
            return f64::NAN;
        }
        if *value > out || (*value == 0.0 && out == 0.0 && value.is_sign_positive()) {
            out = *value;
        }
    }
    out
}

/// `Math.min` over numbers: NaN when any is, -0 below +0.
pub fn js_min(values: &[f64]) -> f64 {
    let mut out = f64::INFINITY;
    for value in values {
        if value.is_nan() {
            return f64::NAN;
        }
        if *value < out || (*value == 0.0 && out == 0.0 && value.is_sign_negative()) {
            out = *value;
        }
    }
    out
}

/// A text's UTF-16 units from `start` to `end`, as `slice` cuts them: a negative place counted from
/// the end.
pub fn slice(text: &str, start: f64, end: Option<f64>) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let len = units.len() as f64;
    let place = |at: f64| {
        let at = if at.is_nan() { 0.0 } else { at.trunc() };
        if at < 0.0 { (len + at).max(0.0) } else { at.min(len) }
    };
    let from = place(start);
    let to = end.map_or(len, place);
    if from >= to {
        return String::new();
    }
    String::from_utf16_lossy(&units[from as usize..to as usize])
}

/// What an evaluation gives: its value, how many data items it read and how many of those had none.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluated {
    pub value: Value,
    pub items: usize,
    pub missing: usize,
}

struct Run<'a> {
    scope: &'a dyn Scope,
    /// Set where the web throws: an argument read that is not there, a day past the calendar.
    broke: bool,
    items: usize,
    missing: usize,
    offset: f64,
}

impl Run<'_> {
    fn item(&mut self, kind: ItemKind, body: &str) -> Value {
        let value = self.scope.item(kind, body, self.offset);
        if matches!(kind, ItemKind::Hash | ItemKind::R | ItemKind::A) {
            self.items += 1;
            if value.is_missing() {
                self.missing += 1;
            }
        }
        value
    }

    fn run(&mut self, node: Option<&Node>) -> Value {
        let Some(node) = node else {
            self.broke = true;
            return Value::Null;
        };
        match &node.kind {
            NodeKind::Number(value) => Value::Number(*value),
            NodeKind::Text(value) => Value::Text(value.clone()),
            NodeKind::Boolean(value) => Value::Bool(*value),
            NodeKind::Item { kind, body } => self.item(*kind, body),
            NodeKind::Name(_) => Value::Null,
            NodeKind::Unary { op, arg } => {
                let value = self.run(Some(arg));
                if op == "!" {
                    return Value::Bool(!truthy(&value));
                }
                if value == Value::Null {
                    return Value::Null;
                }
                Value::Number(if op == "-" { -number(&value) } else { number(&value) })
            }
            NodeKind::Binary { op, left, right } => self.binary(op, left, right),
            NodeKind::After { name, target, args, .. } => {
                if name != ".periodOffset" {
                    return self.run(Some(target));
                }
                let by = number(&self.run(args.first()));
                self.offset += by;
                let value = self.run(Some(target));
                self.offset -= by;
                value
            }
            NodeKind::Call { name, args, .. } => self.call(name, args),
        }
    }

    fn binary(&mut self, op: &str, left_node: &Node, right_node: &Node) -> Value {
        if op == "&&" {
            return Value::Bool(truthy(&self.run(Some(left_node))) && truthy(&self.run(Some(right_node))));
        }
        if op == "||" {
            return Value::Bool(truthy(&self.run(Some(left_node))) || truthy(&self.run(Some(right_node))));
        }
        let left = self.run(Some(left_node));
        let right = self.run(Some(right_node));
        if op == "==" || op == "!=" {
            let both = left != Value::Null && right != Value::Null;
            let same = if matches!(left, Value::Number(_)) || matches!(right, Value::Number(_)) {
                both && number(&left) == number(&right)
            } else if matches!(left, Value::Bool(_)) || matches!(right, Value::Bool(_)) {
                both && truthy(&left) == truthy(&right)
            } else {
                text(&left) == text(&right)
            };
            return Value::Bool(if op == "==" { same } else { !same });
        }
        if matches!(op, "<" | ">" | "<=" | ">=") {
            if left == Value::Null || right == Value::Null {
                return Value::Bool(false);
            }
            if let (Value::Text(a), Value::Text(b)) = (&left, &right) {
                let order = a.encode_utf16().cmp(b.encode_utf16());
                return Value::Bool(match op {
                    "<" => order.is_lt(),
                    ">" => order.is_gt(),
                    "<=" => order.is_le(),
                    _ => order.is_ge(),
                });
            }
            let (a, b) = (number(&left), number(&right));
            return Value::Bool(match op {
                "<" => a < b,
                ">" => a > b,
                "<=" => a <= b,
                _ => a >= b,
            });
        }
        if left == Value::Null && right == Value::Null {
            return Value::Null;
        }
        let (a, b) = (number(&left), number(&right));
        match op {
            "+" => Value::Number(a + b),
            "-" => Value::Number(a - b),
            "*" => Value::Number(a * b),
            "/" => {
                if b == 0.0 {
                    Value::Null
                } else {
                    Value::Number(a / b)
                }
            }
            "%" => {
                if b == 0.0 {
                    Value::Null
                } else {
                    Value::Number(a % b)
                }
            }
            "^" => Value::Number(power(a, b)),
            _ => Value::Null,
        }
    }

    fn call(&mut self, name: &str, arg_nodes: &[Node]) -> Value {
        if name == "if" {
            return if truthy(&self.run(arg_nodes.first())) {
                self.run(arg_nodes.get(1))
            } else {
                self.run(arg_nodes.get(2))
            };
        }
        if name == "d2:hasValue" {
            let value = match arg_nodes.first() {
                Some(Node {
                    kind: NodeKind::Text(body),
                    ..
                }) => self.scope.item(ItemKind::Hash, body, self.offset),
                None => {
                    self.broke = true;
                    Value::Null
                }
                other => self.run(other),
            };
            return Value::Bool(!value.is_missing());
        }
        let args: Vec<Value> = arg_nodes.iter().map(|one| self.run(Some(one))).collect();
        let first = args.first().cloned().unwrap_or(Value::Null);
        let second = args.get(1).cloned().unwrap_or(Value::Null);
        let third = args.get(2).cloned().unwrap_or(Value::Null);
        let present = || -> Vec<f64> { args.iter().filter(|one| **one != Value::Null).map(number).collect() };
        match name {
            "isNull" => Value::Bool(first == Value::Null),
            "isNotNull" => Value::Bool(first != Value::Null),
            "firstNonNull" => args
                .iter()
                .find(|one| **one != Value::Null)
                .cloned()
                .unwrap_or(Value::Null),
            "greatest" => {
                let present = present();
                if present.is_empty() {
                    Value::Null
                } else {
                    Value::Number(js_max(&present))
                }
            }
            "least" => {
                let present = present();
                if present.is_empty() {
                    Value::Null
                } else {
                    Value::Number(js_min(&present))
                }
            }
            "log" => {
                if first == Value::Null {
                    Value::Null
                } else if second == Value::Null {
                    Value::Number(number(&first).ln())
                } else {
                    Value::Number(number(&first).ln() / number(&second).ln())
                }
            }
            "log10" => {
                if first == Value::Null {
                    Value::Null
                } else {
                    Value::Number(number(&first).log10())
                }
            }
            "removeZeros" => {
                if first == Value::Null || number(&first) == 0.0 {
                    Value::Null
                } else {
                    first
                }
            }
            "d2:daysBetween" | "d2:weeksBetween" | "d2:monthsBetween" | "d2:yearsBetween" => {
                let (Some(from), Some(to)) = (date(&first), date(&second)) else {
                    return Value::Null;
                };
                let between = (to - from) as f64 * DAY;
                Value::Number(match name {
                    "d2:daysBetween" => round(between / DAY),
                    "d2:weeksBetween" => (between / (7.0 * DAY)).trunc(),
                    "d2:monthsBetween" => months_between(from, to),
                    _ => (months_between(from, to) / 12.0).trunc(),
                })
            }
            "d2:addDays" => {
                let Some(from) = date(&first) else {
                    return Value::Null;
                };
                let at = from as f64 * DAY + number(&second) * DAY;
                // A day past the calendar's end is an Invalid Date, whose ISO text the web cannot write.
                if !at.is_finite() || at.abs() > 8.64e15 {
                    self.broke = true;
                    return Value::Null;
                }
                Value::Text(iso_day((at.trunc() / DAY).floor() as i64))
            }
            "d2:ceil" => Value::Number(number(&first).ceil()),
            "d2:floor" => Value::Number(number(&first).floor()),
            "d2:round" => {
                let places = if second == Value::Null { 0.0 } else { number(&second) };
                Value::Number(round(number(&first) * power(10.0, places)) / power(10.0, places))
            }
            "d2:concatenate" => Value::Text(args.iter().map(text).collect()),
            "d2:length" => Value::Number(text(&first).encode_utf16().count() as f64),
            "d2:left" => Value::Text(slice(&text(&first), 0.0, Some(number(&second)))),
            "d2:right" => {
                if number(&second) <= 0.0 {
                    Value::Text(String::new())
                } else {
                    Value::Text(slice(&text(&first), -number(&second), None))
                }
            }
            "d2:substring" => Value::Text(slice(&text(&first), number(&second), Some(number(&third)))),
            "d2:modulus" => {
                if number(&second) == 0.0 {
                    Value::Null
                } else {
                    Value::Number(number(&first) % number(&second))
                }
            }
            "d2:zing" => Value::Number(js_max(&[0.0, number(&first)])),
            "d2:oizp" => Value::Number(if number(&first) >= 0.0 { 1.0 } else { 0.0 }),
            "d2:validatePattern" => {
                Value::Bool(crate::pattern::full_match(&text(&second), &text(&first)).unwrap_or(false))
            }
            _ => Value::Null,
        }
    }
}

/// An expression's value in a scope, with how many data items it read and how many had no value;
/// `Err` where the web's evaluator throws: an argument read that the call does not have (`if` with
/// too few, `.periodOffset()`, `d2:hasValue()`), or a day `d2:addDays` puts past the calendar.
/// Every expression a check passes without an error evaluates.
pub fn try_evaluate(node: &Node, scope: &dyn Scope) -> Result<Evaluated, Evaluated> {
    let mut run = Run {
        scope,
        broke: false,
        items: 0,
        missing: 0,
        offset: 0.0,
    };
    let value = run.run(Some(node));
    let value = match value {
        Value::Number(number) if !number.is_finite() => Value::Null,
        other => other,
    };
    let out = Evaluated {
        value,
        items: run.items,
        missing: run.missing,
    };
    if run.broke { Err(out) } else { Ok(out) }
}

/// An expression's value in a scope, with how many data items it read and how many had no value;
/// null where the web's evaluator throws (see `try_evaluate`).
pub fn evaluate(node: &Node, scope: &dyn Scope) -> Evaluated {
    try_evaluate(node, scope).unwrap_or_else(|mut out| {
        out.value = Value::Null;
        out
    })
}

/// The text with each closed block comment replaced by what `by` makes of it; an unclosed one stays.
pub fn without_block_comments_by(text: &str, by: impl Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut at = 0;
    while let Some(open) = text[at..].find("/*").map(|found| found + at) {
        let Some(close) = text[open + 2..].find("*/").map(|found| found + open + 2) else {
            break;
        };
        out.push_str(&text[at..open]);
        out.push_str(&by(&text[open..close + 2]));
        at = close + 2;
    }
    out.push_str(&text[at..]);
    out
}

/// The text with each closed block comment left out; an unclosed one stays.
pub fn without_block_comments(text: &str) -> String {
    without_block_comments_by(text, |_| String::new())
}

/// Every run of space as one space, as `.replace(/\s+/g, ' ')` writes it.
pub fn collapse_spaces(text: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in text.chars() {
        if is_space(c) {
            if !gap {
                out.push(' ');
            }
            gap = true;
        } else {
            out.push(c);
            gap = false;
        }
    }
    out
}

/// The source with each item written by `name`, as the server describes an expression in words.
pub fn describe(source: &str, name: impl Fn(ItemKind, &str) -> Option<String>) -> String {
    let Ok(tokens) = tokenize(source) else {
        return source.to_owned();
    };
    let units: Vec<u16> = source.encode_utf16().collect();
    let mut out: Vec<u16> = Vec::new();
    let mut at = 0;
    for token in &tokens {
        let TokenKind::Item(kind, body) = &token.kind else {
            continue;
        };
        let Some(said) = name(*kind, body) else {
            continue;
        };
        out.extend_from_slice(&units[at..token.from]);
        out.extend(said.encode_utf16());
        at = token.to;
    }
    out.extend_from_slice(&units[at..]);
    let joined = String::from_utf16_lossy(&out);
    let plain = without_block_comments(&joined);
    // `//` up to the end of its line goes.
    let mut kept = String::new();
    let mut rest = plain.as_str();
    while let Some(start) = rest.find("//") {
        kept.push_str(&rest[..start]);
        rest = &rest[start..];
        rest = &rest[rest.find('\n').unwrap_or(rest.len())..];
    }
    kept.push_str(rest);
    crate::js::trim(&collapse_spaces(&kept)).to_owned()
}
