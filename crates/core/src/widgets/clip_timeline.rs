//! The multi-track timeline of a video editor, after sokkel-ui's `ClipTimeline` (sokkel-design's
//! `clip-timeline` spec).

use std::sync::Arc;

use egui::{
    Color32, CornerRadius, CursorIcon, DragAndDrop, Id, Key, Mesh, Painter, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, TextWrapMode, Ui, Vec2, WidgetText, pos2, vec2,
};

use crate::{Icon, TextRole, Tokens, icons, mono_font, mono_size, tokens};

/// What a track holds, which sets its icon, its clips' color by default and which clips may move
/// onto it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClipTrackKind {
    Video,
    Audio,
    Title,
}

impl ClipTrackKind {
    /// The kind's icon: a film, a sound wave, a T.
    pub fn icon(self) -> Icon {
        match self {
            Self::Video => Icon::Film,
            Self::Audio => Icon::AudioLines,
            Self::Title => Icon::Type,
        }
    }

    /// The color its clips wear when they name none: video blue, audio green, a title violet.
    pub fn default_color(self) -> KindColor {
        match self {
            Self::Video => KindColor::Blue,
            Self::Audio => KindColor::Green,
            Self::Title => KindColor::Violet,
        }
    }
}

/// One of the kind colors, `kind-<color>` in the tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KindColor {
    Blue,
    Green,
    Jade,
    Pink,
    Teal,
    Violet,
}

impl KindColor {
    pub const ALL: [KindColor; 6] = [
        Self::Blue,
        Self::Green,
        Self::Jade,
        Self::Pink,
        Self::Teal,
        Self::Violet,
    ];

    /// The color's name in the tokens: `kind-<key>` and `kind-<key>-ink`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Jade => "jade",
            Self::Pink => "pink",
            Self::Teal => "teal",
            Self::Violet => "violet",
        }
    }
}

/// One row of the timeline.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineTrack {
    pub id: String,
    pub name: String,
    pub kind: ClipTrackKind,
    /// Its sound off: its clips drawn faint.
    pub muted: bool,
    /// Its clips neither move, trim, split nor delete.
    pub locked: bool,
    /// Left out of the picture: its clips drawn faint.
    pub hidden: bool,
}

impl TimelineTrack {
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: ClipTrackKind) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            muted: false,
            locked: false,
            hidden: false,
        }
    }

    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }
}

/// One clip on a track, its times in seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineClip {
    pub id: String,
    /// The id of the track it lies on.
    pub track: String,
    pub start: f64,
    pub duration: f64,
    /// Where in its source it starts; 0 by default.
    pub in_point: f64,
    /// How long its source is: trimming never reaches past either end of it.
    pub source_duration: Option<f64>,
    pub label: String,
    /// Its color; by its track's kind when `None`.
    pub color: Option<KindColor>,
    /// The loudness of its whole source from 0 to 1, drawn across an audio clip for the part it plays.
    pub waveform: Option<Arc<[f32]>>,
}

impl TimelineClip {
    pub fn new(
        id: impl Into<String>,
        track: impl Into<String>,
        start: f64,
        duration: f64,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            track: track.into(),
            start,
            duration,
            in_point: 0.0,
            source_duration: None,
            label: label.into(),
            color: None,
            waveform: None,
        }
    }

    pub fn in_point(mut self, at: f64) -> Self {
        self.in_point = at;
        self
    }

    pub fn source_duration(mut self, length: f64) -> Self {
        self.source_duration = Some(length);
        self
    }

    pub fn color(mut self, color: KindColor) -> Self {
        self.color = Some(color);
        self
    }

    pub fn waveform(mut self, peaks: impl Into<Arc<[f32]>>) -> Self {
        self.waveform = Some(peaks.into());
        self
    }

    fn end(&self) -> f64 {
        self.start + self.duration
    }
}

/// What a move or a trim changed of one clip; only what changed is given.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClipChange {
    pub id: String,
    pub start: Option<f64>,
    pub duration: Option<f64>,
    pub in_point: Option<f64>,
    pub track: Option<String>,
}

/// A header's toggle, pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackChange {
    Muted(bool),
    Hidden(bool),
    Locked(bool),
}

/// What a thing dragged from another part carries onto the timeline, as sokkel-ui's `DragData`: set
/// it as egui's drag payload (`egui::DragAndDrop::set_payload`) while it is dragged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DragData {
    /// What kind of thing it is, such as `media`.
    pub kind: String,
    /// What it is called, said once it lands.
    pub label: String,
    /// How long it lasts once dropped, in seconds, when known (the web's `value.duration`).
    pub duration: Option<f64>,
    /// The app's own value, such as the source's id.
    pub value: String,
}

/// A thing from elsewhere, let go on a track.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipDrop {
    pub data: DragData,
    /// The id of the track it was let go on.
    pub track: String,
    /// Where it lands, in seconds: snapped, and clear of the track's clips when its length is known.
    pub at: f64,
}

/// The words a clip timeline says. `{…}` are filled in.
#[derive(Clone, Debug)]
pub struct ClipTimelineWords {
    pub label: String,
    pub playhead: String,
    pub kind_video: String,
    pub kind_audio: String,
    pub kind_title: String,
    pub mute: String,
    pub lock: String,
    pub hide: String,
    /// `{label}, {start} to {end}`.
    pub clip: String,
    /// `Start {at}`, on the card while a clip moves.
    pub start: String,
    /// `Length {length}`, on the card while a clip is trimmed.
    pub length: String,
    /// `{clip} moved to {at}`.
    pub moved: String,
    /// `{clip} moved to {track} at {at}`.
    pub moved_to: String,
    /// `{clip} trimmed to {length}`.
    pub trimmed: String,
    /// `{clip} split at {at}`.
    pub split: String,
    pub deleted_one: String,
    /// `{count} clips deleted`.
    pub deleted_other: String,
    pub deleted_ripple_one: String,
    /// `{count} clips deleted, the gap closed`.
    pub deleted_ripple_other: String,
    /// `{clip} cannot go there`.
    pub blocked: String,
    /// `{item} added to {track} at {at}`.
    pub dropped: String,
    /// `Playhead at {at}`.
    pub at: String,
    /// For the zoom buttons an app draws beside the timeline.
    pub zoom_in: String,
    pub zoom_out: String,
    pub zoom_fit: String,
}

impl Default for ClipTimelineWords {
    fn default() -> Self {
        Self {
            label: "Timeline".into(),
            playhead: "Playhead".into(),
            kind_video: "Video".into(),
            kind_audio: "Audio".into(),
            kind_title: "Title".into(),
            mute: "Mute".into(),
            lock: "Lock".into(),
            hide: "Hide".into(),
            clip: "{label}, {start} to {end}".into(),
            start: "Start {at}".into(),
            length: "Length {length}".into(),
            moved: "{clip} moved to {at}".into(),
            moved_to: "{clip} moved to {track} at {at}".into(),
            trimmed: "{clip} trimmed to {length}".into(),
            split: "{clip} split at {at}".into(),
            deleted_one: "Clip deleted".into(),
            deleted_other: "{count} clips deleted".into(),
            deleted_ripple_one: "Clip deleted, the gap closed".into(),
            deleted_ripple_other: "{count} clips deleted, the gap closed".into(),
            blocked: "{clip} cannot go there".into(),
            dropped: "{item} added to {track} at {at}".into(),
            at: "Playhead at {at}".into(),
            zoom_in: "Zoom in".into(),
            zoom_out: "Zoom out".into(),
            zoom_fit: "Fit".into(),
        }
    }
}

impl ClipTimelineWords {
    /// The kind's name, beside its icon.
    pub fn kind(&self, kind: ClipTrackKind) -> &str {
        match kind {
            ClipTrackKind::Video => &self.kind_video,
            ClipTrackKind::Audio => &self.kind_audio,
            ClipTrackKind::Title => &self.kind_title,
        }
    }

