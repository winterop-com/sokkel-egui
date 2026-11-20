//! The data layer held against the web's own: `web/dump.ts` runs the web example's modules
//! (`model.ts`, `seed.ts`, `tables.ts`, `sql.ts`, `pipeline.ts`, `pipeline-edit.ts`, `simulate.ts`
//! and the `yaml` package they read with) under bun on the inputs in `web/inputs.json`, the zone
//! UTC and the clock and timers held, and writes what they give as JSON. Each test builds the same
//! JSON here and asks for it to be equal.
//!
//! The web's answer is kept in `web/data.json`. Where bun and `.parity/sokkel-ui` are at hand the
//! dump is made again and must match it, so a change on either side shows (`STUDIO_WEB_KEEP=1`
//! keeps the new one); elsewhere (GitHub's CI) the kept answer is read.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use serde_json::{Map, Value, json};
use studio::backend::Backend;
use studio::js::len16 as length;
use studio::model::{FileKind, KINDS, Prefs, WorkFile, bad_name, clock, counted, file_order, say_duration, stem};
use studio::pipeline::{STEP_KINDS, Tone, describe_word, pipeline_schema, read_pipeline, reference_words, step_order};
use studio::pipeline_edit::{
    add_step, connect, disconnect, free_id, remove_steps, rename_step, set_step_fields, starting_settings, step_fields,
};
use studio::seed::{SAMPLES, seed_files, template};
use studio::sql::{Table, Value as Cell, run_query};
use studio::tables::tables_with;
use studio::yaml::{Kind, Node, Scalar, Style, js_number, parse};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The web's answer: made again under bun where it can be, and then held to the kept one.
fn web() -> &'static Value {
    static WEB: OnceLock<Value> = OnceLock::new();
    WEB.get_or_init(|| {
        let kept_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/web/data.json");
        let kept: Option<Value> = std::fs::read_to_string(&kept_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        let fresh = root()
            .join(".parity/sokkel-ui/examples/studio/web/node_modules")
            .exists()
            .then(|| {
                Command::new("bun")
                    .arg("examples/studio/tests/web/dump.ts")
                    .current_dir(root())
                    .env("TZ", "UTC")
                    .output()
                    .ok()
            })
            .flatten()
            .filter(|output| output.status.success())
            .and_then(|output| serde_json::from_slice::<Value>(&output.stdout).ok());
        match (fresh, kept) {
            (Some(fresh), kept) => {
                if std::env::var_os("STUDIO_WEB_KEEP").is_some() {
                    let mut written = Vec::new();
                    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
                    let mut writer = serde_json::Serializer::with_formatter(&mut written, formatter);
                    serde::Serialize::serialize(&fresh, &mut writer).expect("JSON");
                    written.push(b'\n');
                    std::fs::write(&kept_path, written).expect("the web's answer is kept");
                } else {
                    assert!(
                        kept.as_ref().is_some_and(|kept| alike(kept, &fresh)),
                        "web/data.json is not what the web gives now; run with STUDIO_WEB_KEEP=1 to keep it"
                    );
                }
                fresh
            }
            (None, Some(kept)) => kept,
            (None, None) => panic!("neither bun nor web/data.json"),
        }
    })
}

fn inputs() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/web/inputs.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("inputs")).expect("JSON")
}

/// The first place two values differ, and what each holds there.
fn first_difference(ours: &Value, theirs: &Value, path: &str) -> Option<String> {
    match (ours, theirs) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in b {
                let inner = format!("{path}.{key}");
                match a.get(key) {
                    Some(mine) => {
                        if let Some(found) = first_difference(mine, value, &inner) {
                            return Some(found);
                        }
                    }
                    None => return Some(format!("{inner}: ours has none, the web's {value}")),
                }
            }
            a.keys()
                .find(|key| !b.contains_key(*key))
                .map(|key| format!("{path}.{key}: the web has none"))
        }
        (Value::Array(a), Value::Array(b)) => {
            for (at, (mine, web)) in a.iter().zip(b).enumerate() {
                if let Some(found) = first_difference(mine, web, &format!("{path}[{at}]")) {
                    return Some(found);
                }
            }
            (a.len() != b.len()).then(|| format!("{path}: {} items, the web's {}", a.len(), b.len()))
        }
        _ => (ours != theirs).then(|| format!("{path}: ours {ours}, the web's {theirs}")),
    }
}

