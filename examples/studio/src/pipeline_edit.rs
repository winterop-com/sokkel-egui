//! The edits the pipeline graph makes, as the web's `pipeline-edit.ts`, each written back into the
//! pipeline's YAML: a step added, removed, renamed or set, and a need added or taken away. Each
//! takes the text and answers with the new text, everything it does not touch written as it was.
//!
//! Where the web throws (a pipeline whose top is a scalar or a list given steps), the text is
//! answered as it was.

use serde_json::{Map, Value, json};

use crate::js::is_space;
use crate::pipeline::in_object;
use crate::yaml::{Document, Kind, Node, Pair, Scalar, parse};

/// Whether a text is a step's id, as the pipeline schema's `^[a-z][a-z0-9-]*$` has it.
pub fn is_step_id(text: &str) -> bool {
    crate::model::is_slug(text)
}

/// The `steps` list, made when the pipeline has none yet; none where the web throws.
fn step_list(doc: &mut Document) -> Option<&mut Node> {
    let found = doc.get("steps").is_some_and(Node::is_seq);
    if !found && !doc.set("steps", Node::seq(Vec::new())) {
        return None;
    }
    doc.contents.as_mut()?.get_mut("steps")
}

/// The maps of the `steps` list.
fn step_items(doc: &mut Document) -> Vec<&mut Node> {
    match doc
        .contents
        .as_mut()
        .filter(|node| node.is_map())
        .and_then(|node| node.get_mut("steps"))
    {
        Some(list) => list
            .items_mut()
            .map(|items| items.iter_mut().filter(|item| item.is_map()).collect())
            .unwrap_or_default(),
        None => Vec::new(),
    }
}

fn id_of(item: &Node) -> Option<&str> {
    item.get("id").and_then(Node::as_str)
}

