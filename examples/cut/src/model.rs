//! An edit, as the web's `model.ts` keeps it: tracks of clips laid along one time axis in seconds.
//! A picture or sound clip plays its source from `in_point` for `duration` seconds, starting at
//! `start` on the timeline; a title clip writes its text over the picture for its time. Every edit
//! is a function from one edit to the next, so undo and redo hold whole edits.
//!
//! The arithmetic is the web's, in the same order, so a time lands on the same frame to the bit.

use serde::{Deserialize, Serialize};

use crate::js::{base36, number_to_string, round};
use crate::media::{Source, SourceKind, source_of};

/// What a track holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackKind {
    Video,
    Audio,
    Title,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub kind: TrackKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

impl Track {
    fn new(id: &str, name: &str, kind: TrackKind) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            kind,
            muted: None,
            locked: None,
            hidden: None,
        }
    }

    pub fn is_muted(&self) -> bool {
        self.muted == Some(true)
    }

    pub fn is_locked(&self) -> bool {
        self.locked == Some(true)
    }

    pub fn is_hidden(&self) -> bool {
        self.hidden == Some(true)
    }
}

/// A change to a track's switches, as the timeline hands it over; what is `None` stays.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackChange {
    pub muted: Option<bool>,
    pub locked: Option<bool>,
    pub hidden: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: String,
    pub track: String,
    /// Where it starts on the timeline, in seconds.
    pub start: f64,
    /// How long it plays, in seconds.
    pub duration: f64,
    /// Where in its source it starts, in seconds; 0 for a title.
    pub in_point: f64,
    /// The bin's source a picture or sound clip plays; none for a title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Its own name; the source's when not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The words a title clip writes over the picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// How loud a sound clip plays, from 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
}

impl Clip {
    /// The source it plays, if it plays one of the bin's.
    pub fn source(&self) -> Option<&'static Source> {
        source_of(self.source.as_deref())
    }
}

/// What undo and redo step between: the tracks and their clips.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Edit {
    pub tracks: Vec<Track>,
    pub clips: Vec<Clip>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    /// Who made it.
    pub owner: String,
    /// When it was made, in milliseconds since 1970.
    pub created: i64,
    /// When it was last changed, in milliseconds since 1970.
    pub updated: i64,
    #[serde(flatten)]
    pub edit: Edit,
}

/// Frames a second.
pub const FPS: f64 = 30.0;

/// A frame's length in seconds.
pub const FRAME: f64 = 1.0 / FPS;

/// What a time counts as the same as another within, in seconds.
const NEAR: f64 = 1e-6;

/// The tracks a new project starts with, from the top: a title over the picture, the picture, and
/// two of sound.
pub fn default_tracks() -> Vec<Track> {
    vec![
        Track::new("T1", "Titles", TrackKind::Title),
        Track::new("V1", "Picture", TrackKind::Video),
        Track::new("A1", "Sound 1", TrackKind::Audio),
        Track::new("A2", "Sound 2", TrackKind::Audio),
    ]
}

/// The larger of two numbers as `Math.max` gives it: +0 over -0.
fn js_max(a: f64, b: f64) -> f64 {
    if a > b || (a == b && a.is_sign_positive()) {
        a
    } else {
        b
    }
}

/// Rounds a time to the nearest frame, so every edit lands on one.
pub fn on_frame(seconds: f64) -> f64 {
    js_max(0.0, round(seconds * FPS) / FPS)
}

/// How long the edit runs: to the end of its last clip.
pub fn length_of(edit: &Edit) -> f64 {
    edit.clips
        .iter()
        .fold(0.0, |last, clip| js_max(last, clip.start + clip.duration))
}

/// Where a clip ends on the timeline.
pub fn end(clip: &Clip) -> f64 {
    clip.start + clip.duration
}

pub fn track_of<'a>(edit: &'a Edit, id: &str) -> Option<&'a Track> {
    edit.tracks.iter().find(|track| track.id == id)
}

