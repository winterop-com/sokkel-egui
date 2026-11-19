//! The data layer held against the web's own: `web/dump.ts` runs the web example's modules
//! (`data/metadata.ts`, `values.ts`, `versions.ts`, `expression.ts`, `check.ts`, `program.ts`,
//! `backend.ts`, `themes.ts`, `ui.tsx`, and the app and its pages with the backend behind them)
//! under bun on the inputs in `web/inputs.json`, the zone UTC, the clock and the timers held and
//! `Math.random` a seeded run, and writes what they give as JSON. Each test builds the same JSON
//! here and asks for it to be equal, every number to the bit.
//!
//! The web's answer is kept in `web/data.json`. Where bun and `.parity/sokkel-ui` are at hand the
//! dump is made again and must match it, so a change on either side shows (`DHIS2_WEB_KEEP=1`
//! keeps the new one); elsewhere (GitHub's CI) the kept answer is read.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use dhis2::app::{self, Page};
use dhis2::backend::{
    self, Backend, INDICATOR_TYPES, OPERATORS, SERVER_ADDRESS, SERVER_NAME, STRATEGIES, aggregate, indicator_seed,
    indicator_value, rules_seed, run_rule, saving_note, unit_name,
};
use dhis2::check::{CheckContext, Checked, check, item_name};
use dhis2::expression::{
    self, ItemKind, Language, Value, describe, parse, tokenize, try_evaluate, without_block_comments,
    without_block_comments_by,
};
use dhis2::metadata::{
    self, REPORTING_METRICS, TODAY, category_combos, children_of, combos, constant, constants, data_element,
    data_elements, data_set, days_in, de, hashed, level_name, option_combo, org_unit, org_unit_tree, org_units,
    path_of, period_label, periods, root, shift_period, sites_under, uid,
};
use dhis2::pages::{
    data_elements as elements_page, functions, indicator, indicators, org_units as units_page, program_rules,
    validation_rule, validation_rules,
};
use dhis2::program::{
    ACTION_TYPES, EVALUATED_VARIABLES, FormValues, ProgramRule, RuleRun, fields, form_item, program, rule_seed,
    run_rules, variables,
};
use dhis2::themes::CARRIED;
use dhis2::ui;
use dhis2::values::{data_value, reports};
use dhis2::versions::{ADDED, LATEST, VERSIONS, names_of};
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
            .join(".parity/sokkel-ui/examples/dhis2/web/node_modules")
            .exists()
            .then(|| {
                Command::new("bun")
                    .arg("examples/dhis2/tests/web/dump.ts")
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
                if std::env::var_os("DHIS2_WEB_KEEP").is_some() {
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
                            panic!("web/data.json is not what the web gives now; run with DHIS2_WEB_KEEP=1 to keep it")
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

// --- Placeholders, as the dump fills them ---------------------------------------------------------------

/// A text with each `$name` the uid it stands for.
fn fill(raw: &str) -> String {
    let de = de();
    let places: [(&str, &str); 15] = [
        ("visits", &de.visits.id),
        ("referrals", &de.referrals.id),
        ("vaccinated", &de.vaccinated.id),
        ("due", &de.due.id),
        ("taken", &de.samples_taken.id),
        ("failed", &de.samples_failed.id),
        ("received", &de.stock_received.id),
        ("used", &de.stock_used.id),
        ("onHand", &de.stock_on_hand.id),
        ("under5", &metadata::age_combo().combos[0].id),
        ("over5", &metadata::age_combo().combos[1].id),
        ("plain", &metadata::default_combo().combos[0].id),
        ("target", &constants()[0].id),
        ("samples", &constants()[1].id),
        ("set", &data_set().id),
    ];
    let mut out = String::new();
    let mut rest = raw;
    while let Some(at) = rest.find('$') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        let name = &after[..end];
        match places.iter().find(|(one, _)| *one == name) {
            Some((_, id)) => out.push_str(id),
            None => {
                out.push('$');
                out.push_str(name);
            }
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

fn unit_of(code: &str) -> String {
    org_units()
        .iter()
        .find(|one| one.code == code)
        .map_or_else(|| code.to_owned(), |one| one.id.clone())
}

fn element_by_key(key: &str) -> &'static metadata::DataElement {
    let de = de();
    match key {
        "visits" => &de.visits,
        "referrals" => &de.referrals,
        "vaccinated" => &de.vaccinated,
        "due" => &de.due,
        "samplesTaken" => &de.samples_taken,
        "samplesFailed" => &de.samples_failed,
        "stockReceived" => &de.stock_received,
        "stockUsed" => &de.stock_used,
        _ => &de.stock_on_hand,
    }
}

/// A reference to something by its kind and code, as the steps write one.
fn id_of(reference: &str) -> String {
    let (kind, code) = reference.split_once(':').unwrap_or((reference, ""));
    match kind {
        "ind" => uid(&format!("ind:{code}")),
        "vr" => uid(&format!("vr:{code}")),
        "prr" => uid(&format!("prr:{code}")),
        "ou" => unit_of(code),
        "de" => element_by_key(code).id.clone(),
        _ => reference.to_owned(),
    }
}

fn units() -> Vec<String> {
    org_units().iter().map(|one| one.id.clone()).collect()
}

fn periods_all() -> Vec<String> {
    let mut out = periods().to_vec();
    out.extend(texts(&inputs()["extraPeriods"]));
    out
}

fn num(value: f64) -> Json {
    norm(json!(value))
}

fn opt(value: Option<f64>) -> Json {
    value.map_or(Json::Null, num)
}

// --- Metadata, values and versions -----------------------------------------------------------------------

#[test]
fn metadata_is_the_webs() {
    let web = &core()["meta"];
    let ours = json!({
        "hashed": inputs()["hashed"].as_array().unwrap().iter().map(|one| hashed(text(one))).collect::<Vec<_>>(),
        "uids": inputs()["uids"].as_array().unwrap().iter().map(|one| uid(text(one))).collect::<Vec<_>>(),
        "orgUnits": to_json(&org_units()),
        "root": root(),
        "tree": to_json(&org_unit_tree()),
        "levels": { "1": level_name(1), "2": level_name(2), "3": level_name(3), "4": level_name(4) },
        "categoryCombos": to_json(&category_combos()),
        "combos": to_json(&combos()),
        "dataElements": to_json(&data_elements()),
        "dataSet": to_json(data_set()),
        "metrics": REPORTING_METRICS,
        "constants": to_json(&constants()),
        "periods": periods(),
        "today": TODAY,
        "units": units().iter().map(|id| json!({
            "children": children_of(id).iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
            "path": path_of(id).iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
            "sites": sites_under(id).iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "unknownUnit": {
            "found": org_unit("nothing").map(to_json),
            "path": path_of("nothing").iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
            "sites": sites_under("nothing").iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
        },
        "lookups": {
            "elements": data_elements().iter().map(|one| one.id.clone()).chain(["nothing".to_owned()]).map(|id| data_element(&id).map(|one| one.code.clone())).collect::<Vec<_>>(),
            "combos": combos().iter().map(|one| one.id.clone()).chain(["nothing".to_owned()]).map(|id| option_combo(&id).map(|one| one.name.clone())).collect::<Vec<_>>(),
            "constants": constants().iter().map(|one| one.id.clone()).chain(["nothing".to_owned()]).map(|id| constant(&id).map(|one| one.code.clone())).collect::<Vec<_>>(),
        },
        "periodLabels": periods().iter().map(|one| period_label(one)).collect::<Vec<_>>(),
        "shifted": inputs()["shifts"].as_array().unwrap().iter().map(|one| shift_period(text(&one[0]), one[1].as_f64().unwrap())).collect::<Vec<_>>(),
        "days": inputs()["days"].as_array().unwrap().iter().map(|one| num(days_in(text(one)))).collect::<Vec<_>>(),
    });
    same(&ours, web, "meta");
}

#[test]
fn values_are_the_webs() {
    let web = &core()["values"];
    let periods = periods_all();
    let units = units();
    let mut values = Vec::new();
    for element in data_elements() {
        let mut asked: Vec<Option<String>> = vec![None];
        asked.extend(element.category_combo.combos.iter().map(|one| Some(one.id.clone())));
        asked.push(Some(if element.category_combo.name == "default" {
            metadata::age_combo().combos[0].id.clone()
        } else {
            metadata::default_combo().combos[0].id.clone()
        }));
        for combo in asked {
            values.push(
                units
                    .iter()
                    .map(|unit| {
                        periods
                            .iter()
                            .map(|period| opt(data_value(&element.id, combo.as_deref(), unit, period)))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>(),
            );
        }
    }
    let ours = json!({
        "values": values,
        "unknown": opt(data_value("nothing", None, root(), &metadata::periods()[0])),
        "reports": units.iter().map(|unit| periods.iter().map(|period| {
            let (actual, expected) = reports(unit, period);
            json!({ "actual": actual, "expected": expected })
        }).collect::<Vec<_>>()).collect::<Vec<_>>(),
    });
    same(&ours, web, "values");
}

#[test]
fn versions_are_the_webs() {
    let web = &core()["versions"];
    let added: Map<String, Json> = ADDED
        .iter()
        .map(|(name, version)| ((*name).to_owned(), json!(version)))
        .collect();
    let ours = json!({
        "versions": VERSIONS,
        "latest": LATEST,
        "added": added,
        "names": VERSIONS.iter().copied().chain(["2.40"]).map(|one| to_json(&names_of(one))).collect::<Vec<_>>(),
    });
    same(&ours, web, "versions");
}

#[test]
fn themes_are_the_webs() {
    same(&json!(CARRIED), &core()["themes"], "themes");
}

// --- Expressions ---------------------------------------------------------------------------------------

fn held_ids() -> Vec<String> {
    indicator_seed().into_iter().map(|one| one.id).collect()
}

fn context_of(language: Language, version: &str) -> CheckContext {
    match language {
        Language::Rule => program_rules::context(version),
        Language::Expression => CheckContext {
            version: version.to_owned(),
            indicators: held_ids(),
            ..CheckContext::default()
        },
    }
}

fn error_json(error: &expression::ExpressionError) -> Json {
    json!({ "error": { "message": error.message, "span": { "from": error.span.from, "to": error.span.to } } })
}

fn evaluated_json(result: Result<expression::Evaluated, expression::Evaluated>) -> Json {
    match result {
        Ok(one) => json!({ "value": one.value.to_json(), "items": one.items, "missing": one.missing }),
        Err(_) => json!({ "throws": true }),
    }
}

/// A form's values as the inputs write them.
fn form_of(value: &Json) -> FormValues {
    value
        .as_object()
        .expect("a form")
        .iter()
        .map(|(key, one)| {
            let value = match one {
                Json::Null => Value::Null,
                Json::Bool(flag) => Value::Bool(*flag),
                Json::Number(number) => Value::Number(number.as_f64().unwrap()),
                Json::String(text) => Value::Text(text.clone()),
                _ => panic!("a form value"),
            };
            (key.clone(), value)
        })
        .collect()
}

fn forms() -> Vec<(FormValues, String)> {
    inputs()["forms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| (form_of(&one[0]), text(&one[1]).to_owned()))
        .collect()
}

#[test]
fn expressions_are_the_webs() {
    let web = core()["expressions"].as_array().expect("expressions");
    let places: Vec<(String, String)> = inputs()["places"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| (unit_of(text(&one[0])), text(&one[1]).to_owned()))
        .collect();
    let forms = forms();
    for (at, raw) in inputs()["expressions"].as_array().unwrap().iter().enumerate() {
        let source = fill(text(raw));
        let parsed = parse(&source);
        let node = parsed.as_ref().ok();
        let checks: Vec<Json> = [Language::Expression, Language::Rule]
            .iter()
            .flat_map(|language| {
                VERSIONS
                    .iter()
                    .map(|version| to_json(&check(&source, *language, &context_of(*language, version)).findings))
                    .collect::<Vec<_>>()
            })
            .collect();
        let bare = CheckContext {
            version: LATEST.to_owned(),
            ..CheckContext::default()
        };
        let ours = json!({
            "tokens": match tokenize(&source) {
                Ok(tokens) => json!({ "ok": tokens.iter().map(expression::Token::to_json).collect::<Vec<_>>() }),
                Err(error) => error_json(&error),
            },
            "parsed": match &parsed {
                Ok(node) => json!({ "ok": node.to_json() }),
                Err(error) => error_json(error),
            },
            "checks": checks,
            "bare": to_json(&check(&source, Language::Expression, &bare).findings),
            "aggregate": node.map(|node| places.iter().map(|(unit, period)| {
                let scope = |kind: ItemKind, body: &str, offset: f64| backend::value_of(kind, body, unit, period, offset);
                match try_evaluate(node, &scope) {
                    Ok(_) => to_json(&aggregate(node, unit, period)),
                    Err(_) => json!({ "throws": true }),
                }
            }).collect::<Vec<_>>()),
            "forms": node.map(|node| forms.iter().map(|(form, date)| {
                let scope = |kind: ItemKind, body: &str, _offset: f64| form_item(form, date, kind, body);
                evaluated_json(try_evaluate(node, &scope))
            }).collect::<Vec<_>>()),
            "described": describe(&source, |kind, body| item_name(kind, dhis2::js::trim(body))),
            "withoutComments": without_block_comments(&source),
            "withoutCommentsBy": without_block_comments_by(&source, |comment| format!("<{}>", comment.encode_utf16().count())),
        });
        same(&ours, &web[at], &format!("expressions[{at}] {source:?}"));
    }
}

#[test]
fn patterns_and_item_names_are_the_webs() {
    let patterns: Vec<Json> = inputs()["patterns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| {
            let form: FormValues = vec![
                ("value".to_owned(), Value::Text(text(&one[0]).to_owned())),
                ("pattern".to_owned(), Value::Text(text(&one[1]).to_owned())),
            ];
            let node = parse("d2:validatePattern(#{value}, #{pattern})").expect("reads");
            let scope = |kind: ItemKind, body: &str, _offset: f64| form_item(&form, "", kind, body);
            expression::evaluate(&node, &scope).value.to_json()
        })
        .collect();
    same(&json!(patterns), &core()["patterns"], "patterns");
    let kinds = |key: &str| match key {
        "#" => ItemKind::Hash,
        "C" => ItemKind::C,
        "R" => ItemKind::R,
        "N" => ItemKind::N,
        "V" => ItemKind::V,
        _ => ItemKind::Days,
    };
    let names: Vec<Option<String>> = inputs()["itemNames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| item_name(kinds(text(&one[0])), &fill(text(&one[1]))))
        .collect();
    same(&json!(names), &core()["itemNames"], "itemNames");
}

// --- The backend's analytics ------------------------------------------------------------------------

#[test]
fn analytics_are_the_webs() {
    let web = &core()["analytics"];
    let periods = periods_all();
    let units = units();
    let types: Map<String, Json> = INDICATOR_TYPES
        .iter()
        .map(|one| {
            (
                one.key().to_owned(),
                json!({ "label": one.label(), "factor": num(one.factor()), "unit": one.unit() }),
            )
        })
        .collect();
    let operators: Map<String, Json> = OPERATORS
        .iter()
        .map(|one| {
            (
                one.key().to_owned(),
                json!({ "label": one.label(), "sign": one.sign() }),
            )
        })
        .collect();
    let strategies: Map<String, Json> = STRATEGIES
        .iter()
        .map(|one| {
            (
                one.key().to_owned(),
                json!({ "label": one.label(), "description": one.description() }),
            )
        })
        .collect();
    let sites: Vec<String> = sites_under(root()).iter().map(|one| one.id.clone()).collect();
    let rules = rules_seed();
    let variants: Vec<Json> = inputs()["variants"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|one| {
            let rule = rules
                .iter()
                .find(|rule| rule.code == text(&one[0]))
                .expect("a rule")
                .clone();
            let units: Vec<String> = texts(&one[1]).iter().map(|code| unit_of(code)).collect();
            let left = parse(&rule.left).expect("reads");
            let right = parse(&rule.right).expect("reads");
            let periods = periods.clone();
            OPERATORS
                .iter()
                .flat_map(|operator| {
                    STRATEGIES.iter().flat_map(|left_strategy| {
                        STRATEGIES
                            .iter()
                            .map(|right_strategy| {
                                let run = run_rule(
                                    &rule.id,
                                    *operator,
                                    *left_strategy,
                                    *right_strategy,
                                    &left,
                                    &right,
                                    &units,
                                    &periods,
                                );
                                json!({
                                    "tried": run.tried, "skipped": run.skipped, "count": run.violations.len(),
                                    "head": to_json(&run.violations.iter().take(3).collect::<Vec<_>>()),
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let ours = json!({
        "server": { "name": SERVER_NAME, "address": SERVER_ADDRESS },
        "me": backend::ME,
        "types": types,
        "operators": operators,
        "strategies": strategies,
        "indicators": to_json(&indicator_seed()),
        "rules": to_json(&rules),
        "values": indicator_seed().iter().map(|one| {
            let top = parse(&one.numerator).expect("reads");
            let bottom = parse(&one.denominator).expect("reads");
            units.iter().map(|unit| periods.iter().map(|period| to_json(&indicator_value(&top, &bottom, one.kind, unit, period))).collect::<Vec<_>>()).collect::<Vec<_>>()
        }).collect::<Vec<_>>(),
        "runs": rules.iter().map(|one| to_json(&run_rule(
            &one.id, one.operator, one.left_strategy, one.right_strategy,
            &parse(&one.left).expect("reads"), &parse(&one.right).expect("reads"), &sites, metadata::periods(),
        ))).collect::<Vec<_>>(),
        "variants": variants,
        "unitNames": units.iter().map(String::as_str).chain(["nothing"]).map(unit_name).collect::<Vec<_>>(),
    });
    same(&ours, web, "analytics");
}

// --- The program ---------------------------------------------------------------------------------------

fn run_json(run: &RuleRun) -> Json {
    let assigned: Map<String, Json> = run
        .assigned
        .iter()
        .map(|(key, value)| (key.clone(), value.to_json()))
        .collect();
    json!({
        "hidden": run.hidden_order,
        "assigned": assigned,
        "effects": to_json(&run.effects),
        "broken": to_json(&run.broken),
    })
}

#[test]
fn program_is_the_webs() {
    let web = &core()["program"];
    let actions: Map<String, Json> = ACTION_TYPES
        .iter()
        .map(|one| {
            let shape = one.shape();
            (
                one.key().to_owned(),
                json!({ "label": shape.label, "field": shape.field, "data": shape.data, "content": shape.content }),
            )
        })
        .collect();
    let forms = forms();
    let seed = rule_seed();
    let edited: Vec<Json> = inputs()["ruleEdits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| {
            let at = one[0].as_u64().unwrap() as usize;
            let mut rules = seed.clone();
            rules[at].condition = fill(text(&one[1]));
            if let Some(data) = one[2].as_str() {
                for action in &mut rules[at].actions {
                    action.data = fill(data);
                }
            }
            json!(
                forms
                    .iter()
                    .map(|(form, date)| run_json(&run_rules(&rules, form, date, LATEST)))
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    let ours = json!({
        "program": { "id": program().id, "name": program().name, "stage": program().stage },
        "fields": to_json(&fields()),
        "variables": variables(),
        "evaluated": EVALUATED_VARIABLES,
        "actions": actions,
        "seed": to_json(&seed),
        "runs": forms.iter().flat_map(|(form, date)| VERSIONS.iter().map(|version| run_json(&run_rules(&seed, form, date, version))).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "edited": edited,
    });
    same(&ours, web, "program");
}

// --- The pages' shared parts -----------------------------------------------------------------------------

#[test]
fn ui_is_the_webs() {
    let web = &core()["ui"];
    let checked = |source: &str, language: Language| check(source, language, &context_of(language, LATEST));
    let sources: Vec<String> = inputs()["expressions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| fill(text(one)))
        .collect();
    let statuses: Map<String, Json> = ui::STATUSES
        .iter()
        .map(|(state, color)| ((*state).to_owned(), json!({ "color": color })))
        .collect();
    let ours = json!({
        "figures": inputs()["figures"].as_array().unwrap().iter().map(|one| ui::figure(one[0].as_f64(), one[1].as_str().unwrap_or(""))).collect::<Vec<_>>(),
        "counts": inputs()["counts"].as_array().unwrap().iter().map(|one| ui::count(one[0].as_f64().unwrap(), text(&one[1]), one[2].as_str())).collect::<Vec<_>>(),
        "issues": sources.iter().map(|source| to_json(&ui::issues_of(source, &checked(source, Language::Expression)))).collect::<Vec<_>>(),
        "verdicts": sources.iter().map(|source| ui::verdict(&[&checked(source, Language::Expression), &checked(source, Language::Rule)]).key()).collect::<Vec<_>>(),
        "describedItems": sources.iter().map(|source| ui::describe_items(source)).collect::<Vec<_>>(),
        "statuses": statuses,
    });
    same(&ours, web, "ui");
}

#[test]
fn editor_words_are_the_webs() {
    let web = &core()["words"];
    let befores = texts(&inputs()["completions"]);
    let hover = texts(&inputs()["hover"]);
    let ours: Vec<Json> = [Language::Expression, Language::Rule]
        .iter()
        .flat_map(|language| {
            let variables = if *language == Language::Rule { variables() } else { Vec::new() };
            VERSIONS
                .iter()
                .map(|version| {
                    json!({
                        "completions": befores.iter().map(|before| to_json(&ui::completions(*language, version, &variables, &fill(before)))).collect::<Vec<_>>(),
                        "hover": hover.iter().map(|word| ui::hover_word(&fill(word))).collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();
    same(&json!(ours), web, "words");
}

// --- The app, step by step --------------------------------------------------------------------------------

/// The state of the page shown, as the web's mounted page keeps it.
enum PageState {
    Indicators {
        query: String,
        types: Vec<String>,
        states: Vec<String>,
    },
    Indicator {
        id: String,
        editor: Option<indicator::Editor>,
    },
    ValidationRules {
        query: String,
    },
    ValidationRule {
        id: String,
        editor: Option<validation_rule::Editor>,
    },
    ProgramRules {
        selected: Option<String>,
        state: program_rules::ProgramRules,
        /// The rule the editor was drawn for, whose own state it keeps.
        editor_of: Option<String>,
    },
    DataElements {
        selected: Option<String>,
        query: String,
    },
    OrgUnits {
        selected: Option<String>,
        state: units_page::OrgUnits,
    },
    Functions {
        query: String,
        kinds: Vec<String>,
        known: Vec<String>,
    },
    NotFound,
}

fn page_state(page: &Page) -> PageState {
    match page {
        Page::Indicators => PageState::Indicators {
            query: String::new(),
            types: Vec::new(),
            states: Vec::new(),
        },
        Page::Indicator(id) => PageState::Indicator {
            id: id.clone(),
            editor: None,
        },
        Page::ValidationRules => PageState::ValidationRules { query: String::new() },
        Page::ValidationRule(id) => PageState::ValidationRule {
            id: id.clone(),
            editor: None,
        },
        Page::ProgramRules(selected) => PageState::ProgramRules {
            selected: selected.clone(),
            state: program_rules::ProgramRules::new(),
            editor_of: None,
        },
        Page::DataElements(selected) => PageState::DataElements {
            selected: selected.clone(),
            query: String::new(),
        },
        Page::OrgUnits(selected) => PageState::OrgUnits {
            selected: selected.clone(),
            state: units_page::OrgUnits::default(),
        },
        Page::Functions => PageState::Functions {
            query: String::new(),
            kinds: Vec::new(),
            known: Vec::new(),
        },
        Page::NotFound => PageState::NotFound,
    }
}

/// The page's name and key, as the web tells a page from the next.
fn key_of(state: &PageState) -> String {
    match state {
        PageState::Indicators { .. } => "Indicators".to_owned(),
        PageState::Indicator { id, .. } => format!("IndicatorPage:{id}"),
        PageState::ValidationRules { .. } => "ValidationRules".to_owned(),
        PageState::ValidationRule { id, .. } => format!("ValidationRulePage:{id}"),
        PageState::ProgramRules { .. } => "ProgramRules".to_owned(),
        PageState::DataElements { .. } => "DataElements".to_owned(),
        PageState::OrgUnits { .. } => "OrgUnits".to_owned(),
        PageState::Functions { .. } => "Functions".to_owned(),
        PageState::NotFound => "NotFound".to_owned(),
    }
}

fn page_key(page: &Page) -> String {
    match page {
        Page::Indicator(id) => format!("IndicatorPage:{id}"),
        Page::ValidationRule(id) => format!("ValidationRulePage:{id}"),
        other => other.name().to_owned(),
    }
}

/// An expression field as the dump reads it: what it lists, says under itself and underlines.
fn field_look(id: &str, label: &str, value: &str, language: Language, checked: &Checked) -> Json {
    json!({
        "id": id,
        "label": label,
        "value": value,
        "language": if language == Language::Expression { "dhis2-expression" } else { "dhis2-rule" },
        "issues": to_json(&ui::issues_of(value, checked)),
        "help": ui::field_help(language, value),
        "marks": to_json(&ui::marks_of(checked)),
        "invalid": checked.has_error(),
    })
}

fn options(pairs: impl IntoIterator<Item = (String, String)>) -> Json {
    json!(
        pairs
            .into_iter()
            .map(|(value, label)| json!({ "value": value, "label": label }))
            .collect::<Vec<_>>()
    )
}

fn ids_of(rows: &[backend::Indicator]) -> Vec<String> {
    rows.iter().map(|one| one.id.clone()).collect()
}

fn chosen(value: &Json) -> Vec<String> {
    texts(value)
}

struct Harness {
    backend: Backend,
    hash: String,
    version: &'static str,
    page: PageState,
    /// What the status line says, as the web's pages last set it.
    foot: Option<String>,
    /// How many of the places gone to since the last look were followed.
    followed: usize,
}

impl Harness {
    fn new() -> Self {
        let now = inputs()["now"].as_i64().expect("now");
        Harness {
            backend: Backend::new(now),
            hash: "#/".to_owned(),
            version: LATEST,
            page: PageState::Indicators {
                query: String::new(),
                types: Vec::new(),
                states: Vec::new(),
            },
            foot: None,
            followed: 0,
        }
    }

    fn route(&self) -> Vec<String> {
        app::route_of(&self.hash)
    }

    /// Goes where the hash now says, a page of another kind or key mounted afresh.
    fn follow(&mut self) {
        let page = app::page_of(&self.route());
        if page_key(&page) == key_of(&self.page) {
            match (&mut self.page, &page) {
                (PageState::ProgramRules { selected, .. }, Page::ProgramRules(next))
                | (PageState::DataElements { selected, .. }, Page::DataElements(next))
                | (PageState::OrgUnits { selected, .. }, Page::OrgUnits(next)) => *selected = next.clone(),
                _ => {}
            }
            return;
        }
        self.page = page_state(&page);
        self.foot = None;
    }

    /// Follows the places gone to since the last look: the hash is the last of them.
    fn went(&mut self) {
        let count = self.backend.went.len();
        if count > self.followed {
            self.followed = count;
            self.hash = format!("#{}", self.backend.went[count - 1]);
            self.follow();
        }
    }

    /// The editors drawn once their row is heard, and handed it as it now is.
    fn sync(&mut self) {
        match &mut self.page {
            PageState::Indicator { id, editor } => {
                match self.backend.indicators.seen().iter().find(|one| one.id == *id) {
                    None => *editor = None,
                    Some(row) => match editor {
                        Some(editor) => editor.saved = row.clone(),
                        None => *editor = Some(indicator::Editor::new(row)),
                    },
                }
            }
            PageState::ValidationRule { id, editor } => {
                match self.backend.validation_rules.seen().iter().find(|one| one.id == *id) {
                    None => *editor = None,
                    Some(row) => match editor {
                        Some(editor) => editor.saved = row.clone(),
                        None => *editor = Some(validation_rule::Editor::new(row)),
                    },
                }
            }
            PageState::ProgramRules {
                selected,
                state,
                editor_of,
            } => {
                let now = state
                    .saved(self.backend.program_rules.seen(), selected.as_deref())
                    .map(|one| one.id);
                if now != *editor_of {
                    state.busy = false;
                    state.deleting = false;
                    *editor_of = now;
                }
            }
            _ => {}
        }
    }

    /// What the answers told the page.
    fn hear(&mut self) {
        for event in self.backend.take_events() {
            match &mut self.page {
                PageState::Indicator {
                    editor: Some(editor), ..
                } => editor.hear(&event),
                PageState::ValidationRule {
                    editor: Some(editor), ..
                } => editor.hear(&event),
                PageState::ProgramRules { state, .. } => state.hear(&event),
                _ => {}
            }
        }
    }

    fn settle(&mut self) {
        self.hear();
        self.went();
        self.sync();
    }

    fn advance(&mut self, to: i64) {
        // Each timer fires on its own, so a page gone to hears what comes after.
        while let Some(at) = self.backend.next_due().filter(|at| *at <= to) {
            self.backend.advance(at);
            self.settle();
        }
        self.backend.advance(to);
        self.settle();
    }

    fn seen_program_rules(&self) -> Vec<ProgramRule> {
        self.backend.program_rules.seen().to_vec()
    }

    // --- Acting ---

    fn act(&mut self, step: &Json) {
        let op = text(&step["op"]);
        let version = self.version;
        let value = &step["value"];
        let filled = || fill(value.as_str().unwrap_or_default());
        match op {
            "go" => {
                let parts: Vec<String> = texts(&step["route"])
                    .iter()
                    .map(|part| if part.contains(':') { id_of(part) } else { part.clone() })
                    .collect();
                self.backend.go(format!("/{}", parts.join("/")));
            }
            "version" => self.version = app::version_of(text(value)),
            "search" => match &mut self.page {
                PageState::Indicators { query, .. }
                | PageState::ValidationRules { query }
                | PageState::DataElements { query, .. }
                | PageState::Functions { query, .. } => *query = text(value).to_owned(),
                _ => panic!("no search here"),
            },
            "filter" => {
                let facet = text(&step["facet"]);
                let values = chosen(&step["values"]);
                match &mut self.page {
                    PageState::Indicators { types, states, .. } => match facet {
                        "Type" => *types = values,
                        _ => *states = values,
                    },
                    PageState::Functions { kinds, known, .. } => match facet {
                        "Kind" => *kinds = values,
                        _ => *known = values,
                    },
                    _ => panic!("no filter here"),
                }
            }
            "button" => {
                let seen_indicators = self.backend.indicators.seen().len();
                let seen_rules = self.backend.validation_rules.seen().len();
                let rows = self.seen_program_rules();
                match &mut self.page {
                    PageState::Indicators { .. } => indicators::create(&mut self.backend, seen_indicators),
                    PageState::ValidationRules { .. } => validation_rules::create(&mut self.backend, seen_rules),
                    PageState::ProgramRules { state, .. } => state.create(&rows, &mut self.backend),
                    _ => panic!("no button here"),
                }
            }
            "edit" if step["form"] == json!(true) => {
                let PageState::ProgramRules { state, .. } = &mut self.page else {
                    panic!("no form here")
                };
                let id = text(&step["id"]);
                if id == "try-event-date" {
                    state.form.set_event_date(value.as_str());
                    return;
                }
                let field = fields()
                    .iter()
                    .find(|one| program_rules::field_id(one.variable) == id)
                    .expect("a field");
                let next = match value {
                    Json::Null => Value::Null,
                    Json::Bool(flag) => Value::Bool(*flag),
                    Json::Number(number) => Value::Number(number.as_f64().unwrap()),
                    Json::String(one) if one.is_empty() => Value::Null,
                    Json::String(one) => Value::Text(one.clone()),
                    _ => panic!("a value"),
                };
                state.form.set(field.variable, next);
            }
            "edit" => {
                let id = text(&step["id"]);
                let rows = self.seen_program_rules();
                match &mut self.page {
                    PageState::Indicator {
                        editor: Some(editor), ..
                    } => {
                        let draft = &mut editor.draft;
                        match id {
                            "indicator-name" => draft.name = filled(),
                            "indicator-description" => draft.description = filled(),
                            "indicator-numerator" => draft.numerator = filled(),
                            "indicator-denominator" => draft.denominator = filled(),
                            _ => draft.kind = serde_json::from_value(value.clone()).expect("a type"),
                        }
                    }
                    PageState::ValidationRule {
                        editor: Some(editor), ..
                    } => {
                        let draft = &mut editor.draft;
                        match id {
                            "rule-name" => draft.name = filled(),
                            "rule-instruction" => draft.instruction = filled(),
                            "rule-left" => draft.left = filled(),
                            "rule-right" => draft.right = filled(),
                            "rule-importance" => draft.importance = serde_json::from_value(value.clone()).unwrap(),
                            "rule-operator" => draft.operator = serde_json::from_value(value.clone()).unwrap(),
                            "rule-left-strategy" => {
                                draft.left_strategy = serde_json::from_value(value.clone()).unwrap()
                            }
                            "rule-right-strategy" => {
                                draft.right_strategy = serde_json::from_value(value.clone()).unwrap()
                            }
                            "rule-from" => editor.from = filled(),
                            _ => editor.to = filled(),
                        }
                    }
                    PageState::ProgramRules { selected, state, .. } => {
                        let saved = state.saved(&rows, selected.as_deref()).expect("a rule");
                        let next = filled();
                        let number = value.as_f64();
                        state.set(&saved, |draft| match id {
                            "program-rule-name" => draft.name = next,
                            "program-rule-description" => draft.description = next,
                            "program-rule-priority" => draft.priority = number.unwrap_or(0.0),
                            _ => draft.condition = next,
                        });
                    }
                    _ => panic!("no editor here"),
                }
            }
            "unit" => {
                let unit = step["code"].as_str().map_or_else(|| root().to_owned(), unit_of);
                match &mut self.page {
                    PageState::Indicator {
                        editor: Some(editor), ..
                    } => editor.unit = unit,
                    PageState::ValidationRule {
                        editor: Some(editor), ..
                    } => editor.unit = unit,
                    _ => panic!("no unit here"),
                }
            }
            "period" => match &mut self.page {
                PageState::Indicator {
                    editor: Some(editor), ..
                } => editor.period = text(value).to_owned(),
                PageState::OrgUnits { state, .. } => state.period = text(value).to_owned(),
                _ => panic!("no period here"),
            },
            "open" => {
                let PageState::Indicator {
                    editor: Some(editor), ..
                } = &mut self.page
                else {
                    panic!("no table here")
                };
                editor.unit = unit_of(text(&step["code"]));
            }
            "save" => match &mut self.page {
                PageState::Indicator {
                    editor: Some(editor), ..
                } => editor.save(&mut self.backend),
                PageState::ValidationRule {
                    editor: Some(editor), ..
                } => editor.save(&mut self.backend),
                PageState::ProgramRules { selected, state, .. } => {
                    let rows = self.backend.program_rules.seen().to_vec();
                    let saved = state.saved(&rows, selected.as_deref()).expect("a rule");
                    state.save(&saved, &mut self.backend);
                }
                _ => panic!("nothing to save"),
            },
            "discard" => match &mut self.page {
                PageState::Indicator {
                    editor: Some(editor), ..
                } => editor.discard(),
                PageState::ValidationRule {
                    editor: Some(editor), ..
                } => editor.discard(),
                PageState::ProgramRules { state, .. } => state.discard(),
                _ => panic!("nothing to discard"),
            },
            "delete" => {
                let confirm = step["confirm"] != json!(false);
                let rows = self.seen_program_rules();
                match &mut self.page {
                    PageState::Indicator {
                        editor: Some(editor), ..
                    } => {
                        editor.deleting = true;
                        if confirm {
                            editor.remove(&mut self.backend);
                        }
                    }
                    PageState::ValidationRule {
                        editor: Some(editor), ..
                    } => {
                        editor.deleting = true;
                        if confirm {
                            editor.remove(&mut self.backend);
                        }
                    }
                    PageState::ProgramRules { selected, state, .. } => {
                        let saved = state.saved(&rows, selected.as_deref()).expect("a rule");
                        state.deleting = true;
                        if confirm {
                            state.remove(&saved, &mut self.backend);
                        }
                    }
                    _ => panic!("nothing to delete"),
                }
            }
            "run" => {
                let ids = ids_of(self.backend.indicators.seen());
                let PageState::ValidationRule {
                    editor: Some(editor), ..
                } = &mut self.page
                else {
                    panic!("nothing to run")
                };
                let checks = editor.checks(version, &ids);
                if checks.runnable()
                    && let Some(toast) = editor.start(&checks)
                {
                    self.backend.toast(toast);
                }
            }
            "select" | "tree" => {
                if let Some(reference) = step["ref"].as_str() {
                    self.backend.go(format!("/program-rules/{}", id_of(reference)));
                }
            }
            "move" => {
                let rows = self.seen_program_rules();
                let PageState::ProgramRules { state, .. } = &mut self.page else {
                    panic!("no list here")
                };
                let index = step["index"].as_u64().unwrap() as usize;
                state.reorder(&rows, &mut self.backend, &id_of(text(&step["ref"])), index);
            }
            "addAction" | "removeAction" | "moveAction" | "action" => {
                let rows = self.seen_program_rules();
                let PageState::ProgramRules { selected, state, .. } = &mut self.page else {
                    panic!("no actions here")
                };
                let saved = state.saved(&rows, selected.as_deref()).expect("a rule");
                let index = step["index"].as_u64().unwrap_or(0) as usize;
                let key = state.editing(&saved).actions.get(index).map(|one| one.key.clone());
                match op {
                    "addAction" => state.add_action(&saved, &mut self.backend),
                    "removeAction" => state.remove_action(&saved, &key.expect("an action")),
                    "moveAction" => state.move_action(
                        &saved,
                        step["from"].as_u64().unwrap() as usize,
                        step["to"].as_u64().unwrap() as usize,
                    ),
                    _ => {
                        let next = filled();
                        let field = text(&step["field"]).to_owned();
                        let value = value.clone();
                        state.set_action(&saved, &key.expect("an action"), |action| match field.as_str() {
                            "type" => action.kind = serde_json::from_value(value).expect("a type"),
                            "field" => action.field = next,
                            "content" => action.content = next,
                            _ => action.data = next,
                        });
                    }
                }
            }
            "clear" => {
                let PageState::ProgramRules { state, .. } = &mut self.page else {
                    panic!("no form here")
                };
                state.form.clear();
            }
            "wait" => {}
            other => panic!("no step {other}"),
        }
    }

    // --- Reading ---

    fn frame_look(&self) -> Json {
        let route = self.route();
        let (place, _) = app::place_of(&route);
        let indicators = self.backend.indicators.seen();
        let rules = self.backend.validation_rules.seen();
        let program_rules = self.backend.program_rules.seen();
        json!({
            "rail": to_json(&app::rail_groups(place)),
            "trail": to_json(&app::trail(&route, indicators, rules)),
            "server": { "version": self.version, "facts": app::server_facts(self.version), "status": self.version },
            "palette": to_json(&app::palette(place)),
            "search": texts(&inputs()["searches"]).iter().map(|query| to_json(&app::search(query, indicators, rules, program_rules))).collect::<Vec<_>>(),
            "searchMin": app::SEARCH_MIN_QUERY,
        })
    }

    fn page_look(&mut self) -> Json {
        let version = self.version;
        let ids = ids_of(self.backend.indicators.seen());
        let (name, mut out, foot): (&str, Json, Option<String>) = match &self.page {
            PageState::Indicators { query, types, states } => {
                let worked = indicators::worked(self.backend.indicators.seen(), version, &ids);
                let shown = indicators::shown(&worked, query, types, states);
                let check_options: Vec<(String, String)> = ["valid", "warnings", "invalid"]
                    .iter()
                    .map(|one| ((*one).to_owned(), (*one).to_owned()))
                    .collect();
                (
                    "Indicators",
                    json!({
                        "header": { "title": "Indicators", "code": null, "description": indicators::description(version) },
                        "rows": to_json(&shown),
                        "valueHeader": indicators::value_header(),
                        "tableFoot": indicators::table_foot(&worked, version),
                        "empty": indicators::empty_words(query, types, states),
                        "filters": [
                            { "facet": "Type", "options": options(indicators::type_options().into_iter().map(|(value, label)| (value.to_owned(), label.to_owned()))), "selected": types },
                            { "facet": "Check", "options": options(check_options), "selected": states },
                        ],
                    }),
                    Some(indicators::foot(shown.len(), worked.len())),
                )
            }
            PageState::Indicator { editor: None, .. } => (
                "IndicatorPage",
                json!({ "loading": false, "empty": true, "message": indicator::EMPTY }),
                self.foot.clone(),
            ),
            PageState::Indicator {
                editor: Some(editor), ..
            } => {
                let checks = editor.checks(version, &ids);
                let here = org_unit(&editor.unit).map(|one| one.name.clone());
                let below = editor.below(&checks);
                let sign = editor.unit_sign();
                let (question, detail) = editor.delete_question();
                (
                    "IndicatorPage",
                    json!({
                        "header": {
                            "title": editor.saved.name, "code": editor.saved.id,
                            "description": (!editor.saved.description.is_empty()).then(|| editor.saved.description.clone()),
                        },
                        "draft": { "name": editor.draft.name, "type": editor.draft.kind.key(), "description": editor.draft.description },
                        "typeOptions": options(indicator::type_options().into_iter().map(|(value, label)| (value.to_owned(), label))),
                        "fields": [
                            field_look("indicator-numerator", "Numerator", &editor.draft.numerator, Language::Expression, &checks.numerator),
                            field_look("indicator-denominator", "Denominator", &editor.draft.denominator, Language::Expression, &checks.denominator),
                        ],
                        "unit": [editor.unit],
                        "period": editor.period,
                        "stats": to_json(&editor.stats(&checks)),
                        "chart": {
                            "empty": !checks.runnable(), "subtitle": here,
                            "rows": editor.year(&checks).into_iter().map(|(month, value)| json!({ "month": month, "value": opt(value) })).collect::<Vec<_>>(),
                            "label": editor.chart_label(),
                        },
                        "below": (!below.is_empty()).then(|| json!({
                            "header": editor.below_header(),
                            "rows": to_json(&below),
                            "cells": below.iter().map(|one| json!([one.name, ui::figure(one.numerator, ""), ui::figure(one.denominator, ""), ui::figure(one.value, sign)])).collect::<Vec<_>>(),
                        })),
                        "changed": editor.changed(),
                        "busy": editor.busy && editor.changed() > 0,
                        "confirm": { "open": editor.deleting, "question": question, "detail": detail },
                    }),
                    Some(editor.foot(&checks)),
                )
            }
            PageState::ValidationRules { query } => {
                let worked = validation_rules::worked(self.backend.validation_rules.seen(), version, &ids);
                let shown = validation_rules::shown(&worked, query);
                (
                    "ValidationRules",
                    json!({
                        "header": { "title": "Validation rules", "code": null, "description": validation_rules::description(version) },
                        "rows": to_json(&shown),
                        "empty": validation_rules::empty_words(query),
                    }),
                    Some(validation_rules::foot(&worked)),
                )
            }
            PageState::ValidationRule { editor: None, .. } => (
                "ValidationRulePage",
                json!({ "loading": false, "empty": true, "message": validation_rule::EMPTY }),
                self.foot.clone(),
            ),
            PageState::ValidationRule {
                editor: Some(editor), ..
            } => {
                let checks = editor.checks(version, &ids);
                let (question, detail) = editor.delete_question();
                let draft = &editor.draft;
                (
                    "ValidationRulePage",
                    json!({
                        "header": {
                            "title": editor.saved.name, "code": editor.saved.id,
                            "description": (!editor.saved.instruction.is_empty()).then(|| editor.saved.instruction.clone()),
                        },
                        "draft": {
                            "name": draft.name, "importance": draft.importance.key(), "instruction": draft.instruction,
                            "operator": draft.operator.key(), "leftStrategy": draft.left_strategy.key(),
                            "rightStrategy": draft.right_strategy.key(), "from": editor.from, "to": editor.to,
                        },
                        "helps": [draft.left_strategy.description(), draft.right_strategy.description()],
                        "options": {
                            "operators": options(validation_rule::operator_options().into_iter().map(|(value, label)| (value.to_owned(), label))),
                            "strategies": options(validation_rule::strategy_options().into_iter().map(|(value, label)| (value.to_owned(), label.to_owned()))),
                            "periods": options(validation_rule::period_options()),
                        },
                        "fields": [
                            field_look("rule-left", "Left side", &draft.left, Language::Expression, &checks.left),
                            field_look("rule-right", "Right side", &draft.right, Language::Expression, &checks.right),
                        ],
                        "unit": [editor.unit],
                        "runnable": checks.runnable(),
                        "stats": to_json(&editor.stats()),
                        "rows": editor.run.as_ref().map(|_| to_json(&editor.rows())),
                        "empty": editor.run.as_ref().map(|_| "The rule holds for every site and month it was tried on."),
                        "changed": editor.changed(),
                        "busy": editor.busy && editor.changed() > 0,
                        "confirm": { "open": editor.deleting, "question": question, "detail": detail },
                    }),
                    Some(editor.foot()),
                )
            }
            PageState::ProgramRules { selected, state, .. } => {
                let rows = self.backend.program_rules.seen();
                let saved = state.saved(rows, selected.as_deref());
                let mut out = json!({
                    "nodes": to_json(&program_rules::ProgramRules::nodes(rows)),
                    "selected": saved.iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
                    "empty": saved.is_none(),
                    "message": program_rules::EMPTY,
                });
                let mut foot = self.foot.clone();
                if let Some(saved) = &saved {
                    let draft = state.editing(saved);
                    let (question, detail) = program_rules::ProgramRules::delete_question(saved);
                    let changed = state.changed(saved);
                    out["editor"] = json!({
                        "header": {
                            "title": saved.name, "code": saved.id,
                            "description": (!saved.description.is_empty()).then(|| saved.description.clone()),
                        },
                        "draft": { "name": draft.name, "priority": num(draft.priority), "description": draft.description },
                        "fields": [field_look("program-rule-condition", "Condition", &draft.condition, Language::Rule, &state.condition(saved, version))],
                        "actions": draft.actions.iter().map(|action| {
                            let shown = program_rules::action_fields(action, version);
                            json!({
                                "key": action.key,
                                "type": action.kind.key(),
                                "field": shown.field.then(|| action.field.clone()),
                                "content": shown.content.then(|| action.content.clone()),
                                "data": shown.data.then(|| field_look(&format!("{}-data", action.key), shown.data_label, &action.data, Language::Rule, &shown.checked)),
                            })
                        }).collect::<Vec<_>>(),
                        "changed": changed,
                        "busy": state.busy && changed > 0,
                        "confirm": { "open": state.deleting, "question": question, "detail": detail },
                    });
                    let rules = state.rules(rows, selected.as_deref());
                    let run = state.form.run(&rules, version);
                    let effects = program_rules::TryForm::effects(&run, &rules);
                    let broken = program_rules::TryForm::broken(&run, &rules);
                    let (stage, _) = program_rules::form_words();
                    let mut sections = vec![
                        json!({ "title": "Try the form", "meta": stage }),
                        json!({ "title": "What the rules did", "meta": effects.len().to_string() }),
                    ];
                    if !broken.is_empty() {
                        sections.push(json!({ "title": "Cannot run", "meta": broken.len().to_string() }));
                    }
                    out["form"] = json!({
                        "eventDate": state.form.event_date,
                        "fields": to_json(&state.form.shown(&run)),
                        "effects": to_json(&effects),
                        "broken": (!broken.is_empty()).then(|| to_json(&broken)),
                        "sections": sections,
                        "words": "No rule acts on the form as it is filled.",
                    });
                    foot = Some(program_rules::TryForm::foot(&run));
                }
                ("ProgramRules", out, foot)
            }
            PageState::DataElements { selected, query } => {
                let indicators = self.backend.indicators.seen();
                let rules = self.backend.validation_rules.seen();
                let rows = elements_page::rows(query, indicators, rules);
                (
                    "DataElements",
                    json!({
                        "header": { "title": "Data elements", "code": null, "description": elements_page::description() },
                        "rows": to_json(&rows),
                        "valueHeader": elements_page::value_header(),
                        "empty": elements_page::EMPTY,
                        "sheet": elements_page::sheet(selected.as_deref(), indicators, rules).map(|one| to_json(&one)),
                    }),
                    Some(elements_page::foot(rows.len())),
                )
            }
            PageState::OrgUnits { selected, state } => {
                let unit = units_page::unit_of(selected.as_deref());
                (
                    "OrgUnits",
                    json!({
                        "header": { "title": unit.name, "code": unit.id, "description": units_page::description(unit) },
                        "facts": to_json(&state.facts(unit)),
                        "chart": {
                            "empty": false, "subtitle": "sites that sent the monthly report",
                            "rows": units_page::year(unit).into_iter().map(|(month, rate)| json!({ "month": month, "rate": opt(rate) })).collect::<Vec<_>>(),
                            "label": format!("Reporting rate by month for {}", unit.name),
                        },
                        "values": to_json(&state.values(unit)),
                        "valuesDescription": units_page::values_description(unit),
                        "period": state.period,
                        "expanded": units_page::default_expanded(unit),
                        "selected": [unit.id],
                    }),
                    Some(units_page::foot(unit)),
                )
            }
            PageState::Functions { query, kinds, known } => {
                let rows = functions::rows(version);
                let shown = functions::shown(&rows, query, kinds, known);
                (
                    "Functions",
                    json!({
                        "header": { "title": "Functions", "code": null, "description": functions::description(version, &rows) },
                        "rows": to_json(&shown),
                        "knownHeader": format!("On {version}"),
                        "facets": ["Kind", format!("On {version}")],
                        "empty": functions::EMPTY,
                    }),
                    Some(functions::foot(version, &rows)),
                )
            }
            PageState::NotFound => ("NotFound", json!({}), None),
        };
        self.foot = foot;
        out["page"] = json!(name);
        out["foot"] = json!(self.foot);
        out
    }

    fn look(&mut self) -> Json {
        let out = json!({
            "route": self.hash,
            "indicators": to_json(&self.backend.indicators.rows),
            "rules": to_json(&self.backend.validation_rules.rows),
            "programRules": to_json(&self.backend.program_rules.rows),
            "saving": saving_note(&self.backend.saving).0,
            "toasts": to_json(&self.backend.toasts),
            "went": self.backend.went,
            "frame": self.frame_look(),
            "view": self.page_look(),
        });
        self.backend.toasts.clear();
        self.backend.went.clear();
        self.followed = 0;
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
    let mut app = Harness::new();
    let mut last = Map::new();
    app.advance(now + 900);
    app.backend.list_all();
    app.advance(now + 1800);
    let first = changed(app.look(), &mut last);
    same(&first, &web[0], "steps[0]");
    for (at, step) in inputs()["steps"].as_array().unwrap().iter().enumerate() {
        let when = step["at"].as_i64().unwrap();
        app.advance(now + when);
        app.act(step);
        app.settle();
        app.advance(now + when + step["wait"].as_i64().unwrap_or(900));
        let look = changed(app.look(), &mut last);
        same(&look, &web[at + 1], &format!("steps[{}] {step}", at + 1));
    }
    assert_eq!(web.len(), inputs()["steps"].as_array().unwrap().len() + 1);
}
