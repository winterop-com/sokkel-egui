//! A JSON Schema (2020-12) checked against a JSON or YAML document and completed from, after
//! sokkel-ui's `code/schema.ts` and its ajv check: the issues placed on the value or key each is
//! about, the keys and values a place allows, and what a key is on hover.

use serde_json::{Map, Value};

use super::document::{ReadDocument, Span, Spans, parse_string, pointer, yaml_scalar};
use super::regex::Regex;
use super::{CodeEditorWords, MarkTone};

/// What is wrong with a value, by rule.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum IssueKind {
    Required,
    Additional(String),
    Type(String),
    Enum(Vec<Value>),
    Const(Value),
    Minimum { limit: f64, exclusive: bool },
    Maximum { limit: f64, exclusive: bool },
    MinLength(u64),
    MaxLength(u64),
    Pattern(String),
    Format(String),
    MinItems(u64),
    MaxItems(u64),
    UniqueItems,
    Deprecated,
    Other(String),
}

/// One thing a schema says about a value: where, how bad and which rule.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Issue {
    pub kind: IssueKind,
    pub path: Vec<String>,
    pub warning: bool,
}

/// A schema's finding placed on the text.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Finding {
    pub from: usize,
    pub to: usize,
    pub tone: MarkTone,
    pub message: String,
}

