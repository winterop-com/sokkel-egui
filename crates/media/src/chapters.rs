//! Chapters as the seconds each one starts at, in order from the start of what plays, after
//! sokkel-ui-media's `chapters.ts` (and `chapterStarts` in `ChapterList.tsx`): the chapter playing,
//! where previous and next go, a chapter's span, where a seek lands and the waveform under a span.
//! The first chapter starts where its start says; anything before it belongs to no chapter.

use serde::Serialize;

/// One chapter of what plays: its title and the second it starts at.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MediaChapter {
    pub title: String,
    pub start: f64,
}

/// How long into a chapter previous goes back to its start rather than to the chapter before.
pub const RESTART_AFTER: f64 = 3.0;

/// The seconds each chapter starts at.
pub fn chapter_starts(chapters: &[MediaChapter]) -> Vec<f64> {
    chapters.iter().map(|chapter| chapter.start).collect()
}

/// The chapter playing at `position`: the last one started by then, or -1 before the first.
pub fn chapter_index(starts: &[f64], position: f64) -> i64 {
    let mut found = -1;
    for (index, start) in starts.iter().enumerate() {
        if *start <= position {
            found = index as i64;
        } else {
            break;
        }
    }
    found
}

/// Where chapter `index` starts and ends: at the next chapter's start, or at `duration` for the last.
pub fn chapter_span(starts: &[f64], index: usize, duration: f64) -> (f64, f64) {
    let start = starts.get(index).copied().unwrap_or(0.0);
    let end = starts.get(index + 1).copied().unwrap_or(duration);
    (start, start.max(end))
}

/// Where previous goes: the start of the chapter playing, or the start of the one before it within
/// the first `RESTART_AFTER` seconds of a chapter. Before the first chapter, the start of what plays.
pub fn previous_chapter_start(starts: &[f64], position: f64) -> f64 {
    let index = chapter_index(starts, position);
    if index < 0 {
        return 0.0;
    }
    let index = index as usize;
    if position - starts[index] > RESTART_AFTER || index == 0 {
        starts[index]
    } else {
        starts[index - 1]
    }
}

/// Where next goes: the start of the chapter after the one playing, or none in the last.
pub fn next_chapter_start(starts: &[f64], position: f64) -> Option<f64> {
    starts.get((chapter_index(starts, position) + 1) as usize).copied()
}

/// Where a seek to `seconds` lands: no earlier than the start, and no later than `duration` once
/// it is known, a finite length above zero. Nowhere when `seconds` is not a finite number.
pub fn seek_point(seconds: f64, duration: f64) -> Option<f64> {
    if !seconds.is_finite() {
        return None;
    }
    let from = seconds.max(0.0);
    Some(if duration.is_finite() && duration > 0.0 {
        duration.min(from)
    } else {
        from
    })
}

/// The part of `waveform`, levels across the whole of what plays, that falls between `start` and `end`.
pub fn waveform_span(waveform: &[f32], duration: f64, start: f64, end: f64) -> &[f32] {
    if duration <= 0.0 || waveform.is_empty() {
        return waveform;
    }
    let count = waveform.len() as f64;
    let from = ((start / duration) * count).floor().max(0.0) as usize;
    let to = ((end / duration) * count).ceil().max(0.0) as usize;
    let from = from.min(waveform.len());
    &waveform[from..to.max(from + 1).min(waveform.len())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_through_chapters_as_the_web_does() {
        let starts = [0.0, 29.0, 53.0];
        assert_eq!(chapter_index(&starts, 30.0), 1);
        assert_eq!(chapter_index(&[5.0], 1.0), -1);
        // within three seconds of a chapter's start, back to the one before it
        assert_eq!(previous_chapter_start(&starts, 31.0), 0.0);
        assert_eq!(previous_chapter_start(&starts, 35.0), 29.0);
        assert_eq!(previous_chapter_start(&starts, 2.0), 0.0);
        assert_eq!(previous_chapter_start(&starts, 54.0), 29.0);
        assert_eq!(next_chapter_start(&starts, 30.0), Some(53.0));
        assert_eq!(next_chapter_start(&starts, 60.0), None);
        assert_eq!(chapter_span(&starts, 2, 100.0), (53.0, 100.0));
        assert_eq!(seek_point(-3.0, 100.0), Some(0.0));
        assert_eq!(seek_point(f64::NAN, 100.0), None);
    }

    #[test]
    fn takes_the_waveform_under_a_span() {
        let levels = [0.1, 0.2, 0.3, 0.4];
        assert_eq!(waveform_span(&levels, 100.0, 50.0, 100.0), &[0.3, 0.4]);
        assert_eq!(waveform_span(&levels, 100.0, 50.0, 50.0), &[0.3]);
        assert_eq!(waveform_span(&levels, 0.0, 0.0, 1.0), &levels);
    }
}
