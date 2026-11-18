//! A pipeline as the studio reads it, as the web's `pipeline.ts`: the schema its YAML is checked
//! and completed against, the steps read out of it with where each is written, and the `${...}`
//! references the schema leaves to the app, checked here.
//!
//! Every place is a byte offset into the pipeline's text, where the web counts UTF-16 units.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;

use serde_json::{Map, Value};

use crate::js::{number_to_string, trim};
use crate::yaml::{Node, parse};

/// The kinds of step, in the order the graph's picker lists them.
pub const STEP_KINDS: [&str; 5] = ["fetch", "transform", "validate", "upload", "notify"];

/// What each kind of step does, said on hover and in completion.
pub fn kind_docs(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "fetch" => "Reads rows from an address on example.org, as CSV, JSON or NDJSON.",
        "transform" => "Reshapes the rows it is given with a jq or SQL script.",
        "validate" => "Checks each row against rules, failing, skipping or warning on the rows it refuses.",
        "upload" => "Writes the rows to a path in a bucket.",
        "notify" => "Posts to a channel when the run ends in one of the states given.",
        _ => return None,
    })
}

/// The JSON Schema (2020-12) every pipeline is written in; each step's `with` follows from its
/// `kind` through `allOf` and `if`.
pub fn pipeline_schema() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(include_str!("pipeline.schema.json")).unwrap_or(Value::Null))
}

/// A value holding a `${...}` reference, which is filled in when the pipeline runs and so is left to
/// the app to check.
pub fn holds_reference(value: &Value) -> bool {
    value.as_str().is_some_and(|text| text.contains("${"))
}

/// What a run of a pipeline offers its references beside params and steps.
pub const RUN_FIELDS: [(&str, &str); 4] = [
    ("id", "The run's number."),
    ("date", "The day the run started, as 2026-11-10."),
    ("started", "When the run started, to the second."),
    ("by", "Who started the run."),
];

/// What a step hands on, for `${steps.<id>.<field>}`.
pub const STEP_FIELDS: [(&str, &str); 3] = [
    ("rows", "How many rows the step gave."),
    ("output", "Where the step wrote what it gave."),
    ("rejected", "How many rows a validate step refused."),
];

/// What every object in the web has from its prototype, which `name in object` finds too.
const PROTOTYPE: [&str; 12] = [
    "toString",
    "toLocaleString",
    "valueOf",
    "hasOwnProperty",
    "propertyIsEnumerable",
    "isPrototypeOf",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
    "__proto__",
    "constructor",
];

/// Whether `name in object` holds for an object with these keys of its own.
pub fn in_object<'a>(mut keys: impl Iterator<Item = &'a str>, name: &str) -> bool {
    keys.any(|key| key == name) || PROTOTYPE.contains(&name)
}

/// What `String(object[name])` gives for a name an object has only from its prototype.
pub fn prototype_text(name: &str) -> Option<String> {
    match name {
        "__proto__" => Some("[object Object]".to_owned()),
        "constructor" => Some("function Object() { [native code] }".to_owned()),
        _ if PROTOTYPE.contains(&name) => Some(format!("function {name}() {{ [native code] }}")),
        _ => None,
    }
}

/// A value as `String(value)` writes it: a list's items joined by commas, a map as `[object Object]`.
pub fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| if item.is_null() { String::new() } else { js_string(item) })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// One step as read from a pipeline.
#[derive(Clone, Debug, PartialEq)]
pub struct StepInfo {
    pub id: String,
    pub kind: String,
    pub needs: Vec<String>,
    /// Its `with`, or an empty map when that is not one.
    pub config: Map<String, Value>,
    pub retries: f64,
    /// Where the step's item is written.
    pub from: usize,
    pub to: usize,
}

/// How a mark reads: an error or a warning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Error,
    Warning,
}

/// A problem the app finds in a pipeline, where it is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    pub from: usize,
    pub to: usize,
    pub tone: Tone,
    pub message: String,
}

/// A pipeline as read: its name, project, params and steps, and what the app checks that the
/// schema cannot (references, unknown needs, repeated ids and loops).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pipeline {
    pub name: String,
    pub project: Option<String>,
    pub params: Map<String, Value>,
    pub steps: Vec<StepInfo>,
    pub marks: Vec<Mark>,
    /// The YAML does not parse; the editor says where.
    pub broken: bool,
}

fn as_record(value: Option<&Value>) -> Map<String, Value> {
    match value {
        Some(Value::Object(map)) => map.clone(),
        _ => Map::new(),
    }
}

/// The steps a step comes after, directly or through others.
pub fn before(steps: &[StepInfo], id: &str) -> HashSet<String> {
    let by_id: HashMap<&str, &StepInfo> = steps.iter().map(|step| (step.id.as_str(), step)).collect();
    let mut seen = HashSet::new();
    let mut queue: VecDeque<String> = by_id.get(id).map(|step| step.needs.clone()).unwrap_or_default().into();
    while let Some(at) = queue.pop_front() {
        if !seen.insert(at.clone()) {
            continue;
        }
        if let Some(step) = by_id.get(at.as_str()) {
            queue.extend(step.needs.iter().cloned());
        }
    }
    seen
}

