//! The data layer held against the web's own: `web/dump.ts` runs the web example's modules
//! (`data/model.ts`, `types.ts`, `seed.ts`, `validate.ts`, `evaluate.ts`, `search.ts`, `server.ts`,
//! `editing.ts`, `themes.ts`, `ui.tsx`, sokkel-ui's diff, and the app and its pages with the server
//! behind them) under bun on the inputs in `web/inputs.json`, the zone UTC, the clock, the timers
//! and `performance.now` held and `Math.random` a seeded run, and writes what they give as JSON.
//! Each test builds the same JSON here and asks for it to be equal, every number to the bit.
//!
//! The web's answer is kept in `web/data.json`. Where bun and `.parity/sokkel-ui` are at hand the
//! dump is made again and must match it, so a change on either side shows (`FHIR_WEB_KEEP=1`
//! keeps the new one); elsewhere (GitHub's CI) the kept answer is read. Carried from the dhis2
//! example's (`examples/dhis2/tests/web.rs`).

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use fhir::app::{self, App, Page, SEARCH_MIN_QUERY};
use fhir::evaluate::{describe_word, element_names, evaluate, functions, variables_offered};
use fhir::js::{parse_instant, pretty};
use fhir::model::{
    Issue, Part, Refused, Severity, concept_text, from_base64, human_name, issue_path, key_of, outcome, quantity_text,
    reference_text, to_base64,
};
use fhir::pages::{self, PageState, Then};
use fhir::search::{RESULT_PARAMS, params_of, read_query, search};
use fhir::seed::{VITALS, seed, template};
use fhir::server::{
    ME, SERVER_ADDRESS, SERVER_BASE, SERVER_NAME, SERVER_VERSION, Stored, history_seed, parse, pointing_at,
    resources_seed,
};
use fhir::themes::CARRIED;
use fhir::types::{DATATYPE_NAMES, ElementDef, TYPE_NAMES, datatype, matches_pattern, pattern_text, schema_for, types};
use fhir::ui::{self, Prefs, count, on_day, state_of, statuses, title_of, when};
use fhir::validate::{cql_header, validate};
use serde_json::{Map, Value as Json, json};
use sokkel_egui::{diff_lines, fold_diff};

fn root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The web's answer: made again under bun where it can be, and then held to the kept one.
fn web() -> &'static Json {
    static WEB: OnceLock<Json> = OnceLock::new();
    WEB.get_or_init(|| {
        let kept_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/web/data.json");
        let kept: Option<Json> = std::fs::read_to_string(&kept_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        let fresh = root_dir()
            .join(".parity/sokkel-ui/examples/fhir/web/node_modules")
            .exists()
            .then(|| {
                Command::new("bun")
                    .arg("examples/fhir/tests/web/dump.ts")
                    .current_dir(root_dir())
                    .env("TZ", "UTC")
                    .output()
                    .ok()
            })
            .flatten()
            .filter(|output| output.status.success())
            .and_then(|output| serde_json::from_slice::<Json>(&output.stdout).ok());
        match (fresh, kept) {
            (Some(fresh), kept) => {
                if std::env::var_os("FHIR_WEB_KEEP").is_some() {
                    let mut written = Vec::new();
                    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
                    let mut writer = serde_json::Serializer::with_formatter(&mut written, formatter);
                    serde::Serialize::serialize(&fresh, &mut writer).expect("JSON");
                    written.push(b'\n');
                    std::fs::write(&kept_path, written).expect("the web's answer is kept");
                    fresh
                } else {
                    // The Rust is held to the kept answer, the fresh one only to be alike.
                    match kept {
                        Some(kept) if alike(&kept, &fresh) => kept,
                        _ => {
                            panic!("web/data.json is not what the web gives now; run with FHIR_WEB_KEEP=1 to keep it")
                        }
                    }
                }
            }
            (None, Some(kept)) => kept,
            (None, None) => panic!("neither bun nor web/data.json"),
        }
    })
}

