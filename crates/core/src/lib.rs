//! sokkel-ui for egui: the same tokens and themes, and primitives drawn from them.
//!
//! Call [`install`] with an [`Appearance`] before drawing (and again when the appearance changes);
//! every widget then reads its colors, corners, sizes and text from [`tokens`].

use std::sync::Arc;

use egui::{
    Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Id, Margin, Stroke, TextStyle, Theme, Visuals,
    epaint::text::{FontTweak, VariationCoords},
    style::WidgetVisuals,
};

pub mod captions;
mod day;
mod download;
pub mod icons;
pub mod numbers;
pub mod periods;
pub mod plugins;
pub mod reactions;
pub mod text;
pub mod tokens;
mod widgets;

pub use day::Day;
pub use download::download;
pub use icons::Icon;
pub use reactions::{Reaction, toggle_reaction};
pub use text::TextRole;
pub use tokens::{
    Appearance, Motion, Scheme, Swatch, TextSpec, ThemeInfo, Tokens, densities, density_level, hex, swatch, text_sizes,
    themes,
};
pub use widgets::*;

/// IBM Plex Sans, variable on its `wght` axis (OFL), as sokkel-ui ships it: Latin and Latin Extended.
const PLEX_SANS: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-Variable-Latin.ttf");
const PLEX_SANS_EXT: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-Variable-LatinExt.ttf");
/// IBM Plex Mono Regular (OFL).
const PLEX_MONO: &[u8] = include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf");
/// IBM Plex Mono Medium and SemiBold (OFL), Latin as `@fontsource/ibm-plex-mono` ships its 500 and
/// 600: static faces, so mono set heavier is drawn from a family of its own for each.
const PLEX_MONO_MEDIUM: &[u8] = include_bytes!("../assets/fonts/IBMPlexMono-Medium-Latin.ttf");
const PLEX_MONO_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/IBMPlexMono-SemiBold-Latin.ttf");

/// The families Plex Mono Medium and SemiBold are installed under, the regular face behind each for
/// the glyphs the Latin subset lacks.
pub const MONO_MEDIUM: &str = "plex-mono-medium";
pub const MONO_SEMIBOLD: &str = "plex-mono-semibold";

/// The size mono is set at in text of `size`: as tall in x-height as `mono-x-height` of `size`, as
/// sokkel-ui's `font-size-adjust` sets it.
pub fn mono_size(tokens: &Tokens, size: f32) -> f32 {
    size * tokens.number("mono-x-height").unwrap_or(PLEX_MONO_X_HEIGHT) / PLEX_MONO_X_HEIGHT
}

/// The size mono is set at inside small text of `size` (`text-xs`, `text-help`, `text-badge`,
/// `text-head`): as tall in x-height as `mono-x-height-small` of `size`, as sokkel-ui's
/// `--mono-adjust` sets it there; [`mono_size`] is the share at body size and above.
pub fn mono_small_size(tokens: &Tokens, size: f32) -> f32 {
    let share = tokens
        .number("mono-x-height-small")
        .or_else(|| tokens.number("mono-x-height"))
        .unwrap_or(PLEX_MONO_X_HEIGHT);
    size * share / PLEX_MONO_X_HEIGHT
}

/// The code face inside text of `role`: at the role's size and weight, as tall in x-height as the
/// web sets mono there ([`mono_small_size`] in text smaller than `text.sm`, [`mono_size`] from it).
pub fn code_font(ctx: &Context, tokens: &Tokens, role: TextRole) -> egui::FontId {
    let size = role.font(tokens).size;
    let size = if size < tokens.text("sm").size {
        mono_small_size(tokens, size)
    } else {
        mono_size(tokens, size)
    };
    mono_font(ctx, size, tokens.text(role.key()).weight)
}

