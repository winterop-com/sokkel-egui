//! The logic of the web's editor pages (`editor/EditorPage.tsx`, `ClipInspector.tsx`, `Bin.tsx`,
//! `Preview.tsx`) and of the projects page (`ProjectsPage.tsx`), without their drawing: what the
//! timeline is handed, where a source from the bin is placed, what the inspector lists and reads,
//! the bars of a sound's card, and the still a project's card shows.

use std::sync::Arc;

use crate::js::{number_to_string, trim};
use crate::media::{KindColor, PEAKS_PER_SECOND, Source, SourceKind, peaks_of};
use crate::model::{
    Clip, ClipPatch, Edit, FPS, FRAME, Project, Track, end, first_track, length_of, length_text, name_of,
    placed_length, timecode, track_of,
};
use crate::render::picture_at;

/// A clip as the timeline draws it: its name, its source's color and length, and a sound's peaks.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineClip {
    pub id: String,
    pub track: String,
    pub start: f64,
    pub duration: f64,
    pub in_point: f64,
    pub source_duration: Option<f64>,
    pub label: String,
    pub color: Option<KindColor>,
    /// The source's peaks, `PEAKS_PER_SECOND` of them a second, for a sound.
    pub waveform: Option<Arc<Vec<f64>>>,
}

/// How many peaks a second the timeline's waveforms hold.
pub const WAVEFORM_RATE: f64 = PEAKS_PER_SECOND;

pub fn timeline_clip(clip: &Clip) -> TimelineClip {
    let source = clip.source();
    TimelineClip {
        id: clip.id.clone(),
        track: clip.track.clone(),
        start: clip.start,
        duration: clip.duration,
        in_point: clip.in_point,
        source_duration: source.map(|source| source.duration),
        label: name_of(clip),
        color: source.map(|source| source.color),
        waveform: source.filter(|source| source.kind == SourceKind::Audio).map(peaks_of),
    }
}

/// How long the timeline runs: the edit and five seconds after it, at least half a minute.
pub fn timeline_duration(length: f64) -> f64 {
    (length + 5.0).max(30.0)
}

/// Where a source from the bin goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// At the playhead, or after what is there.
    Playhead,
    /// After the last clip on its track.
    End,
}

/// The second a source is placed from: the playhead, or the end of its track.
pub fn place_from(edit: &Edit, source: &Source, place: Place, playhead: f64) -> f64 {
    let track = first_track(edit, source.kind.track());
    match (place, track) {
        (Place::End, Some(track)) => edit
            .clips
            .iter()
            .filter(|clip| clip.track == track.id)
            .map(end)
            .fold(0.0, f64::max),
        _ => playhead,
    }
}

/// What the editor says when a source has no track to go on.
pub fn no_track_for(kind: SourceKind) -> String {
    format!(
        "No unlocked {} track to place it on",
        match kind {
            SourceKind::Video => "picture",
            SourceKind::Audio => "sound",
        }
    )
}

/// What the editor says when a title has no track to go on.
pub const NO_TITLE_TRACK: &str = "No unlocked title track to place it on";

/// The words a new title starts with.
pub const NEW_TITLE: &str = "A new title";

/// Whether Split at the playhead has a clip to cut: one chosen (any, when none is chosen) under the
/// playhead.
pub fn chosen_under(edit: &Edit, selected: &[String], playhead: f64) -> bool {
    edit.clips.iter().any(|clip| {
        (selected.is_empty() || selected.contains(&clip.id)) && clip.start < playhead && playhead < end(clip)
    })
}

/// Whether a chosen clip is under the playhead, which the inspector's Split asks.
pub fn any_chosen_under(edit: &Edit, selected: &[String], playhead: f64) -> bool {
    edit.clips
        .iter()
        .any(|clip| selected.contains(&clip.id) && clip.start < playhead && playhead < end(clip))
}

/// The chosen clips that are still in the edit, a clip deleted or undone away no longer chosen.
pub fn present(edit: &Edit, selected: &[String]) -> Vec<String> {
    selected
        .iter()
        .filter(|id| edit.clips.iter().any(|clip| &clip.id == *id))
        .cloned()
        .collect()
}

/// Reads a time typed as a timecode (00:04:12), minutes and seconds (0:04), or plain seconds (4.4).
pub fn parse_time(text: &str) -> Option<f64> {
    let parts: Vec<&str> = trim(text).split(':').collect();
    let number = |part: &str| -> Option<f64> {
        let (whole, fraction) = part.split_once('.').unwrap_or((part, ""));
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        if !digits(whole) || (part.contains('.') && !digits(fraction)) {
            return None;
        }
        part.parse().ok()
    };
    let numbers: Vec<f64> = parts.iter().map(|part| number(part)).collect::<Option<_>>()?;
    match numbers[..] {
        [seconds] => Some(seconds),
        [minutes, seconds] => Some(minutes * 60.0 + seconds),
        [minutes, seconds, frames] => Some(minutes * 60.0 + seconds + frames / FPS),
        [hours, minutes, seconds, frames] => Some(hours * 3600.0 + minutes * 60.0 + seconds + frames / FPS),
        _ => None,
    }
}

