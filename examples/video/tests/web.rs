//! The data layer held against the web's own: `web/dump.ts` runs the web example's modules
//! (`catalog.ts`, `format.ts`, `themes.ts`, `clips/index.ts`, `clips/source.ts`, `data.ts`,
//! `dialogs.tsx`, `parts.tsx`, `Site.tsx`, every page but Settings and NotFound, and the shared
//! `scenes.ts` and `sound.ts` the clips are made from) under bun on the inputs in
//! `web/inputs.json`, the zone UTC, the clock and the timers held and `Math.random` a seeded run,
//! and writes what they give as JSON. Each test builds the same JSON here and asks for it to be
//! equal, every number to the bit.
//!
//! The web's answer is kept in `web/data.json`. Where bun and `.parity/sokkel-ui` are at hand the
//! dump is made again and must match it, so a change on either side shows (`VIDEO_WEB_KEEP=1`
//! keeps the new one); elsewhere (GitHub's CI) the kept answer is read.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use serde_json::{Map, Value, json};
use tidevann_shared::clips::Seeded;
use video::catalog::{
    CHANNELS, Person, TOPICS, VIDEOS, VIEWER, channel_by_id, channel_of, seed_of, video_by_id, videos_of,
};
use video::clips::source::stats_of;
use video::clips::{RENDITIONS, captions_of, clip_of, rendition_of, spec_of, synthesize};
use video::data::{Data, PrefsPatch, Quality, RatingKind, WatchedPatch};
use video::dialogs::{link_of, save_entries, save_new, save_toggle, share, validate_name};
use video::format::{bitrate_of, bytes_of, clock_of, days_ago, plural, short, views_of};
use video::pages::watch::aside::{self, Filter, up_next_for};
use video::pages::watch::comments::{self, Order};
use video::pages::watch::{PlayerState, Watching};
use video::pages::{
    channel, downloads, history, home, library, liked, playlist, playlists, search, settings, subscriptions,
};
use video::parts::{CardLayout, go_to_channel, toggle_watch_later, video_card, video_menu, watch_href};
use video::site::{self, palette_search, player_commands, playlist_commands, rail, run_command, status, trail};
use video::themes::CARRIED;

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
            .join(".parity/sokkel-ui/examples/video/web/node_modules")
            .exists()
            .then(|| {
                Command::new("bun")
                    .arg("examples/video/tests/web/dump.ts")
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
                if std::env::var_os("VIDEO_WEB_KEEP").is_some() {
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
                            panic!("web/data.json is not what the web gives now; run with VIDEO_WEB_KEEP=1 to keep it")
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

fn base() -> &'static str {
    inputs()["base"].as_str().expect("base")
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
fn hash32(words: impl IntoIterator<Item = u32>) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for word in words {
        hash ^= word;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn video(id: &str) -> &'static video::catalog::Video {
    video_by_id(id).unwrap_or_else(|| panic!("no video {id}"))
}

#[test]
fn catalog_is_the_webs() {
    let web = &web()["library"];
    let input = inputs();
    same(&json!(VIEWER), &web["viewer"], "viewer");
    let colors: Map<String, Value> = Person::ALL
        .iter()
        .map(|person| (person.name().to_owned(), json!(person.color())))
        .collect();
    same(&Value::Object(colors), &web["peopleColors"], "peopleColors");
    same(&to_json(&CHANNELS), &web["channels"], "channels");
    let topics: Map<String, Value> = TOPICS
        .iter()
        .map(|(topic, label)| (topic.id().to_owned(), json!(label)))
        .collect();
    same(&Value::Object(topics), &web["topics"], "topics");
    same(&to_json(&VIDEOS), &web["videos"], "videos");
    let of: Vec<Value> = VIDEOS.iter().map(|one| json!(channel_of(one).id)).collect();
    same(&Value::Array(of), &web["channelOf"], "channelOf");
    let videos: Vec<Value> = CHANNELS
        .iter()
        .map(|one| json!(videos_of(one).iter().map(|v| v.id).collect::<Vec<_>>()))
        .collect();
    same(&Value::Array(videos), &web["videosOf"], "videosOf");
    let mut ids: Vec<&str> = VIDEOS.iter().map(|one| one.id).collect();
    ids.push("nowhere");
    let mut by_id: Vec<Value> = ids.iter().map(|id| json!(video_by_id(id).map(|one| one.id))).collect();
    by_id.push(Value::Null);
    same(&Value::Array(by_id), &web["byId"], "byId");
    let mut channel_ids: Vec<&str> = CHANNELS.iter().map(|one| one.id).collect();
    channel_ids.push("nowhere");
    let by_channel: Vec<Value> = channel_ids
        .iter()
        .map(|id| json!(channel_by_id(id).map(|one| one.id)))
        .collect();
    same(&Value::Array(by_channel), &web["channelById"], "channelById");
    let mut seeded: Vec<String> = VIDEOS.iter().map(|one| one.id.to_owned()).collect();
    seeded.extend(
        input["seeds"]
            .as_array()
            .expect("seeds")
            .iter()
            .map(|one| text(one).to_owned()),
    );
    let seeds: Vec<Value> = seeded.iter().map(|id| json!(seed_of(id))).collect();
    same(&Value::Array(seeds), &web["seeds"], "seeds");
    same(&json!(CARRIED), &web["themes"], "themes");
    same(&to_json(&RENDITIONS), &web["renditions"], "renditions");
    let renditions: Vec<Value> = input["renditionOf"]
        .as_array()
        .expect("renditionOf")
        .iter()
        .map(|id| json!(rendition_of(text(id)).id.id()))
        .collect();
    same(&Value::Array(renditions), &web["renditionOf"], "renditionOf");
    let specs: Vec<Value> = VIDEOS
        .iter()
        .map(|one| {
            let spec = spec_of(one);
            norm(json!({
                "kind": spec.kind.name(),
                "hue": spec.hue,
                "accent": spec.accent,
                "seed": spec.seed,
                "title": spec.title,
                "channel": spec.channel,
                "duration": spec.duration,
            }))
        })
        .collect();
    same(&Value::Array(specs), &web["specs"], "specs");
    let random: Vec<Value> = VIDEOS
        .iter()
        .map(|one| {
            let mut next = Seeded::new(seed_of(one.id));
            Value::Array((0..8).map(|_| num(next.next())).collect())
        })
        .collect();
    same(&Value::Array(random), &web["random"], "random");
    let up_next: Vec<Value> = VIDEOS
        .iter()
        .map(|one| json!(up_next_for(one).iter().map(|v| v.id).collect::<Vec<_>>()))
        .collect();
    same(&Value::Array(up_next), &web["upNext"], "upNext");
    let texts: Vec<Value> = VIDEOS.iter().map(|one| json!(site::video_text(one))).collect();
    same(&Value::Array(texts), &web["videoText"], "videoText");
    let hrefs: Vec<Value> = input["hrefs"]
        .as_array()
        .expect("hrefs")
        .iter()
        .map(|pair| json!(watch_href(video(text(&pair[0])), pair[1].as_str())))
        .collect();
    same(&Value::Array(hrefs), &web["hrefs"], "hrefs");
    let links: Vec<Value> = input["links"]
        .as_array()
        .expect("links")
        .iter()
        .map(|pair| json!(link_of(base(), video(text(&pair[0])), pair[1].as_f64())))
        .collect();
    same(&Value::Array(links), &web["links"], "links");
}

#[test]
fn words_are_the_webs() {
    let web = &web()["words"];
    let input = inputs();
    let over = |key: &str, write: &dyn Fn(f64) -> String| -> Value {
        Value::Array(floats(&input[key]).into_iter().map(|one| json!(write(one))).collect())
    };
    same(&over("short", &short), &web["short"], "short");
    same(&over("short", &views_of), &web["viewsOf"], "viewsOf");
    same(&over("daysAgo", &days_ago), &web["daysAgo"], "daysAgo");
    same(&over("bytes", &bytes_of), &web["bytesOf"], "bytesOf");
    same(&over("bitrates", &bitrate_of), &web["bitrateOf"], "bitrateOf");
    same(&over("clocks", &clock_of), &web["clockOf"], "clockOf");
    let plurals: Vec<Value> = input["plural"]
        .as_array()
        .expect("plural")
        .iter()
        .map(|case| json!(plural(f(&case[0]), text(&case[1]), case[2].as_str())))
        .collect();
    same(&Value::Array(plurals), &web["plural"], "plural");
    let captions: Vec<Value> = VIDEOS.iter().map(|one| json!(captions_of(one))).collect();
    same(&Value::Array(captions), &web["captions"], "captions");
}

#[test]
fn sound_is_the_webs() {
    let web = web()["sound"].as_array().expect("sound");
    for (id, web) in inputs()["sound"].as_array().expect("sound").iter().zip(web) {
        let id = text(id);
        let (left, right) = synthesize(&spec_of(video(id)));
        let at = [0, 1000, 48_000, 480_000, left.len() - 1];
        let ours = json!({
            "id": id,
            "frames": left.len(),
            "left": hash32(left.iter().map(|one| one.to_bits())),
            "right": hash32(right.iter().map(|one| one.to_bits())),
            "samples": at
                .iter()
                .map(|index| json!([
                    left.get(*index).map_or(Value::Null, |one| num(f64::from(*one))),
                    right.get(*index).map_or(Value::Null, |one| num(f64::from(*one))),
                ]))
                .collect::<Vec<_>>(),
        });
        same(&ours, web, &format!("sound {id}"));
    }
}

#[test]
fn sources_are_the_webs() {
    let web = &web()["sources"];
    let ours: Vec<Value> = inputs()["bandwidth"]
        .as_array()
        .expect("bandwidth")
        .iter()
        .map(|case| {
            let clip = clip_of(video(text(&case[0])), rendition_of(text(&case[1])));
            to_json(&stats_of(&clip, f(&case[2])))
        })
        .collect();
    same(&Value::Array(ours), web, "sources");
}

/// The site as the steps drive it: the data, the watch page open, the player's state, and the
/// comments reported, for Undo.
struct Site {
    data: Data,
    watching: Option<Watching>,
    player: PlayerState,
    reported: Vec<String>,
}

impl Site {
    fn look(&mut self, answer: Value) -> Map<String, Value> {
        let data = &mut self.data;
        let seen = json!({
            "follows": to_json(&data.follows.rows),
            "ratings": to_json(&data.ratings.rows),
            "history": to_json(&data.history.rows),
            "playlists": to_json(&data.playlists.rows),
            "comments": to_json(&data.comments.rows),
            "downloads": to_json(&data.downloads.rows),
            "prefsRows": to_json(&data.prefs_rows.rows),
            "prefs": to_json(&data.prefs),
            "prefsReady": data.prefs_ready,
            "toasts": to_json(&std::mem::take(&mut data.toasts)),
            "went": to_json(&std::mem::take(&mut data.went)),
            "answer": answer,
        });
        match seen {
            Value::Object(map) => map,
            _ => unreachable!(),
        }
    }

    fn view(&mut self, step: &Value) -> Value {
        let data = &self.data;
        let s = |key: &str| text(&step[key]);
        match s("page") {
            "home" => to_json(&home::view(data, home::Chip::from_value(s("chip")).expect("chip"))),
            "search" => {
                let kind = match s("kind") {
                    "videos" => search::Kind::Videos,
                    "channels" => search::Kind::Channels,
                    _ => search::Kind::All,
                };
                let sort = match s("sort") {
                    "date" => search::Sort::Date,
                    "views" => search::Sort::Views,
                    _ => search::Sort::Relevance,
                };
                to_json(&search::view(data, s("query"), kind, sort))
            }
            "subscriptions" => to_json(&subscriptions::view(data, s("chosen"))),
            "history" => to_json(&history::view(data, s("search"))),
            "playlist" => to_json(&playlist::view(data, s("id"))),
            "playlists" => to_json(&playlists::view(data)),
            "library" => to_json(&library::view(data)),
            "liked" => json!(liked::videos(data).iter().map(|one| one.id).collect::<Vec<_>>()),
            "channel" => {
                let sort = match s("sort") {
                    "popular" => channel::Sort::Popular,
                    "oldest" => channel::Sort::Oldest,
                    _ => channel::Sort::Latest,
                };
                to_json(&channel::view(data, channel_by_id(s("id")).expect("channel"), sort))
            }
            "downloads" => to_json(&downloads::view(data)),
            "comments" => {
                let one = video(s("video"));
                let order = if s("order") == "new" { Order::New } else { Order::Top };
                to_json(&comments::view(data, one, channel_of(one), order))
            }
            "aside" => {
                let one = video(s("video"));
                let list = step["list"].as_str().and_then(|id| data.playlists.seen_get(id));
                let filter = match s("filter") {
                    "channel" => Filter::Channel,
                    "topic" => Filter::Topic,
                    "unwatched" => Filter::Unwatched,
                    _ => Filter::All,
                };
                to_json(&aside::view(data, one, list, filter))
            }
            "card" => {
                let layout = if s("layout") == "row" {
                    CardLayout::Row
                } else {
                    CardLayout::Stack
                };
                to_json(&video_card(
                    data,
                    video(s("id")),
                    layout,
                    step["bare"] == json!(true),
                    step["list"].as_str(),
                    step["playing"] == json!(true),
                ))
            }
            "menu" => json!(video_menu(data, video(s("id")))),
            "save" => to_json(&save_entries(data, video(s("id")))),
            "share" => to_json(&share(
                base(),
                video(s("id")),
                step["second"].as_f64(),
                step["fromHere"] == json!(true),
            )),
            "site" => {
                let route: Vec<String> = step["route"]
                    .as_array()
                    .expect("route")
                    .iter()
                    .map(|one| text(one).to_owned())
                    .collect();
                json!({
                    "rail": to_json(&rail(data, &route)),
                    "trail": to_json(&trail(data, &route)),
                    "playlists": to_json(&playlist_commands(data).into_iter().map(|(one, _)| one).collect::<Vec<_>>()),
                    "player": to_json(&player_commands(data).into_iter().map(|(one, _)| one).collect::<Vec<_>>()),
                    "status": to_json(&status(data)),
                })
            }
            "watch" => to_json(&self.watching.as_ref().expect("a video open").view(data)),
            "settings" => {
                let prefs = &data.prefs;
                let controls = settings::caption_controls(data);
                let notes = settings::notes(data);
                json!({
                    "toggles": [prefs.autoplay, prefs.theater, prefs.stats, controls.on, prefs.history],
                    "speed": video::js::number_to_string(prefs.rate),
                    "speeds": settings::speeds()
                        .into_iter()
                        .map(|(rate, label)| json!([video::js::number_to_string(rate), label]))
                        .collect::<Vec<_>>(),
                    "quality": prefs.quality.id(),
                    "qualities": settings::qualities()
                        .into_iter()
                        .map(|(quality, label)| json!([quality.id(), label]))
                        .collect::<Vec<_>>(),
                    "captions": {
                        "size": controls.size,
                        "color": controls.color,
                        "background": controls.background,
                        "edge": controls.edge,
                    },
                    "sizes": settings::caption_sizes(),
                    "colors": settings::colors(),
                    "backgrounds": settings::backgrounds(),
                    "edges": settings::edges(),
                    "history": notes.history,
                    "clear_disabled": notes.clear_disabled,
                    "downloads": notes.downloads,
                })
            }
            other => panic!("no page {other}"),
        }
    }

    /// Does what a step's button does; the step's answer, if it has one.
    fn act(&mut self, step: &Value) -> Value {
        let s = |key: &str| text(&step[key]);
        let on = || step["on"].as_bool().expect("on");
        let data = &mut self.data;
        match s("op") {
            "view" => return self.view(step),
            "follow" => search::follow(data, channel_by_id(s("channel")).expect("channel"), on()),
            "setFollowing" => data.set_following(s("channel"), on()),
            "prefs" => {
                let patch: PrefsPatch = serde_json::from_value(step["patch"].clone()).expect("a patch");
                data.set_prefs(&patch);
            }
            "keepWatched" => {
                let patch: WatchedPatch = serde_json::from_value(step["patch"].clone()).expect("a patch");
                data.keep_watched(s("id"), patch);
            }
            "menuLater" => toggle_watch_later(data, video(s("id"))),
            "menuChannel" => go_to_channel(data, video(s("id"))),
            "saveToggle" => save_toggle(data, s("list"), video(s("video")), on()),
            "saveNew" | "playlistsNew" => {
                let said = validate_name(data, s("name"));
                if said.is_none() {
                    if s("op") == "saveNew" {
                        save_new(data, s("name"), video(s("video")));
                    } else {
                        playlists::add(data, s("name"));
                    }
                }
                return json!(said);
            }
            "playlistsDelete" => {
                let list = data
                    .playlists
                    .seen()
                    .iter()
                    .find(|one| one.name == s("name"))
                    .cloned()
                    .expect("a playlist");
                playlists::delete(data, &list);
            }
            "playlist" => {
                let id = s("id");
                let index = || step["index"].as_u64().expect("index") as usize;
                match s("do") {
                    "rename" => {
                        let said = playlist::validate_name(s("name"));
                        if said.is_none() {
                            playlist::rename(data, id, s("name"));
                        }
                        return json!(said);
                    }
                    "describe" => playlist::describe(data, id, s("text")),
                    "up" => playlist::move_by(data, id, index(), -1),
                    "down" => playlist::move_by(data, id, index(), 1),
                    "drag" => playlist::drag(
                        data,
                        id,
                        step["from"].as_u64().expect("from") as usize,
                        step["to"].as_u64().expect("to") as usize,
                    ),
                    "remove" => {
                        let list = data.playlists.seen_get(id).cloned().expect("a playlist");
                        let one = playlist::videos_of(&list)[index()].id;
                        playlist::remove(data, id, one);
                    }
                    "play" => playlist::play_all(data, id),
                    "shuffle" => playlist::shuffle(data, id),
                    "delete" => playlist::delete(data, id),
                    other => panic!("no playlist step {other}"),
                }
            }
            "settings" => match s("do") {
                "reset" => settings::reset(data, s("section")),
                "toggle" => settings::toggle(data, s("id"), on()),
                "select" | "segment" => {
                    let value = s("value");
                    match s("id") {
                        "speed" => settings::speed(data, value.parse().expect("a speed")),
                        "quality" => settings::quality(data, Quality::from_id(value).expect("a quality")),
                        "caption-size" => settings::caption_size(data, value.parse().expect("a size")),
                        "caption-color" => settings::caption_color(data, value),
                        "caption-background" => settings::caption_background(data, value),
                        "caption-edge" => settings::caption_edge(data, value),
                        other => panic!("no control {other}"),
                    }
                }
                "clear" => settings::clear_history(data),
                other => panic!("no settings step {other}"),
            },
            "historyRemove" => history::remove(data, s("id")),
            "historyClear" => history::clear(data, step["clearing"] == json!(true)),
            "historyKeep" => history::keep_history(data, on()),
            "likedRemove" => liked::remove(data, s("id")),
            "likedPlay" => liked::play_latest(data),
            "downloadsRemove" => downloads::remove(data, s("id")),
            "comment" => {
                let one = video(s("video"));
                let target = || -> String {
                    match step["id"].as_str() {
                        Some(id) => id.to_owned(),
                        None => comments::here(&self.data, one)
                            .iter()
                            .find(|comment| comment.text == s("body"))
                            .map(|comment| comment.id.clone())
                            .expect("a comment"),
                    }
                };
                let id = if matches!(s("do"), "add" | "undo") {
                    String::new()
                } else {
                    target()
                };
                let data = &mut self.data;
                match s("do") {
                    "add" => comments::add(data, one, s("text"), None),
                    "reply" => comments::add(data, one, s("text"), Some(&id)),
                    "like" => comments::rate(data, one, &id, true),
                    "dislike" => comments::rate(data, one, &id, false),
                    "react" => comments::react(data, one, &id, s("emoji"), s("name")),
                    "menu" => {
                        let mine = comments::here(data, one)
                            .iter()
                            .find(|comment| comment.id == id)
                            .is_some_and(|comment| comment.author == VIEWER);
                        if mine {
                            comments::remove(data, one, &id);
                        } else {
                            comments::report(data, &id);
                            self.reported.push(id);
                        }
                    }
                    "undo" => {
                        if let Some(id) = self.reported.pop() {
                            comments::unreport(data, &id);
                        }
                    }
                    other => panic!("no comment step {other}"),
                }
            }
            "palette" => {
                let found = playlist_commands(data)
                    .into_iter()
                    .chain(player_commands(data))
                    .find(|(one, _)| one.id == s("id"))
                    .expect("a command");
                run_command(data, &found.1);
            }
            "paletteSearch" => {
                let groups = palette_search(s("text"));
                if let Some(open) = step["open"].as_array() {
                    site::open_result(data, text(&open[0]), text(&open[1]));
                }
                return to_json(&groups);
            }
            "submit" => site::submit(data, s("text")),
            "open" => {
                self.watching = Some(
                    Watching::open(
                        s("id"),
                        step["list"].as_str(),
                        step["start"].as_f64(),
                        f(&inputs()["width"]) as f32,
                    )
                    .expect("a video"),
                );
                self.player = PlayerState::default();
            }
            "watch" => {
                let watching = self.watching.as_mut().expect("a video open");
                let data = &mut self.data;
                let player = &mut self.player;
                match s("do") {
                    "play" => {
                        player.playing = true;
                        if let Some(time) = step["time"].as_f64() {
                            player.time = time;
                        }
                        watching.playing_changed(data, true, *player);
                    }
                    "pause" => {
                        player.playing = false;
                        player.time = f(&step["time"]);
                        watching.playing_changed(data, false, *player);
                    }
                    "time" => {
                        player.time = f(&step["time"]);
                        watching.time(data, player.time);
                    }
                    "ended" => {
                        player.playing = false;
                        player.time = watching.video.duration;
                        watching.playing_changed(data, false, *player);
                        watching.ended(data);
                    }
                    "quality" => {
                        let quality = Quality::from_id(s("quality")).expect("a quality");
                        watching.choose_quality(data, quality, Some(*player));
                    }
                    "rate" => watching.rate_changed(data, f(&step["rate"])),
                    "rating" => {
                        let rating = match step["rating"].as_str() {
                            Some("like") => Some(RatingKind::Like),
                            Some("dislike") => Some(RatingKind::Dislike),
                            _ => None,
                        };
                        watching.rate(data, rating);
                    }
                    "follow" => watching.follow(data, on()),
                    "download" => watching.download(data),
                    "cancel" => watching.cancel_download(data),
                    "remove" => watching.remove_download(data),
                    "downloadEntry" => watching.download_entry(data),
                    "theater" => watching.toggle_theater(data),
                    "stats" => watching.toggle_stats(data),
                    "playNext" => watching.play_next(data),
                    "stay" => watching.stay(),
                    other => panic!("no watch step {other}"),
                }
            }
            other => panic!("no step {other}"),
        }
        Value::Null
    }
}

#[test]
fn steps_are_the_webs() {
    let web = web()["steps"].as_array().expect("steps");
    let start = now();
    let mut site = Site {
        data: Data::new(start),
        watching: None,
        player: PlayerState::default(),
        reported: Vec::new(),
    };
    site.data.advance(start + 900);
    let mut last = site.look(Value::Null);
    same(&Value::Object(last.clone()), &web[0], "steps[0]");
    for (index, step) in inputs()["steps"].as_array().expect("steps").iter().enumerate() {
        let at = step["at"].as_i64().expect("at");
        site.data.advance(start + at);
        let answer = site.act(step);
        site.data.advance(start + at + step["wait"].as_i64().unwrap_or(900));
        let seen = site.look(answer);
        let changed: Map<String, Value> = seen
            .iter()
            .filter(|(key, value)| last.get(*key) != Some(*value))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        same(
            &Value::Object(changed),
            &web[index + 1],
            &format!(
                "steps[{}] ({} {} at {at})",
                index + 1,
                text(&step["op"]),
                step["page"].as_str().or(step["do"].as_str()).unwrap_or_default()
            ),
        );
        last = seen;
    }
}
