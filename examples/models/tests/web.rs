//! The data layer held against the web's own: `web/dump.ts` runs the web example's modules
//! (`data/*`, `engine/*`, `actions.tsx`, `pages/pull.tsx`, `themes.ts` and sokkel-ui's
//! `ConversationHistory`, the engines started as the web's `App` starts them)
//! under bun on the inputs in `web/inputs.json`, the zone UTC, the clock and the timers held and
//! `Math.random` a seeded run, and writes what they give as JSON. Each test builds the same JSON
//! here and asks for it to be equal, every number to the bit.
//!
//! The web's answer is kept in `web/data.json`. Where bun and `.parity/sokkel-ui` are at hand the
//! dump is made again and must match it, so a change on either side shows (`MODELS_WEB_KEEP=1`
//! keeps the new one); elsewhere (GitHub's CI) the kept answer is read.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use std::cell::RefCell;
use std::rc::Rc;

use models::data::backend::{Backend, LibraryView};
use models::data::catalog::{
    CatalogQuery, QUANTS, catalog, entry_of, files_of, model_id, search_catalog, search_delay,
};
use models::data::disk::{disk_of, model_path, usage_of, valid_root};
use models::data::format::{
    bytes, clock, clock_seconds, context, counted, counted_as, day, params, remaining, speed, uptime,
};
use models::data::model::{
    Attachment, CAPABILITIES, Comparison, Conversation, Download, FORMATS, Feedback, Format, Loaded, Model, Prefs,
    SOURCE_IDS, SamplingPatch, Source, default_prefs, default_sampling,
};
use models::data::seed::model_memory;
use models::engine::generate::{Ask, pieces, plan, prefill_of, rate_of, title_of, tokens};
use models::pages::chat::{conversation_group, conversation_time};
use models::pages::pull::{pull_of, usual_variant};
use models::runtime::{Host, Promise, Value};
use models::themes::CARRIED;
use serde_json::{Map, Value as Json, json};

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
            .join(".parity/sokkel-ui/examples/models/web/node_modules")
            .exists()
            .then(|| {
                Command::new("bun")
                    .arg("examples/models/tests/web/dump.ts")
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
                if std::env::var_os("MODELS_WEB_KEEP").is_some() {
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
                            panic!("web/data.json is not what the web gives now; run with MODELS_WEB_KEEP=1 to keep it")
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

fn now() -> i64 {
    inputs()["now"].as_i64().expect("now")
}

fn rows_json<T: serde::Serialize>(rows: &[T]) -> Json {
    to_json(&rows)
}

fn numbers(value: &Json) -> Vec<f64> {
    value
        .as_array()
        .expect("numbers")
        .iter()
        .map(|one| one.as_f64().unwrap_or(f64::INFINITY))
        .collect()
}

// --- What the modules give ------------------------------------------------------------------------------

#[test]
fn model_and_themes_are_the_webs() {
    let model = &core()["model"];
    same(&json!(FORMATS.map(|one| one.key())), &model["formats"], "formats");
    let notes: Map<String, Json> = FORMATS
        .iter()
        .map(|one| (one.key().to_owned(), json!(one.note())))
        .collect();
    same(&Json::Object(notes), &model["formatNotes"], "formatNotes");
    let sources: Map<String, Json> = SOURCE_IDS
        .iter()
        .map(|one| {
            (
                one.key().to_owned(),
                json!({"label": one.label(), "detail": one.detail()}),
            )
        })
        .collect();
    same(&Json::Object(sources), &model["sources"], "sources");
    same(
        &json!(SOURCE_IDS.map(|one| one.key())),
        &model["sourceIds"],
        "sourceIds",
    );
    same(
        &json!(CAPABILITIES.map(|one| one.key())),
        &model["capabilities"],
        "capabilities",
    );
    same(&to_json(&default_sampling()), &model["sampling"], "sampling");
    same(&to_json(&default_prefs()), &model["prefs"], "prefs");
    same(&json!(CARRIED), &core()["themes"], "themes");
}

#[test]
fn catalog_is_the_webs() {
    same(&to_json(&catalog()), &core()["catalog"], "catalog");
    same(
        &json!(QUANTS.map(|(quant, said)| [quant, said])),
        &core()["quants"],
        "quants",
    );
    let ids: Vec<String> = inputs()["modelIds"]
        .as_array()
        .expect("ids")
        .iter()
        .map(|pair| model_id(text(&pair[0]), text(&pair[1])))
        .collect();
    same(&json!(ids), &core()["modelIds"], "modelIds");
    let files: Vec<Vec<Json>> = catalog()
        .iter()
        .map(|entry| {
            entry
                .variants
                .iter()
                .map(|variant| to_json(&files_of(entry, variant)))
                .collect()
        })
        .collect();
    same(&json!(files), &core()["files"], "files");
    let entries: Vec<Option<String>> = texts(&inputs()["entryIds"])
        .iter()
        .map(|id| entry_of(id).map(|entry| entry.id.clone()))
        .collect();
    same(&json!(entries), &core()["entries"], "entries");
    let usual: Vec<Json> = catalog().iter().map(|entry| to_json(usual_variant(entry))).collect();
    same(&json!(usual), &core()["usual"], "usual");
    let searches: Vec<Json> = inputs()["searches"]
        .as_array()
        .expect("searches")
        .iter()
        .map(|query| {
            let query = CatalogQuery {
                text: text(&query["text"]).to_owned(),
                source: Source::from_key(text(&query["source"])),
                format: Format::from_key(text(&query["format"])),
            };
            let ids: Vec<&str> = search_catalog(&query).iter().map(|entry| entry.id.as_str()).collect();
            json!({"ids": ids, "delay": search_delay(&query)})
        })
        .collect();
    same(&json!(searches), &core()["searches"], "searches");
}

#[test]
fn seed_is_the_webs() {
    let backend = Backend::new(now());
    let seed = &core()["seed"];
    same(&rows_json(&backend.models.rows), &seed["library"], "library");
    same(
        &rows_json(&backend.conversations.rows),
        &seed["conversations"],
        "conversations",
    );
    same(&rows_json(&backend.downloads.rows), &seed["downloads"], "downloads");
    same(&rows_json(&backend.runner.rows), &seed["runner"], "runner");
    same(&rows_json(&backend.loaded.rows), &seed["loaded"], "loaded");
    same(&rows_json(&backend.events.rows), &seed["events"], "events");
    same(
        &rows_json(&backend.comparisons.rows),
        &seed["comparisons"],
        "comparisons",
    );
    same(&rows_json(&backend.prefs.rows), &seed["prefs"], "prefs");
    same(&rows_json(&backend.views.rows), &seed["views"], "views");
    let memory: Vec<f64> = backend.models.rows.iter().map(model_memory).collect();
    same(&json!(memory), &core()["memory"], "memory");
}

#[test]
fn format_and_disk_are_the_webs() {
    let format = &core()["format"];
    let given = numbers(&inputs()["numbers"]);
    same(
        &json!(given.iter().map(|one| bytes(*one)).collect::<Vec<_>>()),
        &format["bytes"],
        "bytes",
    );
    same(
        &json!(given.iter().map(|one| speed(*one)).collect::<Vec<_>>()),
        &format["speed"],
        "speed",
    );
    same(
        &json!(given.iter().map(|one| context(*one)).collect::<Vec<_>>()),
        &format["context"],
        "context",
    );
    let said: Vec<String> = numbers(&inputs()["params"]).into_iter().map(params).collect();
    same(&json!(said), &format["params"], "params");
    let said: Vec<[String; 2]> = numbers(&inputs()["counts"])
        .into_iter()
        .map(|one| [counted(one, "model"), counted_as(one, "day", "days")])
        .collect();
    same(&json!(said), &format["counted"], "counted");
    let said: Vec<String> = numbers(&inputs()["seconds"]).into_iter().map(remaining).collect();
    same(&json!(said), &format["remaining"], "remaining");
    let times: Vec<i64> = numbers(&inputs()["times"]).into_iter().map(|one| one as i64).collect();
    same(
        &json!(times.iter().map(|one| day(*one)).collect::<Vec<_>>()),
        &format["day"],
        "day",
    );
    same(
        &json!(times.iter().map(|one| clock(*one)).collect::<Vec<_>>()),
        &format["clock"],
        "clock",
    );
    same(
        &json!(times.iter().map(|one| clock_seconds(*one)).collect::<Vec<_>>()),
        &format["clockSeconds"],
        "clockSeconds",
    );
    let said: Vec<String> = numbers(&inputs()["durations"]).into_iter().map(uptime).collect();
    same(&json!(said), &format["uptime"], "uptime");

    let backend = Backend::new(now());
    let library = &backend.models.rows;
    let downloads: Vec<Download> = serde_json::from_value(inputs()["downloads"].clone()).expect("downloads");
    let disks: Vec<Json> = texts(&inputs()["roots"])
        .iter()
        .map(|root| {
            json!({
                "disk": to_json(&disk_of(root)),
                "usage": to_json(&usage_of(root, library, &downloads)),
                "path": model_path(root, &library[0]),
                "valid": valid_root(root),
            })
        })
        .collect();
    same(&json!(disks), &core()["disk"], "disk");
    let pulls: Vec<Option<String>> = inputs()["pullOf"]
        .as_array()
        .expect("pulls")
        .iter()
        .map(|pair| pull_of(&downloads, text(&pair[0]), text(&pair[1])).map(|one| one.id.clone()))
        .collect();
    same(&json!(pulls), &core()["pulls"], "pulls");
    let history: Vec<Json> = numbers(&inputs()["historyTimes"])
        .into_iter()
        .map(|time| {
            let time = time as i64;
            json!([
                conversation_group(false, time, now()),
                conversation_group(true, time, now()),
                conversation_time(time, now())
            ])
        })
        .collect();
    same(&json!(history), &core()["history"], "history");
}

#[test]
fn generate_is_the_webs() {
    let generate = &core()["generate"];
    let given = texts(&inputs()["texts"]);
    same(
        &json!(given.iter().map(|one| tokens(one)).collect::<Vec<_>>()),
        &generate["tokens"],
        "tokens",
    );
    same(
        &json!(given.iter().map(|one| pieces(one)).collect::<Vec<_>>()),
        &generate["pieces"],
        "pieces",
    );
    same(
        &json!(given.iter().map(|one| title_of(one)).collect::<Vec<_>>()),
        &generate["titles"],
        "titles",
    );
    let backend = Backend::new(now());
    let library = &backend.models.rows;
    let rates: Vec<[f64; 3]> = library
        .iter()
        .map(|one| [rate_of(one), prefill_of(one, 0.0), prefill_of(one, 1234.0)])
        .collect();
    same(&json!(rates), &generate["rates"], "rates");
    for (index, ask) in inputs()["plans"].as_array().expect("plans").iter().enumerate() {
        let mut settings = to_json(&default_sampling());
        for (key, value) in ask["settings"].as_object().expect("settings") {
            settings[key] = value.clone();
        }
        let asked = Ask {
            prompt: text(&ask["prompt"]).to_owned(),
            model: library
                .iter()
                .find(|one| one.id == text(&ask["model"]))
                .expect("a model")
                .clone(),
            settings: serde_json::from_value(settings).expect("settings"),
            attachments: serde_json::from_value(ask["attachments"].clone()).expect("attachments"),
            history: ask["history"].as_f64().expect("history"),
            attempt: ask["attempt"].as_i64().expect("attempt"),
        };
        same(
            &to_json(&plan(&asked)),
            &generate["plans"][index],
            &format!("plans[{index}]"),
        );
    }
}

// --- The steps ------------------------------------------------------------------------------------------

/// The id of what a promise was fulfilled with, as the dump writes `value?.id ?? null`.
fn id_of(value: &Value) -> Json {
    macro_rules! try_id {
        ($($kind:ty),*) => {
            $(if let Some(row) = value.downcast_ref::<$kind>() {
                return json!(row.id);
            })*
        };
    }
    try_id!(Conversation, Comparison, Download, Loaded, Model, Prefs, LibraryView);
    Json::Null
}

struct Player {
    backend: Backend,
    settled: Rc<RefCell<Vec<Json>>>,
    pages: Vec<Json>,
    last: Map<String, Json>,
}

impl Player {
    fn follow(&mut self, what: &str, promise: Promise) {
        let settled = self.settled.clone();
        let what = what.to_owned();
        self.backend.on(promise, move |_, outcome| {
            settled.borrow_mut().push(match outcome {
                Ok(value) => json!({"what": what, "ok": id_of(&value)}),
                Err(error) => json!({"what": what, "error": error}),
            });
        });
    }

    fn conversation_of(&self, reference: &Json) -> String {
        match reference.as_u64() {
            Some(index) => self.backend.conversations.rows[index as usize].id.clone(),
            None => text(reference).to_owned(),
        }
    }

    fn comparison_of(&self, reference: &Json) -> String {
        match reference.as_u64() {
            Some(index) => self.backend.comparisons.rows[index as usize].id.clone(),
            None => text(reference).to_owned(),
        }
    }

    fn pull_id(&self, step: &Json) -> String {
        self.backend
            .downloads
            .rows
            .iter()
            .find(|one| one.entry == text(&step["entry"]) && one.quant == text(&step["quant"]))
            .map(|one| one.id.clone())
            .unwrap_or_default()
    }

    fn model(&self, id: &str) -> Model {
        self.backend.models.get(id).expect("a model in the library").clone()
    }

    fn message_of(&self, conversation: &str, index: &Json) -> String {
        let found = self.backend.conversations.get(conversation).expect("a conversation");
        found.messages[index.as_u64().expect("an index") as usize]
            .id()
            .to_owned()
    }

    fn act(&mut self, step: &Json) {
        let op = text(&step["op"]);
        let index = || step["index"].as_u64().expect("an index") as usize;
        match op {
            "pull" => {
                let entry = entry_of(text(&step["entry"])).expect("an entry");
                let variant = entry
                    .variants
                    .iter()
                    .find(|one| one.quant == text(&step["quant"]))
                    .expect("a variant");
                let promise = self.backend.pull(entry, variant);
                self.follow(op, promise);
            }
            "pause" | "resume" | "cancel" | "retry" | "removePull" => {
                let id = self.pull_id(step);
                let promise = match op {
                    "pause" => self.backend.pause_pull(&id),
                    "resume" => self.backend.resume_pull(&id),
                    "cancel" => self.backend.cancel_pull(&id),
                    "retry" => self.backend.retry_pull(&id),
                    _ => self.backend.remove_pull(&id),
                };
                self.follow(op, promise);
            }
            "clearEnded" => {
                let promise = self.backend.clear_ended();
                self.follow(op, promise);
            }
            "prefs" => {
                let patch = step["patch"].clone();
                let promise = self.backend.update(
                    |b| &mut b.prefs,
                    "prefs",
                    move |row| {
                        let mut value = serde_json::to_value(&*row).expect("prefs");
                        for (key, one) in patch.as_object().expect("a patch") {
                            value[key] = one.clone();
                        }
                        *row = serde_json::from_value(value).expect("prefs");
                    },
                );
                self.follow(op, promise);
            }
            "startRunner" => {
                let promise = self.backend.start_runner();
                self.follow(op, promise);
            }
            "stopRunner" => {
                let promise = self.backend.stop_runner();
                self.follow(op, promise);
            }
            "load" => {
                let model = self.model(text(&step["model"]));
                let promise = self.backend.ensure_loaded(&model, false);
                self.follow(op, promise);
            }
            "unload" => {
                let promise = self.backend.unload_model(text(&step["model"]), text(&step["why"]));
                self.follow(op, promise);
            }
            "chatWith" => {
                let model = self.model(text(&step["model"]));
                self.backend.chat_with(&model);
            }
            "send" => {
                let id = self.conversation_of(&step["conversation"]);
                let attachments: Vec<Attachment> = match step.get("attachments") {
                    Some(given) => serde_json::from_value(given.clone()).expect("attachments"),
                    None => Vec::new(),
                };
                let promise = self.backend.send(&id, text(&step["text"]), attachments);
                self.follow(op, promise);
            }
            "regenerate" => {
                let id = self.conversation_of(&step["conversation"]);
                let message = self.message_of(&id, &step["message"]);
                let promise = self.backend.regenerate(&id, &message);
                self.follow(op, promise);
            }
            "stop" => {
                let id = self.conversation_of(&step["conversation"]);
                self.backend.stop_conversation(&id);
            }
            "showVersion" => {
                let id = self.conversation_of(&step["conversation"]);
                let message = self.message_of(&id, &step["message"]);
                let at = step["version"].as_u64().expect("a version") as usize;
                self.backend.show_version(&id, &message, at);
            }
            "feedback" => {
                let id = self.conversation_of(&step["conversation"]);
                let message = self.message_of(&id, &step["message"]);
                let feedback: Option<Feedback> = serde_json::from_value(step["feedback"].clone()).expect("feedback");
                self.backend.give_feedback(&id, &message, feedback);
            }
            "react" => {
                let id = self.conversation_of(&step["conversation"]);
                let message = self.message_of(&id, &step["message"]);
                self.backend
                    .react(&id, &message, text(&step["emoji"]), text(&step["name"]));
            }
            "settings" => {
                let id = self.conversation_of(&step["conversation"]);
                let patch = &step["patch"];
                self.backend.set_settings(
                    &id,
                    SamplingPatch {
                        temperature: patch["temperature"].as_f64(),
                        top_p: patch["topP"].as_f64(),
                        max_tokens: patch["maxTokens"].as_f64(),
                        system: patch["system"].as_str().map(str::to_owned),
                    },
                );
            }
            "conversation" => {
                let id = self.conversation_of(&step["conversation"]);
                let patch = step["patch"].clone();
                let promise = self.backend.update(
                    |b| &mut b.conversations,
                    &id,
                    move |row| {
                        if let Some(title) = patch["title"].as_str() {
                            row.title = title.to_owned();
                        }
                        if let Some(pinned) = patch["pinned"].as_bool() {
                            row.pinned = pinned;
                        }
                    },
                );
                self.follow(op, promise);
            }
            "deleteConversation" => {
                let id = self.conversation_of(&step["conversation"]);
                self.backend.stop_conversation(&id);
                let promise = self.backend.remove(|b| &mut b.conversations, &id);
                self.follow(op, promise);
            }
            "compare" => {
                let picked = texts(&step["models"]);
                let promise = self.backend.compare(
                    text(&step["prompt"]),
                    &picked,
                    step["temperature"].as_f64().expect("a temperature"),
                    step["maxTokens"].as_f64().expect("max tokens"),
                );
                self.follow(op, promise);
            }
            "rerun" => {
                let id = self.comparison_of(&step["comparison"]);
                let promise = self.backend.rerun_column(&id, index());
                self.follow(op, promise);
            }
            "stopColumn" => {
                let id = self.comparison_of(&step["comparison"]);
                self.backend.stop_column(&id, index());
            }
            "stopComparison" => {
                let id = self.comparison_of(&step["comparison"]);
                self.backend.stop_comparison(&id);
            }
            "removeColumn" => {
                let id = self.comparison_of(&step["comparison"]);
                let index = index();
                let shown = self.backend.comparisons.get(&id).expect("a comparison").clone();
                self.backend.stop_column(&id, index);
                let columns: Vec<_> = shown
                    .columns
                    .into_iter()
                    .enumerate()
                    .filter(|(at, _)| *at != index)
                    .map(|(_, column)| column)
                    .collect();
                let promise = self
                    .backend
                    .update(|b| &mut b.comparisons, &id, move |row| row.columns = columns);
                self.follow(op, promise);
            }
            "deleteComparison" => {
                let id = self.comparison_of(&step["comparison"]);
                self.backend.stop_comparison(&id);
                let promise = self.backend.remove(|b| &mut b.comparisons, &id);
                self.follow(op, promise);
            }
            "deleteModels" => {
                let all = self.backend.models.rows.clone();
                let gone: Vec<Model> = texts(&step["models"]).iter().map(|id| self.model(id)).collect();
                let default_model = self.backend.prefs.rows[0].default_model.clone();
                let promise = self.backend.delete_models(gone, all, &default_model);
                self.follow(op, promise);
            }
            "createView" => {
                let name = text(&step["name"]).to_owned();
                let state = serde_json::from_value(step["state"].clone()).expect("a view's state");
                let promise = self.backend.create(
                    |b| &mut b.views,
                    move |id| LibraryView {
                        id,
                        name,
                        state,
                        locked: None,
                    },
                    None,
                );
                self.follow(op, promise);
            }
            "renameView" => {
                let name = text(&step["name"]).to_owned();
                let promise = self
                    .backend
                    .update(|b| &mut b.views, text(&step["view"]), move |row| row.name = name);
                self.follow(op, promise);
            }
            "removeView" => {
                let promise = self.backend.remove(|b| &mut b.views, text(&step["view"]));
                self.follow(op, promise);
            }
            "look" => {}
            "pages" => self.pages.push(pages_look(&self.backend)),
            other => panic!("no step {other}"),
        }
    }

    fn look(&mut self) -> Json {
        let backend = &mut self.backend;
        let mut out = Map::new();
        out.insert("library".into(), rows_json(&backend.models.rows));
        out.insert("conversations".into(), rows_json(&backend.conversations.rows));
        out.insert("downloads".into(), rows_json(&backend.downloads.rows));
        out.insert("runner".into(), rows_json(&backend.runner.rows));
        out.insert("loaded".into(), rows_json(&backend.loaded.rows));
        out.insert("events".into(), rows_json(&backend.events.rows));
        out.insert("comparisons".into(), rows_json(&backend.comparisons.rows));
        out.insert("prefs".into(), rows_json(&backend.prefs.rows));
        out.insert("views".into(), rows_json(&backend.views.rows));
        let live: Vec<Json> = backend
            .live
            .iter()
            .map(|(key, live)| json!([key, to_json(live)]))
            .collect();
        out.insert("live".into(), json!(live));
        out.insert("settled".into(), json!(std::mem::take(&mut *self.settled.borrow_mut())));
        out.insert("went".into(), json!(std::mem::take(&mut backend.went)));
        out.insert("pulled".into(), rows_json(&std::mem::take(&mut backend.pulled)));
        out.insert("pages".into(), json!(std::mem::take(&mut self.pages)));
        let mut changed = Map::new();
        for (key, value) in out {
            let value = norm(value);
            if self.last.get(&key) != Some(&value) {
                changed.insert(key.clone(), value.clone());
            }
            self.last.insert(key, value);
        }
        Json::Object(changed)
    }
}

#[test]
fn steps_are_the_webs() {
    let start = now();
    let mut player = Player {
        backend: Backend::new(start),
        settled: Rc::default(),
        pages: Vec::new(),
        last: Map::new(),
    };
    player.backend.advance(start + 160);
    let theirs = web()["steps"].as_array().expect("steps");
    same(&player.look(), &theirs[0], "steps[0]");
    for (index, step) in inputs()["steps"].as_array().expect("steps").iter().enumerate() {
        let at = start + step["at"].as_i64().expect("at");
        player.backend.advance(at);
        player.act(step);
        player.backend.advance(at + step["wait"].as_i64().unwrap_or(0));
        let ours = player.look();
        let what = format!("steps[{}] ({} at {})", index + 1, text(&step["op"]), step["at"]);
        same(&ours, &theirs[index + 1], &what);
    }
}

// --- The pages ------------------------------------------------------------------------------------------

fn stats_json(stats: &[models::pages::disk::Stat]) -> Json {
    to_json(&stats)
}

fn pairs(items: &[(String, String)]) -> Json {
    json!(items.iter().map(|(a, b)| [a, b]).collect::<Vec<_>>())
}

fn pages_look(backend: &Backend) -> Json {
    use models::pages::{chat, chat_settings, compare, discover, disk, downloads, library, model_page, runner};
    let now = backend.runtime.now;
    let models = &backend.models.rows;
    let loaded = &backend.loaded.rows;
    let conversations = &backend.conversations.rows;
    let pulls = &backend.downloads.rows;
    let prefs = &backend.prefs.rows[0];
    let runner_row = backend.runner.rows.first();

    let library = {
        let fields: Vec<Json> = library::fields_said(models)
            .iter()
            .map(|field| {
                json!({"id": field.id, "label": field.label, "type": field.kind,
                    "options": field.options.iter().map(|(a, b)| [a, b]).collect::<Vec<_>>(), "unit": field.unit})
            })
            .collect();
        let shown: Vec<Json> = inputs()["library"]
            .as_array()
            .expect("library")
            .iter()
            .map(|ask| {
                let filters: Vec<models::data::backend::ViewFilter> =
                    serde_json::from_value(ask["filters"].clone()).expect("filters");
                let filters: Vec<_> = filters.iter().filter_map(|one| one.filter()).collect();
                let rows = library::shown(models, &filters, text(&ask["query"]));
                json!({"ids": rows.iter().map(|one| one.id.clone()).collect::<Vec<_>>(), "total": library::size_total(&rows)})
            })
            .collect();
        json!({
            "count": library::count(models),
            "fields": fields,
            "shown": shown,
            "cells": models.iter().map(|one| library::cells(one, loaded)).collect::<Vec<_>>(),
            "actions": models.iter().map(|one| library::model_actions(one, loaded)).collect::<Vec<_>>(),
        })
    };

    let pages: Vec<Json> = texts(&inputs()["modelPages"])
        .iter()
        .map(|id| match models.iter().find(|one| one.id == *id) {
            None => Json::Null,
            Some(model) => {
                let page = model_page::model_page(model, loaded, conversations, &prefs.root);
                json!({
                    "header": {"title": page.title, "code": page.code, "count": null, "description": page.description},
                    "details": pairs(&page.details),
                    "inUse": pairs(&page.in_use),
                    "filesMeta": page.files_meta,
                    "files": page.files.iter().map(|(a, b, c)| [a, b, c]).collect::<Vec<_>>(),
                    "load": page.load,
                })
            }
        })
        .collect();

    let disk = {
        let page = disk::disk_page(&prefs.root, models, pulls);
        json!({
            "description": page.description,
            "stats": stats_json(&page.stats),
            "subtitle": page.subtitle,
            "whole": to_json(&page.whole),
            "rows": to_json(&page.rows),
            "copies": to_json(&page.copies),
            "copiesMeta": page.copies_meta,
            "path": page.path,
        })
    };

    let discover = {
        let (count, description, downloads) = discover::header(pulls);
        let sheets: Vec<Json> = texts(&inputs()["sheets"])
            .iter()
            .map(|id| {
                let entry = entry_of(id).expect("an entry");
                json!({"about": discover::about(entry), "memory": discover::variant_memory(entry, prefs.memory)})
            })
            .collect();
        let buttons: Vec<Json> = catalog()
            .iter()
            .flat_map(|entry| entry.variants.iter().map(move |variant| (entry, variant)))
            .map(
                |(entry, variant)| match models::pages::pull::pull_button(entry, variant, models, pulls) {
                    models::pages::pull::PullButton::Kept { detail, said, .. } => {
                        json!({"state": "kept", "keptDetail": detail, "error": null, "value": null, "words": {"kept": said}})
                    }
                    models::pages::pull::PullButton::Idle { said, error, .. } => {
                        json!({"state": "idle", "keptDetail": null, "error": error, "value": null, "words": {"download": said}})
                    }
                    models::pages::pull::PullButton::Running {
                        value, paused, queued, ..
                    } => json!({
                        "state": if paused { "paused" } else { "running" },
                        "keptDetail": null, "error": null, "value": value,
                        "words": if queued { json!({"share": "Queued"}) } else { Json::Null },
                    }),
                },
            )
            .collect();
        json!({
            "count": count,
            "description": description,
            "downloads": downloads,
            "shown": catalog().iter().map(|entry| discover::row(entry, models)).collect::<Vec<_>>(),
            "sheets": sheets,
            "buttons": buttons,
        })
    };

    let downloads = {
        let page = downloads::downloads_page(pulls, prefs);
        json!({
            "count": page.count,
            "description": page.description,
            "canClear": page.can_clear,
            "stats": stats_json(&page.stats),
            "rows": to_json(&page.rows),
        })
    };

    let runners: Vec<Json> = inputs()["logViews"]
        .as_array()
        .expect("log views")
        .iter()
        .map(|view| {
            let source = match text(&view[0]) {
                "all" => None,
                other => serde_json::from_value(json!(other)).ok(),
            };
            let page = runner::runner_page(
                runner_row,
                loaded,
                models,
                prefs,
                &backend.events.rows,
                pulls,
                now,
                source,
                text(&view[1]),
            );
            json!({
                "code": page.code,
                "button": [page.button.0, page.button.1],
                "stats": stats_json(&page.stats),
                "memoryMeta": page.memory_meta,
                "memoryLabel": page.memory_label,
                "memoryValue": page.memory_value,
                "loaded": page.loaded,
                "empty": page.empty,
                "checks": to_json(&page.checks),
                "lines": to_json(&page.lines),
            })
        })
        .collect();

    let settings: Vec<Json> = conversations
        .iter()
        .map(|conversation| {
            let model = models.iter().find(|one| one.id == conversation.model);
            let panel = chat_settings::panel(model, models, loaded);
            json!({
                "options": panel.options.iter().map(|(format, items)| json!([format,
                    items.iter().map(|(id, name, quant)| [id.clone(), format!("{name}{quant}")]).collect::<Vec<_>>()])).collect::<Vec<_>>(),
                "facts": panel.facts.map(|facts| facts.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>()),
                "maxHint": panel.max_hint,
                "back": panel.back,
                "presets": chat_settings::PRESETS.map(chat_settings::preset_label),
            })
        })
        .collect();

    let draft = compare::Draft::default();
    let mut ids: Vec<Option<&str>> = vec![None, Some("new")];
    ids.extend(backend.comparisons.rows.iter().map(|one| Some(one.id.as_str())));
    let comparing: Vec<Json> = ids
        .into_iter()
        .map(|id| {
            let page = compare::compare_page(&backend.comparisons.rows, models, &backend.live, &draft, id, now);
            json!({
                "count": page.count,
                "earlier": page.earlier.iter().map(|(a, b)| [a, b]).collect::<Vec<_>>(),
                "keys": page.keys.iter().map(|(a, b)| [a, b]).collect::<Vec<_>>(),
                "canAdd": page.can_add,
                "canRemove": page.can_remove,
                "placeholder": page.placeholder,
                "disabled": page.disabled,
                "running": page.running,
                "asked": page.asked,
                "columns": page.columns,
                "answers": to_json(&page.answers),
            })
        })
        .collect();

    let chats: Vec<Json> = conversations
        .iter()
        .map(|conversation| {
            let replies: Vec<Json> = conversation
                .messages
                .iter()
                .filter_map(|message| match message {
                    models::data::model::Message::Assistant(reply) => Some(reply),
                    _ => None,
                })
                .map(|message| {
                    let said = chat::reply(conversation, message, models, &backend.live, now);
                    let usage = said.usage.as_ref().map(|usage| {
                        let mut usage = to_json(usage);
                        usage["live"] = json!(said.streaming);
                        usage
                    });
                    json!({
                        "name": said.name, "text": said.text, "reasoning": said.reasoning, "state": said.state,
                        "usage": usage, "problem": said.problem, "thinking": said.thinking,
                        "versions": said.versions.map(|(at, of)| [at, of]),
                    })
                })
                .collect();
            let model = models.iter().find(|one| one.id == conversation.model);
            json!({"replies": replies, "placeholder": chat::placeholder(model)})
        })
        .collect();

    let apps: Vec<Json> = inputs()["routes"]
        .as_array()
        .expect("routes")
        .iter()
        .map(|route| {
            let route = texts(route);
            let route: Vec<&str> = route.iter().map(String::as_str).collect();
            let page = route.first().copied().unwrap_or("library");
            let server = models::app::server_menu(runner_row, loaded, now);
            let status = models::app::status_line(runner_row, pulls, loaded, models, now);
            json!({
                "rail": models::app::rail(page, pulls, loaded).iter().map(|(id, title, entries)| json!([id, title, to_json(entries)])).collect::<Vec<_>>(),
                "trail": models::app::trail(&route, models, conversations).iter().map(|(label, href, code)| json!([label, href, code])).collect::<Vec<_>>(),
                "server": {"state": server.state, "facts": server.facts, "checking": server.checking},
                "status": {"live": status.live, "warn": status.warn, "note": status.note,
                    "items": [["loaded", status.loaded], ["on disk", status.on_disk]]},
                "actions": models::app::actions(runner_row, pulls, models, prefs).iter().map(|(id, label, hint)| json!([id, label, hint])).collect::<Vec<_>>(),
                "conversations": models::app::palette_conversations(conversations, models).iter().map(|(id, label, hint)| json!([id, label, hint])).collect::<Vec<_>>(),
            })
        })
        .collect();

    let history: Json = match conversations.first() {
        None => Json::Null,
        Some(_) => json!(
            chat::history(conversations, now)
                .iter()
                .map(|row| json!([row.group, row.group.label(), row.id, row.time, row.actions]))
                .collect::<Vec<_>>()
        ),
    };
    let deletes: Vec<Json> = inputs()["deletes"]
        .as_array()
        .expect("deletes")
        .iter()
        .map(|ids| {
            let doomed: Vec<Model> = texts(ids)
                .iter()
                .filter_map(|id| models.iter().find(|one| one.id == *id).cloned())
                .collect();
            let dialog = models::actions::delete_dialog(&doomed, conversations, &prefs.root);
            let mut items = vec![
                json!(["Space freed", null, dialog.freed]),
                json!(["Files removed", dialog.files, null]),
            ];
            if let Some((label, count)) = dialog.talks {
                items.push(json!([
                    label,
                    count,
                    "They keep their messages and ask for another model."
                ]));
            }
            json!({"question": dialog.question, "detail": dialog.detail, "items": items})
        })
        .collect();
    let settings_page = {
        use models::pages::settings;
        let mut sections = settings::sections(prefs);
        sections.push(settings::demo());
        let resets: Vec<Json> = ["library", "models", "downloads", "runner", "demo", "nothing"]
            .iter()
            .filter_map(|section| settings::reset(section))
            .map(|patch| to_json(&patch))
            .collect();
        json!({"sections": to_json(&sections), "resets": resets})
    };

    json!({
        "history": history,
        "deletes": deletes,
        "settingsPage": settings_page,
        "library": library,
        "models": pages,
        "disk": disk,
        "discover": discover,
        "downloads": downloads,
        "runner": runners,
        "settings": settings,
        "compare": comparing,
        "chat": chats,
        "app": apps,
    })
}
