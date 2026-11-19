//! The data layer held against the web's own: `web/dump.ts` runs the web example's modules
//! (`model.ts`, `seed.ts`, `parse.ts`, `shape.ts`, `share.ts`, `views.ts`, `themes.ts`, and the
//! pages and dialogs with the backend behind them) under bun on the inputs in `web/inputs.json`,
//! the zone UTC, the clock and the timers held and `Math.random` a seeded run, and writes what
//! they give as JSON. Each test builds the same JSON here and asks for it to be equal, every
//! number to the bit.
//!
//! The web's answer is kept in `web/data.json`. Where bun and `.parity/sokkel-ui` are at hand the
//! dump is made again and must match it, so a change on either side shows (`DATAVIZ_WEB_KEEP=1`
//! keeps the new one); elsewhere (GitHub's CI) the kept answer is read.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use dataviz::chart_data::Cell;
use dataviz::model::{
    Aggregate, ColumnType, DataRow, Dataset, DatasetSource, DateBin, Decimals, NumberStyle, PALETTES, Preferences,
    SortOrder, THEME_TOKENS, ViewKind, ViewSpec, counted_as, operator_from_id, operator_id, order_boards, palette_of,
};
use dataviz::parse::{self, ParsedTable};
use dataviz::seed::{REGION_NAMES, seed_boards, seed_charts, seed_datasets};
use dataviz::shape::{self, Shaped, TypedRow};
use dataviz::share::{self, packed_json, shared_dataset, unpack_chart};
use dataviz::themes::CARRIED;
use dataviz::views::{VIEWS, view_label};
use serde_json::{Value, json};
use sokkel_egui::Filter;

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
            .join(".parity/sokkel-ui/examples/dataviz/web/node_modules")
            .exists()
            .then(|| {
                Command::new("bun")
                    .arg("examples/dataviz/tests/web/dump.ts")
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
                if std::env::var_os("DATAVIZ_WEB_KEEP").is_some() {
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
                            panic!(
                                "web/data.json is not what the web gives now; run with DATAVIZ_WEB_KEEP=1 to keep it"
                            )
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

fn inputs() -> &'static Value {
    static INPUTS: OnceLock<Value> = OnceLock::new();
    INPUTS.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/web/inputs.json");
        serde_json::from_str(&std::fs::read_to_string(path).expect("inputs")).expect("JSON")
    })
}

fn now() -> i64 {
    inputs()["now"].as_i64().expect("now")
}

/// A value as the web's JSON writes it: a whole number without its point.
fn norm(value: Value) -> Value {
    match value {
        Value::Number(number) => match number.as_f64() {
            Some(float) if number.is_f64() && float.fract() == 0.0 && float.abs() < 9_007_199_254_740_992.0 => {
                json!(float as i64)
            }
            _ => Value::Number(number),
        },
        Value::Array(items) => Value::Array(items.into_iter().map(norm).collect()),
        Value::Object(map) => Value::Object(map.into_iter().map(|(key, value)| (key, norm(value))).collect()),
        other => other,
    }
}

fn num(value: f64) -> Value {
    norm(json!(value))
}

fn to_json(value: &impl serde::Serialize) -> Value {
    norm(serde_json::to_value(value).expect("JSON"))
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

fn same(ours: &Value, theirs: &Value, what: &str) {
    if let Some(found) = first_difference(ours, theirs, what) {
        panic!("{found}");
    }
}

fn f(value: &Value) -> f64 {
    value.as_f64().expect("a number")
}

fn text(value: &Value) -> &str {
    value.as_str().expect("a text")
}

fn floats(value: &Value) -> Vec<f64> {
    value.as_array().expect("numbers").iter().map(f).collect()
}

/// FNV-1a over 32-bit words, as the dump hashes them.
fn texts(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("texts")
        .iter()
        .map(|one| text(one).to_owned())
        .collect()
}

fn opt_text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn core() -> &'static Value {
    &web()["core"]
}

/// A spec read from the inputs, laid over the base as the dump lays it.
fn spec_of(value: &Value) -> ViewSpec {
    let mut full = json!({
        "view": "bar", "aggregate": "sum", "bin": "month", "sort": "natural",
        "limit": 0, "stack": false, "horizontal": false, "palette": "default"
    });
    for (key, one) in value.as_object().expect("a spec") {
        full[key] = one.clone();
    }
    serde_json::from_value(full).expect("a spec")
}

/// A dataset made from a CSV text, as the dump's `imported` makes one.
fn imported(id: &str, text_in: &str) -> Dataset {
    let table = match parse::parse_table(text_in) {
        Ok((table, _)) => table,
        Err(_) => ParsedTable {
            names: text_in
                .split([';', ','])
                .map(|one| dataviz::js::trim(one).to_owned())
                .collect(),
            rows: Vec::new(),
        },
    };
    let columns = parse::columns_of(&table);
    Dataset {
        id: id.to_owned(),
        name: id.to_owned(),
        description: String::new(),
        source: DatasetSource::Paste,
        rows: parse::rows_of(&table, &columns, id),
        columns,
        updated_at: String::new(),
    }
}

fn datasets() -> &'static Vec<Dataset> {
    static DATASETS: OnceLock<Vec<Dataset>> = OnceLock::new();
    DATASETS.get_or_init(|| {
        let mut all = seed_datasets();
        for (id, csv) in inputs()["extra"].as_object().expect("extra") {
            all.push(imported(id, text(csv)));
        }
        all
    })
}

fn dataset(id: &str) -> &'static Dataset {
    datasets().iter().find(|one| one.id == id).expect("a dataset")
}

fn filters_json(filters: &[Filter]) -> Value {
    Value::Array(
        filters
            .iter()
            .map(|one| json!({"field": one.field, "operator": operator_id(one.operator), "values": one.values}))
            .collect(),
    )
}

fn filters_from(value: &Value) -> Vec<Filter> {
    value
        .as_array()
        .expect("filters")
        .iter()
        .map(|one| {
            Filter::new(
                text(&one["field"]),
                operator_from_id(text(&one["operator"])).expect("an operator"),
                texts(&one["values"]),
            )
        })
        .collect()
}

/// A shaped view in brief, as the dump writes it.
fn brief(shaped: &Shaped) -> Value {
    let mut out = to_json(shaped);
    let object = out.as_object_mut().expect("an object");
    object.remove("rows");
    object.insert("rowCount".into(), json!(shaped.rows.len()));
    object.insert(
        "rowsHead".into(),
        to_json(&shaped.rows.iter().take(3).collect::<Vec<_>>()),
    );
    object.insert("rowsLast".into(), shaped.rows.last().map_or(Value::Null, to_json));
    out
}

fn cell_of(value: &Value) -> Cell {
    match value {
        Value::String(one) => Cell::Text(one.clone()),
        Value::Number(one) => Cell::Number(one.as_f64().expect("a number")),
        _ => Cell::Null,
    }
}

