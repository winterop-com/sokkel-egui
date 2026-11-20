//! FHIRPath, as the web's pages run it through the `fhirpath` engine (5.2.3) against the R4 model:
//! an expression read into a tree, with the words ANTLR's parser refuses one with, and evaluated
//! against a resource or a Bundle to the collection the engine answers, every value written as its
//! `evaluate` writes it (a date, a time and a quantity as text).
//!
//! The model is the workbench's own (`types`): the elements of the types the server keeps, the
//! data types they are made of, a Bundle's, and R4's choice elements on them (`value[x]`,
//! `effective[x]`, `onset[x]`, ...), which is all the engine reads of R4 on the server's
//! resources. Numbers are JavaScript's, as the engine's are by default; equality and order between
//! numbers round both to 1e-8 first, as it does. A quantity is compared and added to one of the
//! same unit; between units the engine converts by UCUM, which this does not (it answers empty).

use std::collections::BTreeMap;

use serde_json::Value;

use crate::js::{
    civil_from_days, days_from_civil, days_in_month, iso_day, json_number as number, number_to_string, round, stringify,
};
use crate::types::{datatype, type_def};
use sokkel_egui::regex::Regex;

// --- Tokens -----------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Delimited(String),
    Str(String),
    Num(String),
    Long(String),
    Date(String),
    DateTime(String),
    Time(String),
    /// `$this`, `$index` or `$total`.
    Var(&'static str),
    Punct(&'static str),
    Eof,
}

#[derive(Clone, Debug)]
struct Token {
    tok: Tok,
    /// The token as written, as ANTLR quotes it in an error.
    text: String,
    line: usize,
    column: usize,
}

const PUNCTS: [&str; 20] = [
    "<=", ">=", "!=", "!~", ".", "[", "]", "(", ")", "{", "}", ",", "+", "-", "*", "/", "|", "&", "=", "~",
];

/// The words that are operators and never name an element.
const OPERATOR_WORDS: [&str; 8] = ["and", "or", "xor", "implies", "div", "mod", "true", "false"];

const CALENDAR_UNITS: [&str; 16] = [
    "year",
    "years",
    "month",
    "months",
    "week",
    "weeks",
    "day",
    "days",
    "hour",
    "hours",
    "minute",
    "minutes",
    "second",
    "seconds",
    "millisecond",
    "milliseconds",
];

fn place(line: usize, column: usize, message: &str) -> String {
    format!("line: {line}; column: {column}; message: {message}")
}

struct Lexer {
    chars: Vec<char>,
    at: usize,
    line: usize,
    column: usize,
    errors: Vec<String>,
}

impl Lexer {
    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.at + ahead).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.at).copied()?;
        self.at += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    fn digits(&self, from: usize, count: usize) -> bool {
        (0..count).all(|k| self.chars.get(from + k).is_some_and(char::is_ascii_digit))
    }

    /// How many characters a time (`HH(:mm(:ss(.f+)?)?)?`) takes from `from`, if one starts there.
    fn time_len(&self, from: usize) -> Option<usize> {
        if !self.digits(from, 2) {
            return None;
        }
        let mut len = 2;
        if self.chars.get(from + len) == Some(&':') && self.digits(from + len + 1, 2) {
            len += 3;
            if self.chars.get(from + len) == Some(&':') && self.digits(from + len + 1, 2) {
                len += 3;
                if self.chars.get(from + len) == Some(&'.') && self.digits(from + len + 1, 1) {
                    len += 2;
                    while self.chars.get(from + len).is_some_and(char::is_ascii_digit) {
                        len += 1;
                    }
                }
            }
        }
        Some(len)
    }

    /// How many characters a zone (`Z` or `+hh:mm`) takes from `from`.
    fn zone_len(&self, from: usize) -> usize {
        match self.chars.get(from) {
            Some('Z') => 1,
            Some('+' | '-')
                if self.digits(from + 1, 2) && self.chars.get(from + 3) == Some(&':') && self.digits(from + 4, 2) =>
            {
                6
            }
            _ => 0,
        }
    }

    /// A date, date and time, or time literal after `@`, and how many characters it takes.
    fn temporal(&self) -> Option<(Tok, usize)> {
        let start = self.at + 1;
        if self.chars.get(start) == Some(&'T') {
            let len = self.time_len(start + 1)?;
            let text: String = self.chars[start + 1..start + 1 + len].iter().collect();
            return Some((Tok::Time(text), len + 2));
        }
        if !self.digits(start, 4) {
            return None;
        }
        let mut len = 4;
        if self.chars.get(start + len) == Some(&'-') && self.digits(start + len + 1, 2) {
            len += 3;
            if self.chars.get(start + len) == Some(&'-') && self.digits(start + len + 1, 2) {
                len += 3;
            }
        }
        if self.chars.get(start + len) == Some(&'T') {
            len += 1;
            if let Some(time) = self.time_len(start + len) {
                len += time;
                len += self.zone_len(start + len);
            }
            let text: String = self.chars[start..start + len].iter().collect();
            return Some((Tok::DateTime(text), len + 1));
        }
        let text: String = self.chars[start..start + len].iter().collect();
        Some((Tok::Date(text), len + 1))
    }

    fn next(&mut self) -> Token {
        loop {
            match self.peek(0) {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek(1) == Some('/') => {
                    while self.peek(0).is_some_and(|c| c != '\n') {
                        self.bump();
                    }
                }
                Some('/') if self.peek(1) == Some('*') => {
                    let close = (self.at + 2..self.chars.len().saturating_sub(1))
                        .find(|k| self.chars[*k] == '*' && self.chars[k + 1] == '/');
                    match close {
                        Some(end) => {
                            while self.at < end + 2 {
                                self.bump();
                            }
                        }
                        None => break,
                    }
                }
                _ => break,
            }
        }
        let (line, column, start) = (self.line, self.column, self.at);
        let make = |lexer: &mut Lexer, tok: Tok, len: usize| {
            for _ in 0..len {
                lexer.bump();
            }
            Token {
                tok,
                text: lexer.chars[start..start + len].iter().collect(),
                line,
                column,
            }
        };
        let Some(c) = self.peek(0) else {
            return Token {
                tok: Tok::Eof,
                text: "<EOF>".to_owned(),
                line,
                column,
            };
        };
        if c.is_ascii_alphabetic() || c == '_' {
            let mut len = 1;
            while self.peek(len).is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                len += 1;
            }
            let word: String = self.chars[start..start + len].iter().collect();
            return make(self, Tok::Ident(word), len);
        }
        if c.is_ascii_digit() {
            let mut len = 1;
            while self.peek(len).is_some_and(|c| c.is_ascii_digit()) {
                len += 1;
            }
            if self.peek(len) == Some('L') {
                let text: String = self.chars[start..start + len].iter().collect();
                return make(self, Tok::Long(text), len + 1);
            }
            if self.peek(len) == Some('.') && self.peek(len + 1).is_some_and(|c| c.is_ascii_digit()) {
                len += 2;
                while self.peek(len).is_some_and(|c| c.is_ascii_digit()) {
                    len += 1;
                }
            }
            let text: String = self.chars[start..start + len].iter().collect();
            return make(self, Tok::Num(text), len);
        }
        if c == '\'' || c == '`' {
            let mut len = 1;
            let mut out = String::new();
            loop {
                match self.peek(len) {
                    None => {
                        let text: String = self.chars[start..].iter().collect();
                        self.errors
                            .push(place(line, column, &format!("token recognition error at: '{text}'")));
                        while self.bump().is_some() {}
                        return self.next();
                    }
                    Some('\\') => {
                        let Some(next) = self.peek(len + 1) else {
                            len += 1;
                            continue;
                        };
                        len += 2;
                        match next {
                            'n' => out.push('\n'),
                            'r' => out.push('\r'),
                            't' => out.push('\t'),
                            'f' => out.push('\u{c}'),
                            'u' => {
                                let hex: String = self.chars[self.at + len..(self.at + len + 4).min(self.chars.len())]
                                    .iter()
                                    .collect();
                                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                    out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                                    len += 4;
                                }
                            }
                            other => out.push(other),
                        }
                    }
                    Some(end) if end == c => {
                        len += 1;
                        let tok = if c == '\'' { Tok::Str(out) } else { Tok::Delimited(out) };
                        return make(self, tok, len);
                    }
                    Some(other) => {
                        out.push(other);
                        len += 1;
                    }
                }
            }
        }
        if c == '@' {
            if let Some((tok, len)) = self.temporal() {
                return make(self, tok, len);
            }
            self.errors.push(place(line, column, "token recognition error at: '@'"));
            self.bump();
            return self.next();
        }
        if c == '$' {
            for name in ["$this", "$index", "$total"] {
                let len = name.chars().count();
                let word: String = self.chars[start..(start + len).min(self.chars.len())].iter().collect();
                if word == name && !self.peek(len).is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return make(self, Tok::Var(name), len);
                }
            }
        }
        if c == '%' {
            return make(self, Tok::Punct("%"), 1);
        }
        if c == '<' || c == '>' {
            if self.peek(1) == Some('=') {
                return make(self, Tok::Punct(if c == '<' { "<=" } else { ">=" }), 2);
            }
            return make(self, Tok::Punct(if c == '<' { "<" } else { ">" }), 1);
        }
        if c == '!' {
            match self.peek(1) {
                Some('=') => return make(self, Tok::Punct("!="), 2),
                Some('~') => return make(self, Tok::Punct("!~"), 2),
                Some(next) => {
                    self.errors
                        .push(place(line, column, &format!("token recognition error at: '!{next}'")));
                    self.bump();
                    self.bump();
                    return self.next();
                }
                None => {
                    self.errors.push(place(line, column, "token recognition error at: '!'"));
                    self.bump();
                    return self.next();
                }
            }
        }
        for punct in PUNCTS {
            if punct.len() == 1 && punct.starts_with(c) {
                return make(self, Tok::Punct(punct), 1);
            }
        }
        self.errors
            .push(place(line, column, &format!("token recognition error at: '{c}'")));
        self.bump();
        self.next()
    }
}

// --- The tree ---------------------------------------------------------------------------------------

