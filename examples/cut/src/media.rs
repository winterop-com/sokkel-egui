//! The media bin, as the web's `media.ts` keeps it: the source clips every project cuts from. A
//! picture is a scene painted at any second, a sound is made sample by sample from the same kind of
//! scene, so a clip plays, scrubs and steps frame by frame with no file behind it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use eframe::egui::{Color32, Painter, Rect};
use tidevann_shared::clips::{SOUND_RATE, SceneKind, SceneSpec, draw_frame, synthesize};

use crate::model::TrackKind;

/// What a source is: a picture or a sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Video,
    Audio,
}

impl SourceKind {
    /// The kind of track its clips go on.
    pub fn track(self) -> TrackKind {
        match self {
            Self::Video => TrackKind::Video,
            Self::Audio => TrackKind::Audio,
        }
    }

    /// The kind as the web writes it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Audio => "audio",
        }
    }
}

/// A color a clip wears on the timeline, one of sokkel-ui's kind colors.
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
    /// The color's name, as its tokens are named after it (`kind-blue`, `kind-blue-ink`).
    pub fn name(self) -> &'static str {
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

/// One clip in the bin: what it is called, what it is painted or made from, and how long it runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Source {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: SourceKind,
    pub scene: SceneKind,
    pub hue: f64,
    pub accent: f64,
    pub seed: u32,
    /// Its full length in seconds.
    pub duration: f64,
    /// Where it was recorded, by plain geography.
    pub place: &'static str,
    /// The color its clips wear on the timeline.
    pub color: KindColor,
}

#[allow(clippy::too_many_arguments)]
const fn source(
    id: &'static str,
    name: &'static str,
    kind: SourceKind,
    scene: SceneKind,
    (hue, accent, seed): (f64, f64, u32),
    duration: f64,
    place: &'static str,
    color: KindColor,
) -> Source {
    Source {
        id,
        name,
        kind,
        scene,
        hue,
        accent,
        seed,
        duration,
        place,
        color,
    }
}

use KindColor::{Blue, Green, Jade, Pink, Teal, Violet};
use SourceKind::{Audio, Video};

/// The bin: nine pictures, then four sounds.
pub const SOURCES: [Source; 13] = [
    source(
        "harbor-dawn",
        "Harbor at dawn",
        Video,
        SceneKind::Sea,
        (212.0, 28.0, 11),
        24.0,
        "A harbor town on the northern coast",
        Blue,
    ),
    source(
        "lighthouse",
        "Lighthouse beam",
        Video,
        SceneKind::Lighthouse,
        (225.0, 48.0, 23),
        18.0,
        "A headland on the western coast",
        Violet,
    ),
    source(
        "coast-line",
        "Coast line train",
        Video,
        SceneKind::Train,
        (200.0, 30.0, 37),
        20.0,
        "A valley line inland",
        Teal,
    ),
    source(
        "ridge-snow",
        "Snow on the ridge",
        Video,
        SceneKind::Peaks,
        (215.0, 190.0, 41),
        16.0,
        "A mountain range in the north",
        Jade,
    ),
    source(
        "aurora",
        "Aurora over the fjord",
        Video,
        SceneKind::Aurora,
        (230.0, 140.0, 53),
        22.0,
        "A fjord in the far north",
        Green,
    ),
    source(
        "city-rain",
        "City in the rain",
        Video,
        SceneKind::Rain,
        (220.0, 38.0, 67),
        14.0,
        "A port city",
        Pink,
    ),
    source(
        "tide-chart",
        "Tide chart",
        Video,
        SceneKind::Tide,
        (205.0, 185.0, 71),
        12.0,
        "The harbor office",
        Blue,
    ),
    source(
        "ferry-route",
        "Ferry route",
        Video,
        SceneKind::Islands,
        (196.0, 95.0, 83),
        20.0,
        "An archipelago in the south",
        Teal,
    ),
    source(
        "wind-farm",
        "Wind over the fields",
        Video,
        SceneKind::Fields,
        (205.0, 100.0, 97),
        18.0,
        "Farmland on the plains",
        Jade,
    ),
    source(
        "surf",
        "Surf on the breakwater",
        Audio,
        SceneKind::Sea,
        (0.0, 0.0, 4),
        30.0,
        "A harbor town on the northern coast",
        Teal,
    ),
    source(
        "rails",
        "Wheels on the rails",
        Audio,
        SceneKind::Train,
        (0.0, 0.0, 5),
        24.0,
        "A valley line inland",
        Violet,
    ),
    source(
        "roof-rain",
        "Rain on the roof",
        Audio,
        SceneKind::Rain,
        (0.0, 0.0, 2),
        20.0,
        "A port city",
        Pink,
    ),
    source(
        "high-wind",
        "Wind on the ridge",
        Audio,
        SceneKind::Peaks,
        (0.0, 0.0, 3),
        26.0,
        "A mountain range in the north",
        Green,
    ),
];

