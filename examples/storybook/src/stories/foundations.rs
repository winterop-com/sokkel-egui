use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, RichText, Sense, Stroke, StrokeKind, Ui, vec2};
use sokkel_egui::{Appearance, TextRole, densities, density_level, hex, icons, tokens};

use super::icon_list::LIBRARY_ICONS;
use super::{State, section};

/// The color groups, each a title and the token names it holds by prefix or by name.
const GROUPS: &[(&str, &[&str])] = &[
    (
        "Surfaces and text",
        &[
            "background",
            "foreground",
            "card",
            "card-foreground",
            "popover",
            "popover-foreground",
            "muted",
            "muted-foreground",
            "faint",
            "secondary",
            "secondary-foreground",
            "accent",
            "accent-foreground",
            "hover",
            "canvas",
        ],
    ),
    (
        "Lines and fields",
        &["border", "border-strong", "input", "field", "ring"],
    ),
    (
        "Actions and brand",
        &[
            "primary",
            "primary-foreground",
            "primary-ink",
            "destructive",
            "brand",
            "brand-foreground",
        ],
    ),
    (
        "States",
        &[
            "good",
            "good-ink",
            "info",
            "info-ink",
            "warning",
            "warning-ink",
            "critical",
            "critical-ink",
        ],
    ),
    ("Run states", &["status-"]),
    ("Kinds", &["kind-"]),
    ("Charts", &["chart-"]),
    ("Sidebar", &["sidebar"]),
];

fn in_group(key: &str, names: &[&str]) -> bool {
    names.iter().any(|name| {
        if name.ends_with('-') || *name == "sidebar" {
            key.starts_with(name)
        } else {
            key == *name
        }
    })
}

fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn colors(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let all: Vec<(&str, Color32)> = t.colors().collect();
    let grouped = |key: &str| GROUPS.iter().any(|(_, names)| in_group(key, names));
    let rest: Vec<&str> = all
        .iter()
        .map(|(key, _)| *key)
        .filter(|key| !grouped(key) && !key.starts_with("code-"))
        .collect();
    for (title, names) in GROUPS {
        section(ui, title);
        let keys: Vec<&str> = all
            .iter()
            .map(|(key, _)| *key)
            .filter(|key| in_group(key, names))
            .collect();
        swatches(ui, &keys);
    }
    section(ui, "Frame parts and the rest");
    swatches(ui, &rest);
}

/// A wrapping grid of swatches: the color over a check, its name and its hex.
fn swatches(ui: &mut Ui, keys: &[&str]) {
    let t = tokens(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
        for key in keys {
            let color = t.color(key);
            let (rect, response) = ui.allocate_exact_size(vec2(152.0, 96.0), Sense::hover());
            let painter = ui.painter();
            let chip = egui::Rect::from_min_size(rect.min, vec2(rect.width(), 48.0));
            let radius = t.radius("surface");
            // a check under the color, so a translucent token shows as one
            painter.rect_filled(chip, radius, t.color("card"));
            if color.a() < 255 {
                let cell = 8.0;
                for row in 0..6 {
                    for col in 0..19 {
                        if (row + col) % 2 == 0 {
                            let min = chip.min + vec2(col as f32 * cell, row as f32 * cell);
                            let r = egui::Rect::from_min_size(min, vec2(cell, cell)).intersect(chip);
                            painter.rect_filled(r, CornerRadius::ZERO, t.color("muted"));
                        }
                    }
                }
            }
            painter.rect(
                chip,
                radius,
                color,
                Stroke::new(1.0, t.color("border")),
                StrokeKind::Inside,
            );
            let ink = t.color("foreground");
            painter.text(
                rect.left_top() + vec2(0.0, 56.0),
                egui::Align2::LEFT_TOP,
                *key,
                TextRole::Help.font(&t),
                ink,
            );
            painter.text(
                rect.left_top() + vec2(0.0, 76.0),
                egui::Align2::LEFT_TOP,
                hex(color),
                mono(12.0),
                t.color("muted-foreground"),
            );
            response.on_hover_text(format!("{key}  {}", hex(color)));
        }
    });
}

