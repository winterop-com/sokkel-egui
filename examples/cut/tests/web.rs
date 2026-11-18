//! The data layer held against the web's own: `web/dump.ts` runs the web example's modules
//! (`model.ts`, `seed.ts`, `media.ts`, `sound.ts`, `render.ts`, `export.ts`, `collections.ts`,
//! `actions.ts`, the hooks `history.ts` and `editor/use-playback.ts`, and the shared `scenes.ts` and
//! `sound.ts` they read) under bun on the inputs in `web/inputs.json`, the zone UTC and the clock,
//! the frame clock and the timers held, and writes what they give as JSON. Each test builds the same
//! JSON here and asks for it to be equal, every number to the bit.
//!
//! The web's answer is kept in `web/data.json`. Where bun and `.parity/sokkel-ui` are at hand the
//! dump is made again and must match it, so a change on either side shows (`CUT_WEB_KEEP=1` keeps
//! the new one); elsewhere (GitHub's CI) the kept answer is read.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use cut::backend::{Backend, Notice, NoticeKind, by_updated};
use cut::editor::{media_drag, parse_time};
use cut::export::{ExportJob, file_name};
use cut::history::EditHistory;
use cut::media::{SOURCES, SourceKind, clip_peaks, peaks_of, source_of, spec_of};
use cut::model::{
    ClipChange, ClipIds, ClipPatch, Edit, ExportSize, Prefs, TrackChange, add_source, add_title, apply_changes,
    clip_at, copy_edit, default_tracks, delete_clips, drop_source, free_start, length_of, length_text, name_of,
    on_frame, patch_clip, placed_length, set_track, split_all, split_clip, timecode,
};
use cut::playback::{Cue, Playback};
use cut::render::{picture_at, titles_at};
use cut::seed::seed_projects;
use cut::sound::{LEAD, mixdown, scheduled};
use serde_json::{Map, Value, json};
use tidevann_shared::clips::{Seeded, synthesize};

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
            .join(".parity/sokkel-ui/examples/cut/web/node_modules")
            .exists()
            .then(|| {
                Command::new("bun")
                    .arg("examples/cut/tests/web/dump.ts")
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
                if std::env::var_os("CUT_WEB_KEEP").is_some() {
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
                        "web/data.json is not what the web gives now; run with CUT_WEB_KEEP=1 to keep it"
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

#[test]
fn seed_bin_and_settings() {
    let web = web();
    let projects: Vec<Value> = seed_projects(now()).iter().map(to_json).collect();
    same(&Value::Array(projects), &web["projects"], "projects");
    let sources: Vec<Value> = SOURCES
        .iter()
        .map(|source| {
            json!({
                "id": source.id,
                "name": source.name,
                "kind": source.kind.name(),
                "scene": source.scene.name(),
                "hue": num(source.hue),
                "accent": num(source.accent),
                "seed": source.seed,
                "duration": num(source.duration),
                "place": source.place,
                "color": source.color.name(),
            })
        })
        .collect();
    same(&Value::Array(sources), &web["sources"], "sources");
    let mut prefs = to_json(&Prefs::default());
    prefs.as_object_mut().expect("prefs").remove("id");
    same(&prefs, &web["prefs"], "prefs");
    let sizes: Map<String, Value> = ExportSize::ALL
        .iter()
        .map(|size| {
            let spec = size.spec();
            (
                size.key().to_owned(),
                json!({ "label": spec.label, "width": spec.width, "height": spec.height, "bitrate": spec.bitrate }),
            )
        })
        .collect();
    same(&Value::Object(sizes), &web["exportSizes"], "exportSizes");
    same(&to_json(&default_tracks()), &web["tracks"], "tracks");
}

#[test]
fn words_and_lookups() {
    let (web, inputs) = (&web()["words"], inputs());
    let times: Vec<f64> = inputs["times"].as_array().expect("times").iter().map(f).collect();
    let moments: Vec<f64> = inputs["moments"].as_array().expect("moments").iter().map(f).collect();
    let projects = seed_projects(now());
    let random: Vec<Vec<Value>> = [1u32, 11, 348, 4_294_967_295]
        .iter()
        .map(|seed| {
            let mut next = Seeded::new(*seed);
            (0..6).map(|_| num(next.next())).collect()
        })
        .collect();
    let ours = json!({
        "timecodes": times.iter().map(|t| timecode(*t)).collect::<Vec<_>>(),
        "lengths": times.iter().map(|t| length_text(*t)).collect::<Vec<_>>(),
        "onFrame": times.iter().map(|t| num(on_frame(*t))).collect::<Vec<_>>(),
        "parsed": strings(&inputs["typed"]).iter().map(|text| parse_time(text).map_or(Value::Null, num)).collect::<Vec<_>>(),
        "fileNames": strings(&inputs["titles"]).iter().map(|title| file_name(title, "webm", "edit")).collect::<Vec<_>>(),
        "names": projects.iter().flat_map(|project| project.edit.clips.iter().map(name_of)).collect::<Vec<_>>(),
        "lengthOf": projects.iter().map(|project| num(length_of(&project.edit))).collect::<Vec<_>>(),
        "placed": SOURCES.iter().map(|source| num(placed_length(source))).collect::<Vec<_>>(),
        "drags": SOURCES.iter().map(|source| {
            let drag = media_drag(source);
            json!({
                "kind": "media",
                "label": drag.label,
                "value": { "kind": drag.kind.name(), "duration": num(drag.duration), "source": drag.source },
            })
        }).collect::<Vec<_>>(),
        "specs": SOURCES.iter().map(|source| {
            let spec = spec_of(source);
            json!({
                "kind": spec.kind.name(),
                "hue": num(spec.hue),
                "accent": num(spec.accent),
                "seed": spec.seed,
                "title": spec.title,
                "channel": spec.channel,
                "duration": num(spec.duration),
            })
        }).collect::<Vec<_>>(),
        "random": random,
        "clipAt": projects.iter().map(|project| {
            project.edit.tracks.iter().map(|track| {
                moments.iter().map(|t| clip_at(&project.edit, &track.id, *t).map_or(Value::Null, |clip| json!(clip.id))).collect::<Vec<_>>()
            }).collect::<Vec<_>>()
        }).collect::<Vec<_>>(),
        "pictures": projects.iter().map(|project| {
            moments.iter().map(|t| picture_at(&project.edit, *t).map_or(Value::Null, |clip| json!(clip.id))).collect::<Vec<_>>()
        }).collect::<Vec<_>>(),
        "titles": projects.iter().map(|project| {
            moments.iter().map(|t| titles_at(&project.edit, *t).iter().map(|clip| clip.id.clone()).collect::<Vec<_>>()).collect::<Vec<_>>()
        }).collect::<Vec<_>>(),
        "freeStart": inputs["freeStarts"].as_array().expect("freeStarts").iter().map(|one| {
            num(free_start(&projects[0].edit, one[0].as_str().expect("track"), f(&one[1]), f(&one[2]), one.get(3).and_then(Value::as_str)))
        }).collect::<Vec<_>>(),
    });
    for key in ours.as_object().expect("words").keys() {
        same(&ours[key], &web[key], key);
    }
}

fn patch_from(value: &Value) -> ClipPatch {
    ClipPatch {
        track: value.get("track").and_then(Value::as_str).map(str::to_owned),
        start: value.get("start").and_then(Value::as_f64),
        duration: value.get("duration").and_then(Value::as_f64),
        in_point: value.get("inPoint").and_then(Value::as_f64),
        name: value.get("name").and_then(Value::as_str).map(str::to_owned),
        text: value.get("text").and_then(Value::as_str).map(str::to_owned),
        volume: value.get("volume").and_then(Value::as_f64),
    }
}

fn track_change(value: &Value) -> TrackChange {
    TrackChange {
        muted: value.get("muted").and_then(Value::as_bool),
        locked: value.get("locked").and_then(Value::as_bool),
        hidden: value.get("hidden").and_then(Value::as_bool),
    }
}

/// The edits of `inputs.json` made through undo and redo, as the editor makes them, with what each
/// step leaves; `ids` goes on to make the ids the backend's duplicate makes after them.
fn replay_edits(ids: &mut ClipIds) -> Vec<Value> {
    let projects = seed_projects(now());
    let mut runs = Vec::new();
    for run in inputs()["edits"].as_array().expect("edits") {
        let project = projects
            .iter()
            .find(|one| Some(one.id.as_str()) == run["project"].as_str())
            .expect("project");
        let mut history = EditHistory::new(project.edit.clone());
        let mut out = Vec::new();
        for step in run["steps"].as_array().expect("steps") {
            let at = now() + step["at"].as_i64().expect("at");
            ids.now = at;
            let mut made: Option<String> = None;
            let edit = history.edit().clone();
            let source = || source_of(step["source"].as_str()).expect("source");
            let t = || f(&step["t"]);
            match step["op"].as_str().expect("op") {
                "patch" => {
                    let id = step["id"].as_str().expect("id");
                    let key = format!("{id}:{}", step["field"].as_str().expect("field"));
                    history.apply(Some(patch_clip(&edit, id, &patch_from(&step["patch"]))), Some(&key), at);
                }
                "split" => {
                    let only = strings(&step["only"]);
                    history.apply(split_all(&edit, t(), &only, ids), None, at);
                }
                "splitClip" => {
                    let done = split_clip(&edit, step["id"].as_str().expect("id"), t(), ids);
                    let key = format!("split:{}", cut::js::number_to_string(t()));
                    history.apply(
                        done.map(|(next, id)| {
                            made = Some(id);
                            next
                        }),
                        Some(&key),
                        at,
                    );
                }
                "delete" => {
                    let next = delete_clips(&edit, &strings(&step["ids"]), step["ripple"] == json!(true));
                    history.apply(Some(next), None, at);
                }
                "changes" => {
                    let changes: Vec<ClipChange> = step["changes"]
                        .as_array()
                        .expect("changes")
                        .iter()
                        .map(|one| ClipChange {
                            id: one["id"].as_str().expect("id").to_owned(),
                            start: one.get("start").and_then(Value::as_f64),
                            duration: one.get("duration").and_then(Value::as_f64),
                            in_point: one.get("inPoint").and_then(Value::as_f64),
                            track: one.get("track").and_then(Value::as_str).map(str::to_owned),
                        })
                        .collect();
                    history.apply(apply_changes(&edit, &changes), None, at);
                }
                "addSource" | "dropSource" | "addTitle" => {
                    let done = match step["op"].as_str() {
                        Some("addSource") => add_source(&edit, source(), t(), ids),
                        Some("dropSource") => {
                            drop_source(&edit, source(), step["track"].as_str().expect("track"), t(), ids)
                        }
                        _ => add_title(
                            &edit,
                            t(),
                            step["text"].as_str().expect("text"),
                            f(&step["duration"]),
                            ids,
                        ),
                    };
                    if let Some((next, id)) = done {
                        made = Some(id);
                        history.apply(Some(next), None, at);
                    }
                }
                "track" => {
                    let next = set_track(&edit, step["id"].as_str().expect("id"), &track_change(&step["change"]));
                    history.apply(Some(next), None, at);
                }
                "undo" => history.undo(),
                "redo" => history.redo(),
                _ => {}
            }
            let shown = if step["op"] == "copy" {
                copy_edit(history.edit(), ids)
            } else {
                history.edit().clone()
            };
            out.push(json!({
                "edit": to_json(&shown),
                "canUndo": history.can_undo(),
                "canRedo": history.can_redo(),
                "made": made,
                "length": num(length_of(history.edit())),
            }));
        }
        runs.push(Value::Array(out));
    }
    runs
}

#[test]
fn edits_with_undo_and_redo() {
    let ours = replay_edits(&mut ClipIds::new(now()));
    let web = web()["edits"].as_array().expect("edits");
    assert_eq!(ours.len(), web.len());
    for (at, (ours, web)) in ours.iter().zip(web).enumerate() {
        same(ours, web, &format!("edits[{at}]"));
    }
}

#[test]
fn playback_and_its_sound() {
    let projects = seed_projects(now());
    let web = web()["playback"].as_array().expect("playback");
    for (run, web) in inputs()["playback"].as_array().expect("playback").iter().zip(web) {
        let project = projects
            .iter()
            .find(|one| Some(one.id.as_str()) == run["project"].as_str())
            .expect("project");
        let mut edit = project.edit.clone();
        let mut looping = false;
        let mut playback = Playback::new(length_of(&edit), looping);
        let mut looks = Vec::new();
        let mut sounds = Vec::new();
        for step in run["steps"].as_array().expect("steps") {
            let at = f(&step["at"]);
            match step["op"].as_str().expect("op") {
                "frame" => playback.frame(at),
                "toggle" => playback.toggle(at),
                "pause" => playback.pause(at),
                "forward" => playback.forward(at),
                "backward" => playback.backward(at),
                "step" => playback.step(f(&step["frames"]), at),
                "seek" => playback.seek(f(&step["t"]), at),
                "loop" => {
                    looping = step["loop"] == json!(true);
                    playback.update(at, length_of(&edit), looping, false);
                }
                "delete" => {
                    edit = delete_clips(&edit, &strings(&step["ids"]), false);
                    playback.update(at, length_of(&edit), looping, true);
                }
                "mute" => {
                    let change = TrackChange {
                        muted: step["muted"].as_bool(),
                        ..TrackChange::default()
                    };
                    edit = set_track(&edit, step["track"].as_str().expect("track"), &change);
                    playback.update(at, length_of(&edit), looping, true);
                }
                other => panic!("no step {other}"),
            }
            for cue in playback.take_cues() {
                if let Cue::Play { from } = cue {
                    let current = at / 1000.0;
                    let starts: Vec<Value> = scheduled(&edit, from)
                        .iter()
                        .map(|one| {
                            json!({
                                "delay": num(((current + LEAD) + one.after) - current),
                                "offset": num(one.offset),
                                "duration": num(one.duration),
                                "gain": num(one.gain),
                            })
                        })
                        .collect();
                    sounds.push(json!({ "at": num(at), "starts": starts }));
                }
            }
            looks.push(json!({
                "playhead": num(playback.playhead),
                "rate": num(playback.rate),
                "playing": playback.playing(),
                "waiting": usize::from(playback.playing()),
            }));
        }
        same(
            &json!({ "looks": looks, "sounds": sounds }),
            web,
            &format!("playback {}", project.id),
        );
    }
}

fn every(list: &[f32]) -> Vec<Value> {
    let stride = inputs()["stride"].as_u64().expect("stride") as usize;
    list.iter().step_by(stride).map(|one| num(f64::from(*one))).collect()
}

#[test]
fn sound_samples_and_peaks() {
    let web = web()["sound"].as_array().expect("sound");
    for (source, web) in SOURCES.iter().zip(web) {
        let (left, right) = synthesize(&spec_of(source));
        let ours = json!({
            "id": source.id,
            "frames": left.len(),
            "left": every(&left),
            "right": every(&right),
            "peaks": (source.kind == SourceKind::Audio).then(|| peaks_of(source).iter().map(|peak| num(*peak)).collect::<Vec<_>>()),
        });
        same(&ours, web, &format!("sound {}", source.id));
    }
}

#[test]
fn clip_peaks_and_mixdown() {
    let projects = seed_projects(now());
    let peaks: Vec<Value> = projects
        .iter()
        .flat_map(|project| project.edit.clips.iter())
        .filter_map(|clip| {
            let source = clip.source().filter(|source| source.kind == SourceKind::Audio)?;
            Some(json!(
                clip_peaks(source, clip.in_point, clip.duration)
                    .into_iter()
                    .map(num)
                    .collect::<Vec<_>>()
            ))
        })
        .collect();
    same(&Value::Array(peaks), &web()["clipPeaks"], "clipPeaks");
    let mixes: Vec<Value> = projects
        .iter()
        .map(|project| {
            let (left, right) = mixdown(&project.edit, length_of(&project.edit));
            json!({ "frames": left.len(), "left": every(&left), "right": every(&right) })
        })
        .collect();
    same(&Value::Array(mixes), &web()["mixes"], "mixes");
}

#[test]
fn export_frames_and_progress() {
    let mut ours = Vec::new();
    for project in seed_projects(now()) {
        for size in ExportSize::ALL {
            let mut job = ExportJob::new(&project.edit, size, false);
            let mut calls = Vec::new();
            while let Some(frame) = job.next_frame() {
                if let Some((share, count)) = frame.progress {
                    calls.push(json!([num(share), count]));
                }
            }
            let (share, count) = job.painted();
            calls.push(json!([num(share), count]));
            ours.push(json!({
                "project": project.id,
                "size": size.key(),
                "calls": calls,
                "frames": job.total,
                "file": null,
            }));
        }
    }
    same(&Value::Array(ours), &web()["exports"], "exports");
}

fn notice(notice: &Notice) -> Value {
    json!({
        "type": match notice.kind {
            NoticeKind::Plain => Value::Null,
            NoticeKind::Success => json!("success"),
            NoticeKind::Warning => json!("warning"),
            NoticeKind::Error => json!("error"),
        },
        "title": notice.title,
        "description": notice.description,
        "action": notice.action,
    })
}

#[test]
fn backend_actions() {
    // The ids the edits made come first, as the web's counter runs over the whole visit.
    let mut ids = ClipIds::new(now());
    replay_edits(&mut ids);
    let mut backend = Backend::seeded(now());
    let (mut kept_projects, mut kept_prefs) = (false, false);
    let mut looks = Vec::new();
    for step in inputs()["backend"].as_array().expect("backend") {
        // Each action is made once the projects are listed, a store's answer after the step.
        let at = now() + step["at"].as_i64().expect("at") + 150;
        ids.now = at;
        let named = |backend: &Backend, name: &str| {
            backend
                .projects
                .iter()
                .find(|one| one.name == name)
                .map(|one| one.id.clone())
        };
        let name = step["name"].as_str().unwrap_or_default();
        let mut toasts = Vec::new();
        let mut went = Vec::new();
        match step["do"].as_str().expect("do") {
            "create" => {
                let (id, said) = backend.create_project(name, at);
                toasts.push(notice(&said));
                went.push(format!("/p/{id}"));
                kept_projects = true;
            }
            "rename" => {
                let id = named(&backend, name).unwrap_or_else(|| "gone".to_owned());
                match backend.rename_project(&id, step["to"].as_str().expect("to"), at) {
                    Ok(()) => kept_projects = true,
                    Err(said) => toasts.push(notice(&said)),
                }
            }
            "duplicate" => {
                let id = named(&backend, name).expect("project");
                let (_, said) = backend.duplicate_project(&id, at, &mut ids).expect("duplicated");
                toasts.push(notice(&said));
                kept_projects = true;
            }
            "delete" => {
                let id = named(&backend, name).expect("project");
                toasts.push(notice(&backend.delete_project(&id)));
                kept_projects = true;
            }
            "save" => {
                let id = named(&backend, name).expect("project");
                let edit: Edit = delete_clips(
                    &backend.project(&id).expect("project").edit,
                    &strings(&step["ids"]),
                    step["ripple"] == json!(true),
                );
                backend.save_edit(&id, &edit, at).expect("saved");
                kept_projects = true;
            }
            "prefs" => {
                let patch = &step["patch"];
                backend.update_prefs(|prefs| {
                    if let Some(snap) = patch["snap"].as_bool() {
                        prefs.snap = snap;
                    }
                    if let Some(size) = patch["exportSize"].as_str() {
                        prefs.export_size = ExportSize::ALL.into_iter().find(|one| one.key() == size).expect("size");
                    }
                });
                kept_prefs = true;
            }
            _ => {}
        }
        looks.push(json!({
            "projects": if kept_projects { to_json(&backend.projects) } else { Value::Null },
            "order": if kept_projects { by_updated(&backend.projects).iter().map(|one| one.id.clone()).collect() } else { Vec::<String>::new() },
            "prefs": if kept_prefs { json!([to_json(&backend.prefs)]) } else { Value::Null },
            "toasts": toasts,
            "went": went,
        }));
    }
    same(&Value::Array(looks), &web()["looks"], "looks");
}
