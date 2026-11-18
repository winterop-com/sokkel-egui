//! JSON and YAML documents read into values, with where each value and its key are written, by
//! JSON Pointer, as sokkel-ui's `readDocument` reads CodeMirror's syntax tree; a document that
//! stops being its language says where.

use std::collections::HashMap;

use serde_json::{Map, Number, Value};

/// Where a value is written, and its key when it has one, as byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub from: usize,
    pub to: usize,
    pub key: Option<(usize, usize)>,
}

pub(crate) type Spans = HashMap<String, Span>;

/// A document read: its value (none while empty), where each value is written, where the text
/// stops being the language, and each document of a YAML stream of several.
#[derive(Clone, Debug, Default)]
pub(crate) struct ReadDocument {
    pub value: Option<Value>,
    pub spans: Spans,
    pub broken: Option<(usize, usize)>,
    pub documents: Vec<(Value, Spans)>,
}

/// A path as one string, a JSON Pointer: `/steps/0/id`; the root is the empty string.
pub(crate) fn pointer<S: AsRef<str>>(path: &[S]) -> String {
    path.iter()
        .map(|part| format!("/{}", part.as_ref().replace('~', "~0").replace('/', "~1")))
        .collect()
}

fn child(at: &str, part: &str) -> String {
    format!("{at}/{}", part.replace('~', "~0").replace('/', "~1"))
}

fn broken_at(text: &str, at: usize) -> (usize, usize) {
    let from = at.min(text.len());
    let to = text[from..].chars().next().map_or(from, |c| from + c.len_utf8());
    (from, to)
}

// --- JSON -------------------------------------------------------------------------------------

/// A JSON document read with its spans; what it holds up to a break is still read.
pub(crate) fn read_json(text: &str) -> ReadDocument {
    let mut reader = Json {
        text,
        b: text.as_bytes(),
        pos: 0,
        spans: Spans::new(),
        error: None,
    };
    reader.space();
    let value = if reader.pos < reader.b.len() {
        let value = reader.value(String::new(), None);
        reader.space();
        if reader.pos < reader.b.len() {
            reader.fail();
        }
        value
    } else {
        None
    };
    ReadDocument {
        value,
        broken: reader.error.map(|at| broken_at(text, at)),
        spans: reader.spans,
        documents: Vec::new(),
    }
}

struct Json<'a> {
    text: &'a str,
    b: &'a [u8],
    pos: usize,
    spans: Spans,
    error: Option<usize>,
}