/// `text` in `format` with its figures at one width, as CSS's `tabular-nums` sets them: egui
/// cannot ask a face for its tabular figures, so each digit 0 to 9 stands in a slot as wide as the
/// face's widest digit at that size, the room it does not fill before it. Figures in a column then
/// line up, and a number keeps its width as it changes.
pub fn tabular_job(ctx: &Context, text: &str, format: egui::TextFormat) -> egui::text::LayoutJob {
    let font = format.font_id.clone();
    let widths: [f32; 10] =
        ctx.fonts_mut(|fonts| std::array::from_fn(|digit| fonts.glyph_width(&font, char::from(b'0' + digit as u8))));
    let widest = widths.iter().copied().fold(0.0, f32::max);
    let mut job = egui::text::LayoutJob::default();
    let mut words = String::new();
    for c in text.chars() {
        match c.to_digit(10) {
            Some(digit) => {
                if !words.is_empty() {
                    job.append(&std::mem::take(&mut words), 0.0, format.clone());
                }
                job.append(&c.to_string(), widest - widths[digit as usize], format.clone());
            }
            None => words.push(c),
        }
    }
    if !words.is_empty() {
        job.append(&words, 0.0, format);
    }
    job
}

/// Plex Mono at `size` itself and the weight nearest `weight` it ships (400, 500 or 600), for text
/// that sets its own size, as a graph does; [`mono_size`] gives the size mono takes beside sans.
/// Before the fonts reach the painter (the first frame after `install`) every weight is the
/// regular face.
pub fn mono_font(ctx: &Context, size: f32, weight: f32) -> egui::FontId {
    let name = if weight >= 550.0 {
        Some(MONO_SEMIBOLD)
    } else if weight >= 450.0 {
        Some(MONO_MEDIUM)
    } else {
        None
    };
    let family = name
        .map(|name| FontFamily::Name(name.into()))
        .filter(|family| {
            let ready = ctx.fonts(|fonts| fonts.definitions().families.contains_key(family));
            if !ready {
                ctx.request_repaint();
            }
            ready
        })
        .unwrap_or(FontFamily::Monospace);
    egui::FontId::new(size, family)
}

/// Plex Mono's x-height as a share of its size. sokkel-ui sizes mono by x-height
/// (`font-size-adjust: var(--mono-x-height)`); egui has no such rule, so mono is scaled by hand.
const PLEX_MONO_X_HEIGHT: f32 = 0.516;

fn tokens_id() -> Id {
    Id::unique("sokkel-egui/tokens")
}

fn installed_id() -> Id {
    Id::unique("sokkel-egui/installed")
}

/// The tokens the context wears, or the default appearance's before [`install`].
pub fn tokens(ctx: &Context) -> Arc<Tokens> {
    if let Some(tokens) = ctx.data(|d| d.get_temp::<Arc<Tokens>>(tokens_id())) {
        return tokens;
    }
    let tokens = Arc::new(Tokens::resolve(&Appearance::default(), false));
    ctx.data_mut(|d| d.insert_temp(tokens_id(), tokens.clone()));
    tokens
}

/// Resolved tokens for theme scopes, kept so a scope drawn every frame resolves once.
#[derive(Clone, Default)]
struct ScopeCache(std::collections::HashMap<(String, bool, String, Option<String>), Arc<Tokens>>);

/// What [`install`] last wore: the appearance, whether the system was dark, and whether the window
/// was wide (from md up), which decides the text size.
type Installed = (Appearance, bool, bool);