/// An expression read.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Empty,
    Bool(bool),
    Str(String),
    Number {
        text: String,
        decimal: bool,
    },
    Date(String),
    DateTime(String),
    Time(String),
    Quantity {
        value: String,
        unit: String,
        calendar: bool,
    },
    /// An element or a type named at the start of a path.
    Name(String),
    /// A function called at the start of a path, on `$this`.
    Function(String, Vec<Expr>),
    This,
    Index,
    Total,
    /// `%name`.
    Constant(String),
    Member(Box<Expr>, String),
    Call(Box<Expr>, String, Vec<Expr>),
    Indexer(Box<Expr>, Box<Expr>),
    /// A sign before an operand, with the line and the column (from 1) it is written at.
    Polarity(&'static str, Box<Expr>, usize, usize),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Type(&'static str, Box<Expr>, String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Closer {
    Paren,
    Args,
    Index,
    Brace,
}

impl Closer {
    fn text(self) -> &'static str {
        match self {
            Closer::Paren | Closer::Args => ")",
            Closer::Index => "]",
            Closer::Brace => "}",
        }
    }
}

const EXPR_SET: &str = "{'+', '-', 'is', 'as', 'in', 'contains', '(', '{', 'true', 'false', '%', '$this', '$index', '$total', 'sort', '`sort`', 'asc', 'desc', DATE, DATETIME, TIME, IDENTIFIER, DELIMITEDIDENTIFIER, STRING, INTEGER, DECIMAL, LONGNUMBER}";
const EXPR_OR_CLOSE_SET: &str = "{'+', '-', 'is', 'as', 'in', 'contains', '(', ')', '{', 'true', 'false', '%', '$this', '$index', '$total', 'sort', '`sort`', 'asc', 'desc', DATE, DATETIME, TIME, IDENTIFIER, DELIMITEDIDENTIFIER, STRING, INTEGER, DECIMAL, LONGNUMBER}";
const MEMBER_SET: &str = "{'is', 'as', 'in', 'contains', '$this', '$index', '$total', 'sort', '`sort`', 'asc', 'desc', IDENTIFIER, DELIMITEDIDENTIFIER}";
const TYPE_SET: &str = "{'is', 'as', 'in', 'contains', 'sort', 'asc', 'desc', IDENTIFIER, DELIMITEDIDENTIFIER}";

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    errors: Vec<String>,
    closers: Vec<Closer>,
}

/// Why the parse stopped: an error already said.
struct Stop;

fn starts_expression(tok: &Tok) -> bool {
    match tok {
        Tok::Ident(word) => !OPERATOR_WORDS.contains(&word.as_str()) || word == "true" || word == "false",
        Tok::Punct(p) => matches!(*p, "+" | "-" | "(" | "{" | "%"),
        Tok::Eof => false,
        _ => true,
    }
}

fn names_member(tok: &Tok) -> bool {
    match tok {
        Tok::Ident(word) => !OPERATOR_WORDS.contains(&word.as_str()),
        Tok::Delimited(_) | Tok::Var(_) => true,
        _ => false,
    }
}

impl Parser {
    fn token(&self) -> &Token {
        &self.tokens[self.at.min(self.tokens.len() - 1)]
    }

    fn ahead(&self, by: usize) -> &Token {
        &self.tokens[(self.at + by).min(self.tokens.len() - 1)]
    }

    fn previous(&self) -> Option<&Token> {
        self.at.checked_sub(1).map(|at| &self.tokens[at])
    }

    fn error(&mut self, token: &Token, message: String) {
        self.errors.push(place(token.line, token.column, &message));
    }

    fn is_punct(&self, punct: &str) -> bool {
        self.token().tok
            == Tok::Punct(match punct {
                "(" => "(",
                ")" => ")",
                "[" => "[",
                "]" => "]",
                "{" => "{",
                "}" => "}",
                "," => ",",
                "." => ".",
                _ => return false,
            })
    }

    /// The operator the token is, with its precedence, when it continues an expression.
    fn infix(&self) -> Option<(&'static str, u8)> {
        Some(match &self.token().tok {
            Tok::Ident(word) => match word.as_str() {
                "implies" => ("implies", 1),
                "or" => ("or", 2),
                "xor" => ("xor", 2),
                "and" => ("and", 3),
                "in" => ("in", 4),
                "contains" => ("contains", 4),
                "is" => ("is", 8),
                "as" => ("as", 8),
                "div" => ("div", 10),
                "mod" => ("mod", 10),
                _ => return None,
            },
            Tok::Punct(p) => match *p {
                "=" => ("=", 5),
                "~" => ("~", 5),
                "!=" => ("!=", 5),
                "!~" => ("!~", 5),
                "<" => ("<", 6),
                "<=" => ("<=", 6),
                ">" => (">", 6),
                ">=" => (">=", 6),
                "|" => ("|", 7),
                "+" => ("+", 9),
                "-" => ("-", 9),
                "&" => ("&", 9),
                "*" => ("*", 10),
                "/" => ("/", 10),
                "." => (".", 12),
                "[" => ("[", 12),
                _ => return None,
            },
            _ => return None,
        })
    }

    fn expression(&mut self, min: u8, first_argument: bool) -> Result<Expr, Stop> {
        let mut left = self.prefix(first_argument)?;
        while let Some((op, prec)) = self.infix() {
            if prec < min {
                break;
            }
            self.at += 1;
            left = match op {
                "." => {
                    let name = self.member()?;
                    if self.is_punct("(") {
                        self.at += 1;
                        let args = self.arguments()?;
                        Expr::Call(Box::new(left), name, args)
                    } else {
                        Expr::Member(Box::new(left), name)
                    }
                }
                "[" => {
                    self.closers.push(Closer::Index);
                    let index = self.expression(0, false)?;
                    self.close(Closer::Index)?;
                    Expr::Indexer(Box::new(left), Box::new(index))
                }
                "is" | "as" => {
                    let kind = self.type_name()?;
                    Expr::Type(op, Box::new(left), kind)
                }
                _ => {
                    let right = self.expression(prec + 1, false)?;
                    Expr::Binary(op, Box::new(left), Box::new(right))
                }
            };
        }
        Ok(left)
    }

    fn type_name(&mut self) -> Result<String, Stop> {
        let mut name = match &self.token().tok {
            Tok::Ident(word) if !OPERATOR_WORDS.contains(&word.as_str()) => word.clone(),
            Tok::Delimited(word) => word.clone(),
            _ => {
                let token = self.token().clone();
                self.error(&token, format!("missing {TYPE_SET} at '{}'", token.text));
                return Err(Stop);
            }
        };
        self.at += 1;
        while self.is_punct(".") && names_member(&self.ahead(1).tok) && !matches!(self.ahead(1).tok, Tok::Var(_)) {
            self.at += 1;
            if let Tok::Ident(word) | Tok::Delimited(word) = &self.token().tok {
                name.push('.');
                name.push_str(word);
            }
            self.at += 1;
        }
        Ok(name)
    }

    fn member(&mut self) -> Result<String, Stop> {
        let token = self.token().clone();
        let name = match &token.tok {
            Tok::Ident(word) if !OPERATOR_WORDS.contains(&word.as_str()) => word.clone(),
            Tok::Delimited(word) => word.clone(),
            Tok::Var(var) => (*var).to_owned(),
            _ => {
                if names_member(&self.ahead(1).tok) {
                    self.error(
                        &token,
                        format!("extraneous input '{}' expecting {MEMBER_SET}", token.text),
                    );
                    self.at += 1;
                    return self.member();
                }
                self.error(
                    &token,
                    format!("mismatched input '{}' expecting {MEMBER_SET}", token.text),
                );
                return Err(Stop);
            }
        };
        self.at += 1;
        Ok(name)
    }

    fn arguments(&mut self) -> Result<Vec<Expr>, Stop> {
        self.closers.push(Closer::Args);
        let mut args = Vec::new();
        if self.is_punct(")") {
            self.at += 1;
            self.closers.pop();
            return Ok(args);
        }
        args.push(self.expression(0, true)?);
        while self.is_punct(",") {
            self.at += 1;
            args.push(self.expression(0, false)?);
        }
        self.close(Closer::Args)?;
        Ok(args)
    }

    /// The closer a group needs, or the error ANTLR gives where it is not.
    fn close(&mut self, closer: Closer) -> Result<(), Stop> {
        let want = closer.text();
        if self.token().tok == Tok::Punct(want) {
            self.at += 1;
            self.closers.pop();
            return Ok(());
        }
        let token = self.token().clone();
        if let Some(previous) = self.previous().filter(|one| matches!(one.tok, Tok::Num(_)))
            && closer != Closer::Brace
        {
            let shown = if token.tok == Tok::Eof {
                String::new()
            } else {
                token.text.clone()
            };
            let message = format!("no viable alternative at input '{}{shown}'", previous.text);
            self.error(&token, message);
            return Err(Stop);
        }
        if self.ahead(1).tok == Tok::Punct(want) && token.tok != Tok::Eof {
            self.error(&token, format!("extraneous input '{}' expecting '{want}'", token.text));
            self.at += 2;
            self.closers.pop();
            return Ok(());
        }
        if token.tok == Tok::Eof && self.closers.len() == 1 {
            self.error(&token, format!("missing '{want}' at '<EOF>'"));
            self.closers.pop();
            return Ok(());
        }
        self.error(&token, format!("mismatched input '{}' expecting '{want}'", token.text));
        Err(Stop)
    }