#[test]
fn model_is_the_webs() {
    let web = &core()["model"];
    let types: Vec<Value> = ColumnType::ALL
        .iter()
        .map(|one| json!({"value": one.id(), "label": one.label(), "description": one.description()}))
        .collect();
    same(&json!(types), &web["columnTypes"], "columnTypes");
    let aggregates: Vec<Value> = Aggregate::ALL
        .iter()
        .map(|one| json!({"value": one.id(), "label": one.label()}))
        .collect();
    same(&json!(aggregates), &web["aggregates"], "aggregates");
    let bins: Vec<Value> = DateBin::ALL
        .iter()
        .map(|one| json!({"value": one.id(), "label": one.label()}))
        .collect();
    same(&json!(bins), &web["bins"], "bins");
    let sorts: Vec<Value> = SortOrder::ALL
        .iter()
        .map(|one| json!({"value": one.id(), "label": one.label()}))
        .collect();
    same(&json!(sorts), &web["sorts"], "sorts");
    let limits: Vec<Value> = dataviz::model::LIMITS
        .iter()
        .map(|(value, label)| json!({"value": value, "label": label}))
        .collect();
    same(&json!(limits), &web["limits"], "limits");
    let palettes: Vec<Value> = PALETTES
        .iter()
        .map(|one| match one.tokens {
            Some(tokens) => json!({"id": one.id, "label": one.label, "tokens": tokens}),
            None => json!({"id": one.id, "label": one.label}),
        })
        .collect();
    same(&json!(palettes), &web["palettes"], "palettes");
    same(&json!(THEME_TOKENS), &web["themeTokens"], "themeTokens");
    same(&to_json(&Preferences::default()), &web["defaultPrefs"], "defaultPrefs");
    let palette_ofs: Vec<Value> = inputs()["paletteOf"]
        .as_array()
        .expect("paletteOf")
        .iter()
        .map(|pair| {
            let prefs = Preferences {
                palette: text(&pair[1]).to_owned(),
                ..Preferences::default()
            };
            json!(palette_of(text(&pair[0]), &prefs))
        })
        .collect();
    same(&json!(palette_ofs), &web["paletteOf"], "paletteOf");
    let orders: Vec<Value> = inputs()["boardOrders"]
        .as_array()
        .expect("orders")
        .iter()
        .map(|order| {
            let order: Option<Vec<String>> = order.as_array().map(|_| texts(order));
            json!(
                order_boards(&seed_boards(), order.as_deref())
                    .iter()
                    .map(|board| board.id.clone())
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    same(&json!(orders), &web["orderBoards"], "orderBoards");
    let counts: Vec<Value> = inputs()["counted"]
        .as_array()
        .expect("counted")
        .iter()
        .map(|one| {
            let noun = text(&one[1]);
            let plural = opt_text(&one[2]).unwrap_or_else(|| format!("{noun}s"));
            json!(counted_as(f(&one[0]) as usize, noun, &plural))
        })
        .collect();
    same(&json!(counts), &web["counted"], "counted");
    let views: Vec<Value> = VIEWS
        .iter()
        .map(|one| json!({"value": one.value.id(), "label": one.label, "description": one.description}))
        .collect();
    same(&json!(views), &core()["views"]["views"], "views");
    let labels: Vec<&str> = ViewKind::ALL.iter().map(|one| view_label(*one)).collect();
    same(&json!(labels), &core()["views"]["labels"], "labels");
    same(&json!(CARRIED), &core()["themes"], "themes");
}

#[test]
fn seed_is_the_webs() {
    let web = &core()["seed"];
    same(&to_json(&seed_datasets()), &web["datasets"], "datasets");
    same(&to_json(&seed_charts()), &web["charts"], "charts");
    same(&to_json(&seed_boards()), &web["boards"], "boards");
    same(&json!(REGION_NAMES), &web["regions"], "regions");
}

#[test]
fn parse_is_the_webs() {
    let web = &core()["parse"];
    let input = inputs();
    let tables: Vec<Value> = input["parse"]
        .as_array()
        .expect("parse")
        .iter()
        .map(|one| match parse::parse_table(text(one)) {
            Err(error) => json!({"error": error}),
            Ok((table, format)) => {
                let columns = parse::columns_of(&table);
                json!({
                    "format": format,
                    "table": table,
                    "columns": columns,
                    "rows": to_json(&parse::rows_of(&table, &columns, "p")),
                })
            }
        })
        .collect();
    same(&json!(tables), &web["tables"], "tables");
    let delimited: Vec<Value> = input["parse"]
        .as_array()
        .expect("parse")
        .iter()
        .map(|one| json!(parse::read_delimited(text(one), None)))
        .collect();
    same(&json!(delimited), &web["delimited"], "delimited");
    let names: Vec<Value> = input["names"]
        .as_array()
        .expect("names")
        .iter()
        .map(|one| json!(parse::unique_names(&texts(one))))
        .collect();
    same(&json!(names), &web["names"], "names");
    let numbers: Vec<Value> = texts(&input["numbers"])
        .iter()
        .map(|one| parse::number_of(one).map_or(Value::Null, num))
        .collect();
    same(&json!(numbers), &web["numbers"], "numbers");
    let days: Vec<Value> = texts(&input["days"])
        .iter()
        .map(|one| json!(parse::day_of(one)))
        .collect();
    same(&json!(days), &web["days"], "days");
    let mut region_texts = texts(&input["days"]);
    region_texts.extend(["Bomi", " grand kru ", "Nowhere"].map(str::to_owned));
    let regions: Vec<bool> = region_texts.iter().map(|one| parse::is_region(one)).collect();
    same(&json!(regions), &web["regions"], "regions");
    let mut all = texts(&input["numbers"]);
    all.extend(texts(&input["days"]));
    let normalized: Vec<String> = all
        .iter()
        .flat_map(|one| ColumnType::ALL.map(|kind| parse::normalize(one, kind)))
        .collect();
    same(&json!(normalized), &web["normalized"], "normalized");
    let csv: Vec<Value> = input["csv"]
        .as_array()
        .expect("csv")
        .iter()
        .map(|one| {
            let rows: Vec<Vec<Cell>> = one[1]
                .as_array()
                .expect("rows")
                .iter()
                .map(|row| row.as_array().expect("a row").iter().map(cell_of).collect())
                .collect();
            json!(parse::to_csv(&texts(&one[0]), &rows))
        })
        .collect();
    same(&json!(csv), &web["csv"], "csv");
    let files: Vec<String> = texts(&input["files"]).iter().map(|one| parse::file_name(one)).collect();
    same(&json!(files), &web["files"], "files");
}

#[test]
fn typed_rows_bins_and_figures_are_the_webs() {
    let web = &core()["shape"];
    let typed: Vec<Value> = datasets()
        .iter()
        .map(|one| to_json(&shape::typed_rows(one).into_iter().take(12).collect::<Vec<TypedRow>>()))
        .collect();
    same(&json!(typed), &web["typed"], "typed");
    let keys: Vec<String> = inputs()["binKeys"]
        .as_array()
        .expect("binKeys")
        .iter()
        .map(|one| shape::bin_key(text(&one[0]), DateBin::from_id(text(&one[1])).expect("a bin")))
        .collect();
    same(&json!(keys), &web["binKeys"], "binKeys");
    let labels: Vec<String> = inputs()["binLabels"]
        .as_array()
        .expect("binLabels")
        .iter()
        .map(|one| {
            shape::bin_label(
                text(&one[0]),
                DateBin::from_id(text(&one[1])).expect("a bin"),
                one[2].as_bool().expect("a flag"),
            )
        })
        .collect();
    same(&json!(labels), &web["binLabels"], "binLabels");
    let formats: Vec<Value> = inputs()["prefs"]
        .as_array()
        .expect("prefs")
        .iter()
        .map(|pair| {
            let numbers = NumberStyle::ALL
                .into_iter()
                .find(|one| one.id() == text(&pair[0]))
                .expect("a style");
            let decimals = Decimals::from_id(text(&pair[1])).expect("decimals");
            json!(
                floats(&inputs()["formats"])
                    .into_iter()
                    .map(|value| shape::format_figure(numbers, decimals, value))
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    same(&json!(formats), &web["formats"], "formats");
    let breaks: Vec<Value> = inputs()["breaks"]
        .as_array()
        .expect("breaks")
        .iter()
        .map(|one| json!(shape::breaks_of(&floats(one)).into_iter().map(num).collect::<Vec<_>>()))
        .collect();
    same(&json!(breaks), &web["breaks"], "breaks");
    let collated: Vec<Value> = inputs()["collate"]
        .as_array()
        .expect("collate")
        .iter()
        .map(|names| {
            let mut sorted = texts(names);
            sorted.sort_by(|a, b| shape::naturally(a, b));
            json!(sorted)
        })
        .collect();
    same(&json!(collated), &web["collated"], "collated");
}

#[test]
fn every_view_is_shaped_as_the_webs() {
    let web = core()["shape"]["combos"].as_array().expect("combos");
    for (at, combo) in inputs()["combos"].as_array().expect("combos").iter().enumerate() {
        let data = dataset(text(&combo[0]));
        let spec = spec_of(&combo[1]);
        let ours = json!({
            "measure": shape::measure_name(data, &spec),
            "shaped": brief(&shape::shape(data, &spec, &[])),
            "suggest": to_json(&shape::suggest(data, &spec)),
        });
        same(&ours, &web[at], &format!("combos[{at}] {combo}"));
    }
}

#[test]
fn filtered_charts_fields_and_starts_are_the_webs() {
    let web = &core()["shape"];
    let mut filtered = Vec::new();
    for chart in seed_charts() {
        for filters in inputs()["filters"].as_array().expect("filters") {
            let data = dataset(&chart.dataset_id);
            let own = shape::filters_for(data, &filters_from(filters));
            filtered.push(json!({"own": filters_json(&own), "shaped": brief(&shape::shape(data, &chart.spec, &own))}));
        }
    }
    let theirs = web["filtered"].as_array().expect("filtered");
    for (at, ours) in filtered.iter().enumerate() {
        same(ours, &theirs[at], &format!("filtered[{at}]"));
    }
    let fields: Vec<Value> = datasets()
        .iter()
        .map(|one| {
            json!(
                shape::filter_fields(one, &shape::typed_rows(one))
                    .iter()
                    .map(|field| {
                        let mut out = json!({"id": field.id, "label": field.label, "type": field.kind});
                        if let Some(options) = &field.options {
                            out["options"] =
                                json!(options.iter().map(|value| json!({"value": value})).collect::<Vec<_>>());
                        }
                        out
                    })
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    same(&json!(fields), &web["fields"], "fields");
    let starting: Vec<Value> = datasets()
        .iter()
        .flat_map(|one| {
            [Aggregate::Sum, Aggregate::Avg].map(|aggregate| {
                let prefs = Preferences {
                    aggregate,
                    ..Preferences::default()
                };
                to_json(&shape::starting_spec(one, &prefs))
            })
        })
        .collect();
    same(&json!(starting), &web["starting"], "starting");
}

#[test]
fn sharing_is_the_webs() {
    let web = &web()["sharing"];
    let packs = web["packs"].as_array().expect("packs");
    for (at, chart) in seed_charts().iter().enumerate() {
        let data = dataset(&chart.dataset_id);
        let ours: Value = serde_json::from_str(&packed_json(&chart.name, &chart.spec, data)).expect("JSON");
        same(&norm(ours), &packs[at]["json"], &format!("packs[{at}].json"));
        let packed = share::pack_chart(&chart.name, &chart.spec, data, true);
        assert!(packed.starts_with('z'), "a deflated link");
        let back = json!({"ok": to_json(&unpack_chart(&packed).expect("it reads back"))});
        same(&back, &packs[at]["back"], &format!("packs[{at}].back"));
        let plain = share::pack_chart(&chart.name, &chart.spec, data, false);
        same(
            &json!({"ok": to_json(&unpack_chart(&plain).expect("it reads back"))}),
            &packs[at]["back"],
            &format!("packs[{at}] plain"),
        );
    }
    for (at, packed) in texts(&web["webPacks"]).iter().enumerate() {
        let back = json!({"ok": to_json(&unpack_chart(packed).expect("the web's link reads"))});
        same(&back, &packs[at]["back"], &format!("webPacks[{at}]"));
    }
    let unpacked: Vec<Value> = texts(&inputs()["shares"])
        .iter()
        .map(|one| match unpack_chart(one) {
            Ok(shared) => json!({"ok": to_json(&shared)}),
            Err(error) => json!({"error": error}),
        })
        .collect();
    same(&json!(unpacked), &web["unpacked"], "unpacked");
    let made: Vec<Value> = texts(&web["webPacks"])
        .iter()
        .map(|one| to_json(&shared_dataset(&unpack_chart(one).expect("it reads"), "shared", "at")))
        .collect();
    same(&json!(made), &web["datasets"], "datasets");
    let origin = text(&inputs()["origin"]);
    let path = text(&inputs()["pathname"]);
    let links: Vec<String> = ["#/shared/zabc", ""]
        .iter()
        .map(|hash| share::link_to(hash, origin, path))
        .collect();
    same(&json!(links), &web["links"], "links");
}

// --- The steps ---------------------------------------------------------------------------------------

use dataviz::backend::{Backend, ImportSource, PrefsPatch, ToastAction, saving_note};
use dataviz::chart_view::{self, chart_kind};
use dataviz::dialogs;
use dataviz::import_dialog::{ImportDialog, Picked};
use dataviz::model::{Board, PanelSpan, SavedChart};
use dataviz::pages::board::{self as board_page, BoardPage, Panel, Placed};
use dataviz::pages::boards::{self as boards_page, BoardsPage};
use dataviz::pages::builder::Builder;
use dataviz::pages::{charts as charts_page, dataset as dataset_page, datasets as datasets_page, settings, shared};
use dataviz::share::SharedChart;
use dataviz::visualizer::{Frame, Page, status};

/// FNV-1a over a text's UTF-8 bytes, as the dump hashes the rows.
fn hash_text(text_in: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in text_in.bytes() {
        hash = (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193);
    }
    hash
}

/// Rows as `JSON.stringify` writes them: each row's id first, then its values in their order.
fn rows_json(rows: &[DataRow]) -> String {
    let written: Vec<String> = rows
        .iter()
        .map(|row| {
            let mut parts = vec![format!("\"id\":{}", dataviz::js::quoted(&row.id))];
            parts.extend(
                row.values
                    .iter()
                    .filter(|(key, _)| key != "id")
                    .map(|(key, value)| format!("{}:{}", dataviz::js::quoted(key), dataviz::js::quoted(value))),
            );
            format!("{{{}}}", parts.join(","))
        })
        .collect();
    format!("[{}]", written.join(","))
}

fn dataset_summary(dataset: &Dataset) -> Value {
    let mut out = to_json(dataset);
    let object = out.as_object_mut().expect("an object");
    object.remove("rows");
    object.insert("rowCount".into(), json!(dataset.rows.len()));
    object.insert(
        "rowsHead".into(),
        to_json(&dataset.rows.iter().take(2).collect::<Vec<_>>()),
    );
    object.insert("rowsHash".into(), json!(hash_text(&rows_json(&dataset.rows))));
    out
}

/// What a confirm dialog asks, and what it does on yes.
#[derive(Clone, Debug)]
enum Confirm {
    DeleteDataset(Dataset, Vec<SavedChart>),
    DropColumn(String, dataviz::model::Column),
    DeleteChart(SavedChart, Vec<Board>),
    DeleteBoard(Board),
}

/// The page open and its own state, made anew when the page's key changes.
#[derive(Clone, Debug)]
enum PageState {
    Datasets { query: String },
    Dataset { filters: Vec<Filter>, query: String },
    Charts { query: String },
    Builder(Box<Builder>),
    Boards(BoardsPage),
    Board(BoardPage),
    Shared(Box<(Result<SharedChart, String>, Dataset)>),
    Other,
}

struct AddState {
    /// The board as its page last handed it, with the items shown.
    board: Option<Board>,
    charts: Vec<SavedChart>,
    datasets: Vec<Dataset>,
    picked: Option<String>,
    span: PanelSpan,
}

struct App {
    backend: Backend,
    route: Vec<String>,
    key: String,
    page: PageState,
    import: ImportDialog,
    board_dialog: Option<(String, String)>,
    add: Option<AddState>,
    share: Option<(String, ViewSpec, Dataset)>,
    settings: bool,
    confirming: Option<(Confirm, String, String)>,
    actions: Vec<Option<ToastAction>>,
}

fn key_of(page: &Page) -> String {
    match page {
        Page::Shared { payload } => format!("SharedPage:{payload}"),
        Page::Loading | Page::Nothing { .. } => "Page".into(),
        Page::Datasets => "DatasetsPage".into(),
        Page::Dataset { id } => format!("DatasetPage:{id}"),
        Page::Builder { dataset } => format!("BuilderPage:new-{}", dataset.clone().unwrap_or_default()),
        Page::Chart { id } => format!("BuilderPage:{id}"),
        Page::Charts => "ChartsPage".into(),
        Page::Boards => "BoardsPage".into(),
        Page::Board { id, .. } => format!("BoardPage:{id}"),
    }
}

impl App {
    fn new() -> Self {
        App {
            backend: Backend::new(now()),
            route: vec!["datasets".into()],
            key: String::new(),
            page: PageState::Other,
            import: ImportDialog::default(),
            board_dialog: None,
            add: None,
            share: None,
            settings: false,
            confirming: None,
            actions: Vec::new(),
        }
    }

    fn frame(&self) -> Frame<'_> {
        Frame::of(&self.backend, &self.route)
    }

    fn datasets(&self) -> &[Dataset] {
        self.backend.datasets.seen()
    }

    fn charts(&self) -> &[SavedChart] {
        self.backend.charts.seen()
    }

    fn boards(&self) -> Vec<Board> {
        self.frame().boards
    }

    /// The page made anew where the route or the data now name another.
    fn sync(&mut self) {
        let page = self.frame().page();
        let key = key_of(&page);
        if key == self.key {
            if let PageState::Builder(builder) = &mut self.page {
                let charts = self.backend.charts.seen().to_vec();
                builder.refresh(&charts);
            }
            return;
        }
        self.key = key;
        let prefs = self.backend.prefs();
        self.page = match page {
            Page::Datasets => PageState::Datasets { query: String::new() },
            Page::Dataset { .. } => PageState::Dataset {
                filters: Vec::new(),
                query: String::new(),
            },
            Page::Charts => PageState::Charts { query: String::new() },
            Page::Builder { dataset } => PageState::Builder(Box::new(Builder::new(
                self.datasets(),
                &prefs,
                None,
                dataset.as_deref(),
            ))),
            Page::Chart { id } => {
                let chart = self.charts().iter().find(|one| one.id == id).cloned();
                PageState::Builder(Box::new(Builder::new(self.datasets(), &prefs, chart.as_ref(), None)))
            }
            Page::Boards => PageState::Boards(BoardsPage::default()),
            Page::Board { id, .. } => {
                let board = self.boards().into_iter().find(|one| one.id == id);
                PageState::Board(BoardPage::new(board.as_ref(), self.charts()))
            }
            Page::Shared { payload } => {
                let read = share::unpack_chart(&payload);
                let made = match &read {
                    Ok(one) => shared_dataset(one, "shared", &self.backend.now_iso()),
                    Err(_) => imported("none", ""),
                };
                PageState::Shared(Box::new((read, made)))
            }
            _ => PageState::Other,
        };
    }

    fn advance(&mut self, to: i64) {
        self.backend.advance(to);
        self.sync();
    }

    fn go(&mut self, route: Vec<String>) {
        self.route = route;
        self.sync();
    }

    fn dataset_shown(&self) -> Option<Dataset> {
        match self.frame().page() {
            Page::Dataset { id } => self.datasets().iter().find(|one| one.id == id).cloned(),
            _ => None,
        }
    }

    fn board_shown(&self) -> Option<Board> {
        match self.frame().page() {
            Page::Board { id, .. } => self.boards().into_iter().find(|one| one.id == id),
            _ => None,
        }
    }

    fn act(&mut self, step: &Value) {
        let op = text(&step["op"]);
        let datasets = self.datasets().to_vec();
        let charts = self.charts().to_vec();
        let prefs = self.backend.prefs();
        match op {
            "go" => self.go(texts(&step["route"])),
            "search" => {
                let value = text(&step["value"]).to_owned();
                match &mut self.page {
                    PageState::Datasets { query } | PageState::Dataset { query, .. } | PageState::Charts { query } => {
                        *query = value
                    }
                    _ => panic!("no search here"),
                }
            }
            "rowAction" => {
                let (row, id) = (text(&step["row"]), text(&step["id"]));
                self.confirming = None;
                match (&self.page, id) {
                    (PageState::Datasets { .. }, "delete") => {
                        let one = datasets.iter().find(|one| one.id == row).expect("a dataset").clone();
                        let (question, detail) = datasets_page::delete_question(&one, &charts);
                        self.confirming = Some((Confirm::DeleteDataset(one, charts.clone()), question, detail));
                    }
                    (PageState::Datasets { .. }, "csv") => {
                        let one = datasets.iter().find(|one| one.id == row).expect("a dataset");
                        datasets_page::download(&mut self.backend, one);
                    }
                    (PageState::Dataset { .. }, _) => {
                        let dataset = self.dataset_shown().expect("a dataset");
                        match id {
                            "duplicate" => {
                                let original = dataset.rows.iter().find(|one| one.id == row).expect("a row");
                                self.backend.duplicate_row(&dataset.id, original);
                            }
                            "delete" => self.backend.remove_rows(&dataset.id, &[row.to_owned()]),
                            "drop" => {
                                let column = dataset
                                    .columns
                                    .iter()
                                    .find(|one| one.id == row)
                                    .expect("a column")
                                    .clone();
                                let (question, detail) = dataset_page::drop_question(&dataset, &charts, &column);
                                self.confirming =
                                    Some((Confirm::DropColumn(dataset.id.clone(), column), question, detail));
                            }
                            _ => panic!("no row action {id}"),
                        }
                    }
                    (PageState::Charts { .. }, _) => {
                        let chart = charts.iter().find(|one| one.id == row).expect("a chart").clone();
                        match id {
                            "duplicate" => self.backend.duplicate_chart(&chart),
                            "delete" => {
                                let boards = self.boards();
                                let (question, detail) = charts_page::delete_question(&chart, &boards);
                                self.confirming = Some((Confirm::DeleteChart(chart, boards), question, detail));
                            }
                            _ => panic!("no chart action {id}"),
                        }
                    }
                    _ => panic!("no row action {id} here"),
                }
            }
            "confirm" => {
                if let Some((confirm, _, _)) = self.confirming.take() {
                    match confirm {
                        Confirm::DeleteDataset(one, charts) => self.backend.delete_dataset(&one, &charts),
                        Confirm::DropColumn(id, column) => self.backend.drop_column(&id, &column),
                        Confirm::DeleteChart(chart, boards) => self.backend.delete_chart(&chart, &boards),
                        Confirm::DeleteBoard(board) => self.backend.delete_board(&board),
                    }
                }
            }
            "cell" => {
                let dataset = self.dataset_shown().expect("a dataset");
                let (row, column, value) = (text(&step["row"]), text(&step["column"]), text(&step["value"]));
                if step["table"].as_i64() == Some(1) {
                    let at = dataset.columns.iter().position(|one| one.id == row).expect("a column");
                    match column {
                        "name" => self
                            .backend
                            .set_column(&dataset.id, at, Some(dataviz::js::trim(value)), None),
                        _ => self
                            .backend
                            .set_column(&dataset.id, at, None, ColumnType::from_id(value)),
                    }
                } else {
                    let column = dataset.columns.iter().find(|one| one.id == column).expect("a column");
                    self.backend.save_cell(&dataset.id, row, column, value);
                }
            }
            "bulk" => {
                let dataset = self.dataset_shown().expect("a dataset");
                self.backend.remove_rows(&dataset.id, &texts(&step["ids"]));
            }
            "button" => match text(&step["label"]) {
                "Row" => {
                    let dataset = self.dataset_shown().expect("a dataset");
                    self.backend.add_row(&dataset.id);
                }
                "Column" => {
                    let dataset = self.dataset_shown().expect("a dataset");
                    self.backend.add_column(&dataset);
                }
                label => {
                    let PageState::Builder(builder) = &mut self.page else {
                        panic!("no builder")
                    };
                    match label {
                        "Save" => builder.save(&mut self.backend, false),
                        "Save as new" => builder.save(&mut self.backend, true),
                        "Discard" => builder.discard(),
                        _ => panic!("no button {label}"),
                    }
                }
            },
            "menu" => {
                self.confirming = None;
                let label = text(&step["label"]);
                match &self.page {
                    PageState::Dataset { .. } => {
                        let dataset = self.dataset_shown().expect("a dataset");
                        match label {
                            "Download CSV" => datasets_page::download(&mut self.backend, &dataset),
                            "Duplicate" => self.backend.duplicate_dataset(&dataset),
                            _ => panic!("no menu {label}"),
                        }
                    }
                    PageState::Builder(builder) => {
                        let builder = builder.clone();
                        builder.download_csv(&mut self.backend, &datasets);
                    }
                    PageState::Board(page) => {
                        let page = page.clone();
                        let board = self.board_shown().expect("a board");
                        match label {
                            "Duplicate" => page.duplicate(&mut self.backend, &board),
                            "Delete" => {
                                let (question, detail) = board_page::delete_question(Some(&board));
                                self.confirming = Some((Confirm::DeleteBoard(board), question, detail.to_owned()));
                            }
                            _ => panic!("no menu {label}"),
                        }
                    }
                    _ => panic!("no menu here"),
                }
            }
            "inline" => {
                let value = text(&step["value"]);
                let index = step["index"].as_i64().unwrap_or(0);
                match &self.page {
                    PageState::Dataset { .. } => {
                        let dataset = self.dataset_shown().expect("a dataset");
                        if index == 0 {
                            self.backend.rename_dataset(&dataset.id, value);
                        } else {
                            self.backend.describe_dataset(&dataset.id, value);
                        }
                    }
                    PageState::Board(page) => {
                        let id = page.board.clone();
                        self.backend.rename_board(&id, value);
                    }
                    _ => panic!("no inline edit here"),
                }
            }
            "filters" => {
                let filters = filters_from(&step["filters"]);
                match &mut self.page {
                    PageState::Dataset { filters: own, .. } => *own = filters,
                    PageState::Board(page) => page.set_filters(&mut self.backend, filters),
                    _ => panic!("no filters here"),
                }
            }
            "builder" => {
                let boards = self.boards();
                let PageState::Builder(builder) = &mut self.page else {
                    panic!("no builder")
                };
                let value = &step["value"];
                match text(&step["do"]) {
                    "name" => builder.set_name(text(value)),
                    "dataset" => builder.set_dataset(&datasets, &prefs, text(value)),
                    "view" => builder.set_view(&datasets, ViewKind::from_id(text(value)).expect("a view")),
                    "slot" => {
                        let field = match text(&step["field"]) {
                            "x" => dataviz::model::Field::X,
                            "y" => dataviz::model::Field::Y,
                            "series" => dataviz::model::Field::Series,
                            "size" => dataviz::model::Field::Size,
                            _ => dataviz::model::Field::Color,
                        };
                        builder.set_slot(field, text(value));
                    }
                    "aggregate" => {
                        builder.set_spec(|spec| spec.aggregate = Aggregate::from_id(text(value)).expect("an aggregate"))
                    }
                    "bin" => builder.set_spec(|spec| spec.bin = DateBin::from_id(text(value)).expect("a bin")),
                    "sort" => builder.set_spec(|spec| spec.sort = SortOrder::from_id(text(value)).expect("a sort")),
                    "limit" => builder.set_spec(|spec| spec.limit = text(value).parse().expect("a limit")),
                    "palette" => builder.set_spec(|spec| spec.palette = text(value).to_owned()),
                    "stack" => builder.set_spec(|spec| spec.stack = value.as_bool().expect("a flag")),
                    "horizontal" => builder.set_spec(|spec| spec.horizontal = value.as_bool().expect("a flag")),
                    "suggestion" => {
                        let picked = builder.suggestions(&datasets, &prefs)[value.as_u64().expect("an index") as usize]
                            .spec
                            .clone();
                        builder.set_spec(|spec| *spec = picked);
                    }
                    "addTo" => {
                        let board = boards.iter().find(|one| one.id == text(value)).expect("a board");
                        let chart = builder.chart.clone().expect("a saved chart");
                        self.backend.add_chart_to(&chart, board);
                    }
                    other => panic!("no builder step {other}"),
                }
            }
            "board" => {
                let board = self.board_shown().expect("a board");
                let PageState::Board(page) = &mut self.page else {
                    panic!("no board")
                };
                let placed: Vec<Placed> = page.placed(&board, &charts, &datasets);
                let entry = step["item"]
                    .as_str()
                    .map(|item| placed.iter().find(|one| one.item.id == item).expect("an item").clone());
                match text(&step["do"]) {
                    "takeOff" => page.take_off(&mut self.backend, &board, &entry.expect("an item")),
                    "csv" => page.download_csv(&mut self.backend, &entry.expect("an item")),
                    "edit" => page.edit(&board),
                    "done" => page.done(&mut self.backend, &board),
                    "cancel" => page.cancel(),
                    "add" => page.add(
                        &mut self.backend,
                        &board,
                        text(&step["chart"]),
                        PanelSpan::from_id(text(&step["span"])).expect("a span"),
                        step["index"].as_u64().expect("an index") as usize,
                    ),
                    other => panic!("no board step {other}"),
                }
            }
            "undo" => {
                let index = step["index"].as_u64().expect("an index") as usize;
                if let Some(Some(action)) = self.actions.iter().rev().nth(index).cloned() {
                    self.backend.press(&action);
                }
            }
            "reorder" => {
                let boards = self.boards();
                let PageState::Boards(page) = &mut self.page else {
                    panic!("no boards page")
                };
                page.reorder(
                    &mut self.backend,
                    &boards,
                    &prefs.id,
                    step["from"].as_u64().expect("from") as usize,
                    step["to"].as_u64().expect("to") as usize,
                );
            }
            "addDialog" => {
                let board = self.board_shown().expect("a board");
                let PageState::Board(page) = &self.page else {
                    panic!("no board")
                };
                let with_items = Board {
                    items: page.items(&board),
                    ..board
                };
                let mut state = AddState {
                    board: None,
                    charts: charts.clone(),
                    datasets: datasets.clone(),
                    picked: None,
                    span: PanelSpan::Half,
                };
                if let Some(pick) = step["pick"].as_str() {
                    state.picked = Some(pick.to_owned());
                }
                if let Some(span) = step["span"].as_str() {
                    state.span = PanelSpan::from_id(span).expect("a span");
                }
                if step["add"].as_bool() == Some(true) {
                    let chosen = dialogs::chosen(state.picked.as_deref(), &with_items, &charts, &datasets);
                    self.backend.add_to_board(&with_items, &chosen, state.span, &charts);
                    // Added, the dialog closes and forgets the chart picked.
                    state.picked = None;
                }
                state.board = Some(with_items);
                self.add = Some(state);
            }
            "boardDialog" => {
                let (name, description) = (text(&step["name"]).to_owned(), text(&step["description"]).to_owned());
                if step["make"].as_bool() == Some(true) {
                    self.backend.create_board(&name, &description, None, true);
                }
                self.board_dialog = Some((name, description));
            }
            "share" => {
                let chart = self
                    .backend
                    .charts
                    .rows
                    .iter()
                    .find(|one| one.id == text(&step["chart"]))
                    .expect("a chart")
                    .clone();
                let dataset = self
                    .backend
                    .datasets
                    .rows
                    .iter()
                    .find(|one| one.id == chart.dataset_id)
                    .expect("a dataset")
                    .clone();
                self.share = Some((chart.name, chart.spec, dataset));
            }
            "openShared" => {
                let payload = match step["payload"].as_str() {
                    Some(payload) => payload.to_owned(),
                    None => {
                        let (name, spec, dataset) = self.share.clone().expect("a share");
                        share::pack_chart(&name, &spec, &dataset, true)
                    }
                };
                self.go(vec!["shared".into(), payload]);
                self.advance(self.backend.now() + 1);
                if step["keep"].as_bool() == Some(true)
                    && let PageState::Shared(page) = &self.page
                    && let (Ok(shared), _) = page.as_ref()
                {
                    let shared = shared.clone();
                    self.backend.keep_shared(&shared);
                }
            }
            "settings" => {
                self.settings = true;
                let id = prefs.id.clone();
                match step["do"].as_str() {
                    Some("reset") => self.backend.reset_section(&id, text(&step["section"])),
                    Some("resetAll") => self.backend.reset_all(),
                    _ => {
                        let value = &step["value"];
                        let mut patch = PrefsPatch::default();
                        match text(&step["id"]) {
                            "palette" => patch.palette = Some(text(value).to_owned()),
                            "style" => {
                                patch.numbers = dataviz::model::NumberStyle::ALL
                                    .into_iter()
                                    .find(|one| one.id() == text(value))
                            }
                            "decimals" => patch.decimals = Decimals::from_id(text(value)),
                            "aggregate" => patch.aggregate = Aggregate::from_id(text(value)),
                            "suggestions" => patch.suggestions = value.as_bool(),
                            other => panic!("no setting {other}"),
                        }
                        let kept = self.backend.preferences.rows[0].id.clone();
                        self.backend.set_prefs(&kept, patch);
                    }
                }
            }
            "import" => {
                let value = &step["value"];
                match text(&step["do"]) {
                    "source" => self.import.set_source(if text(value) == "file" {
                        ImportSource::File
                    } else {
                        ImportSource::Paste
                    }),
                    "paste" => self.import.paste(text(value)),
                    "file" => self.import.pick(Some(Picked {
                        name: text(&step["name"]).to_owned(),
                        content: text(&step["content"]).to_owned(),
                    })),
                    "name" => self.import.name = text(value).to_owned(),
                    "column" => {
                        let index = step["index"].as_u64().expect("an index") as usize;
                        match text(&step["field"]) {
                            "name" => self.import.set_column(index, Some(text(value)), None),
                            _ => self.import.set_column(index, None, ColumnType::from_id(text(value))),
                        }
                    }
                    "save" => {
                        self.import.save(&mut self.backend);
                    }
                    "reopen" => self.import = ImportDialog::default(),
                    other => panic!("no import step {other}"),
                }
            }
            other => panic!("no step {other}"),
        }
    }

    fn chart_look(
        &self,
        dataset: &Dataset,
        spec: &ViewSpec,
        title: &str,
        subtitle: Option<&str>,
        filters: &[Filter],
    ) -> Value {
        let prefs = self.backend.prefs();
        let shaped = shape::shape(dataset, spec, filters);
        let measure = shape::measure_name(dataset, spec);
        if chart_kind(spec.view).is_some() {
            let chart = chart_view::chart_spec(&shaped, spec, &prefs, &measure);
            return json!({
                "kind": "chart",
                "label": chart_view::chart_label(title, &measure),
                "empty": chart_view::is_empty(&shaped),
                "words": chart_view::empty_words(&shaped),
                "spec": {
                    "x": chart.x, "y": chart.y, "series": chart.series, "value": chart.value, "order": chart.order,
                    "aggregate": chart.aggregate, "stack": chart.stack, "horizontal": chart.horizontal,
                    "palette": chart.palette, "fieldNames": chart.field_names.map(|(a, b)| json!([a, b])),
                    "showScale": chart.show_scale, "fit": chart.fit, "rowCount": chart.rows.len(),
                },
            });
        }
        if spec.view == ViewKind::Stat {
            return json!({
                "kind": "stat",
                "value": to_json(&chart_view::stat_value(&shaped)),
                "sparkline": shaped.stat.sparkline.as_ref().map(|line| line.iter().map(|one| num(*one)).collect::<Vec<_>>()),
                "delta": shaped.stat.delta,
                "color": chart_view::lead_color(dataviz::model::palette_of(&spec.palette, &prefs)),
                "note": chart_view::stat_note(&measure, subtitle),
            });
        }
        json!({"kind": "panel", "empty": chart_view::is_empty(&shaped), "words": chart_view::empty_words(&shaped)})
    }

    fn view(&self) -> Value {
        let datasets = self.datasets();
        let charts = self.charts();
        let boards = self.boards();
        let prefs = self.backend.prefs();
        match (&self.page, self.frame().page()) {
            (PageState::Datasets { query }, _) => json!({
                "page": "DatasetsPage",
                "count": datasets_page::count(datasets),
                "shown": datasets_page::search(datasets, query).iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
            }),
            (PageState::Dataset { filters, query }, _) => {
                let Some(dataset) = self.dataset_shown() else {
                    return json!({"page": "DatasetPage", "error": dataset_page::NO_DATASET});
                };
                let lines = dataset_page::lines(&dataset);
                let shown = dataset_page::shown(&dataset, &lines, filters, query);
                let row_of = |id: &str| dataset.rows.iter().find(|row| row.id == id).expect("a row");
                json!({
                    "page": "DatasetPage",
                    "count": dataset_page::count(&dataset),
                    "tab": dataset_page::charts_tab(&dataset, charts),
                    "rows": shown.len(),
                    "ids": shown.iter().take(5).map(|line| line.id.clone()).collect::<Vec<_>>(),
                    "headers": dataset.columns.iter().map(dataset_page::header).collect::<Vec<_>>(),
                    "cells": shown.iter().take(3).map(|line| {
                        dataset.columns.iter().map(|column| to_json(&dataset_page::cell(row_of(&line.id).text(&column.id), line.get(&column.id), &prefs))).collect::<Vec<_>>()
                    }).collect::<Vec<_>>(),
                    "stats": dataset.columns.iter().map(|column| to_json(&dataset_page::stats(&dataset, &lines, column, &prefs))).collect::<Vec<_>>(),
                    "drawn": dataset_page::drawn(&dataset, charts).iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
                    "empty": dataset_page::empty_rows(!filters.is_empty() || !dataviz::js::trim(query).is_empty()),
                })
            }
            (PageState::Charts { query }, _) => {
                let shown = charts_page::search(charts, datasets, query);
                json!({
                    "page": "ChartsPage",
                    "count": charts_page::count(charts),
                    "shown": shown.iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
                    "boards": shown.iter().map(|one| charts_page::boards_cell(one, &boards)).collect::<Vec<_>>(),
                    "cards": shown.iter().map(|one| charts_page::card_description(one, datasets)).collect::<Vec<_>>(),
                })
            }
            (PageState::Builder(builder), _) => self.builder_view(builder),
            (PageState::Boards(page), _) => json!({
                "page": "BoardsPage",
                "count": boards_page::count(&boards),
                "shown": page.shown(&boards).iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
            }),
            (PageState::Board(page), _) => {
                let Some(board) = self.board_shown() else {
                    return json!({"page": "BoardPage", "error": board_page::NO_BOARD});
                };
                let items = page.items(&board);
                let grid = !items.is_empty() || page.arranging();
                let placed = page.placed(&board, charts, datasets);
                let fields = board_page::board_fields(&BoardPage::used(&placed));
                let bar = !items.is_empty();
                json!({
                    "page": "BoardPage",
                    "count": page.count(&board),
                    "description": page.description(&board),
                    "items": if grid { items.iter().map(|one| one.id.clone()).collect::<Vec<_>>() } else { Vec::new() },
                    "arranging": grid && page.arranging(),
                    "fields": if bar { json!(fields.iter().map(|field| {
                        let mut out = json!({"id": field.id, "label": field.label, "type": field.kind});
                        if let Some(options) = &field.options { out["options"] = json!(options); }
                        out
                    }).collect::<Vec<_>>()) } else { Value::Null },
                    "note": if bar { json!(page.filter_note()) } else { Value::Null },
                    "panels": if grid { placed.iter().map(|entry| match page.panel(entry) {
                        Panel::Missing { title, text } => json!({"kind": "missing", "title": title, "text": text}),
                        Panel::Chart { title, subtitle, filters, file } => {
                            let (chart, dataset) = (entry.chart.expect("a chart"), entry.dataset.expect("a dataset"));
                            json!({
                                "kind": "chart", "title": title, "subtitle": subtitle, "filters": filters_json(&filters), "file": file,
                                "view": self.chart_look(dataset, &chart.spec, &title, Some(&subtitle), &filters),
                            })
                        }
                    }).collect::<Vec<_>>() } else { Vec::new() },
                    "palette": if grid { page.palette(&board, charts).iter().map(|one| one.id.clone()).collect::<Vec<_>>() } else { Vec::new() },
                })
            }
            (PageState::Shared(page), _) => match &page.0 {
                Err(error) => json!({"page": "SharedPage", "loading": false, "error": error}),
                Ok(one) => json!({
                    "page": "SharedPage",
                    "title": shared::title(one),
                    "description": shared::description(&page.1),
                    "chart": self.chart_look(&page.1, &one.spec, &one.name, Some(&page.1.name), &[]),
                }),
            },
            (PageState::Other, page) => match page {
                Page::Loading => json!({"page": "Page", "loading": true, "message": null}),
                Page::Nothing { message } => json!({"page": "Page", "loading": false, "message": message}),
                _ => panic!("a page without its state"),
            },
        }
    }

    fn builder_view(&self, builder: &Builder) -> Value {
        let datasets = self.datasets();
        let prefs = self.backend.prefs();
        let Some(dataset) = builder.dataset(datasets) else {
            return json!({"page": "BuilderPage", "empty": "There is no dataset to chart. Import one, or reset the demo data, and come back."});
        };
        let draft = builder.draft.as_ref().expect("a draft");
        let mut palettes = vec![json!(["default", Builder::default_palette_label(&prefs)])];
        palettes.extend(dataviz::model::PALETTES.iter().map(|one| json!([one.id, one.label])));
        let mut save = Vec::new();
        if builder.chart.is_some() {
            save.push(json!([
                "Discard",
                (!builder.dirty()).then_some("Nothing has changed yet.")
            ]));
            save.push(json!(["Save as new", builder.save_as_new_held()]));
        }
        save.push(json!(["Save", builder.save_held()]));
        let boards = self.boards();
        json!({
            "page": "BuilderPage",
            "header": builder.header(),
            "slots": to_json(&builder.slot_fields(datasets)),
            "combine": builder.combine(datasets).map(|(value, numbers)| json!([value, numbers])),
            "bin": builder.shows_bin(datasets).then_some(draft.spec.bin),
            "sort": builder.sort_labels(datasets),
            "palettes": palettes,
            "stack": builder.stack_switch().map(|(a, b)| json!([a, b])),
            "horizontal": builder.horizontal_switch(),
            "note": builder.preview_note(datasets),
            "suggestions": builder.suggestions(datasets, &prefs).iter().map(|one| json!({"title": one.title, "reason": one.reason})).collect::<Vec<_>>(),
            "formNote": builder.note(),
            "save": save,
            "others": builder.other_boards(&boards).iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
            "preview": self.chart_look(dataset, &draft.spec, &builder.title(), Some(&dataset.name), &[]),
            "draft": {"name": draft.name, "dataset": draft.dataset_id},
        })
    }

    fn dialogs(&self, web: &Value) -> Value {
        let import = &self.import;
        json!({
            "import": {
                "why": import.why(),
                "footer": import.footer(),
                "name": import.table.as_ref().map(|_| import.name.clone()),
                "lines": import.lines().iter().filter(|_| import.table.is_some()).map(|one| json!({"name": one.column.name, "type": one.column.kind, "sample": one.sample})).collect::<Vec<_>>(),
            },
            "board": self.board_dialog.as_ref().map(|(name, _)| json!({
                "title": dialogs::board_title(false), "description": dialogs::board_description(None),
                "confirm": dialogs::board_confirm(false), "why": dialogs::board_why(name),
            })),
            "add": self.add.as_ref().and_then(|state| {
                let board = state.board.clone()?;
                let (charts, datasets) = (&state.charts, &state.datasets);
                let chosen = dialogs::chosen(state.picked.as_deref(), &board, charts, datasets);
                Some(json!({
                    "description": dialogs::add_description(&board),
                    "why": dialogs::add_why(&chosen),
                    "chosen": chosen,
                    "offered": to_json(&dialogs::offered(&board, charts, datasets)),
                }))
            }),
            "share": self.share.as_ref().map(|(name, spec, dataset)| {
                // The link is the web's own: its deflated bytes are zlib's, and only read back here.
                let link = web["share"]["link"].as_str().unwrap_or_default();
                let payload = link.split("#/shared/").nth(1).unwrap_or_default();
                let read = unpack_chart(payload).expect("the web's link reads back");
                assert_eq!(read.name, *name);
                assert_eq!(read.spec, *spec);
                assert_eq!(read.dataset.rows.len(), dataset.rows.len());
                json!({
                    "description": dialogs::share_description(name, dataset),
                    "link": share::link_to(&format!("#/shared/{payload}"), text(&inputs()["origin"]), text(&inputs()["pathname"])),
                    "footer": dialogs::link_size(link),
                })
            }),
            "settings": self.settings.then(|| {
                let prefs = &self.backend.preferences.rows[0];
                json!({"style": settings::style_description(prefs), "reset": settings::reset_description(&self.backend.datasets.rows)})
            }),
            "confirm": self.confirming.as_ref().map(|(_, question, detail)| json!({"question": question, "detail": detail})),
        })
    }

    fn look(&mut self, web: &Value) -> Value {
        if let (Some(board), PageState::Board(page)) = (self.board_shown(), &self.page) {
            let board = Board {
                items: page.items(&board),
                ..board
            };
            let (charts, datasets) = (self.charts().to_vec(), self.datasets().to_vec());
            if let Some(state) = &mut self.add {
                state.board = Some(board);
                state.charts = charts;
                state.datasets = datasets;
            }
        }
        let frame = self.frame();
        let rail: Vec<Value> = frame.rail().iter().map(|group| json!({
            "id": group.id,
            "title": group.title,
            "entries": group.entries.iter().map(|one| json!({"id": one.id, "label": one.label, "href": one.href, "active": one.active, "count": one.count})).collect::<Vec<_>>(),
        })).collect();
        let palette: Vec<Value> = frame.commands().iter().map(|group| json!({
            "title": group.title,
            "rank": group.rank,
            "commands": group.commands.iter().map(|one| json!({"id": one.id, "label": one.label, "hint": one.hint, "keywords": one.keywords})).collect::<Vec<_>>(),
        })).collect();
        let (note, tone, live, counts) = status(&self.backend);
        let frame_look =
            json!({"rail": rail, "palette": palette, "trail": to_json(&frame.trail()), "status": counts, "note": note});
        let (saving_text, saving_tone, saving_live) = saving_note(&self.backend.saving);
        let _ = (tone, live);
        let toasts = std::mem::take(&mut self.backend.toasts);
        self.actions.extend(toasts.iter().map(|(_, action)| action.clone()));
        let out = json!({
            "datasets": self.backend.datasets.rows.iter().map(dataset_summary).collect::<Vec<_>>(),
            "charts": to_json(&self.backend.charts.rows),
            "boards": to_json(&self.backend.boards.rows),
            "preferences": to_json(&self.backend.preferences.rows),
            "saving": {"note": saving_text, "tone": saving_tone, "live": saving_live},
            "toasts": toasts.iter().map(|(toast, _)| to_json(toast)).collect::<Vec<_>>(),
            "went": std::mem::take(&mut self.backend.went),
            "downloads": to_json(&std::mem::take(&mut self.backend.downloads)),
            "frame": frame_look,
            "view": self.view(),
            "dialogs": self.dialogs(web),
        });
        norm(out)
    }
}

#[test]
fn every_step_is_the_webs() {
    let web = web()["steps"].as_array().expect("steps");
    let mut app = App::new();
    app.advance(now() + 900);
    let mut last = serde_json::Map::new();
    let mut dialogs_seen = Value::Null;
    let check = |at: usize, look: Value, last: &mut serde_json::Map<String, Value>, step: &Value| {
        let theirs = web[at].as_object().expect("a look");
        for (key, value) in look.as_object().expect("a look") {
            let unchanged = last.get(key) == Some(value);
            match theirs.get(key) {
                Some(web_value) => same(value, web_value, &format!("steps[{at}] {step} .{key}")),
                None => assert!(
                    unchanged,
                    "steps[{at}] {step} .{key}: ours changed, the web's did not: {value}"
                ),
            }
            last.insert(key.clone(), value.clone());
        }
    };
    let first = app.look(&dialogs_seen);
    check(0, first, &mut last, &Value::Null);
    for (at, step) in inputs()["steps"].as_array().expect("steps").iter().enumerate() {
        let start = now() + step["at"].as_i64().expect("at");
        app.advance(start);
        app.act(step);
        app.sync();
        let wait = step["wait"].as_i64().unwrap_or(900);
        app.advance(start + wait);
        if step["follow"].as_bool() == Some(true) {
            if let Some(target) = app.backend.went.iter().rev().find(|one| *one != "reload").cloned() {
                let target = target
                    .trim_start_matches("replace ")
                    .trim_start_matches('#')
                    .trim_start_matches('/');
                app.go(target
                    .split('/')
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect());
            }
            app.advance(start + wait + 1);
        }
        if let Some(dialogs) = web[at + 1].get("dialogs") {
            dialogs_seen = dialogs.clone();
        }
        let look = app.look(&dialogs_seen);
        check(at + 1, look, &mut last, step);
    }
}
