//! sokkel-design's resolved paints (D10): every paint egui reads is there in every theme and
//! scheme, so no part falls back to a choice of its own.

use sokkel_egui::{Appearance, ButtonVariant, PAINTS_READ, Scheme, Tokens, themes};

/// Every paint key egui's parts read.
fn read_by_egui() -> Vec<String> {
    let mut keys: Vec<String> = PAINTS_READ.iter().map(|k| (*k).to_owned()).collect();
    for variant in [
        ButtonVariant::Default,
        ButtonVariant::Outline,
        ButtonVariant::Secondary,
        ButtonVariant::Ghost,
        ButtonVariant::Destructive,
        ButtonVariant::Link,
    ] {
        for state in ["rest", "hover"] {
            keys.push(format!("button.{}.{state}.container", variant.key()));
        }
    }
    keys
}

#[test]
fn every_paint_egui_reads_is_in_every_theme_and_scheme() {
    let mut missing = Vec::new();
    for theme in themes() {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let tokens = Tokens::resolve(
                &Appearance {
                    theme: theme.name.clone(),
                    scheme,
                    ..Default::default()
                },
                false,
            );
            for key in read_by_egui() {
                if tokens.paint(&key).is_none() {
                    missing.push(format!("{} {:?} {key}", theme.name, scheme));
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "paints egui reads that the tokens lack:\n{}",
        missing.join("\n")
    );
}

#[test]
fn a_paint_reads_its_colors() {
    let tokens = Tokens::resolve(&Appearance::default(), false);
    let paint = tokens
        .paint("button.default.rest.container")
        .expect("the default button's paint");
    assert_eq!(paint.ground, Some(tokens.color("primary")));
    assert_eq!(paint.ink, Some(tokens.color("primary-foreground")));
    let focus = tokens.paint("button.default.focus.container").expect("its focus paint");
    assert!(
        focus
            .outline
            .is_some_and(|(_, width, offset)| width == 2.0 && offset == 2.0)
    );
}

#[test]
fn a_chip_paint_reads_its_washes_and_the_remove_buttons_opacity() {
    let tokens = Tokens::resolve(&Appearance::default(), false);
    let selected = tokens
        .paint("chip.good.selected.container")
        .expect("a chosen good chip's paint");
    assert_eq!(selected.edge, selected.ink);
    assert_eq!(selected.ink, Some(tokens.color("good-ink")));
    let rest = tokens
        .paint("chip.base.rest.remove")
        .expect("the remove button's paint");
    assert_eq!(rest.opacity, Some(0.7));
    let hover = tokens.paint("chip.base.hover.remove").expect("its hover paint");
    assert_eq!(hover.opacity, Some(1.0));
    assert!(hover.ground.is_some());
}
