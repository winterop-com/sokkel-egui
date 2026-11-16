//! Every story in tidevann light, the foundations in tidevann dark, and the sample in steel.

use egui_kittest::{Harness, SnapshotResults};
use sokkel_egui::{Appearance, Scheme};
use storybook::{Storybook, stories::STORIES};

/// A story's canvas alone, so a story added to the sidebar changes no other picture.
fn render(theme: &str, scheme: Scheme, path: &str) -> Harness<'static, Storybook> {
    render_with(theme, scheme, path, [1040.0, 900.0], |ui, book| book.show_story(ui))
}

fn render_with(
    theme: &str,
    scheme: Scheme,
    path: &str,
    size: [f32; 2],
    show: fn(&mut eframe::egui::Ui, &mut Storybook),
) -> Harness<'static, Storybook> {
    let appearance = Appearance {
        theme: theme.into(),
        scheme,
        ..Default::default()
    };
    let mut book = Storybook::with(appearance);
    assert!(book.select(path), "no story {path}");
    let mut harness = Harness::builder()
        .with_size(size)
        .build_ui_state(move |ui, book: &mut Storybook| show(ui, book), book);
    // the first frame installs the fonts and the theme, the next ones lay out with them
    harness.run_steps(3);
    harness
}

fn slug(path: &str) -> String {
    path.to_lowercase().replace(['/', ' '], "_")
}

#[test]
fn every_story() {
    let mut results = SnapshotResults::new();
    for story in STORIES {
        let mut harness = render("tidevann", Scheme::Light, story.path);
        results.add(harness.try_snapshot(slug(story.path)));
    }
    for story in STORIES.iter().filter(|story| story.path.starts_with("Foundations/")) {
        let mut harness = render("tidevann", Scheme::Dark, story.path);
        results.add(harness.try_snapshot(format!("{}_dark", slug(story.path))));
    }
    let mut harness = render("steel", Scheme::Light, "Themes/Sample");
    results.add(harness.try_snapshot("themes_sample_steel"));
    // the storybook's own frame once: its sidebar changes with every story added
    let mut harness = render_with(
        "tidevann",
        Scheme::Light,
        "Foundations/Colors",
        [1280.0, 900.0],
        |ui, book| book.show(ui),
    );
    results.add(harness.try_snapshot("storybook_frame"));
}

#[test]
fn every_theme_draws_every_color() {
    for theme in sokkel_egui::themes() {
        for scheme in [Scheme::Light, Scheme::Dark] {
            render(&theme.name, scheme, "Foundations/Colors");
        }
    }
}