/// Whether the kept answer is still the web's. JavaScriptCore may round a `Math` function's result
/// one unit in the last place differently once its JIT has optimized the code, which happens
/// sooner on a busy machine, so a number may move by a few units in its last place between two
/// runs of the same web code; everything else must be the same. The Rust is held to the kept
/// answer exactly.
fn alike(kept: &Json, fresh: &Json) -> bool {
    match (kept, fresh) {
        (Json::Number(a), Json::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => a == b || (a - b).abs() <= a.abs().max(b.abs()) * 8.0 * f64::EPSILON,
            _ => a == b,
        },
        (Json::Array(a), Json::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| alike(a, b)),
        (Json::Object(a), Json::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(key, a)| b.get(key).is_some_and(|b| alike(a, b)))
        }
        _ => kept == fresh,
    }
}

fn inputs() -> &'static Json {
    static INPUTS: OnceLock<Json> = OnceLock::new();
    INPUTS.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/web/inputs.json");
        serde_json::from_str(&std::fs::read_to_string(path).expect("inputs")).expect("JSON")
    })
}

/// A value as the web's JSON writes it: a whole number without its point.
fn norm(value: Json) -> Json {
    match value {
        Json::Number(number) => match number.as_f64() {
            Some(float) if number.is_f64() && float.fract() == 0.0 && float.abs() < 9_007_199_254_740_992.0 => {
                json!(float as i64)
            }
            _ => Json::Number(number),
        },
        Json::Array(items) => Json::Array(items.into_iter().map(norm).collect()),
        Json::Object(map) => Json::Object(map.into_iter().map(|(key, value)| (key, norm(value))).collect()),
        other => other,
    }
}

fn to_json(value: &impl serde::Serialize) -> Json {
    norm(serde_json::to_value(value).expect("JSON"))
}

