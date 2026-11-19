//! How captions look and which track is on, as sokkel-ui's `media/captions.ts` reads a viewer's
//! choice: the core's `captions`, with the track the viewer chose kept beside the style.

use serde::Serialize;
pub use sokkel_egui::captions::{
    CAPTION_BACKGROUNDS, CAPTION_COLORS, CAPTION_EDGES, CAPTION_SIZES, CAPTION_STYLE, CaptionStyle, caption_style,
};

use crate::data::{CaptionChoice, CaptionTrack};

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

impl CaptionSettings {
    /// The style alone, as the player draws a cue with it.
    pub fn style(&self) -> CaptionStyle {
        CaptionStyle {
            size: self.size,
            color: self.color,
            background: self.background,
            edge: self.edge,
        }
    }
}

/// The settings captions are drawn with: each field the viewer chose that is still one the
/// player offers, and the library's defaults for the rest.
pub fn caption_settings(chosen: &CaptionChoice) -> CaptionSettings {
    let style = caption_style(
        chosen.size,
        chosen.color.as_deref(),
        chosen.background.as_deref(),
        chosen.edge.as_deref(),
    );
    CaptionSettings {
        size: style.size,
        color: style.color,
        background: style.background,
        edge: style.edge,
        track: chosen.track.clone(),
    }
}
