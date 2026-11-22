//! sokkel-ui's tokens, resolved by its token script to plain values (`tokens/tokens.resolved.json`,
//! copied from `@winterop-com/sokkel-ui/tokens.resolved.json` by `make tokens`), and the
//! appearance an app picks from them.

use std::collections::HashMap;
use std::sync::OnceLock;

use egui::{Color32, CornerRadius};
use serde::{Deserialize, Serialize};

const SOURCE: &str = include_str!("../tokens/tokens.resolved.json");

#[derive(Deserialize)]
struct Source {
    default: Defaults,
    themes: serde_json::Map<String, serde_json::Value>,
    density: HashMap<String, HashMap<String, f32>>,
    /// Each text size's steps of the scale, in the order the tokens list the levels; only the
    /// levels' names and order are read here, each theme carries its roles at every level.
    #[serde(default, rename = "textSize")]
    text_size: serde_json::Map<String, serde_json::Value>,
    /// Each part's paints in each state, resolved per theme and scheme by sokkel-design.
    #[serde(default)]
    paints: HashMap<String, HashMap<String, HashMap<String, serde_json::Value>>>,
}

#[derive(Deserialize)]
struct Defaults {
    theme: String,
    density: String,
    #[serde(default, rename = "textSize")]
    text_size: Option<String>,
}

#[derive(Deserialize)]
struct ThemeSource {
    label: String,
    #[serde(default)]
    description: Option<String>,
    light: SchemeSource,
    dark: SchemeSource,
}

#[derive(Deserialize)]
struct SchemeSource {
    colors: serde_json::Map<String, serde_json::Value>,
    lengths: serde_json::Map<String, serde_json::Value>,
    numbers: HashMap<String, f32>,
    words: HashMap<String, String>,
    text: HashMap<String, TextSpec>,
    /// Every text role at each text size, by level (`normal`, ...); the theme's own is `text`.
    #[serde(default, rename = "textSizes")]
    text_sizes: HashMap<String, HashMap<String, TextSpec>>,
    /// Box shadows as layers, the way sokkel-design resolves them (`elevation-pop`, ...).
    #[serde(default)]
    elevations: HashMap<String, Vec<Layer>>,
    /// Durations in milliseconds (`motion-base`, `busy-release`, ...).
    #[serde(default)]
    durations: HashMap<String, f32>,
}

/// One layer of a box shadow.
#[derive(Clone, Debug, Deserialize)]
struct Layer {
    x: f32,
    y: f32,
    blur: f32,
    spread: f32,
    color: String,
}

/// A text role as the tokens set it: size and line height in points, weight, tracking in em.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSpec {
    pub size: f32,
    pub line_height: f32,
    pub weight: f32,
    pub tracking: f32,
}

/// A theme as a picker offers it.
#[derive(Clone, Debug)]
pub struct ThemeInfo {
    pub name: String,
    pub label: String,
    pub description: Option<String>,
}

struct Catalog {
    default_theme: String,
    default_density: String,
    default_text_size: String,
    /// The text sizes, smallest first, as the tokens list them.
    text_sizes: Vec<String>,
    themes: Vec<(ThemeInfo, ThemeSource)>,
    density: HashMap<String, HashMap<String, f32>>,
    /// Theme, then `light` or `dark`, then the paint's key.
    paints: HashMap<String, HashMap<String, HashMap<String, Paint>>>,
}

/// A part's colors in one state, as sokkel-design resolves them (`paints` in the resolved tokens):
/// its ground, its edge and its ink, and the focus outline or invalid ring it wears there. A field
/// sokkel-design leaves to the part's surroundings (`inherit`) is `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Paint {
    pub ground: Option<Color32>,
    pub edge: Option<Color32>,
    pub ink: Option<Color32>,
    /// The focus outline: its color, width and offset (negative drawn inside the part).
    pub outline: Option<(Color32, f32, f32)>,
    /// The invalid ring: its color and width.
    pub ring: Option<(Color32, f32)>,
    /// How much of the part shows, as CSS's `opacity`.
    pub opacity: Option<f32>,
}