impl Json<'_> {
    fn fail(&mut self) {
        if self.error.is_none() {
            self.error = Some(self.pos);
        }
    }

    fn space(&mut self) {
        while self.pos < self.b.len() && matches!(self.b[self.pos], b' ' | b'\t' | b'\n' | b'\r') {
            self.pos += 1;
        }
    }

    fn value(&mut self, at: String, key: Option<(usize, usize)>) -> Option<Value> {
        self.space();
        let from = self.pos;
        let value = match self.b.get(self.pos) {
            Some(b'{') => {
                self.pos += 1;
                let mut out = Map::new();
                self.space();
                if self.b.get(self.pos) == Some(&b'}') {
                    self.pos += 1;
                } else {
                    loop {
                        self.space();
                        if self.b.get(self.pos) != Some(&b'"') {
                            self.fail();
                            break;
                        }
                        let key_from = self.pos;
                        let Some(name) = self.string() else {
                            self.fail();
                            break;
                        };
                        let key_span = (key_from, self.pos);
                        self.space();
                        if self.b.get(self.pos) != Some(&b':') {
                            self.fail();
                            self.spans.insert(
                                child(&at, &name),
                                Span {
                                    from: key_from,
                                    to: self.pos,
                                    key: Some(key_span),
                                },
                            );
                            out.insert(name, Value::Null);
                            break;
                        }
                        self.pos += 1;
                        let held = self.value(child(&at, &name), Some(key_span));
                        out.insert(name, held.unwrap_or(Value::Null));
                        if self.error.is_some() {
                            break;
                        }
                        self.space();
                        match self.b.get(self.pos) {
                            Some(b',') => self.pos += 1,
                            Some(b'}') => {
                                self.pos += 1;
                                break;
                            }
                            _ => {
                                self.fail();
                                break;
                            }
                        }
                    }
                }
                Some(Value::Object(out))
            }
            Some(b'[') => {
                self.pos += 1;
                let mut out = Vec::new();
                self.space();
                if self.b.get(self.pos) == Some(&b']') {
                    self.pos += 1;
                } else {
                    loop {
                        let held = self.value(child(&at, &out.len().to_string()), None);
                        let Some(held) = held else { break };
                        out.push(held);
                        if self.error.is_some() {
                            break;
                        }
                        self.space();
                        match self.b.get(self.pos) {
                            Some(b',') => self.pos += 1,
                            Some(b']') => {
                                self.pos += 1;
                                break;
                            }
                            _ => {
                                self.fail();
                                break;
                            }
                        }
                    }
                }
                Some(Value::Array(out))
            }
            Some(b'"') => match self.string() {
                Some(s) => Some(Value::String(s)),
                None => {
                    self.fail();
                    None
                }
            },
            Some(b'-' | b'0'..=b'9') => {
                let start = self.pos;
                if self.b[self.pos] == b'-' {
                    self.pos += 1;
                }
                while self.pos < self.b.len()
                    && matches!(self.b[self.pos], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
                {
                    self.pos += 1;
                }
                match serde_json::from_str::<Number>(&self.text[start..self.pos]) {
                    Ok(n) => Some(Value::Number(n)),
                    Err(_) => {
                        self.pos = start;
                        self.fail();
                        None
                    }
                }
            }
            _ => {
                let rest = &self.text[self.pos..];
                let word = |w: &str| rest.starts_with(w) && !rest[w.len()..].starts_with(|c: char| c.is_alphanumeric());
                if word("true") {
                    self.pos += 4;
                    Some(Value::Bool(true))
                } else if word("false") {
                    self.pos += 5;
                    Some(Value::Bool(false))
                } else if word("null") {
                    self.pos += 4;
                    Some(Value::Null)
                } else {
                    self.fail();
                    None
                }
            }
        };
        if value.is_some() {
            self.spans.insert(
                at,
                Span {
                    from,
                    to: self.pos.max(from),
                    key,
                },
            );
        }
        value
    }

    /// A string at the position, read past its closing quote; `None` when it does not close on its line.
    fn string(&mut self) -> Option<String> {
        let start = self.pos;
        let mut i = self.pos + 1;
        while i < self.b.len() && self.b[i] != b'"' {
            if self.b[i] == b'\n' {
                return None;
            }
            i += if self.b[i] == b'\\' { 2 } else { 1 };
        }
        if i >= self.b.len() {
            return None;
        }
        self.pos = i + 1;
        Some(parse_string(&self.text[start..self.pos]))
    }
}

/// A JSON string's text read, or its inside when it does not read.
pub(crate) fn parse_string(source: &str) -> String {
    serde_json::from_str::<String>(source)
        .unwrap_or_else(|_| source.trim_start_matches('"').trim_end_matches('"').to_owned())
}

// --- YAML -------------------------------------------------------------------------------------