/// Draw `add` in another theme or scheme, as sokkel-ui's `data-theme-scope` themes one element: its
/// widgets read the scope's tokens and egui's own visuals follow them, and the page's come back
/// after. `None` for `dark` keeps the page's scheme.
pub fn scope<R>(ui: &mut egui::Ui, theme: &str, dark: Option<bool>, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let ctx = ui.ctx().clone();
    let outer = tokens(&ctx);
    let dark = dark.unwrap_or(outer.dark);
    let (page, _, wide) = ctx
        .data(|d| d.get_temp::<Installed>(installed_id()))
        .unwrap_or_else(|| (Appearance::default(), false, true));
    let (density, text_size) = (page.density, page.text_size);
    let key = (theme.to_owned(), dark, density.clone(), outer.text_size.clone());
    let inner = ctx.data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<ScopeCache>(Id::unique("sokkel-egui/scopes"));
        cache
            .0
            .entry(key)
            .or_insert_with(|| {
                let scheme = if dark { Scheme::Dark } else { Scheme::Light };
                Arc::new(Tokens::resolve_at(
                    &Appearance {
                        theme: theme.to_owned(),
                        scheme,
                        density,
                        text_size,
                        ..Default::default()
                    },
                    dark,
                    wide,
                ))
            })
            .clone()
    });
    ctx.data_mut(|d| d.insert_temp(tokens_id(), inner.clone()));
    let result = ui
        .scope(|ui| {
            *ui.visuals_mut() = visuals(&inner);
            add(ui)
        })
        .inner;
    ctx.data_mut(|d| d.insert_temp(tokens_id(), outer));
    result
}

/// Wear an appearance: the fonts (once), egui's own visuals and spacing, and the tokens every
/// widget reads. Cheap when nothing changed, so an app may call it every frame.
pub fn install(ctx: &Context, appearance: &Appearance) {
    let system_dark = ctx.system_theme() == Some(Theme::Dark);
    let wide = !is_phone(ctx);
    let key: Installed = (appearance.clone(), system_dark, wide);
    if ctx.data(|d| d.get_temp::<Installed>(installed_id())).as_ref() == Some(&key) {
        return;
    }
    if ctx.data(|d| d.get_temp::<Installed>(installed_id())).is_none() {
        install_fonts(ctx);
        // SVG marks and pictures, drawn by resvg
        egui_extras::install_image_loaders(ctx);
    }
    let tokens = Arc::new(Tokens::resolve_at(appearance, system_dark, wide));
    let theme = if tokens.dark { Theme::Dark } else { Theme::Light };
    ctx.set_theme(theme);
    ctx.style_mut_of(theme, |style| {
        style.visuals = visuals(&tokens);
        let body = tokens.text("body").size;
        let mono = mono_size(&tokens, body);
        style.text_styles = [
            (TextStyle::Heading, TextRole::PageTitle.font(&tokens)),
            (TextStyle::Body, FontId::proportional(body)),
            (TextStyle::Button, TextRole::Action.font(&tokens)),
            (TextStyle::Small, TextRole::Help.font(&tokens)),
            (TextStyle::Monospace, FontId::monospace(mono)),
        ]
        .into();
        // motion off stops egui's own animations: popups, collapses and scroll fades jump to their end
        style.animation_time = if appearance.motion == Motion::Off {
            0.0
        } else {
            1.0 / 12.0
        };
        let spacing = &mut style.spacing;
        spacing.item_spacing = egui::vec2(8.0, 8.0);
        spacing.button_padding = egui::vec2(10.0, 4.0);
        spacing.interact_size.y = tokens.density("control");
        spacing.window_margin = Margin::same(16);
        spacing.menu_margin = Margin::same(tokens.length("palette-inset") as i8);
        // a scroller cuts its content plainly, as the web's do; the palette's list fades its own foot
        spacing.scroll.fade.strength = 0.0;
    });
    ctx.data_mut(|d| {
        d.insert_temp(tokens_id(), tokens);
        d.insert_temp(installed_id(), key);
    });
}