    fn prefix(&mut self, first_argument: bool) -> Result<Expr, Stop> {
        let token = self.token().clone();
        match &token.tok {
            Tok::Punct(p) if *p == "+" || *p == "-" => {
                self.at += 1;
                let operand = self.expression(11, false)?;
                Ok(Expr::Polarity(
                    if *p == "+" { "+" } else { "-" },
                    Box::new(operand),
                    token.line,
                    token.column + 1,
                ))
            }
            Tok::Punct("(") => {
                self.at += 1;
                self.closers.push(Closer::Paren);
                let inner = self.expression(0, false)?;
                self.close(Closer::Paren)?;
                Ok(inner)
            }
            Tok::Punct("{") => {
                self.at += 1;
                self.closers.push(Closer::Brace);
                self.close(Closer::Brace)?;
                Ok(Expr::Empty)
            }
            Tok::Punct("%") => {
                self.at += 1;
                let name = match &self.token().tok {
                    Tok::Ident(word) | Tok::Delimited(word) | Tok::Str(word) => word.clone(),
                    _ => {
                        let at = self.token().clone();
                        self.error(&at, format!("mismatched input '{}' expecting {EXPR_SET}", at.text));
                        return Err(Stop);
                    }
                };
                self.at += 1;
                Ok(Expr::Constant(name))
            }
            Tok::Var(var) => {
                self.at += 1;
                Ok(match *var {
                    "$this" => Expr::This,
                    "$index" => Expr::Index,
                    _ => Expr::Total,
                })
            }
            Tok::Str(text) => {
                self.at += 1;
                Ok(Expr::Str(text.clone()))
            }
            Tok::Date(text) => {
                self.at += 1;
                Ok(Expr::Date(text.clone()))
            }
            Tok::DateTime(text) => {
                self.at += 1;
                Ok(Expr::DateTime(text.clone()))
            }
            Tok::Time(text) => {
                self.at += 1;
                Ok(Expr::Time(text.clone()))
            }
            Tok::Long(text) => {
                self.at += 1;
                Ok(Expr::Number {
                    text: text.clone(),
                    decimal: false,
                })
            }
            Tok::Num(text) => {
                self.at += 1;
                let decimal = text.contains('.');
                match &self.token().tok {
                    Tok::Str(unit) => {
                        let unit = unit.clone();
                        self.at += 1;
                        Ok(Expr::Quantity {
                            value: text.clone(),
                            unit,
                            calendar: false,
                        })
                    }
                    Tok::Ident(word) if CALENDAR_UNITS.contains(&word.as_str()) => {
                        let unit = word.clone();
                        self.at += 1;
                        Ok(Expr::Quantity {
                            value: text.clone(),
                            unit,
                            calendar: true,
                        })
                    }
                    _ => Ok(Expr::Number {
                        text: text.clone(),
                        decimal,
                    }),
                }
            }
            Tok::Ident(word) if word == "true" || word == "false" => {
                self.at += 1;
                Ok(Expr::Bool(word == "true"))
            }
            Tok::Ident(word) if !OPERATOR_WORDS.contains(&word.as_str()) => {
                let name = word.clone();
                self.at += 1;
                self.named(name)
            }
            Tok::Delimited(word) => {
                let name = word.clone();
                self.at += 1;
                self.named(name)
            }
            _ => {
                let set = if first_argument { EXPR_OR_CLOSE_SET } else { EXPR_SET };
                if starts_expression(&self.ahead(1).tok) && token.tok != Tok::Eof {
                    self.error(&token, format!("extraneous input '{}' expecting {set}", token.text));
                    self.at += 1;
                    return self.prefix(false);
                }
                self.error(&token, format!("mismatched input '{}' expecting {set}", token.text));
                Err(Stop)
            }
        }
    }

    fn named(&mut self, name: String) -> Result<Expr, Stop> {
        if self.is_punct("(") {
            self.at += 1;
            let args = self.arguments()?;
            return Ok(Expr::Function(name, args));
        }
        Ok(Expr::Name(name))
    }
}

/// An expression read into its tree, or the words the engine refuses it with: ANTLR's, each with
/// its line and column, one to a line.
pub fn parse(expression: &str) -> Result<Expr, String> {
    let mut lexer = Lexer {
        chars: expression.chars().collect(),
        at: 0,
        line: 1,
        column: 0,
        errors: Vec::new(),
    };
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next();
        let end = token.tok == Tok::Eof;
        tokens.push(token);
        if end {
            break;
        }
    }
    let mut parser = Parser {
        tokens,
        at: 0,
        errors: Vec::new(),
        closers: Vec::new(),
    };
    let parsed = parser.expression(0, false);
    if parsed.is_ok() && parser.token().tok != Tok::Eof {
        let token = parser.token().clone();
        let message = match parser.previous() {
            Some(previous)
                if matches!(previous.tok, Tok::Num(_)) && matches!(token.tok, Tok::Punct(")" | "]" | "," | "}")) =>
            {
                format!("no viable alternative at input '{}{}'", previous.text, token.text)
            }
            _ => format!("extraneous input '{}' expecting <EOF>", token.text),
        };
        parser.error(&token, message);
    }
    let mut errors = lexer.errors;
    errors.extend(parser.errors);
    match parsed {
        Ok(expr) if errors.is_empty() => Ok(expr),
        _ => Err(errors.join("\n")),
    }
}

// --- Values -----------------------------------------------------------------------------------------

/// One item of a collection: an element of the resource, with the type the model gives it, or a
/// value the expression made.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Node {
        value: Value,
        kind: Option<String>,
    },
    Bool(bool),
    Integer(f64),
    Decimal(f64),
    Str(String),
    Date(String),
    DateTime(String),
    Time(String),
    /// A quantity, its unit as the engine keeps it: a calendar word, or a UCUM code in quotes.
    Quantity {
        value: f64,
        unit: String,
    },
}

/// What an item is when an operator or a function reads it.
#[derive(Clone, Debug, PartialEq)]
enum Prim {
    Bool(bool),
    Number(f64, bool),
    Str(String),
    Date(Moment),
    DateTime(Moment),
    Time(Moment),
    Quantity(f64, String),
    Object(Value),
}

/// A date, a date and time, or a time, as the parts it was written with.
#[derive(Clone, Debug, PartialEq)]
struct Moment {
    text: String,
    /// Year, month, day, hour, minute, second, millisecond; as many as were written.
    parts: Vec<i64>,
    /// The offset from UTC in minutes, when one was written.
    zone: Option<i64>,
    /// Whether it is a time of day alone.
    time: bool,
}

fn read_moment(text: &str, time: bool) -> Option<Moment> {
    let mut parts = Vec::new();
    let mut zone = None;
    let bytes = text.as_bytes();
    let mut at = 0;
    let num = |from: usize, len: usize| -> Option<i64> {
        let part = text.get(from..from + len)?;
        part.bytes().all(|b| b.is_ascii_digit()).then(|| part.parse().ok())?
    };
    if !time {
        parts.push(num(0, 4)?);
        at = 4;
        if bytes.get(at) == Some(&b'-') {
            parts.push(num(at + 1, 2)?);
            at += 3;
            if bytes.get(at) == Some(&b'-') {
                parts.push(num(at + 1, 2)?);
                at += 3;
            }
        }
        if bytes.get(at) == Some(&b'T') {
            at += 1;
        }
    }
    if at < bytes.len() && bytes[at].is_ascii_digit() {
        parts.push(num(at, 2)?);
        at += 2;
        if bytes.get(at) == Some(&b':') {
            parts.push(num(at + 1, 2)?);
            at += 3;
            if bytes.get(at) == Some(&b':') {
                parts.push(num(at + 1, 2)?);
                at += 3;
                if bytes.get(at) == Some(&b'.') {
                    let start = at + 1;
                    let mut end = start;
                    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                        end += 1;
                    }
                    let padded: String = text[start..end].chars().chain("000".chars()).take(3).collect();
                    parts.push(padded.parse().ok()?);
                    at = end;
                }
            }
        }
    }
    let rest = &text[at..];
    if rest == "Z" {
        zone = Some(0);
    } else if rest.len() == 6 && (rest.starts_with('+') || rest.starts_with('-')) {
        let hours: i64 = rest[1..3].parse().ok()?;
        let minutes: i64 = rest[4..6].parse().ok()?;
        let sign = if rest.starts_with('-') { -1 } else { 1 };
        zone = Some(sign * (hours * 60 + minutes));
    } else if !rest.is_empty() {
        return None;
    }
    Some(Moment {
        text: text.to_owned(),
        parts,
        zone,
        time,
    })
}

impl Moment {
    /// Its precision as the engine counts it: 0 for a year, up to 6 for milliseconds (a time from
    /// 0 for its hour).
    fn precision(&self) -> usize {
        self.parts.len().saturating_sub(1)
    }

    /// The instant it starts at, in UTC milliseconds, its zone taken into account (none is UTC, the
    /// zone the pages run in).
    fn instant(&self) -> i64 {
        let get = |at: usize, default: i64| self.parts.get(at).copied().unwrap_or(default);
        let (year, month, day, hour, minute, second, milli) = if self.time {
            (1970, 1, 1, get(0, 0), get(1, 0), get(2, 0), get(3, 0))
        } else {
            (
                get(0, 0),
                get(1, 1),
                get(2, 1),
                get(3, 0),
                get(4, 0),
                get(5, 0),
                get(6, 0),
            )
        };
        days_from_civil(year, month, day) * 86_400_000 + hour * 3_600_000 + minute * 60_000 + second * 1000 + milli
            - self.zone.unwrap_or(0) * 60_000
    }

    /// Its parts after its zone is taken into account, in UTC.
    fn utc_parts(&self) -> Vec<i64> {
        if self.zone.unwrap_or(0) == 0 {
            return self.parts.clone();
        }
        let ms = self.instant();
        let days = ms.div_euclid(86_400_000);
        let rest = ms.rem_euclid(86_400_000);
        let (year, month, day) = civil_from_days(days);
        let all = if self.time {
            vec![rest / 3_600_000, rest / 60_000 % 60, rest / 1000 % 60, rest % 1000]
        } else {
            vec![
                year,
                month,
                day,
                rest / 3_600_000,
                rest / 60_000 % 60,
                rest / 1000 % 60,
                rest % 1000,
            ]
        };
        all.into_iter().take(self.parts.len()).collect()
    }
}

/// How two moments compare: their order, or none when they agree as far as both go but one goes
/// further.
fn compare_moments(a: &Moment, b: &Moment) -> Option<std::cmp::Ordering> {
    let (pa, pb) = (a.utc_parts(), b.utc_parts());
    // Seconds and milliseconds are one precision.
    let level = |parts: &[i64], time: bool| {
        let n = parts.len();
        if (!time && n == 7) || (time && n == 4) {
            n - 1
        } else {
            n
        }
    };
    let (la, lb) = (level(&pa, a.time), level(&pb, b.time));
    let shared = la.min(lb);
    for at in 0..shared {
        let (x, y) = (pa[at], pb[at]);
        let seconds = if a.time { 2 } else { 5 };
        let (x, y) = if at == seconds {
            (
                x * 1000 + pa.get(at + 1).copied().unwrap_or(0),
                y * 1000 + pb.get(at + 1).copied().unwrap_or(0),
            )
        } else {
            (x, y)
        };
        if x != y {
            return Some(x.cmp(&y));
        }
    }
    if la != lb {
        return None;
    }
    Some(std::cmp::Ordering::Equal)
}