/// The bin's source under `id`.
pub fn source_of(id: Option<&str>) -> Option<&'static Source> {
    let id = id?;
    SOURCES.iter().find(|source| source.id == id)
}

/// What a source's scene is painted or its sound made from.
pub fn spec_of(source: &Source) -> SceneSpec {
    SceneSpec {
        kind: source.scene,
        hue: source.hue,
        accent: source.accent,
        seed: source.seed,
        title: source.name.to_owned(),
        channel: "Tidevann".to_owned(),
        duration: source.duration,
    }
}

/// Paints a source's picture at second `t` of it on `rect`, without the title card its scene can
/// carry, on black, as the web's stills and preview have it behind what a scene leaves unpainted.
pub fn draw_source(painter: &Painter, rect: Rect, source: &Source, t: f64) {
    painter.rect_filled(rect, 0.0, Color32::BLACK);
    draw_frame(painter, rect, &spec_of(source), t.clamp(0.0, source.duration), false);
}

/// The second a picture source's still is taken at when none is named: a third of the way in.
pub fn poster_time(source: &Source) -> f64 {
    source.duration * 0.35
}

/// A source's two channels of sound.
pub type Samples = Arc<(Vec<f32>, Vec<f32>)>;

/// A sound source's two channels at `SOUND_RATE`, made once and kept for the visit.
pub fn samples_of(source: &Source) -> Samples {
    static MADE: OnceLock<Mutex<HashMap<&'static str, Samples>>> = OnceLock::new();
    let made = MADE.get_or_init(Default::default);
    if let Some(known) = made.lock().ok().and_then(|made| made.get(source.id).cloned()) {
        return known;
    }
    let samples: Samples = Arc::new(synthesize(&spec_of(source)));
    if let Ok(mut made) = made.lock() {
        made.insert(source.id, samples.clone());
    }
    samples
}

/// A sound source's peaks, each from 0 to 1.
pub type Peaks = Arc<Vec<f64>>;

/// How many peaks a second of sound is drawn with.
pub const PEAKS_PER_SECOND: f64 = 8.0;

/// The loudest sample in each eighth of a second of a sound source, from 0 to 1, for its waveform.
pub fn peaks_of(source: &Source) -> Peaks {
    static MADE: OnceLock<Mutex<HashMap<&'static str, Peaks>>> = OnceLock::new();
    let made = MADE.get_or_init(Default::default);
    if let Some(known) = made.lock().ok().and_then(|made| made.get(source.id).cloned()) {
        return known;
    }
    let samples = samples_of(source);
    let (left, right) = (&samples.0, &samples.1);
    let step = (f64::from(SOUND_RATE) / PEAKS_PER_SECOND).floor() as usize;
    let mut out = Vec::new();
    let mut loudest: f64 = 0.0;
    let mut start = 0;
    while start < left.len() {
        let mut peak: f64 = 0.0;
        let end = left.len().min(start + step);
        let mut index = start;
        while index < end {
            let l = f64::from(left[index]).abs();
            let r = right.get(index).map_or(0.0, |r| f64::from(*r).abs());
            peak = peak.max(l).max(r);
            index += 16;
        }
        out.push(peak);
        loudest = loudest.max(peak);
        start += step;
    }
    let scaled: Peaks = Arc::new(
        out.into_iter()
            .map(|peak| if loudest == 0.0 { 0.0 } else { peak / loudest })
            .collect(),
    );
    if let Ok(mut made) = made.lock() {
        made.insert(source.id, scaled.clone());
    }
    scaled
}

/// The peaks of the part of a sound source a clip plays, from `in_point` for `duration` seconds.
pub fn clip_peaks(source: &Source, in_point: f64, duration: f64) -> Vec<f64> {
    let all = peaks_of(source);
    let from = (in_point * PEAKS_PER_SECOND).floor().max(0.0);
    let to = (from + 1.0).max(((in_point + duration) * PEAKS_PER_SECOND).ceil());
    let from = (from as usize).min(all.len());
    let to = (to as usize).clamp(from, all.len());
    all[from..to].to_vec()
}
