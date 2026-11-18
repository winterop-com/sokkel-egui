//! A small SQL engine over the simulated tables, as the web's `sql.ts`: one SELECT with joins, a
//! filter, groups, a sort and a limit. Every refusal carries where in the text it is, so the editor
//! marks it.
//!
//! Every place in the query is a byte offset into it, where the web counts UTF-16 units: the two
//! agree on ASCII. A value is a number, a text, a truth or null, and is compared, added and
//! written as the web's are: numbers as doubles, texts in the collator's order, `String` and
//! `Number` between them.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::js::{length, locale_compare, number_to_string, round, slice, slice_range, sort_by, to_number};

/// A value in a table or a result.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
}

impl Value {
    /// A text value.
    pub fn text(text: &str) -> Self {
        Self::Str(text.to_owned())
    }

    /// The value as `String(value)` writes it.
    pub fn say(&self) -> String {
        match self {
            Self::Null => "null".to_owned(),
            Self::Bool(value) => value.to_string(),
            Self::Num(value) => number_to_string(*value),
            Self::Str(text) => text.clone(),
        }
    }

    /// The value as `Number(value)` reads it.
    fn number(&self) -> f64 {
        match self {
            Self::Null => 0.0,
            Self::Bool(value) => f64::from(u8::from(*value)),
            Self::Num(value) => *value,
            Self::Str(text) => to_number(text),
        }
    }

    /// What `JSON.stringify` makes of the value, as groups and DISTINCT tell values apart: a number
    /// that is not finite reads as null.
    fn key(&self) -> String {
        match self {
            Self::Num(value) if !value.is_finite() => "null".to_owned(),
            Self::Num(value) => number_to_string(*value),
            Self::Null => "null".to_owned(),
            Self::Bool(value) => value.to_string(),
            Self::Str(text) => format!("{text:?}"),
        }
    }
}

/// What a column holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnType {
    Integer,
    Text,
    Real,
}

impl ColumnType {
    /// The type as the tables say it: integer.
    pub fn name(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Text => "text",
            Self::Real => "real",
        }
    }
}

/// A column of a table and what it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub name: String,
    pub kind: ColumnType,
    pub description: String,
}

/// A table a query reads: its columns, and its rows with a value for each column in their order.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub name: String,
    pub description: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Value>>,
}

/// What a query gives: its columns' names, its rows, and how long it took in milliseconds.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub ms: f64,
}

/// A refusal and where it is, as byte offsets into the query; a fault the web throws as something
/// other than its `SqlError` (a LIKE pattern its regular expressions refuse) has no place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlError {
    pub message: String,
    pub at: Option<(usize, usize)>,
}