fn pad2(n: i64) -> String {
    format!("{n:02}")
}

/// The type a primitive element has in the R4 model, from a choice element's suffix.
fn suffix_type(suffix: &str) -> String {
    match suffix {
        "DateTime" | "String" | "Boolean" | "Integer" | "Time" | "Instant" | "Date" | "Decimal" => {
            let mut chars = suffix.chars();
            let first = chars.next().unwrap_or_default().to_ascii_lowercase();
            format!("{first}{}", chars.as_str())
        }
        other => other.to_owned(),
    }
}

/// The R4 choice elements on the types the server keeps, with the types each may hold, in R4's
/// order.
fn choices(kind: &str, name: &str) -> Option<&'static [&'static str]> {
    const VALUE: &[&str] = &[
        "Quantity",
        "CodeableConcept",
        "String",
        "Boolean",
        "Integer",
        "Range",
        "Ratio",
        "SampledData",
        "Time",
        "DateTime",
        "Period",
    ];
    const ONSET: &[&str] = &["DateTime", "Age", "Period", "Range", "String"];
    Some(match (kind, name) {
        ("Observation" | "Component", "value") => VALUE,
        ("Observation", "effective") => &["DateTime", "Period", "Timing", "Instant"],
        ("Condition", "onset" | "abatement") => ONSET,
        ("Patient", "deceased") => &["Boolean", "DateTime"],
        ("Patient", "multipleBirth") => &["Boolean", "Integer"],
        ("Library", "subject") => &["CodeableConcept", "Reference"],
        _ => return None,
    })
}

/// The type the model gives an element of a type.
fn child_kind(kind: &str, name: &str) -> Option<String> {
    let bundle = match (kind, name) {
        ("Bundle", "type") => Some("code"),
        ("Bundle", "total") => Some("unsignedInt"),
        ("Bundle", "link") => Some("BundleLink"),
        ("Bundle", "entry") => Some("BundleEntry"),
        ("BundleLink", "relation") => Some("string"),
        ("BundleLink" | "BundleEntry", "url" | "fullUrl") => Some("uri"),
        ("BundleEntry", "search") => Some("BundleSearch"),
        ("BundleSearch", "mode") => Some("code"),
        ("Bundle", "id") => Some("id"),
        ("Bundle", "meta") => Some("Meta"),
        _ => None,
    };
    if let Some(found) = bundle {
        return Some(found.to_owned());
    }
    let elements = type_def(kind).map(|def| def.elements).or_else(|| datatype(kind))?;
    elements
        .iter()
        .find(|one| one.name == name)
        .map(|one| one.kind.to_owned())
}

/// The type of an element of an object, as the model gives it: a resource's own `id` is a
/// FHIRPath String, not a FHIR type.
fn element_kind(fields: &serde_json::Map<String, Value>, kind: &str, name: &str) -> Option<String> {
    if name == "id" && fields.contains_key("resourceType") {
        return None;
    }
    child_kind(kind, name)
}

/// The parents a type has in the R4 model, itself first.
fn lineage(kind: &str) -> Vec<String> {
    let mut out = vec![kind.to_owned()];
    match kind {
        "code" | "id" | "markdown" => out.push("string".to_owned()),
        "url" | "canonical" => out.push("uri".to_owned()),
        "positiveInt" | "unsignedInt" => out.push("integer".to_owned()),
        "Age" => out.push("Quantity".to_owned()),
        _ => {}
    }
    if type_def(kind).is_some() || kind == "Bundle" || kind == "OperationOutcome" {
        if kind != "Bundle" {
            out.push("DomainResource".to_owned());
        }
        out.push("Resource".to_owned());
    }
    out
}

const SYSTEM_TYPES: [&str; 8] = [
    "Boolean", "String", "Integer", "Decimal", "Date", "DateTime", "Time", "Quantity",
];

/// Whether an item is of the type named, as `is` reads a name: `System.` and `FHIR.` names, and
/// an unqualified one as FHIR's where FHIR has it.
fn is_type(item: &Item, name: &str) -> bool {
    let (namespace, bare) = match name.split_once('.') {
        Some((space, rest)) => (Some(space), rest),
        None => (None, name),
    };
    let system = |bare: &str| -> bool {
        matches!(
            (item, bare),
            (Item::Bool(_), "Boolean")
                | (Item::Str(_), "String")
                | (Item::Integer(_), "Integer")
                | (Item::Decimal(_), "Decimal")
                | (Item::Date(_), "Date")
                | (Item::DateTime(_), "DateTime")
                | (Item::Time(_), "Time")
                | (Item::Quantity { .. }, "Quantity")
        )
    };
    let fhir = |bare: &str| -> bool {
        match item {
            Item::Node { kind: Some(kind), .. } => lineage(kind).iter().any(|one| one == bare),
            _ => false,
        }
    };
    match namespace {
        Some("System") => system(bare),
        Some("FHIR") => fhir(bare),
        _ => {
            if matches!(item, Item::Node { kind: Some(_), .. }) {
                fhir(bare)
            } else {
                SYSTEM_TYPES.contains(&bare) && system(bare)
            }
        }
    }
}

fn prim(item: &Item) -> Prim {
    match item {
        Item::Bool(flag) => Prim::Bool(*flag),
        Item::Integer(value) => Prim::Number(*value, true),
        Item::Decimal(value) => Prim::Number(*value, false),
        Item::Str(text) => Prim::Str(text.clone()),
        Item::Date(text) => read_moment(text, false).map_or(Prim::Str(text.clone()), Prim::Date),
        Item::DateTime(text) => read_moment(text, false).map_or(Prim::Str(text.clone()), Prim::DateTime),
        Item::Time(text) => read_moment(text, true).map_or(Prim::Str(text.clone()), Prim::Time),
        Item::Quantity { value, unit } => Prim::Quantity(*value, unit.clone()),
        Item::Node { value, kind } => match (value, kind.as_deref()) {
            (Value::String(text), Some("date")) => read_moment(text, false).map_or(Prim::Str(text.clone()), Prim::Date),
            (Value::String(text), Some("dateTime" | "instant")) => {
                read_moment(text, false).map_or(Prim::Str(text.clone()), Prim::DateTime)
            }
            (Value::String(text), Some("time")) => read_moment(text, true).map_or(Prim::Str(text.clone()), Prim::Time),
            (Value::String(text), _) => Prim::Str(text.clone()),
            (Value::Bool(flag), _) => Prim::Bool(*flag),
            (Value::Number(figure), kind) => {
                let value = figure.as_f64().unwrap_or(0.0);
                let integer = match kind {
                    Some("decimal") => false,
                    Some("integer" | "positiveInt" | "unsignedInt") => true,
                    _ => value.fract() == 0.0,
                };
                Prim::Number(value, integer)
            }
            (Value::Object(fields), Some(kind)) if lineage(kind).iter().any(|one| one == "Quantity") => {
                let ucum = fields.get("system").and_then(Value::as_str) == Some("http://unitsofmeasure.org");
                match (
                    fields.get("value").and_then(Value::as_f64),
                    fields.get("code").and_then(Value::as_str),
                ) {
                    (Some(amount), Some(code)) if ucum => Prim::Quantity(amount, format!("'{code}'")),
                    _ => Prim::Object(value.clone()),
                }
            }
            (other, _) => Prim::Object(other.clone()),
        },
    }
}

/// An item as `evaluate` hands it back: an element as its JSON, a value made as JSON or text.
pub fn item_json(item: &Item) -> Value {
    match item {
        Item::Node { value, .. } => value.clone(),
        Item::Bool(flag) => Value::Bool(*flag),
        Item::Integer(value) | Item::Decimal(value) => number(*value),
        Item::Str(text) | Item::Date(text) | Item::DateTime(text) | Item::Time(text) => Value::String(text.clone()),
        Item::Quantity { value, unit } => Value::String(format!("{} {unit}", number_to_string(*value))),
    }
}

fn collection_json(items: &[Item]) -> String {
    stringify(&Value::Array(items.iter().map(item_json).collect()))
}

// --- Evaluating ---------------------------------------------------------------------------------------

/// What an evaluation reads besides the expression: the variables, the focus's `$this`, `$index`
/// and `$total`, and the clock for `today()` and `now()`.
struct Scope<'a> {
    vars: &'a BTreeMap<String, Vec<Item>>,
    this: Option<Item>,
    index: Option<usize>,
    total: Option<Vec<Item>>,
    now_ms: i64,
}

type Answer = Result<Vec<Item>, String>;

fn node_of(value: &Value, kind: Option<String>) -> Item {
    let kind = value
        .get("resourceType")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or(kind);
    Item::Node {
        value: value.clone(),
        kind,
    }
}

fn push_values(out: &mut Vec<Item>, value: &Value, kind: Option<String>) {
    match value {
        Value::Array(items) => {
            for one in items.iter().filter(|one| !one.is_null()) {
                out.push(node_of(one, kind.clone()));
            }
        }
        Value::Null => {}
        other => out.push(node_of(other, kind)),
    }
}

/// The elements named `name` of each item, its choice elements read by their base name.
fn navigate(items: &[Item], name: &str) -> Vec<Item> {
    let mut out = Vec::new();
    for item in items {
        let Item::Node {
            value: Value::Object(fields),
            kind,
        } = item
        else {
            continue;
        };
        let kind_name = kind.as_deref().unwrap_or("");
        if name.starts_with(|c: char| c.is_ascii_uppercase())
            && fields.get("resourceType").and_then(Value::as_str) == Some(name)
        {
            out.push(item.clone());
            continue;
        }
        if let Some(child) = fields.get(name) {
            push_values(&mut out, child, element_kind(fields, kind_name, name));
            continue;
        }
        if let Some(types) = choices(kind_name, name) {
            for suffix in types {
                if let Some(child) = fields.get(&format!("{name}{suffix}")) {
                    push_values(&mut out, child, Some(suffix_type(suffix)));
                }
            }
        }
    }
    out
}