/// A clip's name: its own, the title's words, or its source's.
pub fn name_of(clip: &Clip) -> String {
    if let Some(name) = clip.name.as_deref().filter(|name| !name.is_empty()) {
        return name.to_owned();
    }
    if let Some(text) = clip.text.as_deref() {
        return if text.is_empty() { "Title" } else { text }.to_owned();
    }
    clip.source().map_or("Clip", |source| source.name).to_owned()
}

/// Whether a clip plays at second `t`.
pub fn plays_at(clip: &Clip, t: f64) -> bool {
    clip.start <= t + NEAR && t < end(clip) - NEAR
}

/// The clip on `track` that plays at second `t`, if any.
pub fn clip_at<'a>(edit: &'a Edit, track: &str, t: f64) -> Option<&'a Clip> {
    edit.clips.iter().find(|clip| clip.track == track && plays_at(clip, t))
}

/// The first unlocked track of a kind, where a clip of that kind goes when none is named.
pub fn first_track(edit: &Edit, kind: TrackKind) -> Option<&Track> {
    edit.tracks
        .iter()
        .find(|track| track.kind == kind && !track.is_locked())
}

/// What makes new clips' ids: `clip-<now in base 36>-<count>`, the count running over the visit,
/// as the web's `clipId`. `now` is the moment the next id is made at; the editor sets it before an
/// edit, a test holds it.
#[derive(Clone, Debug)]
pub struct ClipIds {
    made: u64,
    pub now: i64,
}

impl ClipIds {
    pub fn new(now: i64) -> Self {
        Self { made: 0, now }
    }

    /// The next id.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> String {
        self.made += 1;
        format!("clip-{}-{}", base36(self.now), self.made)
    }
}

/// Where on `track` a clip of `duration` can start at or after `at` without covering another.
pub fn free_start(edit: &Edit, track: &str, at: f64, duration: f64, ignore: Option<&str>) -> f64 {
    let mut others: Vec<&Clip> = edit
        .clips
        .iter()
        .filter(|clip| clip.track == track && Some(clip.id.as_str()) != ignore)
        .collect();
    others.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
    let mut start = on_frame(at);
    for clip in others {
        if start + duration <= clip.start + NEAR {
            break;
        }
        if start < end(clip) - NEAR {
            start = on_frame(end(clip));
        }
    }
    start
}

/// How long a source's clip runs when it is placed: a picture up to 8 seconds, a sound up to 16.
pub fn placed_length(source: &Source) -> f64 {
    source
        .duration
        .min(if source.kind == SourceKind::Video { 8.0 } else { 16.0 })
}

/// A new clip of a source on `track` at `start`, as long as `placed_length` says.
fn source_clip(source: &Source, track: &str, start: f64, ids: &mut ClipIds) -> Clip {
    Clip {
        id: ids.next(),
        track: track.to_owned(),
        start: on_frame(start),
        duration: placed_length(source),
        in_point: 0.0,
        source: Some(source.id.to_owned()),
        name: None,
        text: None,
        volume: (source.kind == SourceKind::Audio).then_some(0.8),
    }
}

/// An edit with one clip added, and the new clip's id.
fn with_clip(edit: &Edit, clip: Clip) -> (Edit, String) {
    let id = clip.id.clone();
    let mut clips = edit.clips.clone();
    clips.push(clip);
    (
        Edit {
            tracks: edit.tracks.clone(),
            clips,
        },
        id,
    )
}

/// Places a source on `track` at `at`, where a timeline's drop has found it room; the edit with
/// the new clip's id, or none when the track is of another kind or locked.
pub fn drop_source(edit: &Edit, source: &Source, track: &str, at: f64, ids: &mut ClipIds) -> Option<(Edit, String)> {
    let target = track_of(edit, track)?;
    if target.kind != source.kind.track() || target.is_locked() {
        return None;
    }
    let clip = source_clip(source, track, at, ids);
    Some(with_clip(edit, clip))
}

