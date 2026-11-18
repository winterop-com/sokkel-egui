//! Reading a JSON Schema for a form, after sokkel-ui's `schema/read.ts` and `schema/issues.ts`:
//! the schema a field is drawn from, the kind of control it takes, the value a form starts with,
//! and where a value sits in a document.

use serde_json::{Map, Value};

use super::super::code_editor::schema::{literal, resolve};

/// Where a value sits in a document: the keys and indexes from the root down, an index written
/// as its digits.
pub type SchemaPath = Vec<String>;

/// A schema's key, if the schema is an object.
pub(crate) fn get<'a>(schema: &'a Value, key: &str) -> Option<&'a Value> {
    schema.as_object()?.get(key)
}

/// A schema's string key.
pub(crate) fn text<'a>(schema: &'a Value, key: &str) -> Option<&'a str> {
    get(schema, key).and_then(Value::as_str)
}

/// Whether a schema's key is `true`.
pub(crate) fn flag(schema: &Value, key: &str) -> bool {
    get(schema, key) == Some(&Value::Bool(true))
}

/// A path as one string, a JSON Pointer: `/schedule/0/at`. The root is the empty string.
pub fn pointer(path: &[String]) -> String {
    path.iter()
        .map(|part| format!("/{}", part.replace('~', "~0").replace('/', "~1")))
        .collect()
}

/// A JSON Pointer read back into a path.
pub(crate) fn path_of(at: &str) -> SchemaPath {
    if at.is_empty() {
        return Vec::new();
    }
    at[1..]
        .split('/')
        .map(|part| part.replace("~1", "/").replace("~0", "~"))
        .collect()
}

/// The value at a path, or `None` where the path runs out.
pub(crate) fn value_at<'v>(value: &'v Value, path: &[String]) -> Option<&'v Value> {
    let mut current = value;
    for part in path {
        current = match current {
            Value::Object(map) => map.get(part)?,
            Value::Array(list) => list.get(part.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

/// The value with the one at a path replaced; `None` takes the key out of its object, and leaves
/// a list's item empty (`null`), as an unset item.
pub(crate) fn set_at(value: &mut Value, path: &[String], next: Option<Value>) {
    let Some((head, rest)) = path.split_first() else {
        *value = next.unwrap_or(Value::Null);
        return;
    };
    if let Value::Array(list) = value
        && let Ok(index) = head.parse::<usize>()
    {
        while list.len() <= index {
            list.push(Value::Null);
        }
        if rest.is_empty() {
            list[index] = next.unwrap_or(Value::Null);
        } else {
            set_at(&mut list[index], rest, next);
        }
        return;
    }
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    let Value::Object(map) = value else { return };
    if rest.is_empty() {
        match next {
            // taken out where it stood, so the keys after it keep their order
            None => {
                map.shift_remove(head);
            }
            Some(next) => {
                map.insert(head.clone(), next);
            }
        }
    } else {
        let child = map.entry(head.clone()).or_insert(Value::Null);
        set_at(child, rest, next);
    }
}

/// Whether a value counts as given: there, not `null`, and not an empty string.
pub(crate) fn given(value: Option<&Value>) -> bool {
    matches!(value, Some(v) if !v.is_null() && v.as_str() != Some(""))
}

/// Two values as a schema compares them: numbers by their worth, so 2 and 2.0 are the same.
pub(crate) fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b)),
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

/// A value as a field shows it in words: a string as it is, anything else as JSON.
pub(crate) fn shown(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => literal(other),
    }
}

/// A number as a JSON value: whole numbers as integers, so 3 is written `3`, not `3.0`.
pub(crate) fn number_value(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        Value::from(n as i64)
    } else {
        serde_json::Number::from_f64(n).map_or(Value::Null, Value::Number)
    }
}