/// Install Plex Sans and Plex Mono ahead of egui's own fonts, which stay behind them for symbols.
pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    let regular = || FontTweak {
        coords: VariationCoords::new([("wght", 400.0)]),
        ..Default::default()
    };
    fonts.font_data.insert(
        "plex-sans".into(),
        Arc::new(FontData::from_static(PLEX_SANS).tweak(regular())),
    );
    fonts.font_data.insert(
        "plex-sans-ext".into(),
        Arc::new(FontData::from_static(PLEX_SANS_EXT).tweak(regular())),
    );
    fonts
        .font_data
        .insert("plex-mono".into(), Arc::new(FontData::from_static(PLEX_MONO)));
    for (name, bytes) in [(MONO_MEDIUM, PLEX_MONO_MEDIUM), (MONO_SEMIBOLD, PLEX_MONO_SEMIBOLD)] {
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
        fonts
            .families
            .insert(FontFamily::Name(name.into()), vec![name.into(), "plex-mono".into()]);
    }
    // Lucide in a family of its own: its glyphs sit in the private use area, where a text face's
    // own glyphs would shadow them
    fonts.font_data.insert(
        icons::FAMILY.into(),
        Arc::new(FontData::from_static(lucide_icons::LUCIDE_FONT_BYTES)),
    );
    fonts
        .families
        .insert(FontFamily::Name(icons::FAMILY.into()), vec![icons::FAMILY.into()]);
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "plex-sans-ext".into());
    proportional.insert(0, "plex-sans".into());
    // the code face after them for the symbols the Latin faces lack, such as the arrows
    proportional.insert(2, "plex-mono".into());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "plex-mono".into());
    ctx.set_fonts(fonts);
}

fn widget(fill: Color32, stroke: Color32, ink: Color32, tokens: &Tokens) -> WidgetVisuals {
    WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: Stroke::new(1.0, stroke),
        corner_radius: tokens.radius("control"),
        fg_stroke: Stroke::new(1.5, ink),
        expansion: 0.0,
    }
}

/// egui's own visuals from the tokens, for what egui draws itself: panels, windows, scroll bars,
/// selections and its stock widgets.
fn visuals(t: &Tokens) -> Visuals {
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    // Light text on a dark ground: egui's own boost (2c - c^2) inks it a tenth heavier than the
    // web draws it; coverage to the power 0.85 matches Chrome's within 1% at a Retina screen's
    // scale and 6% at 1x (measured on the tasks inbox, `PARITY_SCALE=2`).
    if t.dark {
        v.text_options.color_transfer_function = egui::epaint::FontColorTransferFunction::Gamma(0.85);
    }
    let foreground = t.color("foreground");
    v.override_text_color = None;
    v.panel_fill = t.color("background");
    v.window_fill = t.color("popover");
    v.extreme_bg_color = t.color("field");
    v.text_edit_bg_color = Some(t.color("field"));
    v.faint_bg_color = t.color("muted");
    v.code_bg_color = t.color("muted");
    v.hyperlink_color = t.color("primary-ink");
    v.warn_fg_color = t.color("warning");
    v.error_fg_color = t.color("critical");
    v.window_stroke = Stroke::new(1.0, t.color("border"));
    v.window_corner_radius = t.radius("surface");
    v.menu_corner_radius = t.radius("surface");
    // egui's own windows and popups wear the tokens' elevation, as the library's surfaces do
    v.window_shadow = t.shadow("elevation-pop");
    v.popup_shadow = t.shadow("elevation-pop");
    v.selection.bg_fill = t.color_at("primary", 0.25);
    // selected text keeps its ink, as the browser's own selection does
    v.selection.stroke = Stroke::new(1.0, t.color("foreground"));
    v.text_cursor.stroke = Stroke::new(2.0, t.color("primary"));
    v.widgets.noninteractive = widget(t.color("background"), t.color("border"), foreground, t);
    v.widgets.inactive = widget(t.color("secondary"), t.color("border"), foreground, t);
    v.widgets.hovered = widget(t.color("hover"), t.color("border-strong"), foreground, t);
    v.widgets.active = widget(t.color("hover"), t.color("ring"), foreground, t);
    v.widgets.open = widget(t.color("secondary"), t.color("border"), foreground, t);
    v
}