fn fill(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = template.to_owned();
    for (key, value) in pairs {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

/// A value as a schema writes it: strings quoted, everything else as JSON.
pub(crate) fn literal(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// An issue in a few words.
pub(crate) fn say(issue: &Issue, words: &CodeEditorWords) -> String {
    let count = |n: u64, one: &str, other: &str| fill(if n == 1 { one } else { other }, &[("count", &n.to_string())]);
    match &issue.kind {
        IssueKind::Required => fill(
            &words.issue_required,
            &[("property", issue.path.last().map_or("", String::as_str))],
        ),
        IssueKind::Additional(property) => fill(&words.issue_additional, &[("property", property)]),
        IssueKind::Type(expected) => {
            let a = if expected.starts_with(['a', 'e', 'i', 'o', 'u']) {
                "an"
            } else {
                "a"
            };
            fill(&words.issue_type, &[("a", a), ("expected", expected)])
        }
        IssueKind::Enum(values) => fill(
            &words.issue_enum,
            &[("values", &values.iter().map(literal).collect::<Vec<_>>().join(", "))],
        ),
        IssueKind::Const(value) => fill(&words.issue_const, &[("value", &literal(value))]),
        IssueKind::Minimum { limit, exclusive } => fill(
            if *exclusive {
                &words.issue_exclusive_minimum
            } else {
                &words.issue_minimum
            },
            &[("limit", &number(*limit))],
        ),
        IssueKind::Maximum { limit, exclusive } => fill(
            if *exclusive {
                &words.issue_exclusive_maximum
            } else {
                &words.issue_maximum
            },
            &[("limit", &number(*limit))],
        ),
        IssueKind::MinLength(n) => count(*n, &words.issue_min_length_one, &words.issue_min_length_other),
        IssueKind::MaxLength(n) => count(*n, &words.issue_max_length_one, &words.issue_max_length_other),
        IssueKind::Pattern(pattern) => fill(&words.issue_pattern, &[("pattern", pattern)]),
        IssueKind::Format(format) => fill(&words.issue_format, &[("format", format)]),
        IssueKind::MinItems(n) => count(*n, &words.issue_min_items_one, &words.issue_min_items_other),
        IssueKind::MaxItems(n) => count(*n, &words.issue_max_items_one, &words.issue_max_items_other),
        IssueKind::UniqueItems => words.issue_unique_items.clone(),
        IssueKind::Deprecated => words.issue_deprecated.clone(),
        IssueKind::Other(message) => {
            let mut chars = message.chars();
            let capital = chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default();
            fill(&words.issue_other, &[("message", &capital)])
        }
    }
}

// --- Reading a schema -------------------------------------------------------------------------

fn get<'a>(schema: &'a Value, key: &str) -> Option<&'a Value> {
    schema.as_object()?.get(key)
}

fn percent_decode(text: &str) -> Option<String> {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn anchored<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    match node {
        Value::Object(map) => {
            if map.get("$anchor").and_then(Value::as_str) == Some(name)
                || map.get("$dynamicAnchor").and_then(Value::as_str) == Some(name)
            {
                return Some(node);
            }
            map.values().find_map(|v| anchored(v, name))
        }
        Value::Array(list) => list.iter().find_map(|v| anchored(v, name)),
        _ => None,
    }
}

/// The schema a reference inside the document names: `#`, a JSON Pointer such as
/// `#/$defs/step`, or an anchor such as `#meta`.
fn resolve_local<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let rest = reference.strip_prefix('#')?;
    if rest.is_empty() {
        return Some(root);
    }
    if !rest.starts_with('/') {
        return anchored(root, &percent_decode(rest)?);
    }
    let mut node = root;
    for raw in rest[1..].split('/') {
        let part = percent_decode(raw)?.replace("~1", "/").replace("~0", "~");
        node = match node {
            Value::Object(map) => map.get(&part)?,
            Value::Array(list) => list.get(part.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    node.is_object()
        .then_some(node)
        .or_else(|| node.is_boolean().then_some(node))
}

fn reference(schema: &Value) -> Option<&str> {
    get(schema, "$ref")
        .or_else(|| get(schema, "$dynamicRef"))
        .and_then(Value::as_str)
}

/// A schema with its local `$ref` followed, its own keys over the target's.
fn follow_ref(schema: &Value, root: &Value) -> Value {
    let mut current = schema.clone();
    for _ in 0..16 {
        let Some(target) = reference(&current).and_then(|r| resolve_local(root, r)) else {
            break;
        };
        let mut merged = target.as_object().cloned().unwrap_or_default();
        if let Some(own) = current.as_object() {
            for (key, value) in own {
                if key != "$ref" && key != "$dynamicRef" {
                    merged.insert(key.clone(), value.clone());
                }
            }
        }
        current = Value::Object(merged);
    }
    current
}

/// A schema with its `$ref` followed and its `allOf` merged, as sokkel-ui's `resolve` reads one
/// for a key's type and its required keys.
pub(crate) fn resolve(schema: &Value, root: &Value) -> Value {
    let current = follow_ref(schema, root);
    let Some(parts) = get(&current, "allOf").and_then(Value::as_array) else {
        return current;
    };
    let mut merged = current.as_object().cloned().unwrap_or_default();
    merged.remove("allOf");
    for part in parts {
        let part = resolve(part, root);
        let Some(part) = part.as_object() else { continue };
        if let Some(props) = part.get("properties").and_then(Value::as_object) {
            let mut all = merged
                .get("properties")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            all.extend(props.clone());
            merged.insert("properties".into(), Value::Object(all));
        }
        if let Some(required) = part.get("required").and_then(Value::as_array) {
            let mut all = merged
                .get("required")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            all.extend(required.iter().cloned());
            merged.insert("required".into(), Value::Array(all));
        }
        for (key, value) in part {
            merged.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
    Value::Object(merged)
}

fn types_of(schema: &Value) -> Vec<String> {
    match get(schema, "type") {
        Some(Value::String(t)) => vec![t.clone()],
        Some(Value::Array(list)) => list.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
        _ if get(schema, "properties").is_some() => vec!["object".into()],
        _ => Vec::new(),
    }
}

fn pattern_only(schema: &Value) -> bool {
    get(schema, "type").and_then(Value::as_str) == Some("string")
        && get(schema, "pattern").is_some()
        && get(schema, "format").is_none()
        && get(schema, "enum").is_none()
        && get(schema, "const").is_none()
}

fn choice_label(variants: &[Value], root: &Value, word: &str) -> String {
    let resolved: Vec<Value> = variants.iter().map(|v| resolve(v, root)).collect();
    let is_null = |v: &Value| get(v, "type").and_then(Value::as_str) == Some("null");
    let nullable = resolved.iter().any(is_null);
    let real: Vec<&Value> = variants
        .iter()
        .zip(&resolved)
        .filter(|(_, r)| !is_null(r))
        .map(|(v, _)| v)
        .collect();
    let typed: Vec<&Value> = variants
        .iter()
        .zip(&resolved)
        .filter(|(_, r)| !is_null(r) && !pattern_only(r))
        .map(|(v, _)| v)
        .collect();
    let kept = if typed.len() == 1 { typed } else { real };
    let mut labels: Vec<String> = Vec::new();
    for v in &kept {
        let label = type_label(v, root);
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    let label = if labels.is_empty() {
        "null".to_owned()
    } else if labels.len() > 3 {
        format!("{word} of {}", kept.len())
    } else {
        labels.join(" or ")
    };
    if nullable && !labels.is_empty() && !label.ends_with(" or null") {
        format!("{label} or null")
    } else {
        label
    }
}

/// A schema's type in a few words: `string, email`, `array of step`, `integer or null`.
pub(crate) fn type_label(schema: &Value, root: &Value) -> String {
    let name = get(schema, "$ref")
        .and_then(Value::as_str)
        .and_then(|r| r.rsplit('/').next())
        .map(str::to_owned);
    let resolved = resolve(schema, root);
    if get(&resolved, "const").is_some() {
        return "constant".into();
    }
    if get(&resolved, "enum").is_some() {
        return "one of the values".into();
    }
    if let Some(list) = get(&resolved, "oneOf").and_then(Value::as_array) {
        return choice_label(list, root, "one");
    }
    if let Some(list) = get(&resolved, "anyOf").and_then(Value::as_array) {
        return choice_label(list, root, "any");
    }
    let types: Vec<String> = match get(&resolved, "type") {
        Some(Value::String(t)) => vec![t.clone()],
        Some(Value::Array(l)) => l.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
        _ => Vec::new(),
    };
    let nullable = types.iter().any(|t| t == "null");
    let main: Vec<&String> = types.iter().filter(|t| *t != "null").collect();
    let mut label = if main.is_empty() {
        if get(&resolved, "properties").is_some() {
            "object".into()
        } else {
            "any".into()
        }
    } else if main.len() > 1 {
        main.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" or ")
    } else if main[0] == "array" {
        match get(&resolved, "items") {
            Some(items) => format!("array of {}", type_label(items, root)),
            None => "array".into(),
        }
    } else if main[0] == "object" && name.is_some() {
        name.unwrap_or_default()
    } else {
        main[0].clone()
    };
    if let Some(format) = get(&resolved, "format").and_then(Value::as_str)
        && main.len() == 1
        && main[0] == "string"
    {
        label = format!("string, {format}");
    }
    if nullable { format!("{label} or null") } else { label }
}

// --- Checking ---------------------------------------------------------------------------------

fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b)),
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

fn has_type(value: &Value, kind: &str) -> bool {
    match kind {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        "number" => value.is_number(),
        "integer" => value.as_f64().is_some_and(|n| n.fract() == 0.0),
        _ => true,
    }
}

/// The value with every empty string property taken out, so a cleared field reads as absent.
fn prune(value: &Value) -> Value {
    match value {
        Value::Array(list) => Value::Array(list.iter().map(prune).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(_, v)| v.as_str() != Some(""))
                .map(|(k, v)| (k.clone(), prune(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Everything a schema says is wrong with a value, each at its path, and, as warnings, the
/// deprecated fields it sets.
pub(crate) fn validate(schema: &Value, value: &Value) -> Vec<Issue> {
    let document = prune(value);
    let checker = Checker { root: schema };
    let mut out = Vec::new();
    checker.check(schema, &document, &[], &mut out, 0);
    let explained: Vec<(Vec<String>, bool)> = out
        .iter()
        .map(|i| (i.path.clone(), matches!(i.kind, IssueKind::Format(_))))
        .collect();
    let mut kept: Vec<Issue> = Vec::new();
    for issue in out {
        if matches!(&issue.kind, IssueKind::Type(t) if t == "null") {
            continue;
        }
        // a pattern where a format failed on the same value says nothing more
        if matches!(issue.kind, IssueKind::Pattern(_)) && explained.iter().any(|(p, f)| *f && *p == issue.path) {
            continue;
        }
        if kept.iter().any(|k| k.path == issue.path && k.kind == issue.kind) {
            continue;
        }
        kept.push(issue);
    }
    deprecated(schema, &document, schema, &mut Vec::new(), &mut kept);
    kept
}

fn deprecated(schema: &Value, value: &Value, root: &Value, path: &mut Vec<String>, out: &mut Vec<Issue>) {
    if value.is_null() || value.as_str() == Some("") {
        return;
    }
    let at = resolve(schema, root);
    if get(&at, "deprecated") == Some(&Value::Bool(true)) {
        out.push(Issue {
            kind: IssueKind::Deprecated,
            path: path.clone(),
            warning: true,
        });
    }
    if let (Value::Array(list), Some(items)) = (value, get(&at, "items")) {
        for (index, item) in list.iter().enumerate() {
            path.push(index.to_string());
            deprecated(items, item, root, path, out);
            path.pop();
        }
    }
    if let (Value::Object(map), Some(Value::Object(props))) = (value, get(&at, "properties")) {
        for (key, child) in props {
            if let Some(held) = map.get(key) {
                path.push(key.clone());
                deprecated(child, held, root, path, out);
                path.pop();
            }
        }
    }
}

struct Checker<'a> {
    root: &'a Value,
}

fn issue(kind: IssueKind, path: &[String]) -> Issue {
    Issue {
        kind,
        path: path.to_vec(),
        warning: false,
    }
}

fn with(path: &[String], part: impl Into<String>) -> Vec<String> {
    let mut p = path.to_vec();
    p.push(part.into());
    p
}

impl Checker<'_> {
    fn passes(&self, schema: &Value, value: &Value, path: &[String], depth: u32) -> bool {
        let mut out = Vec::new();
        self.check(schema, value, path, &mut out, depth);
        out.is_empty()
    }

    fn check(&self, schema: &Value, value: &Value, path: &[String], out: &mut Vec<Issue>, depth: u32) {
        if depth > 48 {
            return;
        }
        let map = match schema {
            Value::Bool(true) => return,
            Value::Bool(false) => {
                out.push(issue(IssueKind::Other("boolean schema is false".into()), path));
                return;
            }
            Value::Object(map) => map,
            _ => return,
        };
        if let Some(target) = reference(schema).and_then(|r| resolve_local(self.root, r)) {
            self.check(target, value, path, out, depth + 1);
        }
        if let Some(t) = map.get("type") {
            let kinds: Vec<&str> = match t {
                Value::String(s) => vec![s.as_str()],
                Value::Array(l) => l.iter().filter_map(Value::as_str).collect(),
                _ => Vec::new(),
            };
            if !kinds.is_empty() && !kinds.iter().any(|k| has_type(value, k)) {
                out.push(issue(IssueKind::Type(kinds[0].to_owned()), path));
            }
        }
        if let Some(Value::Array(options)) = map.get("enum")
            && !options.iter().any(|o| same(o, value))
        {
            out.push(issue(IssueKind::Enum(options.clone()), path));
        }
        if let Some(constant) = map.get("const")
            && !same(constant, value)
        {
            out.push(issue(IssueKind::Const(constant.clone()), path));
        }
        match value {
            Value::Object(object) => self.object(map, object, path, out, depth),
            Value::Array(list) => self.array(map, list, path, out, depth),
            Value::String(text) => self.string(map, text, path, out),
            Value::Number(n) => self.number(map, n.as_f64().unwrap_or(0.0), path, out),
            _ => {}
        }
        if let Some(Value::Array(parts)) = map.get("allOf") {
            for part in parts {
                self.check(part, value, path, out, depth + 1);
            }
        }
        for (word, exactly_one) in [("anyOf", false), ("oneOf", true)] {
            if let Some(Value::Array(branches)) = map.get(word) {
                self.choice(branches, value, path, out, depth, exactly_one);
            }
        }
        if let Some(not) = map.get("not")
            && self.passes(not, value, path, depth + 1)
        {
            out.push(issue(IssueKind::Other("must NOT be valid".into()), path));
        }
        if let Some(test) = map.get("if") {
            let branch = if self.passes(test, value, path, depth + 1) {
                map.get("then")
            } else {
                map.get("else")
            };
            if let Some(branch) = branch {
                self.check(branch, value, path, out, depth + 1);
            }
        }
    }

    /// A failed `anyOf` or `oneOf` narrowed to the branch that fits the value: the one whose fixed
    /// properties match it, else the one with the fewest issues, a branch of the value's type first.
    fn choice(&self, branches: &[Value], value: &Value, path: &[String], out: &mut Vec<Issue>, depth: u32, one: bool) {
        let found: Vec<Vec<Issue>> = branches
            .iter()
            .map(|b| {
                let mut o = Vec::new();
                self.check(b, value, path, &mut o, depth + 1);
                o
            })
            .collect();
        let passing = found.iter().filter(|f| f.is_empty()).count();
        if passing > 1 && one {
            out.push(issue(
                IssueKind::Other("must match exactly one schema in oneOf".into()),
                path,
            ));
            return;
        }
        if passing > 0 || branches.is_empty() {
            return;
        }
        let fixed: Vec<Vec<(String, Value)>> = branches.iter().map(|b| self.discriminators(b, 0)).collect();
        let record = value.as_object();
        let fits: Vec<usize> = fixed
            .iter()
            .enumerate()
            .filter(|(_, props)| {
                record.is_some_and(|r| {
                    !props.is_empty() && props.iter().all(|(k, want)| r.get(k).is_some_and(|v| same(v, want)))
                })
            })
            .map(|(i, _)| i)
            .collect();
        if fits.is_empty()
            && let Some(record) = record
            && let Some((shared, _)) = fixed[0].iter().find(|(key, _)| {
                fixed.iter().all(|props| props.iter().any(|(k, _)| k == key)) && record.contains_key(key)
            })
        {
            let mut allowed: Vec<Value> = Vec::new();
            for props in &fixed {
                if let Some((_, want)) = props.iter().find(|(k, _)| k == shared)
                    && !allowed.iter().any(|a| same(a, want))
                {
                    allowed.push(want.clone());
                }
            }
            out.push(issue(IssueKind::Enum(allowed), &with(path, shared.clone())));
            return;
        }
        let candidates: Vec<usize> = if fits.is_empty() {
            (0..branches.len()).collect()
        } else {
            fits
        };
        let rank = |i: usize| {
            let wrong_type = found[i]
                .iter()
                .any(|f| matches!(f.kind, IssueKind::Type(_)) && f.path == path);
            usize::from(wrong_type) * 1_000_000 + found[i].len()
        };
        let best = candidates.into_iter().min_by_key(|i| rank(*i)).unwrap_or(0);
        out.extend(found[best].iter().cloned());
    }

    /// The properties a branch fixes to one value, by `const` or a one-value `enum`, in it or its `allOf`.
    fn discriminators(&self, branch: &Value, depth: u32) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        if depth > 4 {
            return out;
        }
        let schema = follow_ref(branch, self.root);
        if let Some(Value::Object(props)) = get(&schema, "properties") {
            for (key, property) in props {
                let fixed = follow_ref(property, self.root);
                if let Some(c) = get(&fixed, "const") {
                    out.push((key.clone(), c.clone()));
                } else if let Some(Value::Array(list)) = get(&fixed, "enum")
                    && list.len() == 1
                {
                    out.push((key.clone(), list[0].clone()));
                }
            }
        }
        if let Some(Value::Array(parts)) = get(&schema, "allOf") {
            for part in parts {
                out.extend(self.discriminators(part, depth + 1));
            }
        }
        out
    }

    /// The keys a schema accounts for in an object, for `unevaluatedProperties`.
    fn evaluated(
        &self,
        schema: &Value,
        object: &Map<String, Value>,
        path: &[String],
        depth: u32,
        into: &mut Vec<String>,
    ) {
        if depth > 16 {
            return;
        }
        let Some(map) = schema.as_object() else { return };
        if let Some(target) = reference(schema).and_then(|r| resolve_local(self.root, r)) {
            self.evaluated(target, object, path, depth + 1, into);
        }
        if let Some(Value::Object(props)) = map.get("properties") {
            into.extend(props.keys().filter(|k| object.contains_key(*k)).cloned());
        }
        if let Some(Value::Object(patterns)) = map.get("patternProperties") {
            for key in object.keys() {
                if patterns
                    .keys()
                    .any(|p| Regex::new(p, false).and_then(|r| r.is_match(key)).unwrap_or(true))
                {
                    into.push(key.clone());
                }
            }
        }
        if map.contains_key("additionalProperties")
            || matches!(
                map.get("unevaluatedProperties"),
                Some(Value::Object(_)) | Some(Value::Bool(true))
            )
        {
            into.extend(object.keys().cloned());
        }
        if let Some(Value::Array(parts)) = map.get("allOf") {
            for part in parts {
                self.evaluated(part, object, path, depth + 1, into);
            }
        }
        for word in ["anyOf", "oneOf"] {
            if let Some(Value::Array(parts)) = map.get(word) {
                for part in parts {
                    if self.passes(part, &Value::Object(object.clone()), path, depth + 1) {
                        self.evaluated(part, object, path, depth + 1, into);
                    }
                }
            }
        }
        if let Some(test) = map.get("if") {
            let whole = Value::Object(object.clone());
            let branch = if self.passes(test, &whole, path, depth + 1) {
                self.evaluated(test, object, path, depth + 1, into);
                map.get("then")
            } else {
                map.get("else")
            };
            if let Some(branch) = branch {
                self.evaluated(branch, object, path, depth + 1, into);
            }
        }
    }

    fn object(
        &self,
        map: &Map<String, Value>,
        object: &Map<String, Value>,
        path: &[String],
        out: &mut Vec<Issue>,
        depth: u32,
    ) {
        if let Some(Value::Array(required)) = map.get("required") {
            for key in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(key) {
                    out.push(issue(IssueKind::Required, &with(path, key)));
                }
            }
        }
        if let Some(Value::Object(dependent)) = map.get("dependentRequired") {
            for (key, needs) in dependent {
                if object.contains_key(key) {
                    for need in needs.as_array().into_iter().flatten().filter_map(Value::as_str) {
                        if !object.contains_key(need) {
                            out.push(issue(IssueKind::Required, &with(path, need)));
                        }
                    }
                }
            }
        }
        if let Some(Value::Object(dependent)) = map.get("dependentSchemas") {
            for (key, schema) in dependent {
                if object.contains_key(key) {
                    self.check(schema, &Value::Object(object.clone()), path, out, depth + 1);
                }
            }
        }
        let props = map.get("properties").and_then(Value::as_object);
        let patterns: Vec<(Option<Regex>, &Value)> = map
            .get("patternProperties")
            .and_then(Value::as_object)
            .map(|p| p.iter().map(|(k, v)| (Regex::new(k, false), v)).collect())
            .unwrap_or_default();
        for (key, held) in object {
            let mut known = false;
            if let Some(schema) = props.and_then(|p| p.get(key)) {
                known = true;
                self.check(schema, held, &with(path, key.clone()), out, depth + 1);
            }
            for (regex, schema) in &patterns {
                match regex.as_ref().map(|r| r.is_match(key)) {
                    Some(Some(true)) => {
                        known = true;
                        self.check(schema, held, &with(path, key.clone()), out, depth + 1);
                    }
                    Some(Some(false)) => {}
                    // a pattern this engine cannot read or answer in time counts the key as known
                    _ => known = true,
                }
            }
            if !known {
                match map.get("additionalProperties") {
                    Some(Value::Bool(false)) => out.push(issue(IssueKind::Additional(key.clone()), path)),
                    Some(schema @ Value::Object(_)) => {
                        self.check(schema, held, &with(path, key.clone()), out, depth + 1);
                    }
                    _ => {}
                }
            }
            if let Some(names) = map.get("propertyNames")
                && !self.passes(names, &Value::String(key.clone()), path, depth + 1)
            {
                out.push(issue(
                    IssueKind::Other(format!("property name '{key}' is invalid")),
                    path,
                ));
            }
        }
        if let Some(unevaluated) = map.get("unevaluatedProperties") {
            let mut seen = Vec::new();
            let mut bare = map.clone();
            bare.remove("unevaluatedProperties");
            self.evaluated(&Value::Object(bare), object, path, depth + 1, &mut seen);
            for (key, held) in object {
                if seen.contains(key) {
                    continue;
                }
                match unevaluated {
                    Value::Bool(false) => out.push(issue(IssueKind::Additional(key.clone()), path)),
                    schema @ Value::Object(_) => self.check(schema, held, &with(path, key.clone()), out, depth + 1),
                    _ => {}
                }
            }
        }
        let count = object.len() as u64;
        if let Some(min) = map.get("minProperties").and_then(Value::as_u64)
            && count < min
        {
            out.push(issue(
                IssueKind::Other(format!("must NOT have fewer than {min} properties")),
                path,
            ));
        }
        if let Some(max) = map.get("maxProperties").and_then(Value::as_u64)
            && count > max
        {
            out.push(issue(
                IssueKind::Other(format!("must NOT have more than {max} properties")),
                path,
            ));
        }
    }

    fn array(&self, map: &Map<String, Value>, list: &[Value], path: &[String], out: &mut Vec<Issue>, depth: u32) {
        let prefix = map.get("prefixItems").and_then(Value::as_array);
        let skip = prefix.map_or(0, Vec::len);
        if let Some(prefix) = prefix {
            for (index, (schema, item)) in prefix.iter().zip(list).enumerate() {
                self.check(schema, item, &with(path, index.to_string()), out, depth + 1);
            }
        }
        if let Some(items) = map.get("items") {
            for (index, item) in list.iter().enumerate().skip(skip) {
                self.check(items, item, &with(path, index.to_string()), out, depth + 1);
            }
        }
        let len = list.len() as u64;
        if let Some(min) = map.get("minItems").and_then(Value::as_u64)
            && len < min
        {
            out.push(issue(IssueKind::MinItems(min), path));
        }
        if let Some(max) = map.get("maxItems").and_then(Value::as_u64)
            && len > max
        {
            out.push(issue(IssueKind::MaxItems(max), path));
        }
        if map.get("uniqueItems") == Some(&Value::Bool(true))
            && list
                .iter()
                .enumerate()
                .any(|(i, a)| list[..i].iter().any(|b| same(a, b)))
        {
            out.push(issue(IssueKind::UniqueItems, path));
        }
        if let Some(contains) = map.get("contains") {
            let hits = list
                .iter()
                .enumerate()
                .filter(|(i, item)| self.passes(contains, item, &with(path, i.to_string()), depth + 1))
                .count() as u64;
            let min = map.get("minContains").and_then(Value::as_u64).unwrap_or(1);
            if hits < min {
                out.push(issue(
                    IssueKind::Other(format!("must contain at least {min} valid item(s)")),
                    path,
                ));
            }
            if let Some(max) = map.get("maxContains").and_then(Value::as_u64)
                && hits > max
            {
                out.push(issue(
                    IssueKind::Other(format!("must contain at most {max} valid item(s)")),
                    path,
                ));
            }
        }
    }

    fn string(&self, map: &Map<String, Value>, text: &str, path: &[String], out: &mut Vec<Issue>) {
        let len = text.chars().count() as u64;
        if let Some(min) = map.get("minLength").and_then(Value::as_u64)
            && len < min
        {
            out.push(issue(IssueKind::MinLength(min), path));
        }
        if let Some(max) = map.get("maxLength").and_then(Value::as_u64)
            && len > max
        {
            out.push(issue(IssueKind::MaxLength(max), path));
        }
        if let Some(pattern) = map.get("pattern").and_then(Value::as_str)
            && Regex::new(pattern, false).and_then(|r| r.is_match(text)) == Some(false)
        {
            out.push(issue(IssueKind::Pattern(pattern.to_owned()), path));
        }
        if let Some(format) = map.get("format").and_then(Value::as_str)
            && format_holds(format, text) == Some(false)
        {
            out.push(issue(IssueKind::Format(format.to_owned()), path));
        }
    }

    fn number(&self, map: &Map<String, Value>, n: f64, path: &[String], out: &mut Vec<Issue>) {
        let limit = |key: &str| map.get(key).and_then(Value::as_f64);
        if let Some(min) = limit("minimum")
            && n < min
        {
            out.push(issue(
                IssueKind::Minimum {
                    limit: min,
                    exclusive: false,
                },
                path,
            ));
        }
        if let Some(min) = limit("exclusiveMinimum")
            && n <= min
        {
            out.push(issue(
                IssueKind::Minimum {
                    limit: min,
                    exclusive: true,
                },
                path,
            ));
        }
        if let Some(max) = limit("maximum")
            && n > max
        {
            out.push(issue(
                IssueKind::Maximum {
                    limit: max,
                    exclusive: false,
                },
                path,
            ));
        }
        if let Some(max) = limit("exclusiveMaximum")
            && n >= max
        {
            out.push(issue(
                IssueKind::Maximum {
                    limit: max,
                    exclusive: true,
                },
                path,
            ));
        }
        if let Some(step) = limit("multipleOf")
            && step > 0.0
        {
            let q = n / step;
            if (q - q.round()).abs() > 1e-9 {
                out.push(issue(
                    IssueKind::Other(format!("must be multiple of {}", number(step))),
                    path,
                ));
            }
        }
    }
}

/// Whether a string holds a format; `None` for a format not known here, which is passed over.
fn format_holds(format: &str, text: &str) -> Option<bool> {
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    let date = |s: &str| {
        let parts: Vec<&str> = s.split('-').collect();
        parts.len() == 3 && digits(parts[0], 4) && digits(parts[1], 2) && digits(parts[2], 2) && {
            let year: u32 = parts[0].parse().unwrap_or(0);
            let month: u32 = parts[1].parse().unwrap_or(0);
            let day: u32 = parts[2].parse().unwrap_or(0);
            let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
            let days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            (1..=12).contains(&month) && day >= 1 && day <= days[(month - 1) as usize]
        }
    };
    let time = |s: &str| {
        let (clock, zone) = match s.find(['Z', 'z', '+', '-']) {
            Some(i) => (&s[..i], Some(&s[i..])),
            None => (s, None),
        };
        let (clock, fraction) = clock.split_once('.').map_or((clock, None), |(a, b)| (a, Some(b)));
        let parts: Vec<&str> = clock.split(':').collect();
        let ok_clock = parts.len() == 3
            && parts.iter().all(|p| digits(p, 2))
            && parts[0].parse::<u32>().unwrap_or(99) < 24
            && parts[1].parse::<u32>().unwrap_or(99) < 60
            && parts[2].parse::<u32>().unwrap_or(99) <= 60;
        let ok_fraction = fraction.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()));
        let ok_zone = zone.is_some_and(|z| {
            z.eq_ignore_ascii_case("z") || {
                let rest = &z[1..];
                rest.len() == 5 && digits(&rest[..2], 2) && &rest[2..3] == ":" && digits(&rest[3..], 2)
            }
        });
        ok_clock && ok_fraction && ok_zone
    };
    Some(match format {
        "date" => date(text),
        "time" => time(text),
        "date-time" => text
            .split_once(['T', 't', ' '])
            .is_some_and(|(d, t)| date(d) && time(t)),
        "email" => text.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !text.contains(char::is_whitespace)
        }),
        "uri" | "url" | "iri" => text.split_once(':').is_some_and(|(scheme, rest)| {
            scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
                && !rest.is_empty()
                && !text.contains(char::is_whitespace)
        }),
        "uri-reference" | "iri-reference" | "uri-template" => !text.contains(char::is_whitespace),
        "uuid" => {
            let parts: Vec<&str> = text.split('-').collect();
            parts.len() == 5
                && [8, 4, 4, 4, 12]
                    .iter()
                    .zip(&parts)
                    .all(|(n, p)| p.len() == *n && p.bytes().all(|b| b.is_ascii_hexdigit()))
        }
        "ipv4" => {
            let parts: Vec<&str> = text.split('.').collect();
            parts.len() == 4
                && parts.iter().all(|p| {
                    !p.is_empty()
                        && p.len() <= 3
                        && p.bytes().all(|b| b.is_ascii_digit())
                        && p.parse::<u32>().unwrap_or(999) <= 255
                })
        }
        "ipv6" => text.parse::<std::net::Ipv6Addr>().is_ok(),
        "hostname" | "idn-hostname" => {
            !text.is_empty()
                && text.len() <= 253
                && text.split('.').all(|l| {
                    !l.is_empty()
                        && l.len() <= 63
                        && !l.starts_with('-')
                        && !l.ends_with('-')
                        && l.chars().all(|c| c.is_alphanumeric() || c == '-')
                })
        }
        "regex" => Regex::new(text, false).is_some(),
        "json-pointer" => text.is_empty() || text.starts_with('/'),
        "duration" => text.starts_with('P') && text.len() > 1,
        _ => return None,
    })
}