impl Paint {
    fn read(value: &serde_json::Value) -> Self {
        let color = |key: &str| {
            value
                .get(key)
                .and_then(|v| v.as_str())
                .filter(|v| v.starts_with('#'))
                .map(parse_hex)
        };
        let outline = value.get("outline").and_then(|o| {
            let width = o.get("width")?.as_f64()? as f32;
            let offset = o.get("offset").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let inside = o.get("inset").is_some();
            Some((
                parse_hex(o.get("color")?.as_str()?),
                width,
                if inside { -offset } else { offset },
            ))
        });
        let ring = value
            .get("ring")
            .and_then(|r| Some((parse_hex(r.get("color")?.as_str()?), r.get("width")?.as_f64()? as f32)));
        Self {
            ground: color("ground"),
            edge: color("edge"),
            ink: color("ink"),
            outline,
            ring,
            opacity: value.get("opacity").and_then(|v| v.as_f64()).map(|v| v as f32),
        }
    }
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let source: Source = serde_json::from_str(SOURCE).expect("tokens.resolved.json is valid");
        let themes = source
            .themes
            .into_iter()
            .map(|(name, value)| {
                let theme: ThemeSource = serde_json::from_value(value).expect("a resolved theme is valid");
                let info = ThemeInfo {
                    name,
                    label: theme.label.clone(),
                    description: theme.description.clone(),
                };
                (info, theme)
            })
            .collect();
        Catalog {
            default_theme: source.default.theme,
            default_density: source.default.density,
            default_text_size: source.default.text_size.unwrap_or_default(),
            text_sizes: source.text_size.keys().cloned().collect(),
            themes,
            density: source.density,
            paints: source
                .paints
                .into_iter()
                .map(|(theme, schemes)| {
                    let schemes = schemes
                        .into_iter()
                        .map(|(scheme, keys)| (scheme, keys.iter().map(|(k, v)| (k.clone(), Paint::read(v))).collect()))
                        .collect();
                    (theme, schemes)
                })
                .collect(),
        }
    })
}

/// Every theme, the base theme first, in the order the tokens define them.
pub fn themes() -> impl Iterator<Item = &'static ThemeInfo> {
    catalog().themes.iter().map(|(info, _)| info)
}

/// A theme's sample, drawn in that theme whatever the page wears: its background and border, and
/// its text, accent and third chart color as dots on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swatch {
    pub ground: Color32,
    pub edge: Color32,
    pub dots: [Color32; 3],
}

/// The sample of the theme named `name` in the scheme asked for; `None` for a theme the tokens lack.
pub fn swatch(name: &str, dark: bool) -> Option<Swatch> {
    let (_, theme) = catalog().themes.iter().find(|(info, _)| info.name == name)?;
    let scheme = if dark { &theme.dark } else { &theme.light };
    let color = |key: &str| scheme.colors.get(key).and_then(|v| v.as_str()).map(parse_hex);
    Some(Swatch {
        ground: color("background")?,
        edge: color("border")?,
        dots: [color("foreground")?, color("primary")?, color("chart-3")?],
    })
}

/// Every density level, by name.
pub fn densities() -> Vec<&'static str> {
    let mut levels: Vec<&str> = catalog().density.keys().map(String::as_str).collect();
    levels.sort_by_key(|level| (*level != catalog().default_density, *level));
    levels
}

/// Every text size, by name, smallest first: what a person may pick in Settings.
pub fn text_sizes() -> Vec<&'static str> {
    catalog().text_sizes.iter().map(String::as_str).collect()
}

fn default_text_size() -> String {
    catalog().default_text_size.clone()
}

/// Light, dark, or whichever the system shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scheme {
    Light,
    Dark,
    #[default]
    System,
}

/// Whether parts move as they open and change: as the system asks, always, or never.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Motion {
    #[default]
    System,
    On,
    Off,
}

/// What an app picks: a theme, a scheme, a density, the text size and the motion, as sokkel-ui's
/// `ThemeSettings` holds them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Appearance {
    pub theme: String,
    pub scheme: Scheme,
    pub density: String,
    /// The text size from the md width up (`normal` by default); a phone keeps the theme's own.
    #[serde(default = "default_text_size")]
    pub text_size: String,
    #[serde(default)]
    pub motion: Motion,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: catalog().default_theme.clone(),
            scheme: Scheme::System,
            density: catalog().default_density.clone(),
            text_size: catalog().default_text_size.clone(),
            motion: Motion::System,
        }
    }
}

/// A font weight the contract names, read by [`Tokens::weight`]: text set heavier than its role
/// asks for, as the web's `font-medium`, `font-semibold` and `font-bold`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    Medium,
    Semibold,
    Bold,
}

