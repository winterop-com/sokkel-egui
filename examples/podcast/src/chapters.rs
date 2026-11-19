//! Chapters as the seconds each one starts at, as sokkel-ui-media's `chapters.ts` (and
//! `chapterStarts` in `ChapterList.tsx`) keeps them: the chapter playing, where previous and next
//! go, a chapter's span and where a seek lands. Carried here until the media add-on has them.

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
