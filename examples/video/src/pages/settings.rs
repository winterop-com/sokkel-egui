//! The viewer's settings, as the web's `pages/Settings.tsx` keeps them: playback, captions,
//! history and what the device keeps, each kept as it is changed, and each section's Reset to
//! defaults.

use serde::Serialize;
use sokkel_egui::{MEDIA_RATES, format_rate};

use crate::captions::{CAPTION_BACKGROUNDS, CAPTION_COLORS, CAPTION_EDGES, CAPTION_SIZES, caption_settings};
use crate::clips::RENDITIONS;
use crate::data::{CaptionChoice, CaptionTrack, Data, PrefsPatch, Quality, default_prefs};
use crate::format::{bytes_of, plural};
use crate::js::{number_to_string, round};

/// A caption ink's name.
pub fn color_name(color: &str) -> &'static str {
    match color {
        "yellow" => "Yellow",
        "green" => "Green",
        "cyan" => "Cyan",
        "magenta" => "Magenta",
        _ => "White",
    }
}

/// A caption box's name.
pub fn background_name(background: &str) -> &'static str {
    match background {
        "translucent" => "Translucent",
        "solid" => "Solid",
        _ => "None",
    }
}

/// A caption edge's name.
pub fn edge_name(edge: &str) -> &'static str {
    match edge {
        "shadow" => "Drop shadow",
        "none" => "None",
        _ => "Outline",
    }
}

/// The speeds offered, with their words: `Normal` for 1.
pub fn speeds() -> Vec<(f64, String)> {
    MEDIA_RATES
        .iter()
        .map(|rate| {
            (
                *rate,
                if *rate == 1.0 {
                    "Normal".to_owned()
                } else {
                    format_rate(*rate)
                },
            )
        })
        .collect()
}

/// The qualities offered, with their words.
pub fn qualities() -> Vec<(Quality, &'static str)> {
    std::iter::once((Quality::Auto, "Auto"))
        .chain(RENDITIONS.iter().map(|one| (Quality::Rendition(one.id), one.label)))
        .collect()
}

/// The caption sizes offered, as value and words: `50%`.
pub fn caption_sizes() -> Vec<(String, String)> {
    CAPTION_SIZES
        .iter()
        .map(|size| {
            (
                number_to_string(*size),
                format!("{}%", number_to_string(round(size * 100.0))),
            )
        })
        .collect()
}

/// The caption inks offered, as value and name.
pub fn colors() -> Vec<(&'static str, &'static str)> {
    CAPTION_COLORS.iter().map(|one| (*one, color_name(one))).collect()
}

/// The caption boxes offered, as value and name.
pub fn backgrounds() -> Vec<(&'static str, &'static str)> {
    CAPTION_BACKGROUNDS
        .iter()
        .map(|one| (*one, background_name(one)))
        .collect()
}

/// The caption edges offered, as value and name.
pub fn edges() -> Vec<(&'static str, &'static str)> {
    CAPTION_EDGES.iter().map(|one| (*one, edge_name(one))).collect()
}

/// The captions' controls as the page sets them.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CaptionControls {
    pub on: bool,
    pub size: String,
    pub color: &'static str,
    pub background: &'static str,
    pub edge: &'static str,
}

pub fn caption_controls(data: &Data) -> CaptionControls {
    let captions = caption_settings(&data.prefs.captions);
    CaptionControls {
        on: captions.track != CaptionTrack::Off,
        size: number_to_string(captions.size),
        color: captions.color,
        background: captions.background,
        edge: captions.edge,
    }
}

/// A change to the captions: kept over what the viewer chose before.
pub fn set_captions(data: &mut Data, change: impl FnOnce(&mut CaptionChoice)) {
    let mut captions = data.prefs.captions.clone();
    change(&mut captions);
    data.set_prefs(&PrefsPatch {
        captions: Some(captions),
        ..PrefsPatch::default()
    });
}

/// Show captions' switch.
pub fn show_captions(data: &mut Data, on: bool) {
    set_captions(data, |captions| {
        captions.track = if on {
            CaptionTrack::On("en".to_owned())
        } else {
            CaptionTrack::Off
        };
    });
}

pub fn caption_size(data: &mut Data, size: f64) {
    set_captions(data, |captions| captions.size = Some(size));
}

pub fn caption_color(data: &mut Data, color: &str) {
    set_captions(data, |captions| captions.color = Some(color.to_owned()));
}

pub fn caption_background(data: &mut Data, background: &str) {
    set_captions(data, |captions| captions.background = Some(background.to_owned()));
}

pub fn caption_edge(data: &mut Data, edge: &str) {
    set_captions(data, |captions| captions.edge = Some(edge.to_owned()));
}

/// The settings each section holds, which its Reset to defaults puts back.
pub fn held(section: &str) -> &'static [&'static str] {
    match section {
        "playback" => &["autoplay", "theater", "rate", "quality", "stats"],
        "captions" => &["captions"],
        "history" => &["history"],
        _ => &[],
    }
}

/// A section's Reset to defaults: the settings it holds put back as they started.
pub fn reset(data: &mut Data, section: &str) {
    let keys = held(section);
    if keys.is_empty() {
        return;
    }
    let defaults = default_prefs();
    let mut patch = PrefsPatch::default();
    for key in keys {
        match *key {
            "autoplay" => patch.autoplay = Some(defaults.autoplay),
            "theater" => patch.theater = Some(defaults.theater),
            "rate" => patch.rate = Some(defaults.rate),
            "quality" => patch.quality = Some(defaults.quality),
            "stats" => patch.stats = Some(defaults.stats),
            "captions" => patch.captions = Some(defaults.captions.clone()),
            "history" => patch.history = Some(defaults.history),
            _ => {}
        }
    }
    data.set_prefs(&patch);
}

/// What the page says of the history and the downloads.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Notes {
    pub history: String,
    pub clear_disabled: bool,
    pub downloads: String,
}

pub fn notes(data: &Data) -> Notes {
    let history = data.history.seen().len();
    let offline = data.downloads.seen();
    Notes {
        history: format!("{} in your history now.", plural(history as f64, "video", None)),
        clear_disabled: history == 0,
        downloads: format!(
            "{}, {}, kept to watch without the site.",
            plural(offline.len() as f64, "video", None),
            bytes_of(offline.iter().map(|one| one.bytes).sum())
        ),
    }
}

/// One of the page's switches, by its setting's id: autoplay, theater mode, the stats, the
/// captions or keeping the history.
pub fn toggle(data: &mut Data, id: &str, on: bool) {
    let mut patch = PrefsPatch::default();
    match id {
        "autoplay" => patch.autoplay = Some(on),
        "theater" => patch.theater = Some(on),
        "stats" => patch.stats = Some(on),
        "keep-history" => patch.history = Some(on),
        "captions-on" => return show_captions(data, on),
        _ => return,
    }
    data.set_prefs(&patch);
}

/// The speed every video starts at.
pub fn speed(data: &mut Data, rate: f64) {
    data.set_prefs(&PrefsPatch {
        rate: Some(rate),
        ..PrefsPatch::default()
    });
}

/// The quality every video plays at.
pub fn quality(data: &mut Data, quality: Quality) {
    data.set_prefs(&PrefsPatch {
        quality: Some(quality),
        ..PrefsPatch::default()
    });
}

/// Clear watch history's button: the history's own confirm.
pub fn clear_history(data: &mut Data) {
    data.go("/history/clear");
}