/// Asks for each key of the web's answer to be equal, a list item by item, so a failure names the part.
fn same(ours: &Value, theirs: &Value, keys: &[&str]) {
    for key in keys {
        match (&ours[key], &theirs[key]) {
            (Value::Array(mine), Value::Array(web)) => {
                for (at, (mine, web)) in mine.iter().zip(web).enumerate() {
                    if let Some(found) = first_difference(mine, web, &format!("{key}[{at}]")) {
                        panic!(
                            "{found}\n  {}",
                            serde_json::to_string(&web.get("text").or(web.get("source"))).unwrap_or_default()
                        );
                    }
                }
                assert_eq!(mine.len(), web.len(), "{key}: lengths");
            }
            (mine, web) => {
                if let Some(found) = first_difference(mine, web, key) {
                    panic!("{found}");
                }
            }
        }
    }
}

fn cell(value: &Cell) -> Value {
    match value {
        Cell::Null => Value::Null,
        Cell::Bool(b) => Value::Bool(*b),
        Cell::Num(n) => js_number(*n),
        Cell::Str(text) => Value::String(text.clone()),
    }
}

fn table(table: &Table) -> Value {
    json!({
        "name": table.name,
        "description": table.description,
        "columns": table.columns.iter().map(|column| json!({
            "name": column.name,
            "type": column.kind.name(),
            "description": column.description,
        })).collect::<Vec<_>>(),
        "rows": table.rows.iter().map(|row| {
            let mut map = Map::new();
            for (column, value) in table.columns.iter().zip(row) {
                map.insert(column.name.clone(), cell(value));
            }
            Value::Object(map)
        }).collect::<Vec<_>>(),
    })
}

/// A byte offset into a text as the web counts it, in UTF-16 units.
fn units(text: &str, at: usize) -> usize {
    if at > text.len() {
        return length(text) + at - text.len();
    }
    length(&text[..at])
}

fn query(source: &str, tables: &[Table]) -> Value {
    match run_query(source, tables) {
        Ok(result) => json!({
            "source": source,
            "columns": result.columns,
            "rows": result.rows.iter().map(|row| row.iter().map(cell).collect::<Vec<_>>()).collect::<Vec<_>>(),
        }),
        Err(error) => match error.at {
            Some((from, to)) => json!({
                "source": source,
                "error": error.message,
                "from": units(source, from),
                "to": units(source, to),
            }),
            None => json!({ "source": source, "error": error.message }),
        },
    }
}

fn style(style: Option<Style>) -> Value {
    match style {
        None => Value::Null,
        Some(Style::Plain) => json!("PLAIN"),
        Some(Style::Single) => json!("QUOTE_SINGLE"),
        Some(Style::Double) => json!("QUOTE_DOUBLE"),
        Some(Style::Literal) => json!("BLOCK_LITERAL"),
        Some(Style::Folded) => json!("BLOCK_FOLDED"),
    }
}

