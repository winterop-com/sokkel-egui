//! A clip as the player's source, as the web's `clips/source.ts` tells its stats: which of the
//! renditions plays, at the bitrate it came out at, over a link whose speed is simulated, as a
//! streaming engine would tell them.

use serde::Serialize;

use super::{Clip, RENDITIONS};
use crate::js::round;

/// What the simulated link carries, in bits per second, wandering slowly around 24 Mbps.
pub fn bandwidth_at(seconds: f64) -> f64 {
    round(24_000_000.0 + 4_000_000.0 * (seconds / 9.0).sin() + 1_500_000.0 * (seconds / 2.3).sin())
}

/// The level that plays, as an engine counts its levels from the lowest.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Level {
    pub index: usize,
    pub count: usize,
    pub bitrate: f64,
    pub width: u32,
    pub height: u32,
}

/// What the source tells the stats overlay.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SourceStats {
    pub engine: &'static str,
    pub bandwidth: f64,
    pub level: Level,
    pub live: bool,
}

/// The clip's stats at `seconds` on the page's clock.
pub fn stats_of(clip: &Clip, seconds: f64) -> SourceStats {
    let index = RENDITIONS
        .iter()
        .position(|one| one.id == clip.rendition.id)
        .unwrap_or(0);
    SourceStats {
        engine: "Tidevann",
        bandwidth: bandwidth_at(seconds),
        level: Level {
            index: RENDITIONS.len() - 1 - index,
            count: RENDITIONS.len(),
            bitrate: clip.bitrate,
            width: clip.rendition.width,
            height: clip.rendition.height,
        },
        live: false,
    }
}
