//! The data layer held against the web's own: `web/dump.ts` runs the web example's modules
//! (`synth.ts`, `catalog.ts`, `data.ts`, `downloads.ts`, `lyrics.ts`, `player.ts`, `look.ts`,
//! `themes.ts`, `search.tsx`, the radio action in `parts/actions.tsx`, and sokkel-ui-media's
//! `queue.ts` and `lrc.ts`) under bun on the inputs in `web/inputs.json`, the zone UTC, the clock and
//! the timers held and `Math.random` a seeded run, and writes what they give as JSON. Each test
//! builds the same JSON here and asks for it to be equal, every number to the bit.
//!
//! The web's answer is kept in `web/data.json`. Where bun and `.parity/sokkel-ui` are at hand the
//! dump is made again and must match it, so a change on either side shows (`MUSIC_WEB_KEEP=1` keeps
//! the new one); elsewhere (GitHub's CI) the kept answer is read.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use music::catalog::{
    self, StationKind, album_by_id, album_of, albums, albums_of_artist, artist_by_id, artists, bytes_of, format_bytes,
    length_of_songs, minutes, radio_song, song_of, songs, songs_of_album, songs_of_artist, station_from, stations,
    title_from,
};
use music::data::{FavoriteKind, Library, PlayerRow, Settings, Source, SourceKind};
use music::look::{EFFECTS, LOOKS, RAMPS, look_props};
use music::lrc::{ParsedLyrics, lyric_line_at, parse_lyrics};
use music::lyrics::lyrics_of;
use music::player::{Place, PlayFrom, Player};
use music::queue::{Queue, QueueOptions, QueueSeed, RepeatMode, create_queue, seeded_random, shuffled_order};
use music::search::{GroupKind, find, search_groups};
use music::synth::{Seeded, Style, bars_of, length_of, render, wav, waveform_of};
use music::themes::CARRIED;
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
            .join(".parity/sokkel-ui/examples/music/web/node_modules")
            .exists()
            .then(|| {
                Command::new("bun")
                    .arg("examples/music/tests/web/dump.ts")
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
                if std::env::var_os("MUSIC_WEB_KEEP").is_some() {
                    let mut written = Vec::new();
                    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
                    let mut writer = serde_json::Serializer::with_formatter(&mut written, formatter);
                    serde::Serialize::serialize(&fresh, &mut writer).expect("JSON");
                    written.push(b'\n');
                    std::fs::write(&kept_path, written).expect("the web's answer is kept");
                } else {
                    assert_eq!(
                        kept.as_ref(),
                        Some(&fresh),
                        "web/data.json is not what the web gives now; run with MUSIC_WEB_KEEP=1 to keep it"
                    );
                }
                fresh
            }
            (None, Some(kept)) => kept,
            (None, None) => panic!("neither bun nor web/data.json"),
        }
    })
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

