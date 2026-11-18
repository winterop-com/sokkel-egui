//! The edit's sound, as the web's `sound.ts` plays and mixes it: while the edit plays forward at its
//! own speed, every sound clip on a track that is not muted is scheduled from where the playhead
//! stands, at the clip's volume under the master level. The export mixes the same clips into two
//! channels.
//!
//! The web schedules through Web Audio; `scheduled` says what it schedules (each clip's wait, the
//! second of its source it starts at, how long it plays and how loud), for whatever plays the
//! samples here. `mixdown` is the web's to the sample.

use tidevann_shared::clips::SOUND_RATE;

use crate::js::round;
use crate::media::samples_of;
use crate::model::{Clip, Edit, TrackKind, end};

/// How far ahead of now the first sound is scheduled, in seconds, so it starts on time.
pub const LEAD: f64 = 0.03;

/// The sound clips that play: on a sound track that is not muted.
pub fn heard(edit: &Edit) -> Vec<&Clip> {
    let open: Vec<&str> = edit
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Audio && !track.is_muted())
        .map(|track| track.id.as_str())
        .collect();
    edit.clips
        .iter()
        .filter(|clip| open.contains(&clip.track.as_str()) && clip.source.is_some())
        .collect()
}

/// One sound clip as it is scheduled.
#[derive(Clone, Debug, PartialEq)]
pub struct Scheduled {
    pub clip: String,
    pub source: &'static str,
    /// How long after the `LEAD` it starts, in seconds.
    pub after: f64,
    /// The second of its source it starts at.
    pub offset: f64,
    /// How long it plays, in seconds.
    pub duration: f64,
    /// How loud, from 0 to 1, under the master level.
    pub gain: f64,
}

/// What plays when the edit's sound plays from second `from`.
pub fn scheduled(edit: &Edit, from: f64) -> Vec<Scheduled> {
    heard(edit)
        .into_iter()
        .filter(|clip| end(clip) > from)
        .filter_map(|clip| {
            let source = clip.source()?;
            let skipped = (from - clip.start).max(0.0);
            Some(Scheduled {
                clip: clip.id.clone(),
                source: source.id,
                after: (clip.start - from).max(0.0),
                offset: clip.in_point + skipped,
                duration: clip.duration - skipped,
                gain: clip.volume.unwrap_or(1.0),
            })
        })
        .collect()
}

/// The edit's sound mixed into two channels at `SOUND_RATE`, `length` seconds long, for the export.
pub fn mixdown(edit: &Edit, length: f64) -> (Vec<f32>, Vec<f32>) {
    let rate = f64::from(SOUND_RATE);
    let frames = (length * rate).ceil().max(0.0) as usize;
    let mut left = vec![0.0f32; frames];
    let mut right = vec![0.0f32; frames];
    for clip in heard(edit) {
        let Some(source) = clip.source() else {
            continue;
        };
        let samples = samples_of(source);
        let (from_left, from_right) = (&samples.0, &samples.1);
        let volume = clip.volume.unwrap_or(1.0);
        let at = round(clip.start * rate) as i64;
        let from = round(clip.in_point * rate) as i64;
        let count = (round(clip.duration * rate) as i64)
            .min(from_left.len() as i64 - from)
            .min(frames as i64 - at);
        for index in 0..count.max(0) {
            let (to, of) = ((at + index) as usize, (from + index) as usize);
            left[to] = (f64::from(left[to]) + f64::from(from_left[of]) * volume) as f32;
            right[to] = (f64::from(right[to]) + f64::from(from_right[of]) * volume) as f32;
        }
    }
    for value in left.iter_mut().chain(right.iter_mut()) {
        *value = value.clamp(-1.0, 1.0);
    }
    (left, right)
}