    /// `deleted` for `count` clips, closing the gap with `ripple`.
    pub fn deleted(&self, count: usize, ripple: bool) -> String {
        let word = match (count == 1, ripple) {
            (true, false) => &self.deleted_one,
            (false, false) => &self.deleted_other,
            (true, true) => &self.deleted_ripple_one,
            (false, true) => &self.deleted_ripple_other,
        };
        word.replace("{count}", &super::table::thousands(count))
    }
}

/// Fills `{name}` in `word` with each value.
fn fill(word: &str, values: &[(&str, &str)]) -> String {
    let mut out = word.to_owned();
    for (name, value) in values {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// What the timeline was asked this frame. The clips stay where they are until the app moves them.
#[derive(Clone, Debug, Default)]
pub struct ClipTimelineResponse {
    /// Where the playhead goes, on a frame: a press or a drag on the ruler, or a key.
    pub playhead: Option<f64>,
    /// The clips chosen: a press or Enter chooses one; with Shift, Cmd or Ctrl it adds the clip or
    /// takes it away; a press on an empty part of a lane chooses none.
    pub selected: Option<Vec<String>>,
    /// Clips moved or trimmed, only what changed of each (with `changes`).
    pub changes: Vec<ClipChange>,
    /// S: each chosen clip the playhead stands inside, and the time (with `splits`).
    pub splits: Vec<(String, f64)>,
    /// Delete or Backspace on the chosen clips not on a locked track, and whether Shift asked to
    /// close the gap (with `deletes`).
    pub deleted: Option<(Vec<String>, bool)>,
    /// A header's toggle pressed (with `track_toggles`).
    pub track_changes: Vec<(String, TrackChange)>,
    /// A zoom asked for with Cmd or Ctrl and the wheel, or a pinch (with `zoom_asks`).
    pub zoom: Option<f32>,
    /// The zoom the fitted view draws at for the lanes' width now, for an app's zoom buttons.
    pub fit: f32,
    /// A thing from elsewhere let go on a track that takes it (with `drops`).
    pub dropped: Option<ClipDrop>,
    /// What a screen reader is told of the change made this frame.
    pub said: Option<String>,
}

/// The headers' column from 640 wide, and below.
const HEADER_WIDTH: f32 = 176.0;
const HEADER_WIDTH_NARROW: f32 = 112.0;
/// A track's header and lane.
const ROW_HEIGHT: f32 = 56.0;
/// The ruler and the corner over the headers.
const RULER_HEIGHT: f32 = 32.0;
/// A clip's and a landing's room from the lane's top and bottom.
const CLIP_INSET: f32 = 4.0;
const CLIP_MIN_WIDTH: f32 = 2.0;
const GRIP_WIDTH: f32 = 8.0;
/// How near in px an edge comes to a time before it snaps to it.
const SNAP_REACH: f32 = 8.0;
/// The least room a labeled tick takes.
const TICK_ROOM: f64 = 88.0;
/// Minor ticks are drawn only while a fifth of a step is at least this wide.
const MINOR_TICK_MIN: f64 = 8.0;
/// The lanes' width taken until there is a room to measure.
const DEFAULT_ROOM: f32 = 640.0;
/// How far right of and below the pointer the card stands.
const CARD_OFFSET: f32 = 12.0;

/// The steps ticks are placed at, in seconds, past the frame steps.
const SECOND_STEPS: [f64; 13] = [
    1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0,
];

/// The narrowest a second is drawn by a zoom the timeline asks for, in px.
pub const CLIP_TIMELINE_ZOOM_MIN: f32 = 0.05;
/// The widest.
pub const CLIP_TIMELINE_ZOOM_MAX: f32 = 600.0;
/// The fit's floor: a second at least this wide.
pub const CLIP_TIMELINE_FIT_PPS: f32 = 16.0;
/// A length that would outgrow this room at the floor is fitted to it instead.
pub const CLIP_TIMELINE_FIT_ROOM: f32 = 960.0;

const NEAR: f64 = 1e-6;

/// The zoom at which a timeline `length` seconds long fits lanes `room` px wide, never tighter than
/// the floor, or than `length` fitted to the fit's room when that is tighter still.
pub fn clip_fit_zoom(room: f32, length: f64) -> f32 {
    let span = length.max(1.0) as f32;
    (room / span).max(CLIP_TIMELINE_FIT_PPS.min(CLIP_TIMELINE_FIT_ROOM / span))
}

/// A time as timecode, `HH:MM:SS:FF`, at `fps` frames a second.
pub fn format_timecode(seconds: f64, fps: f64) -> String {
    let rate = fps.round().max(1.0) as u64;
    let frames = (seconds.max(0.0) * rate as f64).round() as u64;
    let whole = frames / rate;
    let width = (rate - 1).to_string().len().max(2);
    format!(
        "{:02}:{:02}:{:02}:{:0width$}",
        whole / 3600,
        (whole % 3600) / 60,
        whole % 60,
        frames % rate,
    )
}

/// The target nearest `time` within `reach` seconds.
fn nearest(time: f64, targets: &[f64], reach: f64) -> Option<f64> {
    let mut best: Option<f64> = None;
    for &target in targets {
        let distance = (target - time).abs();
        if distance <= reach && best.is_none_or(|b| distance < (b - time).abs()) {
            best = Some(target);
        }
    }
    best
}

/// A time snapped: to the nearest of `targets` within `reach` seconds, or else to the nearest frame.
pub fn snap_time(time: f64, targets: &[f64], reach: f64, fps: f64) -> f64 {
    nearest(time, targets, reach).unwrap_or_else(|| (time * fps).round() / fps)
}

/// Where a clip of `duration` asked to start at `start` lands among `others` (start, duration) on
/// its track, never on one: where asked when that is free, else the nearest free place beside a clip
/// in the way, or `None` when none is within the clip's own length of where it was asked.
pub fn place_clip(start: f64, duration: f64, others: &[(f64, f64)]) -> Option<f64> {
    let asked = start.max(0.0);
    let free = |at: f64| {
        at >= -NEAR
            && others
                .iter()
                .all(|(s, d)| at + duration <= s + NEAR || at >= s + d - NEAR)
    };
    if free(asked) {
        return Some(asked);
    }
    let mut best: Option<f64> = None;
    for (s, d) in others {
        for at in [s + d, s - duration] {
            if free(at) && best.is_none_or(|b| (at - asked).abs() < (b - asked).abs()) {
                best = Some(at);
            }
        }
    }
    best.filter(|b| (b - asked).abs() <= duration)
}

/// The next zoom a button asks for: `steps` steps of a quarter in, or out below zero, held within
/// the zoom's bounds.
pub fn zoom_step(pixels_per_second: f32, steps: i32) -> f32 {
    (pixels_per_second * 1.25f32.powi(steps)).clamp(CLIP_TIMELINE_ZOOM_MIN, CLIP_TIMELINE_ZOOM_MAX)
}

/// The step between labeled ticks for a zoom, in seconds: the first of 1, 2, 5 and 10 frames, then
/// 1 s to 1 h, that is at least 88 px wide.
pub fn clip_tick_step(pixels_per_second: f32, fps: f64) -> f64 {
    let frame = 1.0 / fps.round().max(1.0);
    [frame, 2.0 * frame, 5.0 * frame, 10.0 * frame]
        .into_iter()
        .chain(SECOND_STEPS)
        .find(|step| step * pixels_per_second as f64 >= TICK_ROOM)
        .unwrap_or(SECOND_STEPS[SECOND_STEPS.len() - 1])
}

/// Which part of a clip a drag carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grip {
    Move,
    Start,
    End,
}

/// Where a drag would put a clip.
#[derive(Clone, Debug, PartialEq)]
struct Proposal {
    id: String,
    grip: Grip,
    start: f64,
    duration: f64,
    in_point: f64,
    track: String,
    /// False when it would cover another clip.
    fits: bool,
    /// The time an edge snapped to, drawn as a line.
    snapped: Option<f64>,
}

/// A drag in hand, kept across frames.
#[derive(Clone, Debug)]
struct Carry {
    id: String,
    grip: Grip,
    /// How far from the grabbed edge the pointer was pressed, in seconds.
    grab: f64,
    /// Escape put it back: nothing lands when it is let go.
    cancelled: bool,
}

/// Where a thing from outside would land.
#[derive(Clone, Debug)]
struct Landing {
    track: String,
    start: f64,
    duration: Option<f64>,
    fits: bool,
    snapped: Option<f64>,
}

/// Which things from outside a track takes.
type Accepts<'a> = Box<dyn Fn(&DragData, &TimelineTrack) -> bool + 'a>;

/// The multi-track timeline of a video editor, after sokkel-ui's `ClipTimeline`: in a box on the
/// card, a row per track, its header at the start (its kind's icon, its name, and with
/// `track_toggles` the toggles to mute or hide it and to lock it) and its lane after it, under a
/// ruler of timecode that fits the zoom. Clips are blocks in their color with their label, an audio
/// clip drawing its waveform and a title dashed; the chosen ones are outlined. A press or a drag on
/// the ruler moves the playhead, a line across every track. With `changes` a clip drags along its
/// track or onto another of the same kind and either end trims it, snapped to the playhead, other
/// clips' edges and 0, a card at the pointer saying the new start or length, never landing on
/// another clip. With `drops` a [`DragData`] dragged from elsewhere lands on a track that takes it.
/// The lanes scroll both ways while the ruler stays at the top, the headers at the start and the
/// playhead's timecode over both. It fills the room the `ui` gives it.
///
/// From the keyboard each clip is a stop of its own, Enter choosing it; Left and Right move the
/// playhead a frame (a second with Shift), Home and End to either end, Alt with them the chosen
/// clips a frame, S splits them at the playhead and Delete deletes them (Shift closes the gap).
#[must_use = "show it with `.show(ui)`"]
pub struct ClipTimeline<'a> {
    id_salt: Id,
    tracks: &'a [TimelineTrack],
    clips: &'a [TimelineClip],
    duration: f64,
    playhead: f64,
    selected: &'a [String],
    changes: bool,
    splits: bool,
    deletes: bool,
    drops: bool,
    accepts: Option<Accepts<'a>>,
    track_toggles: bool,
    zoom: Option<f32>,
    zoom_asks: bool,
    fps: f64,
    snap: bool,
    words: ClipTimelineWords,
}

