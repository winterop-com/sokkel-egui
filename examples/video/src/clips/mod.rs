//! Each video's clip, as the web's `clips/index.ts` keeps them: the renditions a clip is offered
//! at, the scene it is painted from (`spec_of`), its poster's moment and its captions as WebVTT.
//!
//! The web makes each clip once, in a worker, by drawing every frame of the scene on a canvas and
//! encoding it with its sound into a WebM file (`clips/worker.ts`), keeps the files between visits
//! in IndexedDB (`clips/kept.ts`), and plays them in a video element. Here nothing is encoded:
//! the picture is painted from the same scene at the second being shown
//! (`tidevann_shared::clips::draw_frame`) and the sound made from it
//! (`tidevann_shared::clips::synthesize`), so a clip is ready at once at any rendition. What the
//! web reads from the made file, its size, is `Clip::bytes`: the rendition's bitrate over the
//! video's length.

pub mod source;

use serde::{Deserialize, Serialize};
use tidevann_shared::clips::SceneSpec;

use crate::catalog::{Video, channel_of, seed_of};
use crate::js::{number_to_string, round};

pub use tidevann_shared::clips::{SOUND_RATE, draw_frame, synthesize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RenditionId {
    #[serde(rename = "720")]
    P720,
    #[serde(rename = "480")]
    P480,
    #[serde(rename = "360")]
    P360,
    #[serde(rename = "240")]
    P240,
}

impl RenditionId {
    pub fn id(self) -> &'static str {
        match self {
            RenditionId::P720 => "720",
            RenditionId::P480 => "480",
            RenditionId::P360 => "360",
            RenditionId::P240 => "240",
        }
    }

    pub fn from_id(id: &str) -> Option<RenditionId> {
        RENDITIONS.iter().map(|one| one.id).find(|one| one.id() == id)
    }
}

/// One rendition a clip is made at: its picture's size and the bitrate it is encoded at.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Rendition {
    pub id: RenditionId,
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
    pub bitrate: f64,
}

/// The renditions offered, best first.
pub const RENDITIONS: [Rendition; 4] = [
    Rendition {
        id: RenditionId::P720,
        label: "720p",
        width: 1280,
        height: 720,
        bitrate: 1_600_000.0,
    },
    Rendition {
        id: RenditionId::P480,
        label: "480p",
        width: 854,
        height: 480,
        bitrate: 850_000.0,
    },
    Rendition {
        id: RenditionId::P360,
        label: "360p",
        width: 640,
        height: 360,
        bitrate: 450_000.0,
    },
    Rendition {
        id: RenditionId::P240,
        label: "240p",
        width: 426,
        height: 240,
        bitrate: 220_000.0,
    },
];

/// The rendition under `id`, 480p when there is none.
pub fn rendition_of(id: &str) -> &'static Rendition {
    RENDITIONS
        .iter()
        .find(|one| one.id.id() == id)
        .unwrap_or(&RENDITIONS[1])
}

/// How many frames each second of a clip has.
pub const FPS: f64 = 24.0;

/// The share of a video its poster is taken at: a third of the way in, without its title card.
pub const POSTER_AT: f64 = 0.35;

/// What a clip's scene is drawn from.
pub fn spec_of(video: &Video) -> SceneSpec {
    SceneSpec {
        kind: video.scene,
        hue: video.hue,
        accent: video.accent,
        seed: seed_of(video.id),
        title: video.title.to_owned(),
        channel: channel_of(video).name.to_owned(),
        duration: video.duration,
    }
}

/// The second a video's poster shows.
pub fn poster_time(video: &Video) -> f64 {
    video.duration * POSTER_AT
}

/// What the web keeps a video's clip at a rendition under.
pub fn key_of(video: &Video, rendition: &Rendition) -> String {
    format!("{}:{}", video.id, rendition.id.id())
}

/// A clip ready to play, as the player's stats and the downloads read it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Clip {
    pub rendition: Rendition,
    /// Its size, in bytes: the rendition's bitrate over the video's length.
    pub bytes: f64,
    /// The bits per second it came out at, from its size and length.
    pub bitrate: f64,
    /// Whether it has a sound track; always, as the sound is made here.
    pub sound: bool,
    /// How long it took to make, in milliseconds; nothing, as it is painted as it plays.
    pub made: f64,
    /// Whether it was kept from an earlier visit rather than made now.
    pub kept: bool,
}

/// The clip of `video` at `rendition`.
pub fn clip_of(video: &Video, rendition: &Rendition) -> Clip {
    let bytes = round(rendition.bitrate * video.duration / 8.0);
    Clip {
        rendition: *rendition,
        bytes,
        bitrate: round(bytes * 8.0 / video.duration),
        sound: true,
        made: 0.0,
        kept: false,
    }
}

/// One caption: from and to, in seconds, and its words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Cue {
    pub start: f64,
    pub end: f64,
    pub text: &'static str,
}

/// The video's captions: its lines spread evenly after the three seconds of the title card, each
/// gone 0.4 seconds before the next.
pub fn cues_of(video: &Video) -> Vec<Cue> {
    let start = 3.0;
    let each = (video.duration - start - 1.0) / video.captions.len() as f64;
    video
        .captions
        .iter()
        .enumerate()
        .map(|(index, line)| Cue {
            start: start + index as f64 * each,
            end: start + (index + 1) as f64 * each - 0.4,
            text: line,
        })
        .collect()
}

/// A second as WebVTT stamps it: `00:01:05.250`.
pub fn stamp(seconds: f64) -> String {
    let whole = seconds.floor();
    let millis = round((seconds - whole) * 1000.0);
    format!(
        "00:{:0>2}:{:0>2}.{:0>3}",
        number_to_string((whole / 60.0).floor()),
        number_to_string(whole % 60.0),
        number_to_string(millis)
    )
}

/// The video's captions as a WebVTT file.
pub fn captions_of(video: &Video) -> String {
    let cues: Vec<String> = cues_of(video)
        .iter()
        .map(|cue| format!("{} --> {}\n{}", stamp(cue.start), stamp(cue.end), cue.text))
        .collect();
    format!("WEBVTT\n\n{}\n", cues.join("\n\n"))
}