// --- Placing ----------------------------------------------------------------------------------

/// Where an issue sits: the value it names when that is one line, else its key, else the first
/// line of the nearest value written above it.
fn place(issue: &Issue, spans: &Spans, text: &str) -> (usize, usize) {
    let target: Vec<String> = match &issue.kind {
        IssueKind::Additional(property) => with(&issue.path, property.clone()),
        IssueKind::Required => issue.path[..issue.path.len().saturating_sub(1)].to_vec(),
        _ => issue.path.clone(),
    };
    let whole = !matches!(issue.kind, IssueKind::Additional(_) | IssueKind::Required);
    let line_end = |at: usize| text[at..].find('\n').map_or(text.len(), |e| at + e);
    for depth in (0..=target.len()).rev() {
        let Some(span) = spans.get(&pointer(&target[..depth])) else {
            continue;
        };
        let end = line_end(span.from);
        if whole && depth == target.len() && span.to <= end && !text[span.from..].starts_with(['[', '{']) {
            return (span.from, span.to);
        }
        if let Some(key) = span.key {
            return key;
        }
        return (span.from, span.to.min(end));
    }
    (0, line_end(0))
}

/// What a schema finds wrong with a document, placed on its text: the syntax error where the
/// text stops being the language, else each issue, each document of a stream on its own.
pub(crate) fn findings(
    text: &str,
    read: &ReadDocument,
    schema: &Value,
    language_name: &str,
    words: &CodeEditorWords,
    deferred: Option<&dyn Fn(&Value) -> bool>,
) -> Vec<(Finding, Vec<String>)> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    if let Some((from, to)) = read.broken {
        return vec![(
            Finding {
                from,
                to,
                tone: MarkTone::Error,
                message: fill(&words.syntax, &[("language", language_name)]),
            },
            Vec::new(),
        )];
    }
    let first = read.value.clone().map(|v| (v, read.spans.clone()));
    let documents: Vec<(Value, Spans)> = if read.documents.is_empty() {
        first.into_iter().collect()
    } else {
        read.documents.clone()
    };
    let mut out = Vec::new();
    for (value, spans) in documents {
        for issue in validate(schema, &value) {
            // a value the app checks itself, or one under it, is left to the app
            if let Some(deferred) = deferred
                && (0..=issue.path.len()).any(|depth| value_at(&value, &issue.path[..depth]).is_some_and(deferred))
            {
                continue;
            }
            let (from, to) = place(&issue, &spans, text);
            let path = match &issue.kind {
                IssueKind::Additional(p) => with(&issue.path, p.clone()),
                _ => issue.path.clone(),
            };
            out.push((
                Finding {
                    from,
                    to,
                    tone: if issue.warning {
                        MarkTone::Warning
                    } else {
                        MarkTone::Error
                    },
                    message: say(&issue, words),
                },
                path,
            ));
        }
    }
    out
}