fn tree(text: &str, node: &Node) -> Value {
    let mut out = Map::new();
    match &node.kind {
        Kind::Scalar { value, style: s, .. } => {
            out.insert(
                "v".to_owned(),
                match value {
                    Scalar::Null => Value::Null,
                    Scalar::Bool(b) => Value::Bool(*b),
                    Scalar::Num(n) => js_number(*n),
                    Scalar::Str(t) => Value::String(t.clone()),
                },
            );
            out.insert("type".to_owned(), style(*s));
        }
        Kind::Map(pairs) => {
            out.insert(
                "map".to_owned(),
                pairs
                    .iter()
                    .map(|pair| json!([tree(text, &pair.key), tree(text, &pair.value)]))
                    .collect(),
            );
        }
        Kind::Seq(items) => {
            out.insert("seq".to_owned(), items.iter().map(|item| tree(text, item)).collect());
        }
    }
    out.insert(
        "range".to_owned(),
        node.range
            .map_or(Value::Null, |range| json!(range.map(|at| units(text, at)))),
    );
    if node.flow == Some(true) {
        out.insert("flow".to_owned(), json!(true));
    }
    if let Some(before) = &node.comment_before {
        out.insert("cb".to_owned(), json!(before));
    }
    if let Some(comment) = &node.comment {
        out.insert("c".to_owned(), json!(comment));
    }
    if node.space_before {
        out.insert("sb".to_owned(), json!(true));
    }
    Value::Object(out)
}

#[test]
fn the_seeded_workspace_reads_as_the_web_reads_it() {
    let files = seed_files();
    let mut samples = Map::new();
    for (id, text) in SAMPLES {
        samples.insert(id.to_owned(), json!(text));
    }
    let mut order: Vec<WorkFile> = [
        ("b.yaml", Some(1)),
        ("a.yaml", None),
        ("C.yaml", None),
        ("z.yaml", Some(0)),
        ("c.yaml", None),
        ("m.yaml", Some(1)),
    ]
    .into_iter()
    .map(|(name, rank)| WorkFile {
        name: name.to_owned(),
        rank,
        ..files[0].clone()
    })
    .collect();
    order.sort_by(file_order);
    let now = inputs()["now"].as_i64().expect("now");
    let ours = json!({
        "files": serde_json::to_value(&files).expect("files"),
        "samples": samples,
        "templates": ([FileKind::Pipeline, FileKind::Schema, FileKind::Query]
            .map(|kind| template(kind, "stock-alerts", studio::model::Project::Juniper))),
        "prefs": serde_json::to_value(Prefs::default()).expect("prefs"),
        "kinds": KINDS.iter().map(|one| json!({
            "kind": one.kind, "label": one.label, "plural": one.plural, "extension": one.extension,
        })).collect::<Vec<_>>(),
        "stems": files.iter().map(|file| stem(&file.name, file.kind)).collect::<Vec<_>>(),
        "badNames": [
            bad_name("", &[]),
            bad_name("  ", &[]),
            bad_name("Nightly", &[]),
            bad_name("9lives", &[]),
            bad_name("a_b", &[]),
            bad_name("nightly-export", &["nightly-export"]),
            bad_name(" ok-2 ", &["ok"]),
        ],
        "durations": ([0.0, 0.4, 840.0, 999.5, 1000.0, 1250.0, 2449.0, 59_949.0, 59_999.0, 60_000.0, 72_400.0, 119_600.0, 3_600_000.0]
            .map(say_duration)),
        "clocks": ([now, now + 3_723_456, 0].map(clock)),
        "counted": [
            counted(1.0, "row", None),
            counted(0.0, "row", None),
            counted(1_234_567.0, "run", None),
            counted(2.0, "query", Some("queries")),
            counted(1.5, "step", None),
        ],
        "order": order.iter().map(|file| file.name.clone()).collect::<Vec<_>>(),
    });
    same(
        &ours,
        web(),
        &[
            "files",
            "samples",
            "templates",
            "prefs",
            "kinds",
            "stems",
            "badNames",
            "durations",
            "clocks",
            "counted",
            "order",
        ],
    );
}

#[test]
fn the_tables_and_queries_give_what_the_web_gives() {
    let tables = tables_with(&[]);
    let mut sources: Vec<String> = seed_files()
        .into_iter()
        .filter(|file| file.kind == FileKind::Query)
        .map(|file| file.text)
        .collect();
    sources.extend(
        inputs()["queries"]
            .as_array()
            .expect("queries")
            .iter()
            .map(|one| one.as_str().expect("a query").to_owned()),
    );
    let ours = json!({
        "tables": tables.iter().map(table).collect::<Vec<_>>(),
        "queries": sources.iter().map(|source| query(source, &tables)).collect::<Vec<_>>(),
    });
    same(&ours, web(), &["tables", "queries"]);
}