/// Places a source on its kind's first track at `at`, or after what is there; the edit with the
/// new clip's id.
pub fn add_source(edit: &Edit, source: &Source, at: f64, ids: &mut ClipIds) -> Option<(Edit, String)> {
    let track = first_track(edit, source.kind.track())?;
    let start = free_start(edit, &track.id, at, placed_length(source), None);
    let clip = source_clip(source, &track.id, start, ids);
    Some(with_clip(edit, clip))
}

/// Adds a title of `duration` seconds at `at` on the first title track.
pub fn add_title(edit: &Edit, at: f64, text: &str, duration: f64, ids: &mut ClipIds) -> Option<(Edit, String)> {
    let track = first_track(edit, TrackKind::Title)?;
    let id = ids.next();
    let clip = Clip {
        id,
        track: track.id.clone(),
        start: free_start(edit, &track.id, at, duration, None),
        duration,
        in_point: 0.0,
        source: None,
        name: None,
        text: Some(text.to_owned()),
        volume: None,
    };
    Some(with_clip(edit, clip))
}

/// The longest a clip may run from `in_point`: to its source's end, or without limit for a title.
pub fn max_duration(clip: &Clip, in_point: f64) -> f64 {
    clip.source().map_or(f64::INFINITY, |source| source.duration - in_point)
}

/// A change to one clip; what is `None` stays as it is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClipPatch {
    pub track: Option<String>,
    pub start: Option<f64>,
    pub duration: Option<f64>,
    pub in_point: Option<f64>,
    pub name: Option<String>,
    pub text: Option<String>,
    pub volume: Option<f64>,
}

/// Changes one clip, held to its source and to the frames.
pub fn patch_clip(edit: &Edit, id: &str, patch: &ClipPatch) -> Edit {
    let clips = edit
        .clips
        .iter()
        .map(|clip| {
            if clip.id != id {
                return clip.clone();
            }
            let mut next = clip.clone();
            if let Some(track) = &patch.track {
                next.track = track.clone();
            }
            if let Some(start) = patch.start {
                next.start = start;
            }
            if let Some(duration) = patch.duration {
                next.duration = duration;
            }
            if let Some(in_point) = patch.in_point {
                next.in_point = in_point;
            }
            if let Some(name) = &patch.name {
                next.name = Some(name.clone());
            }
            if let Some(text) = &patch.text {
                next.text = Some(text.clone());
            }
            if let Some(volume) = patch.volume {
                next.volume = Some(volume);
            }
            let source = next.source();
            next.start = on_frame(next.start);
            next.in_point = match source {
                None => 0.0,
                Some(source) => on_frame(next.in_point).min(source.duration - FRAME),
            };
            next.duration = js_max(FRAME, on_frame(next.duration).min(max_duration(&next, next.in_point)));
            next
        })
        .collect();
    Edit {
        tracks: edit.tracks.clone(),
        clips,
    }
}

/// Cuts a clip in two at second `at` of the timeline; the edit with the new tail's id, or none
/// when `at` is not inside it.
pub fn split_clip(edit: &Edit, id: &str, at: f64, ids: &mut ClipIds) -> Option<(Edit, String)> {
    let clip = edit.clips.iter().find(|one| one.id == id)?;
    let cut = on_frame(at);
    if cut <= clip.start + FRAME / 2.0 || cut >= end(clip) - FRAME / 2.0 {
        return None;
    }
    let head = cut - clip.start;
    let tail = Clip {
        id: ids.next(),
        start: cut,
        duration: on_frame(clip.duration - head),
        in_point: if clip.source.is_none() {
            0.0
        } else {
            on_frame(clip.in_point + head)
        },
        ..clip.clone()
    };
    let tail_id = tail.id.clone();
    let mut tail = Some(tail);
    let clips = edit
        .clips
        .iter()
        .flat_map(|one| {
            if one.id == id {
                let mut first = one.clone();
                first.duration = on_frame(head);
                vec![Some(first), tail.take()]
            } else {
                vec![Some(one.clone())]
            }
        })
        .flatten()
        .collect();
    Some((
        Edit {
            tracks: edit.tracks.clone(),
            clips,
        },
        tail_id,
    ))
}