/// The steps in an order each comes after what it needs, or none when they loop.
pub fn step_order(steps: &[StepInfo]) -> Option<Vec<StepInfo>> {
    let mut done: HashSet<&str> = HashSet::new();
    let mut order = Vec::new();
    let known: HashSet<&str> = steps.iter().map(|step| step.id.as_str()).collect();
    while order.len() < steps.len() {
        let next = steps.iter().find(|step| {
            !done.contains(step.id.as_str())
                && step
                    .needs
                    .iter()
                    .all(|need| done.contains(need.as_str()) || !known.contains(need.as_str()))
        })?;
        done.insert(&next.id);
        order.push(next.clone());
    }
    Some(order)
}

/// Every `${...}` in a text: where it starts and ends, and what is inside.
pub fn references(text: &str) -> Vec<(usize, usize, &str)> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(open) = text[at..].find("${") {
        let start = at + open;
        match text[start + 2..].find('}') {
            Some(close) => {
                let end = start + 2 + close + 1;
                found.push((start, end, &text[start + 2..end - 1]));
                at = end;
            }
            None => break,
        }
    }
    found
}

/// Reads a pipeline's YAML: its steps and where they are, and what its references get wrong.
pub fn read_pipeline(text: &str) -> Pipeline {
    let Some(doc) = parse(text) else {
        return Pipeline {
            broken: true,
            ..Pipeline::default()
        };
    };
    let value = doc.to_js();
    let record = as_record(Some(&value));
    let params = as_record(record.get("params"));
    let mut marks = Vec::new();
    let mut steps: Vec<StepInfo> = Vec::new();
    let mut need_nodes: HashMap<String, Vec<&Node>> = HashMap::new();
    if let Some(list) = doc.get("steps").filter(|node| node.is_seq()) {
        for item in list.items() {
            let (true, Some(range)) = (item.is_map(), item.range) else {
                continue;
            };
            let raw = as_record(Some(&item.to_js()));
            let id = raw.get("id").and_then(Value::as_str).unwrap_or("").to_owned();
            if id.is_empty() {
                continue;
            }
            let needs: Vec<String> = match raw.get("needs") {
                Some(Value::Array(needs)) => needs
                    .iter()
                    .filter_map(|need| need.as_str().map(str::to_owned))
                    .collect(),
                _ => Vec::new(),
            };
            if let Some(needs_node) = item.get("needs").filter(|node| node.is_seq()) {
                need_nodes.insert(id.clone(), needs_node.items().iter().collect());
            }
            let id_node = item.get("id");
            if steps.iter().any(|step| step.id == id)
                && let Some(node) = id_node.filter(|node| node.is_scalar())
                && let Some(range) = node.range
            {
                marks.push(Mark {
                    from: range[0],
                    to: range[1],
                    tone: Tone::Error,
                    message: format!("Another step is already named {id}."),
                });
            }
            steps.push(StepInfo {
                kind: raw.get("kind").and_then(Value::as_str).unwrap_or("").to_owned(),
                needs,
                config: as_record(raw.get("with")),
                retries: raw.get("retries").and_then(Value::as_f64).unwrap_or(0.0),
                from: range[0],
                to: range[1],
                id,
            });
        }
    }
    let ids: HashSet<&str> = steps.iter().map(|step| step.id.as_str()).collect();
    for step in &steps {
        for node in need_nodes.get(&step.id).into_iter().flatten() {
            let (Some(need), Some(range)) = (node.as_str(), node.range) else {
                continue;
            };
            let message = if need == step.id {
                "A step cannot need itself.".to_owned()
            } else if !ids.contains(need) {
                format!("No step is named {need}.")
            } else if before(&steps, need).contains(&step.id) {
                format!("{need} already comes after {}, so this makes a loop.", step.id)
            } else {
                continue;
            };
            marks.push(Mark {
                from: range[0],
                to: range[1],
                tone: Tone::Error,
                message,
            });
        }
    }
    // Each reference in a string names a param, a step this one comes after, or the run.
    if let Some(contents) = &doc.contents {
        contents.visit_scalars(&mut |node| {
            let (Some(value), Some(range)) = (node.as_str(), node.range) else {
                return;
            };
            if !value.contains("${") {
                return;
            }
            let start = range[0];
            let source = &text[start..range[1]];
            let owner = steps.iter().find(|step| start >= step.from && start < step.to);
            for (from, to, inside) in references(source) {
                if let Some((tone, message)) = check_reference(trim(inside), &params, &steps, owner) {
                    marks.push(Mark {
                        from: start + from,
                        to: start + to,
                        tone,
                        message,
                    });
                }
            }
        });
    }
    Pipeline {
        name: record.get("name").and_then(Value::as_str).unwrap_or("").to_owned(),
        project: record.get("project").and_then(Value::as_str).map(str::to_owned),
        params,
        steps,
        marks,
        broken: false,
    }
}