fn children_of(items: &[Item]) -> Vec<Item> {
    let mut out = Vec::new();
    for item in items {
        if let Item::Node {
            value: Value::Object(fields),
            kind,
        } = item
        {
            let kind_name = kind.as_deref().unwrap_or("");
            for (key, child) in fields {
                if key == "resourceType" || key.starts_with('_') {
                    continue;
                }
                push_values(
                    &mut out,
                    child,
                    element_kind(fields, kind_name, key).or_else(|| choice_kind(kind_name, key)),
                );
            }
        }
    }
    out
}

/// The type of a choice element named in full, such as `valueQuantity`.
fn choice_kind(kind: &str, key: &str) -> Option<String> {
    for base in [
        "value",
        "effective",
        "onset",
        "abatement",
        "deceased",
        "multipleBirth",
        "subject",
    ] {
        if let Some(suffix) = key.strip_prefix(base)
            && choices(kind, base).is_some_and(|types| types.contains(&suffix))
        {
            return Some(suffix_type(suffix));
        }
    }
    None
}

fn round_max(value: f64) -> f64 {
    round(value / 1e-8) * 1e-8
}

/// Deep equality of two items, as `=` reads two singletons: none where it cannot say.
fn equal_items(a: &Item, b: &Item) -> Option<bool> {
    match (prim(a), prim(b)) {
        (Prim::Number(x, _), Prim::Number(y, _)) => Some(round_max(x) == round_max(y)),
        (Prim::Str(x), Prim::Str(y)) => Some(x == y),
        (Prim::Bool(x), Prim::Bool(y)) => Some(x == y),
        (Prim::Date(x) | Prim::DateTime(x), Prim::Date(y) | Prim::DateTime(y)) | (Prim::Time(x), Prim::Time(y)) => {
            compare_moments(&x, &y).map(|order| order == std::cmp::Ordering::Equal)
        }
        (Prim::Quantity(x, ux), Prim::Quantity(y, uy)) => {
            if ux == uy {
                Some(round_max(x) == round_max(y))
            } else {
                None
            }
        }
        (Prim::Object(x), Prim::Object(y)) => Some(x == y),
        _ => Some(false),
    }
}

fn equal_lists(a: &[Item], b: &[Item]) -> Option<bool> {
    if a.len() != b.len() {
        return Some(false);
    }
    let mut unsure = false;
    for (x, y) in a.iter().zip(b) {
        match equal_items(x, y) {
            Some(false) => return Some(false),
            None => unsure = true,
            Some(true) => {}
        }
    }
    if unsure { None } else { Some(true) }
}

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn equivalent_items(a: &Item, b: &Item) -> bool {
    match (prim(a), prim(b)) {
        (Prim::Str(x), Prim::Str(y)) => normalized(&x) == normalized(&y),
        (Prim::Number(x, _), Prim::Number(y, _)) => {
            let places = |v: f64| {
                let text = number_to_string(v);
                text.split_once('.').map_or(0, |(_, f)| f.len())
            };
            let digits = places(x).min(places(y)) as i32;
            let scale = 10f64.powi(digits);
            round(x * scale) / scale == round(y * scale) / scale
        }
        (Prim::Date(x) | Prim::DateTime(x), Prim::Date(y) | Prim::DateTime(y)) | (Prim::Time(x), Prim::Time(y)) => {
            compare_moments(&x, &y) == Some(std::cmp::Ordering::Equal)
        }
        _ => equal_items(a, b) == Some(true),
    }
}

fn equivalent_lists(a: &[Item], b: &[Item]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut left: Vec<&Item> = b.iter().collect();
    for x in a {
        match left.iter().position(|y| equivalent_items(x, y)) {
            Some(at) => {
                left.remove(at);
            }
            None => return false,
        }
    }
    true
}

fn singleton_error(items: &[Item], kind: &str) -> String {
    format!(
        "Unexpected collection{}; expected singleton of type {kind}",
        collection_json(items)
    )
}

/// A collection read as one Boolean: empty is none, one Boolean is itself, one other item is true.
fn as_bool(items: &[Item]) -> Result<Option<bool>, String> {
    match items {
        [] => Ok(None),
        [one] => Ok(Some(match prim(one) {
            Prim::Bool(flag) => flag,
            _ => true,
        })),
        _ => Err(singleton_error(items, "Boolean")),
    }
}

fn as_string(items: &[Item]) -> Result<Option<String>, String> {
    match items {
        [] => Ok(None),
        [Item::Str(text)]
        | [
            Item::Node {
                value: Value::String(text),
                ..
            },
        ] => Ok(Some(text.clone())),
        [_] => Err(format!("Expected string, but got: {}", collection_json(items))),
        _ => Err(singleton_error(items, "String")),
    }
}

fn as_number(items: &[Item], kind: &str) -> Result<Option<(f64, bool)>, String> {
    match items {
        [] => Ok(None),
        [one] => match prim(one) {
            Prim::Number(value, integer) if kind != "Integer" || value.fract() == 0.0 => Ok(Some((value, integer))),
            _ => Err(format!(
                "Expected {}, but got: {}",
                kind.to_lowercase(),
                collection_json(items)
            )),
        },
        _ => Err(singleton_error(items, kind)),
    }
}

fn boolean(flag: Option<bool>) -> Vec<Item> {
    flag.map(|one| vec![Item::Bool(one)]).unwrap_or_default()
}

fn make_number(value: f64, integer: bool) -> Item {
    if integer {
        Item::Integer(value)
    } else {
        Item::Decimal(value)
    }
}

fn distinct(items: Vec<Item>) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    for item in items {
        if !out.iter().any(|one| equal_items(one, &item) == Some(true)) {
            out.push(item);
        }
    }
    out
}

/// A moment written back at a precision, as the engine writes the result of date arithmetic: a
/// date's parts alone, a time with its zone as the page's (UTC, written `-00:00`).
fn write_moment(ms: i64, precision: usize, time: bool) -> String {
    let days = ms.div_euclid(86_400_000);
    let rest = ms.rem_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second, milli) = (rest / 3_600_000, rest / 60_000 % 60, rest / 1000 % 60, rest % 1000);
    let clock = |precision: usize| {
        let mut text = pad2(hour);
        if precision > 0 {
            text.push(':');
            text.push_str(&pad2(minute));
            if precision > 1 {
                text.push(':');
                text.push_str(&pad2(second));
                if milli != 0 {
                    text.push_str(&format!(".{milli:03}"));
                }
            }
        }
        text
    };
    if time {
        return clock(precision);
    }
    let mut text = year.to_string();
    if precision > 0 {
        text.push('-');
        text.push_str(&pad2(month));
        if precision > 1 {
            text.push('-');
            text.push_str(&pad2(day));
            if precision > 2 {
                text.push('T');
                text.push_str(&clock(precision - 3));
                text.push_str("-00:00");
            }
        }
    }
    text
}

/// A calendar unit's name, without its plural, from a word or a UCUM code.
fn time_unit(unit: &str) -> Option<&'static str> {
    Some(match unit.trim_matches('\'') {
        "year" | "years" => "year",
        "month" | "months" => "month",
        "week" | "weeks" | "wk" => "week",
        "day" | "days" | "d" => "day",
        "hour" | "hours" | "h" => "hour",
        "minute" | "minutes" | "min" => "minute",
        "second" | "seconds" | "s" => "second",
        "millisecond" | "milliseconds" | "ms" => "millisecond",
        _ => return None,
    })
}

/// Adds months as date-fns does: the day held to the new month's last.
fn add_months(ms: i64, months: i64) -> i64 {
    let days = ms.div_euclid(86_400_000);
    let rest = ms.rem_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    let total = year * 12 + (month - 1) + months;
    let (year, month) = (total.div_euclid(12), total.rem_euclid(12) + 1);
    let day = day.min(days_in_month(year, month));
    days_from_civil(year, month, day) * 86_400_000 + rest
}

const DURATION_UNITS: &str = "years,months,weeks,days,hours,minutes,seconds,milliseconds,year,month,week,day,hour,minute,second,millisecond,'wk','d','h','min','s','ms'";

fn duration_error() -> String {
    format!(
        "For date/time arithmetic, the unit of the quantity must be one of the following time-based units: {DURATION_UNITS}"
    )
}

fn moment_plus(moment: &Moment, value: f64, unit: &str, kind: &str) -> Result<Item, String> {
    let Some(unit) = time_unit(unit) else {
        return Err(duration_error());
    };
    let value = value.trunc() as i64;
    let precision = moment.precision();
    let start = moment.instant();
    let ms = match unit {
        "year" => add_months(start, value * 12),
        "month" => add_months(start, value),
        "week" => start + value * 7 * 86_400_000,
        "day" => start + value * 86_400_000,
        "hour" => start + value * 3_600_000,
        "minute" => start + value * 60_000,
        "second" => start + value * 1000,
        _ => start + value,
    };
    let text = if moment.time {
        write_moment(ms, precision, true)
    } else {
        write_moment(ms, precision.min(if kind == "Date" { 2 } else { 6 }), false)
    };
    Ok(match kind {
        "Date" => Item::Date(text),
        "Time" => Item::Time(text),
        _ => Item::DateTime(text),
    })
}