/// Whether a time read from a field is within its bounds, as the inspector's time fields take it.
pub fn within(value: f64, min: Option<f64>, max: Option<f64>) -> bool {
    min.is_none_or(|min| value >= min - 1e-6) && max.is_none_or(|max| value <= max + 1e-6)
}

/// The bounds of the inspector's time fields for a clip: start, in point, out point and duration,
/// each its lowest and highest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeBounds {
    pub start: (Option<f64>, Option<f64>),
    pub in_point: (Option<f64>, Option<f64>),
    pub out_point: (Option<f64>, Option<f64>),
    pub duration: (Option<f64>, Option<f64>),
}

pub fn time_bounds(clip: &Clip) -> TimeBounds {
    let source = clip.source();
    TimeBounds {
        start: (Some(0.0), None),
        in_point: (Some(0.0), Some(clip.in_point + clip.duration - FRAME)),
        out_point: (Some(clip.in_point + FRAME), source.map(|source| source.duration)),
        duration: (Some(FRAME), source.map(|source| source.duration - clip.in_point)),
    }
}

/// The change a new in point makes: the clip keeps its out point.
pub fn in_point_patch(clip: &Clip, in_point: f64) -> ClipPatch {
    ClipPatch {
        in_point: Some(in_point),
        duration: Some(clip.in_point + clip.duration - in_point),
        ..ClipPatch::default()
    }
}

/// The change a new out point makes.
pub fn out_point_patch(clip: &Clip, out: f64) -> ClipPatch {
    ClipPatch {
        duration: Some(out - clip.in_point),
        ..ClipPatch::default()
    }
}

/// The volume slider's value, in percent.
pub fn volume_percent(clip: &Clip) -> f64 {
    crate::js::round(clip.volume.unwrap_or(1.0) * 100.0)
}

/// The inspector's title: the chosen clip's name, how many are chosen, or `Edit`.
pub fn inspector_title(edit: &Edit, selected: &[String]) -> String {
    let chosen: Vec<&Clip> = edit.clips.iter().filter(|clip| selected.contains(&clip.id)).collect();
    match chosen[..] {
        [clip] => name_of(clip),
        [] => "Edit".to_owned(),
        _ => format!("{} clips", chosen.len()),
    }
}

/// What the inspector lists about the edit when no clip is chosen.
pub fn edit_summary(edit: &Edit) -> Vec<(&'static str, String)> {
    vec![
        ("Length", timecode(length_of(edit))),
        ("Clips", edit.clips.len().to_string()),
        ("Frame rate", format!("{} frames a second", number_to_string(FPS))),
    ]
}

/// What the inspector lists about a clip under About.
pub fn clip_about(edit: &Edit, clip: &Clip) -> Vec<(&'static str, String)> {
    let track: Option<&Track> = track_of(edit, &clip.track);
    let mut items = vec![
        (
            "Track",
            track.map_or_else(|| clip.track.clone(), |track| track.name.clone()),
        ),
        ("Ends at", timecode(end(clip))),
    ];
    if let Some(source) = clip.source() {
        items.push(("Source", source.name.to_owned()));
        items.push(("Source length", length_text(source.duration)));
        items.push(("Recorded at", source.place.to_owned()));
    }
    items
}

/// What the inspector lists when several clips are chosen: each one's name, start and length.
pub fn chosen_list(edit: &Edit, selected: &[String]) -> Vec<(String, String)> {
    edit.clips
        .iter()
        .filter(|clip| selected.contains(&clip.id))
        .map(|clip| {
            (
                name_of(clip),
                format!("{} for {}", timecode(clip.start), length_text(clip.duration)),
            )
        })
        .collect()
}

/// How many bars a sound's card draws.
pub const BARS: usize = 48;

/// A sound's peaks gathered into `BARS` bars, the loudest of each run.
pub fn bars_of(source: &Source) -> Vec<f64> {
    let peaks = peaks_of(source);
    let each = (peaks.len() / BARS).max(1);
    (0..BARS.min(peaks.len()))
        .map(|index| {
            peaks
                .iter()
                .skip(index * each)
                .take(each)
                .fold(0.0, |loudest: f64, peak| loudest.max(*peak))
        })
        .collect()
}

/// What a bin tile carries onto the timeline: its kind, the length it is placed at, and its source.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaDrag {
    pub label: &'static str,
    pub kind: SourceKind,
    pub duration: f64,
    pub source: &'static str,
}

pub fn media_drag(source: &Source) -> MediaDrag {
    MediaDrag {
        label: source.name,
        kind: source.kind,
        duration: placed_length(source),
        source: source.id,
    }
}

/// The still a project's card shows: its picture a second into the edit (half its first picture
/// clip when that is shorter), as a source and the second of it; none for an edit without one.
pub fn thumbnail_of(project: &Project) -> Option<(&'static Source, f64)> {
    let mut pictures: Vec<&Clip> = project
        .edit
        .clips
        .iter()
        .filter(|clip| clip.source().is_some_and(|source| source.kind == SourceKind::Video))
        .collect();
    pictures.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
    let first = pictures.first()?;
    let at = first.start + (first.duration / 2.0).min(1.0);
    let clip = picture_at(&project.edit, at)?;
    let source = clip.source()?;
    Some((source, clip.in_point + at - clip.start))
}