/// A plain YAML scalar as the core schema reads it: null, a boolean, a number, else the string.
pub(crate) fn yaml_scalar(source: &str) -> Value {
    let plain = source.split('\n').map(str::trim).collect::<Vec<_>>().join(" ");
    let p = plain.as_str();
    match p {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let unsigned = p.strip_prefix(['-', '+']).unwrap_or(p);
    if digits(unsigned) {
        if let Ok(n) = p.trim_start_matches('+').parse::<i64>() {
            return Value::Number(n.into());
        }
        if let Some(n) = p.parse::<f64>().ok().and_then(Number::from_f64) {
            return Value::Number(n);
        }
    }
    if let Some(oct) = p.strip_prefix("0o")
        && let Ok(n) = i64::from_str_radix(oct, 8)
    {
        return Value::Number(n.into());
    }
    if let Some(hex) = p.strip_prefix("0x")
        && let Ok(n) = i64::from_str_radix(hex, 16)
    {
        return Value::Number(n.into());
    }
    let float = {
        let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
            Some(e) => (&unsigned[..e], Some(&unsigned[e + 1..])),
            None => (unsigned, None),
        };
        let mantissa_ok = match mantissa.split_once('.') {
            Some((a, b)) => {
                (digits(a) || a.is_empty()) && (digits(b) || b.is_empty()) && !(a.is_empty() && b.is_empty())
            }
            None => digits(mantissa),
        };
        let exponent_ok = exponent.is_none_or(|e| digits(e.strip_prefix(['-', '+']).unwrap_or(e)));
        mantissa_ok && exponent_ok
    };
    if float && let Some(n) = p.parse::<f64>().ok().and_then(Number::from_f64) {
        return Value::Number(n);
    }
    Value::String(plain)
}

fn yaml_quoted(source: &str) -> String {
    if let Some(inner) = source.strip_prefix('\'') {
        return inner.strip_suffix('\'').unwrap_or(inner).replace("''", "'");
    }
    let joined: String = source
        .split('\n')
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                l.to_owned()
            } else {
                format!(" {}", l.trim_start())
            }
        })
        .collect();
    serde_json::from_str::<String>(&joined).unwrap_or_else(|_| {
        let inner = source.strip_prefix('"').unwrap_or(source);
        inner.strip_suffix('"').unwrap_or(inner).to_owned()
    })
}

#[derive(Clone, Copy, Debug)]
struct Line {
    /// Where the line starts in the document.
    start: usize,
    /// Where its content starts: past its indent, or past a list item's dash on a line re-read.
    content: usize,
    /// Where its content ends, before a comment and trailing spaces.
    end: usize,
    /// Its indent: the column its content starts at.
    indent: usize,
}

impl Line {
    fn blank(&self) -> bool {
        self.content >= self.end
    }
}

/// A YAML document or stream read with its spans; each document of a stream is read on its own.
pub(crate) fn read_yaml(text: &str) -> ReadDocument {
    let mut all: Vec<Line> = Vec::new();
    let mut offset = 0;
    let mut error = None;
    for raw in text.split('\n') {
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        if raw[indent..].starts_with('\t') && error.is_none() && !raw.trim().is_empty() {
            error = Some(offset + indent);
        }
        let comment = super::highlight::yaml_comment(raw, indent).unwrap_or(raw.len());
        let end = raw[..comment].trim_end().len().max(indent);
        all.push(Line {
            start: offset,
            content: offset + indent,
            end: offset + end,
            indent,
        });
        offset += raw.len() + 1;
    }
    // the documents of a stream, split at `---` and ended by `...`
    let mut documents: Vec<Vec<Line>> = vec![Vec::new()];
    let mut directives = true;
    for line in all {
        let content = &text[line.content..line.end];
        if line.indent == 0 && (content == "---" || content.starts_with("--- ")) {
            if !documents.last().is_some_and(|d| d.iter().all(Line::blank)) || !directives {
                documents.push(Vec::new());
            }
            directives = false;
            if content.len() > 3 {
                let mut rest = line;
                rest.content += 4;
                rest.content += text[rest.content..rest.end].len() - text[rest.content..rest.end].trim_start().len();
                rest.indent = rest.content - rest.start;
                if let Some(d) = documents.last_mut() {
                    d.push(rest);
                }
            }
            continue;
        }
        if line.indent == 0 && content == "..." {
            documents.push(Vec::new());
            continue;
        }
        if line.indent == 0 && content.starts_with('%') {
            continue;
        }
        directives = false;
        if let Some(d) = documents.last_mut() {
            d.push(line);
        }
    }
    let mut read = ReadDocument::default();
    let mut first = true;
    let mut values = Vec::new();
    for lines in documents {
        if lines.iter().all(Line::blank) && !first {
            continue;
        }
        let mut parser = Yaml {
            text,
            lines,
            at: 0,
            spans: Spans::new(),
            anchors: HashMap::new(),
            error: None,
        };
        let value = parser.node(0, String::new(), None);
        parser.skip_blank();
        if parser.at < parser.lines.len() {
            let at = parser.lines[parser.at].content;
            parser.fail(at);
        }
        if read.broken.is_none()
            && let Some(at) = error.take().or(parser.error)
        {
            read.broken = Some(broken_at(text, at));
        }
        if first {
            read.value = Some(value.clone());
            read.spans = parser.spans.clone();
            first = false;
        }
        values.push((value, parser.spans));
    }
    if read.broken.is_none()
        && let Some(at) = error
    {
        read.broken = Some(broken_at(text, at));
    }
    if values.len() > 1 {
        read.documents = values;
    }
    read
}