impl<'a> ClipTimeline<'a> {
    /// `duration` is the timeline's length in seconds; the ruler reaches at least this far, and
    /// further when a clip does. `playhead` is where the playhead stands.
    pub fn new(
        id_salt: impl egui::AsIdSalt,
        tracks: &'a [TimelineTrack],
        clips: &'a [TimelineClip],
        duration: f64,
        playhead: f64,
    ) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            tracks,
            clips,
            duration,
            playhead,
            selected: &[],
            changes: false,
            splits: false,
            deletes: false,
            drops: false,
            accepts: None,
            track_toggles: false,
            zoom: None,
            zoom_asks: false,
            fps: 30.0,
            snap: true,
            words: ClipTimelineWords::default(),
        }
    }

    /// The ids of the clips chosen, drawn outlined.
    pub fn selected(mut self, ids: &'a [String]) -> Self {
        self.selected = ids;
        self
    }

    /// Clips move and trim, as with the web's `onClipsChange`: grips, drags and Alt with the arrows.
    pub fn changes(mut self, on: bool) -> Self {
        self.changes = on;
        self
    }

    /// S splits the chosen clips, as with `onSplit`.
    pub fn splits(mut self, on: bool) -> Self {
        self.splits = on;
        self
    }

    /// Delete and Backspace delete the chosen clips, as with `onDelete`.
    pub fn deletes(mut self, on: bool) -> Self {
        self.deletes = on;
        self
    }

    /// A [`DragData`] payload let go on a track lands there, as with `onDrop`.
    pub fn drops(mut self, on: bool) -> Self {
        self.drops = on;
        self
    }

    /// Which things from outside a track takes; every one without it. A locked track takes none.
    pub fn accepts(mut self, accepts: impl Fn(&DragData, &TimelineTrack) -> bool + 'a) -> Self {
        self.accepts = Some(Box::new(accepts));
        self
    }

    /// The headers draw their toggles, as with `onTrackChange`.
    pub fn track_toggles(mut self, on: bool) -> Self {
        self.track_toggles = on;
        self
    }

    /// How wide a second is drawn, in px; `None` fits the length to the lanes. With neither a zoom
    /// nor `zoom_asks` the timeline keeps its own zoom.
    pub fn zoom(mut self, zoom: Option<f32>) -> Self {
        self.zoom = zoom;
        self
    }

    /// A zoom asked for with the wheel or a pinch is told in the response, as with `onZoomChange`.
    pub fn zoom_asks(mut self, on: bool) -> Self {
        self.zoom_asks = on;
        self
    }

    /// Frames a second; 30 by default.
    pub fn fps(mut self, fps: f64) -> Self {
        self.fps = fps;
        self
    }

    /// Whether a dragged edge snaps to the playhead, another clip's edge and 0; on by default.
    pub fn snap(mut self, snap: bool) -> Self {
        self.snap = snap;
        self
    }

    pub fn words(mut self, words: ClipTimelineWords) -> Self {
        self.words = words;
        self
    }

    fn track(&self, id: &str) -> Option<&'a TimelineTrack> {
        self.tracks.iter().find(|t| t.id == id)
    }

    fn clip(&self, id: &str) -> Option<&'a TimelineClip> {
        self.clips.iter().find(|c| c.id == id)
    }

    fn locked(&self, clip: &TimelineClip) -> bool {
        self.track(&clip.track).is_some_and(|t| t.locked)
    }

    fn timecode(&self, seconds: f64) -> String {
        format_timecode(seconds, self.fps)
    }

    fn frame(&self) -> f64 {
        1.0 / self.fps.round().max(1.0)
    }

    fn to_frame(&self, time: f64) -> f64 {
        let frame = self.frame();
        (time / frame).round() * frame
    }

    /// The times an edge snaps to: 0, the playhead and every other clip's edges.
    fn targets(&self, except: &str) -> Vec<f64> {
        if !self.snap {
            return Vec::new();
        }
        let mut out = vec![0.0, self.playhead];
        for clip in self.clips.iter().filter(|c| c.id != except) {
            out.push(clip.start);
            out.push(clip.end());
        }
        out
    }

    fn neighbors(&self, track: &str, except: &str) -> Vec<(f64, f64)> {
        self.clips
            .iter()
            .filter(|c| c.track == track && c.id != except)
            .map(|c| (c.start, c.duration))
            .collect()
    }

    /// A span asked to start at `asked`, snapped by whichever end comes nearer a target, else to a
    /// frame: its start, and the time it snapped to.
    fn snap_span(&self, asked: f64, length: f64, targets: &[f64], pps: f32) -> (f64, Option<f64>) {
        let near = (SNAP_REACH / pps) as f64;
        let by_start = nearest(asked, targets, near);
        let by_end = if length > 0.0 {
            nearest(asked + length, targets, near)
        } else {
            None
        };
        if let Some(s) = by_start
            && by_end.is_none_or(|e| (s - asked).abs() <= (e - asked - length).abs())
        {
            return (s, Some(s));
        }
        if let Some(e) = by_end {
            return (e - length, Some(e));
        }
        (self.to_frame(asked), None)
    }

    /// Where the drag in `carry` would put its clip, the pointer at `time` over `over`'s lane.
    fn propose(&self, carry: &Carry, time: f64, over: Option<&TimelineTrack>, pps: f32) -> Option<Proposal> {
        let clip = self.clip(&carry.id)?;
        let home = self.track(&clip.track)?;
        let asked = time - carry.grab;
        let targets = self.targets(&clip.id);
        let near = (SNAP_REACH / pps) as f64;
        let frame = self.frame();
        match carry.grip {
            Grip::Move => {
                let track = over.filter(|t| t.kind == home.kind && !t.locked).unwrap_or(home);
                let (start, snapped) = self.snap_span(asked, clip.duration, &targets, pps);
                let placed = place_clip(start.max(0.0), clip.duration, &self.neighbors(&track.id, &clip.id));
                Some(Proposal {
                    id: clip.id.clone(),
                    grip: Grip::Move,
                    start: placed.unwrap_or(start.max(0.0)),
                    duration: clip.duration,
                    in_point: clip.in_point,
                    track: track.id.clone(),
                    fits: placed.is_some(),
                    snapped: if placed.is_some_and(|p| (p - start).abs() < NEAR) {
                        snapped
                    } else {
                        None
                    },
                })
            }
            Grip::Start => {
                let others = self.neighbors(&clip.track, &clip.id);
                let snapped = nearest(asked, &targets, near);
                let at = snapped.unwrap_or_else(|| self.to_frame(asked));
                let before = others
                    .iter()
                    .map(|(s, d)| s + d)
                    .filter(|e| *e <= clip.start + NEAR)
                    .fold(0f64.max(clip.start - clip.in_point), f64::max);
                let start = at.max(before).min(clip.end() - frame);
                Some(Proposal {
                    id: clip.id.clone(),
                    grip: Grip::Start,
                    start,
                    duration: clip.end() - start,
                    in_point: clip.in_point + start - clip.start,
                    track: clip.track.clone(),
                    fits: true,
                    snapped: snapped.filter(|s| (start - s).abs() < NEAR),
                })
            }
            Grip::End => {
                let others = self.neighbors(&clip.track, &clip.id);
                let snapped = nearest(asked, &targets, near);
                let at = snapped.unwrap_or_else(|| self.to_frame(asked));
                let source_end = clip
                    .source_duration
                    .map_or(f64::INFINITY, |d| clip.start + d - clip.in_point);
                let after = others
                    .iter()
                    .map(|(s, _)| *s)
                    .filter(|s| *s >= clip.end() - NEAR)
                    .fold(source_end, f64::min);
                let finish = at.min(after).max(clip.start + frame);
                Some(Proposal {
                    id: clip.id.clone(),
                    grip: Grip::End,
                    start: clip.start,
                    duration: finish - clip.start,
                    in_point: clip.in_point,
                    track: clip.track.clone(),
                    fits: true,
                    snapped: snapped.filter(|s| (finish - s).abs() < NEAR),
                })
            }
        }
    }

    /// The change a proposal makes to its clip, or `None` when it makes none.
    fn change_of(&self, to: &Proposal) -> Option<ClipChange> {
        let clip = self.clip(&to.id)?;
        if !to.fits {
            return None;
        }
        let mut change = ClipChange {
            id: clip.id.clone(),
            ..ClipChange::default()
        };
        if (to.start - clip.start).abs() > NEAR {
            change.start = Some(to.start);
        }
        if (to.duration - clip.duration).abs() > NEAR {
            change.duration = Some(to.duration);
        }
        if (to.in_point - clip.in_point).abs() > NEAR {
            change.in_point = Some(to.in_point);
        }
        if to.track != clip.track {
            change.track = Some(to.track.clone());
        }
        let changed =
            change.start.is_some() || change.duration.is_some() || change.in_point.is_some() || change.track.is_some();
        changed.then_some(change)
    }

    fn say_change(&self, clip: &TimelineClip, change: &ClipChange) -> String {
        if let Some(length) = change.duration {
            return fill(
                &self.words.trimmed,
                &[("clip", &clip.label), ("length", &self.timecode(length))],
            );
        }
        let at = self.timecode(change.start.unwrap_or(clip.start));
        match change.track.as_deref().and_then(|t| self.track(t)) {
            Some(track) => fill(
                &self.words.moved_to,
                &[("clip", &clip.label), ("track", &track.name), ("at", &at)],
            ),
            None => fill(&self.words.moved, &[("clip", &clip.label), ("at", &at)]),
        }
    }

    /// Whether a track takes a thing dragged from outside.
    fn takes(&self, track: &TimelineTrack, drag: &DragData) -> bool {
        !track.locked && self.accepts.as_ref().is_none_or(|accepts| accepts(drag, track))
    }

    fn landing(&self, drag: &DragData, track: &TimelineTrack, asked: f64, pps: f32) -> Option<Landing> {
        if !self.takes(track, drag) {
            return None;
        }
        let length = drag.duration.filter(|d| *d > 0.0);
        let (start, snapped) = self.snap_span(asked, length.unwrap_or(0.0), &self.targets(""), pps);
        let at = start.max(0.0);
        let Some(length) = length else {
            return Some(Landing {
                track: track.id.clone(),
                start: at,
                duration: None,
                fits: true,
                snapped,
            });
        };
        let placed = place_clip(at, length, &self.neighbors(&track.id, ""));
        Some(Landing {
            track: track.id.clone(),
            start: placed.unwrap_or(at),
            duration: Some(length),
            fits: placed.is_some(),
            snapped: if placed.is_some_and(|p| (p - start).abs() < NEAR) {
                snapped
            } else {
                None
            },
        })
    }

    pub fn show(self, ui: &mut Ui) -> ClipTimelineResponse {
        let t = tokens(ui.ctx());
        let id = ui.make_persistent_id(self.id_salt);
        let mut out = ClipTimelineResponse::default();
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let inner = rect.shrink(1.0);
        let narrow = ui.ctx().content_rect().width() < 640.0;
        let header_width = if narrow { HEADER_WIDTH_NARROW } else { HEADER_WIDTH };
        let lanes_left = inner.left() + header_width;
        let lanes_top = inner.top() + RULER_HEIGHT;
        let room = (inner.right() - lanes_left).max(0.0);
        let room = if room > 0.0 { room } else { DEFAULT_ROOM };
        let lanes_view = Rect::from_min_max(pos2(lanes_left, lanes_top), inner.max);
        let ruler_view = Rect::from_min_max(pos2(lanes_left, inner.top()), pos2(inner.right(), lanes_top));
        let headers_view = Rect::from_min_max(pos2(inner.left(), lanes_top), pos2(lanes_left, inner.bottom()));

        let total = self
            .clips
            .iter()
            .map(TimelineClip::end)
            .fold(self.duration.max(0.0), f64::max);
        let fit = clip_fit_zoom(room, total);
        out.fit = fit;
        let own_key = id.with("own");
        let own: Option<f32> = ui.data(|d| d.get_temp(own_key));
        let pps = self.zoom.or(own).unwrap_or(fit).max(CLIP_TIMELINE_ZOOM_MIN);
        let width = (total as f32 * pps).max(room);
        let reach = (width / pps) as f64;
        let content_height = self.tracks.len() as f32 * ROW_HEIGHT;

        // the scroll, both ways, held in reach
        let scroll_key = id.with("scroll");
        let mut scroll: Vec2 = ui.data(|d| d.get_temp(scroll_key)).unwrap_or_default();
        let anchor_key = id.with("anchor");
        if let Some((time, x, from)) = ui.data(|d| d.get_temp::<(f64, f32, f32)>(anchor_key))
            && (from - pps).abs() > f32::EPSILON
        {
            scroll.x = time as f32 * pps - x;
            ui.data_mut(|d| d.remove::<(f64, f32, f32)>(anchor_key));
        }
        let hovered = ui.rect_contains_pointer(rect);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        if hovered {
            let (zoom, delta) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta()));
            if (zoom - 1.0).abs() > f32::EPSILON {
                let floor = (fit / 2.0).clamp(CLIP_TIMELINE_ZOOM_MIN, CLIP_TIMELINE_ZOOM_MAX);
                let next = (pps * zoom).clamp(floor, CLIP_TIMELINE_ZOOM_MAX);
                if (next - pps).abs() > 1e-6 {
                    let x = pointer.map_or(room / 2.0, |p| p.x - lanes_left);
                    let time = ((x + scroll.x) / pps) as f64;
                    if self.zoom_asks {
                        out.zoom = Some(next);
                        ui.data_mut(|d| d.insert_temp(anchor_key, (time, x, pps)));
                    } else if self.zoom.is_none() {
                        ui.data_mut(|d| d.insert_temp(own_key, next));
                        scroll.x = time as f32 * next - x;
                    }
                }
            } else {
                scroll -= delta;
            }
        }
        // the playhead in sight when first drawn, and as a key moves it
        let follow_key = id.with("follow");
        if ui.data(|d| d.get_temp::<bool>(follow_key)).unwrap_or(true) {
            let x = self.playhead as f32 * pps;
            if x < scroll.x {
                scroll.x = x - room / 4.0;
            } else if x > scroll.x + room {
                scroll.x = x - room * 3.0 / 4.0;
            }
            ui.data_mut(|d| d.insert_temp(follow_key, false));
        }
        let lanes_height = (inner.bottom() - lanes_top).max(0.0);
        scroll.x = scroll.x.clamp(0.0, (width - room).max(0.0));
        scroll.y = scroll.y.clamp(0.0, (content_height - lanes_height).max(0.0));
        ui.data_mut(|d| d.insert_temp(scroll_key, scroll));

        let x_of = |time: f64| lanes_left - scroll.x + time as f32 * pps;
        let time_at = |x: f32| ((x - lanes_left + scroll.x) / pps) as f64;
        let lane_top = |index: usize| lanes_top - scroll.y + index as f32 * ROW_HEIGHT;
        let lane_under = |p: Pos2| -> Option<&'a TimelineTrack> {
            if !lanes_view.contains(p) {
                return None;
            }
            let index = ((p.y - lanes_top + scroll.y) / ROW_HEIGHT).floor();
            if index < 0.0 {
                return None;
            }
            self.tracks.get(index as usize)
        };
        let seek = |time: f64| -> f64 { self.to_frame(time).clamp(0.0, self.to_frame(reach)) };

        let mut ours: Vec<Id> = Vec::new();
        let chosen: Vec<&TimelineClip> = self.clips.iter().filter(|c| self.selected.contains(&c.id)).collect();

        // the headers' toggles, first in the tab order
        let mut toggles: Vec<(Rect, Icon, bool, Response)> = Vec::new();
        if self.track_toggles {
            for (index, track) in self.tracks.iter().enumerate() {
                let top = lane_top(index);
                let name_line = 20.0;
                let block = name_line + 2.0 + t.density("control-xs");
                let row = top + ((ROW_HEIGHT - 1.0) - block) / 2.0 + name_line + 2.0;
                let side = t.density("control-xs");
                let x = inner.left() + 12.0 - 6.0;
                let first = if track.kind == ClipTrackKind::Audio {
                    let icon = if track.muted { Icon::VolumeX } else { Icon::Volume2 };
                    (icon, track.muted, &self.words.mute, TrackChange::Muted(!track.muted))
                } else {
                    let icon = if track.hidden { Icon::EyeOff } else { Icon::Eye };
                    (icon, track.hidden, &self.words.hide, TrackChange::Hidden(!track.hidden))
                };
                let lock = (
                    if track.locked { Icon::Lock } else { Icon::LockOpen },
                    track.locked,
                    &self.words.lock,
                    TrackChange::Locked(!track.locked),
                );
                for (n, (icon, on, label, change)) in [first, lock].into_iter().enumerate() {
                    let r = Rect::from_min_size(pos2(x + n as f32 * side, row), vec2(side, side));
                    let hit = r.intersect(headers_view);
                    let tid = id.with(("toggle", index, n));
                    ours.push(tid);
                    let response = ui.interact(hit, tid, Sense::click());
                    let label = label.clone();
                    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, &label));
                    if response.clicked() {
                        out.track_changes.push((track.id.clone(), change));
                    }
                    let response = response.on_hover_text(label);
                    toggles.push((r, icon, on, response));
                }
            }
        }

        // the ruler: a press or a drag moves the playhead
        let ruler = ui.interact(ruler_view, id.with("ruler"), Sense::click_and_drag());
        if ruler.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeColumn);
        }
        if (ruler.is_pointer_button_down_on() || ruler.dragged())
            && let Some(p) = ruler.interact_pointer_pos()
        {
            let at = seek(time_at(p.x));
            if (at - self.playhead).abs() > NEAR {
                out.playhead = Some(at);
            }
        }
        let head = self.playhead.clamp(0.0, reach);
        let knob = Rect::from_min_size(pos2(x_of(head) - 6.0, lanes_top - 16.0), vec2(12.0, 16.0));
        let knob_id = id.with("knob");
        ours.push(knob_id);
        let knob_response = ui.interact(knob.intersect(ruler_view), knob_id, Sense::focusable_noninteractive());
        let playhead_text = self.timecode(self.playhead);
        knob_response.widget_info(|| {
            let mut info = egui::WidgetInfo::labeled(egui::Role::Slider, true, &self.words.playhead);
            info.value = Some(self.playhead);
            info.current_text_value = Some(playhead_text.clone());
            info
        });

        // the lanes: a press on an empty part lets the chosen clips go
        for (index, track) in self.tracks.iter().enumerate() {
            let r =
                Rect::from_min_size(pos2(lanes_left, lane_top(index)), vec2(room, ROW_HEIGHT)).intersect(lanes_view);
            if r.height() <= 0.0 {
                continue;
            }
            let lane = ui.interact(r, id.with(("lane", &track.id)), Sense::click());
            if lane.clicked() && !self.selected.is_empty() {
                out.selected = Some(Vec::new());
            }
        }

        // the drag in hand
        let carry_key = id.with("carry");
        let mut carry: Option<Carry> = ui.data(|d| d.get_temp(carry_key));
        let pointer_time = pointer.map(|p| time_at(p.x));
        let mut proposal: Option<Proposal> = None;
        let mut card: Option<(String, Pos2)> = None;

        // the clips, then their grips over them
        struct Drawn<'c> {
            clip: &'c TimelineClip,
            track: &'c TimelineTrack,
            rect: Rect,
            response: Option<Response>,
            grip_hot: Option<Rect>,
        }
        let mut drawn: Vec<Drawn<'_>> = Vec::new();
        for (index, track) in self.tracks.iter().enumerate() {
            let top = lane_top(index);
            let last = index + 1 == self.tracks.len();
            let bottom = top + ROW_HEIGHT - if last { 0.0 } else { 1.0 };
            for clip in self.clips.iter().filter(|c| c.track == track.id) {
                let left = x_of(clip.start);
                let r = Rect::from_min_max(
                    pos2(left, top + CLIP_INSET),
                    pos2(
                        left + (clip.duration as f32 * pps).max(CLIP_MIN_WIDTH),
                        bottom - CLIP_INSET,
                    ),
                );
                let hit = r.intersect(lanes_view);
                let mut entry = Drawn {
                    clip,
                    track,
                    rect: r,
                    response: None,
                    grip_hot: None,
                };
                let cid = id.with(("clip", &clip.id));
                ours.push(cid);
                if hit.width() <= 0.0 || hit.height() <= 0.0 {
                    drawn.push(entry);
                    continue;
                }
                let changing = self.changes && !track.locked;
                let sense = if changing {
                    Sense::click_and_drag()
                } else {
                    Sense::click()
                };
                let response = ui.interact(hit, cid, sense);
                let label = fill(
                    &self.words.clip,
                    &[
                        ("label", &clip.label),
                        ("start", &self.timecode(clip.start)),
                        ("end", &self.timecode(clip.end())),
                    ],
                );
                let is_chosen = self.selected.contains(&clip.id);
                response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, is_chosen, &label));
                if response.clicked() {
                    response.request_focus();
                    let adding = ui.input(|i| i.modifiers.shift || i.modifiers.command || i.modifiers.ctrl);
                    let next = if !adding {
                        vec![clip.id.clone()]
                    } else if is_chosen {
                        self.selected.iter().filter(|s| **s != clip.id).cloned().collect()
                    } else {
                        let mut all = self.selected.to_vec();
                        all.push(clip.id.clone());
                        all
                    };
                    out.selected = Some(next);
                }
                let mut grip_hot = None;
                if changing {
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::Grab);
                    }
                    let mut started = response.drag_started().then_some(Grip::Move);
                    for (grip, side) in [(Grip::Start, r.left()), (Grip::End, r.right() - GRIP_WIDTH)] {
                        let g = Rect::from_min_size(pos2(side, r.top()), vec2(GRIP_WIDTH, r.height())).intersect(hit);
                        if g.width() <= 0.0 {
                            continue;
                        }
                        let gr = ui.interact(g, cid.with(("grip", grip as u8)), Sense::click_and_drag());
                        if gr.hovered() || gr.dragged() {
                            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                            grip_hot = Some(g);
                        }
                        if gr.clicked() {
                            out.selected = Some(vec![clip.id.clone()]);
                        }
                        if gr.drag_started() {
                            started = Some(grip);
                        }
                        if gr.dragged() || gr.drag_stopped() {
                            entry.response = Some(gr);
                        }
                    }
                    if let Some(grip) = started {
                        let from = ui.input(|i| i.pointer.press_origin()).map_or(0.0, |p| time_at(p.x));
                        let edge = if grip == Grip::End { clip.end() } else { clip.start };
                        carry = Some(Carry {
                            id: clip.id.clone(),
                            grip,
                            grab: from - edge,
                            cancelled: false,
                        });
                    }
                }
                entry.grip_hot = grip_hot;
                if entry.response.is_none() {
                    entry.response = Some(response);
                }
                drawn.push(entry);
            }
        }

        // the drag: where it would land, and its end
        if let Some(c) = carry.as_mut() {
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                c.cancelled = true;
            }
            let down = ui.input(|i| i.pointer.primary_down());
            if !c.cancelled
                && let Some(time) = pointer_time
            {
                proposal = self.propose(c, time, pointer.and_then(lane_under), pps);
            }
            if !down {
                if !c.cancelled
                    && let (Some(to), Some(clip)) = (proposal.as_ref(), self.clip(&c.id))
                {
                    if !to.fits {
                        out.said = Some(fill(&self.words.blocked, &[("clip", &clip.label)]));
                    } else if let Some(change) = self.change_of(to) {
                        out.said = Some(self.say_change(clip, &change));
                        out.changes.push(change);
                    }
                }
                carry = None;
                proposal = None;
            } else if let (Some(p), Some(to)) = (pointer, proposal.as_ref()) {
                let words = if to.grip == Grip::Move {
                    fill(&self.words.start, &[("at", &self.timecode(to.start))])
                } else {
                    fill(&self.words.length, &[("length", &self.timecode(to.duration))])
                };
                card = Some((words, p + vec2(CARD_OFFSET, CARD_OFFSET)));
            }
        }
        match &carry {
            Some(c) => ui.data_mut(|d| {
                d.insert_temp(carry_key, c.clone());
            }),
            None => ui.data_mut(|d| d.remove::<Carry>(carry_key)),
        }

        // a thing from elsewhere over a track
        let mut landing: Option<Landing> = None;
        if self.drops
            && let Some(data) = DragAndDrop::payload::<DragData>(ui.ctx())
            && let Some(p) = pointer
            && let Some(track) = lane_under(p)
        {
            landing = self.landing(&data, track, time_at(p.x), pps);
            if ui.input(|i| i.pointer.any_released())
                && let Some(to) = &landing
            {
                DragAndDrop::clear_payload(ui.ctx());
                if to.fits {
                    out.said = Some(fill(
                        &self.words.dropped,
                        &[
                            ("item", &data.label),
                            ("track", &track.name),
                            ("at", &self.timecode(to.start)),
                        ],
                    ));
                    out.dropped = Some(ClipDrop {
                        data: (*data).clone(),
                        track: to.track.clone(),
                        at: to.start,
                    });
                } else {
                    out.said = Some(fill(&self.words.blocked, &[("clip", &data.label)]));
                }
                landing = None;
            }
        }

        // the keys, while focus is inside the timeline
        let focused = ui.memory(|m| m.focused());
        if let Some(f) = focused.filter(|f| ours.contains(f)) {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    f,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: false,
                        tab: false,
                        escape: false,
                    },
                )
            });
            let (mods, keys) = ui.input(|i| {
                let keys: Vec<Key> = [
                    Key::ArrowLeft,
                    Key::ArrowRight,
                    Key::Home,
                    Key::End,
                    Key::S,
                    Key::Delete,
                    Key::Backspace,
                ]
                .into_iter()
                .filter(|k| i.key_pressed(*k))
                .collect();
                (i.modifiers, keys)
            });
            let command = mods.command || mods.ctrl || mods.mac_cmd;
            let by_key = |at: f64, out: &mut ClipTimelineResponse| {
                let at = seek(at);
                out.playhead = Some(at);
                out.said = Some(fill(&self.words.at, &[("at", &self.timecode(at))]));
            };
            for key in keys {
                match key {
                    Key::ArrowLeft | Key::ArrowRight if !command => {
                        let sign = if key == Key::ArrowLeft { -1.0 } else { 1.0 };
                        if mods.alt {
                            self.nudge(&chosen, sign * self.frame(), &mut out);
                        } else {
                            let by = if mods.shift { 1.0 } else { self.frame() };
                            by_key(self.playhead + sign * by, &mut out);
                            ui.data_mut(|d| d.insert_temp(follow_key, true));
                        }
                    }
                    Key::Home | Key::End if !command && !mods.alt => {
                        by_key(if key == Key::Home { 0.0 } else { total }, &mut out);
                        ui.data_mut(|d| d.insert_temp(follow_key, true));
                    }
                    Key::S if !command && !mods.alt && self.splits => {
                        let at = self.to_frame(self.playhead);
                        for clip in &chosen {
                            if self.locked(clip) || at <= clip.start + NEAR || at >= clip.end() - NEAR {
                                continue;
                            }
                            out.splits.push((clip.id.clone(), at));
                            out.said = Some(fill(
                                &self.words.split,
                                &[("clip", &clip.label), ("at", &self.timecode(at))],
                            ));
                        }
                    }
                    Key::Delete | Key::Backspace if !command && self.deletes && !chosen.is_empty() => {
                        let ids: Vec<String> = chosen
                            .iter()
                            .filter(|c| !self.locked(c))
                            .map(|c| c.id.clone())
                            .collect();
                        if !ids.is_empty() {
                            out.said = Some(self.words.deleted(ids.len(), mods.shift));
                            out.deleted = Some((ids, mods.shift));
                        }
                    }
                    _ => {}
                }
            }
        }

        if !ui.is_rect_visible(rect) {
            return out;
        }

        // painting: the box, the lanes and their clips, the ruler, the headers, the corner
        let painter = ui.painter_at(rect);
        let surface = t.radius("surface");
        let card_ground = t.color("card");
        let border = t.color("border");
        painter.rect_filled(rect, surface, card_ground);

        let lanes = painter.with_clip_rect(lanes_view);
        for (index, track) in self.tracks.iter().enumerate() {
            let top = lane_top(index);
            let lane = Rect::from_min_size(pos2(x_of(0.0), top), vec2(width, ROW_HEIGHT));
            if track.locked {
                lanes.rect_filled(lane, 0.0, t.color_at("muted", 0.5));
            }
            if index + 1 < self.tracks.len() {
                lanes.hline(lane.x_range(), lane.bottom() - 0.5, Stroke::new(1.0, border));
            }
        }
        // the chosen ones over the rest
        let order: Vec<&Drawn<'_>> = drawn
            .iter()
            .filter(|d| !self.selected.contains(&d.clip.id))
            .chain(drawn.iter().filter(|d| self.selected.contains(&d.clip.id)))
            .collect();
        for d in order {
            let faded = proposal.as_ref().is_some_and(|p| p.id == d.clip.id) || d.track.hidden || d.track.muted;
            self.paint_clip(ui, &lanes, &t, d.clip, d.track, d.rect, faded);
            if let Some(g) = d.grip_hot {
                let color = d.clip.color.unwrap_or(d.track.kind.default_color());
                let ink = t.color(&format!("kind-{}-ink", color.key()));
                lanes.rect_filled(g, 0.0, ink.gamma_multiply(0.25));
            }
            if self.selected.contains(&d.clip.id) {
                lanes.rect_stroke(
                    d.rect,
                    t.radius("control"),
                    Stroke::new(2.0, t.color("primary")),
                    StrokeKind::Outside,
                );
            }
            if let Some(r) = &d.response
                && r.has_focus()
            {
                let width = t.length("focus-ring-width");
                let offset = t.length("focus-ring-offset");
                lanes.rect_stroke(
                    d.rect.expand(offset),
                    t.radius("control"),
                    Stroke::new(width, t.color("ring")),
                    StrokeKind::Outside,
                );
            }
        }
        // where a drag would land
        let landing_box = |track: &str, start: f64, length: Option<f64>, color: Color32, wash: bool| {
            let Some(index) = self.tracks.iter().position(|t| t.id == track) else {
                return;
            };
            let top = lane_top(index);
            let last = index + 1 == self.tracks.len();
            let bottom = top + ROW_HEIGHT - if last { 0.0 } else { 1.0 };
            let (left, right) = match length {
                Some(l) => (x_of(start), x_of(start + l)),
                None => (x_of(start) - 2.0, x_of(start) + 2.0),
            };
            let r = Rect::from_min_max(pos2(left, top + CLIP_INSET), pos2(right, bottom - CLIP_INSET));
            if wash {
                lanes.rect_filled(r, t.radius("control"), t.color_at("primary", 0.1));
            }
            dashed_round_rect(&lanes, r, t.radius("control"), Stroke::new(2.0, color));
        };
        if let Some(p) = &proposal {
            let color = if p.fits {
                t.color("primary")
            } else {
                t.color("destructive")
            };
            landing_box(&p.track, p.start, Some(p.duration), color, false);
        }
        if let Some(l) = &landing {
            let color = if l.fits {
                t.color("primary")
            } else {
                t.color("destructive")
            };
            landing_box(&l.track, l.start, l.duration, color, l.fits);
        }
        if let Some(at) = proposal
            .as_ref()
            .and_then(|p| p.snapped)
            .or(landing.as_ref().and_then(|l| l.snapped))
        {
            lanes.vline(x_of(at), lanes_view.y_range(), Stroke::new(1.0, t.color("ring")));
        }
        let head_x = x_of(head);
        lanes.vline(head_x, lanes_view.y_range(), Stroke::new(1.0, t.color("primary")));

        // the ruler
        let rule = painter.with_clip_rect(ruler_view);
        rule.rect_filled(ruler_view, 0.0, card_ground);
        let step = clip_tick_step(pps, self.fps);
        let minor = step / 5.0;
        if minor * pps as f64 >= MINOR_TICK_MIN {
            let mut index = 1;
            while index as f64 * minor <= reach + NEAR {
                if index % 5 != 0 {
                    let x = x_of(index as f64 * minor);
                    rule.rect_filled(
                        Rect::from_min_size(pos2(x, lanes_top - 1.0 - 6.0), vec2(1.0, 6.0)),
                        0.0,
                        border,
                    );
                }
                index += 1;
            }
        }
        let tiny = t.text("2xs");
        let tick_font = mono_font(ui.ctx(), mono_size(&t, tiny.size), tiny.weight);
        let tick_ink = t.color("muted-foreground");
        let mut index = 0;
        while index as f64 * step <= reach + NEAR {
            let at = index as f64 * step;
            let x = x_of(at);
            if x > ruler_view.left() - 120.0 && x < ruler_view.right() {
                rule.rect_filled(
                    Rect::from_min_size(pos2(x, inner.top()), vec2(1.0, RULER_HEIGHT - 1.0)),
                    0.0,
                    t.color("border-strong"),
                );
                let galley = rule.layout_no_wrap(self.timecode(at), tick_font.clone(), tick_ink);
                rule.galley(pos2(x + 5.0, inner.top() + 4.0), galley, tick_ink);
            }
            index += 1;
        }
        rule.vline(
            head_x,
            inner.top()..=lanes_top - 1.0,
            Stroke::new(1.0, t.color("primary")),
        );
        let mini = t.radius("mini");
        let knob_radius = CornerRadius {
            nw: 0,
            ne: 0,
            sw: mini.sw,
            se: mini.se,
        };
        let knob = Rect::from_min_size(pos2(head_x - 6.0, lanes_top - 1.0 - 16.0), vec2(12.0, 16.0));
        rule.rect_filled(knob, knob_radius, t.color("primary"));
        if knob_response.has_focus() {
            let width = t.length("focus-ring-width");
            let offset = t.length("focus-ring-offset");
            painter.rect_stroke(
                knob.expand(offset),
                knob_radius,
                Stroke::new(width, t.color("ring")),
                StrokeKind::Outside,
            );
        }
        painter.hline(ruler_view.x_range(), lanes_top - 0.5, Stroke::new(1.0, border));

        // the headers' column
        let side = painter.with_clip_rect(headers_view);
        side.rect_filled(headers_view, 0.0, card_ground);
        let name_role = TextRole::Badge;
        for (index, track) in self.tracks.iter().enumerate() {
            let top = lane_top(index);
            let last = index + 1 == self.tracks.len();
            let block = 20.0
                + if self.track_toggles {
                    2.0 + t.density("control-xs")
                } else {
                    0.0
                };
            let line_center = top + ((ROW_HEIGHT - if last { 0.0 } else { 1.0 }) - block) / 2.0 + 10.0;
            let x = inner.left() + 12.0;
            icons::paint(
                &side,
                pos2(x + 7.0, line_center),
                track.kind.icon(),
                14.0,
                t.color("muted-foreground"),
            );
            let name_room = (header_width - 1.0 - 24.0 - 14.0 - 6.0).max(0.0);
            let name = WidgetText::from(name_role.rich(&t, &track.name)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                name_room,
                name_role.font(&t),
            );
            side.galley(
                pos2(x + 14.0 + 6.0, line_center - name.size().y / 2.0),
                name,
                t.color("foreground"),
            );
            if !last {
                side.hline(
                    inner.left()..=lanes_left,
                    top + ROW_HEIGHT - 0.5,
                    Stroke::new(1.0, border),
                );
            }
        }
        for (r, icon, on, response) in &toggles {
            let radius = CornerRadius::same(t.radius("control").nw.min(10));
            let hot = response.hovered();
            if *on {
                side.rect_filled(*r, radius, t.color("muted"));
            } else if hot {
                side.rect_filled(*r, radius, t.color("hover"));
            }
            let ink = if *on || hot {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            icons::paint(&side, r.center(), *icon, 12.0, ink);
            super::focus_ring(&painter, &t, *r, radius, response);
        }
        painter.vline(lanes_left - 0.5, inner.y_range(), Stroke::new(1.0, border));

        // the corner: the playhead's timecode
        let corner = Rect::from_min_max(inner.min, pos2(lanes_left - 1.0, lanes_top - 1.0));
        painter.rect_filled(corner, 0.0, card_ground);
        let help = t.text("help");
        let corner_font = mono_font(ui.ctx(), mono_size(&t, help.size), help.weight);
        let galley = painter.layout_no_wrap(self.timecode(self.playhead), corner_font, t.color("foreground"));
        painter.galley(
            pos2(corner.left() + 12.0, corner.center().y - galley.size().y / 2.0),
            galley,
            t.color("foreground"),
        );
        painter.hline(inner.left()..=lanes_left, lanes_top - 0.5, Stroke::new(1.0, border));
        painter.rect_stroke(rect, surface, Stroke::new(1.0, border), StrokeKind::Inside);

        // the card at the pointer while a clip is carried
        if let Some((words, at)) = card {
            egui::Area::new(id.with("card"))
                .order(egui::Order::Tooltip)
                .fixed_pos(at)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    egui::Frame::new()
                        .fill(card_ground)
                        .stroke(Stroke::new(1.0, border))
                        .corner_radius(surface)
                        .shadow(t.shadow("elevation-pop"))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            let font = mono_font(ui.ctx(), mono_size(&t, help.size), help.weight);
                            ui.label(egui::RichText::new(words).font(font).color(t.color("foreground")));
                        });
                });
        }
        out
    }

    /// Moves the chosen clips a frame along their tracks, all or none.
    fn nudge(&self, chosen: &[&TimelineClip], by: f64, out: &mut ClipTimelineResponse) {
        if !self.changes {
            return;
        }
        let mut changes = Vec::new();
        for clip in chosen {
            let start = self.to_frame(clip.start + by);
            let others = self.neighbors(&clip.track, &clip.id);
            if self.locked(clip) || start < -NEAR || place_clip(start, clip.duration, &others) != Some(start.max(0.0)) {
                out.said = Some(fill(&self.words.blocked, &[("clip", &clip.label)]));
                return;
            }
            changes.push(ClipChange {
                id: clip.id.clone(),
                start: Some(start.max(0.0)),
                ..ClipChange::default()
            });
        }
        if changes.is_empty() {
            return;
        }
        if chosen.len() == 1 {
            out.said = Some(self.say_change(chosen[0], &changes[0]));
        }
        out.changes.extend(changes);
    }

    /// A clip: its wash, edge (dashed for a title), its waveform and its label.
    #[allow(clippy::too_many_arguments)]
    fn paint_clip(
        &self,
        ui: &Ui,
        painter: &Painter,
        t: &Tokens,
        clip: &TimelineClip,
        track: &TimelineTrack,
        rect: Rect,
        faded: bool,
    ) {
        let color = clip.color.unwrap_or(track.kind.default_color());
        let key = format!("kind-{}", color.key());
        let strength = if faded { 0.5 } else { 1.0 };
        let wash = t.color_at(&key, 0.25).gamma_multiply(strength);
        let edge = t.color_at(&key, 0.6).gamma_multiply(strength);
        let ink = t.color(&format!("{key}-ink")).gamma_multiply(strength);
        let radius = t.radius("control");
        painter.rect_filled(rect, radius, wash);
        if track.kind == ClipTrackKind::Title {
            dashed_round_rect(painter, rect, radius, Stroke::new(1.0, edge));
        } else {
            painter.rect_stroke(rect, radius, Stroke::new(1.0, edge), StrokeKind::Inside);
        }
        let inside = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        if track.kind == ClipTrackKind::Audio
            && let Some(peaks) = &clip.waveform
            && peaks.len() > 1
        {
            let area = Rect::from_min_max(
                pos2(rect.left() + 1.0, rect.top() + 1.0 + 20.0),
                pos2(rect.right() - 1.0, rect.bottom() - 1.0 - 4.0),
            );
            paint_wave(&inside, area, peaks, clip, ink.gamma_multiply(0.6));
        }
        let role = TextRole::Badge;
        let mut x = rect.left() + 1.0 + 8.0;
        let line_top = rect.top() + 1.0 + 4.0;
        let line_center = line_top + 8.0;
        if track.kind == ClipTrackKind::Title {
            icons::paint(&inside, pos2(x + 6.0, line_center), Icon::Type, 12.0, ink);
            x += 12.0 + 4.0;
        }
        let room = (rect.right() - 1.0 - 8.0 - x).max(0.0);
        if room > 0.0 {
            let label = WidgetText::from(role.rich(t, &clip.label).line_height(Some(16.0))).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                room,
                role.font(t),
            );
            inside.galley(pos2(x, line_center - label.size().y / 2.0), label, ink);
        }
    }
}