/// The schema's types without `null`, and whether `null` is allowed, by `type` or by a branch of
/// `anyOf` or `oneOf` that is `{type: null}`.
pub(crate) fn types(schema: &Value) -> (Vec<String>, bool) {
    let all: Vec<String> = match get(schema, "type") {
        Some(Value::String(t)) => vec![t.clone()],
        Some(Value::Array(list)) => list.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
        _ => Vec::new(),
    };
    let is_null = |v: &Value| text(v, "type") == Some("null");
    let variants = ["anyOf", "oneOf"]
        .iter()
        .filter_map(|k| get(schema, k).and_then(Value::as_array))
        .flatten()
        .any(is_null);
    let nullable = all.iter().any(|t| t == "null") || variants;
    (all.into_iter().filter(|t| t != "null").collect(), nullable)
}

/// The schema a field is drawn from: its `$ref` followed and `allOf` merged, and
/// `anyOf: [X, {type: null}]`, the shape a nullable field is often written in, read as X.
pub(crate) fn field_schema(schema: &Value, root: &Value) -> Value {
    let resolved = resolve(schema, root);
    let Some(variants) = get(&resolved, "anyOf")
        .or_else(|| get(&resolved, "oneOf"))
        .and_then(Value::as_array)
    else {
        return resolved;
    };
    let real: Vec<&Value> = variants.iter().filter(|v| text(v, "type") != Some("null")).collect();
    if real.len() != 1 {
        return resolved;
    }
    let mut merged = resolve(real[0], root).as_object().cloned().unwrap_or_default();
    if let Some(own) = resolved.as_object() {
        for (key, value) in own {
            if key != "anyOf" && key != "oneOf" {
                merged.insert(key.clone(), value.clone());
            }
        }
    }
    Value::Object(merged)
}

const SCALARS: [&str; 4] = ["integer", "number", "boolean", "string"];

