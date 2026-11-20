//! JSON as the web reads and writes it: `JSON.parse`, with JavaScriptCore's words for what it
//! cannot read and an object's keys in the order JavaScript keeps them, `JSON.stringify` with and
//! without its indent, and `Object.keys`.

use serde_json::{Map, Value};

use super::number::number_to_string;

/// A value `JSON.parse` gives: its objects' keys in the order JavaScript keeps them, the ones that
/// read as array indexes first, rising, then the rest as they first came.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The value under a key of an object, or under an index of an array, as `value[key]` reads it.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries.iter().find(|(name, _)| name == key).map(|(_, value)| value),
            Json::Array(items) => index_of_key(key).and_then(|at| items.get(at as usize)),
            _ => None,
        }
    }

    /// Its keys, as `Object.keys` lists them.
    pub fn keys(&self) -> Vec<String> {
        match self {
            Json::Object(entries) => entries.iter().map(|(name, _)| name.clone()).collect(),
            Json::Array(items) => (0..items.len()).map(|at| at.to_string()).collect(),
            Json::Text(text) => (0..text.encode_utf16().count()).map(|at| at.to_string()).collect(),
            _ => Vec::new(),
        }
    }

    /// Whether it is an object or an array, as `typeof value === 'object'` and not null.
    pub fn is_object(&self) -> bool {
        matches!(self, Json::Object(_) | Json::Array(_))
    }

    /// It as `JSON.stringify` writes it, without spaces.
    pub fn stringify(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Json::Number(value) => {
                if value.is_finite() {
                    out.push_str(&number_to_string(*value));
                } else {
                    out.push_str("null");
                }
            }
            Json::Text(text) => out.push_str(&quote(text)),
            Json::Array(items) => {
                out.push('[');
                for (at, item) in items.iter().enumerate() {
                    if at > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Object(entries) => {
                out.push('{');
                for (at, (key, value)) in entries.iter().enumerate() {
                    if at > 0 {
                        out.push(',');
                    }
                    out.push_str(&quote(key));
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// The index a key names where it is one, as an array index is: a whole number below 2^32 - 1
/// written without leading zeros.
fn index_of_key(key: &str) -> Option<u32> {
    if key.is_empty() || (key.len() > 1 && key.starts_with('0')) || !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    key.parse::<u64>()
        .ok()
        .filter(|value| *value < 4_294_967_295)
        .map(|value| value as u32)
}

/// An object's entries in the order JavaScript keeps them.
fn ordered(entries: Vec<(String, Json)>) -> Vec<(String, Json)> {
    let (mut indexes, rest): (Vec<_>, Vec<_>) = entries.into_iter().partition(|(key, _)| index_of_key(key).is_some());
    indexes.sort_by_key(|(key, _)| index_of_key(key));
    indexes.extend(rest);
    indexes
}

/// A text read as `JSON.parse` reads it, or the message bun's `SyntaxError` gives for what it
/// cannot read.
pub fn json_parse(text: &str) -> Result<Json, String> {
    let mut parser = JsonParser {
        chars: text.chars().collect(),
        at: 0,
    };
    let value = parser.value()?;
    parser.space();
    if parser.at < parser.chars.len() {
        return Err(parser.fail("Unable to parse JSON string"));
    }
    Ok(value)
}

/// What a JSON document is read into, a token at a time, as JavaScriptCore's parser reads it.
enum Token {
    Open(char),
    Close(char),
    Comma,
    Colon,
    Text(String),
    Number(f64),
    Word(String),
    End,
}

struct JsonParser {
    chars: Vec<char>,
    at: usize,
}

impl JsonParser {
    fn fail(&self, message: &str) -> String {
        format!("JSON Parse error: {message}")
    }

    fn space(&mut self) {
        while self.at < self.chars.len() && matches!(self.chars[self.at], ' ' | '\t' | '\n' | '\r') {
            self.at += 1;
        }
    }

    fn token(&mut self) -> Result<Token, String> {
        self.space();
        let Some(&c) = self.chars.get(self.at) else {
            return Ok(Token::End);
        };
        match c {
            '{' | '[' => {
                self.at += 1;
                Ok(Token::Open(c))
            }
            '}' | ']' => {
                self.at += 1;
                Ok(Token::Close(c))
            }
            ',' => {
                self.at += 1;
                Ok(Token::Comma)
            }
            ':' => {
                self.at += 1;
                Ok(Token::Colon)
            }
            '"' => self.text(),
            '\'' => Err(self.fail("Single quotes (') are not allowed in JSON")),
            '-' | '0'..='9' => self.number(),
            c if c.is_ascii_alphabetic() || c == '_' || c == '$' => {
                let start = self.at;
                while self
                    .chars
                    .get(self.at)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
                {
                    self.at += 1;
                }
                Ok(Token::Word(self.chars[start..self.at].iter().collect()))
            }
            '.' => Err(self.fail("Unexpected token '.'")),
            _ => Err(self.fail(&format!("Unrecognized token '{c}'"))),
        }
    }

    fn number(&mut self) -> Result<Token, String> {
        let start = self.at;
        if self.chars[self.at] == '-' {
            self.at += 1;
            if !self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                return Err(self.fail("Invalid number"));
            }
        }
        if self.chars[self.at] == '0' {
            self.at += 1;
        } else {
            while self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                self.at += 1;
            }
        }
        if self.chars.get(self.at) == Some(&'.') {
            self.at += 1;
            if !self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                return Err(self.fail("Invalid digits after decimal point"));
            }
            while self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                self.at += 1;
            }
        }
        if matches!(self.chars.get(self.at), Some('e' | 'E')) {
            let before = self.at;
            self.at += 1;
            if matches!(self.chars.get(self.at), Some('+' | '-')) {
                self.at += 1;
            }
            if self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                while self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                    self.at += 1;
                }
            } else {
                self.at = before;
            }
        }
        let written: String = self.chars[start..self.at].iter().collect();
        Ok(Token::Number(written.parse::<f64>().unwrap_or(f64::NAN)))
    }

    fn text(&mut self) -> Result<Token, String> {
        self.at += 1;
        let mut out = String::new();
        let mut units: Vec<u16> = Vec::new();
        let flush = |units: &mut Vec<u16>, out: &mut String| {
            if !units.is_empty() {
                out.push_str(&String::from_utf16_lossy(units));
                units.clear();
            }
        };
        loop {
            let Some(&c) = self.chars.get(self.at) else {
                return Err(self.fail("Unterminated string"));
            };
            match c {
                '"' => {
                    self.at += 1;
                    flush(&mut units, &mut out);
                    return Ok(Token::Text(out));
                }
                '\\' => {
                    let Some(&next) = self.chars.get(self.at + 1) else {
                        return Err(self.fail("Unterminated string"));
                    };
                    let plain = match next {
                        '"' => Some('"'),
                        '\\' => Some('\\'),
                        '/' => Some('/'),
                        'b' => Some('\u{8}'),
                        'f' => Some('\u{c}'),
                        'n' => Some('\n'),
                        'r' => Some('\r'),
                        't' => Some('\t'),
                        'u' => None,
                        other => return Err(self.fail(&format!("Invalid escape character {other}"))),
                    };
                    if let Some(plain) = plain {
                        flush(&mut units, &mut out);
                        out.push(plain);
                        self.at += 2;
                        continue;
                    }
                    let hex: String = self.chars[self.at + 2..(self.at + 6).min(self.chars.len())]
                        .iter()
                        .collect();
                    match u16::from_str_radix(&hex, 16) {
                        Ok(unit) if hex.len() == 4 && hex.chars().all(|c| c.is_ascii_hexdigit()) => {
                            units.push(unit);
                            self.at += 6;
                        }
                        _ => {
                            let shown: String = self.chars[self.at..(self.at + 6).min(self.chars.len())]
                                .iter()
                                .collect();
                            return Err(self.fail(&format!("\"{shown}\" is not a valid unicode escape")));
                        }
                    }
                }
                c if (c as u32) < 0x20 => return Err(self.fail("Unterminated string")),
                c => {
                    flush(&mut units, &mut out);
                    out.push(c);
                    self.at += 1;
                }
            }
        }
    }

    /// A value where one must stand.
    fn value(&mut self) -> Result<Json, String> {
        let token = self.token()?;
        self.value_from(token)
    }

    fn value_from(&mut self, token: Token) -> Result<Json, String> {
        match token {
            Token::Open('[') => self.array(),
            Token::Open(_) => self.object(),
            Token::Text(text) => Ok(Json::Text(text)),
            Token::Number(value) => Ok(Json::Number(value)),
            Token::Word(word) => match word.as_str() {
                "true" => Ok(Json::Bool(true)),
                "false" => Ok(Json::Bool(false)),
                "null" => Ok(Json::Null),
                _ => Err(self.fail(&format!("Unexpected identifier \"{word}\""))),
            },
            Token::End => Err(self.fail("Unexpected EOF")),
            Token::Close(c) => Err(self.fail(&format!("Unexpected token '{c}'"))),
            Token::Comma => Err(self.fail("Unexpected token ','")),
            Token::Colon => Err(self.fail("Unexpected token ':'")),
        }
    }

    /// A token after a value, where an error of its own says no more than that the close was
    /// wanted; where a key stands, the error is the lexer's, as JavaScriptCore puts the lexer's
    /// words first (`{'a': 1}` is refused for its single quotes).
    fn after(&mut self) -> Option<Token> {
        self.token().ok()
    }

    fn array(&mut self) -> Result<Json, String> {
        let mut items = Vec::new();
        let first = self.token()?;
        if matches!(first, Token::Close(']')) {
            return Ok(Json::Array(items));
        }
        items.push(self.value_from(first)?);
        loop {
            match self.after() {
                Some(Token::Close(']')) => return Ok(Json::Array(items)),
                Some(Token::Comma) => {
                    let next = self.token()?;
                    if matches!(next, Token::Close(']')) {
                        return Err(self.fail("Unexpected comma at the end of array expression"));
                    }
                    items.push(self.value_from(next)?);
                }
                _ => return Err(self.fail("Expected ']'")),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        let mut entries: Vec<(String, Json)> = Vec::new();
        let first = self.token()?;
        let mut key = match Some(first) {
            Some(Token::Close('}')) => return Ok(Json::Object(entries)),
            Some(Token::Text(key)) => key,
            _ => return Err(self.fail("Expected '}'")),
        };
        loop {
            if !matches!(self.after(), Some(Token::Colon)) {
                return Err(self.fail("Expected ':' before value in object property definition"));
            }
            let value = self.value()?;
            match entries.iter_mut().find(|(name, _)| *name == key) {
                Some(entry) => entry.1 = value,
                None => entries.push((key, value)),
            }
            match self.after() {
                Some(Token::Close('}')) => return Ok(Json::Object(ordered(entries))),
                Some(Token::Comma) => match Some(self.token()?) {
                    Some(Token::Text(next)) => key = next,
                    _ => return Err(self.fail("Property name must be a string literal")),
                },
                _ => return Err(self.fail("Expected '}'")),
            }
        }
    }
}

/// A number as JSON holds it: a whole one as an integer, so it is written without a point.
pub fn json_number(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0 {
        Value::from(value as i64)
    } else {
        serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
    }
}

fn quote_into(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A text as `JSON.stringify` quotes it.
pub fn quote(text: &str) -> String {
    let mut out = String::new();
    quote_into(text, &mut out);
    out
}

fn write_json(value: &Value, indent: usize, depth: usize, out: &mut String) {
    let pad = |out: &mut String, depth: usize| {
        if indent > 0 {
            out.push('\n');
            out.push_str(&" ".repeat(indent * depth));
        }
    };
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(figure) => {
            let value = figure.as_f64().unwrap_or(0.0);
            out.push_str(&if value.is_finite() {
                number_to_string(value)
            } else {
                "null".to_owned()
            });
        }
        Value::String(text) => quote_into(text, out),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                pad(out, depth + 1);
                write_json(item, indent, depth + 1, out);
            }
            pad(out, depth);
            out.push(']');
        }
        Value::Object(fields) => {
            if fields.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (at, (key, item)) in fields.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                pad(out, depth + 1);
                quote_into(key, out);
                out.push(':');
                if indent > 0 {
                    out.push(' ');
                }
                write_json(item, indent, depth + 1, out);
            }
            pad(out, depth);
            out.push('}');
        }
    }
}

