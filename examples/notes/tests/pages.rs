//! The screens parity/notes.json lists, drawn as the web's are shot: the vault opened at
//! 2026-11-18 09:00 UTC. `bun scripts/parity.ts board notes` puts them beside the web's.

use egui_kittest::{Harness, SnapshotResults};
use notes::NotesApp;
use notes::app::DEMO_START;
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, NotesApp> {
    let mut app = NotesApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut NotesApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness
}

#[test]
fn every_screen() {
    let mut results = SnapshotResults::new();
    for (name, path) in [
        ("notes_all", "notes"),
        ("notes_folder", "folder/runbooks"),
        ("notes_note", "note/welcome"),
        ("notes_daily", "daily"),
        ("notes_tags", "tags"),
        ("notes_graph", "graph"),
        ("notes_search", "search"),
        ("notes_search_found", "search/export"),
        ("notes_settings", "settings"),
        ("notes_settings_daily", "settings/daily"),
    ] {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
}

#[test]
fn every_dialog() {
    use notes::backend::Asking;
    let mut results = SnapshotResults::new();
    let asks = [
        (
            "notes_dialog_rename",
            "note/welcome",
            Asking::RenameNote { id: "welcome".into() },
        ),
        (
            "notes_dialog_move",
            "note/welcome",
            Asking::MoveNote { id: "welcome".into() },
        ),
        (
            "notes_dialog_delete_folder",
            "folder/runbooks",
            Asking::DeleteFolder { id: "runbooks".into() },
        ),
        (
            "notes_dialog_make",
            "note/welcome",
            Asking::Create {
                title: "Export retention".into(),
                folder: None,
            },
        ),
    ];
    for (name, path, asking) in asks {
        let mut harness = render(path);
        harness.state_mut().ask(asking);
        harness.run_steps(4);
        results.add(harness.try_snapshot(name));
    }
}

#[test]
fn the_palettes() {
    let mut results = SnapshotResults::new();
    let mut harness = render("notes");
    harness.state_mut().palette = Some(Default::default());
    harness.run_steps(4);
    results.add(harness.try_snapshot("notes_palette"));
    let mut harness = render("note/welcome");
    harness.state_mut().switching = Some(("export ret".into(), 0));
    harness.run_steps(4);
    results.add(harness.try_snapshot("notes_switcher_make"));
}