fn arithmetic(op: &str, left: &[Item], right: &[Item]) -> Answer {
    if left.is_empty() || right.is_empty() {
        return Ok(Vec::new());
    }
    if left.len() > 1 {
        return Err(singleton_error(left, "Any"));
    }
    if right.len() > 1 {
        return Err(singleton_error(right, "Any"));
    }
    let (a, b) = (prim(&left[0]), prim(&right[0]));
    match (op, a, b) {
        ("+", Prim::Str(x), Prim::Str(y)) => Ok(vec![Item::Str(x + &y)]),
        (_, Prim::Number(x, xi), Prim::Number(y, yi)) => {
            let integer = xi && yi;
            Ok(match op {
                "+" => vec![make_number(x + y, integer)],
                "-" => vec![make_number(x - y, integer)],
                "*" => vec![make_number(x * y, integer)],
                "/" => {
                    if y == 0.0 {
                        Vec::new()
                    } else {
                        vec![Item::Decimal(x / y)]
                    }
                }
                "div" => {
                    if y == 0.0 {
                        Vec::new()
                    } else {
                        vec![make_number((x / y).trunc(), integer)]
                    }
                }
                _ => {
                    if y == 0.0 {
                        Vec::new()
                    } else {
                        vec![make_number(x % y, integer)]
                    }
                }
            })
        }
        ("+" | "-", Prim::Date(m), Prim::Quantity(value, unit)) => {
            moment_plus(&m, if op == "-" { -value } else { value }, &unit, "Date").map(|one| vec![one])
        }
        ("+" | "-", Prim::DateTime(m), Prim::Quantity(value, unit)) => {
            moment_plus(&m, if op == "-" { -value } else { value }, &unit, "DateTime").map(|one| vec![one])
        }
        ("+" | "-", Prim::Time(m), Prim::Quantity(value, unit)) => {
            moment_plus(&m, if op == "-" { -value } else { value }, &unit, "Time").map(|one| vec![one])
        }
        ("+" | "-", Prim::Date(_) | Prim::DateTime(_) | Prim::Time(_), _) => Err(duration_error()),
        ("+" | "-", Prim::Quantity(x, ux), Prim::Quantity(y, uy)) if ux == uy => Ok(vec![Item::Quantity {
            value: if op == "+" { x + y } else { x - y },
            unit: ux,
        }]),
        ("*", Prim::Quantity(x, unit), Prim::Number(y, _)) | ("*", Prim::Number(y, _), Prim::Quantity(x, unit)) => {
            Ok(vec![Item::Quantity { value: x * y, unit }])
        }
        ("/", Prim::Quantity(x, unit), Prim::Number(y, _)) => {
            if y == 0.0 {
                Ok(Vec::new())
            } else {
                Ok(vec![Item::Quantity { value: x / y, unit }])
            }
        }
        ("+" | "-" | "*" | "/", Prim::Str(x), _) if op != "+" => Err(format!("Cannot convert {x} to a number")),
        ("+" | "-" | "*" | "/" | "div" | "mod", Prim::Str(x), Prim::Number(..)) => {
            Err(format!("Cannot convert {x} to a number"))
        }
        _ => Ok(Vec::new()),
    }
}

fn order(op: &str, left: &[Item], right: &[Item]) -> Answer {
    if left.is_empty() || right.is_empty() {
        return Ok(Vec::new());
    }
    if left.len() > 1 {
        return Err(singleton_error(left, "Any"));
    }
    if right.len() > 1 {
        return Err(singleton_error(right, "Any"));
    }
    let (a, b) = (prim(&left[0]), prim(&right[0]));
    let class = |p: &Prim| match p {
        Prim::Str(_) => "String",
        Prim::Bool(_) => "Boolean",
        Prim::Object(Value::Array(_)) => "Array",
        Prim::Object(_) => "Object",
        _ => "FP_Type",
    };
    let shown = |p: &Prim| match p {
        Prim::Str(text) => text.clone(),
        Prim::Bool(flag) => flag.to_string(),
        Prim::Number(value, _) => number_to_string(*value),
        Prim::Date(m) | Prim::DateTime(m) | Prim::Time(m) => m.text.clone(),
        Prim::Quantity(value, unit) => format!("{} {unit}", number_to_string(*value)),
        Prim::Object(_) => "[object Object]".to_owned(),
    };
    if class(&a) != class(&b) {
        return Err(format!(
            "InequalityExpression: Type of \"{}\" ({}) did not match type of \"{}\" ({})",
            shown(&a),
            class(&a),
            shown(&b),
            class(&b)
        ));
    }
    let ordering = match (&a, &b) {
        (Prim::Number(x, _), Prim::Number(y, _)) => round_max(*x).partial_cmp(&round_max(*y)),
        (Prim::Str(x), Prim::Str(y)) => Some(x.encode_utf16().cmp(y.encode_utf16())),
        (Prim::Bool(x), Prim::Bool(y)) => Some(x.cmp(y)),
        (Prim::Object(_), Prim::Object(_)) => Some(std::cmp::Ordering::Equal),
        (Prim::Date(x) | Prim::DateTime(x), Prim::Date(y) | Prim::DateTime(y)) | (Prim::Time(x), Prim::Time(y)) => {
            compare_moments(x, y)
        }
        (Prim::Date(_) | Prim::DateTime(_), _) => {
            return Err("Invalid comparison of a DateTime with something else".to_owned());
        }
        (Prim::Time(_), _) => return Err("Invalid comparison of a time with something else".to_owned()),
        (Prim::Quantity(x, ux), Prim::Quantity(y, uy)) if ux == uy => round_max(*x).partial_cmp(&round_max(*y)),
        _ => None,
    };
    // Two objects compare as JavaScript compares them: as the same text.
    if matches!(a, Prim::Object(_)) && (op == "<" || op == ">") {
        return Ok(vec![Item::Bool(false)]);
    }
    let Some(ordering) = ordering else {
        return Ok(Vec::new());
    };
    use std::cmp::Ordering::{Greater, Less};
    Ok(vec![Item::Bool(match op {
        "<" => ordering == Less,
        "<=" => ordering != Greater,
        ">" => ordering == Greater,
        _ => ordering != Less,
    })])
}

/// An expression's value against a resource or a Bundle, `%resource` and `%context` bound to it
/// and the other variables as given; `now_ms` is the clock `today()` and `now()` read.
pub fn evaluate(
    expr: &Expr,
    subject: &Value,
    vars: &BTreeMap<String, Vec<Item>>,
    now_ms: i64,
) -> Result<Vec<Item>, String> {
    let root = node_of(subject, None);
    let mut all = vars.clone();
    all.insert("resource".to_owned(), vec![root.clone()]);
    all.insert("context".to_owned(), vec![root.clone()]);
    all.insert(
        "ucum".to_owned(),
        vec![Item::Str("http://unitsofmeasure.org".to_owned())],
    );
    let scope = Scope {
        vars: &all,
        this: Some(root.clone()),
        index: None,
        total: None,
        now_ms,
    };
    eval(expr, &[root], &scope)
}

fn with_this<'a>(scope: &Scope<'a>, item: &Item, index: usize) -> Scope<'a> {
    Scope {
        vars: scope.vars,
        this: Some(item.clone()),
        index: Some(index),
        total: scope.total.clone(),
        now_ms: scope.now_ms,
    }
}

fn eval(expr: &Expr, focus: &[Item], scope: &Scope) -> Answer {
    match expr {
        Expr::Empty => Ok(Vec::new()),
        Expr::Bool(flag) => Ok(vec![Item::Bool(*flag)]),
        Expr::Str(text) => Ok(vec![Item::Str(text.clone())]),
        Expr::Number { text, decimal } => {
            let value: f64 = text.parse().unwrap_or(f64::NAN);
            Ok(vec![if *decimal {
                Item::Decimal(value)
            } else {
                Item::Integer(value)
            }])
        }
        Expr::Date(text) => {
            let valid = read_moment(text, false).is_some_and(|m| valid_moment(&m));
            if valid {
                Ok(vec![Item::Date(text.clone())])
            } else {
                Err(format!("Invalid date literal: @{text}"))
            }
        }
        Expr::DateTime(text) => {
            let valid = read_moment(text, false).is_some_and(|m| valid_moment(&m));
            if valid {
                Ok(vec![Item::DateTime(text.clone())])
            } else {
                Err(format!("Invalid dateTime literal: @{text}"))
            }
        }
        Expr::Time(text) => Ok(vec![Item::Time(text.clone())]),
        Expr::Quantity { value, unit, calendar } => Ok(vec![Item::Quantity {
            value: value.parse().unwrap_or(f64::NAN),
            unit: if *calendar { unit.clone() } else { format!("'{unit}'") },
        }]),
        Expr::Name(name) => Ok(navigate(focus, name)),
        Expr::Function(name, args) => call(name, args, focus, scope),
        Expr::This => Ok(scope.this.iter().cloned().collect()),
        Expr::Index => Ok(scope
            .index
            .map(|index| vec![Item::Integer(index as f64)])
            .unwrap_or_default()),
        Expr::Total => Ok(scope.total.clone().unwrap_or_default()),
        Expr::Constant(name) => scope
            .vars
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Attempting to access an undefined environment variable: {name}")),
        Expr::Member(base, name) => {
            let items = eval(base, focus, scope)?;
            Ok(navigate(&items, name))
        }
        Expr::Call(base, name, args) => {
            let items = eval(base, focus, scope)?;
            call(name, args, &items, scope)
        }
        Expr::Indexer(base, index) => {
            let items = eval(base, focus, scope)?;
            let at = eval(index, focus, scope)?;
            let Some((at, _)) = as_number(&at, "Integer")? else {
                return Ok(Vec::new());
            };
            Ok(if at >= 0.0 {
                items.get(at as usize).cloned().into_iter().collect()
            } else {
                Vec::new()
            })
        }
        Expr::Polarity(sign, operand, line, column) => {
            let items = eval(operand, focus, scope)?;
            let [one] = items.as_slice() else {
                return Err(format!(
                    "Unary {sign} can only be applied to an individual number or Quantity."
                ));
            };
            match prim(one) {
                Prim::Number(value, integer) if *sign == "-" => Ok(vec![make_number(-value, integer)]),
                Prim::Quantity(value, unit) if *sign == "-" => Ok(vec![Item::Quantity { value: -value, unit }]),
                Prim::Number(..) | Prim::Quantity(..) | Prim::Date(_) | Prim::DateTime(_) | Prim::Time(_) => Ok(items),
                _ => Err(format!(
                    "Unary {sign} can only be applied to a number or Quantity. (at {line}:{column})"
                )),
            }
        }
        Expr::Type(op, base, kind) => {
            let items = eval(base, focus, scope)?;
            match items.as_slice() {
                [] => Ok(Vec::new()),
                [one] => {
                    let matches = is_type(one, kind);
                    Ok(if *op == "is" {
                        vec![Item::Bool(matches)]
                    } else if matches {
                        vec![one.clone()]
                    } else {
                        Vec::new()
                    })
                }
                _ => Err(format!(
                    "Expected singleton on left side of \"{op}\", got {}",
                    collection_json(&items)
                )),
            }
        }
        Expr::Binary(op, left, right) => binary(op, left, right, focus, scope),
    }
}