/// Where the value at a path is written, its key included.
pub(crate) fn path_range(read: &ReadDocument, path: &[String]) -> Option<(usize, usize)> {
    let span = read.spans.get(&pointer(path))?;
    Some((span.key.map_or(span.from, |k| k.0), span.to))
}

// --- Completion -------------------------------------------------------------------------------

/// Where the caret is in the document's structure: the path to the object or list it is in, and
/// whether it is at a key or a value.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Slot {
    pub path: Vec<String>,
    pub at_key: bool,
    pub key: Option<String>,
    pub from: usize,
    pub quoted: bool,
}

enum Frame {
    Object { key: Option<String>, at_key: bool },
    Array(usize),
}

/// The slot at a position in JSON, read by scanning the text before it.
pub(crate) fn json_slot(source: &str, pos: usize) -> Option<Slot> {
    let b = source.as_bytes();
    let mut stack: Vec<Frame> = Vec::new();
    let mut string_start: Option<usize> = None;
    let mut i = 0;
    while i < pos {
        let c = b[i];
        if let Some(start) = string_start {
            if c == b'\\' {
                i += 1;
            } else if c == b'"' {
                if let Some(Frame::Object { key, at_key: true }) = stack.last_mut() {
                    *key = Some(parse_string(&source[start..=i]));
                }
                string_start = None;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => string_start = Some(i),
            b'{' => stack.push(Frame::Object {
                key: None,
                at_key: true,
            }),
            b'[' => stack.push(Frame::Array(0)),
            b'}' | b']' => {
                stack.pop();
            }
            b':' => {
                if let Some(Frame::Object { at_key, .. }) = stack.last_mut() {
                    *at_key = false;
                }
            }
            b',' => match stack.last_mut() {
                Some(Frame::Object { key, at_key }) => {
                    *at_key = true;
                    *key = None;
                }
                Some(Frame::Array(index)) => *index += 1,
                None => {}
            },
            _ => {}
        }
        i += 1;
    }
    let top = stack.last()?;
    let path: Vec<String> = stack[..stack.len() - 1]
        .iter()
        .map(|f| match f {
            Frame::Object { key, .. } => key.clone().unwrap_or_default(),
            Frame::Array(i) => i.to_string(),
        })
        .collect();
    let quoted = string_start.is_some();
    let from = match string_start {
        Some(s) => s + 1,
        None => {
            let before = &source[..pos];
            pos - (before.len()
                - before
                    .trim_end_matches(|c: char| c.is_alphanumeric() || c == '_' || c == '-')
                    .len())
        }
    };
    Some(match top {
        Frame::Array(index) => Slot {
            path: with(&path, index.to_string()),
            at_key: false,
            key: None,
            from,
            quoted,
        },
        Frame::Object { at_key: true, .. } => Slot {
            path,
            at_key: true,
            key: None,
            from,
            quoted,
        },
        Frame::Object { key, .. } => Slot {
            path,
            at_key: false,
            key: key.clone(),
            from,
            quoted,
        },
    })
}

struct YamlLine {
    dashes: Vec<usize>,
    key: Option<(String, usize, bool)>,
    content: usize,
}

/// The columns a YAML line opens: its dashes and its key, each with the column it starts at.
fn yaml_line(line: &str) -> YamlLine {
    let mut dashes = Vec::new();
    let mut column = line.len() - line.trim_start_matches(' ').len();
    loop {
        let rest = &line[column..];
        if rest.starts_with('-') && (rest.len() == 1 || rest[1..].starts_with(' ')) {
            dashes.push(column);
            column += 1 + (rest[1..].len() - rest[1..].trim_start_matches(' ').len());
        } else {
            break;
        }
    }
    let rest = &line[column..];
    let key = super::document::key_end(rest).map(|end| {
        let raw = &rest[..end];
        let name = if raw.starts_with('"') {
            parse_string(raw)
        } else {
            raw.trim_matches('\'').to_owned()
        };
        let after = &rest[end..];
        let after = after[after.find(':').map_or(after.len(), |c| c + 1)..].to_owned();
        let after = after.split(" #").next().unwrap_or("").trim().to_owned();
        (name, column, after.is_empty() || after.starts_with('#'))
    });
    YamlLine {
        dashes,
        key,
        content: column,
    }
}

fn text_lines(text: &str) -> Vec<(usize, &str)> {
    let mut offset = 0;
    text.split('\n')
        .map(|l| {
            let at = offset;
            offset += l.len() + 1;
            (at, l)
        })
        .collect()
}

fn skip_line(line: &str) -> bool {
    line.trim().is_empty() || line.trim_start().starts_with('#')
}

fn items_before(lines: &[(usize, &str)], number: usize, column: usize) -> usize {
    let mut count = 0;
    for (_, text) in lines[..number].iter().rev() {
        if skip_line(text) {
            continue;
        }
        let parsed = yaml_line(text);
        let start = parsed.dashes.first().copied().unwrap_or(parsed.content);
        if start < column {
            break;
        }
        if parsed.dashes.first() == Some(&column) {
            count += 1;
        }
    }
    count
}

/// The slot at a position in YAML, read from the indentation of the lines above it.
pub(crate) fn yaml_slot(text: &str, pos: usize) -> Slot {
    let lines = text_lines(text);
    let number = lines.iter().rposition(|(at, _)| *at <= pos).unwrap_or(0);
    let (line_from, _) = lines[number];
    let before = &text[line_from..pos];
    let here = yaml_line(before);
    let word = before.len() - before.trim_end_matches(|c: char| !c.is_whitespace() && c != ':').len();
    let from = pos - word;
    let mut tail: Vec<String> = Vec::new();
    for (i, dash) in here.dashes.iter().enumerate() {
        tail.push(if i == 0 {
            items_before(&lines, number, *dash).to_string()
        } else {
            "0".into()
        });
    }
    let mut target = here.dashes.first().copied().unwrap_or(here.content);
    let mut path: Vec<Vec<String>> = Vec::new();
    for n in (0..number).rev() {
        if target == 0 {
            break;
        }
        let above = lines[n].1;
        if skip_line(above) {
            continue;
        }
        let parsed = yaml_line(above);
        let mut found: Vec<String> = Vec::new();
        if let Some((name, column, open)) = &parsed.key
            && *column < target
            && *open
        {
            found.insert(0, name.clone());
            target = *column;
        }
        for dash in parsed.dashes.iter().rev() {
            if *dash < target {
                found.insert(
                    0,
                    if Some(dash) == parsed.dashes.first() {
                        items_before(&lines, n, *dash).to_string()
                    } else {
                        "0".into()
                    },
                );
                target = *dash;
            }
        }
        if !found.is_empty() {
            path.insert(0, found);
        }
    }
    let mut at: Vec<String> = path.into_iter().flatten().collect();
    at.extend(tail);
    if let Some((name, column, _)) = &here.key
        && pos - line_from > column + name.len()
    {
        return Slot {
            path: at,
            at_key: false,
            key: Some(name.clone()),
            from,
            quoted: false,
        };
    }
    Slot {
        path: at,
        at_key: true,
        key: None,
        from,
        quoted: false,
    }
}

fn string_end(b: &[u8], start: usize) -> usize {
    let mut end = start + 1;
    while end < b.len() && b[end] != b'"' && b[end] != b'\n' {
        end += if b[end] == b'\\' { 2 } else { 1 };
    }
    if end < b.len() && b[end] == b'"' {
        end + 1
    } else {
        end.min(b.len())
    }
}

/// The keys of the JSON object around `pos`, without the key being written there.
pub(crate) fn json_keys(source: &str, pos: usize) -> Vec<String> {
    let b = source.as_bytes();
    let mut found = Vec::new();
    let mut opens: Vec<usize> = Vec::new();
    let mut quote: Option<usize> = None;
    let mut i = 0;
    while i < pos {
        let c = b[i];
        if quote.is_some() {
            if c == b'\\' {
                i += 1;
            } else if c == b'"' {
                quote = None;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => quote = Some(i),
            b'{' | b'[' => opens.push(i),
            b'}' | b']' => {
                opens.pop();
            }
            _ => {}
        }
        i += 1;
    }
    let Some(&open) = opens.last() else { return found };
    if b[open] != b'{' {
        return found;
    }
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c == b'-';
    let (mut start, mut end) = (pos, pos);
    if let Some(q) = quote {
        start = q;
        end = string_end(b, q);
    } else {
        while start > open + 1 && word(b[start - 1]) {
            start -= 1;
        }
        while end < b.len() && word(b[end]) {
            end += 1;
        }
    }
    let mut scan = |from: usize, to: usize| {
        let mut depth: i32 = 0;
        let mut pending: Option<String> = None;
        let mut i = from;
        while i < to {
            let c = b[i];
            if c == b'"' {
                let close = string_end(b, i);
                if depth == 0 {
                    pending = Some(parse_string(&source[i..close]));
                }
                i = close;
                continue;
            }
            if c == b':'
                && depth == 0
                && let Some(p) = pending.take()
            {
                found.push(p);
            }
            if !c.is_ascii_whitespace() {
                pending = None;
            }
            if c == b'{' || c == b'[' {
                depth += 1;
            } else if c == b'}' || c == b']' {
                depth -= 1;
                if depth < 0 {
                    return;
                }
            }
            i += 1;
        }
    };
    scan(open + 1, start);
    scan(end, source.len());
    found
}

/// The keys a YAML mapping already holds beside the line at `pos`, read from the lines with the same key column.
pub(crate) fn yaml_keys(text: &str, pos: usize, column: usize) -> Vec<String> {
    let lines = text_lines(text);
    let here = lines.iter().rposition(|(at, _)| *at <= pos).unwrap_or(0);
    let start_of = |p: &YamlLine| p.dashes.first().copied().unwrap_or(p.content);
    let mut found = Vec::new();
    for (_, line) in lines[..here].iter().rev() {
        if skip_line(line) {
            continue;
        }
        let parsed = yaml_line(line);
        if let Some((name, c, _)) = &parsed.key
            && *c == column
            && start_of(&parsed) <= column
        {
            found.push(name.clone());
        }
        if start_of(&parsed) < column {
            break;
        }
    }
    for (_, line) in &lines[here + 1..] {
        if skip_line(line) {
            continue;
        }
        let parsed = yaml_line(line);
        if start_of(&parsed) < column {
            break;
        }
        if let Some((name, c, _)) = &parsed.key
            && *c == column
            && parsed.dashes.is_empty()
        {
            found.push(name.clone());
        }
    }
    found
}

/// Whether a value meets an `if`: const, enum, type, required, properties, allOf, anyOf, not.
fn satisfies(condition: &Value, value: Option<&Value>) -> bool {
    let value_or_null = value.unwrap_or(&Value::Null);
    if let Some(c) = get(condition, "const")
        && !value.is_some_and(|v| same(c, v))
    {
        return false;
    }
    if let Some(Value::Array(options)) = get(condition, "enum")
        && !options.iter().any(|o| value.is_some_and(|v| same(o, v)))
    {
        return false;
    }
    let kinds = match get(condition, "type") {
        Some(Value::String(t)) => vec![t.clone()],
        Some(Value::Array(l)) => l.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
        _ => Vec::new(),
    };
    if !kinds.is_empty() && !kinds.iter().any(|k| value.is_some_and(|v| has_type(v, k))) {
        return false;
    }
    if let Some(Value::Array(required)) = get(condition, "required") {
        let Some(record) = value_or_null.as_object() else {
            return false;
        };
        if required
            .iter()
            .filter_map(Value::as_str)
            .any(|k| !record.contains_key(k))
        {
            return false;
        }
    }
    if let (Some(Value::Object(props)), Some(record)) = (get(condition, "properties"), value_or_null.as_object()) {
        for (key, inner) in props {
            if let Some(held) = record.get(key)
                && !satisfies(inner, Some(held))
            {
                return false;
            }
        }
    }
    if let Some(Value::Array(parts)) = get(condition, "allOf")
        && !parts.iter().all(|p| satisfies(p, value))
    {
        return false;
    }
    if let Some(Value::Array(parts)) = get(condition, "anyOf")
        && !parts.iter().any(|p| satisfies(p, value))
    {
        return false;
    }
    if let Some(not) = get(condition, "not")
        && satisfies(not, value)
    {
        return false;
    }
    true
}

fn value_at<'a>(document: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut at = document;
    for part in path {
        at = match at {
            Value::Object(m) => m.get(part)?,
            Value::Array(l) => l.get(part.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(at)
}

/// The schemas a value at the path may follow: one, each branch of a choice, each part of an
/// `allOf`, and a conditional's `then` or `else`, decided against the document when it is given.
pub(crate) fn schemas_at(schema: &Value, path: &[String], document: Option<&Value>) -> Vec<Value> {
    let root = schema;
    fn expand(each: &Value, at: &[String], root: &Value, document: Option<&Value>, out: &mut Vec<Value>, depth: u32) {
        if depth > 12 {
            return;
        }
        let followed = follow_ref(each, root);
        let mut own = followed.as_object().cloned().unwrap_or_default();
        let all_of = own.remove("allOf");
        let test = own.remove("if");
        let then = own.remove("then");
        let otherwise = own.remove("else");
        let branches: Vec<Value> = ["anyOf", "oneOf"]
            .iter()
            .filter_map(|k| own.get(*k).and_then(Value::as_array).cloned())
            .flatten()
            .collect();
        out.push(Value::Object(own));
        for branch in &branches {
            expand(branch, at, root, document, out, depth + 1);
        }
        for part in all_of.as_ref().and_then(Value::as_array).into_iter().flatten() {
            expand(part, at, root, document, out, depth + 1);
        }
        if let Some(test) = test.filter(Value::is_object) {
            let holds = document.map(|d| satisfies(&test, value_at(d, at)));
            if holds != Some(false)
                && let Some(then) = then.as_ref().filter(|v| v.is_object())
            {
                expand(then, at, root, document, out, depth + 1);
            }
            if holds != Some(true)
                && let Some(otherwise) = otherwise.as_ref().filter(|v| v.is_object())
            {
                expand(otherwise, at, root, document, out, depth + 1);
            }
        }
    }
    let mut current = Vec::new();
    expand(schema, &[], root, document, &mut current, 0);
    for depth in 0..path.len() {
        let part = &path[depth];
        let at = &path[..=depth];
        let mut next = Vec::new();
        for each in &current {
            let picked: Option<Value> = if let Ok(index) = part.parse::<usize>()
                && (get(each, "items").is_some() || get(each, "prefixItems").is_some())
            {
                get(each, "prefixItems")
                    .and_then(Value::as_array)
                    .and_then(|p| p.get(index))
                    .or_else(|| get(each, "items"))
                    .cloned()
            } else if let Some(named) = get(each, "properties").and_then(|p| p.get(part)) {
                Some(named.clone())
            } else {
                get(each, "additionalProperties").filter(|a| a.is_object()).cloned()
            };
            if let Some(picked) = picked {
                expand(&picked, at, root, document, &mut next, 0);
            }
        }
        current = next;
    }
    current
}

/// An option as offered and put in.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SchemaOption {
    pub label: String,
    pub detail: Option<String>,
    pub info: Option<String>,
    pub kind: &'static str,
    pub boost: i32,
    pub deprecated: bool,
    /// The text put in.
    pub insert: String,
    /// Offsets into `insert` to select once it is in.
    pub select: (usize, usize),
    /// A snippet's fields as offsets into `insert`, reached in turn with Tab.
    pub fields: Vec<(usize, usize)>,
}

fn quote_yaml(value: &Value) -> String {
    match value {
        Value::String(s)
            if !s.is_empty()
                && s.chars()
                    .next()
                    .is_some_and(|c| c.is_alphanumeric() || "_./@-".contains(c))
                && s.chars().all(|c| c.is_alphanumeric() || " _./@-".contains(c))
                && yaml_scalar(s).is_string() =>
        {
            s.clone()
        }
        other => literal(other),
    }
}

fn write_value(value: &Value, json: bool) -> String {
    if json { literal(value) } else { quote_yaml(value) }
}

/// Text with fields: pieces of plain text and fields, built into one string and its field offsets.
#[derive(Default)]
struct Snippet {
    text: String,
    fields: Vec<(usize, usize)>,
}

impl Snippet {
    fn push(&mut self, s: &str) {
        self.text.push_str(s);
    }
    fn field(&mut self, s: &str) {
        let from = self.text.len();
        self.text.push_str(s);
        self.fields.push((from, self.text.len()));
    }
}

/// The value a required key is put in with: its default, its constant or first choice, or a field
/// for the type it takes.
fn required_value(property: Option<&Value>, root: &Value, json: bool, out: &mut Snippet) {
    let resolved = property.map(|p| resolve(p, root)).unwrap_or(Value::Null);
    let given = get(&resolved, "default")
        .or_else(|| get(&resolved, "const"))
        .or_else(|| get(&resolved, "enum").and_then(Value::as_array).and_then(|l| l.first()));
    if let Some(given) = given {
        out.field(&write_value(given, json));
        return;
    }
    let kinds = types_of(&resolved);
    match kinds.as_slice() {
        [k] if k == "string" => {
            if json {
                out.push("\"");
                out.field("");
                out.push("\"");
            } else {
                out.field("");
            }
        }
        [k] if k == "number" || k == "integer" => out.field("0"),
        [k] if k == "boolean" => out.field("false"),
        [k] if k == "object" => {
            out.push("{");
            out.field("");
            out.push("}");
        }
        [k] if k == "array" => {
            out.push("[");
            out.field("");
            out.push("]");
        }
        [k] if k == "null" => out.push("null"),
        _ => out.field(""),
    }
}

/// A key whose object has `required` keys, put in whole with each required key and a field for its
/// value; `None` for an object that requires nothing. `column` is where the key starts on its line
/// and `indent` the line's own indent.
fn required_snippet(
    name: &str,
    property: &Value,
    root: &Value,
    json: bool,
    column: usize,
    indent: usize,
) -> Option<Snippet> {
    let resolved = resolve(property, root);
    let kinds = types_of(&resolved);
    let mut required: Vec<String> = Vec::new();
    for key in get(&resolved, "required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(k) = key.as_str()
            && !required.iter().any(|r| r == k)
        {
            required.push(k.to_owned());
        }
    }
    if kinds.len() != 1 || kinds[0] != "object" || required.is_empty() {
        return None;
    }
    let props = get(&resolved, "properties");
    let mut out = Snippet::default();
    if json {
        let pad = " ".repeat(indent);
        out.push(&format!("{}: {{\n", literal(&Value::String(name.to_owned()))));
        for (i, key) in required.iter().enumerate() {
            out.push(&format!("{pad}  {}: ", literal(&Value::String(key.clone()))));
            required_value(props.and_then(|p| p.get(key)), root, json, &mut out);
            out.push(if i + 1 < required.len() { ",\n" } else { "\n" });
        }
        out.push(&format!("{pad}}}"));
    } else {
        let pad = " ".repeat(column + 2);
        out.push(&format!("{name}:"));
        for key in &required {
            out.push(&format!("\n{pad}{key}: "));
            required_value(props.and_then(|p| p.get(key)), root, json, &mut out);
        }
    }
    Some(out)
}

#[allow(clippy::too_many_arguments)]
fn key_option(
    name: &str,
    property: &Value,
    root: &Value,
    json: bool,
    column: usize,
    required: bool,
    words: &CodeEditorWords,
    indent: usize,
) -> SchemaOption {
    let resolved = resolve(property, root);
    let kinds = types_of(&resolved);
    let deprecated = get(&resolved, "deprecated") == Some(&Value::Bool(true));
    let mut option = SchemaOption {
        label: name.to_owned(),
        detail: Some(if deprecated {
            words.deprecated.clone()
        } else {
            type_label(property, root)
        }),
        info: get(&resolved, "description").and_then(Value::as_str).map(str::to_owned),
        kind: "property",
        boost: if deprecated { -99 } else { i32::from(required) },
        deprecated,
        insert: String::new(),
        select: (0, 0),
        fields: Vec::new(),
    };
    let default = get(&resolved, "default");
    if default.is_none()
        && let Some(snippet) = required_snippet(name, property, root, json, column, indent)
    {
        option.select = snippet
            .fields
            .first()
            .copied()
            .unwrap_or((snippet.text.len(), snippet.text.len()));
        option.insert = snippet.text;
        option.fields = snippet.fields;
        return option;
    }
    let set = |option: &mut SchemaOption, insert: String, select: (usize, usize)| {
        option.insert = insert;
        option.select = select;
    };
    if json {
        let head = format!("{}: ", literal(&Value::String(name.to_owned())));
        if let Some(default) = default {
            let value = literal(default);
            let len = head.len();
            set(&mut option, format!("{head}{value}"), (len, len + value.len()));
            return option;
        }
        let empty = match kinds.as_slice() {
            [k] if k == "object" => Some("{}"),
            [k] if k == "array" => Some("[]"),
            [k] if k == "string" && get(&resolved, "enum").is_none() => Some("\"\""),
            _ => None,
        };
        let len = head.len();
        match empty {
            Some(e) => set(&mut option, format!("{head}{e}"), (len + 1, len + 1)),
            None => set(&mut option, head, (len, len)),
        }
        return option;
    }
    let head = format!("{name}:");
    if let Some(default) = default {
        let value = quote_yaml(default);
        let len = head.len() + 1;
        set(&mut option, format!("{head} {value}"), (len, len + value.len()));
        return option;
    }
    let pad = " ".repeat(column + 2);
    let insert = match kinds.as_slice() {
        [k] if k == "object" => format!("{head}\n{pad}"),
        [k] if k == "array" => format!("{head}\n{pad}- "),
        _ => format!("{head} "),
    };
    let len = insert.len();
    set(&mut option, insert, (len, len));
    option
}

/// The property names and values a schema allows where the caret is.
#[allow(clippy::too_many_arguments)]
pub(crate) fn options(
    schema: &Value,
    json: bool,
    slot: &Slot,
    present: &[String],
    column: usize,
    document: Option<&Value>,
    words: &CodeEditorWords,
    indent: usize,
) -> Vec<SchemaOption> {
    if slot.at_key {
        let mut seen: Vec<String> = present.to_vec();
        let mut out = Vec::new();
        for each in schemas_at(schema, &slot.path, document) {
            let required: Vec<&str> = get(&each, "required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            if let Some(Value::Object(props)) = get(&each, "properties") {
                for (name, property) in props {
                    if seen.contains(name) {
                        continue;
                    }
                    seen.push(name.clone());
                    out.push(key_option(
                        name,
                        property,
                        schema,
                        json,
                        column,
                        required.contains(&name.as_str()),
                        words,
                        indent,
                    ));
                }
            }
        }
        // required first, deprecated last, each kept in the schema's order
        out.sort_by_key(|o| -o.boost);
        return out;
    }
    let path = match &slot.key {
        Some(key) => with(&slot.path, key.clone()),
        None => slot.path.clone(),
    };
    let mut values: Vec<Value> = Vec::new();
    for each in schemas_at(schema, &path, document) {
        let found: Vec<Value> = if let Some(c) = get(&each, "const") {
            vec![c.clone()]
        } else if let Some(Value::Array(list)) = get(&each, "enum") {
            list.clone()
        } else if types_of(&each).iter().any(|t| t == "boolean") {
            vec![Value::Bool(true), Value::Bool(false)]
        } else {
            Vec::new()
        };
        for v in found {
            if !values.iter().any(|x| same(x, &v)) {
                values.push(v);
            }
        }
    }
    values
        .into_iter()
        .map(|value| {
            let written = write_value(&value, json);
            let len = written.len();
            SchemaOption {
                label: match &value {
                    Value::String(s) => s.clone(),
                    _ => written.clone(),
                },
                detail: None,
                info: None,
                kind: "enum",
                boost: 0,
                deprecated: false,
                insert: written,
                select: (len, len),
                fields: Vec::new(),
            }
        })
        .collect()
}

/// What the schema says of the key under a position: its type and its description, and the key's range.
pub(crate) fn hover(
    schema: &Value,
    read: &ReadDocument,
    pos: usize,
) -> Option<(String, Option<String>, (usize, usize))> {
    let (path, key) = read
        .spans
        .iter()
        .filter_map(|(p, span): (&String, &Span)| span.key.filter(|k| k.0 <= pos && pos <= k.1).map(|k| (p.clone(), k)))
        .min_by_key(|(_, k)| k.1 - k.0)?;
    let parts: Vec<String> = path
        .split('/')
        .skip(1)
        .map(|p| p.replace("~1", "/").replace("~0", "~"))
        .collect();
    let found = schemas_at(schema, &parts, read.value.as_ref());
    let typed = found.iter().find(|each| {
        ["type", "anyOf", "oneOf", "enum", "const", "properties"]
            .iter()
            .any(|k| get(each, k).is_some())
    });
    let description = found
        .iter()
        .find_map(|each| get(each, "description").and_then(Value::as_str).map(str::to_owned));
    if typed.is_none() && description.is_none() {
        return None;
    }
    let kind = type_label(typed.unwrap_or(&Value::Object(Map::new())), schema);
    Some((kind, description, key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::code_editor::document::{read_json, read_yaml};
    use serde_json::json;

    fn pipeline() -> Value {
        json!({
            "type": "object",
            "required": ["name", "steps"],
            "additionalProperties": false,
            "properties": {
                "name": {"type": "string", "pattern": "^[a-z][a-z0-9-]*$", "description": "The pipeline's name."},
                "schedule": {"type": "string", "pattern": "^\\S+( \\S+){4}$"},
                "project": {"enum": ["harbor", "juniper", "larkspur"]},
                "steps": {"type": "array", "minItems": 1, "items": {"$ref": "#/$defs/step"}}
            },
            "$defs": {
                "step": {
                    "type": "object",
                    "required": ["id", "kind"],
                    "additionalProperties": false,
                    "properties": {
                        "id": {"type": "string"},
                        "kind": {"enum": ["fetch", "upload"]},
                        "retries": {"type": "integer", "minimum": 0, "maximum": 5, "default": 0},
                        "old": {"type": "string", "deprecated": true}
                    }
                }
            }
        })
    }

    fn messages(text: &str, yaml: bool) -> Vec<(String, String)> {
        let read = if yaml { read_yaml(text) } else { read_json(text) };
        let words = CodeEditorWords::default();
        findings(
            text,
            &read,
            &pipeline(),
            if yaml { "YAML" } else { "JSON" },
            &words,
            None,
        )
        .into_iter()
        .map(|(f, _)| (text[f.from..f.to].to_owned(), f.message))
        .collect()
    }

    #[test]
    fn a_valid_document_has_no_problems() {
        assert!(
            messages(
                r#"{"name": "nightly-export", "steps": [{"id": "fetch", "kind": "fetch"}]}"#,
                false
            )
            .is_empty()
        );
        assert!(messages("name: nightly-export\nsteps:\n  - id: fetch\n    kind: fetch\n", true).is_empty());
    }

    #[test]
    fn json_problems_sit_on_their_values_and_keys() {
        let found = messages(
            r#"{"name": "Nightly", "project": "tidal", "retries": 1, "steps": [{"id": "fetch", "kind": "fetch", "retries": 9}]}"#,
            false,
        );
        assert!(
            found.contains(&("\"Nightly\"".into(), "Must match ^[a-z][a-z0-9-]*$".into())),
            "{found:?}"
        );
        assert!(found.contains(&(
            "\"tidal\"".into(),
            "Expected one of \"harbor\", \"juniper\", \"larkspur\"".into()
        )));
        assert!(found.contains(&("\"retries\"".into(), "retries is not a known property".into())));
        assert!(found.contains(&("9".into(), "Must be at most 5".into())));
    }

    #[test]
    fn yaml_problems_name_what_is_missing_and_wrong() {
        let found = messages(
            "name: nightly-export\nsteps:\n  - id: fetch\n    old: x\n  - kind: upload\n    id: 3\n",
            true,
        );
        assert!(found.contains(&("3".into(), "Expected a string".into())), "{found:?}");
        assert!(found.contains(&("id: fetch".into(), "Missing the property kind".into())));
        assert!(
            found
                .iter()
                .any(|(_, m)| m == "Deprecated: still accepted, and may be removed in a later version")
        );
        let empty = messages("name: x\nsteps: []\n", true);
        assert!(
            empty.contains(&("steps".into(), "Needs at least 1 item".into())),
            "{empty:?}"
        );
        let missing = messages("name: x\n", true);
        assert!(missing.iter().any(|(_, m)| m == "Missing the property steps"));
    }

    #[test]
    fn broken_text_is_one_syntax_problem() {
        let found = messages("{\"name\": }", false);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, "Not valid JSON");
    }

    #[test]
    fn a_choice_reports_the_branch_that_fits() {
        let schema = json!({
            "oneOf": [
                {"type": "object", "properties": {"kind": {"const": "fetch"}, "url": {"type": "string"}}, "required": ["kind", "url"]},
                {"type": "object", "properties": {"kind": {"const": "upload"}, "bucket": {"type": "string"}}, "required": ["kind", "bucket"]}
            ]
        });
        let issues = validate(&schema, &json!({"kind": "upload"}));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, IssueKind::Required);
        assert_eq!(issues[0].path, vec!["bucket".to_owned()]);
        let wrong = validate(&schema, &json!({"kind": "notify"}));
        assert_eq!(wrong[0].kind, IssueKind::Enum(vec![json!("fetch"), json!("upload")]));
    }

    #[test]
    fn keys_complete_with_required_first_and_snippets() {
        let schema = pipeline();
        let text = "{\n  \"steps\": [\n    {}\n  ]\n}";
        let pos = text.find("{}").unwrap() + 1;
        let slot = json_slot(text, pos).unwrap();
        assert_eq!(slot.path, vec!["steps".to_owned(), "0".to_owned()]);
        assert!(slot.at_key);
        let words = CodeEditorWords::default();
        let offered = options(&schema, true, &slot, &[], 5, None, &words, 4);
        let labels: Vec<&str> = offered.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(&labels[..2], &["id", "kind"]);
        assert_eq!(labels.last(), Some(&"old"));
        let retries = offered.iter().find(|o| o.label == "retries").unwrap();
        assert_eq!(retries.insert, "\"retries\": 0");
        assert_eq!(&retries.insert[retries.select.0..retries.select.1], "0");

        let top = json_slot("{\"name\": \"x\", }", 14).unwrap();
        let keys = json_keys("{\"name\": \"x\", }", 14);
        let offered = options(&schema, true, &top, &keys, 14, None, &words, 0);
        let steps = offered.iter().find(|o| o.label == "steps").unwrap();
        assert_eq!(steps.insert, "\"steps\": []");
        assert!(!offered.iter().any(|o| o.label == "name"));
    }

    #[test]
    fn yaml_values_complete_from_the_enum() {
        let text = "name: x\nsteps:\n  - id: a\n    kind: ";
        let slot = yaml_slot(text, text.len());
        assert_eq!(slot.path, vec!["steps".to_owned(), "0".to_owned()]);
        assert_eq!(slot.key.as_deref(), Some("kind"));
        let words = CodeEditorWords::default();
        let offered = options(&pipeline(), false, &slot, &[], 4, None, &words, 4);
        let labels: Vec<&str> = offered.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, ["fetch", "upload"]);
    }

    #[test]
    fn a_required_object_goes_in_as_a_snippet() {
        let schema = json!({"type": "object", "properties": {"step": {"$ref": "#/$defs/step"}}, "$defs": pipeline()["$defs"].clone()});
        let words = CodeEditorWords::default();
        let slot = yaml_slot("st", 2);
        let offered = options(&schema, false, &slot, &[], 0, None, &words, 0);
        let step = &offered[0];
        assert_eq!(step.insert, "step:\n  id: \n  kind: fetch");
        assert_eq!(step.fields.len(), 2);
        assert_eq!(&step.insert[step.fields[1].0..step.fields[1].1], "fetch");
    }

    #[test]
    fn hover_says_a_key_s_type_and_description() {
        let text = "name: x\n";
        let read = read_yaml(text);
        let (kind, description, _) = hover(&pipeline(), &read, 1).unwrap();
        assert_eq!(kind, "string");
        assert_eq!(description.as_deref(), Some("The pipeline's name."));
    }
}