/// The scalar types a field takes when it takes more than one and nothing else, as `anyOf` of
/// `{type: string}` and `{type: integer}` with no `type` of its own, or `type: [string, integer]`;
/// `None` for any other field. `null` is left out.
pub fn scalar_union(schema: &Value, root: &Value) -> Option<Vec<String>> {
    let variants = get(schema, "anyOf")
        .or_else(|| get(schema, "oneOf"))
        .and_then(Value::as_array);
    if get(schema, "enum").is_some() || (variants.is_some() && get(schema, "type").is_some()) {
        return None;
    }
    let branches: Vec<Value> = variants.cloned().unwrap_or_else(|| vec![schema.clone()]);
    let mut kinds: Vec<String> = Vec::new();
    for branch in &branches {
        let at = resolve(branch, root);
        // a branch of set values counts as their types: `30` or `"none"`
        let values = get(&at, "const")
            .map(|c| vec![c.clone()])
            .or_else(|| get(&at, "enum").and_then(Value::as_array).cloned());
        let found: Vec<String> = match values {
            Some(values) => values
                .iter()
                .filter(|v| !v.is_null())
                .map(|v| {
                    match v {
                        Value::String(_) => "string",
                        Value::Bool(_) => "boolean",
                        Value::Number(_) => "number",
                        _ => "object",
                    }
                    .to_owned()
                })
                .collect(),
            None => types(&at).0,
        };
        for kind in found {
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
    }
    (kinds.len() > 1 && kinds.iter().all(|k| SCALARS.contains(&k.as_str()))).then_some(kinds)
}

/// Whether text reads as a decimal number: digits with a point and an exponent, a sign before.
fn decimal(text: &str) -> bool {
    let body = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (mantissa, exponent) = match body.find(['e', 'E']) {
        Some(at) => (&body[..at], Some(&body[at + 1..])),
        None => (body, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok = digits(whole) && digits(fraction) && !(whole.is_empty() && fraction.is_empty());
    let exponent_ok = exponent.is_none_or(|e| {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        !e.is_empty() && digits(e)
    });
    mantissa_ok && exponent_ok
}

/// Text typed for a field that takes several scalar types, read as the narrowest it parses as: an
/// integer, then a number, then true or false, else the text itself; `None` for nothing typed.
pub fn read_scalar(text: &str, allowed: &[String]) -> Option<Value> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let takes = |kind: &str| allowed.iter().any(|a| a == kind);
    let body = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
    if takes("integer") && !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit()) {
        return trimmed.parse::<f64>().ok().map(number_value);
    }
    if takes("number") && decimal(trimmed) {
        return trimmed.parse::<f64>().ok().map(number_value);
    }
    if takes("boolean") && (trimmed == "true" || trimmed == "false") {
        return Some(Value::Bool(trimmed == "true"));
    }
    Some(Value::String(text.to_owned()))
}

/// A form's value with every nested object in place, and the schema defaults a field cannot show
/// itself: an object's or a list's, and a required field's. An optional field with a plain default
/// is left unset and shows the default as what applies. A required boolean without a default, and
/// a list's boolean item, starts as false, as its switch reads.
pub fn initial_value(schema: &Value) -> Value {
    initial(schema, schema, true).unwrap_or(Value::Null)
}

pub(crate) fn initial(schema: &Value, root: &Value, keep: bool) -> Option<Value> {
    let at = field_schema(schema, root);
    if let Some(default) = get(&at, "default")
        && (keep || default.is_object() || default.is_array())
    {
        return Some(default.clone());
    }
    if let Some(constant) = get(&at, "const") {
        return Some(constant.clone());
    }
    if let Some(Value::Object(properties)) = get(&at, "properties") {
        let required: Vec<&str> = get(&at, "required")
            .and_then(Value::as_array)
            .map(|r| r.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let mut out = Map::new();
        for (key, child) in properties {
            if let Some(value) = initial(child, root, required.contains(&key.as_str())) {
                out.insert(key.clone(), value);
            }
        }
        return Some(Value::Object(out));
    }
    let main = types(&at).0;
    if main.first().map(String::as_str) == Some("array") {
        return Some(Value::Array(Vec::new()));
    }
    // a required switch reads off, so it starts as false rather than unset
    if keep && main.len() == 1 && main[0] == "boolean" && get(&at, "enum").is_none() {
        return Some(Value::Bool(false));
    }
    None
}

/// What a list's new or emptied item holds: its initial value, else `""` for a plain string, so a
/// list never holds nothing where a string goes. A number or a choice without a default stays unset.
pub fn blank_item(items: &Value, root: &Value) -> Option<Value> {
    if let Some(start) = initial(items, root, true) {
        return Some(start);
    }
    let at = field_schema(items, root);
    let main = types(&at).0;
    (main.len() == 1 && main[0] == "string" && get(&at, "enum").is_none() && scalar_union(&at, root).is_none())
        .then(|| Value::String(String::new()))
}

/// The kind of control a field takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Its properties in a fieldset.
    Object,
    /// A list of objects, each a box of fields.
    Records,
    /// A list of scalars, each a control.
    List,
    /// A set of choices, a checkbox each.
    Choices,
    /// An object of any keys whose values are each one scalar: a key and a value per row.
    Map,
    Enum,
    Boolean,
    Number,
    /// A choice of scalar types in one text box.
    Scalars,
    Text,
    LongText,
    /// Anything else, edited as JSON.
    Json,
}

pub(crate) fn kind_of(at: &Value, root: &Value) -> Kind {
    let main = types(at).0;
    let first = main.first().map(String::as_str);
    if get(at, "properties").is_some() {
        return Kind::Object;
    }
    if first == Some("array")
        && let Some(items) = get(at, "items")
    {
        let items = field_schema(items, root);
        if get(&items, "enum").is_some() {
            return Kind::Choices;
        }
        if get(&items, "properties").is_some() {
            return Kind::Records;
        }
        return match kind_of(&items, root) {
            Kind::Json | Kind::Object | Kind::Map => Kind::Json,
            _ => Kind::List,
        };
    }
    // an object of any keys whose values are each one scalar, such as headers or environment variables
    if let Some(extra @ Value::Object(_)) = get(at, "additionalProperties")
        && (main.is_empty() || first == Some("object"))
    {
        return match kind_of(&field_schema(extra, root), root) {
            Kind::Enum | Kind::Boolean | Kind::Number | Kind::Scalars | Kind::Text => Kind::Map,
            _ => Kind::Json,
        };
    }
    if get(at, "enum").is_some() {
        return Kind::Enum;
    }
    if scalar_union(at, root).is_some() {
        return Kind::Scalars;
    }
    if main.len() > 1 {
        return Kind::Json;
    }
    match first {
        Some("boolean") => Kind::Boolean,
        Some("number" | "integer") => Kind::Number,
        Some("string") => {
            let long = get(at, "maxLength").and_then(Value::as_u64).unwrap_or(0) > 200
                || text(at, "format") == Some("markdown")
                || get(at, "contentMediaType").is_some();
            if long { Kind::LongText } else { Kind::Text }
        }
        _ => Kind::Json,
    }
}

/// Whether a field takes half a row: numbers, choices, switches, dates and short text.
pub(crate) fn short(at: &Value, kind: Kind) -> bool {
    match kind {
        Kind::Number | Kind::Enum | Kind::Boolean | Kind::Scalars => true,
        Kind::Text => {
            matches!(text(at, "format"), Some("date" | "email"))
                || get(at, "maxLength").and_then(Value::as_u64).is_some_and(|n| n <= 64)
                || get(at, "pattern").is_some()
        }
        _ => false,
    }
}

/// The schema default an unset field shows as what applies, or `None` when it has none to show.
pub(crate) fn shown_default<'a>(at: &'a Value, value: Option<&Value>) -> Option<&'a Value> {
    if value.is_some_and(|v| !v.is_null()) {
        return None;
    }
    get(at, "default").filter(|d| !d.is_null())
}

