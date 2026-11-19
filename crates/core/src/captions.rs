//! How captions look and which track is on, as sokkel-ui's `media/captions.ts` reads a viewer's
//! choice: the sizes, inks, boxes and edges offered, the style drawn from a choice kept by hand or
//! by an older version (each field that is not one of the options falling back alone), which of a
//! video's tracks shows, and the tokens a cue is painted with.

use egui::Color32;
use serde::Serialize;

use crate::Tokens;

/// The sizes the captions menu offers, as shares of the usual.
pub const CAPTION_SIZES: [f64; 5] = [0.5, 0.75, 1.0, 1.5, 2.0];
/// The inks a viewer may pick, each a `caption-*` token.
pub const CAPTION_COLORS: [&str; 5] = ["white", "yellow", "green", "cyan", "magenta"];
/// What stands behind caption text: nothing, a see-through box, or a solid one.
pub const CAPTION_BACKGROUNDS: [&str; 3] = ["none", "translucent", "solid"];
/// What holds caption text against a bright picture: an outline, a drop shadow, or nothing.
pub const CAPTION_EDGES: [&str; 3] = ["outline", "shadow", "none"];

/// The smallest and largest share of the usual size a kept setting may ask for.
const SMALLEST: f64 = 0.25;
const LARGEST: f64 = 4.0;

/// How captions look.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct CaptionStyle {
    /// The text's size as a share of the usual (`caption-size`): 1 is the usual, 0.5 half of it.
    pub size: f64,
    pub color: &'static str,
    pub background: &'static str,
    pub edge: &'static str,
}

/// Captions as the library draws them until told otherwise: words on the picture, outlined, with
/// no box.
pub const CAPTION_STYLE: CaptionStyle = CaptionStyle {
    size: 1.0,
    color: "white",
    background: "none",
    edge: "outline",
};

fn one_of(options: &[&'static str], value: Option<&str>) -> Option<&'static str> {
    let value = value?;
    options.iter().copied().find(|one| *one == value)
}

/// The style captions are drawn with, from what the viewer chose: each field that is still one the
/// player offers, and the library's defaults for the rest, so one bad field never loses the others.
pub fn caption_style(
    size: Option<f64>,
    color: Option<&str>,
    background: Option<&str>,
    edge: Option<&str>,
) -> CaptionStyle {
    CaptionStyle {
        size: size
            .filter(|size| size.is_finite() && (SMALLEST..=LARGEST).contains(size))
            .unwrap_or(CAPTION_STYLE.size),
        color: one_of(&CAPTION_COLORS, color).unwrap_or(CAPTION_STYLE.color),
        background: one_of(&CAPTION_BACKGROUNDS, background).unwrap_or(CAPTION_STYLE.background),
        edge: one_of(&CAPTION_EDGES, edge).unwrap_or(CAPTION_STYLE.edge),
    }
}

/// What a track is remembered by: its language where it has one, as the same language is chosen
/// again on the next video, else its label.
pub fn caption_track_key<'a>(label: &'a str, lang: Option<&'a str>) -> &'a str {
    match lang {
        Some(lang) if !lang.is_empty() => lang,
        _ => label,
    }
}

/// Which of `tracks` (label, language, shown by default) shows: the one the viewer chose by its key,
/// or failing that by the language the key names (`en` finds `en-GB`); none when they turned
/// captions off (`Some(None)`); and the default track when they chose none or the chosen one is not
/// offered here.
pub fn chosen_caption_track(tracks: &[(&str, Option<&str>, bool)], track: Option<Option<&str>>) -> Option<usize> {
    let key = match track {
        Some(None) => return None,
        Some(Some(key)) => Some(key),
        None => None,
    };
    if let Some(key) = key {
        if let Some(at) = tracks
            .iter()
            .position(|(label, lang, _)| caption_track_key(label, *lang) == key)
        {
            return Some(at);
        }
        let language = key.split('-').next().unwrap_or_default().to_lowercase();
        if let Some(at) = tracks.iter().position(|(_, lang, _)| {
            lang.is_some_and(|lang| lang.split('-').next().unwrap_or_default().to_lowercase() == language)
        }) {
            return Some(at);
        }
    }
    tracks.iter().position(|(_, _, default)| *default)
}

/// A cue's ink: the `caption-<color>` token.
pub fn caption_ink(t: &Tokens, style: &CaptionStyle) -> Color32 {
    t.color(&format!("caption-{}", style.color))
}

/// What stands behind a cue's words: nothing, `caption-ground` at 62%, or all of it.
pub fn caption_ground(t: &Tokens, style: &CaptionStyle) -> Color32 {
    match style.background {
        "translucent" => t.color_at("caption-ground", 0.62),
        "solid" => t.color("caption-ground"),
        _ => Color32::TRANSPARENT,
    }
}

/// The size a cue is written at in a player `width` wide: `caption-size` times the style's share,
/// sized against the player's width as the web's container query sizes it, never under
/// `caption-size-min`.
pub fn caption_text_size(t: &Tokens, style: &CaptionStyle, width: f32) -> f32 {
    // `caption-size` is written as a share of the player's width, such as `2.2cqw`
    let share = t
        .word("caption-size")
        .and_then(|word| word.strip_suffix("cqw"))
        .and_then(|number| number.trim().parse::<f32>().ok())
        .unwrap_or(2.2);
    (width * share / 100.0 * style.size as f32).max(t.length("caption-size-min"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bad_field_falls_back_alone() {
        let style = caption_style(Some(9.0), Some("yellow"), Some("glass"), None);
        assert_eq!(style.size, 1.0);
        assert_eq!(style.color, "yellow");
        assert_eq!(style.background, "none");
        assert_eq!(style.edge, "outline");
    }

    #[test]
    fn the_chosen_track_or_its_language_or_the_default() {
        let tracks = [("English", Some("en-GB"), false), ("Norsk", Some("nb"), true)];
        assert_eq!(chosen_caption_track(&tracks, Some(Some("en"))), Some(0));
        assert_eq!(chosen_caption_track(&tracks, Some(None)), None);
        assert_eq!(chosen_caption_track(&tracks, None), Some(1));
    }
}
