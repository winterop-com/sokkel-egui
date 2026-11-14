//! The storage browser's screens and what its controls do: a folder lists its files, a file shows
//! what it holds, the tree opens and shuts and is searched, a file pressed in a list is chosen.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::{Appearance, FileKind, Scheme, tree_matches, visible_rows};
use storage::StorageApp;

fn render(scheme: Scheme, setup: impl FnOnce(&mut StorageApp)) -> Harness<'static, StorageApp> {
    let mut app = StorageApp::on(Appearance {
        scheme,
        ..Default::default()
    });
    setup(&mut app);
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut StorageApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(6);
    harness
}

#[test]
fn a_folder() {
    let mut harness = render(Scheme::Light, |_| {});
    harness.snapshot("storage");
}

#[test]
fn a_table_file_in_dark() {
    let mut harness = render(Scheme::Dark, |app| {
        app.choose("harbor-exports/nightly-export/2026-11-17/orders.csv");
    });
    harness.snapshot("storage_csv");
}

#[test]
fn a_text_file() {
    let mut harness = render(Scheme::Light, |app| {
        app.choose("larkspur-index/config/schedule.toml");
    });
    harness.snapshot("storage_text");
}

#[test]
fn every_path_is_found_and_its_parents_open() {
    let mut app = StorageApp::on(Appearance::default());
    app.choose("juniper-inventory/photos/shelf-a.jpg");
    let entry = app.entry().expect("the photo");
    assert_eq!(entry.kind, FileKind::Image);
    assert!(app.open.contains("juniper-inventory"));
    assert!(app.open.contains("juniper-inventory/photos"));
    let rows = visible_rows(&app.nodes, &app.open, None);
    assert!(
        rows.iter()
            .any(|r| r.node.id == "juniper-inventory/photos/shelf-a.jpg" && r.level == 3)
    );
}

#[test]
fn a_search_keeps_the_matches_and_what_is_above_them() {
    let app = StorageApp::on(Appearance::default());
    let kept = tree_matches(&app.nodes, "manifest");
    assert!(kept.contains("larkspur-index"));
    assert!(kept.contains("larkspur-index/rebuild-index/manifest.json"));
    assert!(!kept.contains("harbor-exports"));
}

#[test]
fn a_branch_opens_when_chosen_in_the_tree() {
    let mut harness = render(Scheme::Light, |_| {});
    assert!(!harness.state().open.contains("larkspur-index"));
    harness.get_by_label("larkspur-index").click();
    harness.run_steps(3);
    assert!(harness.state().open.contains("larkspur-index"));
    assert_eq!(harness.state().chosen.as_deref(), Some("larkspur-index"));
}

#[test]
fn a_file_pressed_in_the_list_is_chosen() {
    let mut harness = render(Scheme::Light, |_| {});
    harness
        .get_by(|node| node.label().is_some_and(|label| label.starts_with("summary.json, ")))
        .click();
    harness.run_steps(3);
    assert_eq!(
        harness.state().chosen.as_deref(),
        Some("harbor-exports/nightly-export/2026-11-17/summary.json")
    );
}