#[test]
fn yaml_reads_and_writes_as_the_web_package_does() {
    let ours: Vec<Value> = inputs()["yaml"]
        .as_array()
        .expect("yaml")
        .iter()
        .map(|text| {
            let text = text.as_str().expect("a text");
            match parse(text) {
                None => json!({ "text": text, "broken": true }),
                Some(doc) => json!({
                    "text": text,
                    "broken": false,
                    "js": doc.to_js(),
                    "tree": {
                        "contents": doc.contents.as_ref().map_or(Value::Null, |node| tree(text, node)),
                        "cb": doc.comment_before,
                        "c": doc.comment,
                    },
                    "written": doc.write(),
                }),
            }
        })
        .collect();
    same(&json!({ "yaml": ours }), web(), &["yaml"]);
}

fn read(text: &str) -> Value {
    let pipeline = read_pipeline(text);
    let mut out = json!({
        "name": pipeline.name,
        "params": pipeline.params,
        "steps": pipeline.steps.iter().map(|step| json!({
            "id": step.id,
            "kind": step.kind,
            "needs": step.needs,
            "config": step.config,
            "retries": js_number(step.retries),
            "from": units(text, step.from),
            "to": units(text, step.to),
        })).collect::<Vec<_>>(),
        "marks": pipeline.marks.iter().map(|mark| json!({
            "from": units(text, mark.from),
            "to": units(text, mark.to),
            "tone": match mark.tone { Tone::Error => "error", Tone::Warning => "warning" },
            "message": mark.message,
        })).collect::<Vec<_>>(),
        "broken": pipeline.broken,
    });
    if let Some(project) = &pipeline.project {
        out["project"] = json!(project);
    }
    let order = step_order(&pipeline.steps).map(|order| order.into_iter().map(|step| step.id).collect::<Vec<_>>());
    let words: Vec<Value> = reference_words(&pipeline)
        .iter()
        .map(|word| {
            let mut one = json!({ "label": word.label, "detail": word.detail, "kind": word.kind });
            if let Some(docs) = &word.docs {
                one["docs"] = json!(docs);
            }
            one
        })
        .collect();
    let described: Vec<Option<String>> = inputs()["words"]
        .as_array()
        .expect("words")
        .iter()
        .map(|word| describe_word(&pipeline, word.as_str().expect("a word")))
        .collect();
    json!({ "text": text, "read": out, "order": order, "words": words, "described": described })
}