/// A value as `JSON.stringify(value)` writes it.
pub fn stringify(value: &Value) -> String {
    let mut out = String::new();
    write_json(value, 0, 0, &mut out);
    out
}

/// A value as `JSON.stringify(value, null, 2)` writes it.
pub fn pretty(value: &Value) -> String {
    let mut out = String::new();
    write_json(value, 2, 0, &mut out);
    out
}

/// The keys `Object.keys` gives of a parsed JSON value: an array's indices, an object's keys with
/// those that read as indices first and in order, nothing for anything else.
pub fn object_keys(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(items) => (0..items.len()).map(|index| index.to_string()).collect(),
        serde_json::Value::Object(map) => {
            let index = |key: &str| -> Option<u32> {
                let number: u32 = key.parse().ok()?;
                (number < u32::MAX && number.to_string() == key).then_some(number)
            };
            let mut numbered: Vec<(u32, String)> = map
                .keys()
                .filter_map(|key| index(key).map(|number| (number, key.clone())))
                .collect();
            numbered.sort();
            let mut keys: Vec<String> = numbered.into_iter().map(|(_, key)| key).collect();
            keys.extend(map.keys().filter(|key| index(key).is_none()).cloned());
            keys
        }
        _ => Vec::new(),
    }
}

impl Json {
    /// It as a `serde_json` value, its keys in the same order and a whole number an integer
    /// (`json_number`).
    pub fn into_value(self) -> Value {
        match self {
            Json::Null => Value::Null,
            Json::Bool(value) => Value::Bool(value),
            Json::Number(value) => json_number(value),
            Json::Text(text) => Value::String(text),
            Json::Array(items) => Value::Array(items.into_iter().map(Json::into_value).collect()),
            Json::Object(entries) => Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, value.into_value()))
                    .collect::<Map<String, Value>>(),
            ),
        }
    }
}

/// A text read as `JSON.parse` reads it into a `serde_json` value (`json_parse`), or the words
/// JavaScriptCore refuses it with.
pub fn parse_json(text: &str) -> Result<Value, String> {
    json_parse(text).map(Json::into_value)
}
