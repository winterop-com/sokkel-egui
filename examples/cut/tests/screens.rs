//! The editor's screens at the web's routes, at 1440 by 900 on the parity day (`parity/cut.json`):
//! `bun scripts/parity.ts board cut` puts them beside the web's. Then the dialogs, a clip chosen
//! with the inspector on it, and the palette.

use cut::CutApp;
use cut::app::DEMO_START;
use cut::dialogs::Asking;
use egui_kittest::{Harness, SnapshotResults};
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, CutApp> {
    let mut app = CutApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut CutApp| {
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
        ("cut_projects", "projects"),
        ("cut_editor", "p/harbor"),
        ("cut_settings", "settings"),
    ] {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
}

#[test]
fn the_dialogs_a_chosen_clip_and_the_palette() {
    let mut results = SnapshotResults::new();

    // the export dialog over the harbor edit
    let mut harness = render("p/harbor");
    harness
        .state_mut()
        .editor
        .as_mut()
        .expect("the editor is open")
        .exporting = Some(cut::editor_page::ExportView {
        file: cut::export::default_file_name("Harbor morning"),
        size: 1,
    });
    harness.run_steps(4);
    results.add(harness.try_snapshot("cut_export"));

    // the lighthouse chosen, the playhead inside it
    let mut harness = render("p/harbor");
    {
        let view = harness.state_mut().editor.as_mut().expect("the editor is open");
        view.selected = vec!["harbor-v2".into()];
        view.playback.seek(10.5, 0.0);
    }
    harness.run_steps(4);
    results.add(harness.try_snapshot("cut_clip"));

    // a sound chosen: its volume
    let mut harness = render("p/harbor");
    harness
        .state_mut()
        .editor
        .as_mut()
        .expect("the editor is open")
        .selected = vec!["harbor-a1".into()];
    harness.run_steps(4);
    results.add(harness.try_snapshot("cut_sound"));

    // naming a new project
    let mut harness = render("projects");
    harness.state_mut().ask(Asking::New);
    harness.run_steps(4);
    results.add(harness.try_snapshot("cut_new_project"));

    // the palette over the editor
    let mut harness = render("p/harbor");
    harness.state_mut().palette = Some(Default::default());
    harness.run_steps(4);
    results.add(harness.try_snapshot("cut_palette"));
}

#[test]
fn an_edit_is_saved_a_moment_after_it_changes_and_undone() {
    let mut harness = render("p/harbor");
    {
        let app = harness.state_mut();
        let now = app.now();
        let view = app.editor.as_mut().expect("the editor is open");
        view.selected = vec!["harbor-v4".into()];
        view.remove(false, now);
    }
    harness.run_steps(2);
    let clips = |app: &CutApp| app.backend.project("harbor").map(|p| p.edit.clips.len());
    // not yet: the save waits for the edit to rest
    assert_eq!(clips(harness.state()), Some(7));
    harness.state_mut().wait(600);
    harness.run_steps(2);
    assert_eq!(clips(harness.state()), Some(6));
    // undone, and kept again
    {
        let app = harness.state_mut();
        let now = app.now();
        app.editor.as_mut().expect("the editor is open").undo(now);
        app.wait(600);
    }
    harness.run_steps(2);
    assert_eq!(clips(harness.state()), Some(7));
}

#[test]
fn a_project_made_opens_in_the_editor_and_one_deleted_leaves_it() {
    let mut harness = render("projects");
    harness.state_mut().ask(Asking::New);
    harness.run_steps(2);
    harness.state_mut().dialog.name = "Juniper at dusk".into();
    harness.run_steps(2);
    // the dialog's own confirm, as the reader presses it
    use egui_kittest::kittest::Queryable;
    harness.get_by_label("Create").click();
    harness.run_steps(3);
    let app = harness.state();
    assert_eq!(app.path.first().map(String::as_str), Some("p"));
    let id = app.path[1].clone();
    assert_eq!(
        app.backend.project(&id).map(|p| p.name.as_str()),
        Some("Juniper at dusk")
    );
    assert!(app.editor.as_ref().is_some_and(|e| e.project == id));

    harness.state_mut().ask(Asking::Delete(id.clone()));
    harness.run_steps(2);
    // the dialog's Delete, drawn over the header's own
    harness
        .get_all_by_label("Delete")
        .last()
        .expect("the dialog asks")
        .click();
    harness.run_steps(3);
    let app = harness.state();
    assert!(app.backend.project(&id).is_none());
    assert_eq!(app.path, vec!["projects".to_owned()]);
    assert!(app.editor.is_none());
}