/// Cuts every clip under the playhead on an unlocked track, only those in `only` when it names
/// any; none when nothing was under it.
pub fn split_all(edit: &Edit, at: f64, only: &[String], ids: &mut ClipIds) -> Option<Edit> {
    let mut next: Option<Edit> = None;
    for clip in &edit.clips {
        if !only.is_empty() && !only.contains(&clip.id) {
            continue;
        }
        if track_of(edit, &clip.track).is_some_and(Track::is_locked) {
            continue;
        }
        if let Some((done, _)) = split_clip(next.as_ref().unwrap_or(edit), &clip.id, at, ids) {
            next = Some(done);
        }
    }
    next
}

/// Takes clips off; with `ripple`, what follows each on its track moves left by the gap it leaves,
/// so the track closes up. A clip on a locked track stays.
pub fn delete_clips(edit: &Edit, ids: &[String], ripple: bool) -> Edit {
    let gone: Vec<usize> = edit
        .clips
        .iter()
        .enumerate()
        .filter(|(_, clip)| ids.contains(&clip.id) && !track_of(edit, &clip.track).is_some_and(Track::is_locked))
        .map(|(index, _)| index)
        .collect();
    let mut clips: Vec<Clip> = edit
        .clips
        .iter()
        .enumerate()
        .filter(|(index, _)| !gone.contains(index))
        .map(|(_, clip)| clip.clone())
        .collect();
    if ripple {
        // Latest first, so each shift is measured on the track as it stood.
        let mut removed: Vec<&Clip> = gone.iter().map(|index| &edit.clips[*index]).collect();
        removed.sort_by(|a, b| b.start.partial_cmp(&a.start).unwrap_or(std::cmp::Ordering::Equal));
        for removed in removed {
            for clip in &mut clips {
                if clip.track == removed.track && clip.start >= end(removed) - NEAR {
                    clip.start = on_frame(clip.start - removed.duration);
                }
            }
        }
    }
    Edit {
        tracks: edit.tracks.clone(),
        clips,
    }
}

/// A move or trim a timeline hands over for one clip; what is `None` stays.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClipChange {
    pub id: String,
    pub start: Option<f64>,
    pub duration: Option<f64>,
    pub in_point: Option<f64>,
    pub track: Option<String>,
}

/// Moves or trims clips as a timeline hands the changes over, each held to its source and to the
/// frames; a clip only moves onto an unlocked track of its own kind. None when no change named a
/// clip of the edit, as the web hands back the same edit then and no step of undo is made.
pub fn apply_changes(edit: &Edit, changes: &[ClipChange]) -> Option<Edit> {
    let mut next: Option<Edit> = None;
    for change in changes {
        let current = next.as_ref().unwrap_or(edit);
        let Some(clip) = current.clips.iter().find(|one| one.id == change.id) else {
            continue;
        };
        let mut track = change.track.clone();
        if let Some(to) = &track {
            let from = track_of(current, &clip.track).map(|track| track.kind);
            let to = track_of(current, to);
            if to.is_none_or(|to| Some(to.kind) != from || to.is_locked()) {
                track = None;
            }
        }
        let patch = ClipPatch {
            track,
            start: change.start,
            duration: change.duration,
            in_point: change.in_point,
            ..ClipPatch::default()
        };
        next = Some(patch_clip(current, &change.id, &patch));
    }
    next
}

/// An edit with one track's switches changed.
pub fn set_track(edit: &Edit, id: &str, change: &TrackChange) -> Edit {
    let tracks = edit
        .tracks
        .iter()
        .map(|track| {
            let mut track = track.clone();
            if track.id == id {
                if change.muted.is_some() {
                    track.muted = change.muted;
                }
                if change.locked.is_some() {
                    track.locked = change.locked;
                }
                if change.hidden.is_some() {
                    track.hidden = change.hidden;
                }
            }
            track
        })
        .collect();
    Edit {
        tracks,
        clips: edit.clips.clone(),
    }
}