fn needs_of(item: &Node) -> Vec<String> {
    item.get("needs")
        .filter(|node| node.is_seq())
        .map(|needs| {
            needs
                .items()
                .iter()
                .filter_map(|need| need.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Sets a step's needs, written in brackets; none takes the key out.
fn set_needs(item: &mut Node, needs: &[String]) {
    if needs.is_empty() {
        item.delete("needs");
        return;
    }
    let mut list = Node::seq(needs.iter().map(|need| Node::text(need)).collect());
    list.flow = Some(true);
    if item.has("needs") {
        item.set("needs", list);
    } else if let Some(pairs) = item.pairs_mut() {
        // A new `needs` goes after `kind`, where the pipelines write it.
        let at = pairs
            .iter()
            .position(|pair| pair.key.scalar_value() == Some(&Scalar::Str("kind".to_owned())));
        let pair = Pair {
            key: Node::text("needs"),
            value: list,
        };
        pairs.insert(at.map_or(pairs.len(), |at| at + 1), pair);
    }
}

/// A value as a node, its lists of plain values written in brackets as the pipelines write them.
fn made(value: &Value) -> Node {
    let mut node = Node::from_value(value);
    node.visit_seqs_mut(&mut |seq| {
        if seq.items().iter().all(Node::is_scalar) {
            seq.flow = Some(true);
        }
    });
    node
}

/// A value as `JSON.stringify` writes it, numbers as the web writes them.
fn json_text(value: &Value) -> String {
    match value {
        Value::Number(n) => {
            serde_json::to_string(&crate::yaml::js_number(n.as_f64().unwrap_or(f64::NAN))).unwrap_or_default()
        }
        Value::Array(items) => format!("[{}]", items.iter().map(json_text).collect::<Vec<_>>().join(",")),
        Value::Object(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(key, value)| format!(
                    "{}:{}",
                    serde_json::to_string(key).unwrap_or_default(),
                    json_text(value)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn scalar_of(value: &Value) -> Option<Scalar> {
    Some(match value {
        Value::Null => Scalar::Null,
        Value::Bool(b) => Scalar::Bool(*b),
        Value::Number(n) => Scalar::Num(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(text) => Scalar::Str(text.clone()),
        _ => return None,
    })
}

/// Sets `key` of `map` to `value`, changing only what differs: a map's keys one by one, a plain
/// value in its own quotes, a list kept in brackets when it was. None or an empty text takes the
/// key out.
fn assign(map: &mut Node, key: &str, value: Option<&Value>) {
    let Some(value) = value.filter(|value| value.as_str() != Some("")) else {
        map.delete(key);
        return;
    };
    let Some(node) = map.get_mut(key) else {
        map.set(key, made(value));
        return;
    };
    if let (true, Value::Object(entries)) = (node.is_map(), value) {
        let gone: Vec<String> = node
            .pairs()
            .iter()
            .filter_map(|pair| pair.key.as_str())
            .filter(|inner| !in_object(entries.keys().map(String::as_str), inner))
            .map(str::to_owned)
            .collect();
        for inner in gone {
            node.delete(&inner);
        }
        for (inner, next) in entries {
            assign(node, inner, Some(next));
        }
        return;
    }
    if let (Kind::Scalar { value: old, .. }, Some(next)) = (&mut node.kind, scalar_of(value)) {
        if *old != next {
            *old = next;
        }
        return;
    }
    if let (true, Value::Array(items)) = (node.is_seq(), value) {
        if json_text(&node.to_js()) == json_text(value) {
            return;
        }
        let mut list = made(value);
        list.flow = Some(node.flow == Some(true) && items.iter().all(|item| !item.is_array() && !item.is_object()));
        map.set(key, list);
        return;
    }
    map.set(key, made(value));
}

/// Applies `change` to the pipeline's document; the text as it was when it does not read, or
/// when the change finds the web would throw.
fn edit(text: &str, change: impl FnOnce(&mut Document) -> Option<()>) -> String {
    let Some(mut doc) = parse(text) else {
        return text.to_owned();
    };
    match change(&mut doc) {
        Some(()) => doc.write(),
        None => text.to_owned(),
    }
}

/// An id not yet taken, from a kind: fetch, fetch-2, fetch-3.
pub fn free_id(taken: &[&str], kind: &str) -> String {
    if !taken.contains(&kind) {
        return kind.to_owned();
    }
    (2..)
        .map(|n| format!("{kind}-{n}"))
        .find(|id| !taken.contains(&id.as_str()))
        .unwrap_or_default()
}

/// What a new step of each kind starts with: settings the schema takes, for the person to change.
pub fn starting_settings(kind: &str) -> Value {
    match kind {
        "fetch" => json!({ "url": "https://api.example.org/", "format": "json" }),
        "transform" => json!({ "language": "jq", "script": "." }),
        "validate" => json!({ "rules": [{ "field": "id", "check": "required" }], "on_invalid": "fail" }),
        "upload" => json!({ "bucket": "exports", "path": "out/${run.date}.json" }),
        "notify" => json!({ "channel": "#alerts", "on": ["failed"] }),
        _ => Value::Null,
    }
}

/// Adds a step of `kind` after the steps in `needs`, written after the last of them so the list
/// still reads in order; with `before`, the step goes between `needs` and that step, which then
/// needs the new step in their place. Answers with the text and the new step's id, empty when the
/// text does not read.
pub fn add_step(text: &str, kind: &str, needs: &[String], before: Option<&str>) -> (String, String) {
    let Some(mut doc) = parse(text) else {
        return (text.to_owned(), String::new());
    };
    let taken: Vec<String> = step_items(&mut doc)
        .iter()
        .filter_map(|item| id_of(item).map(str::to_owned))
        .collect();
    let id = free_id(&taken.iter().map(String::as_str).collect::<Vec<_>>(), kind);
    let mut step = Node::from_value(&json!({ "id": id, "kind": kind }));
    set_needs(&mut step, needs);
    step.set("with", made(&starting_settings(kind)));
    let Some(list) = step_list(&mut doc) else {
        return (text.to_owned(), String::new());
    };
    list.flow = Some(false);
    let Some(items) = list.items_mut() else {
        return (text.to_owned(), String::new());
    };
    let after = needs
        .iter()
        .map(|need| {
            items
                .iter()
                .position(|item| item.is_map() && id_of(item) == Some(need))
                .map_or(-1, |at| at as isize)
        })
        .fold(-1, isize::max);
    let at = if after == -1 { items.len() } else { after as usize + 1 };
    items.insert(at, step);
    if let Some(before) = before
        && let Some(target) = items
            .iter_mut()
            .enumerate()
            .find(|(index, item)| *index != at && item.is_map() && id_of(item) == Some(before))
            .map(|(_, item)| item)
    {
        let mut next: Vec<String> = Vec::new();
        for need in needs_of(target) {
            let need = if needs.contains(&need) { id.clone() } else { need };
            if !next.contains(&need) {
                next.push(need);
            }
        }
        set_needs(target, &next);
    }
    (doc.write(), id)
}

/// Takes steps out, and out of the needs of the steps that came after them.
pub fn remove_steps(text: &str, ids: &[&str]) -> String {
    edit(text, |doc| {
        let list = step_list(doc)?;
        if let Some(items) = list.items_mut() {
            items.retain(|item| !(item.is_map() && ids.contains(&id_of(item).unwrap_or(""))));
        }
        for item in step_items(doc) {
            let needs = needs_of(item);
            if needs.iter().any(|need| ids.contains(&need.as_str())) {
                let kept: Vec<String> = needs.into_iter().filter(|need| !ids.contains(&need.as_str())).collect();
                set_needs(item, &kept);
            }
        }
        Some(())
    })
}

/// Makes `target` need `source`.
pub fn connect(text: &str, source: &str, target: &str) -> String {
    edit(text, |doc| {
        if let Some(item) = step_items(doc).into_iter().find(|item| id_of(item) == Some(target)) {
            let mut needs = needs_of(item);
            if !needs.iter().any(|need| need == source) {
                needs.push(source.to_owned());
                set_needs(item, &needs);
            }
        }
        Some(())
    })
}

/// Takes `source` out of what `target` needs.
pub fn disconnect(text: &str, source: &str, target: &str) -> String {
    edit(text, |doc| {
        if let Some(item) = step_items(doc).into_iter().find(|item| id_of(item) == Some(target)) {
            let needs: Vec<String> = needs_of(item).into_iter().filter(|need| need != source).collect();
            set_needs(item, &needs);
        }
        Some(())
    })
}

/// `${steps.<from>` in a text made `${steps.<to>`, the spaces after `${` kept, where a `.`, a `}` or
/// a space follows the id.
fn rename_references(text: &str, from: &str, to: &str) -> String {
    let mut out = String::new();
    let mut at = 0;
    while let Some(open) = text[at..].find("${") {
        let start = at + open;
        let spaces_end = text[start + 2..]
            .char_indices()
            .find(|(_, c)| !is_space(*c))
            .map_or(text.len(), |(index, _)| start + 2 + index);
        let wanted = format!("steps.{from}");
        let follows = text[spaces_end..]
            .strip_prefix(&wanted)
            .and_then(|rest| rest.chars().next());
        if follows.is_some_and(|c| c == '.' || c == '}' || is_space(c)) {
            out.push_str(&text[at..start]);
            out.push_str("${");
            out.push_str(&text[start + 2..spaces_end]);
            out.push_str("steps.");
            out.push_str(to);
            at = spaces_end + wanted.len();
        } else {
            out.push_str(&text[at..start + 2]);
            at = start + 2;
        }
    }
    out.push_str(&text[at..]);
    out
}

/// Gives a step a new id, in its own item, in every step's needs and in every `${steps.<id>...}`
/// reference.
pub fn rename_step(text: &str, from: &str, to: &str) -> String {
    edit(text, |doc| {
        for item in step_items(doc) {
            if id_of(item) == Some(from) {
                item.set_scalar("id", Scalar::Str(to.to_owned()));
            }
            let needs = needs_of(item);
            if needs.iter().any(|need| need == from) {
                let renamed: Vec<String> = needs
                    .into_iter()
                    .map(|need| if need == from { to.to_owned() } else { need })
                    .collect();
                set_needs(item, &renamed);
            }
        }
        if let Some(contents) = &mut doc.contents {
            contents.visit_scalars_mut(&mut |node| {
                if let Kind::Scalar {
                    value: Scalar::Str(value),
                    ..
                } = &mut node.kind
                    && value.contains("${")
                {
                    *value = rename_references(value, from, to);
                }
            });
        }
        Some(())
    })
}

/// A step's settings as they are written: every field but its id, kind and needs; none when there
/// is no such step.
pub fn step_fields(text: &str, id: &str) -> Option<Map<String, Value>> {
    let mut doc = parse(text)?;
    let item = step_items(&mut doc).into_iter().find(|item| id_of(item) == Some(id))?;
    let Value::Object(mut all) = item.to_js() else {
        return None;
    };
    for key in ["id", "kind", "needs"] {
        all.shift_remove(key);
    }
    Some(all)
}

/// Writes a step's settings: each field given is set, in the place it is written or else at the
/// end, and each field left out or empty is taken out, its id, kind and needs kept.
pub fn set_step_fields(text: &str, id: &str, fields: &Map<String, Value>) -> String {
    edit(text, |doc| {
        let Some(item) = step_items(doc).into_iter().find(|item| id_of(item) == Some(id)) else {
            return Some(());
        };
        let gone: Vec<String> = item
            .pairs()
            .iter()
            .filter_map(|pair| pair.key.as_str())
            .filter(|key| !matches!(*key, "id" | "kind" | "needs"))
            .filter(|key| !in_object(fields.keys().map(String::as_str), key))
            .map(str::to_owned)
            .collect();
        for key in gone {
            item.delete(&key);
        }
        for (key, value) in fields {
            let settings = item
                .pairs()
                .iter()
                .position(|pair| pair.key.scalar_value() == Some(&Scalar::Str("with".to_owned())));
            let empty = value.as_str() == Some("");
            match settings {
                Some(at) if key != "with" && !item.has(key) && !empty => {
                    // A new field goes before the settings, where the pipelines write retries and timeout.
                    if let Some(pairs) = item.pairs_mut() {
                        pairs.insert(
                            at,
                            Pair {
                                key: Node::text(key),
                                value: made(value),
                            },
                        );
                    }
                }
                _ => assign(item, key, Some(value)),
            }
        }
        Some(())
    })
}