pub fn typography(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    section(ui, "Roles");
    egui::Grid::new("roles")
        .num_columns(3)
        .spacing(vec2(32.0, 14.0))
        .show(ui, |ui| {
            for role in TextRole::ALL {
                let spec = t.text(role.key());
                ui.label(
                    RichText::new(role.key())
                        .font(mono(13.0))
                        .color(t.color("muted-foreground")),
                );
                ui.label(
                    role.rich(&t, "Nightly export ran in 4m 12s")
                        .color(t.color("foreground")),
                );
                let tracking = if spec.tracking == 0.0 {
                    String::new()
                } else {
                    format!(" · {}em", spec.tracking)
                };
                ui.label(
                    RichText::new(format!(
                        "{} / {} · {}{tracking}",
                        spec.size, spec.line_height, spec.weight
                    ))
                    .font(mono(13.0))
                    .color(t.color("muted-foreground")),
                );
                ui.end_row();
            }
        });
    section(ui, "Faces");
    ui.label(TextRole::Body.rich(
        &t,
        "IBM Plex Sans: the quick brown fox jumps over the lazy dog, 0123456789",
    ));
    let mono_size = t.text("body").size * t.number("mono-x-height").unwrap_or(0.516) / 0.516;
    ui.label(
        RichText::new("IBM Plex Mono: fn send_digest(run: &Run) -> Result<(), Error>, 0123456789")
            .font(mono(mono_size))
            .color(t.color("foreground")),
    );
    section(ui, "Weights");
    ui.horizontal(|ui| {
        for weight in [300.0, 400.0, 500.0, 600.0, 700.0] {
            ui.label(
                RichText::new(format!("Plex {weight}"))
                    .font(TextRole::Body.font(&t))
                    .variation("wght", weight)
                    .color(t.color("foreground")),
            );
            ui.add_space(16.0);
        }
    });
}

pub fn corners(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    section(ui, "Roles");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(20.0, 20.0);
        for role in ["mini", "chip", "thumb", "sm", "control", "md", "node", "surface", "3xl"] {
            let radius = t.length(&format!("radius-{role}"));
            let (rect, _) = ui.allocate_exact_size(vec2(112.0, 112.0), Sense::hover());
            let tile = egui::Rect::from_min_size(rect.min, vec2(112.0, 72.0));
            ui.painter().rect(
                tile,
                CornerRadius::same(radius.round() as u8),
                t.color("muted"),
                Stroke::new(1.0, t.color("border-strong")),
                StrokeKind::Inside,
            );
            ui.painter().text(
                rect.left_top() + vec2(0.0, 80.0),
                egui::Align2::LEFT_TOP,
                role,
                TextRole::Help.font(&t),
                t.color("foreground"),
            );
            ui.painter().text(
                rect.left_top() + vec2(0.0, 96.0),
                egui::Align2::LEFT_TOP,
                format!("{radius}px"),
                mono(12.0),
                t.color("muted-foreground"),
            );
        }
    });
    section(ui, "The base");
    ui.label(
        TextRole::Body
            .rich(
                &t,
                format!(
                    "radius {}px · control radius {}px · surface radius {}px",
                    t.length("radius"),
                    t.length("control-radius"),
                    t.length("surface-radius")
                ),
            )
            .color(t.color("muted-foreground")),
    );
}

pub fn density(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let current = Appearance::default();
    let levels = densities();
    section(ui, "Lengths per level");
    let table: Vec<(&str, Vec<(&str, f32)>)> = levels
        .iter()
        .map(|level| (*level, density_level(level).unwrap_or_default()))
        .collect();
    let keys: Vec<&str> = table[0].1.iter().map(|(key, _)| *key).collect();
    egui::Grid::new("density")
        .num_columns(1 + table.len())
        .spacing(vec2(24.0, 10.0))
        .show(ui, |ui| {
            ui.label(TextRole::Head.rich(&t, "Token").color(t.color("muted-foreground")));
            for (level, _) in &table {
                let mark = if *level == current.density { " (default)" } else { "" };
                ui.label(
                    TextRole::Head
                        .rich(&t, format!("{level}{mark}"))
                        .color(t.color("muted-foreground")),
                );
            }
            ui.end_row();
            for key in keys {
                ui.label(RichText::new(key).font(mono(13.0)).color(t.color("foreground")));
                for (_, lengths) in &table {
                    let value = lengths.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).unwrap_or(0.0);
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(vec2(value, 10.0), Sense::hover());
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(2), t.color_at("primary", 0.6));
                        ui.label(
                            RichText::new(format!("{value}"))
                                .font(mono(12.0))
                                .color(t.color("muted-foreground")),
                        );
                    });
                }
                ui.end_row();
            }
        });
}