/// A copy of an edit with new clip ids, for a duplicated project.
pub fn copy_edit(edit: &Edit, ids: &mut ClipIds) -> Edit {
    Edit {
        tracks: edit.tracks.clone(),
        clips: edit
            .clips
            .iter()
            .map(|clip| Clip {
                id: ids.next(),
                ..clip.clone()
            })
            .collect(),
    }
}

/// A time as timecode, `HH:MM:SS:FF`, at `fps` frames a second, as sokkel-ui's `formatTimecode`.
pub fn format_timecode(seconds: f64, fps: f64) -> String {
    let rate = round(fps).max(1.0) as i64;
    let frames = round(js_max(seconds, 0.0) * rate as f64) as i64;
    let whole = frames.div_euclid(rate);
    let width = (rate - 1).to_string().len();
    format!(
        "{:02}:{:02}:{:02}:{:0width$}",
        whole / 3600,
        (whole % 3600) / 60,
        whole % 60,
        frames % rate,
    )
}

/// A time as an editor writes it, at the edit's frame rate: 00:01:24:12.
pub fn timecode(seconds: f64) -> String {
    format_timecode(seconds, FPS)
}

/// A length in words a person reads: 1:24, or 0:08.
pub fn length_text(seconds: f64) -> String {
    let whole = round(seconds);
    let minutes = (whole / 60.0).floor();
    let rest = whole % 60.0;
    let rest = number_to_string(rest);
    let padded = if rest.chars().count() < 2 {
        format!("{}{rest}", "0".repeat(2 - rest.chars().count()))
    } else {
        rest
    };
    format!("{}:{padded}", number_to_string(minutes))
}

/// The resolution an export is made at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExportSize {
    #[serde(rename = "720")]
    P720,
    #[serde(rename = "480")]
    P480,
    #[serde(rename = "360")]
    P360,
}

/// What an export size makes: its label, its frame's width and height, and its bits a second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportSpec {
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
    pub bitrate: u32,
}

impl ExportSize {
    /// Every size, in the order the export dialog offers them.
    pub const ALL: [ExportSize; 3] = [Self::P720, Self::P480, Self::P360];

    /// The size as the web keys it: "720", "480", "360".
    pub fn key(self) -> &'static str {
        match self {
            Self::P720 => "720",
            Self::P480 => "480",
            Self::P360 => "360",
        }
    }

    pub fn spec(self) -> ExportSpec {
        match self {
            Self::P720 => ExportSpec {
                label: "720p",
                width: 1280,
                height: 720,
                bitrate: 2_500_000,
            },
            Self::P480 => ExportSpec {
                label: "480p",
                width: 854,
                height: 480,
                bitrate: 1_200_000,
            },
            Self::P360 => ExportSpec {
                label: "360p",
                width: 640,
                height: 360,
                bitrate: 700_000,
            },
        }
    }
}

/// The person's settings, kept by the backend.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub id: String,
    /// Clips snap to each other's edges and to the playhead while dragged.
    pub snap: bool,
    /// Plays from the start again at the end.
    #[serde(rename = "loop")]
    pub looping: bool,
    /// How long a new title runs, in seconds.
    pub title_length: f64,
    /// The resolution an export is made at.
    pub export_size: ExportSize,
    /// The preview draws at half size, for a slow machine.
    pub light_preview: bool,
    /// Delete closes the gap a clip leaves, as Ripple delete does.
    pub ripple_by_default: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            id: "prefs".to_owned(),
            snap: true,
            looping: false,
            title_length: 4.0,
            export_size: ExportSize::P480,
            light_preview: false,
            ripple_by_default: false,
        }
    }
}
