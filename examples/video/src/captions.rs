//! How captions look and which track is on, as sokkel-ui's `media/captions.ts` reads a viewer's
//! choice: the sizes, inks, boxes and edges offered, and the settings drawn from a choice kept by
//! hand or by an older version, each field that is not one of the options falling back alone.
//! Carried here until the core has the captions' settings.

use serde::Serialize;

use crate::data::{CaptionChoice, CaptionTrack};

/// The sizes the captions menu offers, as shares of the usual.
pub const CAPTION_SIZES: [f64; 5] = [0.5, 0.75, 1.0, 1.5, 2.0];
pub const CAPTION_COLORS: [&str; 5] = ["white", "yellow", "green", "cyan", "magenta"];
pub const CAPTION_BACKGROUNDS: [&str; 3] = ["none", "translucent", "solid"];
pub const CAPTION_EDGES: [&str; 3] = ["outline", "shadow", "none"];

/// The smallest and largest share of the usual size a kept setting may ask for.
const SMALLEST: f64 = 0.25;
const LARGEST: f64 = 4.0;

/// The settings captions are drawn with.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CaptionSettings {
    pub size: f64,
    pub color: &'static str,
    pub background: &'static str,
    pub edge: &'static str,
    #[serde(skip_serializing_if = "CaptionTrack::is_unchosen")]
    pub track: CaptionTrack,
}

/// Captions as the library draws them until told otherwise: words on the picture, outlined, with
/// no box.
pub const CAPTION_STYLE: (f64, &str, &str, &str) = (1.0, "white", "none", "outline");

fn one_of(options: &[&'static str], value: Option<&String>) -> Option<&'static str> {
    let value = value?;
    options.iter().copied().find(|one| *one == value)
}

/// The settings captions are drawn with: each field the viewer chose that is still one the
/// player offers, and the library's defaults for the rest.
pub fn caption_settings(chosen: &CaptionChoice) -> CaptionSettings {
    let (size, color, background, edge) = CAPTION_STYLE;
    CaptionSettings {
        size: chosen
            .size
            .filter(|size| size.is_finite() && (SMALLEST..=LARGEST).contains(size))
            .unwrap_or(size),
        color: one_of(&CAPTION_COLORS, chosen.color.as_ref()).unwrap_or(color),
        background: one_of(&CAPTION_BACKGROUNDS, chosen.background.as_ref()).unwrap_or(background),
        edge: one_of(&CAPTION_EDGES, chosen.edge.as_ref()).unwrap_or(edge),
        track: chosen.track.clone(),
    }
}