fn refuse<T>(message: impl Into<String>, from: usize, to: usize) -> Result<T, SqlError> {
    Err(SqlError {
        message: message.into(),
        at: Some((from, to)),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenType {
    Word,
    Number,
    Text,
    Op,
    End,
}

#[derive(Clone, Debug)]
struct Token {
    kind: TokenType,
    text: String,
    from: usize,
    to: usize,
}

const KEYWORDS: [&str; 33] = [
    "select", "distinct", "from", "where", "group", "by", "having", "order", "asc", "desc", "limit", "offset", "join",
    "inner", "left", "outer", "on", "as", "and", "or", "not", "in", "like", "is", "null", "true", "false", "between",
    "case", "when", "then", "else", "end",
];

fn keyword(text: &str) -> bool {
    KEYWORDS.contains(&text.to_lowercase().as_str())
}

/// Whether a character is one the web's `\w` matches.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn tokenize(source: &str) -> Result<Vec<Token>, SqlError> {
    let mut tokens = Vec::new();
    let mut at = 0;
    let char_at = |at: usize| source[at..].chars().next();
    while let Some(c) = char_at(at) {
        if crate::js::is_space(c) {
            at += c.len_utf8();
            continue;
        }
        if source[at..].starts_with("--") {
            at = source[at..].find('\n').map_or(source.len(), |end| at + end);
            continue;
        }
        if source[at..].starts_with("/*") {
            let Some(end) = source[at + 2..].find("*/") else {
                return refuse("This comment is never closed.", at, source.len());
            };
            at = at + 2 + end + 2;
            continue;
        }
        let from = at;
        if c.is_ascii_alphabetic() || c == '_' {
            while char_at(at).is_some_and(is_word) {
                at += 1;
            }
            tokens.push(Token {
                kind: TokenType::Word,
                text: source[from..at].to_owned(),
                from,
                to: at,
            });
        } else if c == '"' {
            let Some(end) = source[at + 1..].find('"') else {
                return refuse("This name is never closed.", from, source.len());
            };
            let end = at + 1 + end;
            at = end + 1;
            tokens.push(Token {
                kind: TokenType::Word,
                text: source[from + 1..end].to_owned(),
                from,
                to: at,
            });
        } else if c.is_ascii_digit() || (c == '.' && char_at(at + 1).is_some_and(|next| next.is_ascii_digit())) {
            while char_at(at).is_some_and(|next| next.is_ascii_digit() || next == '.') {
                at += 1;
            }
            tokens.push(Token {
                kind: TokenType::Number,
                text: source[from..at].to_owned(),
                from,
                to: at,
            });
        } else if c == '\'' {
            let mut text = String::new();
            at += 1;
            loop {
                let Some(next) = char_at(at) else {
                    return refuse("This text is never closed; end it with a quote.", from, source.len());
                };
                if next == '\'' {
                    if char_at(at + 1) == Some('\'') {
                        text.push('\'');
                        at += 2;
                        continue;
                    }
                    at += 1;
                    break;
                }
                text.push(next);
                at += next.len_utf8();
            }
            tokens.push(Token {
                kind: TokenType::Text,
                text,
                from,
                to: at,
            });
        } else {
            let two: String = source[at..].chars().take(2).collect();
            if ["<=", ">=", "<>", "!=", "||"].contains(&two.as_str()) {
                at += 2;
                tokens.push(Token {
                    kind: TokenType::Op,
                    text: two,
                    from,
                    to: at,
                });
            } else if "=<>+-*/%(),.;".contains(c) {
                at += 1;
                tokens.push(Token {
                    kind: TokenType::Op,
                    text: c.to_string(),
                    from,
                    to: at,
                });
            } else {
                return refuse(format!("{c} has no meaning here."), from, from + c.len_utf8());
            }
        }
    }
    tokens.push(Token {
        kind: TokenType::End,
        text: String::new(),
        from: source.len(),
        to: source.len(),
    });
    Ok(tokens)
}

/// Where a column was found: the table in the query's scope, by its place, and the column's place
/// in it; none for a name of the output, as ORDER BY and HAVING may use.
type Bound = Option<(usize, usize)>;

#[derive(Clone, Debug)]
enum ExprKind {
    Lit(Value),
    Col {
        table: Option<String>,
        name: String,
        bound: Bound,
    },
    Bin {
        op: String,
        l: Box<Expr>,
        r: Box<Expr>,
    },
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Call {
        name: String,
        args: Vec<Expr>,
        star: bool,
        distinct: bool,
    },
    In {
        e: Box<Expr>,
        list: Vec<Expr>,
        not: bool,
    },
    Like {
        e: Box<Expr>,
        pattern: Box<Expr>,
        not: bool,
    },
    IsNull {
        e: Box<Expr>,
        not: bool,
    },
    Between {
        e: Box<Expr>,
        lo: Box<Expr>,
        hi: Box<Expr>,
        not: bool,
    },
    Case {
        whens: Vec<(Expr, Expr)>,
        otherwise: Option<Box<Expr>>,
    },
}

#[derive(Clone, Debug)]
struct Expr {
    kind: ExprKind,
    from: usize,
    to: usize,
}

#[derive(Clone, Debug)]
enum Item {
    Star {
        table: Option<String>,
        from: usize,
        to: usize,
    },
    Expr {
        e: Expr,
        name: String,
    },
}

#[derive(Clone, Debug)]
struct Source {
    table: String,
    alias: String,
    from: usize,
    to: usize,
}

#[derive(Clone, Debug)]
struct Join {
    source: Source,
    on: Expr,
    left: bool,
}

#[derive(Clone, Debug)]
struct Statement {
    distinct: bool,
    items: Vec<Item>,
    from: Source,
    joins: Vec<Join>,
    filter: Option<Expr>,
    group_by: Vec<Expr>,
    having: Option<Expr>,
    order_by: Vec<(Expr, bool)>,
    limit: Option<f64>,
    offset: Option<f64>,
}

const AGGREGATES: [&str; 5] = ["count", "sum", "avg", "min", "max"];
const SCALARS: [&str; 9] = [
    "lower", "upper", "length", "round", "abs", "coalesce", "substr", "date", "replace",
];

struct Parser<'a> {
    tokens: Vec<Token>,
    source: &'a str,
    at: usize,
}

impl Parser<'_> {
    fn token(&self) -> &Token {
        &self.tokens[self.at.min(self.tokens.len() - 1)]
    }

    fn is(&self, word: &str) -> bool {
        let token = self.token();
        match token.kind {
            TokenType::Word => token.text.to_lowercase() == word,
            TokenType::Op => token.text == word,
            _ => false,
        }
    }

    fn take(&mut self, word: &str) -> bool {
        if !self.is(word) {
            return false;
        }
        self.at += 1;
        true
    }

    fn expect(&mut self, word: &str, what: &str) -> Result<Token, SqlError> {
        let token = self.token().clone();
        if !self.is(word) {
            let end = if token.kind == TokenType::End {
                ", at the end"
            } else {
                ""
            };
            return refuse(format!("Expected {what} here{end}."), token.from, token.to);
        }
        self.at += 1;
        Ok(token)
    }

    fn name(&mut self, what: &str) -> Result<Token, SqlError> {
        let token = self.token().clone();
        if token.kind != TokenType::Word || keyword(&token.text) {
            return refuse(
                format!("Expected {what} here."),
                token.from,
                token.to.max(token.from + 1),
            );
        }
        self.at += 1;
        Ok(token)
    }

    fn statement(&mut self) -> Result<Statement, SqlError> {
        if self.token().kind == TokenType::End {
            return refuse("Write a SELECT to run.", 0, 0);
        }
        self.expect("select", "SELECT")?;
        let distinct = self.take("distinct");
        let mut items = vec![self.item()?];
        while self.take(",") {
            items.push(self.item()?);
        }
        self.expect("from", "FROM")?;
        let from = self.source_()?;
        let mut joins = Vec::new();
        loop {
            let mut left = false;
            if self.take("left") {
                left = true;
                self.take("outer");
                self.expect("join", "JOIN")?;
            } else if self.take("inner") {
                self.expect("join", "JOIN")?;
            } else if !self.take("join") {
                break;
            }
            let source = self.source_()?;
            self.expect("on", "ON")?;
            let on = self.expr()?;
            joins.push(Join { source, on, left });
        }
        let filter = if self.take("where") { Some(self.expr()?) } else { None };
        let mut group_by = Vec::new();
        if self.take("group") {
            self.expect("by", "BY")?;
            group_by.push(self.expr()?);
            while self.take(",") {
                group_by.push(self.expr()?);
            }
        }
        let having = if self.take("having") { Some(self.expr()?) } else { None };
        let mut order_by = Vec::new();
        if self.take("order") {
            self.expect("by", "BY")?;
            loop {
                let e = self.expr()?;
                let desc = self.take("desc");
                if !desc {
                    self.take("asc");
                }
                order_by.push((e, desc));
                if !self.take(",") {
                    break;
                }
            }
        }
        let limit = if self.take("limit") {
            Some(self.count("LIMIT")?)
        } else {
            None
        };
        let offset = if self.take("offset") {
            Some(self.count("OFFSET")?)
        } else {
            None
        };
        self.take(";");
        let rest = self.token().clone();
        if rest.kind != TokenType::End {
            let message = if rest.text.to_lowercase() == "select" {
                "Run one statement at a time.".to_owned()
            } else {
                format!("{} is not expected here.", rest.text)
            };
            return refuse(message, rest.from, rest.to);
        }
        Ok(Statement {
            distinct,
            items,
            from,
            joins,
            filter,
            group_by,
            having,
            order_by,
            limit,
            offset,
        })
    }

    fn count(&mut self, what: &str) -> Result<f64, SqlError> {
        let token = self.token().clone();
        if token.kind != TokenType::Number || !token.text.bytes().all(|b| b.is_ascii_digit()) {
            return refuse(format!("{what} takes a whole number."), token.from, token.to);
        }
        self.at += 1;
        Ok(to_number(&token.text))
    }

    fn source_(&mut self) -> Result<Source, SqlError> {
        let table = self.name("a table")?;
        self.take("as");
        let token = self.token().clone();
        let alias = if token.kind == TokenType::Word && !keyword(&token.text) {
            self.at += 1;
            token.text
        } else {
            table.text.clone()
        };
        Ok(Source {
            table: table.text.to_lowercase(),
            alias: alias.to_lowercase(),
            from: table.from,
            to: table.to,
        })
    }

    fn item(&mut self) -> Result<Item, SqlError> {
        let token = self.token().clone();
        if self.take("*") {
            return Ok(Item::Star {
                table: None,
                from: token.from,
                to: token.to,
            });
        }
        let next = self.tokens.get(self.at + 1).map(|one| one.text.clone());
        let after = self.tokens.get(self.at + 2).cloned();
        if token.kind == TokenType::Word
            && next.as_deref() == Some(".")
            && let Some(after) = after.filter(|after| after.text == "*")
        {
            self.at += 3;
            return Ok(Item::Star {
                table: Some(token.text.to_lowercase()),
                from: token.from,
                to: after.to,
            });
        }
        let e = self.expr()?;
        let mut name = match &e.kind {
            ExprKind::Col { name, .. } => name.clone(),
            _ => self.source[e.from..e.to].to_owned(),
        };
        if self.take("as") {
            name = self.name("a name for the column")?.text;
        } else if self.token().kind == TokenType::Word && !keyword(&self.token().text) {
            name = self.name("a name")?.text;
        }
        Ok(Item::Expr { e, name })
    }

    fn expr(&mut self) -> Result<Expr, SqlError> {
        let mut left = self.and()?;
        while self.is("or") {
            self.at += 1;
            let right = self.and()?;
            left = binary("or", left, right);
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, SqlError> {
        let mut left = self.not()?;
        while self.is("and") {
            self.at += 1;
            let right = self.not()?;
            left = binary("and", left, right);
        }
        Ok(left)
    }

    fn not(&mut self) -> Result<Expr, SqlError> {
        let token = self.token().clone();
        if self.take("not") {
            let e = self.not()?;
            let to = e.to;
            return Ok(Expr {
                kind: ExprKind::Not(Box::new(e)),
                from: token.from,
                to,
            });
        }
        self.compare()
    }

    fn compare(&mut self) -> Result<Expr, SqlError> {
        let left = self.add()?;
        let token = self.token().clone();
        if token.kind == TokenType::Op && ["=", "!=", "<>", "<", "<=", ">", ">="].contains(&token.text.as_str()) {
            self.at += 1;
            let right = self.add()?;
            let op = if token.text == "<>" { "!=" } else { token.text.as_str() };
            return Ok(binary(op, left, right));
        }
        let from = left.from;
        if self.take("is") {
            let not = self.take("not");
            let end = self.expect("null", "NULL")?;
            return Ok(Expr {
                kind: ExprKind::IsNull { e: Box::new(left), not },
                from,
                to: end.to,
            });
        }
        let not = self.take("not");
        if self.take("like") {
            let pattern = self.add()?;
            let to = pattern.to;
            return Ok(Expr {
                kind: ExprKind::Like {
                    e: Box::new(left),
                    pattern: Box::new(pattern),
                    not,
                },
                from,
                to,
            });
        }
        if self.take("in") {
            self.expect("(", "(")?;
            let mut list = vec![self.add()?];
            while self.take(",") {
                list.push(self.add()?);
            }
            let end = self.expect(")", ")")?;
            return Ok(Expr {
                kind: ExprKind::In {
                    e: Box::new(left),
                    list,
                    not,
                },
                from,
                to: end.to,
            });
        }
        if self.take("between") {
            let lo = self.add()?;
            self.expect("and", "AND")?;
            let hi = self.add()?;
            let to = hi.to;
            return Ok(Expr {
                kind: ExprKind::Between {
                    e: Box::new(left),
                    lo: Box::new(lo),
                    hi: Box::new(hi),
                    not,
                },
                from,
                to,
            });
        }
        if not {
            let token = self.token();
            return refuse("Expected LIKE, IN or BETWEEN after NOT.", token.from, token.to);
        }
        Ok(left)
    }

    fn add(&mut self) -> Result<Expr, SqlError> {
        let mut left = self.mul()?;
        while self.token().kind == TokenType::Op && ["+", "-", "||"].contains(&self.token().text.as_str()) {
            let op = self.token().text.clone();
            self.at += 1;
            let right = self.mul()?;
            left = binary(&op, left, right);
        }
        Ok(left)
    }

    fn mul(&mut self) -> Result<Expr, SqlError> {
        let mut left = self.unary()?;
        while self.token().kind == TokenType::Op && ["*", "/", "%"].contains(&self.token().text.as_str()) {
            let op = self.token().text.clone();
            self.at += 1;
            let right = self.unary()?;
            left = binary(&op, left, right);
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, SqlError> {
        let token = self.token().clone();
        if self.take("-") {
            let e = self.unary()?;
            let to = e.to;
            return Ok(Expr {
                kind: ExprKind::Neg(Box::new(e)),
                from: token.from,
                to,
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, SqlError> {
        let token = self.token().clone();
        let (from, to) = (token.from, token.to);
        let lit = |value: Value| Expr {
            kind: ExprKind::Lit(value),
            from,
            to,
        };
        if token.kind == TokenType::Number {
            self.at += 1;
            return Ok(lit(Value::Num(to_number(&token.text))));
        }
        if token.kind == TokenType::Text {
            self.at += 1;
            return Ok(lit(Value::Str(token.text)));
        }
        if self.take("(") {
            let e = self.expr()?;
            let end = self.expect(")", ")")?;
            return Ok(Expr {
                kind: e.kind,
                from,
                to: end.to,
            });
        }
        if self.take("null") {
            return Ok(lit(Value::Null));
        }
        if self.take("true") {
            return Ok(lit(Value::Bool(true)));
        }
        if self.take("false") {
            return Ok(lit(Value::Bool(false)));
        }
        if self.take("case") {
            let mut whens = Vec::new();
            while self.take("when") {
                let when = self.expr()?;
                self.expect("then", "THEN")?;
                whens.push((when, self.expr()?));
            }
            if whens.is_empty() {
                let token = self.token();
                return refuse("CASE takes at least one WHEN.", token.from, token.to);
            }
            let otherwise = if self.take("else") {
                Some(Box::new(self.expr()?))
            } else {
                None
            };
            let end = self.expect("end", "END")?;
            return Ok(Expr {
                kind: ExprKind::Case { whens, otherwise },
                from,
                to: end.to,
            });
        }
        if token.kind == TokenType::Word && !keyword(&token.text) {
            self.at += 1;
            if self.take("(") {
                let name = token.text.to_lowercase();
                if !AGGREGATES.contains(&name.as_str()) && !SCALARS.contains(&name.as_str()) {
                    let all: Vec<&str> = AGGREGATES.iter().chain(SCALARS.iter()).copied().collect();
                    return refuse(
                        format!("No function is named {}. There are {}.", token.text, all.join(", ")),
                        from,
                        to,
                    );
                }
                let distinct = self.take("distinct");
                let mut args = Vec::new();
                let mut star = false;
                if self.take("*") {
                    star = true;
                } else if !self.is(")") {
                    args.push(self.expr()?);
                    while self.take(",") {
                        args.push(self.expr()?);
                    }
                }
                let end = self.expect(")", ")")?;
                return Ok(Expr {
                    kind: ExprKind::Call {
                        name,
                        args,
                        star,
                        distinct,
                    },
                    from,
                    to: end.to,
                });
            }
            if self.take(".") {
                let column = self.name("a column")?;
                return Ok(Expr {
                    kind: ExprKind::Col {
                        table: Some(token.text.to_lowercase()),
                        name: column.text.to_lowercase(),
                        bound: None,
                    },
                    from,
                    to: column.to,
                });
            }
            return Ok(Expr {
                kind: ExprKind::Col {
                    table: None,
                    name: token.text.to_lowercase(),
                    bound: None,
                },
                from,
                to,
            });
        }
        let message = if token.kind == TokenType::End {
            "The query ends too soon.".to_owned()
        } else {
            format!("{} is not expected here.", token.text)
        };
        refuse(message, from, to.max(from + 1))
    }
}

fn binary(op: &str, l: Expr, r: Expr) -> Expr {
    let (from, to) = (l.from, r.to);
    Expr {
        kind: ExprKind::Bin {
            op: op.to_owned(),
            l: Box::new(l),
            r: Box::new(r),
        },
        from,
        to,
    }
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Num(value) => *value != 0.0,
        Value::Str(text) => !text.is_empty(),
    }
}

/// How two values compare, as the web's `compare`: null first, numbers by their difference (NaN
/// when either is), a truth as a number beside a number or a truth, anything else as texts.
fn compare(a: &Value, b: &Value) -> f64 {
    match (a, b) {
        (Value::Null, Value::Null) => 0.0,
        (Value::Null, _) => -1.0,
        (_, Value::Null) => 1.0,
        (Value::Num(x), Value::Num(y)) => x - y,
        (Value::Str(_), _) | (_, Value::Str(_)) => match locale_compare(&a.say(), &b.say()) {
            Ordering::Less => -1.0,
            Ordering::Equal => 0.0,
            Ordering::Greater => 1.0,
        },
        _ if a.number().is_nan() || b.number().is_nan() => match locale_compare(&a.say(), &b.say()) {
            Ordering::Less => -1.0,
            Ordering::Equal => 0.0,
            Ordering::Greater => 1.0,
        },
        _ => a.number() - b.number(),
    }
}

fn has_aggregate(e: Option<&Expr>) -> bool {
    let Some(e) = e else {
        return false;
    };
    match &e.kind {
        ExprKind::Call { name, args, .. } => {
            AGGREGATES.contains(&name.as_str()) || args.iter().any(|arg| has_aggregate(Some(arg)))
        }
        ExprKind::Bin { l, r, .. } => has_aggregate(Some(l)) || has_aggregate(Some(r)),
        ExprKind::Not(e) | ExprKind::Neg(e) | ExprKind::IsNull { e, .. } => has_aggregate(Some(e)),
        ExprKind::In { e, list, .. } => has_aggregate(Some(e)) || list.iter().any(|one| has_aggregate(Some(one))),
        ExprKind::Like { e, pattern, .. } => has_aggregate(Some(e)) || has_aggregate(Some(pattern)),
        ExprKind::Between { e, lo, hi, .. } => {
            has_aggregate(Some(e)) || has_aggregate(Some(lo)) || has_aggregate(Some(hi))
        }
        ExprKind::Case { whens, otherwise } => {
            whens
                .iter()
                .any(|(when, result)| has_aggregate(Some(when)) || has_aggregate(Some(result)))
                || has_aggregate(otherwise.as_deref())
        }
        _ => false,
    }
}

/// The tables in a query's scope, by alias, in the order the query names them.
struct Scope<'t> {
    tables: Vec<(String, &'t Table)>,
}

impl Scope<'_> {
    fn find(&self, alias: &str) -> Option<usize> {
        self.tables.iter().position(|(one, _)| one == alias)
    }

    /// Finds every column before a row is read, so a wrong name is refused on an empty table too.
    fn bind(&self, e: Option<&mut Expr>, allow_output: bool, output: &HashSet<String>) -> Result<(), SqlError> {
        let Some(e) = e else {
            return Ok(());
        };
        let (from, to) = (e.from, e.to);
        match &mut e.kind {
            ExprKind::Col { table, name, bound } => {
                if let Some(table) = table {
                    let Some(at) = self.find(table) else {
                        return refuse(format!("No table in this query is called {table}."), from, to);
                    };
                    let found = self.tables[at].1;
                    let Some(column) = found.columns.iter().position(|c| &c.name == name) else {
                        return refuse(format!("{} has no column {name}.", found.name), from, to);
                    };
                    *bound = Some((at, column));
                    return Ok(());
                }
                let owners: Vec<(usize, usize)> = self
                    .tables
                    .iter()
                    .enumerate()
                    .filter_map(|(at, (_, table))| {
                        table
                            .columns
                            .iter()
                            .position(|c| &c.name == name)
                            .map(|column| (at, column))
                    })
                    .collect();
                if owners.len() == 1 {
                    *bound = Some(owners[0]);
                    return Ok(());
                }
                if owners.len() > 1 {
                    let names: Vec<&str> = owners.iter().map(|(at, _)| self.tables[*at].0.as_str()).collect();
                    return refuse(
                        format!(
                            "{name} is in {}; say which, as {}.{name}.",
                            names.join(" and "),
                            names[0]
                        ),
                        from,
                        to,
                    );
                }
                if allow_output && output.contains(name.as_str()) {
                    return Ok(());
                }
                refuse(format!("No column is named {name}."), from, to)
            }
            ExprKind::Bin { l, r, .. } => {
                self.bind(Some(l), allow_output, output)?;
                self.bind(Some(r), allow_output, output)
            }
            ExprKind::Not(e) | ExprKind::Neg(e) | ExprKind::IsNull { e, .. } => {
                self.bind(Some(e), allow_output, output)
            }
            ExprKind::Call { args, .. } => {
                for arg in args {
                    self.bind(Some(arg), allow_output, output)?;
                }
                Ok(())
            }
            ExprKind::In { e, list, .. } => {
                self.bind(Some(e), allow_output, output)?;
                for one in list {
                    self.bind(Some(one), allow_output, output)?;
                }
                Ok(())
            }
            ExprKind::Like { e, pattern, .. } => {
                self.bind(Some(e), allow_output, output)?;
                self.bind(Some(pattern), allow_output, output)
            }
            ExprKind::Between { e, lo, hi, .. } => {
                self.bind(Some(e), allow_output, output)?;
                self.bind(Some(lo), allow_output, output)?;
                self.bind(Some(hi), allow_output, output)
            }
            ExprKind::Case { whens, otherwise } => {
                for (when, result) in whens {
                    self.bind(Some(when), allow_output, output)?;
                    self.bind(Some(result), allow_output, output)?;
                }
                self.bind(otherwise.as_deref_mut(), allow_output, output)
            }
            ExprKind::Lit(_) => Ok(()),
        }
    }
}

/// A row of the joined tables: for each table in scope, the row of it taken, or none (a LEFT JOIN
/// that found nothing, or the empty group).
type Row = Vec<Option<usize>>;

struct Context<'r> {
    row: &'r Row,
    group: Option<&'r [Row]>,
}

/// The names and values of a result row's columns so far, as ORDER BY and HAVING read them.
type Output = HashMap<String, Value>;

struct Engine<'t> {
    scope: Scope<'t>,
}

impl Engine<'_> {
    fn cell(&self, row: &Row, at: usize, column: usize) -> Value {
        row.get(at)
            .copied()
            .flatten()
            .and_then(|index| self.scope.tables[at].1.rows.get(index))
            .and_then(|values| values.get(column))
            .cloned()
            .unwrap_or(Value::Null)
    }

    fn evaluate(&self, e: &Expr, context: &Context, output: Option<&Output>) -> Result<Value, SqlError> {
        let value = match &e.kind {
            ExprKind::Lit(value) => value.clone(),
            ExprKind::Col { name, bound, .. } => match bound {
                None => output
                    .and_then(|output| output.get(name))
                    .cloned()
                    .unwrap_or(Value::Null),
                Some((at, column)) => {
                    let row = context.group.and_then(|group| group.first()).unwrap_or(context.row);
                    self.cell(row, *at, *column)
                }
            },
            ExprKind::Not(inner) => Value::Bool(!truthy(&self.evaluate(inner, context, output)?)),
            ExprKind::Neg(inner) => match self.evaluate(inner, context, output)? {
                Value::Null => Value::Null,
                value => Value::Num(-value.number()),
            },
            ExprKind::IsNull { e, not } => {
                let is_null = self.evaluate(e, context, output)? == Value::Null;
                Value::Bool(if *not { !is_null } else { is_null })
            }
            ExprKind::Bin { op, l, r } => {
                if op == "and" {
                    return Ok(Value::Bool(
                        truthy(&self.evaluate(l, context, output)?) && truthy(&self.evaluate(r, context, output)?),
                    ));
                }
                if op == "or" {
                    return Ok(Value::Bool(
                        truthy(&self.evaluate(l, context, output)?) || truthy(&self.evaluate(r, context, output)?),
                    ));
                }
                let a = self.evaluate(l, context, output)?;
                let b = self.evaluate(r, context, output)?;
                if op == "||" {
                    return Ok(if a == Value::Null || b == Value::Null {
                        Value::Null
                    } else {
                        Value::Str(format!("{}{}", a.say(), b.say()))
                    });
                }
                if a == Value::Null || b == Value::Null {
                    return Ok(Value::Null);
                }
                let (x, y) = (a.number(), b.number());
                match op.as_str() {
                    "=" => Value::Bool(compare(&a, &b) == 0.0),
                    "!=" => Value::Bool(compare(&a, &b) != 0.0),
                    "<" => Value::Bool(compare(&a, &b) < 0.0),
                    "<=" => Value::Bool(compare(&a, &b) <= 0.0),
                    ">" => Value::Bool(compare(&a, &b) > 0.0),
                    ">=" => Value::Bool(compare(&a, &b) >= 0.0),
                    "+" => Value::Num(x + y),
                    "-" => Value::Num(x - y),
                    "*" => Value::Num(x * y),
                    "/" if y == 0.0 => Value::Null,
                    "/" => Value::Num(x / y),
                    "%" if y == 0.0 => Value::Null,
                    "%" => Value::Num(x % y),
                    _ => Value::Null,
                }
            }
            ExprKind::In { e, list, not } => {
                let value = self.evaluate(e, context, output)?;
                if value == Value::Null {
                    return Ok(Value::Null);
                }
                let mut found = false;
                for one in list {
                    if compare(&value, &self.evaluate(one, context, output)?) == 0.0 {
                        found = true;
                        break;
                    }
                }
                Value::Bool(if *not { !found } else { found })
            }
            ExprKind::Like { e, pattern, not } => {
                let value = self.evaluate(e, context, output)?;
                let pattern = self.evaluate(pattern, context, output)?;
                if value == Value::Null || pattern == Value::Null {
                    return Ok(Value::Null);
                }
                let matched = like(&value.say(), &pattern.say())?;
                Value::Bool(if *not { !matched } else { matched })
            }
            ExprKind::Between { e, lo, hi, not } => {
                let value = self.evaluate(e, context, output)?;
                let lo = self.evaluate(lo, context, output)?;
                let hi = self.evaluate(hi, context, output)?;
                if value == Value::Null || lo == Value::Null || hi == Value::Null {
                    return Ok(Value::Null);
                }
                let inside = compare(&value, &lo) >= 0.0 && compare(&value, &hi) <= 0.0;
                Value::Bool(if *not { !inside } else { inside })
            }
            ExprKind::Case { whens, otherwise } => {
                for (when, result) in whens {
                    if truthy(&self.evaluate(when, context, output)?) {
                        return self.evaluate(result, context, output);
                    }
                }
                match otherwise {
                    Some(otherwise) => self.evaluate(otherwise, context, output)?,
                    None => Value::Null,
                }
            }
            ExprKind::Call {
                name,
                args,
                star,
                distinct,
            } => return self.call(e, name, args, *star, *distinct, context, output),
        };
        Ok(value)
    }

    #[allow(clippy::too_many_arguments)]
    fn call(
        &self,
        e: &Expr,
        name: &str,
        args: &[Expr],
        star: bool,
        distinct: bool,
        context: &Context,
        output: Option<&Output>,
    ) -> Result<Value, SqlError> {
        if AGGREGATES.contains(&name) {
            let Some(group) = context.group else {
                return refuse(format!("{name} needs a group of rows."), e.from, e.to);
            };
            if star {
                if name != "count" {
                    return refuse(format!("{name}(*) means nothing; name a column."), e.from, e.to);
                }
                return Ok(Value::Num(group.len() as f64));
            }
            let (Some(arg), 1) = (args.first(), args.len()) else {
                return refuse(format!("{name} takes one value."), e.from, e.to);
            };
            let mut values = Vec::new();
            for row in group {
                let value = self.evaluate(arg, &Context { row, group: None }, None)?;
                if value != Value::Null {
                    values.push(value);
                }
            }
            if distinct {
                // The last value under each key, in the place of the first.
                let mut keys: Vec<String> = Vec::new();
                let mut kept: Vec<Value> = Vec::new();
                for value in values {
                    let key = match &value {
                        Value::Num(n) => format!("number:{}", number_to_string(*n)),
                        Value::Bool(b) => format!("boolean:{b}"),
                        Value::Str(text) => format!("string:{text}"),
                        Value::Null => "object:null".to_owned(),
                    };
                    match keys.iter().position(|one| *one == key) {
                        Some(at) => kept[at] = value,
                        None => {
                            keys.push(key);
                            kept.push(value);
                        }
                    }
                }
                values = kept;
            }
            if name == "count" {
                return Ok(Value::Num(values.len() as f64));
            }
            let Some(first) = values.first().cloned() else {
                return Ok(Value::Null);
            };
            if name == "min" {
                return Ok(values
                    .into_iter()
                    .skip(1)
                    .fold(first, |a, b| if compare(&a, &b) <= 0.0 { a } else { b }));
            }
            if name == "max" {
                return Ok(values
                    .into_iter()
                    .skip(1)
                    .fold(first, |a, b| if compare(&a, &b) >= 0.0 { a } else { b }));
            }
            let sum = values.iter().fold(0.0, |total, value| total + value.number());
            return Ok(Value::Num(if name == "sum" { sum } else { sum / values.len() as f64 }));
        }
        let mut values = Vec::new();
        for arg in args {
            values.push(self.evaluate(arg, context, output)?);
        }
        let first = values.first().cloned().unwrap_or(Value::Null);
        let second = values.get(1).cloned().unwrap_or(Value::Null);
        let third = values.get(2).cloned().unwrap_or(Value::Null);
        let null = Value::Null;
        Ok(match name {
            "coalesce" => values.into_iter().find(|v| *v != Value::Null).unwrap_or(Value::Null),
            _ if first == null => Value::Null,
            "lower" => Value::Str(first.say().to_lowercase()),
            "upper" => Value::Str(first.say().to_uppercase()),
            "length" => Value::Num(length(&first.say()) as f64),
            "abs" => Value::Num(first.number().abs()),
            "round" => {
                let places = if second == null { 0.0 } else { second.number() };
                let scale = 10f64.powf(places);
                Value::Num(round(first.number() * scale) / scale)
            }
            "date" => Value::Str(slice(&first.say(), 0.0, Some(10.0))),
            "substr" => {
                let start_at = if second == null { 1.0 } else { second.number() };
                let start = (start_at - 1.0).max(0.0);
                let start = if start_at.is_nan() { f64::NAN } else { start };
                let text = first.say();
                if third == null {
                    Value::Str(slice(&text, start, None))
                } else {
                    Value::Str(slice(&text, start, Some(start + third.number())))
                }
            }
            "replace" => {
                let search = if second == null { String::new() } else { second.say() };
                let with = if third == null { String::new() } else { third.say() };
                Value::Str(crate::js::replace_all(&first.say(), &search, &with))
            }
            _ => Value::Null,
        })
    }
}

/// One thing a LIKE pattern matches: a UTF-16 unit in either case, or any one unit; repeated any
/// number of times when starred.
#[derive(Clone, Copy, Debug)]
struct Atom {
    unit: Option<u16>,
    star: bool,
}

/// A unit as a case-blind match of the web's regular expressions reads it: in upper case, unless
/// that takes more than one unit or turns a character past ASCII into one within it.
fn canonical(unit: u16) -> u16 {
    let Some(c) = char::from_u32(u32::from(unit)) else {
        return unit;
    };
    let mut upper = c.to_uppercase();
    match (upper.next(), upper.next()) {
        (Some(one), None) if (one as u32) < 0x10000 => {
            if unit >= 128 && (one as u32) < 128 {
                unit
            } else {
                one as u16
            }
        }
        _ => unit,
    }
}

/// Whether a text matches a LIKE pattern, as the web reads one: `%` any run, `_` any one unit,
/// case-blind, and a `*` repeating what is before it (the web leaves it out of what it escapes); a
/// `*` with nothing to repeat is refused as the web's regular expressions refuse it.
fn like(text: &str, pattern: &str) -> Result<bool, SqlError> {
    let mut atoms: Vec<Atom> = Vec::new();
    for unit in pattern.encode_utf16() {
        match unit {
            0x25 => atoms.push(Atom { unit: None, star: true }),
            0x5f => atoms.push(Atom {
                unit: None,
                star: false,
            }),
            0x2a => match atoms.last_mut() {
                Some(last) if !last.star => last.star = true,
                _ => {
                    return Err(SqlError {
                        message: "SyntaxError: Invalid regular expression: nothing to repeat".to_owned(),
                        at: None,
                    });
                }
            },
            unit => atoms.push(Atom {
                unit: Some(canonical(unit)),
                star: false,
            }),
        }
    }
    let units: Vec<u16> = text.encode_utf16().map(canonical).collect();
    // matched[a][u]: whether atoms from a on match units from u on.
    let mut matched = vec![vec![false; units.len() + 1]; atoms.len() + 1];
    matched[atoms.len()][units.len()] = true;
    for a in (0..atoms.len()).rev() {
        for u in (0..=units.len()).rev() {
            let atom = atoms[a];
            let one = u < units.len() && atom.unit.is_none_or(|want| want == units[u]);
            matched[a][u] = if atom.star {
                matched[a + 1][u] || (one && matched[a][u + 1])
            } else {
                one && matched[a + 1][u + 1]
            };
        }
    }
    Ok(matched[0][0])
}

/// Runs a query over the tables.
pub fn run_query(source: &str, tables: &[Table]) -> Result<QueryResult, SqlError> {
    let started = Instant::now();
    let mut statement = Parser {
        tokens: tokenize(source)?,
        source,
        at: 0,
    }
    .statement()?;
    let mut scope = Scope { tables: Vec::new() };
    let sources: Vec<Source> = std::iter::once(statement.from.clone())
        .chain(statement.joins.iter().map(|join| join.source.clone()))
        .collect();
    for one in &sources {
        let Some(table) = tables.iter().find(|table| table.name == one.table) else {
            let names: Vec<&str> = tables.iter().map(|table| table.name.as_str()).collect();
            return refuse(
                format!("No table is named {}. The tables are {}.", one.table, names.join(", ")),
                one.from,
                one.to,
            );
        };
        if scope.find(&one.alias).is_some() {
            return refuse(
                format!("{} names two tables; give one another name.", one.alias),
                one.from,
                one.to,
            );
        }
        scope.tables.push((one.alias.clone(), table));
    }

    let output_names: HashSet<String> = statement
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Expr { name, .. } => Some(name.to_lowercase()),
            Item::Star { .. } => None,
        })
        .collect();
    for item in &mut statement.items {
        match item {
            Item::Star {
                table: Some(table),
                from,
                to,
            } if scope.find(table).is_none() => {
                return refuse(format!("No table in this query is called {table}."), *from, *to);
            }
            Item::Expr { e, .. } => scope.bind(Some(e), false, &output_names)?,
            Item::Star { .. } => {}
        }
    }
    for join in &mut statement.joins {
        scope.bind(Some(&mut join.on), false, &output_names)?;
    }
    if let Some(filter) = &statement.filter
        && has_aggregate(Some(filter))
    {
        return refuse(
            "WHERE cannot hold a count or a sum; use HAVING.",
            filter.from,
            filter.to,
        );
    }
    scope.bind(statement.filter.as_mut(), false, &output_names)?;
    for e in &mut statement.group_by {
        scope.bind(Some(e), false, &output_names)?;
    }
    scope.bind(statement.having.as_mut(), true, &output_names)?;
    for (e, _) in &mut statement.order_by {
        if !matches!(e.kind, ExprKind::Lit(Value::Num(_))) {
            scope.bind(Some(e), true, &output_names)?;
        }
    }

    let engine = Engine { scope };
    let count = engine.scope.tables.len();
    // FROM and the joins, as rows holding a row of each table in scope.
    let mut rows: Vec<Row> = (0..engine.scope.tables[0].1.rows.len())
        .map(|index| {
            let mut row = vec![None; count];
            row[0] = Some(index);
            row
        })
        .collect();
    for (at, join) in statement.joins.iter().enumerate() {
        let table = at + 1;
        let mut next = Vec::new();
        for row in &rows {
            let mut matched = false;
            for index in 0..engine.scope.tables[table].1.rows.len() {
                let mut joined = row.clone();
                joined[table] = Some(index);
                if truthy(&engine.evaluate(
                    &join.on,
                    &Context {
                        row: &joined,
                        group: None,
                    },
                    None,
                )?) {
                    next.push(joined);
                    matched = true;
                }
            }
            if !matched && join.left {
                next.push(row.clone());
            }
        }
        rows = next;
    }
    if let Some(filter) = &statement.filter {
        let mut kept = Vec::new();
        for row in rows {
            if truthy(&engine.evaluate(filter, &Context { row: &row, group: None }, None)?) {
                kept.push(row);
            }
        }
        rows = kept;
    }

    // The columns: a star stands for every column of its table, or of every table.
    enum Out<'e> {
        Cell(usize, usize),
        Expr(&'e Expr),
    }
    let mut columns: Vec<(String, Out)> = Vec::new();
    for item in &statement.items {
        match item {
            Item::Star { table, .. } => {
                let chosen: Vec<usize> = match table {
                    None => (0..count).collect(),
                    Some(table) => engine.scope.find(table).into_iter().collect(),
                };
                for at in chosen {
                    let (alias, found) = &engine.scope.tables[at];
                    for (column, info) in found.columns.iter().enumerate() {
                        let name = if count > 1 && table.is_none() {
                            format!("{alias}.{}", info.name)
                        } else {
                            info.name.clone()
                        };
                        columns.push((name, Out::Cell(at, column)));
                    }
                }
            }
            Item::Expr { e, name } => columns.push((name.clone(), Out::Expr(e))),
        }
    }

    let grouped = !statement.group_by.is_empty()
        || statement.items.iter().any(|item| match item {
            Item::Expr { e, .. } => has_aggregate(Some(e)),
            Item::Star { .. } => false,
        })
        || has_aggregate(statement.having.as_ref())
        || statement.order_by.iter().any(|(e, _)| has_aggregate(Some(e)));
    let empty: Row = vec![None; count];
    // Each result row's context: a row, or a group of rows and the first of them.
    let mut contexts: Vec<(Row, Option<Vec<Row>>)> = Vec::new();
    if grouped {
        let mut keys: Vec<String> = Vec::new();
        let mut groups: Vec<Vec<Row>> = Vec::new();
        for row in rows {
            let mut key = Vec::new();
            for e in &statement.group_by {
                key.push(engine.evaluate(e, &Context { row: &row, group: None }, None)?.key());
            }
            let key = key.join(",");
            match keys.iter().position(|one| *one == key) {
                Some(at) => groups[at].push(row),
                None => {
                    keys.push(key);
                    groups.push(vec![row]);
                }
            }
        }
        if statement.group_by.is_empty() && groups.is_empty() {
            groups.push(Vec::new());
        }
        contexts = groups
            .into_iter()
            .map(|group| (group.first().cloned().unwrap_or_else(|| empty.clone()), Some(group)))
            .collect();
    } else {
        contexts.extend(rows.into_iter().map(|row| (row, None)));
    }

    struct Done<'c> {
        context: &'c (Row, Option<Vec<Row>>),
        output: Output,
        values: Vec<Value>,
    }
    let mut out: Vec<Done> = Vec::new();
    for context in &contexts {
        let reading = Context {
            row: &context.0,
            group: context.1.as_deref(),
        };
        let mut output = Output::new();
        let mut values = Vec::new();
        for (name, column) in &columns {
            let value = match column {
                Out::Cell(at, index) => {
                    let row = reading.group.and_then(|group| group.first()).unwrap_or(reading.row);
                    engine.cell(row, *at, *index)
                }
                Out::Expr(e) => engine.evaluate(e, &reading, Some(&output))?,
            };
            output.insert(name.to_lowercase(), value.clone());
            values.push(value);
        }
        out.push(Done {
            context,
            output,
            values,
        });
    }
    let reading = |one: &Done| -> (Row, Option<Vec<Row>>) { one.context.clone() };
    if let Some(having) = &statement.having {
        let mut kept = Vec::new();
        for one in out {
            let (row, group) = reading(&one);
            let context = Context {
                row: &row,
                group: group.as_deref(),
            };
            if truthy(&engine.evaluate(having, &context, Some(&one.output))?) {
                kept.push(one);
            }
        }
        out = kept;
    }
    if statement.distinct {
        let mut seen = HashSet::new();
        out.retain(|one| {
            let key: Vec<String> = one.values.iter().map(Value::key).collect();
            seen.insert(key.join(","))
        });
    }
    if !statement.order_by.is_empty() {
        let mut keyed: Vec<(usize, Vec<Value>)> = Vec::new();
        for (index, one) in out.iter().enumerate() {
            let (row, group) = reading(one);
            let context = Context {
                row: &row,
                group: group.as_deref(),
            };
            let mut keys = Vec::new();
            for (e, _) in &statement.order_by {
                if let ExprKind::Lit(Value::Num(place)) = e.kind {
                    let index = place - 1.0;
                    if index < 0.0 || index >= columns.len() as f64 {
                        return refuse(format!("There are {} columns to sort by.", columns.len()), e.from, e.to);
                    }
                    keys.push(
                        if index.fract() == 0.0 {
                            one.values.get(index as usize).cloned()
                        } else {
                            None
                        }
                        .unwrap_or(Value::Null),
                    );
                    continue;
                }
                keys.push(engine.evaluate(e, &context, Some(&one.output))?);
            }
            keyed.push((index, keys));
        }
        sort_by(&mut keyed, |a, b| {
            for (at, (_, desc)) in statement.order_by.iter().enumerate() {
                let by = compare(&a.1[at], &b.1[at]);
                if by != 0.0 {
                    if by.is_nan() {
                        return Ordering::Equal;
                    }
                    let by = if *desc { -by } else { by };
                    return if by < 0.0 { Ordering::Less } else { Ordering::Greater };
                }
            }
            Ordering::Equal
        });
        let mut slots: Vec<Option<Done>> = out.into_iter().map(Some).collect();
        out = keyed.into_iter().filter_map(|(index, _)| slots[index].take()).collect();
    }
    let start = statement.offset.unwrap_or(0.0);
    let (from, to) = slice_range(out.len(), start, statement.limit.map(|limit| start + limit));
    let rows: Vec<Vec<Value>> = out.drain(..).skip(from).take(to - from).map(|one| one.values).collect();
    Ok(QueryResult {
        columns: columns.into_iter().map(|(name, _)| name).collect(),
        rows,
        ms: started.elapsed().as_secs_f64() * 1000.0,
    })
}