fn check_reference(
    reference: &str,
    params: &Map<String, Value>,
    steps: &[StepInfo],
    owner: Option<&StepInfo>,
) -> Option<(Tone, String)> {
    let mut parts = reference.split('.');
    let root = parts.next().unwrap_or("");
    let name = parts.next();
    let field = parts.next();
    let rest = parts.count();
    let error = |message: String| Some((Tone::Error, message));
    match root {
        "params" => {
            let Some(name) = name.filter(|name| !name.is_empty()) else {
                return error("Name a param: ${params.<name>}.".to_owned());
            };
            if !in_object(params.keys().map(String::as_str), name) {
                let known: Vec<&str> = params.keys().map(String::as_str).collect();
                let tail = if known.is_empty() {
                    " The pipeline has no params.".to_owned()
                } else {
                    format!(" The params are {}.", known.join(", "))
                };
                return error(format!("No param is named {name}.{tail}"));
            }
            field.and_then(|_| error("A param has no fields.".to_owned()))
        }
        "run" => {
            if name.is_none_or(|name| !in_object(RUN_FIELDS.iter().map(|(key, _)| *key), name)) {
                let fields: Vec<&str> = RUN_FIELDS.iter().map(|(key, _)| *key).collect();
                return error(format!("A run has {}.", fields.join(", ")));
            }
            None
        }
        "steps" => {
            let Some(step) = steps.iter().find(|one| Some(one.id.as_str()) == name) else {
                return error(format!("No step is named {}.", name.unwrap_or("")));
            };
            if field.is_none_or(|field| !in_object(STEP_FIELDS.iter().map(|(key, _)| *key), field)) || rest > 0 {
                let fields: Vec<&str> = STEP_FIELDS.iter().map(|(key, _)| *key).collect();
                return error(format!("A step hands on {}.", fields.join(", ")));
            }
            let Some(owner) = owner else {
                return error("Only a step can read what another step hands on.".to_owned());
            };
            if owner.id == step.id {
                return error("A step cannot read what it hands on itself.".to_owned());
            }
            if !before(steps, &owner.id).contains(&step.id) {
                return Some((
                    Tone::Warning,
                    format!("{} does not come after {}; add it to needs.", owner.id, step.id),
                ));
            }
            None
        }
        _ => error("Unknown reference. Start it with params, steps or run.".to_owned()),
    }
}

/// What a word completes to, as the editor's completion lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub label: String,
    pub detail: String,
    pub docs: Option<String>,
    /// keyword, variable, function or property.
    pub kind: &'static str,
}

/// The words a reference completes from: the roots, the params, the steps and their fields, and
/// the run's.
pub fn reference_words(pipeline: &Pipeline) -> Vec<Completion> {
    let word = |label: &str, detail: String, docs: Option<&str>, kind| Completion {
        label: label.to_owned(),
        detail,
        docs: docs.map(str::to_owned),
        kind,
    };
    let mut out = vec![
        word("params", "root".to_owned(), Some("The pipeline's params."), "keyword"),
        word("steps", "root".to_owned(), Some("What the steps hand on."), "keyword"),
        word("run", "root".to_owned(), Some("The run this is."), "keyword"),
    ];
    out.extend(
        pipeline
            .params
            .iter()
            .map(|(name, value)| word(name, format!("param = {}", js_string(value)), None, "variable")),
    );
    out.extend(
        pipeline
            .steps
            .iter()
            .map(|step| word(&step.id, format!("{} step", step.kind), None, "function")),
    );
    out.extend(
        STEP_FIELDS
            .iter()
            .map(|(label, docs)| word(label, "step field".to_owned(), Some(docs), "property")),
    );
    out.extend(
        RUN_FIELDS
            .iter()
            .map(|(label, docs)| word(label, "run field".to_owned(), Some(docs), "property")),
    );
    out
}

/// What a word in a pipeline is, for its hover: a kind of step, a step or a param.
pub fn describe_word(pipeline: &Pipeline, word: &str) -> Option<String> {
    if let Some(kind) = kind_docs(word) {
        let step = pipeline.steps.iter().find(|one| one.id == word);
        return Some(match step {
            Some(step) if step.kind != word => format!("{kind} Also a {} step here.", step.kind),
            _ => kind.to_owned(),
        });
    }
    if let Some(step) = pipeline.steps.iter().find(|one| one.id == word) {
        let after = if step.needs.is_empty() {
            ", first to run".to_owned()
        } else {
            format!(", after {}", step.needs.join(", "))
        };
        return Some(format!("A {} step{after}.", step.kind));
    }
    match pipeline.params.get(word) {
        Some(value) => Some(format!("A param, {}.", js_string(value))),
        None => prototype_text(word).map(|text| format!("A param, {text}.")),
    }
}