#[test]
fn pipelines_read_and_edit_as_the_web_reads_and_edits_them() {
    let files = seed_files();
    let mut texts: Vec<String> = files
        .iter()
        .filter(|file| file.kind == FileKind::Pipeline)
        .map(|file| file.text.clone())
        .collect();
    let inputs = inputs();
    texts.extend(
        inputs["pipelines"]
            .as_array()
            .expect("pipelines")
            .iter()
            .map(|one| one.as_str().expect("a pipeline").to_owned()),
    );
    let text_of = |one: &Value| -> String {
        match (one["text"].as_str(), one["file"].as_str()) {
            (Some(text), _) => text.to_owned(),
            (None, Some(id)) => files
                .iter()
                .find(|file| file.id == id)
                .map(|file| file.text.clone())
                .unwrap_or_default(),
            _ => String::new(),
        }
    };
    let strings = |value: &Value| -> Vec<String> {
        value
            .as_array()
            .map(|list| list.iter().filter_map(|one| one.as_str().map(str::to_owned)).collect())
            .unwrap_or_default()
    };
    let edits: Vec<Value> = inputs["edits"]
        .as_array()
        .expect("edits")
        .iter()
        .map(|one| {
            let text = text_of(one);
            let s = |key: &str| one[key].as_str().unwrap_or("").to_owned();
            match one["op"].as_str().expect("an op") {
                "addStep" => {
                    let (text, id) = add_step(&text, &s("kind"), &strings(&one["needs"]), one["before"].as_str());
                    json!({ "text": text, "id": id })
                }
                "removeSteps" => {
                    let ids = strings(&one["ids"]);
                    json!(remove_steps(&text, &ids.iter().map(String::as_str).collect::<Vec<_>>()))
                }
                "connect" => json!(connect(&text, &s("source"), &s("target"))),
                "disconnect" => json!(disconnect(&text, &s("source"), &s("target"))),
                "renameStep" => json!(rename_step(&text, &s("from"), &s("to"))),
                "stepFields" => step_fields(&text, &s("id")).map_or(Value::Null, Value::Object),
                "setStepFields" => {
                    json!(set_step_fields(
                        &text,
                        &s("id"),
                        one["fields"].as_object().expect("fields")
                    ))
                }
                other => panic!("no edit {other}"),
            }
        })
        .collect();
    let free_ids: Vec<String> = inputs["freeIds"]
        .as_array()
        .expect("free ids")
        .iter()
        .map(|one| {
            let taken = strings(&one[0]);
            free_id(
                &taken.iter().map(String::as_str).collect::<Vec<_>>(),
                one[1].as_str().expect("a kind"),
            )
        })
        .collect();
    let ours = json!({
        "schema": pipeline_schema(),
        "pipelines": texts.iter().map(|text| read(text)).collect::<Vec<_>>(),
        "freeIds": free_ids,
        "starting": STEP_KINDS.iter().map(|kind| starting_settings(kind)).collect::<Vec<_>>(),
        "edits": edits,
    });
    same(&ours, web(), &["schema", "pipelines", "freeIds", "starting", "edits"]);
}

#[test]
fn runs_go_as_the_web_runs_them() {
    let inputs = inputs();
    let now = inputs["now"].as_i64().expect("now");
    let mut backend = Backend::seeded();
    let mut looks = Vec::new();
    let mut refusals = Vec::new();
    for step in inputs["timeline"].as_array().expect("timeline") {
        let at = step["at"].as_i64().expect("at");
        backend.tick(now + at);
        match step["do"].as_str().expect("what") {
            "start" => {
                let id = step["file"].as_str().expect("a file");
                backend.revert(id);
                if let Some(text) = step["text"].as_str() {
                    backend.edit(id, text);
                }
                if let Some(Err(notice)) = backend.run(id, now + at) {
                    refusals.push(json!({ "at": at, "message": notice.description }));
                }
                backend.revert(id);
            }
            "cancel" => {
                let number = step["run"].as_i64().expect("a run");
                if let Some(id) = backend
                    .runs
                    .iter()
                    .find(|run| run.number == number)
                    .map(|run| run.id.clone())
                {
                    backend.cancel(&id, now + at);
                }
            }
            _ => looks.push(json!({ "at": at, "runs": serde_json::to_value(&backend.runs).expect("runs") })),
        }
    }
    let after = tables_with(&backend.runs);
    let ours = json!({
        "looks": looks,
        "refusals": refusals,
        "afterRuns": inputs["afterRuns"]
            .as_array()
            .expect("queries")
            .iter()
            .map(|source| query(source.as_str().expect("a query"), &after))
            .collect::<Vec<_>>(),
    });
    same(&ours, web(), &["looks", "refusals", "afterRuns"]);
}

/// Whether the kept answer is still the web's. JavaScriptCore may round a `Math` function's result
/// one unit in the last place differently once its JIT has optimized the code, which happens
/// sooner on a busy machine, so a number may move by a few units in its last place between two
/// runs of the same web code; everything else must be the same. The Rust is held to the kept
/// answer exactly.
fn alike(kept: &Value, fresh: &Value) -> bool {
    match (kept, fresh) {
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => a == b || (a - b).abs() <= a.abs().max(b.abs()) * 8.0 * f64::EPSILON,
            _ => a == b,
        },
        (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| alike(a, b)),
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(key, a)| b.get(key).is_some_and(|b| alike(a, b)))
        }
        _ => kept == fresh,
    }
}
