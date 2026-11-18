//! The playhead and how it moves, as the web's `editor/use-playback.ts` moves it: still, or played
//! at a rate (backward with a negative one) from where it stood, on the frame clock. The sound plays
//! with it while it plays forward at its own speed, and starts again from the playhead whenever the
//! edit changes under it.
//!
//! The web's hook reacts to its state after each render: a new rate, length or loop setting sets
//! the moment play is measured from, and a new rate or edit starts or stops the sound. Here each
//! call takes the moment it is made at (`now`, in milliseconds on the frame clock) and does the same
//! at once. What the sound should do is handed over as `Cue`s, which the editor takes and plays
//! with `sound::scheduled`.

use crate::model::{FRAME, on_frame};

/// The fastest J and L play, each press doubling the speed up to it.
pub const FASTEST: f64 = 4.0;

/// What the sound should do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cue {
    /// Play the edit's sound from this second, stopping what plays.
    Play { from: f64 },
    /// Stop every sound.
    Stop,
}

#[derive(Clone, Debug)]
pub struct Playback {
    pub playhead: f64,
    /// 0 while still; 1 plays forward at its own speed, -1 backward, 2 and 4 faster.
    pub rate: f64,
    /// Where play was measured from: the second, and the moment on the frame clock.
    anchor: (f64, f64),
    /// The longest the playhead runs: the edit's length, at least a frame.
    limit: f64,
    looping: bool,
    /// The rate, limit and loop setting as the last change saw them.
    seen: (f64, f64, bool),
    cues: Vec<Cue>,
}

impl Default for Playback {
    fn default() -> Self {
        Self::new(0.0, false)
    }
}

impl Playback {
    /// Still at the start, over an edit `length` seconds long.
    pub fn new(length: f64, looping: bool) -> Self {
        let limit = length.max(FRAME);
        Self {
            playhead: 0.0,
            rate: 0.0,
            anchor: (0.0, 0.0),
            limit,
            looping,
            seen: (0.0, limit, looping),
            cues: vec![Cue::Stop],
        }
    }

    pub fn playing(&self) -> bool {
        self.rate != 0.0
    }

    /// The longest the playhead runs.
    pub fn limit(&self) -> f64 {
        self.limit
    }

    /// What the sound should do since they were last taken.
    pub fn take_cues(&mut self) -> Vec<Cue> {
        std::mem::take(&mut self.cues)
    }

    /// Does what the web's effects do after a render: measures play from here when the rate,
    /// length or loop changed while playing, and starts or stops the sound when the rate or the
    /// edit changed.
    fn react(&mut self, now: f64, edit_changed: bool) {
        let (rate, limit, looping) = self.seen;
        let rate_changed = rate.to_bits() != self.rate.to_bits();
        let measured = rate_changed || limit.to_bits() != self.limit.to_bits() || looping != self.looping;
        if measured && self.rate != 0.0 {
            self.anchor = (self.playhead, now);
        }
        if rate_changed || edit_changed {
            self.cues.push(if self.rate == 1.0 {
                Cue::Play { from: self.playhead }
            } else {
                Cue::Stop
            });
        }
        self.seen = (self.rate, self.limit, self.looping);
    }

    /// What the editor tells it as the edit or the settings change: the edit's length, whether play
    /// loops, and whether the edit itself changed.
    pub fn update(&mut self, now: f64, length: f64, looping: bool, edit_changed: bool) {
        self.limit = length.max(FRAME);
        self.looping = looping;
        self.react(now, edit_changed);
    }

    /// Moves the playhead for the frame drawn at `now`, while it plays.
    pub fn frame(&mut self, now: f64) {
        if self.rate == 0.0 {
            return;
        }
        let mut t = self.anchor.0 + ((now - self.anchor.1) / 1000.0) * self.rate;
        if self.rate > 0.0 && t >= self.limit {
            if self.looping && self.rate == 1.0 {
                self.anchor = (0.0, now);
                t = 0.0;
                self.cues.push(Cue::Play { from: 0.0 });
            } else {
                self.playhead = on_frame(self.limit);
                self.rate = 0.0;
                self.react(now, false);
                return;
            }
        }
        if self.rate < 0.0 && t <= 0.0 {
            self.playhead = 0.0;
            self.rate = 0.0;
            self.react(now, false);
            return;
        }
        self.playhead = t;
    }

    /// Puts the playhead at `seconds`, held to the edit.
    pub fn seek(&mut self, seconds: f64, now: f64) {
        let t = seconds.min(self.limit).max(0.0);
        self.anchor = (t, now);
        self.playhead = t;
        if self.rate == 1.0 {
            self.cues.push(Cue::Play { from: t });
        }
    }

    /// Plays from the playhead, from the start again once at the end; or stops.
    pub fn toggle(&mut self, now: f64) {
        if self.rate != 0.0 {
            self.rate = 0.0;
        } else {
            if self.playhead >= self.limit - FRAME / 2.0 {
                self.playhead = 0.0;
            }
            self.rate = 1.0;
        }
        self.react(now, false);
    }

    /// K: stops, on a frame.
    pub fn pause(&mut self, now: f64) {
        self.rate = 0.0;
        self.playhead = on_frame(self.playhead);
        self.react(now, false);
    }

    /// L: plays forward, faster on each press while it already does.
    pub fn forward(&mut self, now: f64) {
        self.rate = if self.rate <= 0.0 {
            1.0
        } else {
            FASTEST.min(self.rate * 2.0)
        };
        self.react(now, false);
    }

    /// J: plays backward, faster on each press while it already does.
    pub fn backward(&mut self, now: f64) {
        self.rate = if self.rate >= 0.0 {
            -1.0
        } else {
            (-FASTEST).max(self.rate * 2.0)
        };
        self.react(now, false);
    }

    /// Steps by whole frames, stopping play.
    pub fn step(&mut self, frames: f64, now: f64) {
        self.rate = 0.0;
        self.playhead = on_frame(self.limit)
            .min(on_frame(self.playhead + frames * FRAME))
            .max(0.0);
        self.react(now, false);
    }
}

/// The speed a playback rate is written as beside the transport, `2x` or `-4x`; none at its own
/// speed or still.
pub fn rate_label(rate: f64) -> Option<String> {
    (rate != 0.0 && rate != 1.0).then(|| {
        format!(
            "{}{}x",
            if rate < 0.0 { "-" } else { "" },
            crate::js::number_to_string(rate.abs())
        )
    })
}
