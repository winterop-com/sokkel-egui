//! The inputs the builder and the settings share, as the web's `inputs.tsx` makes them: the
//! palettes offered, each with its swatches.

use crate::model::{PALETTES, THEME_TOKENS};

/// A palette offered: its value, its label and the tokens its swatches show.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PaletteOption {
    pub value: String,
    pub label: String,
    pub swatches: Vec<&'static str>,
}

/// A palette's colors as a row of swatches: its first five, or the theme's.
pub fn swatches(tokens: Option<&'static [&'static str]>) -> Vec<&'static str> {
    tokens.unwrap_or(&THEME_TOKENS).iter().take(5).copied().collect()
}

/// The palettes as options, each with its colors; `with_default` adds the one the settings name,
/// with its label and its tokens.
pub fn palette_options(with_default: Option<(String, Option<&'static [&'static str]>)>) -> Vec<PaletteOption> {
    let mut out = Vec::new();
    if let Some((label, tokens)) = with_default {
        out.push(PaletteOption {
            value: "default".to_owned(),
            label,
            swatches: swatches(tokens),
        });
    }
    out.extend(PALETTES.iter().map(|palette| PaletteOption {
        value: palette.id.to_owned(),
        label: palette.label.to_owned(),
        swatches: swatches(palette.tokens),
    }));
    out
}