/// One theme in one scheme at one density and text size, every value ready to paint with.
#[derive(Debug)]
pub struct Tokens {
    pub theme: String,
    pub dark: bool,
    /// The text size the text roles were resolved at: the appearance's from the md width up, none
    /// (the theme's own sizes) below it.
    pub text_size: Option<String>,
    colors: HashMap<String, Color32>,
    lengths: HashMap<String, f32>,
    /// Every color and length in the order the tokens define them, for listing them.
    color_order: Vec<String>,
    length_order: Vec<String>,
    numbers: HashMap<String, f32>,
    words: HashMap<String, String>,
    text: HashMap<String, TextSpec>,
    density: HashMap<String, f32>,
    elevations: HashMap<String, Vec<egui::Shadow>>,
    durations: HashMap<String, f32>,
    paints: HashMap<String, Paint>,
}

impl Tokens {
    /// The tokens an appearance wears from the md width up. An unknown theme, density or text size
    /// falls back to the default one; `system_dark` answers `Scheme::System`.
    pub fn resolve(appearance: &Appearance, system_dark: bool) -> Self {
        Self::resolve_at(appearance, system_dark, true)
    }

    /// The tokens an appearance wears, `wide` from the md width up: below it the text keeps the
    /// theme's own sizes, as the web's text size answers only from md.
    pub fn resolve_at(appearance: &Appearance, system_dark: bool, wide: bool) -> Self {
        let catalog = catalog();
        let (info, theme) = catalog
            .themes
            .iter()
            .find(|(info, _)| info.name == appearance.theme)
            .or_else(|| {
                catalog
                    .themes
                    .iter()
                    .find(|(info, _)| info.name == catalog.default_theme)
            })
            .expect("the tokens have their default theme");
        let dark = match appearance.scheme {
            Scheme::Light => false,
            Scheme::Dark => true,
            Scheme::System => system_dark,
        };
        let scheme = if dark { &theme.dark } else { &theme.light };
        let density = catalog
            .density
            .get(&appearance.density)
            .or_else(|| catalog.density.get(&catalog.default_density))
            .expect("the tokens have their default density");
        let level = [appearance.text_size.as_str(), catalog.default_text_size.as_str()]
            .into_iter()
            .find(|level| wide && scheme.text_sizes.contains_key(*level));
        let text = level.map_or(&scheme.text, |level| &scheme.text_sizes[level]);
        Self {
            theme: info.name.clone(),
            dark,
            text_size: level.map(str::to_owned),
            colors: scheme
                .colors
                .iter()
                .map(|(key, value)| (key.clone(), parse_hex(value.as_str().expect("a hex color"))))
                .collect(),
            lengths: scheme
                .lengths
                .iter()
                .map(|(key, value)| (key.clone(), value.as_f64().expect("a length") as f32))
                .collect(),
            color_order: scheme.colors.keys().cloned().collect(),
            length_order: scheme.lengths.keys().cloned().collect(),
            numbers: scheme.numbers.clone(),
            durations: scheme.durations.clone(),
            paints: catalog
                .paints
                .get(&info.name)
                .and_then(|s| s.get(if dark { "dark" } else { "light" }))
                .cloned()
                .unwrap_or_default(),
            words: scheme.words.clone(),
            text: text.clone(),
            density: density.clone(),
            elevations: scheme
                .elevations
                .iter()
                .map(|(key, layers)| {
                    let shadows = layers
                        .iter()
                        .map(|l| egui::Shadow {
                            offset: [l.x.round() as i8, l.y.round() as i8],
                            blur: l.blur.round().clamp(0.0, 255.0) as u8,
                            spread: l.spread.round().clamp(0.0, 255.0) as u8,
                            color: parse_hex(&l.color),
                        })
                        .collect();
                    (key.clone(), shadows)
                })
                .collect(),
        }
    }

    /// A color token, such as `primary` or `muted-foreground`.
    pub fn color(&self, key: &str) -> Color32 {
        match self.colors.get(key) {
            Some(color) => *color,
            None => panic!("no color token `{key}` in {}", self.theme),
        }
    }

    /// A color token, or `fallback` where the theme leaves it to what the part drew before (the
    /// frame parts such as `section-head-ink` are `currentcolor` in some themes).
    pub fn color_or(&self, key: &str, fallback: Color32) -> Color32 {
        self.colors.get(key).copied().unwrap_or(fallback)
    }