/// An audio clip's loudness for the part of its source it plays, mirrored about the middle and
/// stretched over `area`.
fn paint_wave(painter: &Painter, area: Rect, peaks: &[f32], clip: &TimelineClip, color: Color32) {
    let source = clip.source_duration.unwrap_or(clip.in_point + clip.duration);
    if source <= 0.0 || area.width() <= 0.0 {
        return;
    }
    let len = peaks.len();
    let from = ((clip.in_point / source) * len as f64).floor().max(0.0) as usize;
    let to = (((clip.in_point + clip.duration) / source) * len as f64)
        .ceil()
        .max((from + 2) as f64) as usize;
    let shown = &peaks[from.min(len)..to.min(len)];
    if shown.len() < 2 {
        return;
    }
    let last = (shown.len() - 1) as f32;
    let half = |p: f32| p.clamp(0.0, 1.0) * 0.5;
    let mut mesh = Mesh::default();
    for (i, p) in shown.iter().enumerate() {
        let x = area.left() + area.width() * i as f32 / last;
        let h = half(*p) * area.height();
        mesh.colored_vertex(pos2(x, area.center().y - h), color);
        mesh.colored_vertex(pos2(x, area.center().y + h), color);
        if i > 0 {
            let a = (2 * (i - 1)) as u32;
            mesh.add_triangle(a, a + 1, a + 2);
            mesh.add_triangle(a + 1, a + 3, a + 2);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// A rounded rectangle's edge drawn dashed, as `border-dashed` draws it.
fn dashed_round_rect(painter: &Painter, rect: Rect, radius: CornerRadius, stroke: Stroke) {
    let inset = stroke.width / 2.0;
    let r = rect.shrink(inset);
    let rad = (radius.nw as f32 - inset)
        .max(0.0)
        .min(r.width() / 2.0)
        .min(r.height() / 2.0);
    let mut points = Vec::new();
    let corner = |points: &mut Vec<Pos2>, center: Pos2, from: f32| {
        for k in 0..=4 {
            let a = from + k as f32 / 4.0 * std::f32::consts::FRAC_PI_2;
            points.push(center + rad * vec2(a.cos(), a.sin()));
        }
    };
    use std::f32::consts::PI;
    corner(&mut points, pos2(r.right() - rad, r.top() + rad), -PI / 2.0);
    corner(&mut points, pos2(r.right() - rad, r.bottom() - rad), 0.0);
    corner(&mut points, pos2(r.left() + rad, r.bottom() - rad), PI / 2.0);
    corner(&mut points, pos2(r.left() + rad, r.top() + rad), PI);
    points.push(points[0]);
    let dash = (stroke.width * 3.0).max(3.0);
    painter.extend(Shape::dashed_line(&points, stroke, dash, dash));
}