pub fn code_colors(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    section(ui, "A code block");
    // a small program, cut by hand into the kinds of word the code-* tokens color
    let lines: &[&[(&str, &str)]] = &[
        &[("code-comment", "// Send the digest once the export has run.")],
        &[
            ("code-keyword", "export async function "),
            ("code-function", "sendDigest"),
            ("code-punctuation", "("),
            ("code-variable", "run"),
            ("code-punctuation", ": "),
            ("code-type", "Run"),
            ("code-punctuation", ") {"),
        ],
        &[
            ("", "    "),
            ("code-keyword", "if "),
            ("code-punctuation", "("),
            ("code-variable", "run"),
            ("code-punctuation", "."),
            ("code-property", "state"),
            ("code-operator", " !== "),
            ("code-string", "'succeeded'"),
            ("code-punctuation", ") "),
            ("code-keyword", "return"),
        ],
        &[
            ("", "    "),
            ("code-keyword", "await "),
            ("code-function", "notify"),
            ("code-punctuation", "("),
            ("code-string", "'ada@example.org'"),
            ("code-punctuation", ", { "),
            ("code-property", "retries"),
            ("code-punctuation", ": "),
            ("code-number", "3"),
            ("code-punctuation", ", "),
            ("code-property", "strict"),
            ("code-punctuation", ": "),
            ("code-constant", "true"),
            ("code-punctuation", " })"),
        ],
        &[("code-punctuation", "}")],
    ];
    egui::Frame::new()
        .fill(t.color("muted"))
        .stroke(Stroke::new(1.0, t.color("border")))
        .corner_radius(t.radius("surface"))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            for line in lines {
                let mut job = egui::text::LayoutJob::default();
                for (token, text) in *line {
                    let color = if token.is_empty() {
                        t.color("foreground")
                    } else {
                        t.color(token)
                    };
                    job.append(text, 0.0, egui::TextFormat::simple(mono(13.5), color));
                }
                ui.label(job);
            }
        });
    section(ui, "Tokens");
    let keys: Vec<&str> = t
        .colors()
        .map(|(key, _)| key)
        .filter(|key| key.starts_with("code-"))
        .collect();
    swatches(ui, &keys);
}

pub fn icons(state: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let known: Vec<(&str, icons::Icon)> = LIBRARY_ICONS
        .iter()
        .filter_map(|name| icons::by_name(name).map(|icon| (*name, icon)))
        .collect();
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.icon_search)
                .hint_text(TextRole::Body.rich(&t, "Search").color(t.color("muted-foreground")))
                .desired_width(240.0),
        );
        ui.add_space(12.0);
        for size in [16.0, 20.0, 24.0] {
            ui.selectable_value(&mut state.icon_size, size, format!("{size}"));
        }
        ui.add_space(12.0);
        let missing = LIBRARY_ICONS.len() - known.len();
        let note = if missing == 0 {
            format!("{} icons", known.len())
        } else {
            format!(
                "{} icons; {missing} names lucide-react keeps as aliases are not in the font",
                known.len()
            )
        };
        ui.label(TextRole::Help.rich(&t, note).color(t.color("muted-foreground")));
    });
    ui.add_space(16.0);
    let needle = state.icon_search.to_lowercase().replace(' ', "-");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        for (name, icon) in known.iter().filter(|(name, _)| name.contains(&needle)) {
            let (rect, response) = ui.allocate_exact_size(vec2(112.0, 84.0), Sense::click());
            let painter = ui.painter();
            if response.hovered() {
                painter.rect_filled(rect, t.radius("surface"), t.color("hover"));
            }
            icons::paint(
                painter,
                rect.center_top() + vec2(0.0, 32.0),
                *icon,
                state.icon_size,
                t.color("foreground"),
            );
            let galley = egui::WidgetText::from(TextRole::Help.rich(&t, *name)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                rect.width() - 8.0,
                TextRole::Help.font(&t),
            );
            let pos = egui::pos2(rect.center().x - galley.size().x / 2.0, rect.top() + 58.0);
            ui.painter().galley(pos, galley, t.color("muted-foreground"));
            if response.on_hover_text(*name).clicked() {
                ui.ctx().copy_text((*name).to_owned());
            }
        }
    });
}