    /// A color token at a share of its alpha, as Tailwind's `bg-primary/15` paints it.
    pub fn color_at(&self, key: &str, alpha: f32) -> Color32 {
        self.color(key).gamma_multiply(alpha)
    }

    /// Every color token by name, in the order the tokens define them.
    pub fn colors(&self) -> impl Iterator<Item = (&str, Color32)> {
        self.color_order.iter().map(|key| (key.as_str(), self.colors[key]))
    }

    /// Every length token by name, in the order the tokens define them.
    pub fn lengths(&self) -> impl Iterator<Item = (&str, f32)> {
        self.length_order.iter().map(|key| (key.as_str(), self.lengths[key]))
    }

    /// The density level these tokens were resolved at, with every length it sets.
    pub fn density_lengths(&self) -> Vec<(&str, f32)> {
        let mut lengths: Vec<_> = self.density.iter().map(|(key, value)| (key.as_str(), *value)).collect();
        lengths.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(b.0)));
        lengths
    }

    /// A length token in points, such as `radius` or `table-head-size`.
    pub fn length(&self, key: &str) -> f32 {
        match self.lengths.get(key) {
            Some(length) => *length,
            None => panic!("no length token `{key}` in {}", self.theme),
        }
    }

    /// A corner by role: `control`, `surface`, `chip`, `thumb` and the other `radius-*` tokens.
    pub fn radius(&self, role: &str) -> CornerRadius {
        CornerRadius::same(self.length(&format!("radius-{role}")).floor() as u8)
    }

    /// A density length in points, such as `control` or `row`.
    pub fn density(&self, key: &str) -> f32 {
        match self.density.get(key) {
            Some(length) => *length,
            None => panic!("no density token `{key}`"),
        }
    }

    /// A part's paint in a state, as sokkel-design resolves it: `button.outline.hover.container`.
    pub fn paint(&self, key: &str) -> Option<Paint> {
        self.paints.get(key).copied()
    }

    /// Every paint key the theme and scheme carry.
    pub fn paint_keys(&self) -> impl Iterator<Item = &str> {
        self.paints.keys().map(String::as_str)
    }

    /// A duration token in milliseconds, such as `busy-release`.
    pub fn duration(&self, key: &str) -> Option<f32> {
        self.durations.get(key).copied()
    }

    /// A plain number token, such as `mono-x-height`.
    pub fn number(&self, key: &str) -> Option<f32> {
        self.numbers.get(key).copied()
    }

    /// A named font weight on the variable face's `wght` axis, from the theme's
    /// `font-weight-medium`, `font-weight-semibold` and `font-weight-bold`.
    pub fn weight(&self, weight: Weight) -> f32 {
        let (key, before) = match weight {
            Weight::Medium => ("font-weight-medium", 500.0),
            Weight::Semibold => ("font-weight-semibold", 600.0),
            Weight::Bold => ("font-weight-bold", 700.0),
        };
        self.number(key).unwrap_or(before)
    }

    /// A word token, such as `typeface-sans` or `table-head-transform`.
    pub fn word(&self, key: &str) -> Option<&str> {
        self.words.get(key).map(String::as_str)
    }

    /// A box-shadow token, such as `elevation-pop`, as egui shadows, one per layer in the order
    /// written (`x y blur [spread] color`, comma-separated); none for a token the theme lacks.
    pub fn shadows(&self, key: &str) -> Vec<egui::Shadow> {
        // sokkel-design resolves them into layers; a box-shadow written as words is read too
        if let Some(layers) = self.elevations.get(key) {
            return layers.clone();
        }
        let Some(value) = self.word(key) else {
            return Vec::new();
        };
        value
            .split(',')
            .filter_map(|layer| {
                let parts: Vec<&str> = layer.split_whitespace().collect();
                let (color, lengths) = parts.split_last()?;
                let px = |i: usize| -> f32 {
                    lengths
                        .get(i)
                        .and_then(|l| l.trim_end_matches("px").parse().ok())
                        .unwrap_or(0.0)
                };
                Some(egui::Shadow {
                    offset: [px(0).round() as i8, px(1).round() as i8],
                    blur: px(2).round().clamp(0.0, 255.0) as u8,
                    spread: px(3).round().clamp(0.0, 255.0) as u8,
                    color: parse_hex(color),
                })
            })
            .collect()
    }

    /// The deepest layer of a box-shadow token (the one that blurs most): what a single egui
    /// `Frame` shadow draws of `elevation-pop`, which has one layer in every theme.
    pub fn shadow(&self, key: &str) -> egui::Shadow {
        self.shadows(key)
            .into_iter()
            .max_by_key(|s| s.blur)
            .unwrap_or(egui::Shadow::NONE)
    }

    /// A text role, such as `body` or `page-title`.
    pub fn text(&self, role: &str) -> TextSpec {
        match self.text.get(role) {
            Some(spec) => *spec,
            None => panic!("no text role `{role}` in {}", self.theme),
        }
    }
}