/// Whether the form's own control shows an unset field's default in its place, so it need not be
/// said again under it.
pub(crate) fn shows_default(at: &Value, kind: Kind, value: Option<&Value>) -> bool {
    shown_default(at, value).is_some()
        && !flag(at, "writeOnly")
        && matches!(
            kind,
            Kind::Enum | Kind::Number | Kind::LongText | Kind::Scalars | Kind::Text
        )
}

/// The words of a select's empty choice, which unsets the field: named by the default it then shows
/// on an optional field with one, else `none` where unset is allowed, else `None` for no such choice.
pub fn unset_choice(at: &Value, required: bool, none: &str, use_default: fn(&str) -> String) -> Option<String> {
    let fallback = get(at, "default").filter(|d| !d.is_null());
    if let Some(fallback) = fallback
        && !required
    {
        return Some(use_default(&shown(fallback)));
    }
    (types(at).1 || get(at, "default").is_none()).then(|| none.to_owned())
}

/// The schema that describes the value at a path.
pub(crate) fn schema_at(schema: &Value, path: &[String], root: &Value) -> Value {
    let mut at = field_schema(schema, root);
    for part in path {
        let index = part.parse::<usize>().is_ok() && get(&at, "items").is_some();
        let next = if index {
            get(&at, "items").cloned()
        } else {
            get(&at, "properties")
                .and_then(|p| p.get(part))
                .cloned()
                .or_else(|| get(&at, "additionalProperties").filter(|a| a.is_object()).cloned())
        };
        let Some(next) = next else {
            return Value::Object(Map::new());
        };
        at = field_schema(&next, root);
    }
    at
}

/// A field's name from its schema's title down the path, or its key when the schema gives none.
pub(crate) fn title_at(schema: &Value, path: &[String]) -> String {
    let at = schema_at(schema, path, schema);
    text(&at, "title")
        .map(str::to_owned)
        .unwrap_or_else(|| path.last().cloned().unwrap_or_default())
}

