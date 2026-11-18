//! The visualizer's looks the settings and the fullscreen picker offer, as the web's `look.ts`
//! lists them, and how each is drawn: its kind, whether it is mirrored, and the colors and effect
//! the settings choose.

use crate::data::{Effect, Look, Ramp};

/// The looks, each with its label.
pub const LOOKS: [(Look, &str); 5] = [
    (Look::Spectrum, "Bars"),
    (Look::Mirror, "Mirror"),
    (Look::Area, "Area"),
    (Look::Wave, "Wave"),
    (Look::Ring, "Ring"),
];

pub const RAMPS: [(Ramp, &str); 5] = [
    (Ramp::Theme, "Theme"),
    (Ramp::Heat, "Heat"),
    (Ramp::Cool, "Cool"),
    (Ramp::Lagoon, "Lagoon"),
    (Ramp::Ink, "Ink"),
];

pub const EFFECTS: [(Effect, &str); 3] = [
    (Effect::None, "None"),
    (Effect::Glow, "Glow"),
    (Effect::Trails, "Trails"),
];

impl Look {
    /// The look as the web names it.
    pub fn key(self) -> &'static str {
        match self {
            Look::Spectrum => "spectrum",
            Look::Mirror => "mirror",
            Look::Area => "area",
            Look::Wave => "wave",
            Look::Ring => "ring",
        }
    }

    /// A look by its web name (`isLook`).
    pub fn from_key(id: &str) -> Option<Look> {
        LOOKS.iter().map(|(look, _)| *look).find(|look| look.key() == id)
    }
}

impl Ramp {
    pub fn key(self) -> &'static str {
        match self {
            Ramp::Theme => "theme",
            Ramp::Heat => "heat",
            Ramp::Cool => "cool",
            Ramp::Lagoon => "lagoon",
            Ramp::Ink => "ink",
        }
    }
}

impl Effect {
    pub fn key(self) -> &'static str {
        match self {
            Effect::None => "none",
            Effect::Glow => "glow",
            Effect::Trails => "trails",
        }
    }
}

/// What the visualizer draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualizerKind {
    Spectrum,
    Area,
    Wave,
    Ring,
}

impl VisualizerKind {
    pub fn key(self) -> &'static str {
        match self {
            VisualizerKind::Spectrum => "spectrum",
            VisualizerKind::Area => "area",
            VisualizerKind::Wave => "wave",
            VisualizerKind::Ring => "ring",
        }
    }
}

/// The colors the visualizer is drawn in: the theme's primary, or one of the ramps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualizerRamp {
    Primary,
    Heat,
    Cool,
    Lagoon,
    Ink,
}

impl VisualizerRamp {
    pub fn key(self) -> &'static str {
        match self {
            VisualizerRamp::Primary => "primary",
            VisualizerRamp::Heat => "heat",
            VisualizerRamp::Cool => "cool",
            VisualizerRamp::Lagoon => "lagoon",
            VisualizerRamp::Ink => "ink",
        }
    }
}

/// The visualizer's settings for a look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LookProps {
    pub kind: VisualizerKind,
    pub mirror: bool,
    pub ramp: VisualizerRamp,
    pub effect: Effect,
}

/// The visualizer's settings for a look, in the colors and effect chosen.
pub fn look_props(look: Look, ramp: Ramp, effect: Effect) -> LookProps {
    let ramp = match ramp {
        Ramp::Theme => VisualizerRamp::Primary,
        Ramp::Heat => VisualizerRamp::Heat,
        Ramp::Cool => VisualizerRamp::Cool,
        Ramp::Lagoon => VisualizerRamp::Lagoon,
        Ramp::Ink => VisualizerRamp::Ink,
    };
    let (kind, mirror) = match look {
        Look::Mirror => (VisualizerKind::Spectrum, true),
        Look::Area => (VisualizerKind::Area, false),
        Look::Wave => (VisualizerKind::Wave, false),
        Look::Ring => (VisualizerKind::Ring, false),
        Look::Spectrum => (VisualizerKind::Spectrum, false),
    };
    LookProps {
        kind,
        mirror,
        ramp,
        effect,
    }
}