/// A color as `#rrggbb`, or `#rrggbbaa` when it is not opaque.
pub fn hex(color: Color32) -> String {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

/// Every length a density level sets, by name.
pub fn density_level(level: &str) -> Option<Vec<(&'static str, f32)>> {
    let mut lengths: Vec<_> = catalog()
        .density
        .get(level)?
        .iter()
        .map(|(key, value)| (key.as_str(), *value))
        .collect();
    lengths.sort_by(|a, b| a.0.cmp(b.0));
    Some(lengths)
}

fn parse_hex(value: &str) -> Color32 {
    let digits = value.trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).expect("a hex color");
    let alpha = if digits.len() == 8 { byte(6) } else { 255 };
    Color32::from_rgba_unmultiplied(byte(0), byte(2), byte(4), alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_resolves_in_both_schemes() {
        assert_eq!(themes().next().map(|t| t.name.as_str()), Some("tidevann"));
        for theme in themes() {
            for scheme in [Scheme::Light, Scheme::Dark] {
                let appearance = Appearance {
                    theme: theme.name.clone(),
                    scheme,
                    ..Default::default()
                };
                let tokens = Tokens::resolve(&appearance, false);
                assert_eq!(tokens.theme, theme.name);
                assert!(tokens.colors.len() > 100);
            }
        }
    }

    #[test]
    fn a_box_shadow_reads_as_layers() {
        let t = Tokens::resolve(&Appearance::default(), false);
        let pop = t.shadows("elevation-pop");
        assert_eq!(pop.len(), 1);
        assert_eq!((pop[0].offset, pop[0].blur, pop[0].spread), ([0, 14], 44, 0));
        // #23333e at 0x2e, give or take the premultiplied round trip
        let [r, g, b, a] = pop[0].color.to_srgba_unmultiplied();
        assert_eq!(a, 0x2e);
        assert!(
            [(r, 0x23), (g, 0x33), (b, 0x3e)]
                .iter()
                .all(|(x, y)| x.abs_diff(*y) <= 2)
        );
        assert_eq!(t.shadows("elevation-surface").len(), 2);
    }

    #[test]
    fn hex_keeps_its_alpha_unmultiplied() {
        assert_eq!(parse_hex("#006aa0"), Color32::from_rgb(0, 0x6a, 0xa0));
        assert_eq!(
            parse_hex("#006aa014"),
            Color32::from_rgba_unmultiplied(0, 0x6a, 0xa0, 0x14)
        );
    }

    #[test]
    fn densities_list_the_default_first() {
        assert_eq!(densities(), ["regular", "compact"]);
    }

    #[test]
    fn the_text_size_answers_from_md_up() {
        assert_eq!(text_sizes(), ["small", "normal", "large", "larger"]);
        let appearance = Appearance::default();
        assert_eq!(appearance.text_size, "normal");
        let wide = Tokens::resolve_at(&appearance, false, true);
        let body = wide.text("body");
        assert_eq!(
            (wide.text_size.as_deref(), body.size, body.line_height),
            (Some("normal"), 14.0, 20.0)
        );
        let label = wide.text("label");
        assert_eq!((label.size, label.line_height), (14.0, 20.0));
        let phone = Tokens::resolve_at(&appearance, false, false);
        let body = phone.text("body");
        assert_eq!((phone.text_size, body.size, body.line_height), (None, 15.0, 22.0));
        let larger = Appearance {
            text_size: "larger".into(),
            ..Default::default()
        };
        assert_eq!(Tokens::resolve(&larger, false).text("body").size, 16.0);
        let unknown = Appearance {
            text_size: "huge".into(),
            ..Default::default()
        };
        assert_eq!(Tokens::resolve(&unknown, false).text_size.as_deref(), Some("normal"));
    }
}