fn valid_moment(moment: &Moment) -> bool {
    let p = &moment.parts;
    if p.len() >= 2 && !(1..=12).contains(&p[1]) {
        return false;
    }
    if p.len() >= 3 && (p[2] < 1 || p[2] > days_in_month(p[0], p[1])) {
        return false;
    }
    if p.len() >= 4 && p[3] > 23 {
        return false;
    }
    if p.len() >= 5 && p[4] > 59 {
        return false;
    }
    if p.len() >= 6 && p[5] > 59 {
        return false;
    }
    true
}

fn binary(op: &str, left: &Expr, right: &Expr, focus: &[Item], scope: &Scope) -> Answer {
    let a = eval(left, focus, scope)?;
    let b = eval(right, focus, scope)?;
    match op {
        "and" | "or" | "xor" | "implies" => {
            let (x, y) = (as_bool(&a)?, as_bool(&b)?);
            Ok(boolean(match op {
                "and" => match (x, y) {
                    (Some(false), _) | (_, Some(false)) => Some(false),
                    (Some(true), Some(true)) => Some(true),
                    _ => None,
                },
                "or" => match (x, y) {
                    (Some(true), _) | (_, Some(true)) => Some(true),
                    (Some(false), Some(false)) => Some(false),
                    _ => None,
                },
                "xor" => match (x, y) {
                    (Some(p), Some(q)) => Some(p != q),
                    _ => None,
                },
                _ => match (x, y) {
                    (Some(false), _) => Some(true),
                    (Some(true), y) => y,
                    (None, Some(true)) => Some(true),
                    _ => None,
                },
            }))
        }
        "=" | "!=" => {
            if a.is_empty() || b.is_empty() {
                return Ok(Vec::new());
            }
            let same = equal_lists(&a, &b);
            Ok(boolean(if op == "=" { same } else { same.map(|one| !one) }))
        }
        "~" | "!~" => {
            let same = if a.is_empty() && b.is_empty() {
                true
            } else if a.is_empty() || b.is_empty() {
                false
            } else {
                equivalent_lists(&a, &b)
            };
            Ok(vec![Item::Bool(if op == "~" { same } else { !same })])
        }
        "<" | "<=" | ">" | ">=" => order(op, &a, &b),
        "|" => {
            let mut all = a;
            all.extend(b);
            Ok(distinct(all))
        }
        "in" | "contains" => {
            let (one, many) = if op == "in" { (&a, &b) } else { (&b, &a) };
            match one.as_slice() {
                [] => Ok(Vec::new()),
                [item] => Ok(vec![Item::Bool(
                    many.iter().any(|other| equal_items(item, other) == Some(true)),
                )]),
                _ => Err(format!(
                    "Expected singleton on left side of \"{op}\", got {}",
                    collection_json(one)
                )),
            }
        }
        "&" => {
            let text = |items: &[Item]| -> Result<String, String> { Ok(as_string(items)?.unwrap_or_default()) };
            Ok(vec![Item::Str(text(&a)? + &text(&b)?)])
        }
        _ => arithmetic(op, &a, &b),
    }
}

/// Each item's value of an argument evaluated with the item as `$this`.
fn per_item(arg: &Expr, items: &[Item], scope: &Scope) -> Result<Vec<Vec<Item>>, String> {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| eval(arg, std::slice::from_ref(item), &with_this(scope, item, index)))
        .collect()
}

/// An argument evaluated once, against the focus the function was called from.
fn once(arg: &Expr, scope: &Scope) -> Answer {
    let focus: Vec<Item> = scope.this.iter().cloned().collect();
    eval(arg, &focus, scope)
}

fn arity(name: &str, args: &[Expr], range: std::ops::RangeInclusive<usize>) -> Result<(), String> {
    if range.contains(&args.len()) {
        Ok(())
    } else {
        Err(format!("{name} wrong arity: got {}", args.len()))
    }
}