/// FNV-1a over 32-bit words, as the dump hashes the samples.
fn hash32(words: impl IntoIterator<Item = u32>) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for word in words {
        hash ^= word;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

#[test]
fn synth_is_the_webs() {
    let web = &web()["sound"];
    let input = inputs();
    let runs: Vec<Value> = input["seeds"]
        .as_array()
        .expect("seeds")
        .iter()
        .map(|seed| {
            let mut next = Seeded::new(seed.as_u64().expect("seed") as u32);
            Value::Array((0..6).map(|_| num(next.next_f64())).collect())
        })
        .collect();
    same(&Value::Array(runs), &web["random"], "random");
    let lengths: Vec<Value> = input["lengthSeeds"]
        .as_array()
        .expect("seeds")
        .iter()
        .map(|seed| {
            let seed = seed.as_u64().expect("seed") as u32;
            Value::Array(
                Style::ALL
                    .iter()
                    .map(|style| {
                        let (bar, bars) = bars_of(seed, *style);
                        json!({ "bar": num(bar), "bars": bars, "length": num(length_of(seed, *style)) })
                    })
                    .collect(),
            )
        })
        .collect();
    same(&Value::Array(lengths), &web["lengths"], "lengths");
    for (at, id) in strings(&input["renders"]).iter().enumerate() {
        let song = song_of(id, &[]).expect("a song");
        let samples = render(song.seed, song.style);
        let made = wav(&samples);
        let picks = [0, 1000, 22050, 100_000, samples.len() - 1];
        let ours = json!({
            "id": id,
            "length": samples.len(),
            "hash": hash32(samples.iter().map(|sample| sample.to_bits())),
            "samples": picks.iter().map(|index| num(f64::from(samples[*index]))).collect::<Vec<_>>(),
            "waveform": waveform_of(&samples, 180).into_iter().map(num).collect::<Vec<_>>(),
            "coarse": waveform_of(&samples, 7).into_iter().map(num).collect::<Vec<_>>(),
            "wavHead": made[..44].to_vec(),
            "wavHash": hash32(made.iter().map(|byte| u32::from(*byte))),
        });
        same(&ours, &web["renders"][at], &format!("renders[{at}]"));
    }
}

#[test]
fn catalog_is_the_webs() {
    let web = &web()["library"];
    let input = inputs();
    let styles: Map<String, Value> = Style::ALL
        .iter()
        .map(|style| (style.key().to_owned(), json!(style.name())))
        .collect();
    same(&Value::Object(styles), &web["styles"], "styles");
    same(&to_json(&artists()), &web["artists"], "artists");
    same(&to_json(&albums()), &web["albums"], "albums");
    same(&to_json(&songs()), &web["songs"], "songs");
    same(&to_json(&stations()), &web["stations"], "stations");
    let titles: Vec<Value> = input["titleSeeds"]
        .as_array()
        .expect("seeds")
        .iter()
        .map(|seed| {
            let mut random = Seeded::new(seed.as_u64().expect("seed") as u32);
            json!([title_from(&mut random), title_from(&mut random)])
        })
        .collect();
    same(&Value::Array(titles), &web["titles"], "titles");
    let made: Vec<Value> = input["stationsFrom"]
        .as_array()
        .expect("stations")
        .iter()
        .map(|pair| {
            let kind: StationKind = serde_json::from_value(pair[0].clone()).expect("a kind");
            station_from(kind, pair[1].as_str().expect("a ref")).map_or(Value::Null, |station| {
                let mut written = to_json(&station);
                written.as_object_mut().expect("a station").remove("id");
                written
            })
        })
        .collect();
    same(&Value::Array(made), &web["stationFrom"], "stationFrom");
    let radio: Vec<Value> = stations()
        .iter()
        .map(|station| Value::Array((0..4).map(|index| to_json(&radio_song(station, index))).collect()))
        .collect();
    same(&Value::Array(radio), &web["radio"], "radio");
    let ids = strings(&input["songIds"]);
    let found: Vec<Value> = ids
        .iter()
        .map(|id| song_of(id, &[]).map_or(Value::Null, |song| to_json(&song)))
        .collect();
    same(&Value::Array(found), &web["songOf"], "songOf");
    let on: Vec<Value> = ids
        .iter()
        .filter_map(|id| song_of(id, &[]))
        .map(|song| to_json(&album_of(&song, &[])))
        .collect();
    same(&Value::Array(on), &web["albumOf"], "albumOf");
    let by_artist: Vec<Value> = artists()
        .iter()
        .map(|artist| {
            json!(
                albums_of_artist(&artist.id)
                    .iter()
                    .map(|album| album.id.clone())
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    same(&Value::Array(by_artist), &web["albumsOfArtist"], "albumsOfArtist");
    let counts: Vec<Value> = artists()
        .iter()
        .map(|artist| json!(songs_of_artist(&artist.id).len()))
        .collect();
    same(&Value::Array(counts), &web["songsOfArtist"], "songsOfArtist");
    let lengths: Vec<Value> = albums()
        .iter()
        .map(|album| num(length_of_songs(songs_of_album(&album.id))))
        .collect();
    same(&Value::Array(lengths), &web["lengthOfAlbums"], "lengthOfAlbums");
    let written: Vec<Value> = input["minutes"]
        .as_array()
        .expect("minutes")
        .iter()
        .map(|seconds| json!(minutes(f(seconds))))
        .collect();
    same(&Value::Array(written), &web["minutes"], "minutes");
    let album_minutes: Vec<Value> = albums()
        .iter()
        .map(|album| json!(minutes(length_of_songs(songs_of_album(&album.id)))))
        .collect();
    same(&Value::Array(album_minutes), &web["albumMinutes"], "albumMinutes");
    let bytes: Vec<Value> = songs()[..12].iter().map(|song| num(bytes_of(song))).collect();
    same(&Value::Array(bytes), &web["bytesOf"], "bytesOf");
    let formatted: Vec<Value> = input["bytes"]
        .as_array()
        .expect("bytes")
        .iter()
        .map(|bytes| json!(format_bytes(f(bytes))))
        .collect();
    same(&Value::Array(formatted), &web["formatBytes"], "formatBytes");
    assert_eq!(artist_by_id("ada").map(|artist| artist.name.as_str()), Some("Ada"));
    assert!(album_by_id("open-water").is_some());
    assert_eq!(catalog::artist_name("nobody"), "nobody");
}

fn parsed_json(parsed: &ParsedLyrics) -> Value {
    let lines: Vec<Value> = parsed
        .lines
        .iter()
        .map(|line| match line.time {
            Some(time) => json!({ "text": line.text, "time": num(time) }),
            None => json!({ "text": line.text }),
        })
        .collect();
    json!({ "lines": lines, "timed": parsed.timed, "tags": parsed.tags })
}

fn lines_at(parsed: &ParsedLyrics) -> Value {
    let positions = inputs()["lyricPositions"].as_array().expect("positions");
    Value::Array(
        positions
            .iter()
            .map(|position| json!(lyric_line_at(&parsed.lines, f(position)).map_or(-1, |at| at as i64)))
            .collect(),
    )
}

#[test]
fn lyrics_are_the_webs() {
    let web = web();
    let mut sung: Vec<catalog::Song> = songs().iter().step_by(3).cloned().collect();
    sung.extend(
        stations()
            .iter()
            .flat_map(|station| (0..4).map(|index| radio_song(station, index))),
    );
    let ours: Vec<Value> = sung
        .iter()
        .map(|song| match lyrics_of(song, &[]) {
            None => json!({ "id": song.id, "text": null }),
            Some(text) => {
                let parsed = parse_lyrics(&text);
                json!({ "id": song.id, "text": text, "parsed": parsed_json(&parsed), "at": lines_at(&parsed) })
            }
        })
        .collect();
    same(&Value::Array(ours), &web["lyrics"], "lyrics");
    let lrc: Vec<Value> = strings(&inputs()["lrc"])
        .iter()
        .map(|text| {
            let parsed = parse_lyrics(text);
            json!({ "parsed": parsed_json(&parsed), "at": lines_at(&parsed) })
        })
        .collect();
    same(&Value::Array(lrc), &web["lrc"], "lrc");
}

fn seed_of(value: &Value) -> Option<QueueSeed> {
    match value {
        Value::String(text) => Some(QueueSeed::Text(text.clone())),
        Value::Number(number) => number.as_f64().map(QueueSeed::Number),
        _ => None,
    }
}

#[test]
fn queues_are_the_webs() {
    let web = web();
    let mut added = 0;
    let scripts = inputs()["queues"].as_array().expect("queues");
    for (at, script) in scripts.iter().enumerate() {
        let count = script["items"].as_u64().expect("items") as usize;
        let items: Vec<String> = (0..count).map(|index| format!("i{index}")).collect();
        let given = &script["options"];
        let options = QueueOptions {
            start: given["start"].as_f64(),
            shuffle: given["shuffle"].as_bool(),
            repeat: given
                .get("repeat")
                .map(|mode| serde_json::from_value(mode.clone()).expect("a mode")),
            seed: seed_of(&given["seed"]),
            order: given["order"]
                .as_array()
                .map(|order| order.iter().map(|index| index.as_i64().expect("an index")).collect()),
        };
        let mut queue: Queue<String> = create_queue(items, options);
        let mut ours = vec![to_json(&queue)];
        for op in script["ops"].as_array().expect("ops") {
            let name = op[0].as_str().expect("an op");
            let next: Option<Queue<String>> = match name {
                "next" => queue.next(),
                "previous" => queue.previous(),
                "ended" => queue.ended(),
                "play" => Some(queue.play(op[1].as_u64().expect("an index") as usize)),
                "shuffle" => Some(queue.shuffled(op[1].as_bool().expect("on"), seed_of(&op[2]).as_ref(), None, None)),
                "repeat" => Some(queue.repeating(serde_json::from_value::<RepeatMode>(op[1].clone()).expect("a mode"))),
                "add" => {
                    let fresh: Vec<String> = (0..op[1].as_u64().expect("a count"))
                        .map(|_| {
                            added += 1;
                            format!("a{added}")
                        })
                        .collect();
                    Some(queue.add(&fresh, op[2] == "next"))
                }
                "remove" => {
                    let indexes: Vec<i64> = op[1]
                        .as_array()
                        .expect("indexes")
                        .iter()
                        .map(|index| index.as_i64().expect("an index"))
                        .collect();
                    Some(queue.remove(&indexes))
                }
                "move" => Some(queue.moved(
                    op[1].as_u64().expect("from") as usize,
                    op[2].as_u64().expect("to") as usize,
                )),
                other => panic!("no op {other}"),
            };
            ours.push(next.as_ref().map_or(Value::Null, to_json));
            if let Some(next) = next {
                queue = next;
            }
        }
        same(&Value::Array(ours), &web["queues"][at], &format!("queues[{at}]"));
    }
    let shuffles: Vec<Value> = inputs()["shuffles"]
        .as_array()
        .expect("shuffles")
        .iter()
        .map(|one| {
            let mut random = seeded_random(&seed_of(&one[2]).expect("a seed"));
            let order = shuffled_order(one[0].as_u64().expect("a count") as usize, f(&one[1]), &mut || {
                random.next_f64()
            });
            json!(order)
        })
        .collect();
    same(&Value::Array(shuffles), &web["shuffles"], "shuffles");
    assert_eq!(RepeatMode::Off.next(), RepeatMode::All);
    assert_eq!(RepeatMode::One.next(), RepeatMode::Off);
}

#[test]
fn search_is_the_webs() {
    let web = web();
    for (at, query) in strings(&inputs()["queries"]).iter().enumerate() {
        let ids = |found: music::search::Found| {
            json!({
                "artists": found.artists.iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
                "albums": found.albums.iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
                "songs": found.songs.iter().map(|one| one.id.clone()).collect::<Vec<_>>(),
            })
        };
        let groups: Vec<Value> = search_groups(query)
            .into_iter()
            .map(|group| {
                let kind = match group.kind {
                    GroupKind::Artists => "artists",
                    GroupKind::Albums => "albums",
                    GroupKind::Songs => "songs",
                };
                json!({
                    "kind": kind,
                    "title": group.title,
                    "total": group.total,
                    "results": group.results.iter().map(|one| json!({
                        "id": one.id, "title": one.title, "detail": one.detail, "round": one.round,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        let ours = json!({ "all": ids(find(query, usize::MAX)), "three": ids(find(query, 3)), "groups": groups });
        same(&ours, &web["search"][at], &format!("search[{at}] ({query:?})"));
    }
}

#[test]
fn looks_are_the_webs() {
    let web = &web()["looks"];
    let looks: Vec<Value> = LOOKS
        .iter()
        .map(|(look, label)| json!({ "id": look.key(), "label": label }))
        .collect();
    same(&Value::Array(looks), &web["looks"], "looks");
    let ramps: Vec<Value> = RAMPS
        .iter()
        .map(|(ramp, label)| json!({ "value": ramp.key(), "label": label }))
        .collect();
    same(&Value::Array(ramps), &web["ramps"], "ramps");
    let effects: Vec<Value> = EFFECTS
        .iter()
        .map(|(effect, label)| json!({ "value": effect.key(), "label": label }))
        .collect();
    same(&Value::Array(effects), &web["effects"], "effects");
    let is_look: Vec<Value> = strings(&inputs()["looks"])
        .iter()
        .map(|id| json!(music::data::Look::from_key(id).is_some()))
        .collect();
    same(&Value::Array(is_look), &web["isLook"], "isLook");
    let mut props = Vec::new();
    for (look, _) in LOOKS {
        for (ramp, _) in RAMPS {
            for (effect, _) in EFFECTS {
                let made = look_props(look, ramp, effect);
                let mut one = Map::new();
                one.insert("kind".to_owned(), json!(made.kind.key()));
                if made.mirror {
                    one.insert("mirror".to_owned(), json!(true));
                }
                one.insert("ramp".to_owned(), json!(made.ramp.key()));
                one.insert("effect".to_owned(), json!(made.effect.key()));
                props.push(Value::Object(one));
            }
        }
    }
    same(&Value::Array(props), &web["props"], "props");
    same(&json!(CARRIED), &web["themes"], "themes");
}

#[test]
fn defaults_are_the_webs() {
    let web = &web()["defaults"];
    same(&to_json(&PlayerRow::default()), &web["player"], "player");
    same(&to_json(&Settings::default()), &web["settings"], "settings");
    same(&to_json(&Library::default().player), &web["restored"], "restored");
    // What is kept reads back as it was written, and a row missing fields takes the defaults.
    let library = Library::default();
    let kept: Library = serde_json::from_str(&serde_json::to_string(&library).expect("JSON")).expect("JSON");
    assert_eq!(kept, library);
    let partial: Settings = serde_json::from_value(json!({ "volume": 0.3 })).expect("settings");
    assert_eq!(partial.volume, 0.3);
    assert!(partial.resume);
}

/// What the dump looks at after each step.
fn look(library: &Library, player: &Player, settings_kept: bool) -> Map<String, Value> {
    let song = player.current_song(library);
    let pairs = [
        ("favorites", to_json(&library.favorites)),
        ("playlists", to_json(&library.playlists)),
        ("downloads", to_json(&library.downloads)),
        ("stations", to_json(&library.stations)),
        ("history", to_json(&library.history)),
        ("settings", to_json(&library.settings)),
        ("queue", to_json(&player.queue)),
        ("source", to_json(&player.source)),
        ("radio", to_json(&player.radio)),
        ("playing", json!(player.playing)),
        ("duration", num(player.duration)),
        ("position", num(player.position)),
        ("song", song.map_or(Value::Null, |song| json!(song.id))),
        ("hasNext", json!(player.has_next())),
        (
            "upNext",
            json!(player.up_next().iter().map(|(_, at)| *at).collect::<Vec<_>>()),
        ),
        ("kept", to_json(&library.player)),
        (
            "keptSettings",
            if settings_kept {
                to_json(&library.settings)
            } else {
                Value::Null
            },
        ),
    ];
    pairs.into_iter().map(|(key, value)| (key.to_owned(), value)).collect()
}

/// The songs a step plays and where they come from.
fn songs_for(step: &Value, library: &Library) -> (Vec<String>, Source) {
    if let Some(album) = step["album"].as_str() {
        let found = album_by_id(album).expect("an album");
        let songs = songs_of_album(album).iter().map(|song| song.id.clone()).collect();
        return (
            songs,
            Source::new(SourceKind::Album, found.title.clone(), Some(format!("/albums/{album}"))),
        );
    }
    if let Some(artist) = step["artist"].as_str() {
        let found = artist_by_id(artist).expect("an artist");
        let songs = songs_of_artist(artist).iter().map(|song| song.id.clone()).collect();
        return (
            songs,
            Source::new(
                SourceKind::Artist,
                found.name.clone(),
                Some(format!("/artists/{artist}")),
            ),
        );
    }
    let id = step["playlist"].as_str().expect("a playlist");
    let playlist = library.playlist(id).expect("a playlist");
    (
        playlist.entries.iter().map(|entry| entry.song.clone()).collect(),
        Source::new(
            SourceKind::Playlist,
            playlist.name.clone(),
            Some(format!("/playlists/{id}")),
        ),
    )
}

fn text(value: &Value) -> &str {
    value.as_str().expect("a text")
}

fn index(value: &Value) -> usize {
    value.as_u64().expect("an index") as usize
}

#[test]
fn steps_are_the_webs() {
    let web = web()["steps"].as_array().expect("steps");
    let input = inputs();
    let start = now();
    let mut library = Library::default();
    let mut player = Player::default();
    let mut random = Seeded::new(input["random"].as_u64().expect("a seed") as u32);
    let mut draw = move || random.next_f64();
    let mut settings_kept = false;
    // As the app starts: the kept rows read after two answers, the player picked up.
    library.advance(start + 300);
    let row = library.player.clone();
    let resume = library.settings.resume;
    player.restore(&mut library, &row, resume, start + 300);
    library.advance(start + 900);
    let mut before = look(&library, &player, settings_kept);
    let check = |ours: &Map<String, Value>, before: &Map<String, Value>, theirs: &Value, what: &str| {
        let theirs = theirs.as_object().expect("a look");
        for (key, value) in ours {
            match theirs.get(key) {
                Some(web) => same(value, web, &format!("{what}.{key}")),
                None => same(value, &before[key], &format!("{what}.{key} (unchanged on the web)")),
            }
        }
    };
    check(&before, &before, &web[0], "steps[0]");
    for (at, step) in input["steps"].as_array().expect("steps").iter().enumerate() {
        let time = start + step["at"].as_i64().expect("a time");
        library.advance(time);
        let op = text(&step["op"]);
        match op {
            "fav" => {
                let kind: FavoriteKind = serde_json::from_value(step["kind"].clone()).expect("a kind");
                library.set_favorite(kind, text(&step["ref"]), step["on"] == true, time);
            }
            "createPlaylist" => {
                library.create_playlist(text(&step["name"]), &strings(&step["songs"]), time);
            }
            "addToPlaylist" => library.add_to_playlist(text(&step["id"]), &strings(&step["songs"]), time),
            "renamePlaylist" => library.rename_playlist(text(&step["id"]), text(&step["name"]), time),
            "describePlaylist" => library.describe_playlist(text(&step["id"]), text(&step["description"]), time),
            "movePlaylist" => {
                library.move_in_playlist(text(&step["id"]), index(&step["from"]), index(&step["to"]), time)
            }
            "removeFromPlaylist" => {
                let id = text(&step["id"]);
                let key = library.playlist(id).expect("a playlist").entries[index(&step["index"])]
                    .key
                    .clone();
                library.remove_from_playlist(id, &key, time);
            }
            "deletePlaylist" => library.delete_playlist(text(&step["id"])),
            "download" => library.download(text(&step["album"]), time),
            "removeDownload" => library.remove_download(text(&step["album"])),
            "removeAllDownloads" => library.remove_all_downloads(),
            "settings" => {
                let mut written = serde_json::to_value(&library.settings).expect("JSON");
                for (key, value) in step["patch"].as_object().expect("a patch") {
                    written[key] = value.clone();
                }
                library.settings = serde_json::from_value(written).expect("settings");
                settings_kept = true;
            }
            "clearHistory" => library.clear_history(),
            "startRadio" => {
                let kind: StationKind = serde_json::from_value(step["kind"].clone()).expect("a kind");
                if let Some((id, name)) = library.start_radio(kind, text(&step["ref"]), time) {
                    player.play_station(&mut library, &id, &name, time);
                }
            }
            "playSongs" => {
                let (songs, source) = songs_for(step, &library);
                let shuffle = step["shuffle"].as_bool();
                let from = PlayFrom {
                    start: index(&step["start"]),
                    shuffle,
                };
                player.play_songs(&mut library, &songs, source, from, time, &mut draw);
            }
            "playStation" => player.play_station(&mut library, text(&step["id"]), text(&step["label"]), time),
            "next" => player.next(&mut library, time),
            "previous" => player.previous(&mut library, time),
            "seek" => player.seek(&mut library, f(&step["to"]), time),
            "seekBy" => player.seek_by(&mut library, f(&step["by"]), time),
            "toggle" => player.toggle_play(&mut library, time),
            "shuffle" => player.set_shuffle(&mut library, step["on"] == true, time, &mut draw),
            "repeat" => {
                let mode: RepeatMode = serde_json::from_value(step["mode"].clone()).expect("a mode");
                player.set_repeat(&mut library, mode, time);
            }
            "enqueue" => {
                let place = if step["where"] == "next" {
                    Place::Next
                } else {
                    Place::End
                };
                player.enqueue(&mut library, &strings(&step["songs"]), place, time);
            }
            "dequeue" => {
                let positions: Vec<usize> = step["positions"]
                    .as_array()
                    .expect("positions")
                    .iter()
                    .map(index)
                    .collect();
                player.dequeue(&mut library, &positions, time);
            }
            "move" => player.move_in_queue(&mut library, index(&step["from"]), index(&step["to"]), time),
            "clearUpNext" => player.clear_up_next(&mut library, time),
            "playIndex" => player.play_index(&mut library, index(&step["index"]), time),
            "playPosition" => player.play_position(&mut library, index(&step["at"]), time),
            "ended" => player.ended(&mut library, time),
            "restore" => {
                let row: PlayerRow = serde_json::from_value(step["row"].clone()).expect("a row");
                player.restore(&mut library, &row, step["resume"] == true, time);
            }
            other => panic!("no op {other}"),
        }
        library.advance(time + 900);
        let ours = look(&library, &player, settings_kept);
        check(&ours, &before, &web[at + 1], &format!("steps[{}] ({op})", at + 1));
        before = ours;
    }
}