/// The label a field is drawn with: its title, else its key; the key beside a title that does not
/// already say it.
pub(crate) fn label_of(at: &Value, name: &str) -> (String, Option<String>) {
    let Some(title) = text(at, "title") else {
        return (name.to_owned(), None);
    };
    let plain = |s: &str| -> String { s.to_lowercase().chars().filter(char::is_ascii_alphanumeric).collect() };
    let hint = (plain(title) != plain(name)).then(|| name.to_owned());
    (title.to_owned(), hint)
}

/// A read-only schema's own properties marked read-only too, so a locked object or item locks what
/// it holds.
pub(crate) fn lock(schema: &Value) -> Value {
    let mut out = schema.as_object().cloned().unwrap_or_default();
    out.insert("readOnly".into(), Value::Bool(true));
    if let Some(Value::Object(properties)) = out.get_mut("properties") {
        for child in properties.values_mut() {
            if let Value::Object(child) = child {
                child.insert("readOnly".into(), Value::Bool(true));
            } else {
                *child = serde_json::json!({ "readOnly": true });
            }
        }
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn kinds(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_choice_of_scalar_types_is_told_from_other_choices() {
        let root = json!({});
        let union = |s: Value| scalar_union(&s, &root);
        assert_eq!(
            union(json!({"anyOf": [{"type": "string"}, {"type": "integer"}]})),
            Some(kinds(&["string", "integer"]))
        );
        assert_eq!(
            union(json!({"type": ["integer", "boolean", "null"]})),
            Some(kinds(&["integer", "boolean"]))
        );
        assert_eq!(
            union(json!({"anyOf": [{"type": "integer"}, {"enum": ["none"]}]})),
            Some(kinds(&["integer", "string"]))
        );
        assert_eq!(union(json!({"anyOf": [{"type": "string"}, {"type": "object"}]})), None);
        assert_eq!(
            union(json!({"anyOf": [{"type": "string"}, {"type": "string", "pattern": "^a"}]})),
            None
        );
        assert_eq!(
            union(json!({"type": "string", "anyOf": [{"minLength": 1}, {"maxLength": 0}]})),
            None
        );
    }

    #[test]
    fn typed_text_reads_as_the_narrowest_type_it_parses_as() {
        let sizes = kinds(&["string", "integer"]);
        assert_eq!(read_scalar("16384", &sizes), Some(json!(16384)));
        assert_eq!(read_scalar("16KB", &sizes), Some(json!("16KB")));
        assert_eq!(read_scalar("1.5", &sizes), Some(json!("1.5")));
        assert_eq!(read_scalar("1.5", &kinds(&["string", "number"])), Some(json!(1.5)));
        assert_eq!(read_scalar("true", &kinds(&["string", "boolean"])), Some(json!(true)));
        assert_eq!(read_scalar("true", &sizes), Some(json!("true")));
        assert_eq!(read_scalar("  ", &sizes), None);
    }

    #[test]
    fn a_field_left_at_its_default_is_left_out_unless_it_is_required() {
        let schema = json!({
            "type": "object",
            "required": ["code"],
            "properties": {
                "code": {"type": "string", "default": "nightly-export"},
                "priority": {"type": "string", "enum": ["low", "normal", "high"], "default": "normal"},
                "region": {"type": "string", "default": "europe"},
                "retries": {"type": "integer", "default": 3},
                "paused": {"type": "boolean", "default": true},
                "note": {"type": ["string", "null"], "default": null}
            }
        });
        assert_eq!(initial_value(&schema), json!({"code": "nightly-export"}));
    }

    #[test]
    fn a_required_boolean_without_a_default_starts_as_false() {
        let schema = json!({
            "type": "object",
            "required": ["paused"],
            "properties": {"paused": {"type": "boolean"}, "notify": {"type": "boolean"}}
        });
        assert_eq!(initial_value(&schema), json!({"paused": false}));
        assert_eq!(initial_value(&json!({"type": "boolean"})), json!(false));
    }

    #[test]
    fn a_key_an_object_already_has_is_a_key_like_any_other() {
        let schema = json!({
            "type": "object",
            "required": ["__proto__"],
            "properties": {"__proto__": {"type": "string", "default": "x"}}
        });
        assert_eq!(
            serde_json::to_string(&initial_value(&schema)).unwrap(),
            r#"{"__proto__":"x"}"#
        );
    }

    #[test]
    fn an_empty_row_in_a_list_of_text_holds_an_empty_string() {
        let root = json!({});
        assert_eq!(blank_item(&json!({"type": "string"}), &root), Some(json!("")));
        assert_eq!(
            blank_item(&json!({"type": "string", "default": "harbor"}), &root),
            Some(json!("harbor"))
        );
        assert_eq!(blank_item(&json!({"type": "boolean"}), &root), Some(json!(false)));
        assert_eq!(blank_item(&json!({"type": "integer"}), &root), None);
    }

    #[test]
    fn a_select_offers_a_choice_back_to_its_default_while_it_is_optional() {
        let words = |at: Value, required: bool| unset_choice(&at, required, "None", |v| format!("Default ({v})"));
        let method = json!({"type": "string", "enum": ["GET", "POST"], "default": "GET"});
        assert_eq!(words(method.clone(), false).as_deref(), Some("Default (GET)"));
        assert_eq!(
            words(json!({"type": "integer", "enum": [1, 2], "default": 2}), false).as_deref(),
            Some("Default (2)")
        );
        assert_eq!(words(method, true), None);
        assert_eq!(
            words(json!({"type": "string", "enum": ["GET", "POST"]}), false).as_deref(),
            Some("None")
        );
        assert_eq!(
            words(
                json!({"type": ["string", "null"], "enum": ["GET", null], "default": null}),
                false
            )
            .as_deref(),
            Some("None")
        );
    }

    #[test]
    fn fields_take_the_kind_of_control_the_web_gives_them() {
        let root = json!({});
        let kind = |s: Value| kind_of(&field_schema(&s, &root), &root);
        assert_eq!(kind(json!({"type": "string"})), Kind::Text);
        assert_eq!(kind(json!({"type": "string", "maxLength": 500})), Kind::LongText);
        assert_eq!(kind(json!({"enum": ["a", "b"]})), Kind::Enum);
        assert_eq!(kind(json!({"type": "integer"})), Kind::Number);
        assert_eq!(kind(json!({"type": "boolean"})), Kind::Boolean);
        assert_eq!(
            kind(json!({"anyOf": [{"type": "integer"}, {"type": "null"}]})),
            Kind::Number
        );
        assert_eq!(kind(json!({"type": "array", "items": {"type": "string"}})), Kind::List);
        assert_eq!(kind(json!({"type": "array", "items": {"enum": ["a"]}})), Kind::Choices);
        assert_eq!(
            kind(json!({"type": "array", "items": {"type": "object", "properties": {}}})),
            Kind::Records
        );
        assert_eq!(
            kind(json!({"type": "object", "additionalProperties": {"type": "integer"}})),
            Kind::Map
        );
        assert_eq!(
            kind(json!({"type": "object", "additionalProperties": {"type": "object", "properties": {"a": {}}}})),
            Kind::Json
        );
        assert_eq!(
            kind(json!({"anyOf": [{"type": "string"}, {"type": "object"}]})),
            Kind::Json
        );
    }

    #[test]
    fn a_value_is_set_and_unset_at_its_path() {
        let mut value = json!({"code": "x", "tags": ["a"]});
        set_at(&mut value, &["owner".into(), "name".into()], Some(json!("ada")));
        set_at(&mut value, &["tags".into(), "1".into()], Some(json!("b")));
        set_at(&mut value, &["code".into()], None);
        assert_eq!(value, json!({"tags": ["a", "b"], "owner": {"name": "ada"}}));
        assert_eq!(pointer(&["a/b".into(), "c~".into()]), "/a~1b/c~0");
        assert_eq!(path_of("/a~1b/c~0"), vec!["a/b".to_owned(), "c~".to_owned()]);
    }
}
