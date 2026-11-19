//! The data layer held against the web's own: `web/dump.ts` runs the web example's modules
//! (`catalog.ts`, `backend.ts`, `script.ts`, `voice.ts`, `themes.ts`, the provider in `state.tsx`,
//! `parts.tsx`'s words, `pages/Discover.tsx`'s search and the `pages/Stats.tsx` page, and
//! sokkel-ui-media's `chapters.ts`, `ChapterList.tsx`, `Transcript.tsx` and `SleepTimer.tsx`) under
//! bun on the inputs in `web/inputs.json`, the zone UTC, the clock and the timers held and
//! `Math.random` a seeded run, and writes what they give as JSON. Each test builds the same JSON
//! here and asks for it to be equal, every number to the bit.
//!
//! The web's answer is kept in `web/data.json`. Where bun and `.parity/sokkel-ui` are at hand the
//! dump is made again and must match it, so a change on either side shows (`PODCAST_WEB_KEEP=1`
//! keeps the new one); elsewhere (GitHub's CI) the kept answer is read.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use podcast::RATES;
use podcast::backend::{PrefsPatch, day, default_prefs, size_of};
use podcast::catalog::{CATEGORIES, Category, HOSTS, category_label, episodes, episodes_of, hosts_of, shows};
use podcast::chapters::{
    chapter_index, chapter_span, chapter_starts, next_chapter_start, previous_chapter_start, seek_point,
};
use podcast::discover::{find, matches};
use podcast::parts::{ago, megabytes, spoken};
use podcast::script::{Script, Seeded, Syllable, length_of, script_of, spoken_script, voice_of};
use podcast::sleep_timer::{SleepTimer, chapter_timer_step, played_on};
use podcast::state::{Podcasts, Where};
use podcast::stats::Stats;
use podcast::themes::CARRIED;
use podcast::transcript::transcript_line_at;
use podcast::voice::{LEVELS, levels_of, render, wav_of};
use serde_json::{Map, Value, json};

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
            .join(".parity/sokkel-ui/examples/podcast/web/node_modules")
            .exists()
            .then(|| {
                Command::new("bun")
                    .arg("examples/podcast/tests/web/dump.ts")
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
                if std::env::var_os("PODCAST_WEB_KEEP").is_some() {
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
                        _ => panic!(
                            "web/data.json is not what the web gives now; run with PODCAST_WEB_KEEP=1 to keep it"
                        ),
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

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| items.iter().filter_map(|one| one.as_str().map(str::to_owned)).collect())
        .unwrap_or_default()
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

fn host(name: &str) -> podcast::catalog::Host {
    HOSTS.into_iter().find(|one| one.name() == name).expect("a host")
}

#[test]
fn catalog_is_the_webs() {
    let web = &web()["library"];
    same(&to_json(&HOSTS), &web["hosts"], "hosts");
    let categories: Vec<Value> = CATEGORIES
        .iter()
        .map(|(category, label)| json!({ "id": category.id(), "label": label }))
        .collect();
    same(&Value::Array(categories), &web["categories"], "categories");
    same(&to_json(&shows()), &web["shows"], "shows");
    same(&to_json(&episodes()), &web["episodes"], "episodes");
    let of: Vec<Value> = shows()
        .iter()
        .map(|show| {
            json!(
                episodes_of(&show.id)
                    .iter()
                    .map(|one| one.id.clone())
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    same(&Value::Array(of), &web["episodesOf"], "episodesOf");
    let mut ids: Vec<String> = CATEGORIES.iter().map(|(one, _)| one.id().to_owned()).collect();
    ids.push("elsewhere".to_owned());
    let labels: Vec<Value> = ids
        .iter()
        .map(|id| json!(Category::from_id(id).map_or(id.clone(), |one| category_label(one).to_owned())))
        .collect();
    same(&Value::Array(labels), &web["categoryLabels"], "categoryLabels");
    let hosts: Vec<Value> = shows().iter().map(|show| json!(hosts_of(show))).collect();
    same(&Value::Array(hosts), &web["hostsOf"], "hostsOf");
    same(&json!(CARRIED), &web["themes"], "themes");
    same(
        &Value::Array(RATES.iter().map(|rate| num(*rate)).collect()),
        &web["rates"],
        "rates",
    );
}

#[test]
fn seeds_are_the_webs() {
    let web = &web()["seeds"];
    let input = inputs();
    same(&to_json(&default_prefs()), &web["prefs"], "prefs");
    let days: Vec<Value> = input["dayOffsets"]
        .as_array()
        .expect("offsets")
        .iter()
        .map(|offset| json!(day(offset.as_i64().expect("offset"), now())))
        .collect();
    same(&Value::Array(days), &web["days"], "days");
    let from: Vec<Value> = input["dayFroms"]
        .as_array()
        .expect("pairs")
        .iter()
        .map(|pair| json!(day(pair[0].as_i64().expect("offset"), pair[1].as_i64().expect("from"))))
        .collect();
    same(&Value::Array(from), &web["daysFrom"], "daysFrom");
    let mut ids: Vec<String> = episodes().iter().map(|one| one.id.clone()).collect();
    ids.push("nowhere-1".to_owned());
    let sizes: Vec<Value> = ids.iter().map(|id| num(size_of(id))).collect();
    same(&Value::Array(sizes), &web["sizes"], "sizes");
}

#[test]
fn words_are_the_webs() {
    let web = &web()["words"];
    let input = inputs();
    let agos: Vec<Value> = input["ago"]
        .as_array()
        .expect("ago")
        .iter()
        .map(|days| json!(ago(now() - days.as_i64().expect("ms"), now())))
        .collect();
    same(&Value::Array(agos), &web["ago"], "ago");
    let said: Vec<Value> = floats(&input["spoken"])
        .into_iter()
        .map(|one| json!(spoken(one)))
        .collect();
    same(&Value::Array(said), &web["spoken"], "spoken");
    let sizes: Vec<Value> = floats(&input["megabytes"])
        .into_iter()
        .map(|one| json!(megabytes(one)))
        .collect();
    same(&Value::Array(sizes), &web["megabytes"], "megabytes");
}

fn syllable_digest(syllables: &[Syllable]) -> Value {
    let numbers = syllables.iter().flat_map(|one| {
        [one.at, one.length, one.pitch, one.pitch_end, one.gain]
            .into_iter()
            .flat_map(|value| {
                let bits = value.to_bits();
                [bits as u32, (bits >> 32) as u32]
            })
    });
    let words: Vec<String> = syllables
        .iter()
        .map(|one| {
            format!(
                "{}{} {}{}",
                text(&to_json(&one.vowel)),
                text(&to_json(&one.onset)),
                u8::from(one.last),
                one.voice.name()
            )
        })
        .collect();
    json!({
        "count": syllables.len(),
        "numbers": hash32(numbers),
        "words": hash32(words.join(",").bytes().map(u32::from)),
    })
}

#[test]
fn scripts_are_the_webs() {
    let web = &web()["scripts"];
    let input = inputs();
    let runs: Vec<Value> = strings(&input["randomSeeds"])
        .iter()
        .map(|seed| {
            let mut next = Seeded::new(seed);
            Value::Array((0..6).map(|_| num(next.next_f64())).collect())
        })
        .collect();
    same(&Value::Array(runs), &web["random"], "random");
    let voices: Map<String, Value> = HOSTS
        .iter()
        .map(|one| (one.name().to_owned(), to_json(&voice_of(*one))))
        .collect();
    same(&Value::Object(voices), &web["voices"], "voices");
    for (at, episode) in episodes().iter().enumerate() {
        let made = script_of(&episode.id).expect("a script");
        let ours = json!({
            "id": episode.id,
            "duration": num(made.duration),
            "length": num(length_of(&episode.id)),
            "chapters": to_json(&made.chapters),
            "lines": to_json(&made.lines),
            "cues": to_json(&made.cues),
            "syllables": syllable_digest(&made.syllables),
        });
        same(&ours, &web["episodes"][at], &format!("episodes[{at}]"));
    }
    for (at, id) in strings(&input["fullSyllables"]).iter().enumerate() {
        let made = script_of(id).expect("a script");
        let first: Vec<Syllable> = made.syllables.iter().take(60).cloned().collect();
        same(&to_json(&first), &web["syllables"][at], &format!("syllables[{at}]"));
    }
    for (at, case) in input["spokenScripts"].as_array().expect("cases").iter().enumerate() {
        let made = spoken_script(text(&case[0]), host(text(&case[1])), text(&case[2]));
        let ours = json!({
            "duration": num(made.duration),
            "lines": to_json(&made.lines),
            "syllables": to_json(&made.syllables),
        });
        same(&ours, &web["spoken"][at], &format!("spoken[{at}]"));
    }
    assert!(script_of("nowhere-1").is_none());
}

#[test]
fn sound_is_the_webs() {
    let web = &web()["renders"];
    for (at, case) in inputs()["renders"].as_array().expect("renders").iter().enumerate() {
        let made: Script = if text(&case[0]) == "episode" {
            (*script_of(text(&case[1])).expect("a script")).clone()
        } else {
            spoken_script(text(&case[1]), host(text(&case[2])), text(&case[3]))
        };
        let samples = render(&made);
        let wav = wav_of(&samples);
        let picks = [0, 1000, 40_000, 100_000, samples.len() - 1];
        let ours = json!({
            "length": samples.len(),
            "hash": hash32(samples.iter().map(|sample| sample.to_bits())),
            "samples": picks
                .iter()
                .map(|index| samples.get(*index).map_or(Value::Null, |sample| num(f64::from(*sample))))
                .collect::<Vec<_>>(),
            "levels": levels_of(&samples, LEVELS).into_iter().map(num).collect::<Vec<_>>(),
            "coarse": levels_of(&samples, 7).into_iter().map(num).collect::<Vec<_>>(),
            "wavHead": wav[..44].to_vec(),
            "wavHash": hash32(wav.iter().map(|byte| u32::from(*byte))),
        });
        same(&ours, &web[at], &format!("renders[{at}]"));
    }
}

#[test]
fn media_parts_are_the_webs() {
    let web = web();
    let input = inputs();
    for (at, case) in input["chapterCases"].as_array().expect("cases").iter().enumerate() {
        let made = script_of(text(&case[0])).expect("a script");
        let starts = chapter_starts(&made.chapters);
        let positions = floats(&case[1]);
        let ours = json!({
            "starts": starts.iter().map(|one| num(*one)).collect::<Vec<_>>(),
            "index": positions.iter().map(|one| chapter_index(&starts, *one)).collect::<Vec<_>>(),
            "previous": positions.iter().map(|one| num(previous_chapter_start(&starts, *one))).collect::<Vec<_>>(),
            "next": positions
                .iter()
                .map(|one| next_chapter_start(&starts, *one).map_or(Value::Null, num))
                .collect::<Vec<_>>(),
            "spans": (0..starts.len())
                .map(|index| {
                    let (start, end) = chapter_span(&starts, index, made.duration);
                    json!({ "start": num(start), "end": num(end) })
                })
                .collect::<Vec<_>>(),
            "line": positions.iter().map(|one| transcript_line_at(&made.lines, *one)).collect::<Vec<_>>(),
            "seek": positions
                .iter()
                .map(|one| seek_point(*one, made.duration).map_or(Value::Null, num))
                .collect::<Vec<_>>(),
        });
        same(&ours, &web["media"][at], &format!("media[{at}]"));
    }
    let steps: Vec<Value> = input["timerSteps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|step| {
            let (ended, anchor) = chapter_timer_step(
                &[0.0, 10.0, 20.0, 30.0],
                step[0].as_i64().expect("anchor"),
                step[1].as_f64(),
                f(&step[2]),
                f(&step[3]),
                step[4].as_bool().expect("playing"),
            );
            json!({ "ended": ended, "anchor": anchor })
        })
        .collect();
    same(&Value::Array(steps), &web["timerSteps"], "timerSteps");
    let played: Vec<Value> = input["playedOn"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|case| json!(played_on(f(&case[0]), f(&case[1]), f(&case[2]))))
        .collect();
    same(&Value::Array(played), &web["played"], "played");
}

#[test]
fn search_is_the_webs() {
    let web = web();
    let input = inputs();
    let podcasts = Podcasts::new(now());
    let found: Vec<Value> = strings(&input["queries"])
        .iter()
        .map(|query| {
            let (shows, episodes) = find(query, |id| podcasts.episode(id).is_some_and(|row| row.out));
            json!({
                "shows": shows.iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
                "episodes": episodes.iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
            })
        })
        .collect();
    same(&Value::Array(found), &web["found"], "found");
    let matched: Vec<Value> = input["matches"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|case| {
            let texts = strings(&case[1]);
            let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
            json!(matches(text(&case[0]), &texts))
        })
        .collect();
    same(&Value::Array(matched), &web["matched"], "matched");
}

/// The Stats page as the dump reads it from what it draws.
fn stats_look(podcasts: &Podcasts) -> Value {
    let stats = Stats::of(&podcasts.listens, podcasts.now);
    if stats.total == 0 {
        return json!({ "tiles": [], "bars": [], "nothing": true });
    }
    let tiles: Vec<Value> = stats
        .tiles()
        .iter()
        .map(|(label, value, note)| {
            let value = if *label == "Finished" {
                json!(stats.finished)
            } else {
                json!(value)
            };
            json!({ "label": label, "value": value, "note": note })
        })
        .collect();
    let bars = |label: &str, bars: Vec<podcast::stats::Bar>| {
        json!({
            "label": label,
            "items": bars
                .iter()
                .map(|bar| json!({ "id": bar.id, "label": bar.label, "value": num(bar.value), "chosen": bar.chosen }))
                .collect::<Vec<_>>(),
            "written": bars.iter().map(|bar| bar.written.clone()).collect::<Vec<_>>(),
        })
    };
    json!({
        "tiles": tiles,
        "bars": [
            bars("Time listened by show", stats.show_bars()),
            bars("Time listened by day", stats.day_bars(podcasts.now)),
        ],
        "nothing": false,
    })
}

fn look(podcasts: &mut Podcasts, answers: &mut Vec<Value>) -> Map<String, Value> {
    answers.extend(podcasts.checked.drain(..).map(|count| json!(count)));
    let element = &podcasts.element;
    let seen = json!({
        "shows": to_json(&podcasts.shows),
        "episodes": to_json(&podcasts.episodes),
        "player": to_json(&[&podcasts.player]),
        "prefs": to_json(&[&podcasts.prefs]),
        "listens": to_json(&podcasts.listens),
        "current": podcasts.current(),
        "playing": podcasts.playing,
        "position": num(podcasts.position()),
        "media": podcasts.media(),
        "waveform": if podcasts.media() { json!(LEVELS) } else { Value::Null },
        "sleep": {
            "timer": to_json(&podcasts.sleep_timer()),
            "remaining": podcasts.sleep_remaining().map_or(Value::Null, num),
        },
        "element": {
            "paused": element.paused,
            "time": num(element.time),
            "rate": num(element.rate),
            "volume": num(element.volume),
            "muted": element.muted,
        },
        "stats": stats_look(podcasts),
        "toasts": to_json(&std::mem::take(&mut podcasts.toasts)),
        "answers": Value::Array(std::mem::take(answers)),
    });
    match seen {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}

fn sleep_timer(value: &Value) -> Option<SleepTimer> {
    match value {
        Value::Number(minutes) => Some(SleepTimer::Minutes(minutes.as_f64().expect("minutes"))),
        Value::String(kind) if kind == "chapter" => Some(SleepTimer::Chapter),
        Value::String(kind) if kind == "end" => Some(SleepTimer::End),
        _ => None,
    }
}

#[test]
fn steps_are_the_webs() {
    let web = web()["steps"].as_array().expect("steps");
    let start = now();
    let mut podcasts = Podcasts::new(start);
    let mut answers = Vec::new();
    podcasts.advance(start + 900);
    let mut last = look(&mut podcasts, &mut answers);
    same(&Value::Object(last.clone()), &web[0], "steps[0]");
    for (index, step) in inputs()["steps"].as_array().expect("steps").iter().enumerate() {
        let at = step["at"].as_i64().expect("at");
        podcasts.advance(start + at);
        let id = || text(&step["id"]);
        match text(&step["op"]) {
            "play" => podcasts.play(id(), step["from"].as_f64()),
            "setPlaying" => podcasts.set_playing(step["on"].as_bool().expect("on")),
            "seek" => podcasts.seek(f(&step["to"])),
            "skip" => podcasts.skip(f(&step["by"])),
            "rate" => podcasts.set_rate(f(&step["rate"])),
            "volume" => podcasts.set_volume(f(&step["volume"])),
            "muted" => podcasts.set_muted(step["muted"].as_bool().expect("muted")),
            "playNext" => answers.push(json!(podcasts.play_next())),
            "queueAdd" => podcasts.queue_add(
                id(),
                if text(&step["where"]) == "next" {
                    Where::Next
                } else {
                    Where::End
                },
            ),
            "queueRemove" => podcasts.queue_remove(id()),
            "queueMove" => podcasts.queue_move(step["from"].as_i64().expect("from"), step["to"].as_i64().expect("to")),
            "queueClear" => podcasts.queue_clear(),
            "markPlayed" => podcasts.mark_played(id(), step["played"].as_bool().expect("played")),
            "star" => podcasts.star(id(), step["starred"].as_bool().expect("starred")),
            "archive" => podcasts.archive(id()),
            "archiveAll" => podcasts.archive_all(),
            "download" => podcasts.download(id()),
            "removeDownload" => podcasts.remove_download(id()),
            "removeAllDownloads" => podcasts.remove_all_downloads(),
            "checkForNew" => podcasts.check_for_new(),
            "subscribe" => podcasts.subscribe(id()),
            "unsubscribe" => podcasts.unsubscribe(id()),
            "prefs" => {
                let patch: PrefsPatch = serde_json::from_value(step["patch"].clone()).expect("a patch");
                podcasts.set_prefs(&patch);
            }
            "clearStats" => podcasts.clear_stats(),
            "sleep" => podcasts.set_sleep(sleep_timer(&step["timer"])),
            "played" => podcasts.played_to(f(&step["to"])),
            "ended" => podcasts.ended(),
            "away" => podcasts.away(),
            other => panic!("no step {other}"),
        }
        podcasts.advance(start + at + step["wait"].as_i64().unwrap_or(900));
        let seen = look(&mut podcasts, &mut answers);
        let changed: Map<String, Value> = seen
            .iter()
            .filter(|(key, value)| last.get(*key) != Some(*value))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        same(
            &Value::Object(changed),
            &web[index + 1],
            &format!("steps[{}] ({} at {at})", index + 1, text(&step["op"])),
        );
        last = seen;
    }
}