fn call(name: &str, args: &[Expr], items: &[Item], scope: &Scope) -> Answer {
    let arg = |at: usize| once(&args[at], scope);
    match name {
        "where" => {
            arity(name, args, 1..=1)?;
            let mut out = Vec::new();
            for (item, found) in items.iter().zip(per_item(&args[0], items, scope)?) {
                if as_bool(&found)? == Some(true) {
                    out.push(item.clone());
                }
            }
            Ok(out)
        }
        "select" => {
            arity(name, args, 1..=1)?;
            Ok(per_item(&args[0], items, scope)?.into_iter().flatten().collect())
        }
        "exists" => {
            if args.is_empty() {
                return Ok(vec![Item::Bool(!items.is_empty())]);
            }
            let found = call("where", args, items, scope)?;
            Ok(vec![Item::Bool(!found.is_empty())])
        }
        "empty" => Ok(vec![Item::Bool(items.is_empty())]),
        "all" => {
            arity(name, args, 1..=1)?;
            for found in per_item(&args[0], items, scope)? {
                if as_bool(&found)? != Some(true) {
                    return Ok(vec![Item::Bool(false)]);
                }
            }
            Ok(vec![Item::Bool(true)])
        }
        "count" => Ok(vec![Item::Integer(items.len() as f64)]),
        "first" => Ok(items.first().cloned().into_iter().collect()),
        "last" => Ok(items.last().cloned().into_iter().collect()),
        "tail" => Ok(items.iter().skip(1).cloned().collect()),
        "skip" | "take" => {
            arity(name, args, 1..=1)?;
            let n = as_number(&arg(0)?, "Integer")?.map_or(0.0, |(n, _)| n);
            let n = n.max(0.0) as usize;
            Ok(if name == "skip" {
                items.iter().skip(n).cloned().collect()
            } else {
                items.iter().take(n).cloned().collect()
            })
        }
        "single" => match items {
            [] | [_] => Ok(items.to_vec()),
            _ => Err(format!("Expected single, got {}", collection_json(items))),
        },
        "distinct" => Ok(distinct(items.to_vec())),
        "isDistinct" => Ok(vec![Item::Bool(distinct(items.to_vec()).len() == items.len())]),
        "ofType" => {
            let kind = type_argument(args)?;
            Ok(items.iter().filter(|one| is_type(one, &kind)).cloned().collect())
        }
        "is" | "as" => {
            let kind = type_argument(args)?;
            match items {
                [] => Ok(Vec::new()),
                [one] => {
                    let matches = is_type(one, &kind);
                    Ok(if name == "is" {
                        vec![Item::Bool(matches)]
                    } else if matches {
                        vec![one.clone()]
                    } else {
                        Vec::new()
                    })
                }
                _ => Err(format!(
                    "Expected singleton on left side of \"{name}\", got {}",
                    collection_json(items)
                )),
            }
        }
        "iif" => {
            arity(name, args, 2..=3)?;
            let criterion = as_bool(&arg(0)?)?;
            if criterion == Some(true) {
                arg(1)
            } else if args.len() == 3 {
                arg(2)
            } else {
                Ok(Vec::new())
            }
        }
        "not" => Ok(boolean(as_bool(items)?.map(|flag| !flag))),
        "allTrue" | "anyTrue" | "allFalse" | "anyFalse" => {
            let flags = items
                .iter()
                .map(|one| match prim(one) {
                    Prim::Bool(flag) => Ok(flag),
                    _ => Err(singleton_error(std::slice::from_ref(one), "Boolean")),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(vec![Item::Bool(match name {
                "allTrue" => flags.iter().all(|one| *one),
                "anyTrue" => flags.iter().any(|one| *one),
                "allFalse" => flags.iter().all(|one| !*one),
                _ => flags.iter().any(|one| !*one),
            })])
        }
        "contains" | "startsWith" | "endsWith" | "matches" | "indexOf" => {
            arity(name, args, 1..=1)?;
            let Some(text) = as_string(items)? else {
                return Ok(Vec::new());
            };
            let Some(other) = as_string(&arg(0)?)? else {
                return Ok(Vec::new());
            };
            Ok(vec![match name {
                "contains" => Item::Bool(text.contains(&other)),
                "startsWith" => Item::Bool(text.starts_with(&other)),
                "endsWith" => Item::Bool(text.ends_with(&other)),
                "indexOf" => Item::Integer(
                    text.find(&other)
                        .map_or(-1.0, |at| text[..at].encode_utf16().count() as f64),
                ),
                _ => {
                    let Some(pattern) = Regex::new(&other, false) else {
                        return Err(format!("Invalid regular expression: /{other}/"));
                    };
                    Item::Bool(pattern.is_match(&text).unwrap_or(false))
                }
            }])
        }
        "replace" => {
            arity(name, args, 2..=2)?;
            let Some(text) = as_string(items)? else {
                return Ok(Vec::new());
            };
            let (Some(from), Some(to)) = (as_string(&arg(0)?)?, as_string(&arg(1)?)?) else {
                return Ok(Vec::new());
            };
            Ok(vec![Item::Str(if from.is_empty() {
                let mut out = to.clone();
                for c in text.chars() {
                    out.push(c);
                    out.push_str(&to);
                }
                out
            } else {
                text.replace(&from, &to)
            })])
        }
        "substring" => {
            arity(name, args, 1..=2)?;
            let Some(text) = as_string(items)? else {
                return Ok(Vec::new());
            };
            let Some((start, _)) = as_number(&arg(0)?, "Integer")? else {
                return Ok(Vec::new());
            };
            let units: Vec<u16> = text.encode_utf16().collect();
            if start < 0.0 || start as usize >= units.len() {
                return Ok(Vec::new());
            }
            let start = start as usize;
            let end = if args.len() == 2 {
                match as_number(&arg(1)?, "Integer")? {
                    Some((length, _)) => (start + length.max(0.0) as usize).min(units.len()),
                    None => units.len(),
                }
            } else {
                units.len()
            };
            Ok(vec![Item::Str(String::from_utf16_lossy(&units[start..end]))])
        }
        "upper" | "lower" | "length" | "trim" | "toChars" => {
            let Some(text) = as_string(items)? else {
                return Ok(Vec::new());
            };
            Ok(match name {
                "upper" => vec![Item::Str(text.to_uppercase())],
                "lower" => vec![Item::Str(text.to_lowercase())],
                "length" => vec![Item::Integer(text.encode_utf16().count() as f64)],
                "trim" => vec![Item::Str(crate::js::trim(&text).to_owned())],
                _ => text.chars().map(|c| Item::Str(c.to_string())).collect(),
            })
        }
        "join" => {
            let separator = if args.is_empty() {
                String::new()
            } else {
                as_string(&arg(0)?)?.unwrap_or_default()
            };
            let texts = items
                .iter()
                .map(|one| match prim(one) {
                    Prim::Str(text) => Ok(text),
                    _ => Err("Join requires a collection of strings.".to_owned()),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(vec![Item::Str(texts.join(&separator))])
        }
        "split" => {
            arity(name, args, 1..=1)?;
            let Some(text) = as_string(items)? else {
                return Ok(Vec::new());
            };
            let separator = as_string(&arg(0)?)?.unwrap_or_default();
            Ok(text
                .split(separator.as_str())
                .map(|one| Item::Str(one.to_owned()))
                .collect())
        }
        "toString" => match items {
            [] => Ok(Vec::new()),
            [one] => Ok(vec![Item::Str(match prim(one) {
                Prim::Str(text) => text,
                Prim::Number(value, _) => number_to_string(value),
                Prim::Bool(flag) => flag.to_string(),
                Prim::Date(m) | Prim::DateTime(m) | Prim::Time(m) => m.text,
                Prim::Quantity(value, unit) => format!("{} {unit}", number_to_string(value)),
                Prim::Object(_) => return Ok(Vec::new()),
            })]),
            _ => Err(singleton_error(items, "Any")),
        },
        "toInteger" | "toDecimal" => match items {
            [] => Ok(Vec::new()),
            [one] => Ok(match prim(one) {
                Prim::Number(value, integer) => {
                    if name == "toInteger" {
                        if integer {
                            vec![Item::Integer(value)]
                        } else {
                            Vec::new()
                        }
                    } else {
                        vec![Item::Decimal(value)]
                    }
                }
                Prim::Bool(flag) => vec![make_number(f64::from(u8::from(flag)), name == "toInteger")],
                Prim::Str(text) => {
                    let integer = !text.is_empty()
                        && text.trim_start_matches(['+', '-']).chars().all(|c| c.is_ascii_digit())
                        && text.trim_start_matches(['+', '-']).len() + 1 >= text.len();
                    let decimal = integer || {
                        let body = text.trim_start_matches(['+', '-']);
                        text.len() <= body.len() + 1
                            && body.split_once('.').is_some_and(|(w, f)| {
                                !w.is_empty() && !f.is_empty() && w.chars().chain(f.chars()).all(|c| c.is_ascii_digit())
                            })
                    };
                    let value: f64 = text.parse().unwrap_or(f64::NAN);
                    if name == "toInteger" && integer {
                        vec![Item::Integer(value)]
                    } else if name == "toDecimal" && decimal {
                        vec![Item::Decimal(value)]
                    } else {
                        Vec::new()
                    }
                }
                _ => Vec::new(),
            }),
            _ => Err(singleton_error(items, "Any")),
        },
        "toDate" | "toDateTime" => match items {
            [] => Ok(Vec::new()),
            [one] => Ok(match prim(one) {
                Prim::Str(text) | Prim::Date(Moment { text, .. }) | Prim::DateTime(Moment { text, .. }) => {
                    match read_moment(&text, false).filter(valid_moment) {
                        Some(m) if name == "toDate" => {
                            vec![Item::Date(m.text.chars().take(10).collect())]
                        }
                        Some(m) => vec![Item::DateTime(m.text)],
                        None => Vec::new(),
                    }
                }
                _ => Vec::new(),
            }),
            _ => Err(singleton_error(items, "Any")),
        },
        "sum" | "avg" => {
            if items.is_empty() {
                return Ok(Vec::new());
            }
            let mut total = vec![items[0].clone()];
            for item in &items[1..] {
                total = arithmetic("+", &total, std::slice::from_ref(item))?;
                if total.is_empty() {
                    return Ok(Vec::new());
                }
            }
            if name == "sum" {
                return Ok(total);
            }
            arithmetic("/", &total, &[Item::Integer(items.len() as f64)])
        }
        "min" | "max" => {
            if items.is_empty() {
                return Ok(Vec::new());
            }
            let mut best = items[0].clone();
            for item in &items[1..] {
                let better = order(
                    if name == "min" { "<" } else { ">" },
                    std::slice::from_ref(item),
                    std::slice::from_ref(&best),
                )?;
                if better == vec![Item::Bool(true)] {
                    best = item.clone();
                }
            }
            Ok(vec![best])
        }
        "round" | "abs" | "ceiling" | "floor" | "truncate" | "sqrt" | "exp" | "ln" => {
            let Some((value, integer)) = (match items {
                [one] => match prim(one) {
                    Prim::Quantity(value, unit) if name == "abs" => {
                        return Ok(vec![Item::Quantity {
                            value: value.abs(),
                            unit,
                        }]);
                    }
                    Prim::Number(value, integer) => Some((value, integer)),
                    _ => {
                        return Err(format!(
                            "Expected a number or a Quantity, but got {}",
                            stringify(&item_json(one))
                        ));
                    }
                },
                _ => as_number(items, "Number")?,
            }) else {
                return Ok(Vec::new());
            };
            Ok(vec![match name {
                "round" => {
                    let digits = if args.is_empty() {
                        None
                    } else {
                        as_number(&arg(0)?, "Integer")?.map(|(n, _)| n)
                    };
                    match digits {
                        None => make_number(round(value), integer),
                        Some(digits) => {
                            let scale = 10f64.powf(digits);
                            make_number(round(value * scale) / scale, integer)
                        }
                    }
                }
                "abs" => make_number(value.abs(), integer),
                "ceiling" => Item::Integer(value.ceil()),
                "floor" => Item::Integer(value.floor()),
                "truncate" => Item::Integer(value.trunc()),
                "sqrt" => {
                    if value < 0.0 {
                        return Ok(Vec::new());
                    }
                    Item::Decimal(value.sqrt())
                }
                "exp" => Item::Decimal(value.exp()),
                _ => Item::Decimal(value.ln()),
            }])
        }
        "today" => {
            let days = scope.now_ms.div_euclid(86_400_000);
            Ok(vec![Item::Date(iso_day(days))])
        }
        "now" => Ok(vec![Item::DateTime(write_moment(scope.now_ms, 6, false))]),
        "timeOfDay" => Ok(vec![Item::Time(write_moment(scope.now_ms, 2, true))]),
        "children" => Ok(children_of(items)),
        "descendants" => {
            let mut out = Vec::new();
            let mut level = children_of(items);
            while !level.is_empty() {
                out.extend(level.iter().cloned());
                level = children_of(&level);
            }
            Ok(out)
        }
        "repeat" => {
            arity(name, args, 1..=1)?;
            let mut out: Vec<Item> = Vec::new();
            let mut level = items.to_vec();
            loop {
                let found: Vec<Item> = per_item(&args[0], &level, scope)?.into_iter().flatten().collect();
                if found.is_empty() {
                    break;
                }
                let mut fresh = Vec::new();
                for item in found {
                    if !out.iter().any(|one| equal_items(one, &item) == Some(true)) {
                        out.push(item.clone());
                        fresh.push(item);
                    }
                }
                if fresh.is_empty() {
                    break;
                }
                level = fresh;
            }
            Ok(out)
        }
        "aggregate" => {
            arity(name, args, 1..=2)?;
            let mut total = if args.len() == 2 { arg(1)? } else { Vec::new() };
            for (index, item) in items.iter().enumerate() {
                let inner = Scope {
                    vars: scope.vars,
                    this: Some(item.clone()),
                    index: Some(index),
                    total: Some(total.clone()),
                    now_ms: scope.now_ms,
                };
                total = eval(&args[0], std::slice::from_ref(item), &inner)?;
            }
            Ok(total)
        }
        "union" | "combine" | "intersect" | "exclude" | "subsetOf" | "supersetOf" => {
            arity(name, args, 1..=1)?;
            let other = arg(0)?;
            Ok(match name {
                "union" => {
                    let mut all = items.to_vec();
                    all.extend(other);
                    distinct(all)
                }
                "combine" => {
                    let mut all = items.to_vec();
                    all.extend(other);
                    all
                }
                "intersect" => distinct(
                    items
                        .iter()
                        .filter(|one| other.iter().any(|two| equal_items(one, two) == Some(true)))
                        .cloned()
                        .collect(),
                ),
                "exclude" => items
                    .iter()
                    .filter(|one| !other.iter().any(|two| equal_items(one, two) == Some(true)))
                    .cloned()
                    .collect(),
                "subsetOf" => vec![Item::Bool(
                    items
                        .iter()
                        .all(|one| other.iter().any(|two| equal_items(one, two) == Some(true))),
                )],
                _ => vec![Item::Bool(
                    other
                        .iter()
                        .all(|one| items.iter().any(|two| equal_items(one, two) == Some(true))),
                )],
            })
        }
        "hasValue" => Ok(vec![Item::Bool(matches!(
            items,
            [Item::Node {
                value: Value::String(_) | Value::Number(_) | Value::Bool(_),
                ..
            }]
        ))]),
        "extension" => {
            arity(name, args, 1..=1)?;
            let url = as_string(&arg(0)?)?.unwrap_or_default();
            let found = navigate(items, "extension");
            Ok(found
                .into_iter()
                .filter(|one| match one {
                    Item::Node { value, .. } => value.get("url").and_then(Value::as_str) == Some(url.as_str()),
                    _ => false,
                })
                .collect())
        }
        "trace" => Ok(items.to_vec()),
        _ => Err(format!("Not implemented: {name}")),
    }
}

/// The type a type function names, as written.
fn type_argument(args: &[Expr]) -> Result<String, String> {
    fn written(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Name(name) => Some(name.clone()),
            Expr::Member(base, name) => Some(format!("{}.{name}", written(base)?)),
            _ => None,
        }
    }
    args.first()
        .and_then(written)
        .ok_or_else(|| "Expected a type name".to_owned())
}

/// The variables the workbench binds beside `%resource` and `%context`.
pub fn variables(pairs: &[(&str, &str)]) -> BTreeMap<String, Vec<Item>> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), vec![Item::Str((*value).to_owned())]))
        .collect()
}

/// A collection's items as `evaluate` hands them back.
pub fn to_json(items: &[Item]) -> Vec<Value> {
    items.iter().map(item_json).collect()
}
