//! A sleep timer for a long listen, as sokkel-ui-media's `SleepTimer.tsx` decides it: when
//! playback stops, and the pure steps `useSleepTimer` takes, telling playing on from a seek and a
//! chapter's end from a jump. The timer's state as the hook keeps it is in `state`. Carried here
//! until the media add-on has them.

use serde::{Serialize, Serializer};

use crate::chapters::chapter_index;

/// When playback stops: after a number of minutes of playing, at the end of the chapter playing,
/// or at the end of what plays. None is never.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SleepTimer {
    Minutes(f64),
    Chapter,
    End,
}

impl Serialize for SleepTimer {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SleepTimer::Minutes(minutes) => serializer.serialize_f64(*minutes),
            SleepTimer::Chapter => serializer.serialize_str("chapter"),
            SleepTimer::End => serializer.serialize_str("end"),
        }
    }
}

/// The minutes the menu offers unless told others.
pub const SLEEP_MINUTES: [f64; 6] = [5.0, 10.0, 15.0, 30.0, 45.0, 60.0];

/// The fastest a listener plays, and the seconds a report may lag, for telling playing on from a seek.
const FASTEST: f64 = 4.0;
const LAG: f64 = 1.0;

/// Whether a position report `seconds` after the last plays on from `from` to `to`, as playback
/// does, rather than seeking: forward, and no further than playing at any speed could go in that time.
pub fn played_on(from: f64, to: f64, seconds: f64) -> bool {
    let step = to - from;
    step >= 0.0 && step <= seconds.max(0.0) * FASTEST + LAG
}

/// A timer to the end of the chapter, after a position report: whether the chapter it waited on
/// has ended, played on from it into a later one, and else the chapter it waits on now.
pub fn chapter_timer_step(
    chapters: &[f64],
    anchor: i64,
    from: Option<f64>,
    to: f64,
    seconds: f64,
    playing: bool,
) -> (bool, i64) {
    let index = chapter_index(chapters, to);
    if index == anchor {
        return (false, anchor);
    }
    let ended = playing && index > anchor && from.is_some_and(|from| played_on(from, to, seconds));
    (ended, index)
}