struct Yaml<'a> {
    text: &'a str,
    lines: Vec<Line>,
    at: usize,
    spans: Spans,
    anchors: HashMap<String, Value>,
    error: Option<usize>,
}

impl Yaml<'_> {
    fn fail(&mut self, at: usize) {
        if self.error.is_none() {
            self.error = Some(at);
        }
    }

    fn skip_blank(&mut self) {
        while self.at < self.lines.len() && self.lines[self.at].blank() {
            self.at += 1;
        }
    }

    fn content(&self, line: &Line) -> &str {
        &self.text[line.content..line.end]
    }

    /// A node at a deeper indent than `min`: a mapping, a list, or a scalar; null when nothing is there.
    fn node(&mut self, min: usize, at: String, key: Option<(usize, usize)>) -> Value {
        self.skip_blank();
        let Some(line) = self.lines.get(self.at).copied() else {
            return self.null(at, key);
        };
        if line.indent < min {
            return self.null(at, key);
        }
        let content = self.content(&line);
        if content == "-" || content.starts_with("- ") {
            return self.sequence(line.indent, at, key);
        }
        if key_end(content).is_some() {
            return self.mapping(line.indent, at, key);
        }
        // a scalar, a flow collection or an alias standing alone on its line
        let (value, from, to) = self.inline_value(line.content, line.indent.saturating_sub(1), &at);
        self.spans.insert(at, Span { from, to, key });
        value
    }

    fn null(&mut self, at: String, key: Option<(usize, usize)>) -> Value {
        if let Some((from, to)) = key {
            self.spans.insert(at, Span { from, to, key });
        }
        Value::Null
    }

    fn sequence(&mut self, indent: usize, at: String, key: Option<(usize, usize)>) -> Value {
        let mut items = Vec::new();
        let from = self.lines[self.at].content;
        let mut to = from;
        loop {
            self.skip_blank();
            let Some(line) = self.lines.get(self.at).copied() else {
                break;
            };
            let content = self.content(&line);
            if line.indent != indent || !(content == "-" || content.starts_with("- ")) {
                if line.indent > indent {
                    self.fail(line.content);
                }
                break;
            }
            let path = child(&at, &items.len().to_string());
            let after = &content[1..];
            let gap = after.len() - after.trim_start().len();
            let value = if after.trim().is_empty() {
                self.at += 1;
                let value = self.node(indent + 1, path.clone(), None);
                if !self.spans.contains_key(&path) {
                    self.spans.insert(
                        path.clone(),
                        Span {
                            from: line.content,
                            to: line.content + 1,
                            key: None,
                        },
                    );
                }
                value
            } else {
                // the item's content re-read as a line of its own at its column
                let column = indent + 1 + gap;
                self.lines[self.at].content = line.content + 1 + gap;
                self.lines[self.at].indent = column;
                self.node(column, path.clone(), None)
            };
            if let Some(span) = self.spans.get(&path) {
                to = to.max(span.to);
            }
            items.push(value);
        }
        self.spans.insert(at, Span { from, to, key });
        Value::Array(items)
    }

    fn mapping(&mut self, indent: usize, at: String, key: Option<(usize, usize)>) -> Value {
        let mut out = Map::new();
        let mut merged: Vec<Map<String, Value>> = Vec::new();
        let from = self.lines[self.at].content;
        let mut to = from;
        loop {
            self.skip_blank();
            let Some(line) = self.lines.get(self.at).copied() else {
                break;
            };
            if line.indent != indent {
                if line.indent > indent {
                    self.fail(line.content);
                }
                break;
            }
            let content = self.content(&line).to_owned();
            if content == "-" || content.starts_with("- ") {
                break;
            }
            let Some(end) = key_end(&content) else {
                self.fail(line.content);
                break;
            };
            let raw_key = &content[..end];
            let name = if raw_key.starts_with(['"', '\'']) {
                yaml_quoted(raw_key)
            } else {
                raw_key.split_whitespace().collect::<Vec<_>>().join(" ")
            };
            let key_span = (line.content, line.content + end);
            let colon = end + content[end..].find(':').unwrap_or(0);
            let rest_at = line.content + colon + 1;
            let rest = self.text[rest_at..line.end].trim_start();
            let path = child(&at, &name);
            let value = if rest.is_empty() || is_properties_only(rest) {
                let anchor = anchor_of(rest);
                self.at += 1;
                self.skip_blank();
                let next = self.lines.get(self.at).copied();
                let value = match next {
                    Some(n) if n.indent > indent => self.node(indent + 1, path.clone(), Some(key_span)),
                    Some(n)
                        if n.indent == indent && {
                            let c = self.content(&n);
                            c == "-" || c.starts_with("- ")
                        } =>
                    {
                        self.sequence(indent, path.clone(), Some(key_span))
                    }
                    _ => self.null(path.clone(), Some(key_span)),
                };
                if let Some(anchor) = anchor {
                    self.anchors.insert(anchor, value.clone());
                }
                value
            } else {
                let start = line.end - rest.len();
                let (value, vfrom, vto) = self.inline_value(start, indent, &path);
                self.spans.insert(
                    path.clone(),
                    Span {
                        from: vfrom,
                        to: vto,
                        key: Some(key_span),
                    },
                );
                value
            };
            if let Some(span) = self.spans.get(&path) {
                to = to.max(span.to);
            }
            if name == "<<" && !raw_key.starts_with(['"', '\'']) {
                self.spans.remove(&path);
                match value {
                    Value::Object(m) => merged.push(m),
                    Value::Array(list) => merged.extend(list.into_iter().filter_map(|v| match v {
                        Value::Object(m) => Some(m),
                        _ => None,
                    })),
                    _ => {}
                }
                continue;
            }
            out.insert(name, value);
        }
        self.spans.insert(at, Span { from, to, key });
        if merged.is_empty() {
            return Value::Object(out);
        }
        let mut all = Map::new();
        for one in merged.into_iter().rev() {
            all.extend(one);
        }
        all.extend(out);
        Value::Object(all)
    }

    /// A value written after a key or a dash, from `start` on the current line: an alias, a
    /// flow collection, a quoted string, a block literal or a plain scalar, with its anchor and
    /// tag. Moves past every line it takes. `indent` is the indent of the line's key or dash.
    fn inline_value(&mut self, start: usize, indent: usize, at: &str) -> (Value, usize, usize) {
        let line = self.lines[self.at];
        let mut from = start;
        let mut anchor = None;
        // properties: an anchor and a tag, in either order
        loop {
            let rest = &self.text[from..line.end];
            if rest.starts_with('&') || rest.starts_with('!') {
                let len = rest.find([' ', '\t']).unwrap_or(rest.len());
                if rest.starts_with('&') {
                    anchor = Some(rest[1..len].to_owned());
                }
                from += len;
                from += self.text[from..line.end].len() - self.text[from..line.end].trim_start().len();
                if from >= line.end {
                    // the value is on the lines below
                    self.at += 1;
                    let value = self.node(indent + 1, at.to_owned(), None);
                    let to = self.spans.get(at).map_or(from, |s| s.to);
                    if let Some(anchor) = anchor {
                        self.anchors.insert(anchor, value.clone());
                    }
                    return (value, start, to);
                }
            } else {
                break;
            }
        }
        let rest = &self.text[from..line.end];
        let (value, to) = if let Some(name) = rest.strip_prefix('*') {
            self.at += 1;
            (self.anchors.get(name.trim()).cloned().unwrap_or(Value::Null), line.end)
        } else if rest.starts_with('[') || rest.starts_with('{') {
            let mut flow = Flow {
                text: self.text,
                pos: from,
                spans: Vec::new(),
                error: None,
            };
            let value = flow.value(at.to_owned(), None);
            for (path, span) in flow.spans {
                self.spans.insert(path, span);
            }
            if let Some(e) = flow.error {
                self.fail(e);
            }
            let end = flow.pos;
            // past the lines the collection takes, and what follows it on its last line
            while self.at < self.lines.len() && self.lines[self.at].start + self.line_len(self.at) < end {
                self.at += 1;
            }
            if let Some(last) = self.lines.get(self.at).copied() {
                let after = self.text[end.min(last.end)..last.end].trim();
                if !after.is_empty() {
                    self.fail(last.end - after.len());
                }
            }
            self.at += 1;
            (value, end)
        } else if rest.starts_with('"') || rest.starts_with('\'') {
            let quote = rest.as_bytes()[0];
            let b = self.text.as_bytes();
            let mut i = from + 1;
            while i < b.len() {
                if b[i] == b'\\' && quote == b'"' {
                    i += 2;
                    continue;
                }
                if b[i] == quote {
                    if quote == b'\'' && b.get(i + 1) == Some(&b'\'') {
                        i += 2;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            let end = (i + 1).min(b.len());
            if i >= b.len() {
                self.fail(from);
            }
            while self.at < self.lines.len() && self.lines[self.at].start + self.line_len(self.at) < end {
                self.at += 1;
            }
            if let Some(last) = self.lines.get(self.at).copied() {
                let after = self.text[end.min(last.end)..last.end].trim();
                if !after.is_empty() {
                    self.fail(last.end - after.len());
                }
            }
            self.at += 1;
            (Value::String(yaml_quoted(&self.text[from..end])), end)
        } else if rest.starts_with('|') || rest.starts_with('>') {
            let folded = rest.starts_with('>');
            let strip = rest.contains('-');
            let keep = rest.contains('+');
            self.at += 1;
            let mut body: Vec<&str> = Vec::new();
            let mut to = line.end;
            let mut content_indent = None;
            while self.at < self.lines.len() {
                let l = self.lines[self.at];
                let raw = &self.text[l.start..l.start + self.line_len(self.at)];
                if raw.trim().is_empty() {
                    body.push("");
                    self.at += 1;
                    continue;
                }
                if l.indent <= indent {
                    break;
                }
                let ci = *content_indent.get_or_insert(l.indent);
                if l.indent < ci {
                    break;
                }
                body.push(&raw[ci.min(raw.len())..]);
                to = l.start + raw.trim_end().len();
                self.at += 1;
            }
            while body.last() == Some(&"") && !keep {
                body.pop();
            }
            let mut text = if folded {
                let mut out = String::new();
                for (i, l) in body.iter().enumerate() {
                    if i > 0 {
                        out.push(if l.is_empty() || body[i - 1].is_empty() {
                            '\n'
                        } else {
                            ' '
                        });
                    }
                    out.push_str(l);
                }
                out
            } else {
                body.join("\n")
            };
            if !strip && !text.is_empty() {
                text.push('\n');
            }
            (Value::String(text), to)
        } else {
            // a plain scalar, and its continuation lines deeper than its key
            let mut parts = vec![rest.to_owned()];
            let mut to = line.end;
            self.at += 1;
            while self.at < self.lines.len() {
                let l = self.lines[self.at];
                if l.blank() {
                    break;
                }
                let c = self.content(&l);
                if l.indent <= indent || key_end(c).is_some() || c.starts_with("- ") || c == "-" {
                    break;
                }
                parts.push(c.to_owned());
                to = l.end;
                self.at += 1;
            }
            if rest.contains(": ") || rest.ends_with(':') {
                self.fail(from);
            }
            (yaml_scalar(&parts.join("\n")), to)
        };
        if let Some(anchor) = anchor {
            self.anchors.insert(anchor, value.clone());
        }
        (value, from, to)
    }

    fn line_len(&self, index: usize) -> usize {
        let start = self.lines[index].start;
        self.text[start..].find('\n').unwrap_or(self.text.len() - start)
    }
}

/// Whether a value is only an anchor or a tag, its node on the lines below.
fn is_properties_only(rest: &str) -> bool {
    rest.split_whitespace()
        .all(|w| w.starts_with('&') || w.starts_with('!'))
}

fn anchor_of(rest: &str) -> Option<String> {
    rest.split_whitespace()
        .find_map(|w| w.strip_prefix('&').map(str::to_owned))
}

/// The end of the key a line's content starts with: a plain or quoted key and its `:`, followed
/// by a space or the line's end.
pub(crate) fn key_end(content: &str) -> Option<usize> {
    if content.starts_with('"') || content.starts_with('\'') {
        let quote = content.as_bytes()[0];
        let b = content.as_bytes();
        let mut i = 1;
        while i < b.len() && b[i] != quote {
            i += if b[i] == b'\\' && quote == b'"' { 2 } else { 1 };
        }
        let end = (i + 1).min(b.len());
        let after = content[end..].trim_start();
        return (after.starts_with(':') && after[1..].chars().next().is_none_or(char::is_whitespace)).then_some(end);
    }
    if content.starts_with(['[', '{', '&', '*', '!', '|', '>', '#', '%', '@', '`']) {
        return None;
    }
    let b = content.as_bytes();
    for i in 0..b.len() {
        if b[i] == b':' && (i + 1 == b.len() || b[i + 1] == b' ' || b[i + 1] == b'\t') {
            let key = content[..i].trim_end();
            return (!key.is_empty()).then_some(key.len());
        }
    }
    None
}

/// A flow collection, `{ a: 1, b: [x, y] }`, read across lines.
struct Flow<'a> {
    text: &'a str,
    pos: usize,
    spans: Vec<(String, Span)>,
    error: Option<usize>,
}

impl Flow<'_> {
    fn space(&mut self) {
        let b = self.text.as_bytes();
        while self.pos < b.len() {
            match b[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                b'#' if self.pos > 0 && matches!(b[self.pos - 1], b' ' | b'\t' | b'\n') => {
                    while self.pos < b.len() && b[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn fail(&mut self) {
        if self.error.is_none() {
            self.error = Some(self.pos);
        }
    }

    fn value(&mut self, at: String, key: Option<(usize, usize)>) -> Value {
        self.space();
        let b = self.text.as_bytes();
        let from = self.pos;
        let value = match b.get(self.pos) {
            Some(b'{') | Some(b'[') => {
                let mapping = b[self.pos] == b'{';
                let close = if mapping { b'}' } else { b']' };
                self.pos += 1;
                let mut map = Map::new();
                let mut list = Vec::new();
                loop {
                    self.space();
                    match b.get(self.pos) {
                        None => {
                            self.fail();
                            break;
                        }
                        Some(c) if *c == close => {
                            self.pos += 1;
                            break;
                        }
                        _ => {}
                    }
                    if mapping {
                        let key_from = self.pos;
                        let name = self.scalar_text();
                        let key_span = (key_from, self.pos);
                        self.space();
                        let path = child(&at, &name);
                        let held = if b.get(self.pos) == Some(&b':') {
                            self.pos += 1;
                            self.value(path, Some(key_span))
                        } else {
                            self.spans.push((
                                path,
                                Span {
                                    from: key_from,
                                    to: self.pos,
                                    key: Some(key_span),
                                },
                            ));
                            Value::Null
                        };
                        map.insert(name, held);
                    } else {
                        let held = self.value(child(&at, &list.len().to_string()), None);
                        list.push(held);
                    }
                    self.space();
                    match b.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(c) if *c == close => {}
                        _ => {
                            self.fail();
                            break;
                        }
                    }
                    if self.error.is_some() {
                        break;
                    }
                }
                if mapping {
                    Value::Object(map)
                } else {
                    Value::Array(list)
                }
            }
            Some(b'"') | Some(b'\'') => Value::String(self.scalar_text()),
            Some(b'*') => {
                self.scalar_text();
                Value::Null
            }
            _ => {
                let start = self.pos;
                let text = self.scalar_text();
                if self.pos == start {
                    self.fail();
                    self.pos += 1;
                }
                yaml_scalar(&text)
            }
        };
        self.spans.push((
            at,
            Span {
                from,
                to: self.pos,
                key,
            },
        ));
        value
    }

    /// A key or a scalar in flow: quoted, or plain up to its next mark.
    fn scalar_text(&mut self) -> String {
        let b = self.text.as_bytes();
        let from = self.pos;
        if let Some(&q) = b.get(self.pos).filter(|c| **c == b'"' || **c == b'\'') {
            let mut i = self.pos + 1;
            while i < b.len() && b[i] != q {
                i += if b[i] == b'\\' && q == b'"' { 2 } else { 1 };
            }
            self.pos = (i + 1).min(b.len());
            return yaml_quoted(&self.text[from..self.pos]);
        }
        while self.pos < b.len() {
            let c = b[self.pos];
            if matches!(c, b',' | b'[' | b']' | b'{' | b'}' | b'\n')
                || (c == b':' && matches!(b.get(self.pos + 1), None | Some(b' ' | b'\n' | b',' | b']' | b'}')))
            {
                break;
            }
            self.pos += 1;
        }
        self.text[from..self.pos].trim().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_reads_with_spans() {
        let text = r#"{"name": "nightly-export", "steps": [{"id": "fetch"}]}"#;
        let read = read_json(text);
        assert_eq!(
            read.value,
            Some(json!({"name": "nightly-export", "steps": [{"id": "fetch"}]}))
        );
        assert!(read.broken.is_none());
        let span = read.spans["/steps/0/id"];
        assert_eq!(&text[span.from..span.to], "\"fetch\"");
        assert_eq!(&text[span.key.unwrap().0..span.key.unwrap().1], "\"id\"");
    }

    #[test]
    fn broken_json_says_where() {
        let read = read_json("{\"name\": \"x\",\n \"steps\" [] }");
        assert!(read.broken.is_some());
        assert_eq!(read.broken.unwrap().0, 23);
    }

    #[test]
    fn yaml_reads_mappings_lists_and_scalars() {
        let text = "name: nightly-export\nretries: 2\nsteps:\n  - id: fetch\n    kind: fetch\n  - id: upload\n    needs: [fetch]\nnote: |\n  line one\n  line two\nflag: true\n";
        let read = read_yaml(text);
        assert!(read.broken.is_none(), "{:?}", read.broken);
        assert_eq!(
            read.value,
            Some(json!({
                "name": "nightly-export",
                "retries": 2,
                "steps": [{"id": "fetch", "kind": "fetch"}, {"id": "upload", "needs": ["fetch"]}],
                "note": "line one\nline two\n",
                "flag": true
            }))
        );
        let span = read.spans["/steps/1/id"];
        assert_eq!(&text[span.from..span.to], "upload");
        let key = read.spans["/steps/1/needs"].key.unwrap();
        assert_eq!(&text[key.0..key.1], "needs");
    }

    #[test]
    fn yaml_lists_at_the_key_s_indent_anchors_and_streams() {
        let text = "base: &b\n  retries: 1\nsteps:\n- id: a\n  <<: *b\n---\nname: two\n";
        let read = read_yaml(text);
        assert!(read.broken.is_none(), "{:?}", read.broken);
        assert_eq!(
            read.value,
            Some(json!({"base": {"retries": 1}, "steps": [{"id": "a", "retries": 1}]}))
        );
        assert_eq!(read.documents.len(), 2);
        assert_eq!(read.documents[1].0, json!({"name": "two"}));
    }

    #[test]
    fn broken_yaml_says_where() {
        let read = read_yaml("name: x\n  bad: 1\n");
        assert!(read.broken.is_some());
    }
}