/// The first place two values differ, and what each holds there.
fn first_difference(ours: &Json, theirs: &Json, path: &str) -> Option<String> {
    match (ours, theirs) {
        (Json::Object(a), Json::Object(b)) => {
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
        (Json::Array(a), Json::Array(b)) => {
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

fn same(ours: &Json, theirs: &Json, what: &str) {
    if let Some(found) = first_difference(&norm(ours.clone()), theirs, what) {
        panic!("{found}");
    }
}

fn text(value: &Json) -> &str {
    value.as_str().expect("a text")
}

fn texts(value: &Json) -> Vec<String> {
    value
        .as_array()
        .expect("texts")
        .iter()
        .map(|one| text(one).to_owned())
        .collect()
}

fn core() -> &'static Json {
    &web()["core"]
}

fn seed_resources() -> Vec<Json> {
    seed().into_iter().map(|one| one.resource).collect()
}

fn by_key(key: &str) -> Option<Json> {
    seed_resources().into_iter().find(|one| key_of(one) == key)
}

/// A resource an input names: a seed one with fields set and dropped, or one written in full.
fn resource_of(input: &Json) -> Json {
    if let Some(raw) = input.get("raw") {
        return raw.clone();
    }
    let mut base = input
        .get("base")
        .and_then(Json::as_str)
        .and_then(by_key)
        .unwrap_or_else(|| json!({}));
    if let Some(fields) = base.as_object_mut() {
        for (name, value) in input.get("set").and_then(Json::as_object).cloned().unwrap_or_default() {
            fields.insert(name, value);
        }
        for name in input.get("drop").and_then(Json::as_array).cloned().unwrap_or_default() {
            fields.shift_remove(text(&name));
        }
    }
    base
}

fn issue_of(value: &Json) -> Issue {
    let severity = match text(&value["severity"]) {
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        _ => Severity::Information,
    };
    let code: &'static str = Box::leak(text(&value["code"]).to_owned().into_boxed_str());
    let path = value.get("path").and_then(Json::as_array).map(|parts| {
        parts
            .iter()
            .map(|part| match part {
                Json::Number(n) => Part::Index(n.as_u64().unwrap_or(0) as usize),
                other => Part::Name(text(other).to_owned()),
            })
            .collect()
    });
    Issue::new(severity, code, path, text(&value["message"]))
}

fn path_of(value: &Json) -> Option<Vec<Part>> {
    value.as_array().map(|parts| {
        parts
            .iter()
            .map(|part| match part {
                Json::Number(n) => Part::Index(n.as_u64().unwrap_or(0) as usize),
                other => Part::Name(text(other).to_owned()),
            })
            .collect()
    })
}

fn opt_str(value: &Json) -> Option<&str> {
    value.as_str()
}

#[test]
fn model_is_the_webs() {
    let input = inputs();
    let ours = json!({
        "issuePaths": input["issuePaths"].as_array().unwrap().iter().map(|pair| {
            issue_path(text(&pair[0]), path_of(&pair[1]).as_deref())
        }).collect::<Vec<_>>(),
        "outcome": outcome("Patient", &input["outcomeIssues"].as_array().unwrap().iter().map(issue_of).collect::<Vec<_>>()),
        "refused": input["refusals"].as_array().unwrap().iter().map(|pair| {
            Refused::new(pair[0].as_u64().unwrap() as u16, pair[1].as_array().unwrap().iter().map(issue_of).collect()).message()
        }).collect::<Vec<_>>(),
        "concepts": input["concepts"].as_array().unwrap().iter().map(concept_text).collect::<Vec<_>>(),
        "names": input["names"].as_array().unwrap().iter().map(human_name).collect::<Vec<_>>(),
        "quantities": input["quantities"].as_array().unwrap().iter().map(quantity_text).collect::<Vec<_>>(),
        "references": input["references"].as_array().unwrap().iter().map(reference_text).collect::<Vec<_>>(),
        "base64": texts(&input["texts"]).iter().map(|one| {
            let data = to_base64(one);
            json!([data, from_base64(&data)])
        }).collect::<Vec<_>>(),
    });
    same(&ours, &core()["model"], "model");
}

fn element_json(element: &ElementDef) -> Json {
    let mut out = json!({ "name": element.name, "type": element.kind, "doc": element.doc });
    if element.array {
        out["array"] = json!(true);
    }
    if element.required {
        out["required"] = json!(true);
    }
    if let Some(codes) = element.codes {
        out["codes"] = json!(codes);
    }
    if let Some(targets) = element.targets {
        out["targets"] = json!(targets);
    }
    out
}

#[test]
fn types_are_the_webs() {
    let mut datatypes = Map::new();
    for name in DATATYPE_NAMES {
        datatypes.insert(
            name.to_owned(),
            Json::Array(datatype(name).unwrap().iter().map(element_json).collect()),
        );
    }
    let mut names: Vec<&str> = TYPE_NAMES.to_vec();
    names.push("Nothing");
    let ours = json!({
        "names": TYPE_NAMES,
        "types": types().iter().map(|one| json!({
            "name": one.name, "about": one.about, "elements": one.elements.iter().map(element_json).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "datatypes": datatypes,
        "schemas": names.iter().map(|name| schema_for(name).unwrap_or(Json::Null)).collect::<Vec<_>>(),
        "patterns": inputs()["patterns"].as_array().unwrap().iter().map(|pair| {
            let kind = text(&pair[0]);
            if pattern_text(kind).is_some() { json!(matches_pattern(kind, text(&pair[1]))) } else { Json::Null }
        }).collect::<Vec<_>>(),
    });
    same(&ours, &core()["types"], "types");
}

#[test]
fn seed_is_the_webs() {
    let mut names: Vec<&str> = TYPE_NAMES.to_vec();
    names.push("Nothing");
    let ours = json!({
        "vitals": VITALS,
        "templates": names.iter().map(|name| template(name)).collect::<Vec<_>>(),
        "seeded": seed().iter().map(|one| {
            let mut out = json!({ "resource": one.resource });
            if !one.earlier.is_empty() {
                out["earlier"] = json!(one.earlier.iter().map(|(at, resource)| json!({ "at": at, "resource": resource })).collect::<Vec<_>>());
            }
            out
        }).collect::<Vec<_>>(),
    });
    same(&ours, &core()["seed"], "seed");
}

fn seed_exists(key: &str) -> bool {
    resources_seed().iter().any(|row| row.id == key)
}

#[test]
fn validation_is_the_webs() {
    let validations: Vec<Json> = inputs()["validations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|input| {
            to_json(&validate(
                &resource_of(input),
                text(&input["type"]),
                opt_str(&input["id"]),
                &seed_exists,
            ))
        })
        .collect();
    same(&json!(validations), &core()["validations"], "validations");
    let every: Vec<Json> = seed_resources()
        .iter()
        .map(|resource| {
            to_json(&validate(
                resource,
                &fhir::model::type_of(resource),
                Some(&fhir::model::id_of(resource)),
                &seed_exists,
            ))
        })
        .collect();
    same(&json!(every), &core()["everySeed"], "everySeed");
    let headers: Vec<Json> = texts(&inputs()["cql"])
        .iter()
        .map(|one| {
            cql_header(one).map_or(Json::Null, |header| {
                let mut out = json!({ "name": header.name, "line": header.line });
                if let Some(version) = header.version {
                    out["version"] = json!(version);
                }
                out
            })
        })
        .collect();
    same(&json!(headers), &core()["headers"], "headers");
}

fn bundle_of(rows: &[Stored]) -> Json {
    pages::server_bundle(rows)
}

#[test]
fn fhirpath_is_the_webs() {
    let now = inputs()["now"].as_i64().unwrap();
    let rows = resources_seed();
    let subject = |name: &str| {
        if name == "bundle" {
            Some(bundle_of(&rows))
        } else {
            by_key(name)
        }
    };
    let ours = json!({
        "evaluations": inputs()["fhirpath"].as_array().unwrap().iter().map(|pair| {
            to_json(&evaluate(text(&pair[1]), subject(text(&pair[0])).as_ref(), now))
        }).collect::<Vec<_>>(),
        "none": to_json(&evaluate("id", None, now)),
        "functions": functions(),
        "variables": variables_offered(),
        "elementNames": texts(&inputs()["elementNames"]).iter().map(|name| {
            let value = if name == "bundle" {
                Json::Array(seed_resources().into_iter().take(80).collect())
            } else {
                by_key(name).unwrap_or(Json::Null)
            };
            to_json(&element_names(&value))
        }).collect::<Vec<_>>(),
        "words": texts(&inputs()["words"]).iter().map(|word| describe_word(word)).collect::<Vec<_>>(),
    });
    same(&ours, &core()["fhirpath"], "fhirpath");
}

#[test]
fn search_is_the_webs() {
    let rows = resources_seed();
    let mut names: Vec<&str> = TYPE_NAMES.to_vec();
    names.push("Nothing");
    let ours = json!({
        "params": names.iter().map(|name| to_json(&params_of(name))).collect::<Vec<_>>(),
        "resultParams": RESULT_PARAMS,
        "queries": texts(&inputs()["queries"]).iter().map(|one| {
            let (kind, params) = read_query(one);
            json!({ "type": kind, "params": params })
        }).collect::<Vec<_>>(),
        "results": inputs()["searches"].as_array().unwrap().iter().map(|pair| {
            let result = search(&rows, text(&pair[0]), pair[1].as_f64().unwrap());
            let mut out = to_json(&result);
            out["matches"] = json!(result.matches.iter().map(|row| row.id.clone()).collect::<Vec<_>>());
            out
        }).collect::<Vec<_>>(),
    });
    same(&ours, &core()["search"], "search");
}

#[test]
fn server_is_the_webs() {
    let rows = resources_seed();
    let ours = json!({
        "server": { "name": SERVER_NAME, "base": SERVER_BASE, "address": SERVER_ADDRESS, "version": SERVER_VERSION },
        "me": ME,
        "pointing": texts(&inputs()["pointing"]).iter().map(|key| pointing_at(&rows, key)).collect::<Vec<_>>(),
        "parsed": texts(&inputs()["json"]).iter().map(|one| match parse(one) {
            Ok(value) => json!({ "ok": value }),
            Err(refused) => json!({ "refused": { "status": refused.status, "issues": refused.issues, "message": refused.message() } }),
        }).collect::<Vec<_>>(),
    });
    same(&ours, &core()["server"], "server");
}

#[test]
fn diffs_are_the_webs() {
    let ours: Vec<Json> = inputs()["diffs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| {
            let lines = diff_lines(text(&one[0]), text(&one[1]));
            let rows = fold_diff(&lines, one[2].as_u64().unwrap() as usize, &[]);
            json!({ "lines": lines, "rows": rows })
        })
        .collect();
    same(&json!(ours), &core()["diffs"], "diffs");
}

#[test]
fn ui_is_the_webs() {
    let mut status_map = Map::new();
    for (name, style) in statuses() {
        status_map.insert(name.to_owned(), to_json(&style));
    }
    let titled: Vec<Json> = inputs()["titled"].as_array().unwrap().iter().map(resource_of).collect();
    let ours = json!({
        "statuses": status_map,
        "when": inputs()["instants"].as_array().unwrap().iter().map(|one| when(one.as_str())).collect::<Vec<_>>(),
        "onDay": inputs()["days"].as_array().unwrap().iter().map(|one| on_day(one.as_str())).collect::<Vec<_>>(),
        "counts": inputs()["counts"].as_array().unwrap().iter().map(|one| {
            count(one[0].as_u64().unwrap() as usize, text(&one[1]), one[2].as_str())
        }).collect::<Vec<_>>(),
        "titles": seed_resources().iter().map(title_of).collect::<Vec<_>>(),
        "states": seed_resources().iter().map(state_of).collect::<Vec<_>>(),
        "extraTitles": titled.iter().map(title_of).collect::<Vec<_>>(),
        "extraStates": titled.iter().map(state_of).collect::<Vec<_>>(),
        "prefs": Prefs::default(),
    });
    same(&ours, &core()["ui"], "ui");
    let _ = parse_instant;
    let _ = ui::number;
}

#[test]
fn themes_are_the_webs() {
    same(&json!(CARRIED), &core()["themes"], "themes");
}

// --- The app, step by step ----------------------------------------------------------------------------

/// The text an edit step leaves: its own, or the editor's with a part replaced, or its JSON with
/// fields set and dropped.
fn edited(current: &str, step: &Json) -> String {
    if let Some(value) = step["value"].as_str() {
        return value.to_owned();
    }
    if let Some(pair) = step["replace"].as_array() {
        return current.replacen(text(&pair[0]), text(&pair[1]), 1);
    }
    let mut parsed: Json = fhir::js::parse_json(current).expect("the editor's JSON");
    if let Some(fields) = parsed.as_object_mut() {
        for (name, value) in step["set"].as_object().cloned().unwrap_or_default() {
            fields.insert(name, value);
        }
        for name in step["drop"].as_array().cloned().unwrap_or_default() {
            fields.shift_remove(text(&name));
        }
    }
    pretty(&parsed)
}

/// The values the app keeps with `useStored`, as the browser keeps them.
const STORED: [&str; 6] = [
    "prefs",
    "search-query",
    "search-ran",
    "search-side",
    "fhirpath-expression",
    "fhirpath-subject",
];

struct Harness {
    app: App,
    seed_rows: Vec<Stored>,
    seed_history: usize,
}

impl Harness {
    fn new() -> Self {
        Harness {
            app: App::new(inputs()["now"].as_i64().unwrap()),
            seed_rows: resources_seed(),
            seed_history: history_seed().len(),
        }
    }

    fn page(&self) -> Page {
        self.app.page_of()
    }

    fn press(&mut self, label: &str) {
        let page = self.page();
        let app = &mut self.app;
        // A button with a reason cannot be pressed.
        let reason = |buttons: &[pages::ButtonLook]| {
            buttons
                .iter()
                .find(|one| one.label == label)
                .unwrap_or_else(|| panic!("no button {label}"))
                .reason
                .is_some()
        };
        match page {
            Page::Resource(kind, id) => {
                if reason(&pages::resource_look(app, &kind, &id).buttons) {
                    return;
                }
                match label {
                    "FHIRPath" => app.go(&format!("/fhirpath/{kind}/{id}")),
                    "Delete" => {
                        if let PageState::Resource { deleting, .. } = &mut app.page {
                            *deleting = true;
                        }
                    }
                    "Revert" => app.editing.set_draft(&format!("{kind}/{id}"), None),
                    _ => pages::resource_save(app, &kind, &id),
                }
            }
            Page::Resources(kind) => match label {
                "New" => app.go(&format!("/resources/{kind}/new")),
                _ => panic!("no button {label}"),
            },
            Page::CqlLibraries => pages::make_library(app),
            Page::CqlEditor(id) => {
                if reason(&pages::cql_look(app, &id).buttons) {
                    return;
                }
                match label {
                    "Library resource" => app.go(&format!("/resources/Library/{id}")),
                    "Revert" => app.editing.set_draft(&format!("Library/{id}#cql"), None),
                    "Publish" => pages::cql_save(app, &id, Some("active")),
                    _ => pages::cql_save(app, &id, None),
                }
            }
            Page::FhirPath(subject) => match label {
                "Evaluate" => pages::fhirpath_run(app, subject.as_deref()),
                _ => panic!("no button {label}"),
            },
            Page::Search => {
                let look = pages::search_look(app);
                if reason(&look.buttons) {
                    return;
                }
                let result = pages::search_result(app);
                match label {
                    "Save" => {
                        if let PageState::Search(state) = &mut app.page {
                            state.saving = true;
                        }
                    }
                    "Search" => pages::search_run(app, None),
                    "Previous page" => {
                        let result = result.expect("a result");
                        pages::search_page(app, (result.offset - result.count).max(0.0));
                    }
                    "Next page" => {
                        let result = result.expect("a result");
                        pages::search_page(app, result.offset + result.count);
                    }
                    _ => panic!("no button {label}"),
                }
            }
            _ => panic!("no button {label} here"),
        }
    }

    fn act(&mut self, step: &Json) {
        let op = text(&step["op"]);
        if op == "button" {
            self.press(text(&step["label"]));
            self.app.settle();
            return;
        }
        let page = self.page();
        let app = &mut self.app;
        match op {
            "go" => app.go(text(&step["path"])),
            "prefs" => app.set_pref(text(&step["id"]), &step["value"]),
            "stat" => app.go(&pages::stat_route(text(&step["label"]))),
            "open" => {
                let id = text(&step["id"]);
                let table = step["table"].as_u64().unwrap_or(0);
                match page {
                    Page::Overview => {
                        let versions = pages::recent(app.backend.history.seen());
                        let version = versions.iter().find(|one| one.id == id).expect("a change");
                        let route = pages::change_route(version);
                        app.go(&route);
                    }
                    Page::Resources(_) => app.go(&format!("/resources/{id}")),
                    Page::CqlLibraries => {
                        let row = app.rows().iter().find(|row| row.id == id).expect("a library").clone();
                        app.go(&format!("/cql/{}", fhir::model::id_of(&row.resource)));
                    }
                    Page::CqlEditor(_) => {
                        let version = app
                            .backend
                            .history
                            .seen()
                            .iter()
                            .find(|one| one.id == id)
                            .expect("a version")
                            .version_id;
                        if let PageState::CqlEditor { compared, .. } = &mut app.page {
                            *compared = Some(version);
                        }
                    }
                    Page::Search => {
                        if table == 0 {
                            app.go(&format!("/resources/{id}"));
                        } else if pages::search_side(app) == "saved" {
                            let one = pages::searches(app)
                                .into_iter()
                                .find(|one| one.id == id)
                                .expect("a search");
                            pages::search_set(app, &one.query);
                        } else {
                            pages::search_add_param(app, id);
                        }
                    }
                    _ => panic!("nothing to open on {page:?}"),
                }
            }
            "search" => {
                if let PageState::Resources { query } = &mut app.page {
                    *query = text(&step["value"]).to_owned();
                }
            }
            "segment" => {
                let value = text(&step["value"]);
                match page {
                    Page::Resources(_) => app.go(&format!("/resources/{value}")),
                    Page::Search => {
                        if step["index"].as_u64().unwrap_or(0) == 0 {
                            if let PageState::Search(state) = &mut app.page {
                                state.view = value.to_owned();
                            }
                        } else {
                            app.kept.set("search-side", json!(value));
                        }
                    }
                    _ => panic!("no segments here"),
                }
            }
            "edit" => match page {
                Page::Resource(kind, id) => {
                    let current = pages::resource_view(app, &kind, &id).text;
                    pages::resource_edit(app, &kind, &id, &edited(&current, step));
                }
                Page::CqlEditor(id) => {
                    let current = pages::cql_view(app, &id).text;
                    pages::cql_edit(app, &id, &edited(&current, step));
                }
                Page::FhirPath(_) => {
                    let current = app.kept.text("fhirpath-expression", pages::FHIRPATH_EXAMPLES[0].2);
                    app.kept.set("fhirpath-expression", json!(edited(&current, step)));
                }
                Page::Search => {
                    let current = pages::search_query(app);
                    app.kept.set("search-query", json!(edited(&current, step)));
                }
                _ => panic!("no editor here"),
            },
            "run" => pages::search_run(app, step["value"].as_str()),
            "confirm" => {
                if let Page::Resource(kind, id) = page {
                    pages::resource_remove(app, &kind, &id);
                }
            }
            "cancel" => {
                if let PageState::Resource { deleting, .. } = &mut app.page {
                    *deleting = false;
                }
            }
            "example" => match page {
                Page::FhirPath(subject) => pages::fhirpath_example(app, subject.as_deref(), text(&step["label"])),
                Page::Search => {
                    let label = text(&step["label"]);
                    let query = pages::SEARCH_EXAMPLES
                        .iter()
                        .find(|one| one.0 == label)
                        .expect("an example")
                        .1;
                    pages::search_set(app, query);
                }
                _ => panic!("no examples here"),
            },
            "subject" => {
                if let Page::FhirPath(subject) = page {
                    pages::fhirpath_choose(app, subject.as_deref(), text(&step["value"]));
                }
            }
            "version" => {
                if let Some(panel) = &mut app.panel {
                    panel.chosen = step["value"].as_i64();
                }
            }
            "reference" => app.go(&format!("/resources/{}", text(&step["key"]))),
            "remove" => pages::search_remove(app, text(&step["id"])),
            "reorder" => pages::search_reorder(
                app,
                step["from"].as_u64().unwrap() as usize,
                step["to"].as_u64().unwrap() as usize,
            ),
            "label" => {
                if let PageState::Search(state) = &mut app.page {
                    state.label = text(&step["value"]).to_owned();
                }
            }
            "dialog" => {
                if step["close"] == json!(true) {
                    if let PageState::Search(state) = &mut app.page {
                        state.saving = false;
                    }
                } else {
                    pages::search_save(app);
                }
            }
            "server" => {
                let resource = resource_of(&step["resource"]);
                let ticket = match text(&step["call"]) {
                    "update" => app.backend.update(
                        text(&step["type"]),
                        text(&step["id"]),
                        resource,
                        step["readAt"].as_f64().unwrap(),
                    ),
                    "create" => app.backend.create(text(&step["type"]), resource),
                    _ => app.backend.remove(text(&step["key"])),
                };
                app.wait(ticket, Then::Record);
            }
            "wait" => {}
            other => panic!("no step {other}"),
        }
        app.settle();
    }

    // --- Reading ---

    fn frame_look(&self) -> Json {
        let route = self.app.route();
        let (place, _) = app::place_of(&route);
        let rows = self.app.rows();
        let groups = app::rail_groups(place, app::resource_type_of(&route).as_deref(), &app::counts(rows));
        json!({
            "rail": to_json(&groups),
            "trail": to_json(&app::trail(&route)),
            "palette": to_json(&app::palette(&groups)),
            "search": texts(&inputs()["paletteSearches"]).iter().map(|query| to_json(&app::search(query, rows))).collect::<Vec<_>>(),
            "contentKey": app::content_key(&route),
            "settingsOpen": false,
        })
    }

    fn view_look(&self) -> Json {
        let app = &self.app;
        let page = self.page();
        let rows = app.rows();
        let mut out = match &page {
            Page::Overview => {
                let measured = pages::measured(rows);
                let mut out = json!({
                    "header": to_json(&pages::overview_header()),
                    "loading": !app.backend.resources.loaded(),
                    "stats": pages::overview_stats(rows).iter().map(|(label, value)| json!({ "label": label, "value": value })).collect::<Vec<_>>(),
                    "chart": {
                        "title": "Vital signs measured", "subtitle": "per month, by measurement",
                        "label": "Vital signs measured per month by measurement", "rows": to_json(&measured),
                        "x": "month", "y": "count", "series": "vital",
                    },
                    "changes": to_json(&pages::changes_table(&pages::recent(app.backend.history.seen()))),
                });
                out["foot"] = to_json(&pages::overview_foot(rows));
                out
            }
            Page::Resources(kind) => {
                let query = match &app.page {
                    PageState::Resources { query } => query.clone(),
                    _ => String::new(),
                };
                let look = pages::resources_look(kind, rows, &query);
                json!({
                    "header": to_json(&pages::resources_header()),
                    "api": format!("/{kind}"),
                    "loading": !app.backend.resources.loaded(),
                    "query": query,
                    "types": look.types.iter().map(|(value, label, count)| json!({ "value": value, "label": label, "count": count })).collect::<Vec<_>>(),
                    "type": kind,
                    "table": to_json(&look.table),
                    "sortValues": to_json(&look.sort_values),
                    "defaultSort": { "column": "updated", "direction": "desc" },
                    "foot": to_json(&look.foot),
                })
            }
            Page::Resource(kind, id) => to_json(&pages::resource_look(app, kind, id)),
            Page::FhirPath(subject) => to_json(&pages::fhirpath_look(app, subject.as_deref())),
            Page::CqlLibraries => {
                let busy = matches!(app.page, PageState::CqlLibraries { busy: true });
                json!({
                    "header": to_json(&pages::libraries_header()),
                    "buttons": [{ "label": "New", "reason": null, "busy": busy }],
                    "table": to_json(&pages::libraries_table(rows)),
                    "foot": {
                        "note": count(rows.iter().filter(|row| row.kind == "Library").count(), "library", Some("libraries")),
                        "items": null,
                    },
                })
            }
            Page::CqlEditor(id) => to_json(&pages::cql_look(app, id)),
            Page::Search => to_json(&pages::search_look(app)),
            Page::NotFound => json!({ "foot": { "note": null, "items": null } }),
        };
        out["page"] = json!(page.name());
        out
    }

    fn panel_look(&self) -> Json {
        match self.page() {
            Page::Resource(kind, id) => to_json(&pages::panel_look(&self.app, &kind, &id)),
            _ => Json::Null,
        }
    }

    fn server_look(&self) -> Json {
        let backend = &self.app.backend;
        let rows = &backend.resources.rows;
        let history = &backend.history.rows;
        json!({
            "resources": rows.iter().map(|row| format!("{}@{}", row.id, fhir::model::version_id(&row.resource).unwrap_or_default())).collect::<Vec<_>>(),
            "touched": rows.iter().filter(|row| !self.seed_rows.contains(row)).map(to_json).collect::<Vec<_>>(),
            "history": if backend.history.written { to_json(&history[..history.len() - self.seed_history].to_vec()) } else { json!([]) },
            "saved": if backend.saved.written { to_json(&backend.saved.rows) } else { Json::Null },
        })
    }

    fn look(&mut self) -> Json {
        let app = &self.app;
        let mut stored = Map::new();
        for key in STORED {
            if let Some(value) = app.kept.get(key) {
                stored.insert(key.to_owned(), value.clone());
            }
        }
        let mut drafts = Map::new();
        for (key, draft) in &app.editing.drafts {
            drafts.insert(key.clone(), to_json(draft));
        }
        let mut answers = Map::new();
        for (key, issues) in &app.editing.answers {
            answers.insert(key.clone(), to_json(issues));
        }
        let out = json!({
            "route": app.hash,
            "server": self.server_look(),
            "stores": { "drafts": drafts, "answers": answers, "reveal": null, "stored": stored },
            "toasts": to_json(&app.toasts),
            "went": app.went,
            "calls": app.calls,
            "frame": self.frame_look(),
            "view": self.view_look(),
            "panel": self.panel_look(),
        });
        self.app.toasts.clear();
        self.app.went.clear();
        self.app.calls.clear();
        out
    }
}

/// The keys whose values changed since the last look, as the dump keeps them.
fn changed(seen: Json, last: &mut Map<String, Json>) -> Json {
    let seen = norm(seen);
    let mut out = Map::new();
    let mut written = Map::new();
    for (key, value) in seen.as_object().expect("a look") {
        if last.get(key) != Some(value) {
            out.insert(key.clone(), value.clone());
        }
        written.insert(key.clone(), value.clone());
    }
    *last = written;
    Json::Object(out)
}

#[test]
fn steps_are_the_webs() {
    let web = web()["steps"].as_array().expect("steps");
    let now = inputs()["now"].as_i64().unwrap();
    let mut harness = Harness::new();
    let mut last = Map::new();
    harness.app.advance(now + 1800);
    let first = changed(harness.look(), &mut last);
    same(&first, &web[0], "steps[0]");
    for (at, step) in inputs()["steps"].as_array().unwrap().iter().enumerate() {
        let when = step["at"].as_i64().unwrap();
        harness.app.advance(now + when);
        harness.act(step);
        harness.app.advance(now + when + step["wait"].as_i64().unwrap_or(900));
        let look = changed(harness.look(), &mut last);
        same(&look, &web[at + 1], &format!("steps[{}] {step}", at + 1));
    }
    assert_eq!(web.len(), inputs()["steps"].as_array().unwrap().len() + 1);
    let _ = SEARCH_MIN_QUERY;
}
